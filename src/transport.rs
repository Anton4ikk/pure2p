//! Network transport module
//!
//! This module handles peer-to-peer networking including:
//! - HTTP/2 POST /output endpoint for receiving messages
//! - Direct peer-to-peer message sending
//! - Delivery state tracking and logging
//! - Integration with message queue for retry logic

use crate::{protocol::MessageEnvelope, Error, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Represents a peer in the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Peer {
    /// Unique peer identifier (UID)
    pub id: String,
    /// Network address (e.g., "192.168.1.100:8080" or "example.com:8080")
    pub addr: String,
    /// Public key for encryption
    pub public_key: Vec<u8>,
}

/// Delivery state for logging
#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryState {
    /// Message successfully delivered
    Success,
    /// Message queued for later delivery
    Queued,
    /// Message failed, will retry
    Retry {
        /// Current attempt number
        attempt: u32,
        /// Next retry timestamp in milliseconds
        next_retry_ms: i64,
    },
    /// Message failed permanently (max retries exceeded)
    Failed,
}

/// Callback type for handling received messages
pub type MessageHandler = Arc<dyn Fn(MessageEnvelope) + Send + Sync>;

/// Network transport layer
pub struct Transport {
    /// Local binding address
    local_addr: Option<SocketAddr>,
    /// Known peers
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Message handler callback
    message_handler: Arc<Mutex<Option<MessageHandler>>>,
    /// HTTP client for sending messages
    client: Client<HttpConnector, Full<Bytes>>,
}

impl Transport {
    /// Create a new transport instance
    pub fn new() -> Self {
        let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();

        Self {
            local_addr: None,
            peers: Arc::new(Mutex::new(Vec::new())),
            message_handler: Arc::new(Mutex::new(None)),
            client,
        }
    }

    /// Set the message handler callback
    pub async fn set_message_handler<F>(&self, handler: F)
    where
        F: Fn(MessageEnvelope) + Send + Sync + 'static,
    {
        let mut guard = self.message_handler.lock().await;
        *guard = Some(Arc::new(handler));
    }

    /// Start the transport layer and listen for incoming connections
    pub async fn start(&mut self, addr: SocketAddr) -> Result<()> {
        info!("Starting transport on {}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Transport(format!("Failed to bind to {}: {}", addr, e)))?;

        // Get the actual bound address (important when using port 0 for auto-assignment)
        let actual_addr = listener.local_addr()
            .map_err(|e| Error::Transport(format!("Failed to get local address: {}", e)))?;

        self.local_addr = Some(actual_addr);

        let message_handler = self.message_handler.clone();

        // Spawn listener task
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_addr)) => {
                        debug!("Accepted connection from {}", remote_addr);

                        let io = TokioIo::new(stream);
                        let handler = message_handler.clone();

                        tokio::spawn(async move {
                            let service = service_fn(move |req| {
                                handle_request(req, handler.clone())
                            });

                            if let Err(e) = http1::Builder::new()
                                .serve_connection(io, service)
                                .await
                            {
                                error!("Error serving connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        info!("Transport listening on {}", actual_addr);
        Ok(())
    }

    /// Add or update a peer
    pub async fn add_peer(&self, peer: Peer) -> Result<()> {
        let mut peers = self.peers.lock().await;

        // Check if peer already exists and update, otherwise add
        if let Some(existing) = peers.iter_mut().find(|p| p.id == peer.id) {
            *existing = peer;
            debug!("Updated existing peer");
        } else {
            peers.push(peer);
            debug!("Added new peer");
        }

        Ok(())
    }

    /// Remove a peer
    pub async fn remove_peer(&self, peer_id: &str) -> Result<()> {
        let mut peers = self.peers.lock().await;
        peers.retain(|p| p.id != peer_id);
        Ok(())
    }

    /// Get a peer by ID
    pub async fn get_peer(&self, peer_id: &str) -> Option<Peer> {
        let peers = self.peers.lock().await;
        peers.iter().find(|p| p.id == peer_id).cloned()
    }

    /// Send a message to a peer via HTTP POST to their /output endpoint
    ///
    /// Returns the delivery state:
    /// - Success: Message delivered immediately
    /// - Error: Message failed to deliver (caller should queue for retry)
    pub async fn send(&self, peer_addr: &str, envelope: &MessageEnvelope) -> Result<DeliveryState> {
        info!("Sending message to peer at {}", peer_addr);

        // Serialize the envelope to CBOR
        let payload = envelope.to_cbor()?;

        // Construct the POST request
        let url = format!("http://{}/output", peer_addr);

        let req = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", "application/cbor")
            .body(Full::new(Bytes::from(payload)))
            .map_err(|e| Error::Transport(format!("Failed to build request: {}", e)))?;

        // Send the request
        match self.client.request(req).await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Message delivered successfully to {}", peer_addr);
                    Ok(DeliveryState::Success)
                } else {
                    warn!(
                        "Message delivery failed with status {}: {}",
                        response.status(),
                        peer_addr
                    );
                    Err(Error::Transport(format!(
                        "Delivery failed with status {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                error!("Failed to send message to {}: {}", peer_addr, e);
                Err(Error::Transport(format!("Send failed: {}", e)))
            }
        }
    }

    /// Get list of known peers
    pub async fn peers(&self) -> Vec<Peer> {
        let peers = self.peers.lock().await;
        peers.clone()
    }

    /// Get the local listening address
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle incoming HTTP requests
async fn handle_request(
    req: Request<Incoming>,
    message_handler: Arc<Mutex<Option<MessageHandler>>>,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/output") => {
            debug!("Received POST /output request");

            // Read the body
            let body = req.collect().await?.to_bytes();

            // Deserialize the message envelope from CBOR
            match MessageEnvelope::from_cbor(&body) {
                Ok(envelope) => {
                    info!(
                        "Received message from {} to {}",
                        envelope.from_uid, envelope.to_uid
                    );

                    // Call the message handler if set
                    let handler_guard = message_handler.lock().await;
                    if let Some(handler) = handler_guard.as_ref() {
                        handler(envelope);
                    } else {
                        warn!("No message handler set, message dropped");
                    }

                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::from("Message received")))
                        .unwrap())
                }
                Err(e) => {
                    error!("Failed to deserialize message: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from(format!(
                            "Invalid message format: {}",
                            e
                        ))))
                        .unwrap())
                }
            }
        }
        _ => {
            debug!("Received unsupported request: {} {}", req.method(), req.uri().path());
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap())
        }
    }
}

