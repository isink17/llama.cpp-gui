use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub llama_server_path: String,
    #[serde(default)]
    pub model_path: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_context_size")]
    pub context_size: u32,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default)]
    pub gpu_layers: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_download_folder")]
    pub download_folder: String,
    #[serde(default)]
    pub recent_model_paths: Vec<String>,
    #[serde(default)]
    pub recent_server_paths: Vec<String>,
    #[serde(default)]
    pub recent_model_urls: Vec<String>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_context_size() -> u32 {
    4096
}
fn default_threads() -> u32 {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    (cpus / 2).max(2)
}
fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    512
}
fn default_download_folder() -> String {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return std::path::PathBuf::from(profile)
            .join("Downloads")
            .join("LLMModels")
            .to_string_lossy()
            .into_owned();
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join("Downloads")
            .join("LLMModels")
            .to_string_lossy()
            .into_owned();
    }
    "LLMModels".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llama_server_path: String::new(),
            model_path: String::new(),
            host: default_host(),
            port: default_port(),
            context_size: default_context_size(),
            threads: default_threads(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            gpu_layers: 0,
            download_folder: default_download_folder(),
            recent_model_paths: Vec::new(),
            recent_server_paths: Vec::new(),
            recent_model_urls: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub model_path: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_context_size")]
    pub context_size: u32,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default)]
    pub gpu_layers: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaServerHealthStatus {
    pub healthy: bool,
    pub status_code: Option<u16>,
    pub message: Option<String>,
    pub url: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamRequest {
    pub server_url: Option<String>,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatStreamState {
    Streaming,
    Completed,
    Cancelled,
    Failed,
}

impl ChatStreamState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamStatus {
    pub stream_id: String,
    pub state: ChatStreamState,
    pub model: String,
    pub bytes_received: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamEvent {
    pub stream_id: String,
    pub event_type: String,
    pub data: Option<String>,
    pub state: ChatStreamState,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedModelDownload {
    pub download_url: String,
    pub suggested_file_name: String,
    pub request_headers: Option<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::{
        ChatStreamState, ChatStreamStatus, DownloadState, DownloadStatus, LlamaServerHealthStatus,
        Settings,
    };
    use serde_json::{json, to_value};

    #[test]
    fn terminal_state_helpers_distinguish_active_and_finished_states() {
        assert!(!DownloadState::Downloading.is_terminal());
        assert!(DownloadState::Completed.is_terminal());
        assert!(DownloadState::Cancelled.is_terminal());
        assert!(DownloadState::Failed.is_terminal());

        assert!(!ChatStreamState::Streaming.is_terminal());
        assert!(ChatStreamState::Completed.is_terminal());
        assert!(ChatStreamState::Cancelled.is_terminal());
        assert!(ChatStreamState::Failed.is_terminal());
    }

    #[test]
    fn settings_serializes_with_camel_case_fields() {
        let settings = Settings {
            llama_server_path: "/usr/bin/llama-server".to_string(),
            model_path: "/models/test.gguf".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8080,
            context_size: 4096,
            threads: 4,
            gpu_layers: 0,
            temperature: 0.25,
            max_tokens: 1024,
            download_folder: "/downloads".to_string(),
            recent_model_paths: vec![],
            recent_server_paths: vec![],
            recent_model_urls: vec![],
        };
        let value = to_value(&settings).unwrap();

        assert_eq!(value["llamaServerPath"], json!("/usr/bin/llama-server"));
        assert_eq!(value["modelPath"], json!("/models/test.gguf"));
        assert_eq!(value["host"], json!("127.0.0.1"));
        assert_eq!(value["port"], json!(8080));
        assert_eq!(value["contextSize"], json!(4096));
        assert_eq!(value["threads"], json!(4));
        assert_eq!(value["gpuLayers"], json!(0));
        assert_eq!(value["temperature"], json!(0.25));
        assert_eq!(value["maxTokens"], json!(1024));
        assert_eq!(value["downloadFolder"], json!("/downloads"));
    }

    #[test]
    fn download_state_serializes_as_camel_case_strings() {
        assert_eq!(
            to_value(DownloadState::Downloading).unwrap(),
            json!("downloading")
        );
        assert_eq!(
            to_value(DownloadState::Completed).unwrap(),
            json!("completed")
        );
        assert_eq!(
            to_value(DownloadState::Cancelled).unwrap(),
            json!("cancelled")
        );
        assert_eq!(to_value(DownloadState::Failed).unwrap(), json!("failed"));
    }

    #[test]
    fn download_status_serializes_with_camel_case_fields() {
        let value = to_value(DownloadStatus {
            download_id: "dl-1".to_string(),
            source_url: "https://example.com/model.bin".to_string(),
            destination_path: "C:/models/model.bin".to_string(),
            state: DownloadState::Downloading,
            bytes_downloaded: 128,
            total_bytes: Some(256),
            percent_complete: Some(50.0),
            error: None,
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "downloadId": "dl-1",
                "sourceUrl": "https://example.com/model.bin",
                "destinationPath": "C:/models/model.bin",
                "state": "downloading",
                "bytesDownloaded": 128,
                "totalBytes": 256,
                "percentComplete": 50.0,
                "error": null,
            })
        );
    }

    #[test]
    fn chat_stream_state_serializes_as_camel_case_strings() {
        assert_eq!(
            to_value(ChatStreamState::Streaming).unwrap(),
            json!("streaming")
        );
        assert_eq!(
            to_value(ChatStreamState::Completed).unwrap(),
            json!("completed")
        );
        assert_eq!(
            to_value(ChatStreamState::Cancelled).unwrap(),
            json!("cancelled")
        );
        assert_eq!(to_value(ChatStreamState::Failed).unwrap(), json!("failed"));
    }

    #[test]
    fn chat_stream_status_serializes_with_camel_case_fields() {
        let value = to_value(ChatStreamStatus {
            stream_id: "stream-1".to_string(),
            state: ChatStreamState::Streaming,
            model: "llama-3.1".to_string(),
            bytes_received: 4096,
            error: Some("connection lost".to_string()),
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "streamId": "stream-1",
                "state": "streaming",
                "model": "llama-3.1",
                "bytesReceived": 4096,
                "error": "connection lost",
            })
        );
    }

    #[test]
    fn health_status_serializes_with_camel_case_fields() {
        let value = to_value(LlamaServerHealthStatus {
            healthy: true,
            status_code: Some(200),
            message: Some("ok".to_string()),
            url: "http://127.0.0.1:8080/health".to_string(),
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "healthy": true,
                "statusCode": 200,
                "message": "ok",
                "url": "http://127.0.0.1:8080/health",
            })
        );
    }
}
