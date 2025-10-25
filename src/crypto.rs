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
use x25519_dalek::PublicKey as X25519PublicKey;

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
    /// Ed25519 public key (for signing/verification)
    pub public_key: Vec<u8>,
    /// Ed25519 private key (should be kept secure)
    pub(crate) private_key: Vec<u8>,
    /// X25519 public key (for key exchange)
    pub x25519_public: Vec<u8>,
    /// X25519 private key (should be kept secure)
    pub(crate) x25519_secret: Vec<u8>,
    /// Unique identifier derived from Ed25519 public key
    pub uid: UID,
}

impl KeyPair {
    /// Generate a new key pair (Ed25519 for signing + X25519 for key exchange)
    pub fn generate() -> Result<Self> {
        use rand::rngs::OsRng;

        // Generate Ed25519 keypair for signing/verification
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let private_key = signing_key.to_bytes().to_vec();
        let public_key = verifying_key.to_bytes().to_vec();
        let uid = UID::from_public_key(&public_key);

        // Generate X25519 keypair for key exchange
        // Generate 32 random bytes for the secret key
        let mut x25519_secret_bytes = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut x25519_secret_bytes);

        // Derive public key from secret: public = basepoint * secret
        let x25519_public_bytes = x25519_dalek::x25519(x25519_secret_bytes, x25519_dalek::X25519_BASEPOINT_BYTES);
        let x25519_public = X25519PublicKey::from(x25519_public_bytes);

        Ok(KeyPair {
            public_key,
            private_key,
            x25519_public: x25519_public.to_bytes().to_vec(),
            x25519_secret: x25519_secret_bytes.to_vec(),
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

    /// Derive a shared secret with a remote peer's X25519 public key
    pub fn derive_shared_secret(&self, remote_x25519_public: &[u8; 32]) -> Result<[u8; 32]> {
        let local_secret_bytes: [u8; 32] = self.x25519_secret.as_slice()
            .try_into()
            .map_err(|_| Error::Crypto("Invalid X25519 secret key length".to_string()))?;

        let remote_public = X25519PublicKey::from(*remote_x25519_public);

        // Perform the scalar multiplication: local_secret * remote_public
        let shared_secret = diffie_hellman(&local_secret_bytes, remote_public.as_bytes());

        Ok(shared_secret)
    }
}

/// Internal helper: Perform X25519 Diffie-Hellman
fn diffie_hellman(secret: &[u8; 32], public: &[u8; 32]) -> [u8; 32] {
    // x25519 function performs the scalar multiplication with proper clamping
    x25519_dalek::x25519(*secret, *public)
}

/// Derive a shared secret using X25519 ECDH
///
/// This function performs Diffie-Hellman key exchange using the X25519 elliptic curve.
/// The resulting shared secret can be used as a base key for AEAD encryption.
///
/// # Arguments
///
/// * `local_priv` - Local X25519 private key (32 bytes)
/// * `remote_pub` - Remote X25519 public key (32 bytes)
///
/// # Returns
///
/// A 32-byte shared secret derived from the key exchange
///
/// # Example
///
/// ```
/// use pure2p::crypto::derive_shared_secret;
///
/// # fn example() -> pure2p::Result<()> {
/// let alice_secret = [1u8; 32];
/// let bob_public = [2u8; 32];
///
/// let shared_secret = derive_shared_secret(&alice_secret, &bob_public);
/// assert_eq!(shared_secret.len(), 32);
/// # Ok(())
/// # }
/// ```
pub fn derive_shared_secret(local_priv: &[u8; 32], remote_pub: &[u8; 32]) -> [u8; 32] {
    diffie_hellman(local_priv, remote_pub)
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

