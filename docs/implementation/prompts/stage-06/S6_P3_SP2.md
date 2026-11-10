# Subphase S6.P3.SP2 Execution Prompt: Ultra-Low-Latency QUIC/WebRTC Media Transport & Adaptive Bitrate

## 1. Role & Context
You are the Real-Time Network Transport & Congestion Control Engineer for UMCF. In Subphase `S6.P3.SP2`, you will implement `LowLatencyMediaTransport`, `AdaptiveFecCodec`, and `DelayGradientAbrController` in `crates/umcf-stream/src/transport.rs` and `src/abr.rs`. Traditional TCP/reliable-stream video delivery suffers from head-of-line blocking when a single Wi-Fi packet drops; your transport uses encrypted QUIC Unreliable Datagrams (RFC 9221) and WebRTC media channels with systematic Reed-Solomon Forward Error Correction (FEC), pacing, and sub-100ms delay-gradient Adaptive Bitrate (ABR) control.

## 2. Prerequisites & Dependencies
- `S6.P3.SP1` (`ZeroLatencyVideoEncoder`, `EncodedFrameMetadata`).
- `crates/umcf-net` (QUIC datagrams + prioritized control streams, WebRTC peer connection bridge for browser clients).

## 3. Implementation Tasks
1. **Frame Packetizer, Pacer & Adaptive FEC (`crates/umcf-stream/src/transport.rs`)**:
   - Fragment each encoded video frame into MTU-safe (`<= 1200` bytes) `MediaShardPacket` datagrams:
     ```rust
     pub struct MediaShardHeader {
         pub stream_id: u32,
         pub frame_seq: u64,
         pub shard_index: u16,
         pub data_shards_count: u16,
         pub parity_shards_count: u16,
         pub is_idr: bool,
         pub capture_timestamp_us: u64,
         pub send_timestamp_us: u64,
     }
     ```
   - **Systematic Reed-Solomon GF($2^8$) FEC (`AdaptiveFecCodec`)**:
     - Dynamically adjust FEC parity overhead between `5%` (clean LAN) and `35%` (lossy Wi-Fi/cellular) based on receiver feedback.
     - If up to `parity_shards_count` packets of a frame are lost in transit, the receiver reconstructs the exact frame in < 0.3 ms without waiting for a network retransmission RTT.
   - **Selective NACK & Instant PLI (Picture Loss Indication)**:
     - If packet loss on a frame exceeds FEC capacity and `RTT < 15 ms`, issue a single-shot NACK for missing shards; otherwise, immediately send a `PliRequest` on the high-priority control stream triggering `force_idr_on_next_frame()` on the host encoder.
   - **Jitter Buffer & Hardware Decoder Presentation (`ZeroDelayJitterBuffer`)**:
     - Reassemble shards in lock-free ring buffers; feed complete frames immediately to the client hardware decoder (`MediaCodec`, `VideoToolbox`, `D3D11VA`/`NVDEC`, `VAAPI`, or WebCodecs `VideoDecoder`) synced to vsync.
2. **Delay-Gradient Adaptive Bitrate Controller (`crates/umcf-stream/src/abr.rs`)**:
   - Measure one-way queuing delay gradient ($\Delta d = (t_{\text{recv}, i} - t_{\text{recv}, i-1}) - (t_{\text{send}, i} - t_{\text{send}, i-1})$) via Kalman/EWMA filter alongside packet loss ratio and decoder queue depth:
     - **State `Hold` / `RampUp`**: When queuing delay gradient $\le 2\text{ ms}$ and loss $< 0.5\%$, smoothly ramp encoder bitrate up by 8% per second toward user target.
     - **State `PreemptiveBackoff` (Early Congestion Detection)**: When queuing delay gradient exceeds $+5\text{ ms}$ (detecting Wi-Fi bufferbloat *before* packets drop), immediately reduce encoder target bitrate by $15\%–30\%$.
     - **State `ResolutionDownshift`**: If required bitrate falls below the minimum perceptual threshold for 1080p60 (e.g., $< 3.5\text{ Mbps}$), dynamically step down capture/encode scale to 720p60 or 1080p30; step back up after 5 seconds of stable headroom.
3. **UI Integration**:
   - Flutter `RemoteStreamClientView` & Svelte 5 `BrowserStreamPlayer.svelte`: Toggleable real-time Telemetry Overlay HUD graphing Glass-to-Glass Latency (ms), Network RTT & Jitter (ms), FEC Recovery Rate (%), and dynamic ABR Bitrate (Mbps).

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `transport.rs` and `abr.rs`; 100% test pass rate.
- Under 5% random packet drop injection at 20% FEC parity ratio, >= 98.5% of frames are reconstructed instantaneously at the receiver with 0 PLI keyframe requests and < 1 ms FEC decode overhead.
- When available link bandwidth drops abruptly from 25 Mbps to 5 Mbps, `DelayGradientAbrController` detects the queuing delay build-up and lowers encoder bitrate to `<= 4.8 Mbps` within `<= 100 ms`, keeping one-way network latency `< 20 ms`.
- End-to-end loopback glass-to-glass latency (synthetic timestamp barcode -> capture -> encode -> QUIC datagram transport -> decode -> barcode read) is `<= 20.0 ms` median (and `<= 25.0 ms` on physical LAN).

## 5. Required Tests
- **`[UNIT]`**:
  - `test_mtu_packetizer_and_reed_solomon_fec_shard_reconstruction`
  - `test_delay_gradient_kalman_filter_detects_bufferbloat_before_loss`
  - `test_abr_state_machine_bitrate_backoff_and_resolution_downshift`
  - `test_pli_throttling_and_nack_selection_by_rtt`
- **`[INT]`**:
  - `test_end_to_end_barcode_timestamp_glass_to_glass_latency_under_25ms`
  - `test_live_stream_under_5_percent_packet_loss_fec_recovery`
- **`[SIM]`**:
  - Gilbert-Elliott bursty Wi-Fi packet loss model + step-function bandwidth shaping (`25 Mbps -> 5 Mbps -> 15 Mbps`) verifying smooth ABR adaptation without stream freeze.
- **`[HW]`**:
  - Stream 1080p60 over physical Wi-Fi 6 / LAN between two devices and record P50/P95/P99 glass-to-glass latency.
- **`[MANUAL]`**:
  - Enable the Telemetry Overlay HUD in Flutter and Svelte 5 during a live stream and observe FEC recovery counters and bitrate adaptation.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p3/sp2_glass_to_glass_barcode_latency.json`
- `artifacts/stage-06/p3/sp2_fec_and_abr_simulation_trace.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document QUIC Datagram media framing, Reed-Solomon FEC sizing, and Delay-Gradient ABR math in `docs/architecture/LOW_LATENCY_STREAMING.md` (Section 2).
- Mark `S6.P3.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If video shards are sent over head-of-line blocking TCP/reliable streams by default when datagrams are available, or if ABR oscillation causes continuous resolution flickering.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S6.P3.SP3`.
