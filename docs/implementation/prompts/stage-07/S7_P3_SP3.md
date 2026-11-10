# Subphase S7.P3.SP3 Execution Prompt: Complete Flutter Mobile/Desktop UI & Svelte 5 Admin Dashboard Integration

## 1. Role & Context
You are the Lead Full-Stack & Cross-Platform UI Architect for UMCF. In Subphase `S7.P3.SP3`, you will complete the end-to-end integration of both first-party user interfaces—the **Flutter Mobile/Desktop Application** (`ui/flutter_app` targeting Android, iOS, Windows, macOS, and Linux) and the **Svelte 5 Web Administration & Control Dashboard** (`ui/admin_web`)—connecting every screen, state store, chart, and interactive control to the real `umcf-api-gateway` and WebSocket/gRPC streams with zero mock or hardcoded data.

## 2. Prerequisites & Dependencies
- `S7.P2.SP1` (`umcf-api-gateway`, `@umcf/sdk` TypeScript SDK) and `S7.P3.SP2` (`umcf-ledger`, `umcf-marketplace`).
- All Stage 1–7 backend services operational.

## 3. Implementation Tasks
1. **Complete Flutter Cross-Platform Application (`ui/flutter_app/lib/`)**:
   - Implement responsive adaptive navigation (`NavigationRail` / sidebar on Desktop & Tablet landscape; `NavigationBar` bottom tabs on Mobile portrait) across 8 unified feature modules:
     1. **`features/cluster/` (`ClusterOverviewScreen`, `DevicePairingQrScreen`, `NodeDetailSheet`)**: Live topology graph, QR scanner/generator for instant mTLS pairing, per-node CPU/GPU/NPU/RAM/Thermal/Battery telemetry gauges.
     2. **`features/jobs/` (`JobSchedulerScreen`, `WorkflowTemplateGalleryScreen`, `DagExecutionGraphWidget`)**: Submit WASM/Container/Manifest jobs, inspect live DAG stage progress, stream logs, cancel/retry jobs.
     3. **`features/ai/` (`ModelRegistryScreen`, `InferencePlaygroundScreen`, `PrivateAiEnclaveSheet`)**: Pull/inspect GGUF models, view layer partitioning ring across devices, stream chat completions with KV-cache hit badges, toggle `StrictLocalOnly` Private AI shield.
     4. **`features/gpu_render/` (`DistributedRenderStudioScreen`)**: Submit WGSL shaders / 3D render scenes, watch progressive tile composition with Node Attribution color overlay, view Canary verification audit.
     5. **`features/media/` (`TranscodeQueueScreen`)**: Submit distributed video transcodes, view GOP segment timeline across nodes, inspect SSIM/PSNR quality badges and preview output.
     6. **`features/stream/` (`RemoteStreamClientView`, `StreamHostConfigScreen`)**: Low-latency hardware-decoded remote desktop/game stream player with customizable Virtual Gamepad/Touch/Stylus overlay and real-time Glass-to-Glass Latency HUD.
     7. **`features/storage/` (`DistributedStorageScreen`)**: Upload/download/pin files for offline access, configure hard storage quota slider, inspect Reed-Solomon $RS(k,m)$ shard placement map, trigger GC.
     8. **`features/marketplace/` (`ComputeWalletAndMarketScreen`)**: View credit balance & escrow, configure "Contribute Device" battery/Wi-Fi rules, browse live Ask/Bid order book, verify dual-signed cryptographic receipts.