/// Log delivery state
pub fn log_delivery_state(peer_addr: &str, state: &DeliveryState) {
    match state {
        DeliveryState::Success => {
            info!("✓ Message delivered successfully to {}", peer_addr);
        }
        DeliveryState::Queued => {
            info!("⊙ Message queued for delivery to {}", peer_addr);
        }
        DeliveryState::Retry { attempt, next_retry_ms } => {
            warn!(
                "⟲ Message delivery failed to {} (attempt {}), will retry at timestamp {}",
                peer_addr, attempt, next_retry_ms
            );
        }
        DeliveryState::Failed => {
            error!("✗ Message delivery failed permanently to {}", peer_addr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_transport_creation() {
        let transport = Transport::new();
        assert!(transport.local_addr().is_none());
    }

    #[test]
    fn test_peer_equality() {
        let peer1 = Peer {
            id: "alice".to_string(),
            addr: "127.0.0.1:8080".to_string(),
            public_key: vec![1, 2, 3],
        };

        let peer2 = Peer {
            id: "alice".to_string(),
            addr: "127.0.0.1:8080".to_string(),
            public_key: vec![1, 2, 3],
        };

        assert_eq!(peer1, peer2);
    }

    #[test]
    fn test_delivery_state_equality() {
        assert_eq!(DeliveryState::Success, DeliveryState::Success);
        assert_eq!(DeliveryState::Queued, DeliveryState::Queued);
        assert_eq!(DeliveryState::Failed, DeliveryState::Failed);

        assert_eq!(
            DeliveryState::Retry {
                attempt: 1,
                next_retry_ms: 1000
            },
            DeliveryState::Retry {
                attempt: 1,
                next_retry_ms: 1000
            }
        );
    }

    #[tokio::test]
    async fn test_add_peer() {
        let transport = Transport::new();

        let peer = Peer {
            id: "bob".to_string(),
            addr: "192.168.1.1:8080".to_string(),
            public_key: vec![4, 5, 6],
        };

        transport.add_peer(peer.clone()).await.expect("Failed to add peer");

        let peers = transport.peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, "bob");
    }

    #[tokio::test]
    async fn test_add_duplicate_peer_updates() {
        let transport = Transport::new();

        let peer1 = Peer {
            id: "alice".to_string(),
            addr: "127.0.0.1:8080".to_string(),
            public_key: vec![1, 2, 3],
        };

        transport.add_peer(peer1).await.expect("Failed to add peer");

        // Add peer with same ID but different address
        let peer2 = Peer {
            id: "alice".to_string(),
            addr: "127.0.0.1:9090".to_string(),
            public_key: vec![1, 2, 3],
        };

        transport.add_peer(peer2).await.expect("Failed to update peer");

        let peers = transport.peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].addr, "127.0.0.1:9090"); // Should be updated
    }

    #[tokio::test]
    async fn test_remove_peer() {
        let transport = Transport::new();

        let peer = Peer {
            id: "charlie".to_string(),
            addr: "10.0.0.1:8080".to_string(),
            public_key: vec![7, 8, 9],
        };

        transport.add_peer(peer).await.expect("Failed to add peer");
        assert_eq!(transport.peers().await.len(), 1);

        transport.remove_peer("charlie").await.expect("Failed to remove peer");
        assert_eq!(transport.peers().await.len(), 0);
    }

    #[tokio::test]
    async fn test_get_peer() {
        let transport = Transport::new();

        let peer = Peer {
            id: "dave".to_string(),
            addr: "172.16.0.1:8080".to_string(),
            public_key: vec![10, 11, 12],
        };

        transport.add_peer(peer.clone()).await.expect("Failed to add peer");

        let retrieved = transport.get_peer("dave").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "dave");

        let missing = transport.get_peer("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_start_transport() {
        let mut transport = Transport::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap(); // Use port 0 for auto-assignment

        transport.start(addr).await.expect("Failed to start transport");

        assert!(transport.local_addr().is_some());

        // Give the server a moment to start
        sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_message_handler_callback() {
        let mut transport = Transport::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        // Create a counter to verify the handler is called
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Set up message handler
        transport
            .set_message_handler(move |_envelope| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        transport.start(addr).await.expect("Failed to start transport");

        let local_addr = transport.local_addr().unwrap();

        // Give server time to start
        sleep(Duration::from_millis(100)).await;

        // Create a test message envelope
        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let recipient_keypair = KeyPair::generate().expect("Failed to generate recipient keypair");

        let envelope = MessageEnvelope::new(&keypair.uid, &recipient_keypair.uid, b"test message".to_vec());

        // Send a message to the transport
        let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();

        let url = format!("http://{}/output", local_addr);
        let payload = envelope.to_cbor().expect("Failed to serialize");

        let req = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", "application/cbor")
            .body(Full::new(Bytes::from(payload)))
            .expect("Failed to build request");

        let response = client.request(req).await.expect("Failed to send request");
        assert!(response.status().is_success());

        // Give handler time to process
        sleep(Duration::from_millis(100)).await;

        // Verify the handler was called
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_send_message_to_nonexistent_peer() {
        let transport = Transport::new();

        let keypair = KeyPair::generate().expect("Failed to generate keypair");
        let recipient_keypair = KeyPair::generate().expect("Failed to generate recipient keypair");

        let envelope = MessageEnvelope::new(&keypair.uid, &recipient_keypair.uid, b"test".to_vec());

        // Try to send to a peer that doesn't exist
        let result = transport.send("127.0.0.1:9999", &envelope).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_post_output_endpoint_with_invalid_data() {
        let mut transport = Transport::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        transport.start(addr).await.expect("Failed to start transport");
        let local_addr = transport.local_addr().unwrap();

        sleep(Duration::from_millis(100)).await;

        // Send invalid data
        let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();

        let url = format!("http://{}/output", local_addr);
        let req = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .body(Full::new(Bytes::from("invalid cbor data")))
            .expect("Failed to build request");

        let response = client.request(req).await.expect("Failed to send request");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_unsupported_endpoint() {
        let mut transport = Transport::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        transport.start(addr).await.expect("Failed to start transport");
        let local_addr = transport.local_addr().unwrap();

        sleep(Duration::from_millis(100)).await;

        let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();

        // Try GET request (unsupported)
        let url = format!("http://{}/output", local_addr);
        let req = Request::builder()
            .method(Method::GET)
            .uri(&url)
            .body(Full::new(Bytes::new()))
            .expect("Failed to build request");

        let response = client.request(req).await.expect("Failed to send request");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_roundtrip_message_delivery() {
        // Set up sender
        let mut sender_transport = Transport::new();
        let sender_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        sender_transport
            .start(sender_addr)
            .await
            .expect("Failed to start sender");

        // Set up receiver
        let mut receiver_transport = Transport::new();
        let receiver_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received_messages.clone();

        receiver_transport
            .set_message_handler(move |envelope| {
                let clone = received_clone.clone();
                tokio::spawn(async move {
                    clone.lock().await.push(envelope);
                });
            })
            .await;

        receiver_transport
            .start(receiver_addr)
            .await
            .expect("Failed to start receiver");

        let receiver_local_addr = receiver_transport.local_addr().unwrap();

        sleep(Duration::from_millis(100)).await;

        // Create and send message
        let sender_keypair = KeyPair::generate().expect("Failed to generate keypair");
        let recipient_keypair = KeyPair::generate().expect("Failed to generate recipient keypair");
        let payload = b"Hello, peer!".to_vec();

        let envelope = MessageEnvelope::new(&sender_keypair.uid, &recipient_keypair.uid, payload.clone());

        let result = sender_transport
            .send(&receiver_local_addr.to_string(), &envelope)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DeliveryState::Success);

        // Wait for message to be processed
        sleep(Duration::from_millis(200)).await;

        // Verify message was received
        let messages = received_messages.lock().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].from_uid, sender_keypair.uid.as_str());
        assert_eq!(messages[0].to_uid, recipient_keypair.uid.as_str());
        assert_eq!(messages[0].payload, payload);
    }

    #[test]
    fn test_log_delivery_state() {
        // Just ensure the logging function doesn't panic
        log_delivery_state("127.0.0.1:8080", &DeliveryState::Success);
        log_delivery_state("127.0.0.1:8080", &DeliveryState::Queued);
        log_delivery_state(
            "127.0.0.1:8080",
            &DeliveryState::Retry {
                attempt: 2,
                next_retry_ms: 5000,
            },
        );
        log_delivery_state("127.0.0.1:8080", &DeliveryState::Failed);
    }
}
