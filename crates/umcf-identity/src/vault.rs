//! Production-grade encrypted local vault using Argon2id and XChaCha20-Poly1305 AEAD.
//! Strictly enforces `#![forbid(unsafe_code)]`, memory zeroization, and 100% tamper detection.

use crate::error::IdentityError;
use crate::keys::NodeKeypair;
use crate::storage::{KeyStorageBackend, PersistedIdentity};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand_core::{OsRng, RngCore};
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, Zeroizing};

/// Canonical 14-byte magic header identifying a valid UMCF encrypted vault file.
pub const VAULT_MAGIC: &[u8; 14] = b"UMCF-VAULT-v1\0";

/// Total header size in bytes before ciphertext (14 magic + 32 salt + 24 nonce + 12 params + 4 len).
pub const VAULT_HEADER_SIZE: usize = 14 + 32 + 24 + 12 + 4;

/// Maximum allowable memory cost (256 MiB) to prevent memory exhaustion DoS from corrupted headers.
pub const MAX_ARGON2_M_COST: u32 = 256 * 1024;
/// Minimum allowable memory cost for Argon2id.
pub const MIN_ARGON2_M_COST: u32 = 8;
/// Maximum allowable time iterations to prevent CPU starvation DoS.
pub const MAX_ARGON2_T_COST: u32 = 64;
/// Minimum allowable time iterations.
pub const MIN_ARGON2_T_COST: u32 = 1;
/// Maximum allowable parallelism.
pub const MAX_ARGON2_P_COST: u32 = 64;
/// Minimum allowable parallelism.
pub const MIN_ARGON2_P_COST: u32 = 1;
/// Maximum allowable ciphertext length (16 MiB).
pub const MAX_VAULT_CIPHERTEXT_LEN: usize = 16 * 1024 * 1024;

/// Argon2id key derivation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultArgon2Params {
    /// Memory cost in KiB. Default: 65536 KiB (64 MiB).
    pub m_cost: u32,
    /// Time cost in iterations. Default: 3.
    pub t_cost: u32,
    /// Parallelism (lanes). Default: 4.
    pub p_cost: u32,
}

impl Default for VaultArgon2Params {
    fn default() -> Self {
        Self {
            m_cost: 65536, // 64 MiB
            t_cost: 3,
            p_cost: 4,
        }
    }
}

impl VaultArgon2Params {
    /// Fast parameters for unit tests and microbenchmarks (`1 MiB, t=1, p=1`).
    #[must_use]
    pub const fn fast_for_testing() -> Self {
        Self {
            m_cost: 1024,
            t_cost: 1,
            p_cost: 1,
        }
    }
}

/// Encrypted local vault protecting node identities on disk.
/// Employs Argon2id password-based key derivation and XChaCha20-Poly1305 AEAD encryption.
pub struct LocalEncryptedVault {
    path: PathBuf,
    passphrase: Zeroizing<String>,
    params: VaultArgon2Params,
}

impl std::fmt::Debug for LocalEncryptedVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalEncryptedVault")
            .field("path", &self.path)
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

impl LocalEncryptedVault {
    /// Creates a new vault handle bound to a filesystem path, passphrase, and Argon2id parameters.
    #[must_use]
    pub fn new(path: PathBuf, passphrase: impl Into<String>, params: VaultArgon2Params) -> Self {
        Self {
            path,
            passphrase: Zeroizing::new(passphrase.into()),
            params,
        }
    }

    /// Returns the filesystem path to the vault file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns true if the vault file currently exists on disk.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Encrypts and writes raw bytes to the vault file atomically with 0600 permissions.
    pub fn encrypt_and_save(&self, plaintext: &[u8]) -> Result<(), IdentityError> {
        let mut salt = [0u8; 32];
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce_bytes);

        // 1. Derive 256-bit encryption key via Argon2id
        let argon2_params = Params::new(
            self.params.m_cost,
            self.params.t_cost,
            self.params.p_cost,
            Some(32),
        )
        .map_err(|e| IdentityError::StorageError(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);
        let mut derived_key = [0u8; 32];
        argon2
            .hash_password_into(self.passphrase.as_bytes(), &salt, &mut derived_key)
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;

        // 2. Encrypt plaintext using XChaCha20-Poly1305 AEAD
        let cipher = XChaCha20Poly1305::new_from_slice(&derived_key)
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        derived_key.zeroize();

        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| IdentityError::VaultIntegrityFailure(e.to_string()))?;

        let ciphertext_len = u32::try_from(ciphertext.len())
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;

        // 3. Assemble binary vault envelope
        let total_size = VAULT_HEADER_SIZE + ciphertext.len();
        let mut buffer = Vec::with_capacity(total_size);
        buffer.extend_from_slice(VAULT_MAGIC);
        buffer.extend_from_slice(&salt);
        buffer.extend_from_slice(&nonce_bytes);
        buffer.extend_from_slice(&self.params.m_cost.to_le_bytes());
        buffer.extend_from_slice(&self.params.t_cost.to_le_bytes());
        buffer.extend_from_slice(&self.params.p_cost.to_le_bytes());
        buffer.extend_from_slice(&ciphertext_len.to_le_bytes());
        buffer.extend_from_slice(&ciphertext);

