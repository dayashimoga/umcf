//! Cryptographic device keypair implementation supporting Ed25519 signing and X25519 key agreement.
//! Strictly enforces `#![forbid(unsafe_code)]` and zeroize-on-drop memory cleanup.

use crate::did::DidKey;
use crate::error::IdentityError;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use umcf_core::NodeId;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Domain separation prefix prepended to all UMCF signatures to prevent cross-protocol replay attacks.
pub const DOMAIN_SEPARATION_PREFIX: &[u8] = b"UMCF-SIG-v1:";

/// Context string for deterministic X25519 key derivation from Ed25519 seed.
const X25519_DERIVATION_CONTEXT: &str = "UMCF-X25519-DERIVATION-v1";

/// Hardware-capable cryptographic node keypair.
/// Holds an Ed25519 signing key for message authentication and an X25519 static secret for key agreement.
pub struct NodeKeypair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    x25519_secret: X25519Secret,
    x25519_public: X25519Public,
    seed: [u8; 32],
}

impl Drop for NodeKeypair {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

impl ZeroizeOnDrop for NodeKeypair {}

impl NodeKeypair {
    /// Generates a new cryptographically secure keypair using the operating system CSPRNG (`OsRng`).
    #[must_use]
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        rand_core::RngCore::fill_bytes(&mut OsRng, &mut seed);
        Self::from_seed(&seed)
    }

    /// Deterministically derives a `NodeKeypair` from a 32-byte seed.
    ///
    /// Derivation details:
    /// - Ed25519 SigningKey is initialized directly from the seed.
    /// - X25519 StaticSecret is derived using BLAKE3 key derivation with context `UMCF-X25519-DERIVATION-v1`
    ///   and clamped according to RFC 7748.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();

        // Deterministic X25519 derivation using BLAKE3 KDF
        let mut x25519_scalar = blake3::derive_key(X25519_DERIVATION_CONTEXT, seed);
        // RFC 7748 scalar clamping
        x25519_scalar[0] &= 248;
        x25519_scalar[31] &= 127;
        x25519_scalar[31] |= 64;

        let x25519_secret = X25519Secret::from(x25519_scalar);
        let x25519_public = X25519Public::from(&x25519_secret);

        x25519_scalar.zeroize();

        Self {
            signing_key,
            verifying_key,
            x25519_secret,
            x25519_public,
            seed: *seed,
        }
    }

    /// Signs a message with domain separation prefix (`UMCF-SIG-v1:`).
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let mut payload = Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + message.len());
        payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        payload.extend_from_slice(message);

        let signature: Signature = self.signing_key.sign(&payload);
        signature.to_bytes()
    }

    /// Verifies a domain-separated signature against a provided message using this keypair's public key.
    pub fn verify(&self, message: &[u8], signature_bytes: &[u8; 64]) -> Result<(), IdentityError> {
        Self::verify_with_key(&self.verifying_key, message, signature_bytes)
    }

    /// Verifies a domain-separated signature using an arbitrary Ed25519 verifying key.
    pub fn verify_with_key(
        verifying_key: &VerifyingKey,
        message: &[u8],
        signature_bytes: &[u8; 64],
    ) -> Result<(), IdentityError> {
        let mut payload = Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + message.len());
        payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        payload.extend_from_slice(message);

        let sig = Signature::from_bytes(signature_bytes);
        verifying_key
            .verify_strict(&payload, &sig)
            .map_err(|_| IdentityError::InvalidSignature)
    }

    /// Performs X25519 Diffie-Hellman key agreement with a remote peer's public key.
    /// Returns the 32-byte shared secret.
    #[must_use]
    pub fn diffie_hellman(&self, peer_public: &X25519Public) -> [u8; 32] {
        self.x25519_secret.diffie_hellman(peer_public).to_bytes()
    }

    /// Returns the Ed25519 verifying key.
    #[must_use]
    pub const fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Returns the raw 32-byte Ed25519 public key.
    #[must_use]
    pub fn ed25519_public_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Returns the X25519 public key for key agreement.
    #[must_use]
    pub const fn x25519_public(&self) -> &X25519Public {
        &self.x25519_public
    }

    /// Returns the raw 32-byte X25519 public key.
    #[must_use]
    pub fn x25519_public_bytes(&self) -> [u8; 32] {
        *self.x25519_public.as_bytes()
    }

    /// Computes the canonical UMCF `NodeId` from this keypair's public key.
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        NodeId::from_public_key_bytes(&self.ed25519_public_bytes()).unwrap_or_else(|_| {
            NodeId::parse("did:umcf:00000000000000000000000000000000").expect("fallback node id")
        })
    }

    /// Returns the W3C multicodec `did:key:z6Mk...` identifier for this keypair.
    #[must_use]
    pub fn did_key(&self) -> String {
        DidKey::from_ed25519_bytes(&self.ed25519_public_bytes())
    }
}

impl std::fmt::Debug for NodeKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeKeypair")
            .field("did_key", &self.did_key())
            .field("node_id", &self.node_id())
            .field("ed25519_public", &hex::encode(self.ed25519_public_bytes()))
            .field("x25519_public", &hex::encode(self.x25519_public_bytes()))
            .finish_non_exhaustive()
    }
}

mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}
