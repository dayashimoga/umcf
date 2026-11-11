//! Systematic Reed-Solomon GF(2^8) Erasure Coding & Convergent Authenticated Encryption.
//!
//! Subphase S7.P1.SP2:
//! - Convergent authenticated chunk cipher using HKDF-SHA256 and XChaCha20-Poly1305.
//! - Systematic Reed-Solomon GF(2^8) Cauchy matrix erasure codec.
//! - Tolerates up to M arbitrary data/parity shard dropouts with exact reconstruction.
//! - Shannon entropy auditing and pre-reconstruction BLAKE3 verification.

#![forbid(unsafe_code)]

use crate::error::StorageError;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Galois Field GF(2^8) Tables & Fast SIMD-Ready Arithmetic
// ---------------------------------------------------------------------------

/// Galois Field GF(2^8) lookup tables initialized with generator alpha = 0x02 and poly 0x11D.
#[derive(Debug, Clone)]
pub struct Gf256 {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Default for Gf256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Gf256 {
    /// Constructs table-driven GF(2^8) field arithmetic.
    #[must_use]
    pub const fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut val = 1u16;
        let mut i = 0usize;
        while i < 255 {
            let b = val as u8;
            exp[i] = b;
            exp[i + 255] = b;
            log[b as usize] = i as u8;
            val <<= 1;
            if (val & 0x100) != 0 {
                val ^= 0x11D;
            }
            i += 1;
        }
        exp[510] = exp[0];
        exp[511] = exp[1];
        Self { exp, log }
    }

    /// Constant-time-like multiplication in GF(2^8).
    #[inline]
    #[must_use]
    pub fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let log_a = self.log[a as usize] as usize;
            let log_b = self.log[b as usize] as usize;
            self.exp[log_a + log_b]
        }
    }

    /// Multiplicative inverse in GF(2^8). Returns 0 for 0.
    #[inline]
    #[must_use]
    pub fn inv(&self, a: u8) -> u8 {
        if a == 0 {
            0
        } else {
            let log_a = self.log[a as usize] as usize;
            self.exp[255 - log_a]
        }
    }

    /// Division in GF(2^8).
    #[inline]
    #[must_use]
    pub fn div(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            let log_a = self.log[a as usize] as usize;
            let log_b = self.log[b as usize] as usize;
            self.exp[log_a + 255 - log_b]
        }
    }
}

pub static GF: Gf256 = Gf256::new();

// ---------------------------------------------------------------------------
// Shannon Entropy Audit Helper
// ---------------------------------------------------------------------------

/// Computes the Shannon entropy of a byte slice in bits per byte (max: 8.0).
#[must_use]
pub fn shannon_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let total = data.len() as f64;
    let mut entropy = 0.0f64;
    for &count in &counts {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.log2();
        }
    }
    entropy
}

// ---------------------------------------------------------------------------
// HMAC-SHA256 & HKDF-SHA256 Primitives
// ---------------------------------------------------------------------------

/// Computes HMAC-SHA256 over arbitrary key and data.
#[must_use]
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let kh = Sha256::digest(key);
        k[..32].copy_from_slice(&kh);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let outer_hash = outer.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer_hash);
    out
}

/// Derives key and nonce via HKDF-SHA256 (RFC 5869).
#[must_use]
pub fn hkdf_sha256_expand(prk: &[u8; 32], info: &[u8], out_len: usize) -> Vec<u8> {
    let mut okm = Vec::with_capacity(out_len);
    let mut prev = Vec::new();
    let mut counter = 1u8;
    while okm.len() < out_len {
        let mut msg = Vec::with_capacity(prev.len() + info.len() + 1);
        msg.extend_from_slice(&prev);
        msg.extend_from_slice(info);
        msg.push(counter);
        let block = hmac_sha256(prk, &msg);
        prev = block.to_vec();
        let take = (out_len - okm.len()).min(32);
        okm.extend_from_slice(&block[..take]);
        counter += 1;
    }
    okm
}

// ---------------------------------------------------------------------------
// Convergent Authenticated Chunk Cipher
// ---------------------------------------------------------------------------

