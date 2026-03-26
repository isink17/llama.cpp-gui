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
        let value = to_value(Settings {
            server_url: "http://localhost:8080".to_string(),
            max_tokens: 1024,
            temperature: 0.25,
        })
        .unwrap();

        assert_eq!(
            value,
            json!({
                "serverUrl": "http://localhost:8080",
                "maxTokens": 1024,
                "temperature": 0.25,
            })
        );
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
