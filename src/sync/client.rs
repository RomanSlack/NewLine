use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub path: String,
    pub size: u64,
    pub etag: String,
    pub modified: DateTime<Utc>,
    #[serde(rename = "contentType")]
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSyncState {
    pub pinned: Vec<String>,
    #[serde(rename = "lastSync")]
    pub last_sync: DateTime<Utc>,
    pub version: u32,
}

pub struct SyncClient {
    client: reqwest::blocking::Client,
    base_url: String,
    api_key: String,
}

impl SyncClient {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// List all files on the remote server
    pub fn list_files(&self) -> Result<Vec<FileMeta>> {
        let url = format!("{}/api/v1/files", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to list files: {} - {}", status, body));
        }

        let list: FileListResponse = response.json()?;
        Ok(list.files)
    }

    /// Get file content from remote
    pub fn get_file(&self, path: &str) -> Result<(String, FileMeta)> {
        let url = format!("{}/api/v1/files/{}", self.base_url, path);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to get file {}: {} - {}", path, status, body));
        }

        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let modified = response
            .headers()
            .get("x-modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain")
            .to_string();

        let content = response.text()?;

        let meta = FileMeta {
            path: path.to_string(),
            size: content.len() as u64,
            etag,
            modified,
            content_type,
        };

        Ok((content, meta))
    }

    /// Upload file to remote
    pub fn put_file(&self, path: &str, content: &str) -> Result<FileMeta> {
        let url = format!("{}/api/v1/files/{}", self.base_url, path);

        let response = self
            .client
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(content.to_string())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to put file {}: {} - {}", path, status, body));
        }

        let meta: FileMeta = response.json()?;
        Ok(meta)
    }

    /// Delete file from remote
    pub fn delete_file(&self, path: &str) -> Result<()> {
        let url = format!("{}/api/v1/files/{}", self.base_url, path);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "Failed to delete file {}: {} - {}",
                path,
                status,
                body
            ));
        }

        Ok(())
    }

    /// Get sync state from remote
    pub fn get_state(&self) -> Result<RemoteSyncState> {
        let url = format!("{}/api/v1/state", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to get state: {} - {}", status, body));
        }

        let state: RemoteSyncState = response.json()?;
        Ok(state)
    }

    /// Update sync state on remote
    pub fn update_state(&self, pinned: &[String]) -> Result<RemoteSyncState> {
        let url = format!("{}/api/v1/state", self.base_url);

        let body = serde_json::json!({
            "pinned": pinned
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Failed to update state: {} - {}", status, body));
        }

        let state: RemoteSyncState = response.json()?;
        Ok(state)
    }

    /// Test connection to the server
    pub fn test_connection(&self) -> Result<()> {
        let url = format!("{}/api/v1/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Connection test failed: {} - {}", status, body));
        }

        Ok(())
    }
}
