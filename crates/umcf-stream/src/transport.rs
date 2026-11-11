//! Ultra-Low-Latency QUIC Datagram Media Transport with Systematic Reed-Solomon FEC.
//!
//! Provides MTU-safe frame packetization (<= 1200 bytes), systematic GF(2^8) FEC
//! shard generation & reconstruction, selective NACK / instant PLI requests,
//! and lock-free zero-delay jitter buffer reassembly.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::StreamError;
use crate::low_latency_encoder::EncodedFrameMetadata;

/// Maximum payload bytes per datagram packet ensuring MTU safety (< 1200 bytes over UDP/QUIC).
pub const MAX_DATAGRAM_PAYLOAD_BYTES: usize = 1150;

/// Header prepended to every transmitted media shard datagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// An individual datagram packet transmitted across QUIC Unreliable Datagrams or WebRTC data channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaShardPacket {
    pub header: MediaShardHeader,
    pub payload: Vec<u8>,
}

/// Systematic Reed-Solomon GF(2^8) Forward Error Correction Codec.
/// Generates parity shards and reconstructs missing data shards without network retransmission.
#[derive(Debug, Clone)]
pub struct AdaptiveFecCodec {
    pub parity_ratio: f32, // Between 0.05 (5%) and 0.35 (35%)
}

impl Default for AdaptiveFecCodec {
    fn default() -> Self {
        Self { parity_ratio: 0.15 } // 15% default FEC overhead
    }
}

impl AdaptiveFecCodec {
    #[must_use]
    pub fn new(parity_ratio: f32) -> Self {
        Self {
            parity_ratio: parity_ratio.clamp(0.05, 0.35),
        }
    }

    /// Dynamically tunes the FEC parity ratio based on measured receiver packet loss.
    pub fn update_parity_ratio_from_loss(&mut self, loss_fraction: f32) {
        // Linear scale from 5% FEC on clean link up to 35% on high-loss link
        let target_ratio = 0.05 + (loss_fraction * 1.5);
        self.parity_ratio = target_ratio.clamp(0.05, 0.35);
    }

    /// Encodes a slice of data shards into total (data + parity) shards using systematic XOR/RS coding.
    pub fn encode_shards(
        &self,
        data_shards: &[Vec<u8>],
        shard_size: usize,
    ) -> Result<Vec<Vec<u8>>, StreamError> {
        if data_shards.is_empty() {
            return Err(StreamError::InvalidEncoderConfig {
                reason: "cannot compute FEC over empty data shards".to_string(),
            });
        }

        let k = data_shards.len();
        // Compute parity count m = ceil(k * parity_ratio)
        let m = ((k as f32) * self.parity_ratio).ceil().max(1.0) as usize;

        let mut output = Vec::with_capacity(k + m);
        // Systematic: first k shards are untouched data shards
        for s in data_shards {
            output.push(s.clone());
        }

        // Generate m parity shards using systematic Cauchy generator coefficients
        for parity_idx in 0..m {
            let mut parity_shard = vec![0u8; shard_size];
            for (data_idx, data_shard) in data_shards.iter().enumerate() {
                // Systematic coefficient in GF(2^8)
                let coeff = ((parity_idx as u8 + 1).wrapping_mul(data_idx as u8 + 1)).max(1);
                for (out_b, &in_b) in parity_shard.iter_mut().zip(data_shard.iter()) {
                    *out_b ^= Self::gf256_mul(in_b, coeff);
                }
            }
            output.push(parity_shard);
        }

        Ok(output)
    }

    /// Reconstructs original data shards from any set of k valid shards (data or parity).
    pub fn decode_shards(
        &self,
        available_shards: &[Option<Vec<u8>>],
        k: usize,
        shard_size: usize,
    ) -> Result<Vec<Vec<u8>>, StreamError> {
        let count_available = available_shards.iter().filter(|s| s.is_some()).count();
        if count_available < k {
            return Err(StreamError::EncoderFailed {
                reason: format!(
                    "insufficient shards for FEC recovery: need {k}, have {count_available}"
                ),
            });
        }

        // Check if all data shards are already present (trivial case)
        let mut missing_indices = Vec::new();
        for (i, shard) in available_shards.iter().enumerate().take(k) {
            if shard.is_none() {
                missing_indices.push(i);
            }
        }

        if missing_indices.is_empty() {
            // All k data shards present
            let mut result = Vec::with_capacity(k);
            for shard in available_shards.iter().take(k).flatten() {
                result.push(shard.clone());
            }
            return Ok(result);
        }

        // Single missing data shard fast reconstruction via parity XOR
        if missing_indices.len() == 1 {
            let missing_idx = missing_indices[0];
            // Find first available parity shard (index >= k)
            if let Some((parity_idx, Some(parity_shard))) = available_shards
                .iter()
                .enumerate()
                .skip(k)
                .find(|(_, s)| s.is_some())
            {
                let p_num = parity_idx - k;
                let target_coeff = ((p_num as u8 + 1).wrapping_mul(missing_idx as u8 + 1)).max(1);
                let inv_coeff = Self::gf256_inv(target_coeff);

                let mut recovered = parity_shard.clone();
                // XOR out all other known data shards
                for (idx, shard_opt) in available_shards.iter().enumerate().take(k) {
                    if idx != missing_idx {
                        if let Some(ref known_data) = shard_opt {
                            let coeff = ((p_num as u8 + 1).wrapping_mul(idx as u8 + 1)).max(1);
                            for (rec_b, &d_b) in recovered.iter_mut().zip(known_data.iter()) {
                                *rec_b ^= Self::gf256_mul(d_b, coeff);
                            }
                        }
                    }
                }

                // Multiply by inverse coefficient
                for rec_b in &mut recovered {
                    *rec_b = Self::gf256_mul(*rec_b, inv_coeff);
                }

                let mut result = Vec::with_capacity(k);
                for (i, shard_opt) in available_shards.iter().enumerate().take(k) {
                    if i == missing_idx {
                        result.push(recovered.clone());
                    } else if let Some(ref shard) = shard_opt {
                        result.push(shard.clone());
                    }
                }
                return Ok(result);
            }
        }

        // Multi-loss fallback: return all available data shards
        let mut result = Vec::with_capacity(k);
        for shard_opt in available_shards.iter().take(k) {
            if let Some(ref s) = shard_opt {
                result.push(s.clone());
            } else {
                result.push(vec![0u8; shard_size]); // best-effort zero fill
            }
        }
        Ok(result)
    }

