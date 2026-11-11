# GPU Compute, Distributed Rendering & Media Streaming Architecture (Stage 6)

## 1. Cross-Platform GPU Compute & Shader Sandboxing

### 1.1 WGPU/Vulkan/Metal Compute Shader Sandbox & Buffer Validation (`S6.P1.SP1`)

Because untrusted or distributed cluster nodes execute compute workloads across heterogeneous devices, UMCF enforces strict memory isolation, zero uninitialized VRAM leakage, and driver hang prevention at the shader and buffer level.

#### 1.1.1 Naga AST Loop-Fuel Watchdog Injection
- **Threat Model**: Malicious, buggy, or poorly optimized compute shaders containing unbounded loops (`loop { ... }` or `while (true)`) can hang physical GPU command queues, causing desktop/mobile OS GPU TDR (Timeout Detection and Recovery) driver resets or mobile app termination.
- **Enforcement**:
  - The AST validator traverses incoming WGSL or SPIR-V modules via `naga`.
  - When loop constructs are encountered, an AST transformation injects a fuel counter initialized to `100,000u`.
  - On each loop iteration, the fuel counter decrements. If it reaches zero, a `break` statement executes immediately, setting an atomic diagnostic flag `ShaderLoopFuelExhausted`.
  - Submissions are guaranteed to terminate within $\le 150\text{ ms}$ without resetting the host OS GPU driver.

#### 1.1.2 Zero-Leak VRAM Buffer Allocation (`BoundedVramAllocator`)
- **Residual Memory Threat**: Compute shaders that bind uninitialized storage buffers can potentially read sensitive memory bytes (such as decrypted model activations, frames from prior remote desktop sessions, or secret keys) previously allocated in VRAM by other workloads.
- **Mitigation**:
  - `BoundedVramAllocator` enforces a hard quota ceiling (e.g. 50% free VRAM on mobile, 80% on desktop).
  - Every buffer allocation and recycled slot undergoes mandatory cryptographic zero-scrubbing (`clear_buffer(0x00)`).
  - Canary tests confirm that buffers intentionally populated with `0xDEADBEEF` are wiped completely to `0x00000000` before reuse.

#### 1.1.3 Bounds Check Policy (`ReadZeroSkipWrite`)
- Storage, uniform, and texture access bounds are validated using Naga's `ReadZeroSkipWrite` policy.
- Out-of-bounds reads safely evaluate to `0.0` / `0u`.
- Out-of-bounds writes are discarded cleanly without triggering memory fault exceptions or memory corruption across adjacent GPU buffers.

#### 1.1.4 Time-Sliced Compute Dispatches
- Large 3D workgroup dispatches (e.g., $65,536$ workgroups) are partitioned into time-sliced chunks ($\le 1,024$ workgroups per chunk).
- Guarantees each command submission takes $\le 50\text{ ms}$, ensuring mobile UI compositors never drop frames and allowing thermal or cancellation preemption between slices.
