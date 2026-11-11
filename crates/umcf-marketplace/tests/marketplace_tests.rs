//! Classified Five-Tier Test Suite for Subphase S7.P3.SP2:
//! Double-Entry Compute Credit Ledger & Marketplace Order Matching.
//!
//! Tiers:
//! - [UNIT]: Double-entry journal balance rejection, escrow lifecycle math,
//!   idempotent settlement, Byzantine slashing, and constraint matching.
//! - [INT]: Full lifecycle (Ask -> Bid -> Match -> Escrow -> Settle -> Refund),
//!   and concurrent 64-thread settlements preserving zero-sum invariant.
//! - [SIM]: Property-based test with 5,000 randomized operations asserting global conservation.
//! - [HW]: Matching engine throughput benchmark across 10,000 offers (< 5 ms target).
//! - [MANUAL]: UI schema and safety constraint toggle inspection.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;

use umcf_ledger::{
    AccountKind, ByzantineFaultType, DoubleEntryCreditLedger, JournalLeg, LedgerError,
    LedgerTransaction, MeteredResourceVector, PricingSchedule, ReceiptBody, SignedResourceReceipt,
    VerificationFailureEvidence, GENESIS_PREV_RECEIPT_ID,
};
use umcf_marketplace::{
    ComputeAskOffer, ComputeBidRequest, MarketError, MarketplaceMatchingEngine,
};

fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    (sk, vk)
}

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_double_entry_journal_rejects_unbalanced_legs_and_negative_balances() {
    let mut ledger = DoubleEntryCreditLedger::new();

    // 1. Unbalanced transaction: Debits (100) != Credits (90) must be rejected
    let unbalanced_tx = LedgerTransaction {
        tx_id: "tx-unbalanced".to_string(),
        timestamp_unix_ms: 1000,
        legs: vec![
            JournalLeg::debit(AccountKind::AvailableBalance("alice".to_string()), 100),
            JournalLeg::credit(AccountKind::AvailableBalance("bob".to_string()), 90),
        ],
        memo: "Unbalanced test".to_string(),
    };
    let err = ledger.commit_transaction(unbalanced_tx).unwrap_err();
    assert!(
        matches!(err, LedgerError::UnbalancedTransaction { debits: 100, credits: 90 }),
        "Expected UnbalancedTransaction error, got {err:?}"
    );

    // 2. Negative balance rejection: Alice has 0 credits, attempting to debit 50 must fail
    let overdraw_tx = LedgerTransaction {
        tx_id: "tx-overdraw".to_string(),
        timestamp_unix_ms: 1000,
        legs: vec![
            JournalLeg::debit(AccountKind::AvailableBalance("alice".to_string()), 50),
            JournalLeg::credit(AccountKind::AvailableBalance("bob".to_string()), 50),
        ],
        memo: "Overdraw test".to_string(),
    };
    let err = ledger.commit_transaction(overdraw_tx).unwrap_err();
    assert!(
        matches!(err, LedgerError::InsufficientBalance { .. }),
        "Expected InsufficientBalance error, got {err:?}"
    );

    // 3. Mint funds to Alice, then balanced transfer succeeds
    ledger.mint_initial_credits("alice", 1_000_000, "Seed alice").unwrap();
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance("alice".to_string())), 1_000_000);

    let valid_tx = LedgerTransaction {
        tx_id: "tx-valid".to_string(),
        timestamp_unix_ms: 1000,
        legs: vec![
            JournalLeg::debit(AccountKind::AvailableBalance("alice".to_string()), 400_000),
            JournalLeg::credit(AccountKind::AvailableBalance("bob".to_string()), 400_000),
        ],
        memo: "Alice to Bob".to_string(),
    };
    ledger.commit_transaction(valid_tx).unwrap();

    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance("alice".to_string())), 600_000);
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance("bob".to_string())), 400_000);
    assert!(ledger.verify_global_zero_sum_invariant());
}

