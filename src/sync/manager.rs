use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::client::SyncClient;
use super::state::{AppConfig, LocalSyncState};

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub uploaded: Vec<String>,
    pub downloaded: Vec<String>,
    pub deleted_local: Vec<String>,
    pub deleted_remote: Vec<String>,
    pub errors: Vec<String>,
}

impl SyncResult {
    pub fn is_empty(&self) -> bool {
        self.uploaded.is_empty()
            && self.downloaded.is_empty()
            && self.deleted_local.is_empty()
            && self.deleted_remote.is_empty()
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.uploaded.is_empty() {
            parts.push(format!("{} uploaded", self.uploaded.len()));
        }
        if !self.downloaded.is_empty() {
            parts.push(format!("{} downloaded", self.downloaded.len()));
        }
        if !self.deleted_local.is_empty() {
            parts.push(format!("{} deleted locally", self.deleted_local.len()));
        }
        if !self.deleted_remote.is_empty() {
            parts.push(format!("{} deleted remotely", self.deleted_remote.len()));
        }
        if parts.is_empty() {
            "Already in sync".to_string()
        } else {
            parts.join(", ")
        }
    }
}

pub struct SyncManager {
    client: SyncClient,
    projects_dir: PathBuf,
    state: LocalSyncState,
}

impl SyncManager {
    pub fn new(projects_dir: &Path) -> Result<Self> {
        let config = AppConfig::load();

        if !config.sync.is_configured() {
            return Err(anyhow!(
                "Sync not configured. Please set up ~/.config/nextline/config.json"
            ));
        }

        let client = SyncClient::new(&config.sync.url, &config.sync.api_key)?;
        let state = LocalSyncState::load(projects_dir);

        Ok(Self {
            client,
            projects_dir: projects_dir.to_path_buf(),
            state,
        })
    }

    pub fn test_connection(&self) -> Result<()> {
        self.client.test_connection()
    }

    /// Perform full sync using last-write-wins strategy with deletion tracking
    pub fn sync(&mut self) -> Result<SyncResult> {
        let mut result = SyncResult {
            uploaded: Vec::new(),
            downloaded: Vec::new(),
            deleted_local: Vec::new(),
            deleted_remote: Vec::new(),
            errors: Vec::new(),
        };

        // Get remote file list
        let remote_files = self.client.list_files()?;
        let remote_map: std::collections::HashMap<String, _> = remote_files
            .into_iter()
            .map(|f| (f.path.clone(), f))
            .collect();

        // Get local files
        let local_files = self.list_local_files()?;
        let local_set: HashSet<String> = local_files.iter().cloned().collect();
        let remote_set: HashSet<String> = remote_map.keys().cloned().collect();

        // Previously synced files (from our state tracking)
        let previously_synced: HashSet<String> = self.state.files.keys().cloned().collect();

        // Files only on local
        for path in local_set.difference(&remote_set) {
            if previously_synced.contains(path) {
                // Was synced before, now missing remotely = deleted on another device
                // Delete locally to propagate the deletion
                match self.delete_local_file(path) {
                    Ok(_) => result.deleted_local.push(path.clone()),
                    Err(e) => result.errors.push(format!("Delete local {}: {}", path, e)),
                }
            } else {
                // Never synced = new local file, upload it
                match self.upload_file(path) {
                    Ok(_) => result.uploaded.push(path.clone()),
                    Err(e) => result.errors.push(format!("Upload {}: {}", path, e)),
                }
            }
        }

        // Files only on remote
        for path in remote_set.difference(&local_set) {
            if previously_synced.contains(path) {
                // Was synced before, now missing locally = deleted on this device
                // Delete remotely to propagate the deletion
                match self.client.delete_file(path) {
                    Ok(_) => {
                        self.state.remove_file(path);
                        result.deleted_remote.push(path.clone());
                    }
                    Err(e) => result.errors.push(format!("Delete remote {}: {}", path, e)),
                }
            } else {
                // Never synced = new remote file, download it
                match self.download_file(path) {
                    Ok(_) => result.downloaded.push(path.clone()),
                    Err(e) => result.errors.push(format!("Download {}: {}", path, e)),
                }
            }
        }

        // Files on both - compare timestamps (last-write-wins)
        for path in local_set.intersection(&remote_set) {
            let remote_meta = remote_map.get(path).unwrap();
            let local_modified = self.get_local_modified(path)?;

            // Compare timestamps - remote wins if newer
            if remote_meta.modified > local_modified {
                match self.download_file(path) {
                    Ok(_) => result.downloaded.push(path.clone()),
                    Err(e) => result.errors.push(format!("Download {}: {}", path, e)),
                }
            } else if local_modified > remote_meta.modified {
                match self.upload_file(path) {
                    Ok(_) => result.uploaded.push(path.clone()),
                    Err(e) => result.errors.push(format!("Upload {}: {}", path, e)),
                }
            }
            // If equal, do nothing
        }

        // Sync pinned files
        if let Err(e) = self.sync_pinned() {
            result.errors.push(format!("Sync pinned: {}", e));
        }

        // Save state
        self.state.last_sync = Some(Utc::now());
        if let Err(e) = self.state.save(&self.projects_dir) {
            result.errors.push(format!("Save state: {}", e));
        }

        Ok(result)
    }

