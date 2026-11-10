# Subphase S6.P1.SP3 Execution Prompt: Deterministic Verification & Checksum Auditing of Rendered Outputs

## 1. Role & Context
You are the Distributed Verification & Graphics Quality Assurance Engineer for UMCF. In Subphase `S6.P1.SP3`, you will implement `RenderOutputVerifier` in `crates/umcf-gpu/src/verifier.rs`. In a heterogeneous multi-device cluster (combining Qualcomm Adreno, ARM Mali, Apple Silicon, AMD RDNA, and NVIDIA GPUs), integer compute workloads must be bit-exact (verified via BLAKE3), while IEEE-754 floating-point rendering workloads can exhibit minor ULPs of divergence due to FMA (fused multiply-add) and transcendental hardware differences—or severe corruption due to buggy drivers or Byzantine cheating nodes returning blank/garbage tiles to claim compute credits.

## 2. Prerequisites & Dependencies
- `S6.P1.SP1` (`GpuComputeDispatcher`) and `S6.P1.SP2` (`DistributedRenderCoordinator`, `FrameCompositor`).
- `crates/umcf-crypto` (BLAKE3, Ed25519 receipts, ChaCha20 deterministic PRNG for canary selection).

## 3. Implementation Tasks
1. **Dual-Mode Verification Engine (`crates/umcf-gpu/src/verifier.rs`)**:
   - Define `VerificationPolicy`:
     - `DeterministicBitExact`: For integer/fixed-point compute buffers (`u32`, `i32`, `u8` bitmaps). Requires 100% BLAKE3 hash equality against redundant/canary evaluation.
     - `FloatingPointPerceptual { min_ssim: f64, min_psnr_db: f64, max_pixel_abs_diff: f32, reject_nan_inf: bool }`: Default `min_ssim = 0.998`, `min_psnr_db = 48.0`, `max_pixel_abs_diff = 1e-2`, `reject_nan_inf = true`.
2. **Intrinsic Tile Sanity & Canary Spot-Check Auditing**:
   - **Stage 1: Zero-Cost Intrinsic Tile Sanity Check (Every Tile)**:
     - Scan every incoming `CompletedTilePacket` for `NaN`, `±Inf`, all-zero/black tiles (when adjacent/coarse pass energy is non-zero), or invalid dimensions/byte lengths.
   - **Stage 2: Secret Canary Micro-Tile Auditing (`CanaryAuditor`)**:
     - For each worker node participating in a render job, the coordinator secretly selects a random $8 \times 8$ pixel sub-region (`CanarySubTile`) inside 5% of assigned tiles using a CSPRNG seeded by the coordinator's private key.
     - The coordinator (or a trusted verifier peer) computes only the $8 \times 8$ canary sub-tile (representing $< 0.4\%$ compute overhead on a $128 \times 128$ tile) and compares it against the corresponding $8 \times 8$ crop inside the worker's submitted tile using `VerificationPolicy`.
   - **Stage 3: Quarantine, Slashing Signal & Automatic Re-Render**:
     - If a worker fails either Stage 1 (NaN/Inf) or Stage 2 (Canary mismatch), the coordinator immediately discards all unverified tiles from that worker for the current frame, records a signed `VerificationFailureEvidence` struct (consumed by the Stage 7 reputation/ledger system), bans the worker from the job, and re-queues the invalidated tiles onto verified nodes.
3. **UI Integration**:
   - Flutter & Svelte 5: Verification badge on completed renders (`Verified: 100% Canary Pass | Avg SSIM: 0.99994 | 0 NaN Pixels`) and a "Verification Audit Log" modal showing per-node canary checks and any rejected faulty tiles.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-gpu/src/verifier.rs`; 100% test pass rate.
- Canary $8 \times 8$ sub-tile verification adds <= 1.0% total compute overhead to the coordinator while catching 100% of constant-color, random-noise, transposed, or wrong-frame tiles submitted by a faulty/Byzantine worker.
- Legitimate cross-vendor IEEE-754 floating-point variance (e.g., `sin`/`cos`/`sqrt` 1–2 ULP differences between CPU reference, Metal, and Vulkan) passes `FloatingPointPerceptual` with SSIM >= 0.999, whereas a shader bug or corrupted tile is rejected.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_ssim_psnr_and_max_abs_diff_calculation_accuracy`
  - `test_nan_and_inf_pixel_detection_in_rgba16f_and_rgba32f`
  - `test_secret_canary_subtile_coordinate_extraction_and_comparison`
  - `test_deterministic_bit_exact_mode_blake3_verification`
- **`[INT]`**:
  - `test_byzantine_worker_injecting_corrupted_tile_detected_and_rerendered`: In a 3-worker render job, make Worker #2 corrupt 1 tile with black pixels and 1 tile with subtle noise (PSNR 28 dB); verify `CanaryAuditor` catches Worker #2, bans it, re-renders its tiles on Workers #1/#3, and produces a 100% clean final frame.
- **`[SIM]`**:
  - Simulate cross-vendor ULP drift ($\pm 2 \text{ ULPs}$ on FP32) vs Byzantine lazy worker (returning downscaled/interpolated pixels) across 500 frames.
- **`[HW]`**:
  - Compare rendered tile outputs between CPU software rasterizer/compute and hardware GPU (Vulkan/Metal/DX12) and record empirical SSIM/PSNR distributions.
- **`[MANUAL]`**:
  - Inspect the "Verification Audit Log" in the Svelte 5 / Flutter Render Studio and trigger the "Inject Faulty Tile (Test Mode)" button to watch live detection and automatic tile re-dispatch.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p1/sp3_canary_byzantine_detection_report.json`
- `artifacts/stage-06/p1/sp3_cross_vendor_fp_variance_study.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the 3-stage verification pipeline and Canary Sub-Tile math in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Section 1.3).
- Mark `S6.P1.SP3` and Phase `S6.P1` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If corrupted/NaN tiles are ever blitted into the final exported frame without verification, or if canary coordinates are predictable by the worker before submitting its tile.
- **Completion Gate**: Phase `S6.P1` completion gate passed (`S6_P1_SP1`..`SP3` green, >= 95% coverage). Proceed to Phase `S6.P2`.