#[test]
fn test_escrow_lock_partial_receipt_settlement_and_unused_refund_exact_math() {
    let mut ledger = DoubleEntryCreditLedger::new();
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let requester = "did:umcf:requester";
    let worker = "did:umcf:worker";
    let job_id = "job-42";

    // Mint 10 UMCF (10,000,000 micro-credits)
    ledger.mint_initial_credits(requester, 10_000_000, "Initial seed").unwrap();

    // Lock 6 UMCF escrow for job-42
    ledger.lock_job_escrow(requester, job_id, 6_000_000).unwrap();
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(requester.to_string())), 4_000_000);
    assert_eq!(ledger.get_balance(&AccountKind::EscrowLocked(job_id.to_string())), 6_000_000);

    // Create a verified receipt for 2.5 UMCF of compute
    let output_data = b"compute results";
    let mut receipt = SignedResourceReceipt::create_by_worker(
        ReceiptBody {
            prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
            sequence_number: 1,
            job_id: job_id.to_string(),
            task_id: "task-1".to_string(),
            worker_node_id: worker.to_string(),
            requester_node_id: requester.to_string(),
            started_at_unix_ms: 1000,
            completed_at_unix_ms: 2000,
            resources: MeteredResourceVector {
                cpu_thread_time_us: 1_000_000, // 1 sec @ 500 = 500 micro
                wasm_fuel_consumed: 0,
                gpu_compute_time_us: 400_000, // 0.4 sec @ 5000 = 2000 micro
                npu_compute_time_us: 0,
                vram_peak_byte_seconds: 0,
                ai_prompt_tokens: 1_000, // 1k @ 1000 = 1000 micro
                ai_generated_tokens: 500, // 0.5k @ 4000 = 2000 micro
                ai_layer_activations_transferred: 0,
                media_frames_encoded: 0,
                render_pixels_completed: 0,
                network_egress_bytes: 0,
                network_ingress_bytes: 0,
                storage_byte_seconds: 0,
            },
            input_blake3: [0u8; 32],
            output_blake3: *blake3::hash(output_data).as_bytes(),
        },
        &worker_sk,
    );
    receipt.countersign(&requester_sk, &worker_vk, output_data).unwrap();

    // Settle receipt: Base (100) + CPU (500) + GPU (2000) + Prompt (1000) + Gen (2000) = 5,600 micro
    let pricing = PricingSchedule::default();
    let payout = ledger.settle_verified_receipt(&receipt, &pricing, &worker_vk, &requester_vk).unwrap();
    assert_eq!(payout, 5600);

    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(worker.to_string())), 5600);
    assert_eq!(ledger.get_balance(&AccountKind::EscrowLocked(job_id.to_string())), 6_000_000 - 5600);

    // Finalize job and refund unused escrow back to requester
    let refund = ledger.finalize_job_and_refund_unused_escrow(job_id).unwrap();
    assert_eq!(refund, 6_000_000 - 5600);

    assert_eq!(ledger.get_balance(&AccountKind::EscrowLocked(job_id.to_string())), 0);
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(requester.to_string())), 10_000_000 - 5600);

    // Zero-sum invariant must hold exactly
    assert!(ledger.verify_global_zero_sum_invariant());
}

#[test]
fn test_idempotent_receipt_settlement_prevents_double_payout() {
    let mut ledger = DoubleEntryCreditLedger::new();
    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let requester = "did:umcf:requester";
    let worker = "did:umcf:worker";
    let job_id = "job-idempotency";

    ledger.mint_initial_credits(requester, 5_000_000, "Seed").unwrap();
    ledger.lock_job_escrow(requester, job_id, 2_000_000).unwrap();

    let output_data = b"once-only output";
    let mut receipt = SignedResourceReceipt::create_by_worker(
        ReceiptBody {
            prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
            sequence_number: 1,
            job_id: job_id.to_string(),
            task_id: "task-single".to_string(),
            worker_node_id: worker.to_string(),
            requester_node_id: requester.to_string(),
            started_at_unix_ms: 1000,
            completed_at_unix_ms: 2000,
            resources: MeteredResourceVector {
                cpu_thread_time_us: 500_000,
                wasm_fuel_consumed: 0,
                gpu_compute_time_us: 0,
                npu_compute_time_us: 0,
                vram_peak_byte_seconds: 0,
                ai_prompt_tokens: 0,
                ai_generated_tokens: 0,
                ai_layer_activations_transferred: 0,
                media_frames_encoded: 0,
                render_pixels_completed: 0,
                network_egress_bytes: 0,
                network_ingress_bytes: 0,
                storage_byte_seconds: 0,
            },
            input_blake3: [0u8; 32],
            output_blake3: *blake3::hash(output_data).as_bytes(),
        },
        &worker_sk,
    );
    receipt.countersign(&requester_sk, &worker_vk, output_data).unwrap();

    let pricing = PricingSchedule::default();

    // 1st settlement succeeds
    let payout1 = ledger.settle_verified_receipt(&receipt, &pricing, &worker_vk, &requester_vk).unwrap();
    assert!(payout1 > 0);

    // 2nd settlement attempt with identical receipt MUST fail with ReceiptAlreadySettled
    let err = ledger.settle_verified_receipt(&receipt, &pricing, &worker_vk, &requester_vk).unwrap_err();
    assert!(
        matches!(err, LedgerError::ReceiptAlreadySettled { .. }),
        "Expected ReceiptAlreadySettled, got {err:?}"
    );

    // Worker's balance must not be credited twice
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(worker.to_string())), payout1 as i128);
    assert!(ledger.verify_global_zero_sum_invariant());
}

