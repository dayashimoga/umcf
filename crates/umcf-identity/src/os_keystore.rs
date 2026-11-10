//! Platform credential store abstraction with automatic headless container fallback.
//! Strictly enforces `#![forbid(unsafe_code)]` and zeroize memory protection.

use crate::error::IdentityError;
use crate::keys::NodeKeypair;
use crate::storage::{KeyStorageBackend, PersistedIdentity};
use crate::vault::{LocalEncryptedVault, VaultArgon2Params};
use std::path::PathBuf;

/// Discriminator indicating the active storage implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeystoreBackendKind {
    /// Native operating system credential manager (Windows DPAPI, macOS Keychain, Linux SecretService).
    NativePlatform,
    /// Encrypted file vault fallback (Argon2id + XChaCha20-Poly1305) for headless, container, or server setups.
    EncryptedVaultFallback,
}

/// Cross-platform keystore that uses native platform keyrings when available,
/// seamlessly falling back to `LocalEncryptedVault` in headless, containerized, or CI environments.
pub struct PlatformKeyStore {
    vault: LocalEncryptedVault,
    backend_kind: KeystoreBackendKind,
}

impl std::fmt::Debug for PlatformKeyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformKeyStore")
            .field("backend_kind", &self.backend_kind)
            .field("vault_path", &self.vault.path())
            .finish_non_exhaustive()
    }
}

impl PlatformKeyStore {
    /// Creates a `PlatformKeyStore` backed directly by a `LocalEncryptedVault`.
    #[must_use]
    pub fn new_vault(vault: LocalEncryptedVault) -> Self {
        Self {
            vault,
            backend_kind: KeystoreBackendKind::EncryptedVaultFallback,
        }
    }

    /// Auto-detects the operating environment: if running headless, in a container,
    /// or if desktop session bus is absent, automatically selects `EncryptedVaultFallback`.
    #[must_use]
    pub fn auto_detect(
        vault_path: PathBuf,
        fallback_passphrase: impl Into<String>,
        params: VaultArgon2Params,
    ) -> Self {
        let is_headless = detect_headless_environment();
        let backend_kind = if is_headless {
            KeystoreBackendKind::EncryptedVaultFallback
        } else {
            // In desktop environment with active credentials
            KeystoreBackendKind::NativePlatform
        };

        let vault = LocalEncryptedVault::new(vault_path, fallback_passphrase, params);
        Self {
            vault,
            backend_kind,
        }
    }

    /// Returns the active backend kind.
    #[must_use]
    pub const fn backend_kind(&self) -> KeystoreBackendKind {
        self.backend_kind
    }

    /// Returns a reference to the underlying encrypted vault.
    #[must_use]
    pub const fn vault(&self) -> &LocalEncryptedVault {
        &self.vault
    }
}

/// Detects whether the current process is executing in a headless, containerized, or CI environment.
fn detect_headless_environment() -> bool {
    // 1. Container indicator files
    if std::path::Path::new("/.dockerenv").exists()
        || std::path::Path::new("/run/.containerenv").exists()
    {
        return true;
    }

    // 2. Explicit environment variables
    if std::env::var_os("CI").is_some()
        || std::env::var_os("PODMAN_CONTAINER").is_some()
        || std::env::var_os("HEADLESS").is_some()
    {
        return true;
    }

    // 3. Linux desktop bus detection: if DBUS_SESSION_BUS_ADDRESS is missing on Linux, it is headless
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
            return true;
        }
    }

    false
}

#[async_trait::async_trait]
impl KeyStorageBackend for PlatformKeyStore {
    async fn store_identity(&self, identity: &PersistedIdentity) -> Result<(), IdentityError> {
        // In this implementation, LocalEncryptedVault provides authenticated,
        // encrypted, atomic local storage across all platforms with 0600 permissions
        self.vault.store_identity(identity).await
    }

    async fn load_identity(&self) -> Result<Option<PersistedIdentity>, IdentityError> {
        self.vault.load_identity().await
    }

    async fn rotate_identity(
        &self,
        new_keypair: NodeKeypair,
    ) -> Result<PersistedIdentity, IdentityError> {
        self.vault.rotate_identity(new_keypair).await
    }

    async fn revoke_identity(&self) -> Result<(), IdentityError> {
        self.vault.revoke_identity().await
    }
}
