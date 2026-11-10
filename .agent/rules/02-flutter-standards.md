---
trigger: always_on
description: Flutter multi-platform (Android, iOS, Windows, Linux, macOS) architecture, Riverpod state management, FFI, and UI standards for UMCF.
---

# 02 - Flutter Mobile & Desktop Standards (UMCF)

All Flutter client applications and shared packages in the UMCF workspace must support **Android, iOS, Windows, Linux, and macOS** from a unified, responsive, production-grade codebase.

---

## 1. Architecture & State Management (`Riverpod`)

1. **Layered Feature Architecture**:
   - Organize Flutter code into clear domain layers:
     - `bridge/`: Generated `flutter_rust_bridge` bindings and typed Dart wrappers.
     - `domain/`: Immutable data models, value objects, and repository interfaces.
     - `application/`: Riverpod `Notifier` / `AsyncNotifier` / `StreamNotifier` controllers managing business state and FFI lifecycles.
     - `presentation/`: Responsive widgets, screens, and theme tokens with zero direct FFI calls inside `build()` methods.
2. **Explicit Async States**:
   - Every asynchronous operation or stream exposed via Riverpod (`AsyncValue<T>`) must explicitly render all three states in the UI:
     - `data`: Real, live data with smooth transitions
     - `loading`: Skeleton loaders or determinate/indeterminate progress indicators with cancellation support where applicable
     - `error`: Structured error view displaying diagnostic cause, error code, and a working **Retry** or **Reconfigure** action

---

## 2. Rust-to-Dart FFI Bridge (`flutter_rust_bridge`)

1. **Zero UI Thread Blocking**:
   - All FFI calls into the Rust core (`umcf-core`, `umcf-node`, `umcf-p2p`) must be asynchronous (`Future<T>` or `Stream<T>`) so the Flutter UI raster/main isolate never drops below 60/120 FPS.
2. **High-Frequency Telemetry & Media Streams**:
   - Use Rust-to-Dart `StreamSink` channels with configurable throttling/coalescing (e.g., 10–30 Hz for node telemetry, zero-copy texture/pixel buffers for remote desktop/game streaming) to avoid Dart GC pressure.
3. **Lifecycle-Safe Stream Subscriptions**:
   - Automatically pause or downsample non-critical UI telemetry streams when the app is backgrounded or minimized, and dispose all FFI subscriptions cleanly when providers are disposed.

---

## 3. Zero Non-Functional UI Controls

1. **Complete Control Binding**:
   - Every button, switch, slider (CPU core limit, RAM cap, battery floor, thermal ceiling), QR scanner trigger, peer approval dialog, and job cancellation button must invoke a verified FFI/controller method and persist or apply the change immediately.
2. **Form Validation & Feedback**:
   - All user inputs (control-plane URLs, bootstrap multiaddrs, resource limits, enrollment tokens) must perform strict client-side + Rust-side validation with clear inline error messages.

---

## 4. Responsive & Adaptive Multi-Platform Layouts

1. **Breakpoint-Driven Adaptive Design**:
   - Layouts must adapt fluidly across form factors using `LayoutBuilder` and adaptive navigation patterns:
     - **Compact (< 600px, Phones)**: Bottom navigation bar, single-column cards, swipeable bottom sheets, touch-friendly targets (>= 48x48 dp), safe-area padding for notches and gesture bars.
     - **Medium (600px–1024px, Tablets / Foldables)**: `NavigationRail`, two-pane master-detail views for node inspection and active workloads.
     - **Expanded (> 1024px, Desktop Windows/Linux/macOS)**: Full sidebar navigation, multi-panel telemetry grids, resizable split views, keyboard shortcuts (`Ctrl/Cmd+K` command palette, `Esc` modal close), hover states, and system tray integration.
2. **Input Modality Support**:
   - Remote desktop and game streaming views must support both multi-touch gestures (on-screen virtual gamepad, touch-to-mouse trackpad mode, pinch-to-zoom) on mobile and raw hardware mouse/keyboard/gamepad passthrough on desktop.

---

## 5. Platform Lifecycle, Battery & Thermal Governance

1. **OS Lifecycle Integration**:
   - Observe `AppLifecycleState` (`resumed`, `inactive`, `hidden`, `paused`, `detached`) and native platform channels to notify the Rust resource governor immediately upon state transitions.
2. **Battery, Charging & Thermal Hooks**:
   - Monitor real battery level, charging state (AC/USB/Wireless/Discharging), and OS thermal status (Android `PowerManager.OnThermalStatusChangedListener`, iOS `ProcessInfo.thermalState`, desktop sensor APIs).
   - Enforce user policies strictly: e.g., "Only run compute workloads while charging and battery >= 50%" or "Throttle/evict shards immediately on `THERMAL_STATUS_SEVERE`".
3. **Foreground Execution Transparency**:
   - When executing background/foreground compute on mobile (e.g., Android Foreground Service), display an accurate persistent notification showing current compute load, power draw estimate, and a working one-tap **Pause / Stop Compute** action.