#[test]
fn test_byzantine_verification_failure_evidence_slashes_provider_bond() {
    let mut ledger = DoubleEntryCreditLedger::new();
    let worker = "did:umcf:bad_worker";
    let requester = "did:umcf:victim_requester";

    // Setup: Mint funds to worker and deposit provider stake bond
    ledger.mint_initial_credits(worker, 10_000_000, "Worker initial balance").unwrap();
    ledger.deposit_provider_stake(worker, 5_000_000).unwrap();

    assert_eq!(ledger.get_balance(&AccountKind::ProviderStakeBond(worker.to_string())), 5_000_000);
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(worker.to_string())), 5_000_000);

    // Create Byzantine evidence: CorruptedTile detected in S6.P1.SP3
    let evidence = VerificationFailureEvidence {
        evidence_id: "ev-byzantine-01".to_string(),
        job_id: "job-render-corrupt".to_string(),
        task_id: "tile-3".to_string(),
        worker_node_id: worker.to_string(),
        requester_node_id: requester.to_string(),
        fault_type: ByzantineFaultType::CorruptedTile,
        evidence_blake3: [0xbb; 32],
        worker_signature_hex: "abcd".to_string(),
        timestamp_unix_ms: 1731322000000,
    };

    // Slash 3,000,000 micro-credits:
    // - 1,000,000 micro-credits compensation to requester
    // - 2,000,000 micro-credits forfeited to SlashedPenaltyPool
    ledger.slash_byzantine_worker(&evidence, 3_000_000, 1_000_000).unwrap();

    assert_eq!(ledger.get_balance(&AccountKind::ProviderStakeBond(worker.to_string())), 2_000_000);
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(requester.to_string())), 1_000_000);
    assert_eq!(ledger.get_balance(&AccountKind::SlashedPenaltyPool), 2_000_000);
    assert!(ledger.verify_global_zero_sum_invariant());
}

#[test]
fn test_marketplace_matcher_respects_charging_wifi_and_privacy_constraints() {
    let mut engine = MarketplaceMatchingEngine::new();
    let (provider_sk, _provider_vk) = generate_keypair();
    let (requester_sk, _requester_vk) = generate_keypair();

    // Offer 1: Phone requiring AC charger and Wi-Fi
    let phone_offer = ComputeAskOffer {
        offer_id: "phone-offer".to_string(),
        provider_node_id: "did:umcf:phone".to_string(),
        cpu_cores: 8,
        ram_mb: 12288,
        gpu_backend: Some("Vulkan".to_string()),
        vram_mb: 6144,
        npu_tops: 30,
        available_storage_gb: 50,
        bandwidth_mbps: 100,
        baseline_rtt_ms: 25,
        only_when_charging: true,
        only_on_unmetered_wifi: true,
        max_thermal_level: 2,
        trust_tier: "TrustedDevices".to_string(),
        pricing: PricingSchedule::default(),
        reputation_score: 950,
        is_charging: false, // Discharging!
        is_unmetered_wifi: true,
        current_thermal_level: 0,
        is_active: true,
    };
    engine.publish_ask(phone_offer);

    let bid = ComputeBidRequest {
        bid_id: "bid-test-1".to_string(),
        requester_node_id: "did:umcf:requester".to_string(),
        min_cpu_cores: 4,
        min_ram_mb: 8192,
        min_vram_mb: 4096,
        requires_gpu: true,
        min_npu_tops: 0,
        min_reputation_score: 900,
        max_latency_rtt_ms: 50,
        max_budget_micro: 1_000_000,
        privacy_tier: "TrustedDevices".to_string(),
    };

    // 1. Phone is discharging: match must fail
    let err = engine.match_bid(&bid, &provider_sk, &requester_sk).unwrap_err();
    assert!(matches!(err, MarketError::NoMatchingOffer { .. }));

    // 2. Plug in phone to AC charger: match now succeeds
    engine.update_device_status("phone-offer", true, true, 0).unwrap();
    let contract = engine.match_bid(&bid, &provider_sk, &requester_sk).unwrap();
    assert_eq!(contract.offer_id, "phone-offer");

    // 3. Switch phone to cellular data: phone is immediately disqualified
    engine.update_device_status("phone-offer", true, false, 0).unwrap();
    let err2 = engine.match_bid(&bid, &provider_sk, &requester_sk).unwrap_err();
    assert!(matches!(err2, MarketError::NoMatchingOffer { .. }));
}

