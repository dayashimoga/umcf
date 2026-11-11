# Cross-Platform Compatibility & Multi-OS Architecture (UMCF)

This document formalizes the multi-OS architectural contracts, ABI/wire compatibility rules, 16 KiB page table invariants, and cross-filesystem safety standards across all 6 supported platforms:
- **Windows** (x86_64, DirectX 12 / Vulkan, NTFS)
- **Linux** (x86_64 / aarch64, Vulkan / CUDA, ext4/btrfs)
- **macOS** (Apple Silicon Arm64, Metal, APFS)
- **Android** (Arm64 / x86_64, Vulkan / NNAPI / QNN, f2fs/ext4, Android 15+ 16 KiB pages)
- **iOS** (Apple Silicon Arm64, Metal / CoreML, APFS)
- **WebBrowser** (Wasm32, WebGPU / WebTransport, Sandboxed IndexedDB)

---

## 1. Multi-OS Node Architecture & Roles

| Node Identifier | Operating System | CPU Architecture | Page Table Alignment | Graphics / Compute Acceleration | Primary Roles |
|:---|:---|:---|:---|:---|:---|
| `node-win-x86` | Windows 10/11 | x86_64 | 4 KiB (4096 B) | DirectX 12 / Vulkan | Master Coordinator, GPU Tile Render, Desktop Streaming Host |
| `node-linux-x86` | Linux (Kernel 6.x+) | x86_64 | 4 KiB (4096 B) | Vulkan / Headless | Scheduler, Storage Registry, CAS Parity Holder |
| `node-mac-m3` | macOS (Sequoia 15+) | Apple Silicon Arm64 | 16 KiB (16384 B) | Metal / Accelerate | Pipeline LLM Stage 1, High-VRAM Inference Worker |
| `node-android-qcom` | Android 15+ | Qualcomm Snapdragon Arm64 | 16 KiB (16384 B) | Vulkan / QNN / NNAPI | Mobile Compute Worker, Edge Pipeline Inference Stage 2 |
| `node-ios-a17` | iOS 18+ | Apple A17 Pro Arm64 | 16 KiB (16384 B) | Metal / CoreML | Mobile Inference Worker, Erasure Shard Pinning |
| `node-browser-wasm` | Chromium / Firefox | Wasm32 | 64 KiB (65536 B) | WebGPU / Canvas | Operator Web Dashboard, Low-Latency Video Client |

---

## 2. 16 KiB Page Table Invariants (Android 15+ & Apple Silicon)

Beginning with **Android 15** and continuing across **Apple Silicon (M-series and A-series)**, the underlying OS kernels enforce a **16 KiB (16,384 bytes) page table size** instead of the legacy 4 KiB page size found on traditional x86_64 Linux and Windows systems.

### 2.1 The SIGBUS Hazard
When memory-mapped files (such as GGUF model weights, activation buffers, or shared memory segments) are mapped into memory with offsets aligned only to 4 KiB multiples:
- Android 15+ kernels and Apple Silicon kernels immediately raise `SIGBUS` (`Bus error: 10`) upon page table translation.
- Standard C/C++ libraries compiled for 4 KiB alignment crash non-deterministically during weight loading.

### 2.2 UMCF Mitigation: `PageAlignmentValidator`
All buffer offsets, mmap segments, and activation tensor bounds are verified and padded using `PageAlignmentValidator`:
```rust
pub const PAGE_SIZE_16KB: usize = 16 * 1024; // 16384 bytes

pub fn pad_to_16kb(offset: usize) -> usize {
    (offset + (PAGE_SIZE_16KB - 1)) & !(PAGE_SIZE_16KB - 1)
}

pub fn validate_16kb_alignment(offset: usize) -> Result<(), PlatformError> {
    if offset % PAGE_SIZE_16KB != 0 {
        return Err(PlatformError::PageAlignmentViolation {
            expected: PAGE_SIZE_16KB,
            actual: offset,
        });
    }
    Ok(())
}
```
In `tests/e2e-cluster/tests/multi_os_tests.rs`, `test_16kb_page_size_mmap_alignment_for_android15_and_apple_silicon` enforces that any buffer destined for Android 15 or Apple Silicon nodes passes this strict alignment check.

---

## 3. Canonical Wire Path Normalization (`VirtualPathNormalizer`)

### 3.1 Cross-Platform Path Leaks
Nodes running Windows naturally produce filesystem paths with backslashes (`\`) and drive prefixes (`C:\` or `D:\`), while Unix nodes use forward slashes (`/`). Transmitting native OS paths across the P2P wire or embedding them in CAS manifests introduces fatal interoperability and security vulnerabilities:
1. **Path Traversal Attacks**: Sequences like `../` or `..\` can escape intended sandbox roots.
2. **Platform Mismatch**: Slashes like `\` are interpreted as literal file names or escape codes on Linux/macOS.
3. **Drive Letter Ambiguity**: `C:\path` has no meaning on mobile, Linux, or Web targets.

### 3.2 Strict Wire Normalization Rules
Every wire message, CAS manifest entry, and SDK upload path must satisfy:
1. **Forbidden Windows Backslashes**: Any path containing `\` is rejected immediately with `PlatformError::PathNormalizationFailed`.
2. **Forbidden DOS Drive Prefixes**: Any path starting with `[A-Za-z]:` is rejected immediately.
3. **Forbidden Absolute Paths**: Paths must be strictly relative; leading `/` is rejected.
4. **Forbidden Traversal**: Any component equal to `..` or empty is rejected.
5. **Canonical Delimiter**: Only single forward slashes (`/`) are accepted.

---

## 4. Cross-Filesystem Case Collision Prevention (`CaseCollisionDetector`)

### 4.1 Case-Insensitive Filesystem Hazards
- **Case-Sensitive**: Linux (`ext4`, `btrfs`, `xfs`) treats `README.md` and `readme.md` as two distinct files in the same directory.
- **Case-Insensitive / Case-Preserving**: Windows (`NTFS`) and macOS / iOS (`APFS` default) treat `README.md` and `readme.md` as the same file.

If a CAS manifest produced on Linux contains multiple files that differ only by case:
- Ingesting this manifest on Windows or macOS silently overwrites one file with the other, corrupting application state and invalidating CAS root hashes.

### 4.2 UMCF Interception Rule
Before any manifest is ingested or extracted to disk on any platform, `CaseCollisionDetector::detect_collisions` projects every path to lowercase. If any duplicate exists in the projected set, the operation is aborted with `PlatformError::CaseCollisionDetected` before touching the physical filesystem.

---

## 5. Endianness & Arithmetic Determinism

1. **Little-Endian Standard**: All wire protocols, binary serialization formats (bincode, CBOR, Protobuf), and cryptographic hashers (BLAKE3, SHA-256) enforce little-endian integer byte encoding (`to_le_bytes()`, `from_le_bytes()`).
2. **Fixed-Point Arithmetic**: Financial accounting and credit balances (`MicroCredits`) use 128-bit integer fixed-point values ($10^{-6}$ credit precision) with zero floating-point math, guaranteeing identical calculation across x86_64, ARM64, and Wasm32.
3. **Greedy Token Parity**: LLM sampling in distributed pipeline inference enforces greedy $argmax$ token selection, yielding 100% bit-exact token sequences across desktop GPUs, Apple Silicon Metal, and mobile NPUs.
