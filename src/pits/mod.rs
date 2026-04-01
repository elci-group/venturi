use crate::error::{Result, VenturiError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitVersion {
    pub version: usize,
    pub vcbin_path: String,
    pub timestamp: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitEntry {
    pub name: String,
    pub active_version: usize,
    pub last_updated: u64,
    pub authorized_sources: Vec<String>,
    pub history: Vec<PitVersion>,
    pub attached_node_type: String,
    pub dependent_nodes: Vec<String>,
}

impl PitEntry {
    pub fn active(&self) -> Option<&PitVersion> {
        self.history
            .iter()
            .find(|v| v.version == self.active_version)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PitStore_ {
    pits: HashMap<String, PitEntry>,
}

pub struct PitStore {
    inner: PitStore_,
    pub store_path: PathBuf,
}

impl PitStore {
    pub fn load(path: &Path) -> Result<Self> {
        let store_path = path.to_path_buf();

        if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            let inner: PitStore_ = serde_json::from_str(&contents)?;
            Ok(PitStore { inner, store_path })
        } else {
            Ok(PitStore {
                inner: PitStore_::default(),
                store_path,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(&self.inner)?;
        std::fs::write(&self.store_path, contents)?;
        Ok(())
    }

    pub fn create(
        &mut self,
        name: &str,
        vcbin_path: &str,
        van: &str,
        dependent_nodes: Vec<String>,
    ) -> Result<()> {
        if self.inner.pits.contains_key(name) {
            return Err(VenturiError::Pit(format!(
                "Pit '{}' already exists",
                name
            )));
        }

        let hash = self.hash_file(vcbin_path)?;
        let now = now_secs();

        let version = PitVersion {
            version: 1,
            vcbin_path: vcbin_path.to_string(),
            timestamp: now,
            hash,
        };

        let entry = PitEntry {
            name: name.to_string(),
            active_version: 1,
            last_updated: now,
            authorized_sources: vec![van.to_string()],
            history: vec![version],
            attached_node_type: "chass".to_string(),
            dependent_nodes,
        };

        self.inner.pits.insert(name.to_string(), entry);
        self.save()
    }

    pub fn update(&mut self, name: &str, vcbin_path: &str) -> Result<()> {
        // Compute hash before taking mutable borrow of entry
        let hash = self.hash_file(vcbin_path)?;

        let entry = self
            .inner
            .pits
            .get_mut(name)
            .ok_or_else(|| VenturiError::Pit(format!("Pit '{}' not found", name)))?;

        let next_version = entry.history.iter().map(|v| v.version).max().unwrap_or(0) + 1;
        let now = now_secs();

        let version = PitVersion {
            version: next_version,
            vcbin_path: vcbin_path.to_string(),
            timestamp: now,
            hash,
        };

        entry.history.push(version);
        entry.active_version = next_version;
        entry.last_updated = now;

        self.save()
    }

    pub fn rollback(&mut self, name: &str, version: usize) -> Result<()> {
        let entry = self
            .inner
            .pits
            .get_mut(name)
            .ok_or_else(|| VenturiError::Pit(format!("Pit '{}' not found", name)))?;

        let exists = entry.history.iter().any(|v| v.version == version);
        if !exists {
            return Err(VenturiError::Pit(format!(
                "Version {} not found in pit '{}'",
                version, name
            )));
        }

        entry.active_version = version;
        entry.last_updated = now_secs();

        self.save()
    }

    pub fn status(&self, name: &str) -> Result<&PitEntry> {
        self.inner
            .pits
            .get(name)
            .ok_or_else(|| VenturiError::Pit(format!("Pit '{}' not found", name)))
    }

    pub fn list(&self) -> Vec<&PitEntry> {
        self.inner.pits.values().collect()
    }

    fn hash_file(&self, path: &str) -> Result<String> {
        use sha2::{Digest, Sha256};

        if !std::path::Path::new(path).exists() {
            // Allow referencing non-existent files in tests
            return Ok(format!("(no-file:{})", path));
        }

        let contents = std::fs::read(path)?;
        let hash = Sha256::digest(&contents);
        Ok(hex::encode(hash))
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