2. **Complete Svelte 5 Admin & Operator Dashboard (`ui/admin_web/src/routes/`)**:
   - Build reactive Svelte 5 runes (`$state`, `$derived`, `$effect`) stores powered by `@umcf/sdk` and `/ws/v1/*` WebSocket streams across matching routes:
     - `/` (Cluster Command Center & Real-Time Telemetry Sparklines)
     - `/nodes` (Node Management, RBAC Roles, Trust Certificates & Quarantine Control)
     - `/jobs` (DAG Scheduler Queue, Template Runner & Live Log Stream)
     - `/ai` (Model Registry, Distributed Layer Topology, Private Enclave & OpenAI API Key Manager)
     - `/gpu` (Interactive WGSL Shader Sandbox & Progressive Tile Compositor)
     - `/media` (Distributed Segment Transcoder & Quality Audit Matrix)
     - `/stream` (WebCodecs/WebRTC Browser Stream Client & Host Session Monitor)
     - `/storage` (Content-Addressed Object Browser, Pin Manager, Quota & Erasure Health)
     - `/market` (Double-Entry Ledger Explorer, Cryptographic Receipt Verifier & Order Book)
     - `/settings` (Daemon Config, Security Audit Log, API Keys & Diagnostics)
3. **Zero-Mock Audit & Offline/Reconnection Resilience**:
   - Audit `ui/flutter_app` and `ui/admin_web` to eliminate any static placeholder arrays; all views must bind to live backend state, render skeleton loaders while fetching, display actionable RFC 9457 error banners on failure, and automatically reconnect WebSocket streams with exponential backoff.

## 4. Acceptance Criteria
- >= 95% test coverage across Flutter widget/bloc/riverpod controllers and Svelte 5 stores/components; 100% test pass rate.
- Zero occurrences of mock/placeholder data in `ui/flutter_app/lib` or `ui/admin_web/src`.
- Both UIs remain responsive at 60 FPS while receiving high-frequency telemetry (10 Hz cluster metrics + 60 Hz token/tile/frame streams) via batched UI frame updates (`requestAnimationFrame` / Flutter frame throttling).
- Full Playwright E2E suite (for Svelte 5) and `flutter test integration_test` suite pass across all 8 feature workflows against a live UMCF daemon.

## 5. Required Tests
- **`[UNIT]`**:
  - Flutter state/notifier unit tests for all 8 feature modules (`flutter test`).
  - Svelte 5 Vitest unit tests for reactive `$state` stores, WebSocket cursor reconnection, and receipt signature verification.
- **`[INT]`**:
  - Playwright E2E test suite navigating every route in `ui/admin_web`, pairing a node, uploading a storage file, running a WASM/AI job, inspecting the receipt, and verifying ledger balance updates.
  - Flutter integration tests (`integration_test/app_e2e_test.dart`) exercising all 8 navigation destinations against a live local daemon.
- **`[SIM]`**:
  - Kill and restart the backend daemon while both Flutter and Svelte 5 UIs are open; verify the "Reconnecting to Local UMCF Daemon..." banner appears within 500 ms and automatically recovers all live streams upon daemon restart.
- **`[HW]`**:
  - Verify 60 FPS UI frame timing (`flutter run --profile` / Chrome DevTools performance trace) on desktop and mobile during concurrent AI streaming + telemetry updates.
- **`[MANUAL]`**:
  - Side-by-side manual verification of Flutter App and Svelte 5 Web Dashboard performing a live cluster workflow.

## 6. Required Evidence & Artifacts
- `artifacts/stage-07/p3/sp3_flutter_widget_and_integration_report.json`
- `artifacts/stage-07/p3/sp3_svelte5_playwright_e2e_report.json`
- `artifacts/stage-07/p3/sp3_zero_mock_static_audit.txt`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/ui/ADMIN_AND_MOBILE_UI_GUIDE.md` documenting screen architecture, state management, and keyboard shortcuts.
- Mark `S7.P3.SP3`, Phase `S7.P3`, and **Stage 07** complete in `TODO.md` and `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any UI button is non-functional/unwired, if any screen displays hardcoded dummy numbers instead of daemon state, or if high-frequency WebSocket events freeze the UI thread.
- **Completion Gate**: All Stage 07 gates satisfied (`S7.P1`, `S7.P2`, `S7.P3` at >= 95% coverage, 100% pass rate). Stage 08 (`stage-08`) is now unlocked.
