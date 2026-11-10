---
trigger: always_on
description: Cryptographic identity, P2P encryption, enrollment tokens, RBAC capabilities, resource consent, and sandboxing standards for UMCF.
---

# 04 - Security, Cryptography & Privacy Governance (UMCF)

UMCF turns personal mobile devices, desktops, and servers into a collaborative compute fabric. Security, cryptographic identity, explicit user consent, and workload isolation are foundational invariants—never optional add-ons.

---

## 1. Cryptographic Device Identity & Key Management

1. **Per-Device Asymmetric Identity**:
   - Every UMCF node generates a unique **Ed25519** signing keypair (node identity & capability signing) and **X25519** key-agreement keypair (Noise/TLS session establishment) using a cryptographically secure RNG (`OsRng`).
   - Node IDs are derived deterministically from the BLAKE3/SHA-256 digest of the node's Ed25519 public key.
2. **Hardware-Backed Key Storage**:
   - Private keys must be stored in platform-secure storage whenever available (Android Keystore, iOS Keychain / Secure Enclave, Windows DPAPI / Credential Locker, macOS Keychain, Linux Secret Service / encrypted keystore with Argon2id key derivation) and zeroized in memory (`zeroize` crate) on drop.

---

## 2. Mutually Authenticated End-to-End Encrypted Transport

1. **Mandatory Mutual Authentication**:
   - Plaintext P2P connections are strictly prohibited. Every QUIC data-plane connection and control-plane session must enforce **TLS 1.3 with mutual certificate/public-key verification (mTLS)** or **Noise_IK / Noise_XX** handshake bound to the peer's verified Ed25519 identity.
2. **Peer Pinning & Trust Registry**:
   - Connections from unknown or revoked Node IDs must be rejected during the cryptographic handshake before allocating application stream buffers.

---

## 3. Enrollment & Pairing Security

1. **Short-Lived Single-Use Enrollment Tokens**:
   - Device pairing (via QR code or bootstrap URI) must use cryptographically signed, short-lived tokens (maximum TTL: **5 minutes**, single-use nonce, bound to cluster ID and initiator public key fingerprint).
2. **Out-of-Band Verification**:
   - Pairing flows must verify the cryptographic SAS (Short Authentication String) or public-key commitment embedded in the QR payload to prevent active MITM substitution during enrollment.

---

## 4. Fine-Grained RBAC & Capability Tokens

1. **Principle of Least Privilege**:
   - Mesh participation is governed by signed, attenuatable **Capability Tokens** specifying:
     - Subject Node ID and Issuer Node ID
     - Permitted operations (`inference:submit`, `inference:worker`, `compute:wasm`, `stream:desktop:view`, `stream:desktop:input`, `mesh:admin`)
     - Resource quotas (max RAM MB, max CPU cores, max bandwidth Mbps, allowed model hashes)
     - Strict expiration timestamp (`exp`) and revocation generation counter
2. **Per-Request & Per-Stream Enforcement**:
   - Every incoming RPC and QUIC stream must validate the caller's capability token signature, expiration, revocation status, and scope before executing any workload.

---

## 5. Explicit User Consent & Resource Sharing Controls

1. **Opt-In Hardware Contribution**:
   - No node may execute remote workloads, host model shards, relay traffic, or expose screen/input capture without **explicit, granular opt-in consent** configured by the device owner.
2. **Instant Revocation & Kill Switch**:
   - Every node UI and CLI must provide an immediate **Emergency Stop / Revoke All Peers** action that terminates active QUIC streams, unloads remote shards, zeroes shared memory buffers, and rotates ephemeral session keys within < 500 ms.
3. **Remote Desktop & Input Privacy**:
   - Screen streaming and remote input injection require a distinct, high-visibility permission prompt on the host device, an un-hideable active session indicator banner/tray icon, and instant local hardware input override (local user moving mouse or pressing escape sequence immediately locks out remote input).

---

## 6. Workload Isolation & Zero Secrets in Logs/VCS

1. **Sandboxed Workload Execution**:
   - Untrusted or general-purpose compute tasks must run inside deterministic sandboxes (e.g., fuel-metered, memory-capped WebAssembly / WASI environments with zero ambient filesystem or network access) or strictly validated tensor graph operators (`llama.cpp` with validated GGUF headers and bounds checks).
2. **Zero Secrets in Source or Telemetry**:
   - Never commit private keys, API tokens, Cloudflare credentials, keystore passwords, or `.env` files to git.
   - Redact enrollment tokens, bearer headers, private key material, and user prompts/completions from all structured logs (`tracing` field redaction) and crash reports by default.