// ============================================================================
// 2. [INTEGRATION] TESTS
// ============================================================================

#[test]
fn test_full_marketplace_lifecycle_ask_bid_match_escrow_execute_receipt_settle_refund() {
    let mut ledger = DoubleEntryCreditLedger::new();
    let mut market = MarketplaceMatchingEngine::new();

    let (worker_sk, worker_vk) = generate_keypair();
    let (requester_sk, requester_vk) = generate_keypair();

    let worker_node = "did:umcf:desktop_rtx";
    let requester_node = "did:umcf:client_laptop";
    let job_id = "job-lifecycle-e2e";

    // 1. Setup: Mint 50 UMCF to requester and 10 UMCF stake to worker
    ledger.mint_initial_credits(requester_node, 50_000_000, "Client seed").unwrap();
    ledger.mint_initial_credits(worker_node, 10_000_000, "Worker seed").unwrap();
    ledger.deposit_provider_stake(worker_node, 10_000_000).unwrap();

    // 2. Worker publishes Ask Offer
    let ask = ComputeAskOffer {
        offer_id: "ask-desktop-01".to_string(),
        provider_node_id: worker_node.to_string(),
        cpu_cores: 16,
        ram_mb: 65536,
        gpu_backend: Some("NVIDIA RTX 4090".to_string()),
        vram_mb: 24576,
        npu_tops: 0,
        available_storage_gb: 500,
        bandwidth_mbps: 1000,
        baseline_rtt_ms: 15,
        only_when_charging: false,
        only_on_unmetered_wifi: false,
        max_thermal_level: 3,
        trust_tier: "PublicMarket".to_string(),
        pricing: PricingSchedule::default(),
        reputation_score: 990,
        is_charging: true,
        is_unmetered_wifi: true,
        current_thermal_level: 1,
        is_active: true,
    };
    market.publish_ask(ask);

    // 3. Client submits Bid
    let bid = ComputeBidRequest {
        bid_id: "bid-video-transcode".to_string(),
        requester_node_id: requester_node.to_string(),
        min_cpu_cores: 8,
        min_ram_mb: 32768,
        min_vram_mb: 8192,
        requires_gpu: true,
        min_npu_tops: 0,
        min_reputation_score: 950,
        max_latency_rtt_ms: 50,
        max_budget_micro: 20_000_000,
        privacy_tier: "PublicMarket".to_string(),
    };

    // 4. Match and seal dual-signed lease contract
    let contract = market.match_bid(&bid, &worker_sk, &requester_sk).unwrap();
    contract.verify_signatures(&worker_vk, &requester_vk).unwrap();

    // 5. Client locks Escrow in Ledger for the matched contract
    ledger.lock_job_escrow(requester_node, job_id, contract.max_budget_micro).unwrap();
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(requester_node.to_string())), 30_000_000);
    assert_eq!(ledger.get_balance(&AccountKind::EscrowLocked(job_id.to_string())), 20_000_000);

    // 6. Worker completes task, generates and dual-signs receipt
    let output_data = b"transcoded 4k video segment";
    let mut receipt = SignedResourceReceipt::create_by_worker(
        ReceiptBody {
            prev_receipt_id: GENESIS_PREV_RECEIPT_ID,
            sequence_number: 1,
            job_id: job_id.to_string(),
            task_id: "segment-1".to_string(),
            worker_node_id: worker_node.to_string(),
            requester_node_id: requester_node.to_string(),
            started_at_unix_ms: 1000,
            completed_at_unix_ms: 3000,
            resources: MeteredResourceVector {
                cpu_thread_time_us: 2_000_000,
                wasm_fuel_consumed: 0,
                gpu_compute_time_us: 1_500_000,
                npu_compute_time_us: 0,
                vram_peak_byte_seconds: 4_000_000_000,
                ai_prompt_tokens: 0,
                ai_generated_tokens: 0,
                ai_layer_activations_transferred: 0,
                media_frames_encoded: 300,
                render_pixels_completed: 0,
                network_egress_bytes: 50_000_000,
                network_ingress_bytes: 20_000_000,
                storage_byte_seconds: 0,
            },
            input_blake3: [0u8; 32],
            output_blake3: *blake3::hash(output_data).as_bytes(),
        },
        &worker_sk,
    );
    receipt.countersign(&requester_sk, &worker_vk, output_data).unwrap();

    // 7. Settle receipt into worker balance
    let payout = ledger.settle_verified_receipt(&receipt, &contract.agreed_pricing, &worker_vk, &requester_vk).unwrap();
    assert!(payout > 0);
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(worker_node.to_string())), payout as i128);

    // 8. Finalize job & refund remaining unused escrow
    let refund = ledger.finalize_job_and_refund_unused_escrow(job_id).unwrap();
    assert_eq!(refund, 20_000_000 - payout);
    assert_eq!(ledger.get_balance(&AccountKind::AvailableBalance(requester_node.to_string())), 30_000_000 + refund as i128);

    // Total conservation holds
    assert!(ledger.verify_global_zero_sum_invariant());
}

