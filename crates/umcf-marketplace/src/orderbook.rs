//! Compute marketplace order book, constraint-aware matching engine, and lease contracts.

use std::collections::HashMap;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use umcf_ledger::PricingSchedule;

use crate::error::MarketError;

/// An advertisement published by a compute provider offering device capacity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeAskOffer {
    /// Unique identifier for this ask offer.
    pub offer_id: String,
    /// DID or node identifier of the offering provider.
    pub provider_node_id: String,
    /// Physical CPU cores available for compute tasks.
    pub cpu_cores: u32,
    /// Available system RAM in megabytes.
    pub ram_mb: u64,
    /// GPU backend name (e.g., "NVIDIA GeForce RTX 4090", "Apple M3 Max", "Vulkan") or None.
    pub gpu_backend: Option<String>,
    /// Dedicated VRAM in megabytes.
    pub vram_mb: u64,
    /// Peak NPU TOPS rating (0 if unaccelerated).
    pub npu_tops: u32,
    /// Available storage headroom in gigabytes.
    pub available_storage_gb: u64,
    /// Upstream network bandwidth in Mbps.
    pub bandwidth_mbps: u32,
    /// Typical baseline network RTT in milliseconds.
    pub baseline_rtt_ms: u32,
    /// Mobile safety constraint: only participate when connected to external AC/charger.
    pub only_when_charging: bool,
    /// Mobile safety constraint: only participate when connected to unmetered Wi-Fi/Ethernet.
    pub only_on_unmetered_wifi: bool,
    /// Maximum allowable thermal throttle level (0=Nominal, 1=Fair, 2=Serious, 3=Severe).
    pub max_thermal_level: u8,
    /// Trust tier ("StrictLocal", "TrustedDevices", "PublicMarket").
    pub trust_tier: String,
    /// Minimum pricing schedule accepted by the provider.
    pub pricing: PricingSchedule,
    /// Historical provider reputation score (0..1000, where 1000 = 100%).
    pub reputation_score: u32,
    /// Current real-time charging status.
    pub is_charging: bool,
    /// Current real-time Wi-Fi/Ethernet unmetered status.
    pub is_unmetered_wifi: bool,
    /// Current real-time thermal level.
    pub current_thermal_level: u8,
    /// Master toggle flag enabling or pausing compute contribution.
    pub is_active: bool,
}

/// A request submitted by a client seeking compute capacity for a workload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeBidRequest {
    /// Unique identifier for this bid request.
    pub bid_id: String,
    /// DID or node identifier of the workload requester.
    pub requester_node_id: String,
    /// Minimum required CPU cores.
    pub min_cpu_cores: u32,
    /// Minimum required system RAM in megabytes.
    pub min_ram_mb: u64,
    /// Minimum required VRAM in megabytes.
    pub min_vram_mb: u64,
    /// Whether a discrete or unified hardware GPU accelerator is required.
    pub requires_gpu: bool,
    /// Minimum required NPU TOPS rating.
    pub min_npu_tops: u32,
    /// Minimum required provider reputation score (0..1000).
    pub min_reputation_score: u32,
    /// Maximum acceptable round-trip latency in milliseconds.
    pub max_latency_rtt_ms: u32,
    /// Maximum total escrow budget in micro-credits allocated for the job.
    pub max_budget_micro: u128,
    /// Required privacy/trust tier ("StrictLocal", "TrustedDevices", "PublicMarket").
    pub privacy_tier: String,
}

/// A cryptographically sealed lease contract locking agreed pricing for a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchedLeaseContract {
    /// Unique lease contract identifier.
    pub lease_id: String,
    /// Matched provider ask offer ID.
    pub offer_id: String,
    /// Matched client bid request ID.
    pub bid_id: String,
    /// Provider node identifier.
    pub provider_node_id: String,
    /// Requester node identifier.
    pub requester_node_id: String,
    /// Agreed pricing schedule locked for this lease.
    pub agreed_pricing: PricingSchedule,
    /// Maximum escrow budget allocated for this lease in micro-credits.
    pub max_budget_micro: u128,
    /// Hex-encoded Ed25519 signature from the compute provider.
    pub provider_signature: String,
    /// Hex-encoded Ed25519 countersignature from the workload requester.
    pub requester_signature: String,
    /// UNIX timestamp in milliseconds when the contract was matched and signed.
    pub matched_at_unix_ms: u64,
}

