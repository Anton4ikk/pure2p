//! Cryptographic operations module
//!
//! This module handles all cryptographic operations including:
//! - Key generation and management
//! - Message encryption/decryption
//! - Digital signatures
//! - Key exchange protocols

use crate::{Error, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ring::digest::{Context, SHA256};

/// Unique identifier derived from public key fingerprint
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UID(String);

impl UID {
    /// Create a UID from a public key by computing its SHA-256 fingerprint
    pub fn from_public_key(public_key: &[u8]) -> Self {
        let mut context = Context::new(&SHA256);
        context.update(public_key);
        let digest = context.finish();

        // Convert to hex string (first 16 bytes for a 32-character UID)
        let hex = digest.as_ref()[..16]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        UID(hex)
    }

    /// Get the UID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a cryptographic key pair
#[derive(Debug)]
pub struct KeyPair {
    /// Public key
    pub public_key: Vec<u8>,
    /// Private key (should be kept secure)
    pub(crate) private_key: Vec<u8>,
    /// Unique identifier derived from public key
    pub uid: UID,
}

impl KeyPair {
    /// Generate a new Ed25519 key pair
    pub fn generate() -> Result<Self> {
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let private_key = signing_key.to_bytes().to_vec();
        let public_key = verifying_key.to_bytes().to_vec();
        let uid = UID::from_public_key(&public_key);

        Ok(KeyPair {
            public_key,
            private_key,
            uid,
        })
    }

    /// Sign a message with the private key
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let signing_key = SigningKey::from_bytes(
            self.private_key
                .as_slice()
                .try_into()
                .map_err(|_| Error::Crypto("Invalid private key length".to_string()))?,
        );

        let signature = signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    /// Verify a signature with the public key
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        let verifying_key = VerifyingKey::from_bytes(
            self.public_key
                .as_slice()
                .try_into()
                .map_err(|_| Error::Crypto("Invalid public key length".to_string()))?,
        )
        .map_err(|e| Error::Crypto(format!("Invalid public key: {}", e)))?;

        let signature = Signature::from_bytes(
            signature
                .try_into()
                .map_err(|_| Error::Crypto("Invalid signature length".to_string()))?,
        );

        Ok(verifying_key.verify(message, &signature).is_ok())
    }

    /// Get the UID for this keypair
    pub fn uid(&self) -> &UID {
        &self.uid
    }
}

/// Encrypt data using a shared secret
pub fn encrypt(_data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
    // TODO: Implement encryption using x25519-dalek and ring
    Err(Error::Crypto("Not yet implemented".to_string()))
}

/// Decrypt data using a shared secret
pub fn decrypt(_data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
    // TODO: Implement decryption
    Err(Error::Crypto("Not yet implemented".to_string()))
}

