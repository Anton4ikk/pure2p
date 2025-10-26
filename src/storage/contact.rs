//! Contact management and token generation/parsing
//!
//! This module handles:
//! - Contact struct representing peers in the P2P network
//! - Signed contact token generation and verification
//! - Contact expiry and activation management

use crate::{crypto::UID, Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a contact/peer in the P2P network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Unique identifier (derived from Ed25519 public key)
    pub uid: String,
    /// IP address and port (e.g., "192.168.1.100:8080")
    pub ip: String,
    /// Ed25519 public key bytes (for signature verification)
    pub pubkey: Vec<u8>,
    /// X25519 public key bytes (for key exchange)
    pub x25519_pubkey: Vec<u8>,
    /// Expiration timestamp for this contact entry
    pub expiry: DateTime<Utc>,
    /// Whether this contact is currently active
    pub is_active: bool,
}

impl Contact {
    /// Create a new contact
    pub fn new(
        uid: String,
        ip: String,
        pubkey: Vec<u8>,
        x25519_pubkey: Vec<u8>,
        expiry: DateTime<Utc>,
    ) -> Self {
        Self {
            uid,
            ip,
            pubkey,
            x25519_pubkey,
            expiry,
            is_active: true, // New contacts are active by default
        }
    }

    /// Check if the contact has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiry
    }

    /// Activate this contact
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Deactivate this contact
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

/// Internal struct for contact token serialization (without signature)
#[derive(Debug, Serialize, Deserialize)]
struct ContactTokenPayload {
    ip: String,
    pubkey: Vec<u8>,
    x25519_pubkey: Vec<u8>,
    expiry: DateTime<Utc>,
}

/// Contact token with signature for integrity verification
#[derive(Debug, Serialize, Deserialize)]
struct ContactTokenData {
    payload: ContactTokenPayload,
    signature: Vec<u8>, // 64-byte Ed25519 signature
}

/// Generate a signed contact token from IP, public keys, private key, and expiry
///
/// The token is serialized using CBOR, signed with Ed25519, and encoded as base64 URL-safe without padding.
/// The signature ensures token integrity and authenticity.
///
/// # Arguments
/// * `ip` - IP address and port (e.g., "192.168.1.100:8080")
/// * `pubkey` - Ed25519 public key bytes (for signature verification)
/// * `privkey` - Ed25519 private key bytes (for signing the token)
/// * `x25519_pubkey` - X25519 public key bytes (for key exchange)
/// * `expiry` - Expiration timestamp
///
/// # Returns
/// A base64-encoded signed contact token string
///
/// # Errors
/// Returns an error if signing fails
pub fn generate_contact_token(
    ip: &str,
    pubkey: &[u8],
    privkey: &[u8],
    x25519_pubkey: &[u8],
    expiry: DateTime<Utc>,
) -> Result<String> {
    let payload = ContactTokenPayload {
        ip: ip.to_string(),
        pubkey: pubkey.to_vec(),
        x25519_pubkey: x25519_pubkey.to_vec(),
        expiry,
    };

    // Serialize payload to CBOR (this is what gets signed)
    let payload_cbor = serde_cbor::to_vec(&payload)
        .map_err(|e| Error::CborSerialization(format!("Failed to serialize token payload: {}", e)))?;

    // Sign the payload
    let privkey_array: [u8; 32] = privkey.try_into()
        .map_err(|_| Error::Crypto("Invalid private key length (expected 32 bytes)".to_string()))?;
    let signature = crate::crypto::sign_contact_token(&privkey_array, &payload_cbor)?;

    // Create token with signature
    let token_data = ContactTokenData {
        payload,
        signature: signature.to_vec(),
    };

    // Serialize complete token (payload + signature) to CBOR
    let cbor = serde_cbor::to_vec(&token_data)
        .map_err(|e| Error::CborSerialization(format!("Failed to serialize contact token: {}", e)))?;

    // Encode as base64 URL-safe
    Ok(URL_SAFE_NO_PAD.encode(cbor))
}

/// Parse a contact token, verify signature, and validate expiry
///
/// Decodes a base64 URL-safe token, deserializes CBOR data, verifies the Ed25519 signature,
/// and validates the expiry. This ensures the token is authentic and hasn't been tampered with.
///
/// # Arguments
/// * `token` - Base64-encoded signed contact token string
///
/// # Returns
/// A `Contact` struct if the token is valid, signature is correct, and not expired
///
/// # Errors
/// Returns an error if:
/// - Token decoding fails
/// - CBOR deserialization fails
/// - Signature verification fails (invalid or tampered token)
/// - Contact has expired
pub fn parse_contact_token(token: &str) -> Result<Contact> {
    // Decode from base64
    let cbor = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|e| Error::Storage(format!("Invalid base64 token: {}", e)))?;

    // Deserialize from CBOR
    let data: ContactTokenData = serde_cbor::from_slice(&cbor)
        .map_err(|e| Error::CborSerialization(format!("Invalid token data: {}", e)))?;

    // Verify signature length
    if data.signature.len() != 64 {
        return Err(Error::Crypto(format!(
            "Invalid signature length: expected 64 bytes, got {}",
            data.signature.len()
        )));
    }

    // Re-serialize payload to verify signature
    let payload_cbor = serde_cbor::to_vec(&data.payload)
        .map_err(|e| Error::CborSerialization(format!("Failed to serialize payload for verification: {}", e)))?;

    // Verify signature
    let pubkey_array: [u8; 32] = data.payload.pubkey.as_slice().try_into()
        .map_err(|_| Error::Crypto("Invalid public key length (expected 32 bytes)".to_string()))?;
    let signature_array: [u8; 64] = data.signature.as_slice().try_into()
        .map_err(|_| Error::Crypto("Invalid signature length (expected 64 bytes)".to_string()))?;

    let is_valid = crate::crypto::verify_contact_token(&pubkey_array, &payload_cbor, &signature_array)?;
    if !is_valid {
        return Err(Error::Crypto("Contact token signature verification failed (token may be tampered with)".to_string()));
    }

    // Validate expiry
    if Utc::now() > data.payload.expiry {
        return Err(Error::Storage("Contact token has expired".to_string()));
    }

    // Generate UID from Ed25519 public key
    let uid = UID::from_public_key(&data.payload.pubkey);

    // Create contact
    Ok(Contact::new(
        uid.to_string(),
        data.payload.ip,
        data.payload.pubkey,
        data.payload.x25519_pubkey,
        data.payload.expiry,
    ))
}