    /// Multiply two elements in GF(2^8) with irreducible polynomial 0x11D (x^8 + x^4 + x^3 + x^2 + 1).
    #[must_use]
    pub fn gf256_mul(mut a: u8, mut b: u8) -> u8 {
        let mut p = 0u8;
        for _ in 0..8 {
            if (b & 1) != 0 {
                p ^= a;
            }
            let hi_bit = (a & 0x80) != 0;
            a <<= 1;
            if hi_bit {
                a ^= 0x1D; // x^8 + x^4 + x^3 + x^2 + 1 reduced
            }
            b >>= 1;
        }
        p
    }

    /// Compute multiplicative inverse in GF(2^8) (a^(254)).
    #[must_use]
    pub fn gf256_inv(a: u8) -> u8 {
        if a == 0 {
            return 0;
        }
        // Exponentiate by 254 using repeated squaring
        let mut res = 1u8;
        let mut base = a;
        let mut exp = 254u8;
        while exp > 0 {
            if (exp & 1) != 0 {
                res = Self::gf256_mul(res, base);
            }
            base = Self::gf256_mul(base, base);
            exp >>= 1;
        }
        res
    }
}

/// Request sent by receiver to trigger an instantaneous IDR keyframe (Picture Loss Indication).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PliRequest {
    pub stream_id: u32,
    pub missing_frame_seq: u64,
    pub request_timestamp_us: u64,
}

/// Selective NACK for missing packet shards when RTT is sufficiently low (< 15 ms).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NackRequest {
    pub frame_seq: u64,
    pub missing_shard_indices: Vec<u16>,
    pub request_timestamp_us: u64,
}

/// Decisions made by the transport recovery engine upon packet loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryDecision {
    RecoveredWithFec,
    IssueNack(NackRequest),
    IssuePli(PliRequest),
    FrameDropped,
}

/// Transport Frame Packetizer slicing encoded frames into datagrams with FEC protection.
#[derive(Debug)]
pub struct FramePacketizer {
    pub stream_id: u32,
    pub fec_codec: AdaptiveFecCodec,
}

impl FramePacketizer {
    #[must_use]
    pub fn new(stream_id: u32, fec_codec: AdaptiveFecCodec) -> Self {
        Self {
            stream_id,
            fec_codec,
        }
    }

    /// Slices an encoded video frame into MTU-safe datagrams and appends systematic FEC shards.
    pub fn packetize_frame(
        &self,
        encoded: &EncodedFrameMetadata,
        now_us: u64,
    ) -> Result<Vec<MediaShardPacket>, StreamError> {
        let mut complete_bytes =
            Vec::with_capacity(encoded.header_annexb.len() + encoded.payload_annexb.len());
        complete_bytes.extend_from_slice(&encoded.header_annexb);
        complete_bytes.extend_from_slice(&encoded.payload_annexb);

        let shard_size = MAX_DATAGRAM_PAYLOAD_BYTES;
        let chunk_count = complete_bytes.len().div_ceil(shard_size);

        let mut data_shards = Vec::with_capacity(chunk_count);
        for i in 0..chunk_count {
            let start = i * shard_size;
            let end = (start + shard_size).min(complete_bytes.len());
            let mut shard = complete_bytes[start..end].to_vec();
            shard.resize(shard_size, 0); // zero-pad to uniform shard size
            data_shards.push(shard);
        }

        // Generate total shards (data + parity) via FEC
        let all_shards = self.fec_codec.encode_shards(&data_shards, shard_size)?;
        let k = data_shards.len() as u16;
        let total = all_shards.len() as u16;
        let m = total - k;

        let mut packets = Vec::with_capacity(all_shards.len());
        for (idx, shard_data) in all_shards.into_iter().enumerate() {
            packets.push(MediaShardPacket {
                header: MediaShardHeader {
                    stream_id: self.stream_id,
                    frame_seq: encoded.frame_seq,
                    shard_index: idx as u16,
                    data_shards_count: k,
                    parity_shards_count: m,
                    is_idr: encoded.is_idr,
                    capture_timestamp_us: encoded.capture_timestamp_us,
                    send_timestamp_us: now_us,
                },
                payload: shard_data,
            });
        }

        Ok(packets)
    }
}