    fn list_local_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.collect_files(&self.projects_dir, &self.projects_dir, &mut files)?;
        Ok(files)
    }

    fn collect_files(&self, base: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and sync state
            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                self.collect_files(base, &path, files)?;
            } else if path.is_file() {
                // Only sync supported file types
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if ext == "txt" || ext == "md" || ext == "todo" {
                        let relative = path.strip_prefix(base)?.to_string_lossy().to_string();
                        files.push(relative);
                    }
                }
            }
        }

        Ok(())
    }

    fn get_local_modified(&self, path: &str) -> Result<DateTime<Utc>> {
        let full_path = self.projects_dir.join(path);
        let metadata = fs::metadata(&full_path)?;
        let modified = metadata.modified()?;
        Ok(DateTime::<Utc>::from(modified))
    }

    fn upload_file(&mut self, path: &str) -> Result<()> {
        let full_path = self.projects_dir.join(path);
        let content = fs::read_to_string(&full_path)?;

        let meta = self.client.put_file(path, &content)?;
        self.state
            .mark_synced(path, meta.modified, Some(meta.etag));

        Ok(())
    }

    fn download_file(&mut self, path: &str) -> Result<()> {
        let (content, meta) = self.client.get_file(path)?;
        let full_path = self.projects_dir.join(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)?;
        self.state
            .mark_synced(path, meta.modified, Some(meta.etag));

        Ok(())
    }

    fn delete_local_file(&mut self, path: &str) -> Result<()> {
        let full_path = self.projects_dir.join(path);
        if full_path.exists() {
            fs::remove_file(&full_path)?;
        }
        self.state.remove_file(path);
        Ok(())
    }

    fn sync_pinned(&mut self) -> Result<()> {
        // Read local pinned
        let pinned_path = self.projects_dir.join(".pinned");
        let local_pinned: Vec<String> = if pinned_path.exists() {
            fs::read_to_string(&pinned_path)?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        // Get remote pinned
        let remote_state = self.client.get_state()?;

        // Merge: union of both sets, preserving local order first
        let mut merged: Vec<String> = local_pinned.clone();
        for remote in &remote_state.pinned {
            if !merged.contains(remote) {
                merged.push(remote.clone());
            }
        }

        // Update remote if different
        if merged != remote_state.pinned {
            self.client.update_state(&merged)?;
        }

        // Update local if different
        if merged != local_pinned {
            fs::write(&pinned_path, merged.join("\n"))?;
        }

        Ok(())
    }
}