/// Client-Side Convergent Authenticated Chunk Cipher.
///
/// Encrypts plaintext using XChaCha20-Poly1305 with keys derived convergently
/// from `namespace_secret` and the BLAKE3 digest of the plaintext.
///
/// Within the same security namespace:
/// - Identical plaintexts produce identical ciphertexts, enabling deduplication.
///
/// Across different namespaces:
/// - The derived keys are orthogonal, preventing cross-namespace correlation.
///
/// To untrusted storage nodes without `namespace_secret`:
/// - Payloads exhibit high Shannon entropy (> 7.99 bits/byte) indistinguishable from random.
#[derive(Debug, Clone)]
pub struct ConvergentChunkCipher;

impl ConvergentChunkCipher {
    /// Context string for key expansion.
    pub const KEY_INFO: &'static [u8] = b"UMCF-CONVERGENT-KEY-v1";
    /// Context string for nonce expansion.
    pub const NONCE_INFO: &'static [u8] = b"UMCF-CONVERGENT-NONCE-v1";

    /// Derives deterministic (Key, Nonce) from namespace secret and plaintext BLAKE3 hash.
    #[must_use]
    pub fn derive_keys(
        namespace_secret: &[u8; 32],
        plaintext_blake3: &[u8; 32],
    ) -> ([u8; 32], [u8; 24]) {
        // HKDF-Extract: salt = plaintext_blake3, ikm = namespace_secret
        let prk = hmac_sha256(plaintext_blake3, namespace_secret);
        let key_bytes = hkdf_sha256_expand(&prk, Self::KEY_INFO, 32);
        let nonce_bytes = hkdf_sha256_expand(&prk, Self::NONCE_INFO, 24);

        let mut key = [0u8; 32];
        let mut nonce = [0u8; 24];
        key.copy_from_slice(&key_bytes[..32]);
        nonce.copy_from_slice(&nonce_bytes[..24]);
        (key, nonce)
    }

    /// Encrypts plaintext using convergent XChaCha20-Poly1305 AEAD.
    /// Returns `(ciphertext_with_poly1305_tag, plaintext_blake3)`.
    pub fn encrypt(
        namespace_secret: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, [u8; 32]), StorageError> {
        let plaintext_hash = *blake3::hash(plaintext).as_bytes();
        let (key_bytes, nonce_bytes) = Self::derive_keys(namespace_secret, &plaintext_hash);

        let cipher = XChaCha20Poly1305::new_from_slice(&key_bytes).map_err(|e| {
            StorageError::CipherError {
                reason: e.to_string(),
            }
        })?;
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext =
            cipher
                .encrypt(nonce, plaintext)
                .map_err(|e| StorageError::CipherError {
                    reason: e.to_string(),
                })?;

        Ok((ciphertext, plaintext_hash))
    }

    /// Decrypts ciphertext and verifies both Poly1305 AEAD tag and BLAKE3 plaintext integrity.
    pub fn decrypt(
        namespace_secret: &[u8; 32],
        ciphertext: &[u8],
        expected_blake3: &[u8; 32],
    ) -> Result<Vec<u8>, StorageError> {
        let (key_bytes, nonce_bytes) = Self::derive_keys(namespace_secret, expected_blake3);

        let cipher = XChaCha20Poly1305::new_from_slice(&key_bytes).map_err(|e| {
            StorageError::CipherError {
                reason: e.to_string(),
            }
        })?;
        let nonce = XNonce::from_slice(&nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
            StorageError::CipherVerificationFailed {
                reason: format!("Poly1305 AEAD tag mismatch: {e}"),
            }
        })?;

        let computed_hash = *blake3::hash(&plaintext).as_bytes();
        if computed_hash != *expected_blake3 {
            return Err(StorageError::CipherVerificationFailed {
                reason: format!(
                    "Plaintext BLAKE3 mismatch: expected {}, got {}",
                    hex::encode(expected_blake3),
                    hex::encode(computed_hash)
                ),
            });
        }

        Ok(plaintext)
    }
}

// ---------------------------------------------------------------------------
// Redundancy Profile & Encrypted Shard Structure
// ---------------------------------------------------------------------------

