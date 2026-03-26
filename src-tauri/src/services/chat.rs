use crate::core::models::{
    ChatMessage, ChatStreamEvent, ChatStreamRequest, ChatStreamState, ChatStreamStatus,
};
use reqwest::blocking::Client;
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct ChatService {
    inner: Arc<Mutex<ChatInner>>,
}

struct ChatInner {
    next_stream_id: u64,
    streams: HashMap<String, ChatStreamRecord>,
}

struct ChatStreamRecord {
    status: ChatStreamStatus,
    cancel_requested: Arc<AtomicBool>,
}

impl ChatService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ChatInner {
                next_stream_id: 1,
                streams: HashMap::new(),
            })),
        }
    }

    pub fn start_stream(
        &self,
        app_handle: AppHandle,
        request: ChatStreamRequest,
    ) -> Result<ChatStreamStatus, String> {
        let model = request.model.trim().to_string();
        if model.is_empty() {
            return Err("model cannot be empty".to_string());
        }
        if request.messages.is_empty() {
            return Err("messages cannot be empty".to_string());
        }
        validate_messages(&request.messages)?;

        let (stream_id, cancel_requested, initial_status) = {
            let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
            let stream_id = format!("stream-{}", inner.next_stream_id);
            inner.next_stream_id = inner
                .next_stream_id
                .checked_add(1)
                .ok_or_else(|| "stream id counter overflowed".to_string())?;

            let cancel_requested = Arc::new(AtomicBool::new(false));
            let status = ChatStreamStatus {
                stream_id: stream_id.clone(),
                state: ChatStreamState::Streaming,
                model: model.clone(),
                bytes_received: 0,
                error: None,
            };
            inner.streams.insert(
                stream_id.clone(),
                ChatStreamRecord {
                    status: status.clone(),
                    cancel_requested: Arc::clone(&cancel_requested),
                },
            );
            (stream_id, cancel_requested, status)
        };

        let worker_inner = Arc::clone(&self.inner);
        thread::spawn(move || {
            run_stream_worker(
                worker_inner,
                app_handle,
                stream_id,
                request,
                cancel_requested,
            );
        });

        Ok(initial_status)
    }

    pub fn cancel_stream(&self, stream_id: &str) -> Result<ChatStreamStatus, String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        let stream = inner
            .streams
            .get_mut(stream_id)
            .ok_or_else(|| format!("unknown stream_id: {stream_id}"))?;

        if stream.status.state.is_terminal() {
            return Err(format!(
                "stream {stream_id} is already in state {:?}",
                stream.status.state
            ));
        }

        stream.cancel_requested.store(true, Ordering::SeqCst);
        Ok(stream.status.clone())
    }

    pub fn get_stream_status(&self, stream_id: &str) -> Result<ChatStreamStatus, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        inner
            .streams
            .get(stream_id)
            .map(|s| s.status.clone())
            .ok_or_else(|| format!("unknown stream_id: {stream_id}"))
    }

    pub fn get_stream_statuses(&self) -> Result<Vec<ChatStreamStatus>, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        let mut statuses: Vec<ChatStreamStatus> =
            inner.streams.values().map(|s| s.status.clone()).collect();
        statuses.sort_by(|a, b| a.stream_id.cmp(&b.stream_id));
        Ok(statuses)
    }
}