        // 4. Atomic file write via temporary file
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        }

        let temp_path = self.path.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&temp_path, &buffer)
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;

        set_secure_permissions(&temp_path)?;

        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            IdentityError::StorageError(e.to_string())
        })?;

        Ok(())
    }

    /// Reads and decrypts the vault file, returning raw plaintext bytes.
    pub fn load_and_decrypt(&self) -> Result<Option<Vec<u8>>, IdentityError> {
        if !self.path.exists() {
            return Ok(None);
        }

        let file_bytes =
            std::fs::read(&self.path).map_err(|e| IdentityError::StorageError(e.to_string()))?;

        if file_bytes.len() < VAULT_HEADER_SIZE {
            return Err(IdentityError::VaultIntegrityFailure(
                "vault file is smaller than canonical header".to_string(),
            ));
        }

        // Verify magic
        if &file_bytes[0..14] != VAULT_MAGIC {
            return Err(IdentityError::VaultIntegrityFailure(
                "vault header magic mismatch or file corrupted".to_string(),
            ));
        }

        let mut salt = [0u8; 32];
        salt.copy_from_slice(&file_bytes[14..46]);

        let mut nonce_bytes = [0u8; 24];
        nonce_bytes.copy_from_slice(&file_bytes[46..70]);

        let mut m_bytes = [0u8; 4];
        m_bytes.copy_from_slice(&file_bytes[70..74]);
        let m_cost = u32::from_le_bytes(m_bytes);

        let mut t_bytes = [0u8; 4];
        t_bytes.copy_from_slice(&file_bytes[74..78]);
        let t_cost = u32::from_le_bytes(t_bytes);

        let mut p_bytes = [0u8; 4];
        p_bytes.copy_from_slice(&file_bytes[78..82]);
        let p_cost = u32::from_le_bytes(p_bytes);

        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&file_bytes[82..86]);
        let ciphertext_len = u32::from_le_bytes(len_bytes) as usize;

        // Bounded parameter validation: protect against CPU/memory exhaustion DoS
        if !(MIN_ARGON2_M_COST..=MAX_ARGON2_M_COST).contains(&m_cost)
            || !(MIN_ARGON2_T_COST..=MAX_ARGON2_T_COST).contains(&t_cost)
            || !(MIN_ARGON2_P_COST..=MAX_ARGON2_P_COST).contains(&p_cost)
        {
            return Err(IdentityError::VaultIntegrityFailure(format!(
                "corrupted or out-of-bounds Argon2id parameters: m={}, t={}, p={}",
                m_cost, t_cost, p_cost
            )));
        }

        if ciphertext_len > MAX_VAULT_CIPHERTEXT_LEN
            || file_bytes.len() != VAULT_HEADER_SIZE + ciphertext_len
        {
            return Err(IdentityError::VaultIntegrityFailure(
                "ciphertext length field does not match physical payload size or exceeds ceiling"
                    .to_string(),
            ));
        }

        let ciphertext = &file_bytes[VAULT_HEADER_SIZE..];

        // Derive key using stored Argon2id parameters
        let argon2_params = Params::new(m_cost, t_cost, p_cost, Some(32))
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);
        let mut derived_key = [0u8; 32];
        argon2
            .hash_password_into(self.passphrase.as_bytes(), &salt, &mut derived_key)
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;

        let cipher = XChaCha20Poly1305::new_from_slice(&derived_key)
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        derived_key.zeroize();

        let nonce = XNonce::from_slice(&nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            IdentityError::VaultIntegrityFailure(
                "AEAD authentication tag mismatch: ciphertext tampered or incorrect passphrase"
                    .to_string(),
            )
        })?;

        Ok(Some(plaintext))
    }
}

#[async_trait::async_trait]
impl KeyStorageBackend for LocalEncryptedVault {
    async fn store_identity(&self, identity: &PersistedIdentity) -> Result<(), IdentityError> {
        let plaintext = identity.to_encrypted_payload()?;
        self.encrypt_and_save(&plaintext)
    }

    async fn load_identity(&self) -> Result<Option<PersistedIdentity>, IdentityError> {
        match self.load_and_decrypt()? {
            Some(plaintext) => {
                let identity = PersistedIdentity::from_decrypted_payload(&plaintext)?;
                Ok(Some(identity))
            }
            None => Ok(None),
        }
    }

    async fn rotate_identity(
        &self,
        new_keypair: NodeKeypair,
    ) -> Result<PersistedIdentity, IdentityError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| IdentityError::StorageError(e.to_string()))?
            .as_millis() as u64;

        let mut identity = match self.load_identity().await? {
            Some(existing) => existing,
            None => return Err(IdentityError::KeyNotFound),
        };

        identity.rotate(new_keypair, now);
        self.store_identity(&identity).await?;
        Ok(identity)
    }

    async fn revoke_identity(&self) -> Result<(), IdentityError> {
        if self.path.exists() {
            // Overwrite file with zeroes before unlinking to prevent data remanence
            let len = std::fs::metadata(&self.path)
                .map(|m| m.len() as usize)
                .unwrap_or(VAULT_HEADER_SIZE);
            let zeroes = vec![0u8; len];
            let _ = std::fs::write(&self.path, &zeroes);
            std::fs::remove_file(&self.path)
                .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(unix)]
fn set_secure_permissions(path: &Path) -> Result<(), IdentityError> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, permissions)
        .map_err(|e| IdentityError::StorageError(format!("failed to set 0600 permissions: {e}")))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_secure_permissions(_path: &Path) -> Result<(), IdentityError> {
    Ok(())
}
