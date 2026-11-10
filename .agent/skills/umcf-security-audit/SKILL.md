---
name: umcf-security-audit
description: >-
  Guides cryptographic implementation, security auditing, threat modeling, capability token verification, and privacy compliance for UMCF. Use when implementing or auditing authentication, encryption, QR enrollment, RBAC, FFI boundaries, or dependency security.
---

# UMCF Security, Cryptography & Privacy Audit Skill

Use this skill when designing, implementing, or auditing security-critical subsystems across UMCF (`umcf-crypto`, `umcf-p2p`, `umcf-llama-sys`, `control_plane_worker`, and client apps).

---

## 1. Cryptographic Primitives & Key Hierarchy

Use only audited, constant-time Rust cryptographic crates (`ed25519-dalek`, `x25519-dalek`, `chacha20poly1305` / `aes-gcm`, `blake3`, `sha2`, `argon2`, `rustls`, `zeroize`):

1. **Root Device Identity (`Ed25519`)**:
   - Generated once on first initialization via `OsRng`.
   - `NodeId = "umcf1_" + Base32Lower(BLAKE3(ed25519_pubkey)[0..20])`.
   - Private key bytes wrapped in `Zeroizing<[u8; 32]>` and persisted in platform-native secure storage.
2. **Transport Session Keys (`TLS 1.3 mTLS` / `Noise_XX`)**:
   - Ephemeral X25519 ECDHE key exchange providing forward secrecy.
   - Custom `rustls::client::danger::ServerCertVerifier` and `ClientCertVerifier` implementations that verify self-signed X.509 certificates binding the peer's Ed25519 public key to its `NodeId` against the local Trust Registry.
3. **QR Enrollment & Short Authentication String (SAS)**:
   - Enrollment QR payload contains:
     `umcf://enroll?cluster=<id>&initiator=<node_id>&pubkey=<base64url>&token=<nonce>&exp=<unix_ts>&sig=<ed25519_sig>`
   - Enforce `now <= exp` (max 300 seconds) and single-use nonce consumption on the coordinator.
   - Derive a 6-digit visual SAS code from `HKDF-SHA256(initiator_pubkey || joiner_pubkey || session_nonce)` displayed on both screens for optional human MITM verification.

---

## 2. Capability Token Verification Checklist

Every cross-node operation requires a signed `CapabilityToken`:

```rust
pub struct CapabilityClaims {
    pub token_id: [u8; 16],
    pub issuer_node_id: String,
    pub subject_node_id: String,
    pub cluster_id: String,
    pub permissions: Vec<PermissionScope>,
    pub max_memory_mb: u64,
    pub max_cpu_cores: u16,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub revocation_epoch: u64,
}
```

When auditing any RPC handler or QUIC stream acceptor, verify all 7 checks occur **before** allocating workload resources:
1. Canonical signature verification against `issuer_node_id`'s pinned `Ed25519` public key.
2. `subject_node_id` matches the authenticated TLS/QUIC peer identity of the caller.
3. `issued_at_unix <= current_time <= expires_at_unix` (with <= 30s clock skew tolerance).
4. `token_id` is not in the local revocation list and `revocation_epoch >= cluster_min_epoch`.
5. Requested action is explicitly covered by `permissions`.
6. Requested resource allocation does not exceed `max_memory_mb` or `max_cpu_cores`.
7. Host user's local `ResourceSharingConsent` is currently enabled.

---

## 3. FFI & Untrusted Input Fuzzing / Hardening

1. **GGUF & Wire Frame Parsers**:
   - Validate magic bytes, header version, string lengths, array counts, and tensor dimension products using checked arithmetic (`checked_mul`, `checked_add`) to prevent integer overflow heap corruptions before passing buffers to `llama.cpp`.
2. **Path Traversal Prevention**:
   - Sanitize all shard filenames and model identifiers; resolve paths inside a dedicated sandboxed cache directory and verify `canonical_path.starts_with(&cache_root)`.
3. **Log & Telemetry Redaction**:
   - Ensure custom `Debug` implementations on key structs, enrollment tokens, and prompt payloads output `"[REDACTED]"` instead of raw secrets or user text.
