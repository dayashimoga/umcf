//! Minimized seed corpus runner for offline regression verification across all 15 targets.

use crate::error::FuzzError;
use crate::targets::FuzzTargets;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusRunSummary {
    pub targets_tested: usize,
    pub total_vectors_evaluated: usize,
    pub panics_detected: usize,
    pub crashes_detected: usize,
    pub passed: bool,
    pub details: String,
}

type FuzzTargetFn = fn(&[u8]) -> Result<bool, FuzzError>;
type NamedTarget = (&'static str, FuzzTargetFn);

pub struct CorpusRunner;

impl CorpusRunner {
    /// Generates a diverse suite of adversarial edge-case test vectors.
    pub fn build_adversarial_seed_corpus() -> Vec<(&'static str, Vec<u8>)> {
        // 1-3. Basic seeds
        let mut corpus = vec![
            ("empty_buffer", vec![]),
            ("single_zero", vec![0x00]),
            ("single_ff", vec![0xFF]),
            ("single_7f", vec![0x7F]),
            ("truncated_3_bytes", vec![0x01, 0x02, 0x03]),
            (
                "truncated_7_bytes",
                vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00],
            ),
        ];

        // 4. Length-prefix allocation bomb: claims 2^62 bytes (0x4000_0000_0000_0000)
        let mut bomb_2_pow_62 = Vec::new();
        bomb_2_pow_62.extend_from_slice(&(1u64 << 62).to_le_bytes());
        bomb_2_pow_62.extend_from_slice(b"payload");
        corpus.push(("allocation_bomb_2_pow_62", bomb_2_pow_62));

        // 5. Length-prefix u64::MAX bomb
        let mut bomb_u64_max = Vec::new();
        bomb_u64_max.extend_from_slice(&u64::MAX.to_le_bytes());
        bomb_u64_max.extend_from_slice(b"payload");
        corpus.push(("allocation_bomb_u64_max", bomb_u64_max));

        // 6. Deep recursion bomb: 64 nested brackets (exceeds MAX_RECURSION_DEPTH of 32)
        let mut deep_brackets = vec![b'['; 64];
        deep_brackets.resize(128, b']');
        corpus.push(("deep_recursion_64_levels", deep_brackets));

        // 7. Invalid UTF-8 sequence: lone continuation byte and overlong encoding
        corpus.push(("invalid_utf8_lone_continuation", vec![0x80, 0x81, 0x82]));
        corpus.push(("invalid_utf8_overlong", vec![0xC0, 0xAF]));

        // 8. Synthetic GGUF valid magic with corrupted tensor count
        let mut gguf_bomb = Vec::new();
        gguf_bomb.extend_from_slice(&0x46554747u32.to_le_bytes()); // "GGUF"
        gguf_bomb.extend_from_slice(&3u32.to_le_bytes()); // version 3
        gguf_bomb.extend_from_slice(&1_000_000u64.to_le_bytes()); // 1M tensors (bomb)
        gguf_bomb.extend_from_slice(&5u64.to_le_bytes());
        corpus.push(("gguf_tensor_count_bomb", gguf_bomb));

        // 9. WASM valid magic with invalid version 99
        let mut wasm_bad_ver = Vec::new();
        wasm_bad_ver.extend_from_slice(b"\0asm");
        wasm_bad_ver.extend_from_slice(&99u32.to_le_bytes());
        corpus.push(("wasm_invalid_version", wasm_bad_ver));

        // 10. Valid JSON object
        corpus.push((
            "valid_json",
            b"{\"model\":\"llama-3\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}]}"
                .to_vec(),
        ));

        // 11. Malformed JSON with unclosed string
        corpus.push((
            "malformed_json_unclosed",
            b"{\"model\":\"incomplete...".to_vec(),
        ));

        corpus
    }

    /// Evaluates all 15 fuzz targets against the full adversarial seed corpus.
    pub fn run_corpus_on_all_targets() -> Result<CorpusRunSummary, FuzzError> {
        let corpus = Self::build_adversarial_seed_corpus();
        let mut total_evaluations = 0;
        let panics_detected = 0;

        let target_functions: [NamedTarget; 15] = [
            ("01_p2p_wire_frame", FuzzTargets::fuzz_01_p2p_wire_frame),
            ("02_capability_token", FuzzTargets::fuzz_02_capability_token),
            ("03_wasm_bytecode", FuzzTargets::fuzz_03_wasm_bytecode),
            ("04_scheduler_dag", FuzzTargets::fuzz_04_scheduler_dag),
            ("05_gguf_header", FuzzTargets::fuzz_05_gguf_header),
            (
                "06_llama_tokenizer_grammar",
                FuzzTargets::fuzz_06_llama_tokenizer_grammar,
            ),
            (
                "07_ai_activation_rpc",
                FuzzTargets::fuzz_07_ai_activation_rpc,
            ),
            ("08_openai_json_body", FuzzTargets::fuzz_08_openai_json_body),
            (
                "09_wgsl_shader_sandbox",
                FuzzTargets::fuzz_09_wgsl_shader_sandbox,
            ),
            (
                "10_render_tile_packet",
                FuzzTargets::fuzz_10_render_tile_packet,
            ),
            (
                "11_media_segment_metadata",
                FuzzTargets::fuzz_11_media_segment_metadata,
            ),
            (
                "12_stream_rtp_fec_and_input",
                FuzzTargets::fuzz_12_stream_rtp_fec_and_input,
            ),
            (
                "13_storage_dag_and_erasure_shard",
                FuzzTargets::fuzz_13_storage_dag_and_erasure_shard,
            ),
            (
                "14_workload_manifest_yaml_toml",
                FuzzTargets::fuzz_14_workload_manifest_yaml_toml,
            ),
            (
                "15_resource_receipt_and_market_order",
                FuzzTargets::fuzz_15_resource_receipt_and_market_order,
            ),
        ];

        for (target_name, target_fn) in target_functions {
            for (vec_name, data) in &corpus {
                total_evaluations += 1;
                match target_fn(data) {
                    Ok(_accepted) => {
                        // Handled cleanly
                    }
                    Err(FuzzError::PanicTrapped { reason, .. }) => {
                        return Err(FuzzError::PanicTrapped {
                            target: target_name.to_string(),
                            reason: format!("Trapped panic on seed '{vec_name}': {reason}"),
                        });
                    }
                    Err(_clean_error) => {
                        // Clean error rejection (e.g. AllocationExceeded, RecursionDepthExceeded, MalformedInput)
                    }
                }
            }
        }

        Ok(CorpusRunSummary {
            targets_tested: 15,
            total_vectors_evaluated: total_evaluations,
            panics_detected,
            crashes_detected: 0,
            passed: panics_detected == 0,
            details: format!(
                "Successfully evaluated {total_evaluations} corpus vectors across all 15 fuzz targets: 0 crashes, 0 panics, zero unhandled errors"
            ),
        })
    }
}
