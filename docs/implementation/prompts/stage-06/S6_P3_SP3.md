# Subphase S6.P3.SP3 Execution Prompt: Multi-Modal Input Return Channel (Touch, Gamepad, Keyboard, Mouse, Stylus)

## 1. Role & Context
You are the Human-Computer Interaction & Low-Latency Input Systems Engineer for UMCF. In Subphase `S6.P3.SP3`, you will implement `InputReturnClient` and `InputInjectionHost` in `crates/umcf-stream/src/input_client.rs` and `src/input_host.rs`. To make remote desktop, creative stylus work, and 60/120 FPS gaming feel completely native on mobile phones, tablets, and web browsers, UMCF must transport multi-modal input events (keyboard, relative/absolute mouse, multi-touch gestures, up to 4 gamepads with rumble/motion sensors, and pressure/tilt stylus) in `< 3 ms` with rock-solid security and automatic "stuck-input" release on disconnect.

## 2. Prerequisites & Dependencies
- `S6.P3.SP1` & `S6.P3.SP2` (`umcf-stream` capture & QUIC transport).
- `crates/umcf-crypto` (Ed25519/XChaCha20-Poly1305 session capability authentication & anti-replay sequence numbers).

## 3. Implementation Tasks
1. **Compact Binary Input Protocol & Anti-Replay Guard (`crates/umcf-stream/src/input_protocol.rs`)**:
   - Define compact binary `InputEventPacket` (serialized in `<= 32` bytes with monotonic 64-bit sequence number `seq_no` and `client_timestamp_us`):
     - `KeyboardEvent { scancode: u16, hid_usage: u16, pressed: bool, modifiers: u8 }`
     - `MouseMoveRelative { dx: i16, dy: i16, high_res_subpixel: (i8, i8) }`
     - `MouseMoveAbsolute { norm_x: u16, norm_y: u16, display_id: u8 }` (`0..=65535` normalized coordinates mapped to captured window/monitor bounds)
     - `MouseButton { button: u8, pressed: bool }` & `MouseScroll { wheel_x: i16, wheel_y: i16 }`
     - `TouchEvent { pointer_id: u8, phase: TouchPhase, norm_x: u16, norm_y: u16, pressure: u16, radius: u16 }`
     - `StylusEvent { phase: StylusPhase, norm_x: u16, norm_y: u16, pressure: u16, tilt_x: i16, tilt_y: i16, twist: u16, barrel_button: bool, eraser: bool }`
     - `GamepadState { controller_idx: u8, buttons_bitmask: u32, left_stick: (i16, i16), right_stick: (i16, i16), left_trigger: u16, right_trigger: u16, gyro_rps: Option<[f32; 3]>, accel_mps2: Option<[f32; 3]> }`
   - Define Host-to-Client feedback events:
     - `GamepadRumbleEvent { controller_idx: u8, low_freq_motor: u16, high_freq_motor: u16, left_trigger_motor: u16, right_trigger_motor: u16, duration_ms: u16 }`
     - `CursorShapeUpdate { shape: StandardCursorOrRgbaBitmap, hotspot: (u16, u16) }`
     - `ClipboardSyncEvent { mime_type: String, payload: Bytes }` (opt-in permission gated).
2. **Host Virtual HID Injector & Stuck-Key Safety Watchdog (`crates/umcf-stream/src/input_host.rs`)**:
   - Implement platform input injection backends (`SendInput` / ViGEm/Virtual HID on Windows, `uinput` / XTest / Wayland virtual-pointer/keyboard on Linux, `CGEventPost` / VirtualHID on macOS) plus a deterministic `MockInputSink` for CI verification.
   - **Security & Capability Enforcement**:
     - Verify session capability token grants `Permission::InjectInput` (as opposed to `ViewOnly` observer mode); reject any input packet from `ViewOnly` peers or with replayed `seq_no`.
     - If capturing a single `ApplicationWindow(hwnd)`, restrict absolute mouse/touch coordinates strictly to that window's bounding rectangle.
   - **Automatic Input State Rollback (`ActiveInputStateTracker`)**:
     - Track the exact set of currently pressed keys (`HashSet<u16>`), mouse buttons (`u8`), active touch pointers (`HashSet<u8>`), and non-zero gamepad triggers/buttons.
     - If the client disconnects, the session is revoked, or the user clicks "Pause Input", **immediately synthesize and inject `KeyUp` / `ButtonUp` / `TouchCancel` / neutral gamepad state for every held input within <= 10 ms** so the host OS is never left with a stuck `Shift`, `Ctrl`, `W` key, or mouse drag.
