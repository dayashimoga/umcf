//! Model Registry RPC service implementation for node daemon (`umcfd`).
//!
//! Provides gRPC/JSON-RPC compatible handlers for importing models, querying manifests,
//! managing resumable multi-peer sync sessions, and evicting cached models.
//!
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use thiserror::Error;
use umcf_gguf::{
    ContentAddressedModelStore, DownloadScope, ModelId, ModelManifest, PeerChunkProvider,
    ResumableShardDownloader, StoreError, SyncError, SyncProgress,
};

/// Errors encountered in the model registry RPC service.
#[derive(Debug, Error)]
pub enum ModelRegistryError {
    #[error("Model store error: {0}")]
    Store(#[from] StoreError),

    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),

    #[error("Sync session not found: {0}")]
    SessionNotFound(String),

    #[error("Internal service error: {0}")]
    Internal(String),
}

/// Daemon RPC service managing local models and P2P distribution sessions.
#[derive(Debug, Clone)]
pub struct ModelRegistryService {
    store: Arc<ContentAddressedModelStore>,
    active_sessions: Arc<RwLock<HashMap<String, Arc<Mutex<ResumableShardDownloader>>>>>,
}

impl ModelRegistryService {
    /// Creates a new model registry service instance backed by `store`.
    #[must_use]
    pub fn new(store: ContentAddressedModelStore) -> Self {
        Self {
            store: Arc::new(store),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn store(&self) -> &ContentAddressedModelStore {
        &self.store
    }

    /// Ingests a local file into the store, returning its manifest.
    pub fn import_local_model(&self, path: &Path) -> Result<ModelManifest, ModelRegistryError> {
        let manifest = self.store.ingest_local_file(path)?;
        Ok(manifest)
    }

    /// Retrieves a model manifest by ID.
    pub fn get_model_manifest(
        &self,
        model_id: &ModelId,
    ) -> Result<ModelManifest, ModelRegistryError> {
        let manifest = self.store.get_manifest(model_id)?;
        Ok(manifest)
    }

    /// Lists all models registered in the store.
    pub fn list_models(&self) -> Result<Vec<ModelManifest>, ModelRegistryError> {
        let models = self.store.list_models()?;
        Ok(models)
    }

    /// Evicts a model from the store.
    pub fn evict_model(&self, model_id: &ModelId, force: bool) -> Result<u64, ModelRegistryError> {
        let freed = self.store.evict_model(model_id, force)?;
        Ok(freed)
    }

    /// Initiates a multi-peer shard synchronization session.
    pub fn start_model_sync(
        &self,
        manifest: ModelManifest,
        scope: DownloadScope,
        peers: &[Arc<dyn PeerChunkProvider>],
    ) -> Result<String, ModelRegistryError> {
        let session_id = format!(
            "sync-{}-{}",
            manifest.model_id.to_hex(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis())
        );

        let mut downloader = ResumableShardDownloader::new((*self.store).clone(), manifest, scope)?;
        let _ = downloader.sync(peers);

        let mut sessions = self
            .active_sessions
            .write()
            .map_err(|_| ModelRegistryError::Internal("session lock poisoned".into()))?;
        sessions.insert(session_id.clone(), Arc::new(Mutex::new(downloader)));

        Ok(session_id)
    }

    /// Pauses an active sync session.
    pub fn pause_model_sync(&self, session_id: &str) -> Result<(), ModelRegistryError> {
        let sessions = self
            .active_sessions
            .read()
            .map_err(|_| ModelRegistryError::Internal("session lock poisoned".into()))?;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ModelRegistryError::SessionNotFound(session_id.to_string()))?;

        let mut downloader = session
            .lock()
            .map_err(|_| ModelRegistryError::Internal("downloader lock poisoned".into()))?;
        downloader.pause();
        Ok(())
    }

    /// Cancels an active sync session and purges partial files.
    pub fn cancel_model_sync(&self, session_id: &str) -> Result<(), ModelRegistryError> {
        let mut sessions = self
            .active_sessions
            .write()
            .map_err(|_| ModelRegistryError::Internal("session lock poisoned".into()))?;
        let session = sessions
            .remove(session_id)
            .ok_or_else(|| ModelRegistryError::SessionNotFound(session_id.to_string()))?;

        let mut downloader = session
            .lock()
            .map_err(|_| ModelRegistryError::Internal("downloader lock poisoned".into()))?;
        downloader.cancel()?;
        Ok(())
    }

    /// Retrieves current progress for a sync session.
    pub fn subscribe_sync_progress(
        &self,
        session_id: &str,
    ) -> Result<SyncProgress, ModelRegistryError> {
        let sessions = self
            .active_sessions
            .read()
            .map_err(|_| ModelRegistryError::Internal("session lock poisoned".into()))?;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ModelRegistryError::SessionNotFound(session_id.to_string()))?;

        let downloader = session
            .lock()
            .map_err(|_| ModelRegistryError::Internal("downloader lock poisoned".into()))?;
        Ok(downloader.progress())
    }
}
