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

/// Ping request structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PingRequest {
    /// Contact token of the sender (base64 CBOR with signature)
    /// This allows the receiver to automatically import the sender
    pub contact_token: String,
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

/// Callback type for handling received ping requests
pub type PingHandler = Arc<dyn Fn(String) + Send + Sync>;

/// Network transport layer
#[derive(Clone)]
pub struct Transport {
    /// Local binding address
    local_addr: Option<SocketAddr>,
    /// Known peers
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Message handler callback (legacy - for /output endpoint)
    message_handler: Arc<Mutex<Option<MessageHandler>>>,
    /// New message handler callback (for /message endpoint)
    pub(crate) new_message_handler: Arc<Mutex<Option<NewMessageHandler>>>,
    /// Ping handler callback (for /ping endpoint)
    pub(crate) ping_handler: Arc<Mutex<Option<PingHandler>>>,
    /// HTTP client for sending messages
    client: Client<HttpConnector, Full<Bytes>>,
    /// Local UID for ping responses
    pub(crate) local_uid: Arc<Mutex<Option<String>>>,
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
            ping_handler: Arc::new(Mutex::new(None)),
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

    /// Set the ping handler callback (for /ping endpoint)
    ///
    /// This handler receives the sender's UID when a ping request is received.
    /// The handler can create a chat or perform other actions based on the ping.
    pub async fn set_ping_handler<F>(&self, handler: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let mut guard = self.ping_handler.lock().await;
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
        let ping_handler = self.ping_handler.clone();
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
                        let ping_h = ping_handler.clone();
                        let uid = local_uid.clone();

                        tokio::spawn(async move {
                            let service = service_fn(move |req| {
                                handle_request(req, handler.clone(), new_handler.clone(), ping_h.clone(), uid.clone())
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
    /// * `my_contact_token` - Signed contact token to send (allows receiver to auto-import sender)
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
    ///     vec![99u8; 32], // x25519_pubkey
    ///     Utc::now() + Duration::days(30),
    /// );
    ///
    /// // Send ping with your contact token (empty string for legacy/test usage)
    /// match transport.send_ping(&contact, "").await {
    ///     Ok(response) => println!("Ping OK: {} - {}", response.uid, response.status),
    ///     Err(e) => println!("Ping failed: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_ping(&self, contact: &crate::storage::Contact, my_contact_token: &str) -> Result<PingResponse> {
        info!("Sending ping to {} at {}", contact.uid, contact.ip);

        // Create ping request with sender's contact token (allows receiver to auto-import)
        let ping_request = PingRequest {
            contact_token: my_contact_token.to_string(),
        };

        // Serialize to CBOR
        let ping_body = serde_cbor::to_vec(&ping_request)
            .map_err(|e| Error::CborSerialization(format!("Failed to serialize ping request: {}", e)))?;

        // Construct the POST request to /ping
        let url = format!("http://{}/ping", contact.ip);

        let req = Request::builder()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", "application/cbor")
            .body(Full::new(Bytes::from(ping_body)))
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
    ///     vec![99u8; 32], // x25519_pubkey
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
    ping_handler: Arc<Mutex<Option<PingHandler>>>,
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

            // Read the body to get sender's UID
            let body = req.collect().await?.to_bytes();

            // Deserialize the ping request from CBOR
            match serde_cbor::from_slice::<PingRequest>(&body) {
                Ok(ping_req) => {
                    info!("Received ping with contact token");

                    // Call the ping handler if set (to auto-import sender and create chat)
                    let handler_guard = ping_handler.lock().await;
                    if let Some(handler) = handler_guard.as_ref() {
                        handler(ping_req.contact_token.clone());
                    } else {
                        warn!("No ping handler set");
                    }
                    drop(handler_guard);

                    // Get local UID for response
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
                Err(e) => {
                    error!("Failed to deserialize ping request: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Full::new(Bytes::from(format!(
                            "Invalid ping format: {}",
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
        (&Method::GET, "/health") => {
            debug!("Received GET /health request");

            // Simple health check endpoint for external connectivity verification
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(Full::new(Bytes::from("ok")))
                .unwrap())
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
