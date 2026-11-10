---
name: umcf-flutter-dev
description: >-
  Guides multi-platform Flutter development (Android, iOS, Windows, Linux, macOS) for UMCF, covering flutter_rust_bridge FFI integration, Riverpod state management, adaptive layouts, and battery/thermal lifecycle hooks. Use when building or updating the UMCF Flutter client.
---

# UMCF Flutter Mobile & Desktop Development Skill

Use this skill when building, modifying, or testing the Flutter client application (`apps/flutter_client`) targeting Android, iOS, Windows, Linux, and macOS.

---

## 1. Project Structure & Riverpod Architecture

Organize `apps/flutter_client/lib/` into strict domain modules:

```text
lib/
├── main.dart
├── app.dart
├── core/
│   ├── bridge/           # Generated flutter_rust_bridge bindings + lifecycle service
│   ├── theme/            # Material 3 adaptive color schemes, typography, spacing tokens
│   ├── router/           # GoRouter declarative navigation with deep-link & auth guards
│   └── platform/         # Battery, thermal, foreground service, and window manager channels
├── features/
│   ├── mesh/             # Peer discovery, topology map, QR enrollment scanner/generator
│   ├── inference/        # Distributed chat, model shard manager, pipeline stage visualizer
│   ├── compute/          # Resource sharing sliders, active jobs, hardware capability cards
│   ├── streaming/        # Remote desktop & game streaming canvas, virtual gamepad, input capture
│   └── settings/         # Security identity, capability revocation, diagnostics & audit logs
```

---

## 2. `flutter_rust_bridge` (FRB) Integration Rules

1. **Non-Blocking Calls Only**:
   - Never invoke synchronous FFI functions that perform I/O, network signaling, or model loading on the UI isolate.
   - Expose Rust operations as `async fn` returning `Result<T, UmcfFfiError>` so FRB dispatches them on the Rust runtime and resolves a Dart `Future<T>`.
2. **Reactive Streams from Rust**:
   - Use FRB `StreamSink<T>` for:
     - `MeshTopologyEvent` (peer joined, peer left, RTT update, capability change)
     - `NodeTelemetrySnapshot` (CPU/GPU/NPU load, RAM used, battery %, thermal state)
     - `InferenceTokenEvent` (streaming tokens, TTFT, tokens/sec, stage latency breakdown)
     - `StreamFramePacket` (decoded texture handles / frame statistics)
3. **Structured Error Translation**:
   - Map `UmcfFfiError` variants into user-comprehensible UI banners with explicit recovery actions (e.g., `InsufficientMemory` -> prompt user to pick a smaller quantization or add a peer node to shard the model).

---

## 3. Adaptive Multi-Platform UI Requirements

1. **Responsive Breakpoints**:
   - `< 600px` (Mobile Portrait): Bottom `NavigationBar`, collapsible status drawer, full-screen QR camera modal.
   - `600px - 1024px` (Tablet / Foldable / Mobile Landscape): `NavigationRail` + two-column master-detail view.
   - `> 1024px` (Desktop Windows / macOS / Linux): Extended `NavigationRail` or sidebar, multi-pane telemetry dashboard, keyboard shortcut listeners (`CallbackShortcuts`), and native window titlebar integration.
2. **Zero Dead Controls Checklist**:
   - Before completing any Flutter screen, verify every `ElevatedButton`, `FilledButton`, `IconButton`, `SwitchListTile`, `Slider`, `PopupMenuButton`, and `TextField`:
     - Has a non-null, non-empty handler connected to a Riverpod `Notifier` method.
     - Displays a disabled state or progress indicator while an async action is in flight to prevent double-submission.
     - Updates persistent settings and triggers the corresponding Rust governor/scheduler update immediately.

---

## 4. Platform Lifecycle, Battery & Thermal Governance

1. **Resource Sharing Policy Enforcement**:
   - Bind the UI resource governor controls directly to `UmcfResourcePolicy`:
     - `maxCpuCores`: `1..=hostCores`
     - `maxMemoryMb`: `256..=hostAvailableMb`
     - `requireCharging`: `bool`
     - `minBatteryPercent`: `10..=95`
     - `maxThermalLevel`: `Nominal | Fair | Serious`
2. **Immediate Thermal/Battery Preemption**:
   - When the platform channel emits a battery drop below `minBatteryPercent`, charger disconnect (when `requireCharging == true`), or thermal state >= `Serious`, immediately invoke `rustBridge.preemptActiveWorkloads(reason)` and update the UI status badge.
3. **Emergency Stop Action**:
   - Place an always-accessible **Stop Sharing / Disconnect Mesh** button in the persistent header/status bar that calls `rustBridge.emergencyHalt()` and confirms teardown to the user.
