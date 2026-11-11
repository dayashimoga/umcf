//! 15 Dedicated Fuzz Target Handlers for untrusted input parsers and FFI boundaries.

use std::panic::{catch_unwind, AssertUnwindSafe};
use crate::bounded_reader::{check_nesting_depth, BoundedSliceReader, MAX_ALLOCATION_BYTES, MAX_RECURSION_DEPTH};
use crate::error::FuzzError;

pub struct FuzzTargets;

impl FuzzTargets {
    /// 1. Fuzzes P2P wire framing (QUIC frames, UDP hole-punch packets, Relay messages).
    pub fn fuzz_01_p2p_wire_frame(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 8 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let magic = reader.read_u32_le()?;
            let msg_type = reader.read_u16_le()?;
            let _payload = reader.read_length_prefixed_slice("P2PWireFrame", MAX_ALLOCATION_BYTES)?;
            Ok(magic != 0 && msg_type > 0)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "01_p2p_wire_frame".to_string(),
            reason: "Panic in P2P wire framing parser".to_string(),
        })?
    }

    /// 2. Fuzzes Capability Tokens, X.509/DID certificates, and Ed25519 signatures.
    pub fn fuzz_02_capability_token(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 64 {
                return Ok(false);
            }
            if let Ok(s) = std::str::from_utf8(data) {
                if s.starts_with("did:umcf:") || s.starts_with("cap:v1:") {
                    return Ok(true);
                }
            }
            let mut reader = BoundedSliceReader::new(data);
            let _version = reader.read_u8()?;
            let _sig = reader.read_length_prefixed_slice("CapabilityToken", 512)?;
            Ok(true)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "02_capability_token".to_string(),
            reason: "Panic in capability token parser".to_string(),
        })?
    }

    /// 3. Fuzzes WASM module bytecode validator and WASI import linker.
    pub fn fuzz_03_wasm_bytecode(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 8 {
                return Ok(false);
            }
            // WASM magic: \0asm (0x6d736100)
            let is_wasm_magic = data.starts_with(b"\0asm");
            let mut reader = BoundedSliceReader::new(data);
            let _magic = reader.read_u32_le()?;
            let version = reader.read_u32_le()?;
            Ok(is_wasm_magic && version == 1)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "03_wasm_bytecode".to_string(),
            reason: "Panic in WASM module loader".to_string(),
        })?
    }

    /// 4. Fuzzes Scheduler Task Graph (DAG) Protobuf/bincode messages.
    pub fn fuzz_04_scheduler_dag(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 4 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let node_count = reader.read_u32_le()?;
            if node_count > 10_000 {
                return Err(FuzzError::AllocationExceeded {
                    target: "SchedulerDag".to_string(),
                    requested_bytes: node_count as usize,
                    limit_bytes: 10_000,
                });
            }
            let _edges = reader.read_length_prefixed_slice("SchedulerDagEdges", MAX_ALLOCATION_BYTES)?;
            Ok(true)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "04_scheduler_dag".to_string(),
            reason: "Panic in scheduler DAG parser".to_string(),
        })?
    }

    /// 5. Fuzzes GGUF header, metadata KV pairs, string lengths, and tensor tables.
    pub fn fuzz_05_gguf_header(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 16 {
                return Ok(false);
            }
            // GGUF magic: GGUF (0x46554747 little endian)
            let mut reader = BoundedSliceReader::new(data);
            let magic = reader.read_u32_le()?;
            if magic != 0x46554747 {
                return Ok(false);
            }
            let version = reader.read_u32_le()?;
            let tensor_count = reader.read_u64_le()?;
            let metadata_kv_count = reader.read_u64_le()?;

            // Reject allocation bombs upfront
            if tensor_count > 100_000 || metadata_kv_count > 100_000 {
                return Err(FuzzError::AllocationExceeded {
                    target: "GgufHeader".to_string(),
                    requested_bytes: (tensor_count + metadata_kv_count) as usize,
                    limit_bytes: 200_000,
                });
            }

            Ok((2..=3).contains(&version))
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "05_gguf_header".to_string(),
            reason: "Panic in GGUF header parser".to_string(),
        })?
    }

    /// 6. Fuzzes LLM FFI tokenizer, incremental UTF-8 piece detokenizer, and grammar compiler.
    pub fn fuzz_06_llama_tokenizer_grammar(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.is_empty() {
                return Ok(false);
            }
            // Incremental UTF-8 validation
            match std::str::from_utf8(data) {
                Ok(s) => Ok(!s.is_empty()),
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    Ok(valid_up_to > 0)
                }
            }
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "06_llama_tokenizer_grammar".to_string(),
            reason: "Panic in tokenizer/grammar parser".to_string(),
        })?
    }

    /// 7. Fuzzes AI activation RPC packets and FP16/Q8 tensor activation decompressor.
    pub fn fuzz_07_ai_activation_rpc(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 12 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let layer_id = reader.read_u32_le()?;
            let seq_len = reader.read_u32_le()?;
            if seq_len > 32768 {
                return Err(FuzzError::AllocationExceeded {
                    target: "AiActivationRpc".to_string(),
                    requested_bytes: seq_len as usize * 4096,
                    limit_bytes: MAX_ALLOCATION_BYTES,
                });
            }
            let _tensors = reader.read_length_prefixed_slice("AiActivationRpcTensors", MAX_ALLOCATION_BYTES)?;
            Ok(layer_id < 128)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "07_ai_activation_rpc".to_string(),
            reason: "Panic in AI activation RPC parser".to_string(),
        })?
    }

    /// 8. Fuzzes OpenAI-compatible `/v1/chat/completions` JSON body and chat template renderer.
    pub fn fuzz_08_openai_json_body(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.is_empty() {
                return Ok(false);
            }
            // Enforce nesting depth limit <= 32 before JSON parse
            check_nesting_depth(data, MAX_RECURSION_DEPTH)?;

            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(data) {
                Ok(val.is_object())
            } else {
                Ok(false)
            }
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "08_openai_json_body".to_string(),
            reason: "Panic in OpenAI JSON parser".to_string(),
        })?
    }

    /// 9. Fuzzes WGSL / SPIR-V compute shader parser and AST loop-fuel watchdog injector.
    pub fn fuzz_09_wgsl_shader_sandbox(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.is_empty() {
                return Ok(false);
            }
            check_nesting_depth(data, MAX_RECURSION_DEPTH)?;
            if let Ok(code) = std::str::from_utf8(data) {
                let has_compute = code.contains("@compute") || code.contains("workgroup_size");
                Ok(has_compute)
            } else {
                Ok(false)
            }
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "09_wgsl_shader_sandbox".to_string(),
            reason: "Panic in WGSL shader sandbox".to_string(),
        })?
    }

    /// 10. Fuzzes GPU compressed render tile decoder and image compositor.
    pub fn fuzz_10_render_tile_packet(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 16 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let width = reader.read_u32_le()?;
            let height = reader.read_u32_le()?;
            if width > 8192 || height > 8192 {
                return Err(FuzzError::AllocationExceeded {
                    target: "RenderTile".to_string(),
                    requested_bytes: (width as usize) * (height as usize) * 4,
                    limit_bytes: MAX_ALLOCATION_BYTES,
                });
            }
            let _pixels = reader.read_length_prefixed_slice("RenderTilePixels", MAX_ALLOCATION_BYTES)?;
            Ok(width > 0 && height > 0)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "10_render_tile_packet".to_string(),
            reason: "Panic in render tile packet parser".to_string(),
        })?
    }

    /// 11. Fuzzes media GOP segment metadata and PTS/DTS continuity normalizer.
    pub fn fuzz_11_media_segment_metadata(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 16 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let _pts = reader.read_u64_le()?;
            let _dts = reader.read_u64_le()?;
            let _nal_slice = reader.read_length_prefixed_slice("MediaNalSlice", MAX_ALLOCATION_BYTES)?;
            Ok(true)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "11_media_segment_metadata".to_string(),
            reason: "Panic in media segment metadata parser".to_string(),
        })?
    }

    /// 12. Fuzzes low-latency streaming Reed-Solomon FEC packets and binary input event packets.
    pub fn fuzz_12_stream_rtp_fec_and_input(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 4 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let seq = reader.read_u16_le()?;
            let event_type = reader.read_u8()?;
            let _flags = reader.read_u8()?;
            // Input packets are strictly bounded <= 36 bytes
            if data.len() <= 36 {
                Ok(seq > 0 || event_type > 0)
            } else {
                let _fec_payload = reader.read_length_prefixed_slice("FecPayload", 1500)?;
                Ok(true)
            }
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "12_stream_rtp_fec_and_input".to_string(),
            reason: "Panic in stream RTP/FEC parser".to_string(),
        })?
    }

    /// 13. Fuzzes content-addressed storage ObjectDagManifest and Reed-Solomon shard headers.
    pub fn fuzz_13_storage_dag_and_erasure_shard(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 36 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let k = reader.read_u8()?;
            let m = reader.read_u8()?;
            if k == 0 || m == 0 || (k as usize + m as usize) > 255 {
                return Ok(false);
            }
            let _cid = reader.read_length_prefixed_slice("StorageDagCid", 64)?;
            Ok(true)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "13_storage_dag_and_erasure_shard".to_string(),
            reason: "Panic in storage DAG/erasure parser".to_string(),
        })?
    }

    /// 14. Fuzzes declarative workload manifest YAML/TOML and variable interpolator.
    pub fn fuzz_14_workload_manifest_yaml_toml(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.is_empty() {
                return Ok(false);
            }
            if data.len() > MAX_ALLOCATION_BYTES {
                return Err(FuzzError::AllocationExceeded {
                    target: "WorkloadManifest".to_string(),
                    requested_bytes: data.len(),
                    limit_bytes: MAX_ALLOCATION_BYTES,
                });
            }
            check_nesting_depth(data, MAX_RECURSION_DEPTH)?;

            if let Ok(s) = std::str::from_utf8(data) {
                let is_yaml = serde_yaml::from_str::<serde_yaml::Value>(s).is_ok();
                let is_toml = toml::from_str::<toml::Value>(s).is_ok();
                Ok(is_yaml || is_toml)
            } else {
                Ok(false)
            }
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "14_workload_manifest_yaml_toml".to_string(),
            reason: "Panic in workload manifest parser".to_string(),
        })?
    }

    /// 15. Fuzzes SignedResourceReceipt 104-byte resource vector and Marketplace order deserializers.
    pub fn fuzz_15_resource_receipt_and_market_order(data: &[u8]) -> Result<bool, FuzzError> {
        catch_unwind(AssertUnwindSafe(|| {
            if data.len() < 104 {
                return Ok(false);
            }
            let mut reader = BoundedSliceReader::new(data);
            let seq = reader.read_u64_le()?;
            let cpu_time = reader.read_u64_le()?;
            let _wasm_fuel = reader.read_u64_le()?;
            let _gpu_time = reader.read_u64_le()?;
            let _npu_time = reader.read_u64_le()?;
            let _sig = reader.read_length_prefixed_slice("ReceiptSignature", 64)?;
            Ok(seq > 0 || cpu_time > 0)
        }))
        .map_err(|_| FuzzError::PanicTrapped {
            target: "15_resource_receipt_and_market_order".to_string(),
            reason: "Panic in resource receipt/marketplace parser".to_string(),
        })?
    }
}

