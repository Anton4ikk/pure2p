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

/// Ping response structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PingResponse {
    /// UID of the responding peer
    pub uid: String,
    /// Status message
    pub status: String,
}

/// Message request structure for /message endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageRequest {
    /// UID of the sender
    pub from_uid: String,
    /// Type of message (e.g., "text", "delete", "typing", etc.)
    pub message_type: String,
    /// Message payload (arbitrary bytes)
    pub payload: Vec<u8>,
}

/// Callback type for handling received messages (legacy - for /output endpoint)
pub type MessageHandler = Arc<dyn Fn(MessageEnvelope) + Send + Sync>;

/// Callback type for handling received messages from /message endpoint
pub type NewMessageHandler = Arc<dyn Fn(MessageRequest) + Send + Sync>;

/// Network transport layer
pub struct Transport {
    /// Local binding address
    local_addr: Option<SocketAddr>,
    /// Known peers
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Message handler callback (legacy - for /output endpoint)
    message_handler: Arc<Mutex<Option<MessageHandler>>>,
    /// New message handler callback (for /message endpoint)
    new_message_handler: Arc<Mutex<Option<NewMessageHandler>>>,
    /// HTTP client for sending messages
    client: Client<HttpConnector, Full<Bytes>>,
    /// Local UID for ping responses
    local_uid: Arc<Mutex<Option<String>>>,
}