fn run_stream_worker(
    inner: Arc<Mutex<ChatInner>>,
    app_handle: AppHandle,
    stream_id: String,
    request: ChatStreamRequest,
    cancel_requested: Arc<AtomicBool>,
) {
    let server_url = request
        .server_url
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let endpoint = format!("{}/v1/chat/completions", server_url.trim_end_matches('/'));

    let payload = json!({
        "model": request.model,
        "messages": request.messages,
        "max_tokens": request.max_tokens,
        "temperature": request.temperature,
        "stream": true
    });

    emit_event(
        &app_handle,
        ChatStreamEvent {
            stream_id: stream_id.clone(),
            event_type: "started".to_string(),
            data: None,
            state: ChatStreamState::Streaming,
            error: None,
        },
    );

    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(err) => {
            fail_stream(
                &inner,
                &app_handle,
                &stream_id,
                format!("failed to build HTTP client: {err}"),
            );
            return;
        }
    };

    let response = match client.post(&endpoint).json(&payload).send() {
        Ok(response) => response,
        Err(err) => {
            fail_stream(
                &inner,
                &app_handle,
                &stream_id,
                format!("failed to start chat stream request: {err}"),
            );
            return;
        }
    };

    if !response.status().is_success() {
        fail_stream(
            &inner,
            &app_handle,
            &stream_id,
            format!(
                "chat stream request failed with HTTP status {}",
                response.status()
            ),
        );
        return;
    }

    let mut bytes_received = 0u64;
    let reader = BufReader::new(response);

    for line in reader.lines() {
        if cancel_requested.load(Ordering::SeqCst) {
            set_state(
                &inner,
                &stream_id,
                ChatStreamState::Cancelled,
                None,
                bytes_received,
            );
            emit_event(
                &app_handle,
                ChatStreamEvent {
                    stream_id: stream_id.clone(),
                    event_type: "cancelled".to_string(),
                    data: None,
                    state: ChatStreamState::Cancelled,
                    error: None,
                },
            );
            return;
        }

        let line = match line {
            Ok(line) => line,
            Err(err) => {
                fail_stream(
                    &inner,
                    &app_handle,
                    &stream_id,
                    format!("failed while reading stream response: {err}"),
                );
                return;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let chunk = if let Some(data) = line.strip_prefix("data: ") {
            data.to_string()
        } else {
            line
        };

        if chunk == "[DONE]" {
            set_state(
                &inner,
                &stream_id,
                ChatStreamState::Completed,
                None,
                bytes_received,
            );
            emit_event(
                &app_handle,
                ChatStreamEvent {
                    stream_id: stream_id.clone(),
                    event_type: "completed".to_string(),
                    data: None,
                    state: ChatStreamState::Completed,
                    error: None,
                },
            );
            return;
        }

        bytes_received = bytes_received.saturating_add(chunk.len() as u64);
        set_state(
            &inner,
            &stream_id,
            ChatStreamState::Streaming,
            None,
            bytes_received,
        );
        emit_event(
            &app_handle,
            ChatStreamEvent {
                stream_id: stream_id.clone(),
                event_type: "chunk".to_string(),
                data: Some(chunk),
                state: ChatStreamState::Streaming,
                error: None,
            },
        );
    }

    set_state(
        &inner,
        &stream_id,
        ChatStreamState::Completed,
        None,
        bytes_received,
    );
    emit_event(
        &app_handle,
        ChatStreamEvent {
            stream_id,
            event_type: "completed".to_string(),
            data: None,
            state: ChatStreamState::Completed,
            error: None,
        },
    );
}

fn emit_event(app_handle: &AppHandle, event: ChatStreamEvent) {
    let _ = app_handle.emit("chat_stream_event", event);
}

fn fail_stream(
    inner: &Arc<Mutex<ChatInner>>,
    app_handle: &AppHandle,
    stream_id: &str,
    error: String,
) {
    set_state(
        inner,
        stream_id,
        ChatStreamState::Failed,
        Some(error.clone()),
        0,
    );
    emit_event(
        app_handle,
        ChatStreamEvent {
            stream_id: stream_id.to_string(),
            event_type: "error".to_string(),
            data: None,
            state: ChatStreamState::Failed,
            error: Some(error),
        },
    );
}

fn set_state(
    inner: &Arc<Mutex<ChatInner>>,
    stream_id: &str,
    state: ChatStreamState,
    error: Option<String>,
    bytes_received: u64,
) {
    if let Ok(mut guard) = inner.lock() {
        if let Some(record) = guard.streams.get_mut(stream_id) {
            record.status.state = state;
            record.status.error = error;
            record.status.bytes_received = bytes_received;
        }
    }
}

fn validate_messages(messages: &[ChatMessage]) -> Result<(), String> {
    for msg in messages {
        let role = msg.role.trim();
        if role != "system" && role != "user" && role != "assistant" {
            return Err("message.role must be one of: system, user, assistant".to_string());
        }
        if msg.content.trim().is_empty() {
            return Err("message.content cannot be empty".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn validate_messages_accepts_allowed_roles_and_non_empty_content() {
        let messages = vec![
            message("system", "You are a helpful assistant."),
            message("user", "Hello"),
            message("assistant", "Hi there"),
        ];

        assert_eq!(validate_messages(&messages), Ok(()));
    }

    #[test]
    fn validate_messages_rejects_invalid_role() {
        let messages = vec![message("moderator", "Hello")];

        assert_eq!(
            validate_messages(&messages),
            Err("message.role must be one of: system, user, assistant".to_string())
        );
    }

    #[test]
    fn validate_messages_rejects_empty_content_after_trimming() {
        let messages = vec![message("user", "   ")];

        assert_eq!(
            validate_messages(&messages),
            Err("message.content cannot be empty".to_string())
        );
    }

    #[test]
    fn set_state_updates_existing_stream_status_fields() {
        let inner = Arc::new(Mutex::new(ChatInner {
            next_stream_id: 2,
            streams: HashMap::from([(
                "stream-1".to_string(),
                ChatStreamRecord {
                    status: ChatStreamStatus {
                        stream_id: "stream-1".to_string(),
                        state: ChatStreamState::Streaming,
                        model: "llama-3".to_string(),
                        bytes_received: 12,
                        error: None,
                    },
                    cancel_requested: Arc::new(AtomicBool::new(false)),
                },
            )]),
        }));

        set_state(
            &inner,
            "stream-1",
            ChatStreamState::Failed,
            Some("boom".to_string()),
            42,
        );

        let guard = inner.lock().expect("mutex should not be poisoned");
        let status = &guard.streams["stream-1"].status;

        assert_eq!(status.state, ChatStreamState::Failed);
        assert_eq!(status.error.as_deref(), Some("boom"));
        assert_eq!(status.bytes_received, 42);
        assert_eq!(status.stream_id, "stream-1");
        assert_eq!(status.model, "llama-3");
    }
}