impl MatchedLeaseContract {
    /// Computes the canonical BLAKE3 commitment digest over the contract's immutable fields.
    pub fn compute_canonical_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"UMCF_LEASE_CONTRACT_V1");
        hasher.update(self.lease_id.as_bytes());
        hasher.update(self.offer_id.as_bytes());
        hasher.update(self.bid_id.as_bytes());
        hasher.update(self.provider_node_id.as_bytes());
        hasher.update(self.requester_node_id.as_bytes());
        hasher.update(&self.max_budget_micro.to_le_bytes());
        hasher.update(&self.matched_at_unix_ms.to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Verifies both provider and requester Ed25519 signatures.
    pub fn verify_signatures(
        &self,
        provider_vk: &VerifyingKey,
        requester_vk: &VerifyingKey,
    ) -> Result<(), MarketError> {
        let digest = self.compute_canonical_digest();

        // 1. Verify provider signature
        let p_sig_bytes = hex::decode(&self.provider_signature).map_err(|e| {
            MarketError::InvalidContractSignature(format!("Malformed provider signature hex: {e}"))
        })?;
        if p_sig_bytes.len() != 64 {
            return Err(MarketError::InvalidContractSignature(
                "Invalid provider signature length".to_string(),
            ));
        }
        let mut p_arr = [0u8; 64];
        p_arr.copy_from_slice(&p_sig_bytes);
        let p_sig = ed25519_dalek::Signature::from_bytes(&p_arr);
        provider_vk
            .verify_strict(&digest, &p_sig)
            .map_err(|e| MarketError::InvalidContractSignature(format!("Provider: {e}")))?;

        // 2. Verify requester countersignature
        let r_sig_bytes = hex::decode(&self.requester_signature).map_err(|e| {
            MarketError::InvalidContractSignature(format!("Malformed requester signature hex: {e}"))
        })?;
        if r_sig_bytes.len() != 64 {
            return Err(MarketError::InvalidContractSignature(
                "Invalid requester signature length".to_string(),
            ));
        }
        let mut r_arr = [0u8; 64];
        r_arr.copy_from_slice(&r_sig_bytes);
        let r_sig = ed25519_dalek::Signature::from_bytes(&r_arr);
        requester_vk
            .verify_strict(&digest, &r_sig)
            .map_err(|e| MarketError::InvalidContractSignature(format!("Requester: {e}")))?;

        Ok(())
    }
}

/// Compute Marketplace Order Book and Matching Engine.
#[derive(Debug, Clone, Default)]
pub struct MarketplaceMatchingEngine {
    asks: HashMap<String, ComputeAskOffer>,
    bids: HashMap<String, ComputeBidRequest>,
    contracts: Vec<MatchedLeaseContract>,
}

impl MarketplaceMatchingEngine {
    /// Initializes an empty marketplace engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of active ask offers.
    pub fn active_ask_count(&self) -> usize {
        self.asks.values().filter(|a| a.is_active).count()
    }

    /// Returns the number of registered contracts.
    pub fn contract_count(&self) -> usize {
        self.contracts.len()
    }

    /// Publishes a compute ask offer from a provider node into the order book.
    pub fn publish_ask(&mut self, offer: ComputeAskOffer) {
        self.asks.insert(offer.offer_id.clone(), offer);
    }

    /// Cancels or removes an ask offer from the order book.
    pub fn cancel_ask(&mut self, offer_id: &str) -> Option<ComputeAskOffer> {
        self.asks.remove(offer_id)
    }

    /// Retrieves an ask offer by ID.
    pub fn get_ask(&self, offer_id: &str) -> Option<&ComputeAskOffer> {
        self.asks.get(offer_id)
    }

    /// Updates dynamic mobile battery, charging, Wi-Fi, and thermal status for a provider.
    pub fn update_device_status(
        &mut self,
        offer_id: &str,
        is_charging: bool,
        is_unmetered_wifi: bool,
        current_thermal_level: u8,
    ) -> Result<(), MarketError> {
        let offer = self
            .asks
            .get_mut(offer_id)
            .ok_or_else(|| MarketError::OfferNotFound {
                offer_id: offer_id.to_string(),
            })?;

        offer.is_charging = is_charging;
        offer.is_unmetered_wifi = is_unmetered_wifi;
        offer.current_thermal_level = current_thermal_level;
        Ok(())
    }