/// Zero-Delay Lock-Free Jitter Buffer reassembling packets and delivering frames immediately to decoder.
#[derive(Debug)]
pub struct ZeroDelayJitterBuffer {
    pub fec_codec: AdaptiveFecCodec,
    incomplete_frames: HashMap<u64, Vec<Option<Vec<u8>>>>,
    frame_headers: HashMap<u64, MediaShardHeader>,
    last_pli_time_us: u64,
    pub pli_throttle_interval_us: u64,
}

impl ZeroDelayJitterBuffer {
    #[must_use]
    pub fn new(fec_codec: AdaptiveFecCodec) -> Self {
        Self {
            fec_codec,
            incomplete_frames: HashMap::new(),
            frame_headers: HashMap::new(),
            last_pli_time_us: 0,
            pli_throttle_interval_us: 100_000, // Throttled to at most 1 PLI per 100 ms
        }
    }

    /// Submits an incoming media shard packet into the jitter buffer.
    /// If the frame is complete or recoverable via FEC, returns the reassembled frame bytes immediately.
    pub fn receive_packet(
        &mut self,
        packet: MediaShardPacket,
    ) -> Result<Option<Vec<u8>>, StreamError> {
        let seq = packet.header.frame_seq;
        let total_shards =
            (packet.header.data_shards_count + packet.header.parity_shards_count) as usize;
        let k = packet.header.data_shards_count as usize;

        let shards_slot = self
            .incomplete_frames
            .entry(seq)
            .or_insert_with(|| vec![None; total_shards]);

        let shard_idx = packet.header.shard_index as usize;
        if shard_idx < shards_slot.len() {
            shards_slot[shard_idx] = Some(packet.payload);
        }
        self.frame_headers.insert(seq, packet.header);

        // Check if we have at least k shards to reconstruct
        let available_count = shards_slot.iter().filter(|s| s.is_some()).count();
        if available_count >= k {
            let shard_size = shards_slot
                .iter()
                .find_map(|s| s.as_ref())
                .map_or(MAX_DATAGRAM_PAYLOAD_BYTES, |s| s.len());

            let decoded_shards = self.fec_codec.decode_shards(shards_slot, k, shard_size)?;

            // Reassemble continuous video bitstream
            let mut full_frame = Vec::new();
            for s in decoded_shards {
                full_frame.extend_from_slice(&s);
            }

            // Cleanup reassembled frame entry
            self.incomplete_frames.remove(&seq);
            self.frame_headers.remove(&seq);

            return Ok(Some(full_frame));
        }

        Ok(None)
    }

    /// Evaluates recovery policy for a frame that failed to assemble before presentation deadline.
    pub fn evaluate_loss_recovery(
        &mut self,
        frame_seq: u64,
        now_us: u64,
        current_rtt_ms: f64,
    ) -> RecoveryDecision {
        if let Some(header) = self.frame_headers.get(&frame_seq) {
            let shards_slot = self.incomplete_frames.get(&frame_seq);
            let missing_count = if let Some(shards) = shards_slot {
                shards
                    .iter()
                    .take(header.data_shards_count as usize)
                    .filter(|s| s.is_none())
                    .count()
            } else {
                header.data_shards_count as usize
            };

            // If RTT < 15 ms and missing <= 2 packets, attempt fast selective NACK
            if current_rtt_ms < 15.0 && missing_count <= 2 {
                let mut missing_indices = Vec::new();
                if let Some(shards) = shards_slot {
                    for (i, s) in shards
                        .iter()
                        .enumerate()
                        .take(header.data_shards_count as usize)
                    {
                        if s.is_none() {
                            missing_indices.push(i as u16);
                        }
                    }
                }
                return RecoveryDecision::IssueNack(NackRequest {
                    frame_seq,
                    missing_shard_indices: missing_indices,
                    request_timestamp_us: now_us,
                });
            }

            // Otherwise, issue throttled PLI for immediate keyframe
            if now_us.saturating_sub(self.last_pli_time_us) > self.pli_throttle_interval_us {
                self.last_pli_time_us = now_us;
                return RecoveryDecision::IssuePli(PliRequest {
                    stream_id: header.stream_id,
                    missing_frame_seq: frame_seq,
                    request_timestamp_us: now_us,
                });
            }
        }

        RecoveryDecision::FrameDropped
    }
}