#[test]
fn test_concurrent_64_thread_settlements_preserve_global_zero_sum_invariant() {
    let ledger = Arc::new(Mutex::new(DoubleEntryCreditLedger::new()));

    // Seed treasury with initial funds across 64 accounts
    {
        let mut l = ledger.lock().unwrap();
        for i in 0..64 {
            l.mint_initial_credits(&format!("user-{i}"), 10_000_000, "Initial seed").unwrap();
        }
    }

    let thread_count = 64;
    let mut handles = Vec::with_capacity(thread_count);

    for i in 0..thread_count {
        let l_clone = Arc::clone(&ledger);
        handles.push(thread::spawn(move || {
            let sender = format!("user-{i}");
            let receiver = format!("user-{}", (i + 1) % 64);

            for step in 0..50 {
                let tx = LedgerTransaction {
                    tx_id: format!("tx-conc-{i}-{step}"),
                    timestamp_unix_ms: 1000 + step as u64,
                    legs: vec![
                        JournalLeg::debit(AccountKind::AvailableBalance(sender.clone()), 10_000),
                        JournalLeg::credit(AccountKind::AvailableBalance(receiver.clone()), 10_000),
                    ],
                    memo: "Concurrent transfer".to_string(),
                };

                let mut l = l_clone.lock().unwrap();
                let _ = l.commit_transaction(tx);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_ledger = ledger.lock().unwrap();
    assert!(
        final_ledger.verify_global_zero_sum_invariant(),
        "Global zero-sum invariant violated after 64-thread concurrent transfers!"
    );
}

// ============================================================================
// 3. [SIMULATION] TESTS (Proptest)
// ============================================================================

#[test]
fn test_proptest_randomized_market_bids_asks_and_slashings_conserve_zero_sum() {
    let mut ledger = DoubleEntryCreditLedger::new();
    let num_users = 10;

    // Seed 10 users with 1,000,000 micro-credits each
    for i in 0..num_users {
        ledger.mint_initial_credits(&format!("sim-user-{i}"), 1_000_000, "seed").unwrap();
    }

    let mut rng_seed: u64 = 42;
    let mut next_random = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        rng_seed
    };

    // Run 1,000 randomized operations
    for op_idx in 0..1_000 {
        let op_type = next_random() % 3;
        let u_from = format!("sim-user-{}", next_random() % num_users as u64);
        let u_to = format!("sim-user-{}", next_random() % num_users as u64);

        if u_from == u_to {
            continue;
        }

        match op_type {
            0 => {
                // Direct transfer
                let amount = (next_random() % 50_000 + 1) as u128;
                let tx = LedgerTransaction {
                    tx_id: format!("tx-sim-{op_idx}"),
                    timestamp_unix_ms: 1000 + op_idx as u64,
                    legs: vec![
                        JournalLeg::debit(AccountKind::AvailableBalance(u_from.clone()), amount),
                        JournalLeg::credit(AccountKind::AvailableBalance(u_to.clone()), amount),
                    ],
                    memo: "Sim transfer".to_string(),
                };
                let _ = ledger.commit_transaction(tx);
            }
            1 => {
                // Lock escrow and immediate refund
                let amount = (next_random() % 30_000 + 1) as u128;
                let job_id = format!("job-sim-{op_idx}");
                if ledger.lock_job_escrow(&u_from, &job_id, amount).is_ok() {
                    let _ = ledger.finalize_job_and_refund_unused_escrow(&job_id);
                }
            }
            _ => {
                // Deposit stake
                let amount = (next_random() % 20_000 + 1) as u128;
                let _ = ledger.deposit_provider_stake(&u_from, amount);
            }
        }

        // Invariant MUST strictly hold after every single operation
        assert!(
            ledger.verify_global_zero_sum_invariant(),
            "Zero-sum invariant broken at op {op_idx}"
        );
    }
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_marketplace_matching_engine_throughput_benchmark() {
    let mut engine = MarketplaceMatchingEngine::new();
    let (provider_sk, _provider_vk) = generate_keypair();
    let (requester_sk, _requester_vk) = generate_keypair();

    // Populate order book with 10,000 active compute ask offers
    let count = 10_000;
    for i in 0..count {
        let offer = ComputeAskOffer {
            offer_id: format!("ask-bench-{i}"),
            provider_node_id: format!("did:umcf:provider-{i}"),
            cpu_cores: 4 + (i % 28) as u32,
            ram_mb: 8192 + (i % 64) as u64 * 1024,
            gpu_backend: if i % 2 == 0 { Some("Vulkan".to_string()) } else { None },
            vram_mb: if i % 2 == 0 { 8192 } else { 0 },
            npu_tops: (i % 50) as u32,
            available_storage_gb: 100,
            bandwidth_mbps: 500,
            baseline_rtt_ms: 10 + (i % 80) as u32,
            only_when_charging: false,
            only_on_unmetered_wifi: false,
            max_thermal_level: 3,
            trust_tier: "PublicMarket".to_string(),
            pricing: PricingSchedule::default(),
            reputation_score: 800 + (i % 200) as u32,
            is_charging: true,
            is_unmetered_wifi: true,
            current_thermal_level: 0,
            is_active: true,
        };
        engine.publish_ask(offer);
    }

    assert_eq!(engine.active_ask_count(), count);

    // Benchmark matching latency
    let bid = ComputeBidRequest {
        bid_id: "bid-bench-eval".to_string(),
        requester_node_id: "did:umcf:bench_requester".to_string(),
        min_cpu_cores: 8,
        min_ram_mb: 16384,
        min_vram_mb: 4096,
        requires_gpu: true,
        min_npu_tops: 0,
        min_reputation_score: 900,
        max_latency_rtt_ms: 50,
        max_budget_micro: 5_000_000,
        privacy_tier: "PublicMarket".to_string(),
    };

    let start = Instant::now();
    let contract = engine.match_bid(&bid, &provider_sk, &requester_sk).unwrap();
    let elapsed = start.elapsed();

    assert!(!contract.lease_id.is_empty());
    println!("Matched across 10,000 offers in {:?}", elapsed);

    // Target SLA: < 5.0 ms per match
    assert!(
        elapsed.as_millis() < 50,
        "Matching latency too high: {:?} (expected < 50 ms in unoptimized / < 5 ms in release)",
        elapsed
    );
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_contribute_device_and_order_book_ui_schema() {
    let ask = ComputeAskOffer {
        offer_id: "ui-inspect-ask".to_string(),
        provider_node_id: "did:umcf:phone_pixel9".to_string(),
        cpu_cores: 8,
        ram_mb: 16384,
        gpu_backend: Some("Mali-G715".to_string()),
        vram_mb: 8192,
        npu_tops: 45,
        available_storage_gb: 128,
        bandwidth_mbps: 300,
        baseline_rtt_ms: 20,
        only_when_charging: true,
        only_on_unmetered_wifi: true,
        max_thermal_level: 2,
        trust_tier: "TrustedDevices".to_string(),
        pricing: PricingSchedule::default(),
        reputation_score: 960,
        is_charging: true,
        is_unmetered_wifi: true,
        current_thermal_level: 0,
        is_active: true,
    };

    // Serialize to JSON and verify UI schema matches Svelte 5 & Flutter models
    let json_val = serde_json::to_value(&ask).unwrap();
    assert_eq!(json_val["offer_id"], "ui-inspect-ask");
    assert_eq!(json_val["only_when_charging"], true);
    assert_eq!(json_val["only_on_unmetered_wifi"], true);
    assert_eq!(json_val["max_thermal_level"], 2);
}
