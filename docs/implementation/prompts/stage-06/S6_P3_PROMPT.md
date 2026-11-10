# Phase S6.P3 Execution Prompt: Low-Latency Remote Desktop & Game Streaming Host/Client

## 1. Role & Context
You are the Real-Time Game Streaming & Low-Latency Media Transport Lead for UMCF. Phase `S6.P3` builds `crates/umcf-stream`, enabling a desktop or workstation node in the UMCF cluster to stream heavy desktop applications, 3D CAD tools, or AAA games to mobile phones, tablets, laptops, or web browsers with ultra-low glass-to-glass latency (<= 25 ms on LAN/Wi-Fi 6 at 1080p60/120fps) and a sub-3ms multi-modal input return channel (touch, multi-controller gamepad with rumble/motion, keyboard, relative/absolute mouse, and pressure-sensitive stylus).

This phase consists of three sequential subphases:
1. `S6_P3_SP1.md`: Display Capture & Hardware-Accelerated H.264/HEVC/AV1 Encoding Host
2. `S6_P3_SP2.md`: Ultra-Low-Latency QUIC/WebRTC Media Transport & Adaptive Bitrate
3. `S6_P3_SP3.md`: Multi-Modal Input Return Channel (Touch, Gamepad, Keyboard, Mouse, Stylus)

## 2. Prerequisites & Dependencies
- Completion of Phases `S6.P1` and `S6.P2` (`umcf-gpu` texture interop, `umcf-media` hardware codec probing).
- `crates/umcf-net` (QUIC unreliable datagrams RFC 9221 + prioritized streams / WebRTC media tracks).

## 3. Implementation Tasks
1. **Subphase S6.P3.SP1 (`crates/umcf-stream/src/capture.rs`, `src/low_latency_encoder.rs`)**:
   - Implement zero-copy GPU texture display capture (DXGI Desktop Duplication / Windows.Graphics.Capture on Windows, PipeWire/DMA-BUF/X11 on Linux, ScreenCaptureKit on macOS) + low-latency system loopback audio capture (WASAPI Loopback, PipeWire/PulseAudio, CoreAudio) encoded via 5 ms / 10 ms Opus frames.
   - Implement zero-B-frame, intra-refresh / on-demand IDR low-latency hardware video encoding (NVENC `llhq` / `p1`, AMF `ultralowlatency`, QSV, VideoToolbox, VAAPI) keeping capture + encode latency <= 6 ms at 1080p60.
2. **Subphase S6.P3.SP2 (`crates/umcf-stream/src/transport.rs`, `src/abr.rs`)**:
   - Implement real-time media packetizer over QUIC Datagrams / WebRTC RTP with Reed-Solomon / XOR Forward Error Correction (FEC), NACK/PLI instant keyframe recovery, and delay-gradient Adaptive Bitrate (ABR) controller that adjusts encoder bitrate, resolution, and FPS within <= 100 ms of Wi-Fi congestion.
3. **Subphase S6.P3.SP3 (`crates/umcf-stream/src/input_host.rs`, `src/input_client.rs`)**:
   - Implement high-frequency (up to 500 Hz mouse / 250 Hz gamepad/stylus) encrypted input return channel with OS-level virtual HID/input injection, clipboard synchronization, multi-touch gesture translation, and bidirectional gamepad rumble/haptics feedback.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-stream`; 100% test pass rate.
- Median end-to-end glass-to-glass latency (capture + encode + LAN network + decode + present) <= 25.0 ms at 1080p60 on hardware-accelerated host & client.
- Zero B-frames and strict single-frame VBV buffer sizing verified on all streaming bitstreams.
- Under 5% random UDP packet loss, adaptive FEC recovers >= 98% of video frames without requiring a full IDR keyframe freeze.
- Input return channel is strictly authenticated per session capability token and rate-limited/bounds-checked; revoking session permissions immediately terminates all virtual HID buttons/keys (releasing any held keys/triggers to prevent "stuck key" bugs).

## 5. Required Tests
- **`[UNIT]`**: Dirty-rect / damage region tracker, RTP/QUIC frame fragmentation & FEC matrix, delay-gradient ABR state machine, HID keycode & gamepad axis normalizer, stuck-key auto-release on disconnect.
- **`[INT]`**: Synthetic 60 FPS timestamped counter pattern captured -> encoded -> transmitted over loopback QUIC -> decoded -> OCR/barcode timestamp readback verifying < 20 ms pipeline latency.
- **`[SIM]`**: Network impairment simulation (Wi-Fi spike to 80 ms RTT + 5% packet loss + bandwidth drop from 30 Mbps to 5 Mbps) verifying ABR bitrate step-down and zero stream stall.
- **`[HW]`**: Live 1080p60 desktop/game stream from desktop host to Android/macOS/Web client with hardware decode and physical Bluetooth/USB gamepad + touch input.
- **`[MANUAL]`**: Play an interactive 60 FPS application via Flutter `RemoteStreamClientView` and Svelte 5 browser client, toggling the on-screen Performance Overlay HUD (Capture ms, Encode ms, Network RTT/Jitter ms, Decode ms, FPS, Bitrate).

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p3/glass_to_glass_latency_breakdown.json`
- `artifacts/stage-06/p3/abr_congestion_response_trace.json`
- `artifacts/stage-06/p3/input_return_channel_latency_and_safety.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/architecture/LOW_LATENCY_STREAMING.md` covering capture, zero-latency encode, QUIC/WebRTC FEC transport, and HID input security.
- Mark `S6.P3.SP1`–`S6.P3.SP3` and all of Stage 06 `[x]` in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If disconnecting a streaming client leaves held keyboard/mouse/gamepad buttons stuck on the host OS, if capture leaks GPU texture handles, or if glass-to-glass latency exceeds 45 ms on localhost/LAN.
- **Gate**: Stage 06 complete. Unlocks Stage 07 (`stage-07`).