impl Transport {
    /// Create a new transport instance
    pub fn new() -> Self {
        let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();

        Self {
            local_addr: None,
            peers: Arc::new(Mutex::new(Vec::new())),
            message_handler: Arc::new(Mutex::new(None)),
            new_message_handler: Arc::new(Mutex::new(None)),
            client,
            local_uid: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the local UID for this transport instance
    pub async fn set_local_uid(&self, uid: String) {
        let mut guard = self.local_uid.lock().await;
        *guard = Some(uid);
    }

    /// Set the message handler callback (legacy - for /output endpoint)
    pub async fn set_message_handler<F>(&self, handler: F)
    where
        F: Fn(MessageEnvelope) + Send + Sync + 'static,
    {
        let mut guard = self.message_handler.lock().await;
        *guard = Some(Arc::new(handler));
    }

    /// Set the new message handler callback (for /message endpoint)
    ///
    /// This handler receives MessageRequest objects from the /message endpoint.
    /// The handler is responsible for storing messages in AppState via Chat.
    pub async fn set_new_message_handler<F>(&self, handler: F)
    where
        F: Fn(MessageRequest) + Send + Sync + 'static,
    {
        let mut guard = self.new_message_handler.lock().await;
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
        let new_message_handler = self.new_message_handler.clone();
        let local_uid = self.local_uid.clone();

        // Spawn listener task
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_addr)) => {
                        debug!("Accepted connection from {}", remote_addr);

                        let io = TokioIo::new(stream);
                        let handler = message_handler.clone();
                        let new_handler = new_message_handler.clone();
                        let uid = local_uid.clone();

                        tokio::spawn(async move {
                            let service = service_fn(move |req| {
                                handle_request(req, handler.clone(), new_handler.clone(), uid.clone())
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

    /// Send a ping to a contact to test connectivity
    ///
    /// # Arguments
    /// * `contact` - The contact to ping (uses the Contact struct from storage module)
    ///
    /// # Returns
    /// * `Ok(PingResponse)` - Ping successful, contains UID and status
    /// * `Err(Error)` - Ping failed (network error, timeout, etc.)
    ///
    /// # Example
    /// ```rust,no_run
    /// use pure2p::transport::Transport;
    /// use pure2p::storage::Contact;
    /// use chrono::{Utc, Duration};
    ///
    /// # async fn example() -> pure2p::Result<()> {
    /// let transport = Transport::new();
    /// let contact = Contact::new(
    ///     "alice_uid".to_string(),
    ///     "192.168.1.100:8080".to_string(),
    ///     vec![1, 2, 3],
    ///     Utc::now() + Duration::days(30),
    /// );
    ///
    /// match transport.send_ping(&contact).await {
    ///     Ok(response) => println!("Ping OK: {} - {}", response.uid, response.status),
    ///     Err(e) => println!("Ping failed: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_ping(&self, contact: &crate::storage::Contact) -> Result<PingResponse> {
        info!("Sending ping to {} at {}", contact.uid, contact.ip);

        // Construct the POST request to /ping
        let url = format!("http://{}/ping", contact.ip);

        let req = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", "application/cbor")
            .body(Full::new(Bytes::new()))
            .map_err(|e| Error::Transport(format!("Failed to build ping request: {}", e)))?;

        // Send the request
        match self.client.request(req).await {
            Ok(response) => {
                if response.status().is_success() {
                    // Read the response body
                    let body = response.collect().await
                        .map_err(|e| Error::Transport(format!("Failed to read ping response: {}", e)))?
                        .to_bytes();

                    // Deserialize the ping response from CBOR
                    let ping_response: PingResponse = serde_cbor::from_slice(&body)
                        .map_err(|e| Error::CborSerialization(format!("Failed to deserialize ping response: {}", e)))?;

                    info!("Ping successful: {} - {}", ping_response.uid, ping_response.status);
                    Ok(ping_response)
                } else {
                    warn!("Ping failed with status {}: {}", response.status(), contact.ip);
                    Err(Error::Transport(format!(
                        "Ping failed with status {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                error!("Failed to send ping to {}: {}", contact.ip, e);
                Err(Error::Transport(format!("Ping send failed: {}", e)))
            }
        }
    }

    /// Send a message to a contact via the /message endpoint
    ///
    /// # Arguments
    /// * `contact` - The contact to send the message to
    /// * `from_uid` - The sender's UID
    /// * `message_type` - Type of message (e.g., "text", "delete", "typing")
    /// * `payload` - Message payload as bytes
    ///
    /// # Returns
    /// * `Ok(())` - Message sent successfully
    /// * `Err(Error)` - Failed to send message (caller should queue for retry)
    ///
    /// # Example
    /// ```rust,no_run
    /// use pure2p::transport::Transport;
    /// use pure2p::storage::Contact;
    /// use chrono::{Utc, Duration};
    ///
    /// # async fn example() -> pure2p::Result<()> {
    /// let transport = Transport::new();
    /// let contact = Contact::new(
    ///     "alice_uid".to_string(),
    ///     "192.168.1.100:8080".to_string(),
    ///     vec![1, 2, 3],
    ///     Utc::now() + Duration::days(30),
    /// );
    ///
    /// let payload = b"Hello, Alice!".to_vec();
    /// transport.send_message(&contact, "my_uid", "text", payload).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_message(
        &self,
        contact: &crate::storage::Contact,
        from_uid: &str,
        message_type: &str,
        payload: Vec<u8>,
    ) -> Result<()> {
        info!("Sending {} message to {} at {}", message_type, contact.uid, contact.ip);

        // Create message request
        let msg_req = MessageRequest {
            from_uid: from_uid.to_string(),
            message_type: message_type.to_string(),
            payload,
        };

        // Serialize to CBOR
        let cbor_data = serde_cbor::to_vec(&msg_req)
            .map_err(|e| Error::CborSerialization(format!("Failed to serialize message request: {}", e)))?;

        // Construct the POST request to /message
        let url = format!("http://{}/message", contact.ip);

        let req = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", "application/cbor")
            .body(Full::new(Bytes::from(cbor_data)))
            .map_err(|e| Error::Transport(format!("Failed to build message request: {}", e)))?;

        // Send the request
        match self.client.request(req).await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Message sent successfully to {}", contact.ip);
                    Ok(())
                } else {
                    warn!("Message send failed with status {}: {}", response.status(), contact.ip);
                    Err(Error::Transport(format!(
                        "Message send failed with status {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                error!("Failed to send message to {}: {}", contact.ip, e);
                Err(Error::Transport(format!("Message send failed: {}", e)))
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
    new_message_handler: Arc<Mutex<Option<NewMessageHandler>>>,
    local_uid: Arc<Mutex<Option<String>>>,
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
        (&Method::POST, "/ping") => {
            debug!("Received POST /ping request");

            // Get local UID
            let uid_guard = local_uid.lock().await;
            let uid = uid_guard.as_ref().map(|s| s.clone()).unwrap_or_else(|| "unknown".to_string());
            drop(uid_guard);

            // Create ping response
            let response = PingResponse {
                uid,
                status: "ok".to_string(),
            };

            // Serialize to CBOR
            match serde_cbor::to_vec(&response) {
                Ok(cbor_data) => {
                    info!("Responding to ping");
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/cbor")
                        .body(Full::new(Bytes::from(cbor_data)))
                        .unwrap())
                }
                Err(e) => {
                    error!("Failed to serialize ping response: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Full::new(Bytes::from(format!(
                            "Failed to serialize response: {}",
                            e
                        ))))
                        .unwrap())
                }
            }
        }
        (&Method::POST, "/message") => {
            debug!("Received POST /message request");

            // Read the body
            let body = req.collect().await?.to_bytes();

            // Deserialize the message request from CBOR
            match serde_cbor::from_slice::<MessageRequest>(&body) {
                Ok(msg_req) => {
                    info!(
                        "Received message from {} (type: {})",
                        msg_req.from_uid, msg_req.message_type
                    );

                    // Call the new message handler if set
                    let handler_guard = new_message_handler.lock().await;
                    if let Some(handler) = handler_guard.as_ref() {
                        handler(msg_req);
                    } else {
                        warn!("No message handler set for /message endpoint, message dropped");
                    }

                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::from("Message received")))
                        .unwrap())
                }
                Err(e) => {
                    error!("Failed to deserialize message request: {}", e);
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

    #[test]
    fn test_ping_response_creation() {
        let response = PingResponse {
            uid: "test_uid_123".to_string(),
            status: "ok".to_string(),
        };

        assert_eq!(response.uid, "test_uid_123");
        assert_eq!(response.status, "ok");
    }

    #[test]
    fn test_ping_response_serialization() {
        let response = PingResponse {
            uid: "alice_uid".to_string(),
            status: "ok".to_string(),
        };

        // Serialize to CBOR
        let cbor = serde_cbor::to_vec(&response).expect("Failed to serialize");

        // Deserialize from CBOR
        let deserialized: PingResponse = serde_cbor::from_slice(&cbor).expect("Failed to deserialize");

        assert_eq!(deserialized.uid, response.uid);
        assert_eq!(deserialized.status, response.status);
    }

    #[tokio::test]
    async fn test_set_local_uid() {
        let transport = Transport::new();

        transport.set_local_uid("test_uid_456".to_string()).await;

        let uid_guard = transport.local_uid.lock().await;
        assert_eq!(uid_guard.as_ref().unwrap(), "test_uid_456");
    }

    #[tokio::test]
    async fn test_ping_endpoint() {
        let mut transport = Transport::new();

        // Set local UID
        transport.set_local_uid("server_uid_789".to_string()).await;

        // Start transport on a random port
        let addr = "127.0.0.1:0".parse().unwrap();
        transport.start(addr).await.expect("Failed to start transport");

        // Give the server a moment to start
        sleep(Duration::from_millis(100)).await;

        let local_addr = transport.local_addr().expect("No local address");

        // Create a contact for pinging
        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let contact = Contact::new(
            "server_uid_789".to_string(),
            local_addr.to_string(),
            vec![1, 2, 3],
            Utc::now() + ChronoDuration::days(30),
        );

        // Send ping
        let response = transport.send_ping(&contact).await.expect("Ping failed");

        // Verify response
        assert_eq!(response.uid, "server_uid_789");
        assert_eq!(response.status, "ok");
    }

    #[tokio::test]
    async fn test_ping_unknown_uid() {
        let mut transport = Transport::new();

        // Don't set local UID, should default to "unknown"

        // Start transport on a random port
        let addr = "127.0.0.1:0".parse().unwrap();
        transport.start(addr).await.expect("Failed to start transport");

        sleep(Duration::from_millis(100)).await;

        let local_addr = transport.local_addr().expect("No local address");

        // Create a contact for pinging
        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let contact = Contact::new(
            "any_uid".to_string(),
            local_addr.to_string(),
            vec![1, 2, 3],
            Utc::now() + ChronoDuration::days(30),
        );

        // Send ping
        let response = transport.send_ping(&contact).await.expect("Ping failed");

        // Should respond with "unknown" since UID wasn't set
        assert_eq!(response.uid, "unknown");
        assert_eq!(response.status, "ok");
    }

    #[tokio::test]
    async fn test_ping_unreachable_peer() {
        let transport = Transport::new();

        // Create a contact pointing to a non-existent server
        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let contact = Contact::new(
            "unreachable_uid".to_string(),
            "127.0.0.1:59999".to_string(), // Unlikely to be in use
            vec![1, 2, 3],
            Utc::now() + ChronoDuration::days(30),
        );

        // Ping should fail
        let result = transport.send_ping(&contact).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ping_multiple_peers() {
        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        // Start two transport instances
        let mut transport1 = Transport::new();
        let mut transport2 = Transport::new();

        transport1.set_local_uid("peer1_uid".to_string()).await;
        transport2.set_local_uid("peer2_uid".to_string()).await;

        // Start both transports
        transport1.start("127.0.0.1:0".parse().unwrap()).await.expect("Failed to start transport1");
        transport2.start("127.0.0.1:0".parse().unwrap()).await.expect("Failed to start transport2");

        sleep(Duration::from_millis(100)).await;

        let addr1 = transport1.local_addr().expect("No local address for transport1");
        let addr2 = transport2.local_addr().expect("No local address for transport2");

        // Create contacts
        let contact1 = Contact::new(
            "peer1_uid".to_string(),
            addr1.to_string(),
            vec![1],
            Utc::now() + ChronoDuration::days(30),
        );

        let contact2 = Contact::new(
            "peer2_uid".to_string(),
            addr2.to_string(),
            vec![2],
            Utc::now() + ChronoDuration::days(30),
        );

        // Ping from transport1 to transport2
        let response = transport1.send_ping(&contact2).await.expect("Ping to peer2 failed");
        assert_eq!(response.uid, "peer2_uid");

        // Ping from transport2 to transport1
        let response = transport2.send_ping(&contact1).await.expect("Ping to peer1 failed");
        assert_eq!(response.uid, "peer1_uid");
    }

    #[test]
    fn test_message_request_creation() {
        let msg_req = MessageRequest {
            from_uid: "sender_uid".to_string(),
            message_type: "text".to_string(),
            payload: b"Hello, world!".to_vec(),
        };

        assert_eq!(msg_req.from_uid, "sender_uid");
        assert_eq!(msg_req.message_type, "text");
        assert_eq!(msg_req.payload, b"Hello, world!");
    }

    #[test]
    fn test_message_request_serialization() {
        let msg_req = MessageRequest {
            from_uid: "alice_uid".to_string(),
            message_type: "text".to_string(),
            payload: vec![1, 2, 3, 4, 5],
        };

        // Serialize to CBOR
        let cbor = serde_cbor::to_vec(&msg_req).expect("Failed to serialize");

        // Deserialize from CBOR
        let deserialized: MessageRequest = serde_cbor::from_slice(&cbor).expect("Failed to deserialize");

        assert_eq!(deserialized.from_uid, msg_req.from_uid);
        assert_eq!(deserialized.message_type, msg_req.message_type);
        assert_eq!(deserialized.payload, msg_req.payload);
    }

    #[tokio::test]
    async fn test_set_new_message_handler() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let transport = Transport::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        transport.set_new_message_handler(move |_msg| {
            called_clone.store(true, Ordering::SeqCst);
        }).await;

        // Verify handler is set
        let handler_guard = transport.new_message_handler.lock().await;
        assert!(handler_guard.is_some());

        // Call the handler
        if let Some(handler) = handler_guard.as_ref() {
            let test_msg = MessageRequest {
                from_uid: "test".to_string(),
                message_type: "text".to_string(),
                payload: vec![],
            };
            handler(test_msg);
        }

        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_message_endpoint() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut transport = Transport::new();

        // Set up message handler
        let received = Arc::new(AtomicBool::new(false));
        let received_clone = received.clone();

        transport.set_new_message_handler(move |msg| {
            assert_eq!(msg.from_uid, "sender_uid");
            assert_eq!(msg.message_type, "text");
            assert_eq!(msg.payload, b"Test message");
            received_clone.store(true, Ordering::SeqCst);
        }).await;

        // Start transport
        transport.start("127.0.0.1:0".parse().unwrap()).await.expect("Failed to start transport");
        sleep(Duration::from_millis(100)).await;

        let local_addr = transport.local_addr().expect("No local address");

        // Create a contact
        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let contact = Contact::new(
            "receiver_uid".to_string(),
            local_addr.to_string(),
            vec![1, 2, 3],
            Utc::now() + ChronoDuration::days(30),
        );

        // Send message
        transport.send_message(&contact, "sender_uid", "text", b"Test message".to_vec())
            .await
            .expect("Failed to send message");

        // Give handler time to process
        sleep(Duration::from_millis(100)).await;

        // Verify message was received
        assert!(received.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_message_endpoint_different_types() {
        use std::sync::{Arc, Mutex as StdMutex};

        let mut transport = Transport::new();

        // Track received messages
        let received_messages = Arc::new(StdMutex::new(Vec::new()));
        let received_clone = received_messages.clone();

        transport.set_new_message_handler(move |msg| {
            received_clone.lock().unwrap().push((msg.message_type.clone(), msg.payload.clone()));
        }).await;

        // Start transport
        transport.start("127.0.0.1:0".parse().unwrap()).await.expect("Failed to start transport");
        sleep(Duration::from_millis(100)).await;

        let local_addr = transport.local_addr().expect("No local address");

        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let contact = Contact::new(
            "receiver_uid".to_string(),
            local_addr.to_string(),
            vec![1, 2, 3],
            Utc::now() + ChronoDuration::days(30),
        );

        // Send different message types
        transport.send_message(&contact, "sender", "text", b"Hello".to_vec()).await.expect("Failed to send text");
        transport.send_message(&contact, "sender", "delete", vec![]).await.expect("Failed to send delete");
        transport.send_message(&contact, "sender", "typing", vec![1]).await.expect("Failed to send typing");

        sleep(Duration::from_millis(200)).await;

        let messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].0, "text");
        assert_eq!(messages[0].1, b"Hello");
        assert_eq!(messages[1].0, "delete");
        assert_eq!(messages[2].0, "typing");
    }

    #[tokio::test]
    async fn test_message_unreachable_peer() {
        let transport = Transport::new();

        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let contact = Contact::new(
            "unreachable".to_string(),
            "127.0.0.1:59998".to_string(),
            vec![1, 2, 3],
            Utc::now() + ChronoDuration::days(30),
        );

        // Should fail to send
        let result = transport.send_message(&contact, "sender", "text", b"Test".to_vec()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_bidirectional() {
        use std::sync::{Arc, Mutex as StdMutex};
        use crate::storage::Contact;
        use chrono::{Utc, Duration as ChronoDuration};

        let mut transport1 = Transport::new();
        let mut transport2 = Transport::new();

        let messages1 = Arc::new(StdMutex::new(Vec::new()));
        let messages2 = Arc::new(StdMutex::new(Vec::new()));

        let m1 = messages1.clone();
        let m2 = messages2.clone();

        transport1.set_new_message_handler(move |msg| {
            m1.lock().unwrap().push(msg);
        }).await;

        transport2.set_new_message_handler(move |msg| {
            m2.lock().unwrap().push(msg);
        }).await;

        transport1.start("127.0.0.1:0".parse().unwrap()).await.expect("Failed to start t1");
        transport2.start("127.0.0.1:0".parse().unwrap()).await.expect("Failed to start t2");

        sleep(Duration::from_millis(100)).await;

        let addr1 = transport1.local_addr().unwrap();
        let addr2 = transport2.local_addr().unwrap();

        let contact1 = Contact::new("peer1".to_string(), addr1.to_string(), vec![1], Utc::now() + ChronoDuration::days(30));
        let contact2 = Contact::new("peer2".to_string(), addr2.to_string(), vec![2], Utc::now() + ChronoDuration::days(30));

        // Send from 1 to 2
        transport1.send_message(&contact2, "peer1", "text", b"Hello from 1".to_vec()).await.expect("Send 1->2 failed");

        // Send from 2 to 1
        transport2.send_message(&contact1, "peer2", "text", b"Hello from 2".to_vec()).await.expect("Send 2->1 failed");

        sleep(Duration::from_millis(200)).await;

        // Verify messages received
        let msgs1 = messages1.lock().unwrap();
        let msgs2 = messages2.lock().unwrap();

        assert_eq!(msgs1.len(), 1);
        assert_eq!(msgs1[0].from_uid, "peer2");
        assert_eq!(msgs1[0].payload, b"Hello from 2");

        assert_eq!(msgs2.len(), 1);
        assert_eq!(msgs2[0].from_uid, "peer1");
        assert_eq!(msgs2[0].payload, b"Hello from 1");
    }
}
