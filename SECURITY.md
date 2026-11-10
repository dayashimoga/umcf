# SECURITY POLICY & THREAT MODEL — UMCF

**Version:** `1.0.0`  
**Status:** Active

---

## 1. Supported Versions

| Version | Supported | Notes |
|---|---|---|
| `0.1.x` (Bootstrap / Pre-v1.0) | Yes | Active foundation & staged rollout |

---

## 2. Security Principles

1. **Zero Implicit Trust**: Every device (Master, Worker, Hybrid, or Client) must possess a cryptographic Ed25519 identity keypair and authenticate via mutual TLS 1.3 (mTLS) or Noise_XX over QUIC before exchanging control or data frames.
2. **Explicit Resource-Sharing Consent**: No compute workload, storage chunk, or streaming session may start on a device without an active, locally signed `ResourceConsentPolicy` defining maximum CPU cores/percentage, RAM/VRAM ceiling, storage path and quota, battery minimum threshold, charging requirement, and thermal cutoff temperature.
3. **Strict Control-Plane / Data-Plane Isolation**:
   - The Cloudflare Workers / Google Cloud Run control plane sees only signed node presence records, encrypted SDP/ICE signaling candidates, and anonymized resource receipts.
   - Raw AI prompts, model weights, intermediate activations, rendered frames, and user files flow strictly over end-to-end encrypted P2P data-plane streams or stay completely local in **Private AI Mode**.
4. **Sandboxed Workload Execution**: Worker tasks run inside OS-enforced resource boundaries (Linux cgroups v2 / namespaces, Windows Job Objects, mobile process/memory guards) with no ambient filesystem or network access outside declared job mounts.
5. **No Secrets in Source Control or Logs**:
   - Signing keys (`.p12`, `.jks`, `.keystore`, `.pem`), API tokens, and enrollment secrets must never be committed or printed to stdout/stderr.
   - iOS builds in CI without valid Apple Developer signing credentials produce explicitly marked unsigned archives (`"signed": false`) rather than using shared/insecure certificates.

---

## 3. Threat Model Summary

| Threat ID | Threat Vector | Mitigation in UMCF |
|---|---|---|
| `TM-01` | Rogue node joining LAN/WAN cluster | Single-use, short-lived (`<= 300s`) cryptographic QR/CLI enrollment tokens with Ed25519 mutual challenge-response attestation. |
| `TM-02` | Eavesdropping on distributed AI prompts or activations | End-to-end mTLS 1.3 over QUIC (`quinn` / `rustls`) on all P2P data streams; optional local-only Private AI policy preventing external node dispatch. |
| `TM-03` | Malicious worker returning corrupted compute/render outputs | Deterministic verification hashes, canary layer/tile checks, and signed execution receipts tied to `NodeId`. |
| `TM-04` | Resource exhaustion / battery drain / thermal damage on mobile worker | Hardware resource guard (`umcf-worker`) enforces memory ceilings, thermal cutoffs, and immediate preemption upon interactive user activity. |
| `TM-05` | Control-plane outage or compromise | Local Autonomous Coordinator operates offline over LAN; control-plane records are signed by node Ed25519 keys so the cloud control plane cannot forge task dispatches. |

---

## 4. Reporting a Vulnerability

To report a security vulnerability:
1. **Do not open a public GitHub issue** for unpatched vulnerabilities.
2. Use the GitHub Security Advisory portal ("Report a vulnerability") or contact the maintainers listed in `.github/CODEOWNERS`.
3. Include affected component/crate, protocol version, reproduction steps, and impact assessment.
4. Maintainers acknowledge reports within 48 hours and provide a remediation timeline within 7 days.