    /// Evaluates if an ask offer satisfies all hard constraints of a bid request.
    pub fn is_offer_eligible(offer: &ComputeAskOffer, bid: &ComputeBidRequest) -> bool {
        // 1. Must be active
        if !offer.is_active {
            return false;
        }

        // 2. Mobile safety: Only when charging
        if offer.only_when_charging && !offer.is_charging {
            return false;
        }

        // 3. Mobile safety: Only on unmetered Wi-Fi
        if offer.only_on_unmetered_wifi && !offer.is_unmetered_wifi {
            return false;
        }

        // 4. Thermal throttle ceiling
        if offer.current_thermal_level > offer.max_thermal_level {
            return false;
        }

        // 5. Hardware capacity
        if offer.cpu_cores < bid.min_cpu_cores {
            return false;
        }
        if offer.ram_mb < bid.min_ram_mb {
            return false;
        }
        if bid.requires_gpu && (offer.gpu_backend.is_none() || offer.vram_mb < bid.min_vram_mb) {
            return false;
        }
        if offer.npu_tops < bid.min_npu_tops {
            return false;
        }

        // 6. Network latency constraint
        if offer.baseline_rtt_ms > bid.max_latency_rtt_ms {
            return false;
        }

        // 7. Reputation requirement
        if offer.reputation_score < bid.min_reputation_score {
            return false;
        }

        // 8. Privacy tier matching
        if bid.privacy_tier == "StrictLocal" && offer.trust_tier != "StrictLocal" {
            return false;
        }
        if bid.privacy_tier == "TrustedDevices"
            && offer.trust_tier != "StrictLocal"
            && offer.trust_tier != "TrustedDevices"
        {
            return false;
        }

        // 9. Budget feasibility: max_budget must cover at least the base task fee
        if bid.max_budget_micro < offer.pricing.base_task_fee_micro {
            return false;
        }

        true
    }

    /// Computes a holistic utility score for an eligible offer.
    ///
    /// Higher score is better:
    /// - Higher reputation (+pts)
    /// - Lower base and compute rates (+pts)
    /// - Lower latency (+pts)
    /// - Higher hardware headroom (+pts)
    pub fn compute_utility_score(offer: &ComputeAskOffer) -> u64 {
        let mut score: u64 = 0;

        // Reputation weight (0..1000) * 10 = max 10,000 pts
        score += offer.reputation_score as u64 * 10;

        // Latency penalty: lower latency = higher score (max 5,000 pts)
        let latency_bonus = 5000_u64.saturating_sub(offer.baseline_rtt_ms as u64 * 20);
        score += latency_bonus;

        // Hardware headroom bonus (max 10,000 pts)
        let hw_bonus = (offer.cpu_cores as u64 * 200)
            + (offer.ram_mb / 1024 * 100)
            + (offer.vram_mb / 1024 * 300)
            + (offer.npu_tops as u64 * 150);
        score += hw_bonus.min(10_000);

        // Cost inverse factor (lower base fee = higher score)
        let cost_bonus = 5000_u64.saturating_sub((offer.pricing.base_task_fee_micro as u64).min(5000));
        score += cost_bonus;

        score
    }

    /// Matches an incoming bid request against the current order book.
    ///
    /// Filters eligible offers, ranks by utility score, selects the best match,
    /// and generates a dual-signed `MatchedLeaseContract`.
    pub fn match_bid(
        &mut self,
        bid: &ComputeBidRequest,
        provider_sk: &SigningKey,
        requester_sk: &SigningKey,
    ) -> Result<MatchedLeaseContract, MarketError> {
        let mut eligible_offers: Vec<&ComputeAskOffer> = self
            .asks
            .values()
            .filter(|offer| Self::is_offer_eligible(offer, bid))
            .collect();

        if eligible_offers.is_empty() {
            return Err(MarketError::NoMatchingOffer {
                bid_id: bid.bid_id.clone(),
            });
        }

        // Rank by utility score descending
        eligible_offers.sort_by(|a, b| {
            let score_a = Self::compute_utility_score(a);
            let score_b = Self::compute_utility_score(b);
            score_b.cmp(&score_a)
        });

        let selected = eligible_offers[0];

        // Construct lease contract
        let lease_id = format!("lease-{}-{}", bid.bid_id, selected.offer_id);
        let mut contract = MatchedLeaseContract {
            lease_id,
            offer_id: selected.offer_id.clone(),
            bid_id: bid.bid_id.clone(),
            provider_node_id: selected.provider_node_id.clone(),
            requester_node_id: bid.requester_node_id.clone(),
            agreed_pricing: selected.pricing.clone(),
            max_budget_micro: bid.max_budget_micro,
            provider_signature: String::new(),
            requester_signature: String::new(),
            matched_at_unix_ms: 1_731_322_000_000,
        };

        // Dual-sign the canonical contract digest
        let digest = contract.compute_canonical_digest();
        let p_sig = provider_sk.sign(&digest);
        let r_sig = requester_sk.sign(&digest);

        contract.provider_signature = hex::encode(p_sig.to_bytes());
        contract.requester_signature = hex::encode(r_sig.to_bytes());

        self.contracts.push(contract.clone());
        self.bids.insert(bid.bid_id.clone(), bid.clone());

        Ok(contract)
    }
}