3. **Mobile Virtual Gamepad & Touch-to-Mouse/Trackpad Client (`crates/umcf-stream/src/input_client.rs`)**:
   - Implement two mobile touch modes in Flutter (`RemoteStreamClientView`):
     - **Direct Touch / Stylus Mode**: 1:1 normalized coordinate mapping, 2-finger pinch-to-zoom viewport pan, 2-finger tap for right-click, pressure/tilt stylus pass-through (Apple Pencil / S-Pen).
     - **Virtual Trackpad + On-Screen Gamepad Overlay**: Customizable translucent twin-stick gamepad overlay (A/B/X/Y, D-pad, L1/R1/L2/R2, L3/R3) with device haptic feedback triggered by incoming `GamepadRumbleEvent`.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `input_protocol.rs`, `input_host.rs`, and `input_client.rs`; 100% test pass rate.
- Round-trip input serialization + QUIC delivery + host injection latency is `<= 3.0 ms` on LAN.
- **Zero Stuck Inputs on Disconnect**: Disconnecting a client while 5 keyboard keys, 2 mouse buttons, 3 touch points, and a gamepad trigger are held down causes `ActiveInputStateTracker` to emit release events for 100% of held inputs within <= 10 ms.
- `ViewOnly` observer clients are cryptographically blocked from injecting any mouse, keyboard, gamepad, or clipboard event.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_input_packet_compact_binary_roundtrip_all_variants`
  - `test_normalized_coordinate_to_window_and_multi_monitor_pixel_mapping`
  - `test_anti_replay_sequence_window_rejects_duplicate_or_stale_packets`
  - `test_active_input_state_tracker_synthesizes_release_events_on_drop`
- **`[INT]`**:
  - `test_multi_modal_input_stream_and_forced_disconnect_releases_all_stuck_keys`
  - `test_view_only_capability_token_denies_input_injection_with_security_audit`
  - `test_bidirectional_gamepad_input_and_rumble_feedback_loop`
- **`[SIM]`**:
  - 500 Hz mouse movement + 250 Hz gamepad polling over simulated jittery Wi-Fi link verifying coalescing of out-of-order relative mouse deltas vs discarding stale absolute positions.
- **`[HW]`**:
  - End-to-end physical gamepad / stylus / touch test from mobile/browser client to host desktop.
- **`[MANUAL]`**:
  - Test the customizable On-Screen Virtual Gamepad overlay, Direct Touch vs Trackpad toggle, and "View-Only Guest Link" mode in Flutter `RemoteStreamClientView` and Svelte 5 `/stream`.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p3/sp3_input_latency_and_stuck_key_audit.json`
- `artifacts/stage-06/p3/sp3_view_only_rbac_enforcement.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the binary input wire format, `ActiveInputStateTracker` stuck-key rollback guarantee, and `ViewOnly` RBAC enforcement in `docs/architecture/LOW_LATENCY_STREAMING.md` (Section 3).
- Mark `S6.P3.SP3`, Phase `S6.P3`, and **Stage 06** complete in `TODO.md` and `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any client disconnect can leave a key or mouse button depressed on the host OS, or if an unprivileged/observer peer can inject keystrokes or read host clipboard contents without explicit permission.
- **Completion Gate**: All Stage 06 gates satisfied (`S6.P1`, `S6.P2`, `S6.P3` at >= 95% coverage, 100% pass rate). Stage 07 (`stage-07`) is now unlocked.