/// Redundancy strategy governing chunk durability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedundancyProfile {
    /// Systematic Galois Field GF(2^8) Reed-Solomon Erasure Coding (K data, M parity).
    ErasureCoding {
        data_shards_k: u8,
        parity_shards_m: u8,
    },
    /// Direct multi-node full replication (primarily for small metadata manifests).
    FullReplication { replica_count: u8 },
}

impl RedundancyProfile {
    /// Balanced profile: RS(4, 2) — 1.5x storage overhead, tolerates 2 arbitrary node failures.
    #[must_use]
    pub const fn balanced_rs42() -> Self {
        Self::ErasureCoding {
            data_shards_k: 4,
            parity_shards_m: 2,
        }
    }

    /// High durability profile: RS(6, 3) — 1.5x storage overhead, tolerates 3 arbitrary node failures.
    #[must_use]
    pub const fn high_durability_rs63() -> Self {
        Self::ErasureCoding {
            data_shards_k: 6,
            parity_shards_m: 3,
        }
    }

    /// Small cluster profile: RS(2, 1) — 1.5x overhead, tolerates 1 node failure.
    #[must_use]
    pub const fn low_footprint_rs21() -> Self {
        Self::ErasureCoding {
            data_shards_k: 2,
            parity_shards_m: 1,
        }
    }

    /// 3x Full Replication for hot metadata.
    #[must_use]
    pub const fn replicated_3x() -> Self {
        Self::FullReplication { replica_count: 3 }
    }

    /// Returns total shards produced under this profile.
    #[must_use]
    pub fn total_shards(&self) -> usize {
        match *self {
            Self::ErasureCoding {
                data_shards_k,
                parity_shards_m,
            } => (data_shards_k as usize) + (parity_shards_m as usize),
            Self::FullReplication { replica_count } => replica_count as usize,
        }
    }

    /// Returns required data shards K (or 1 for replication).
    #[must_use]
    pub fn data_shards_k(&self) -> usize {
        match *self {
            Self::ErasureCoding { data_shards_k, .. } => data_shards_k as usize,
            Self::FullReplication { .. } => 1,
        }
    }

    /// Returns parity shards M (or replicas - 1 for replication).
    #[must_use]
    pub fn parity_shards_m(&self) -> usize {
        match *self {
            Self::ErasureCoding {
                parity_shards_m, ..
            } => parity_shards_m as usize,
            Self::FullReplication { replica_count } => (replica_count as usize).saturating_sub(1),
        }
    }
}

/// An individually content-addressed and encrypted shard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedShard {
    /// Zero-based shard index within the encoding generation (0..K-1 data, K..K+M-1 parity).
    pub shard_index: u8,
    /// Whether this is a parity shard (shard_index >= K).
    pub is_parity: bool,
    /// BLAKE3 digest of the shard payload.
    pub shard_hash: [u8; 32],
    /// Raw encrypted shard payload bytes.
    pub payload: Vec<u8>,
    /// Number of data shards K.
    pub data_shards_k: u8,
    /// Number of parity shards M.
    pub parity_shards_m: u8,
    /// Original unpadded ciphertext length in bytes before shard slicing.
    pub original_ciphertext_len: usize,
}

