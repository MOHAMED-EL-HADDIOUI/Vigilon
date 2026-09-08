use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::netparse::{parse_block_target, strip_port};

#[derive(Debug, thiserror::Error)]
pub enum BlocklistError {
    #[error("invalid IP address: {0}")]
    InvalidIp(String),
    #[error("storage: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub ip: String,
    pub comment: Option<String>,
    pub added_at: String,
}

#[derive(Clone)]
pub struct BlocklistManager {
    file_path: PathBuf,
    entries: Arc<RwLock<Vec<BlocklistEntry>>>,
    ip_set: Arc<RwLock<HashSet<String>>>,
}

impl BlocklistManager {
    pub fn new(data_dir: &Path) -> Self {
        let file_path = data_dir.join("blocklist.json");
        let mgr = Self {
            file_path,
            entries: Arc::new(RwLock::new(Vec::new())),
            ip_set: Arc::new(RwLock::new(HashSet::new())),
        };
        mgr.load_sync();
        mgr
    }

    fn load_sync(&self) {
        if let Ok(content) = std::fs::read_to_string(&self.file_path)
            && let Ok(items) = serde_json::from_str::<Vec<BlocklistEntry>>(&content)
        {
            let mut set = HashSet::new();
            for item in &items {
                set.insert(item.ip.trim().to_string());
            }
            if let Ok(mut lock) = self.entries.try_write() {
                *lock = items;
            }
            if let Ok(mut lock) = self.ip_set.try_write() {
                *lock = set;
            }
        }
    }

    pub async fn is_blocked(&self, ip: &str) -> bool {
        let clean = strip_port(ip);
        let set = self.ip_set.read().await;
        set.contains(clean)
    }

    pub async fn list(&self) -> Vec<BlocklistEntry> {
        self.entries.read().await.clone()
    }

    pub async fn add(&self, ip: String, comment: Option<String>) -> Result<(), BlocklistError> {
        let clean_ip = parse_block_target(&ip)
            .ok_or_else(|| BlocklistError::InvalidIp(ip.trim().to_string()))?;
        let entry = BlocklistEntry {
            ip: clean_ip.clone(),
            comment,
            added_at: Utc::now().to_rfc3339(),
        };

        {
            let mut entries = self.entries.write().await;
            if !entries.iter().any(|e| e.ip == clean_ip) {
                entries.push(entry);
            }
            let mut set = self.ip_set.write().await;
            set.insert(clean_ip);
        }

        self.save_async().await.map_err(BlocklistError::from)
    }

    pub async fn remove(&self, ip: &str) -> Result<bool, BlocklistError> {
        let clean_ip = strip_port(ip);
        let removed = {
            let mut entries = self.entries.write().await;
            let initial_len = entries.len();
            entries.retain(|e| e.ip != clean_ip);
            let mut set = self.ip_set.write().await;
            set.remove(clean_ip);
            entries.len() < initial_len
        };

        if removed {
            self.save_async().await?;
        }
        Ok(removed)
    }

    async fn save_async(&self) -> Result<(), std::io::Error> {
        let entries = self.entries.read().await.clone();
        if let Some(parent) = self.file_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let data = serde_json::to_string_pretty(&entries).unwrap_or_default();
        tokio::fs::write(&self.file_path, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ipv6_roundtrips_without_mangling() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = BlocklistManager::new(dir.path());
        mgr.add("2001:db8::1".into(), None).await.unwrap();
        assert!(mgr.is_blocked("2001:db8::1").await);
        assert!(mgr.is_blocked("[2001:db8::1]:443").await);
        assert!(!mgr.is_blocked("2001:db8::2").await);
        assert!(mgr.remove("2001:db8::1").await.unwrap());
        assert!(!mgr.is_blocked("2001:db8::1").await);
    }

    #[tokio::test]
    async fn invalid_entries_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = BlocklistManager::new(dir.path());
        assert!(matches!(
            mgr.add("not an ip".into(), None).await,
            Err(BlocklistError::InvalidIp(_))
        ));
        assert!(matches!(
            mgr.add("".into(), None).await,
            Err(BlocklistError::InvalidIp(_))
        ));
        assert!(mgr.list().await.is_empty());
    }
}
