//! Cryptographic signing and verification for `umcf_proto::WireEnvelope`.
//! Enforces sender DID binding and bit-accurate tamper resistance.

use crate::did::DidKey;
use crate::error::IdentityError;
use crate::keys::NodeKeypair;
use ed25519_dalek::VerifyingKey;
use umcf_proto::envelope::WireEnvelope;

/// Signs a `WireEnvelope` using the node's private key.
/// Binds the `SenderDid` to the keypair's `did:key:z6Mk...` identifier.
pub fn sign_envelope(
    keypair: &NodeKeypair,
    envelope: &mut WireEnvelope,
) -> Result<(), IdentityError> {
    let expected_did = keypair.did_key();

    if envelope.sender_node_id.trim().is_empty() {
        envelope.sender_node_id = expected_did;
    } else if envelope.sender_node_id != expected_did {
        return Err(IdentityError::SenderMismatch {
            expected: expected_did,
            actual: envelope.sender_node_id.clone(),
        });
    }

    // Zero the signature slot before computing bytes to sign
    envelope.signature = [0u8; 64];

    // Compute canonical signable envelope bytes (everything except the trailing 64-byte signature slot)
    let mut signable_bytes = envelope.to_bytes();
    if signable_bytes.len() < 64 {
        return Err(IdentityError::EnvelopeTampered);
    }
    signable_bytes.truncate(signable_bytes.len() - 64);

    let signature = keypair.sign(&signable_bytes);
    envelope.signature = signature;

    Ok(())
}

/// Verifies that a `WireEnvelope` was signed by its advertised `SenderDid` and has not been tampered with.
pub fn verify_envelope(envelope: &WireEnvelope) -> Result<(), IdentityError> {
    let sender_did = envelope.sender_node_id.as_str();

    // Extract Ed25519 public key from the W3C did:key
    let pk_bytes = DidKey::to_ed25519_bytes(sender_did)?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|e| IdentityError::InvalidPublicKey(e.to_string()))?;

    // Compute canonical signable bytes using a replica with zeroed signature
    let mut replica = envelope.clone();
    replica.signature = [0u8; 64];
    let mut signable_bytes = replica.to_bytes();
    if signable_bytes.len() < 64 {
        return Err(IdentityError::EnvelopeTampered);
    }
    signable_bytes.truncate(signable_bytes.len() - 64);

    NodeKeypair::verify_with_key(&verifying_key, &signable_bytes, &envelope.signature)
}
