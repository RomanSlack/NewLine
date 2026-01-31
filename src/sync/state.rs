use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub enabled: bool,
    pub url: String,
    pub api_key: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            api_key: String::new(),
        }
    }
}

impl SyncConfig {
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.url.is_empty() && !self.api_key.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub modified: DateTime<Utc>,
    pub etag: Option<String>,
    pub synced_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalSyncState {
    pub last_sync: Option<DateTime<Utc>>,
    pub files: HashMap<String, FileState>,
}

impl LocalSyncState {
    fn state_file_path(projects_dir: &Path) -> PathBuf {
        projects_dir.join(".sync_state.json")
    }

    pub fn load(projects_dir: &Path) -> Self {
        let path = Self::state_file_path(projects_dir);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str(&content) {
                    return state;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self, projects_dir: &Path) -> Result<()> {
        let path = Self::state_file_path(projects_dir);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn mark_synced(&mut self, path: &str, modified: DateTime<Utc>, etag: Option<String>) {
        self.files.insert(
            path.to_string(),
            FileState {
                modified,
                etag,
                synced_at: Utc::now(),
            },
        );
    }

    pub fn get_file_state(&self, path: &str) -> Option<&FileState> {
        self.files.get(path)
    }

    pub fn remove_file(&mut self, path: &str) {
        self.files.remove(path);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub sync: SyncConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sync: SyncConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("nextline"))
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = serde_json::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(dir) = Self::config_dir() {
            fs::create_dir_all(&dir)?;
            if let Some(path) = Self::config_path() {
                let content = serde_json::to_string_pretty(self)?;
                fs::write(path, content)?;
            }
        }
        Ok(())
    }
}