impl EncryptedShard {
    /// Validates the shard against its internal BLAKE3 hash.
    pub fn verify_integrity(&self) -> Result<(), StorageError> {
        let actual = *blake3::hash(&self.payload).as_bytes();
        if actual != self.shard_hash {
            return Err(StorageError::CorruptedShard {
                shard_index: self.shard_index,
                expected_hash: hex::encode(self.shard_hash),
                computed_hash: hex::encode(actual),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Systematic Cauchy Reed-Solomon GF(2^8) Codec
// ---------------------------------------------------------------------------

/// Systematic Cauchy Reed-Solomon GF(2^8) Erasure Codec.
///
/// Guarantees:
/// 1. Systematic: First K shards are direct, contiguous slices of the padded payload.
/// 2. Cauchy Generator: Generator matrix $P_{i,j} = \frac{1}{x_i \oplus y_j}$ where
///    $x_i = i$, $y_j = 128 + j$. Since $x_i \ne y_j$, every square submatrix is invertible!
/// 3. Inversion via Gaussian Elimination over GF(2^8).
/// 4. BLAKE3 verification before decoding: Corrupted shards are immediately filtered out.
#[derive(Debug, Clone)]
pub struct ReedSolomonErasureEngine {
    profile: RedundancyProfile,
}

impl ReedSolomonErasureEngine {
    /// Creates a new Reed-Solomon engine configured with the given redundancy profile.
    pub fn new(profile: RedundancyProfile) -> Result<Self, StorageError> {
        match profile {
            RedundancyProfile::ErasureCoding {
                data_shards_k,
                parity_shards_m,
            } => {
                if data_shards_k == 0 || parity_shards_m == 0 {
                    return Err(StorageError::ErasureCodingError {
                        reason: "data_shards_k and parity_shards_m must both be > 0".to_string(),
                    });
                }
                if (data_shards_k as usize) + (parity_shards_m as usize) > 128 {
                    return Err(StorageError::ErasureCodingError {
                        reason: "total shards (K + M) cannot exceed 128".to_string(),
                    });
                }
            }
            RedundancyProfile::FullReplication { replica_count } => {
                if replica_count == 0 {
                    return Err(StorageError::ErasureCodingError {
                        reason: "replica_count must be > 0".to_string(),
                    });
                }
            }
        }
        Ok(Self { profile })
    }

    /// Constructs the Cauchy parity generator matrix $P$ of size $M \times K$.
    ///
    /// $P_{i, j} = \text{inv}(x_i \oplus y_j)$ with $x_i = i$, $y_j = 128 + j$.
    #[must_use]
    pub fn build_cauchy_matrix(m: usize, k: usize) -> Vec<Vec<u8>> {
        let mut matrix = vec![vec![0u8; k]; m];
        for (i, row) in matrix.iter_mut().enumerate().take(m) {
            let xi = i as u8;
            for (j, cell) in row.iter_mut().enumerate().take(k) {
                let yj = 128u8 + (j as u8);
                let diff = xi ^ yj;
                *cell = GF.inv(diff);
            }
        }
        matrix
    }

    /// Inverts an $N \times N$ matrix over GF(2^8) via Gaussian elimination.
    /// Returns `StorageError::ErasureCodingError` if singular.
    pub fn invert_matrix(matrix: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, StorageError> {
        let n = matrix.len();
        if n == 0 {
            return Err(StorageError::ErasureCodingError {
                reason: "cannot invert empty matrix".to_string(),
            });
        }
        for row in matrix {
            if row.len() != n {
                return Err(StorageError::ErasureCodingError {
                    reason: "matrix must be square for inversion".to_string(),
                });
            }
        }

        // Augmented matrix [A | I_n]
        let mut aug = vec![vec![0u8; 2 * n]; n];
        for i in 0..n {
            for j in 0..n {
                aug[i][j] = matrix[i][j];
            }
            aug[i][n + i] = 1;
        }

        // Forward and backward elimination
        for col in 0..n {
            // Find pivot row
            let mut pivot_row = None;
            for (row, aug_row) in aug.iter().enumerate().take(n).skip(col) {
                if aug_row[col] != 0 {
                    pivot_row = Some(row);
                    break;
                }
            }
            let Some(pivot) = pivot_row else {
                return Err(StorageError::ErasureCodingError {
                    reason: format!("singular matrix encountered at col {col}"),
                });
            };

            // Swap if necessary
            if pivot != col {
                aug.swap(pivot, col);
            }

            // Normalize pivot row
            let pivot_val = aug[col][col];
            let pivot_inv = GF.inv(pivot_val);
            for cell in aug[col].iter_mut().take(2 * n) {
                *cell = GF.mul(*cell, pivot_inv);
            }

            // Eliminate all other rows
            for row in 0..n {
                if row != col && aug[row][col] != 0 {
                    let factor = aug[row][col];
                    let pivot_copy = aug[col].clone();
                    for (cell, &p_cell) in aug[row].iter_mut().take(2 * n).zip(pivot_copy.iter()) {
                        let sub = GF.mul(factor, p_cell);
                        *cell ^= sub;
                    }
                }
            }
        }

        // Extract right half
        let mut inv = vec![vec![0u8; n]; n];
        for i in 0..n {
            for j in 0..n {
                inv[i][j] = aug[i][n + j];
            }
        }
        Ok(inv)
    }

    /// Encodes ciphertext into systematic encrypted shards according to the redundancy profile.
    pub fn encode(&self, ciphertext: &[u8]) -> Result<Vec<EncryptedShard>, StorageError> {
        match self.profile {
            RedundancyProfile::FullReplication { replica_count } => {
                let shard_hash = *blake3::hash(ciphertext).as_bytes();
                let mut shards = Vec::with_capacity(replica_count as usize);
                for i in 0..replica_count {
                    shards.push(EncryptedShard {
                        shard_index: i,
                        is_parity: i > 0,
                        shard_hash,
                        payload: ciphertext.to_vec(),
                        data_shards_k: 1,
                        parity_shards_m: replica_count.saturating_sub(1),
                        original_ciphertext_len: ciphertext.len(),
                    });
                }
                Ok(shards)
            }
            RedundancyProfile::ErasureCoding {
                data_shards_k,
                parity_shards_m,
            } => {
                let k = data_shards_k as usize;
                let m = parity_shards_m as usize;
                let original_len = ciphertext.len();

                // Compute shard length (ceiling division)
                let shard_len = original_len.div_ceil(k).max(1);
                let padded_len = shard_len * k;

                // Create padded data buffer
                let mut padded = Vec::with_capacity(padded_len);
                padded.extend_from_slice(ciphertext);
                padded.resize(padded_len, 0u8);

                // Slice K data shards
                let mut data_shards = Vec::with_capacity(k);
                for i in 0..k {
                    let start = i * shard_len;
                    let end = start + shard_len;
                    data_shards.push(padded[start..end].to_vec());
                }

                // Compute Cauchy generator matrix P (M x K)
                let cauchy = Self::build_cauchy_matrix(m, k);

                // Compute M parity shards
                let mut parity_shards = vec![vec![0u8; shard_len]; m];
                for (cauchy_row, parity_shard) in
                    cauchy.iter().take(m).zip(parity_shards.iter_mut())
                {
                    for (&coeff, data_shard) in cauchy_row.iter().zip(data_shards.iter()) {
                        for (p_byte, &d_byte) in parity_shard.iter_mut().zip(data_shard.iter()) {
                            *p_byte ^= GF.mul(coeff, d_byte);
                        }
                    }
                }

                // Pack all K + M shards
                let total = k + m;
                let mut result = Vec::with_capacity(total);

                for (i, payload) in data_shards.into_iter().enumerate() {
                    let shard_hash = *blake3::hash(&payload).as_bytes();
                    result.push(EncryptedShard {
                        shard_index: i as u8,
                        is_parity: false,
                        shard_hash,
                        payload,
                        data_shards_k,
                        parity_shards_m,
                        original_ciphertext_len: original_len,
                    });
                }

                for (i, payload) in parity_shards.into_iter().enumerate() {
                    let shard_index = (k + i) as u8;
                    let shard_hash = *blake3::hash(&payload).as_bytes();
                    result.push(EncryptedShard {
                        shard_index,
                        is_parity: true,
                        shard_hash,
                        payload,
                        data_shards_k,
                        parity_shards_m,
                        original_ciphertext_len: original_len,
                    });
                }

                Ok(result)
            }
        }
    }

    /// Decodes and reconstructs original ciphertext from any set of valid shards.
    ///
    /// Pre-reconstruction BLAKE3 verification: Any corrupted shard is detected,
    /// rejected, and treated as missing BEFORE Reed-Solomon matrix inversion!
    pub fn decode(&self, shards: &[Option<EncryptedShard>]) -> Result<Vec<u8>, StorageError> {
        match self.profile {
            RedundancyProfile::FullReplication { .. } => {
                for shard in shards.iter().flatten() {
                    if shard.verify_integrity().is_ok() {
                        return Ok(shard.payload.clone());
                    }
                }
                Err(StorageError::InsufficientShards {
                    required: 1,
                    available: 0,
                })
            }
            RedundancyProfile::ErasureCoding {
                data_shards_k,
                parity_shards_m,
            } => {
                let k = data_shards_k as usize;
                let m = parity_shards_m as usize;

                // Validate shards and filter out corrupt ones
                let mut valid_shards: Vec<(usize, &EncryptedShard)> = Vec::new();
                for (idx, shard_opt) in shards.iter().enumerate() {
                    if let Some(shard) = shard_opt {
                        if shard.verify_integrity().is_ok() {
                            valid_shards.push((idx, shard));
                        } else {
                            tracing::warn!(
                                shard_index = shard.shard_index,
                                "Corrupted shard rejected by BLAKE3 before RS decoding"
                            );
                        }
                    }
                }

                if valid_shards.len() < k {
                    return Err(StorageError::InsufficientShards {
                        required: k,
                        available: valid_shards.len(),
                    });
                }

                let original_len = valid_shards[0].1.original_ciphertext_len;
                let shard_len = valid_shards[0].1.payload.len();

                // Fast path: Check if all K data shards are present and uncorrupted
                let mut all_data_present = true;
                let mut data_map = std::collections::HashMap::new();
                for (_, shard) in &valid_shards {
                    data_map.insert(shard.shard_index as usize, &shard.payload);
                }

                for i in 0..k {
                    if !data_map.contains_key(&i) {
                        all_data_present = false;
                        break;
                    }
                }

                if all_data_present {
                    let mut reassembled = Vec::with_capacity(k * shard_len);
                    for i in 0..k {
                        if let Some(payload) = data_map.get(&i) {
                            reassembled.extend_from_slice(payload);
                        }
                    }
                    reassembled.truncate(original_len);
                    return Ok(reassembled);
                }

                // General path: Select first K valid shards
                let selected = &valid_shards[..k];
                let cauchy = Self::build_cauchy_matrix(m, k);

                // Build K x K submatrix M
                let mut m_matrix = vec![vec![0u8; k]; k];
                for (row_idx, (_, shard)) in selected.iter().enumerate() {
                    let s_idx = shard.shard_index as usize;
                    if s_idx < k {
                        // Identity row
                        m_matrix[row_idx][s_idx] = 1;
                    } else {
                        // Cauchy parity row: index s_idx - k
                        let p_row = s_idx - k;
                        for col in 0..k {
                            m_matrix[row_idx][col] = cauchy[p_row][col];
                        }
                    }
                }

                // Invert M matrix
                let inv_m = Self::invert_matrix(&m_matrix)?;

                // Reconstruct all K data shards
                let mut reconstructed_data = vec![vec![0u8; shard_len]; k];
                for i in 0..k {
                    for (j, (_, shard)) in selected.iter().enumerate() {
                        let coeff = inv_m[i][j];
                        for (byte_idx, &val) in shard.payload.iter().enumerate().take(shard_len) {
                            reconstructed_data[i][byte_idx] ^= GF.mul(coeff, val);
                        }
                    }
                }

                // Concatenate and truncate to original length
                let mut reassembled = Vec::with_capacity(k * shard_len);
                for data_shard in reconstructed_data {
                    reassembled.extend_from_slice(&data_shard);
                }
                reassembled.truncate(original_len);
                Ok(reassembled)
            }
        }
    }

    /// Reconstructs a single missing shard from any K available shards.
    pub fn reconstruct_missing_shard(
        &self,
        shards: &[Option<EncryptedShard>],
        target_index: u8,
    ) -> Result<EncryptedShard, StorageError> {
        // Decode complete ciphertext
        let ciphertext = self.decode(shards)?;
        // Re-encode to produce all shards
        let re_encoded = self.encode(&ciphertext)?;
        for shard in re_encoded {
            if shard.shard_index == target_index {
                return Ok(shard);
            }
        }
        Err(StorageError::ErasureCodingError {
            reason: format!("target shard index {target_index} not found in re-encoded shards"),
        })
    }
}
