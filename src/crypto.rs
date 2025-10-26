//! Cryptographic operations module
//!
//! This module handles all cryptographic operations including:
//! - Key generation and management
//! - Message encryption/decryption
//! - Digital signatures
//! - Key exchange protocols

use crate::{Error, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use x25519_dalek::PublicKey as X25519PublicKey;

/// Unique identifier derived from public key fingerprint
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Encrypted message envelope containing ciphertext, nonce, and authentication tag
///
/// This structure represents an AEAD (Authenticated Encryption with Associated Data)
/// encrypted message using XChaCha20-Poly1305. The authentication tag is embedded
/// in the ciphertext by the AEAD cipher.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    /// 24-byte nonce for XChaCha20-Poly1305
    pub nonce: [u8; 24],
    /// Encrypted data + 16-byte Poly1305 authentication tag (appended)
    pub ciphertext: Vec<u8>,
}

/// Encrypt a message using XChaCha20-Poly1305 AEAD
///
/// This function encrypts plaintext using the provided shared secret and returns
/// an `EncryptedEnvelope` containing the ciphertext, nonce, and authentication tag.
///
/// # Arguments
///
/// * `secret` - 32-byte shared secret (typically from X25519 key exchange)
/// * `plaintext` - Data to encrypt
///
/// # Returns
///
/// An `EncryptedEnvelope` containing:
/// - `nonce`: 24-byte random nonce
/// - `ciphertext`: Encrypted data with embedded 16-byte Poly1305 authentication tag
///
/// # Example
///
/// ```
/// use pure2p::crypto::{encrypt_message, decrypt_message};
///
/// # fn example() -> pure2p::Result<()> {
/// let secret = [0u8; 32]; // In practice, use a real shared secret
/// let plaintext = b"Hello, World!";
///
/// let envelope = encrypt_message(&secret, plaintext)?;
/// let decrypted = decrypt_message(&secret, &envelope)?;
///
/// assert_eq!(plaintext, decrypted.as_slice());
/// # Ok(())
/// # }
/// ```
pub fn encrypt_message(secret: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedEnvelope> {
    use rand::RngCore;

    // Create cipher from the shared secret
    let cipher = XChaCha20Poly1305::new(secret.into());

    // Generate a random 24-byte nonce
    let mut nonce_bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    // Encrypt the plaintext
    // The encrypt method automatically appends the 16-byte Poly1305 tag
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| Error::Crypto(format!("Encryption failed: {}", e)))?;

    Ok(EncryptedEnvelope {
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// Decrypt a message using XChaCha20-Poly1305 AEAD
///
/// This function decrypts an `EncryptedEnvelope` using the provided shared secret
/// and verifies the authentication tag. If the tag verification fails, this indicates
/// the message was tampered with or corrupted.
///
/// # Arguments
///
/// * `secret` - 32-byte shared secret (must match the one used for encryption)
/// * `envelope` - The encrypted envelope to decrypt
///
/// # Returns
///
/// The decrypted plaintext, or an error if:
/// - The authentication tag verification fails (data was tampered with)
/// - The ciphertext is malformed
///
/// # Example
///
/// ```
/// use pure2p::crypto::{encrypt_message, decrypt_message};
///
/// # fn example() -> pure2p::Result<()> {
/// let secret = [0u8; 32];
/// let plaintext = b"Secret message";
///
/// let envelope = encrypt_message(&secret, plaintext)?;
/// let decrypted = decrypt_message(&secret, &envelope)?;
///
/// assert_eq!(plaintext, decrypted.as_slice());
/// # Ok(())
/// # }
/// ```
pub fn decrypt_message(secret: &[u8; 32], envelope: &EncryptedEnvelope) -> Result<Vec<u8>> {
    // Create cipher from the shared secret
    let cipher = XChaCha20Poly1305::new(secret.into());

    // Convert nonce
    let nonce = XNonce::from(envelope.nonce);

    // Decrypt and verify the authentication tag
    // The decrypt method automatically verifies the 16-byte Poly1305 tag
    let plaintext = cipher
        .decrypt(&nonce, envelope.ciphertext.as_ref())
        .map_err(|e| Error::Crypto(format!("Decryption failed (auth tag mismatch or corrupted data): {}", e)))?;

    Ok(plaintext)
}

/// Sign contact token data using Ed25519
///
/// This function creates a digital signature for contact token data to ensure
/// integrity and authenticity. The signature proves that the token was created
/// by the holder of the private key and hasn't been tampered with.
///
/// # Arguments
///
/// * `privkey` - 32-byte Ed25519 private key
/// * `token_data` - The data to be signed (typically serialized contact information)
///
/// # Returns
///
/// A 64-byte Ed25519 signature
///
/// # Example
///
/// ```no_run
/// use pure2p::crypto::sign_contact_token;
///
/// // In practice, you'd get the private key from a KeyPair
/// let privkey = [0u8; 32]; // Your Ed25519 private key
/// let token_data = b"contact token data";
///
/// let signature = sign_contact_token(&privkey, token_data)
///     .expect("Failed to sign token");
/// assert_eq!(signature.len(), 64); // Ed25519 signatures are 64 bytes
/// ```
pub fn sign_contact_token(privkey: &[u8; 32], token_data: &[u8]) -> Result<[u8; 64]> {
    let signing_key = SigningKey::from_bytes(privkey);
    let signature = signing_key.sign(token_data);
    Ok(signature.to_bytes())
}

/// Verify contact token signature using Ed25519
///
/// This function verifies that a contact token signature is valid and that the
/// token data hasn't been tampered with. It ensures the token was created by
/// the holder of the private key corresponding to the given public key.
///
/// # Arguments
///
/// * `pubkey` - 32-byte Ed25519 public key
/// * `token_data` - The data that was signed
/// * `sig` - 64-byte Ed25519 signature to verify
///
/// # Returns
///
/// `true` if the signature is valid, `false` otherwise
///
/// # Example
///
/// ```no_run
/// use pure2p::crypto::{sign_contact_token, verify_contact_token};
///
/// // In practice, you'd get keys from a KeyPair
/// let privkey = [0u8; 32]; // Your Ed25519 private key
/// let pubkey = [1u8; 32];  // Corresponding public key
/// let token_data = b"contact token data";
///
/// let signature = sign_contact_token(&privkey, token_data)
///     .expect("Failed to sign");
///
/// // Verify signature
/// let is_valid = verify_contact_token(&pubkey, token_data, &signature)
///     .expect("Failed to verify");
/// println!("Signature valid: {}", is_valid);
/// ```
pub fn verify_contact_token(pubkey: &[u8; 32], token_data: &[u8], sig: &[u8; 64]) -> Result<bool> {
    let verifying_key = VerifyingKey::from_bytes(pubkey)
        .map_err(|e| Error::Crypto(format!("Invalid public key: {}", e)))?;

    let signature = Signature::from_bytes(sig);

    Ok(verifying_key.verify(token_data, &signature).is_ok())
}

/// Encrypt data using a shared secret (legacy function)
pub fn encrypt(_data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
    // TODO: Implement encryption using x25519-dalek and ring
    Err(Error::Crypto("Not yet implemented - use encrypt_message instead".to_string()))
}

/// Decrypt data using a shared secret (legacy function)
pub fn decrypt(_data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
    // TODO: Implement decryption - use decrypt_message instead
    Err(Error::Crypto("Not yet implemented - use decrypt_message instead".to_string()))
}

