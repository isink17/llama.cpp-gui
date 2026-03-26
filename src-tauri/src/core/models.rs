use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub server_url: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 512,
            temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaProcessStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub last_exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
    Downloading,
    Completed,
    Cancelled,
    Failed,
}

impl DownloadState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStatus {
    pub download_id: String,
    pub source_url: String,
    pub destination_path: String,
    pub state: DownloadState,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percent_complete: Option<f64>,
    pub error: Option<String>,
}
