use crate::core::models::{
    ChatMessage, ChatStreamEvent, ChatStreamRequest, ChatStreamState, ChatStreamStatus,
};
use reqwest::blocking::Client;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::{BufRead, BufReader, ErrorKind};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatFailureKind {
    Timeout,
    Unavailable,
    Other,
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

    if !server_url.starts_with("http://") && !server_url.starts_with("https://") {
        fail_stream(
            &inner,
            &app_handle,
            &server_url,
            &stream_id,
            ChatFailureKind::Other,
            "server_url must start with http:// or https://",
        );
        return;
    }

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
                &endpoint,
                &stream_id,
                ChatFailureKind::Other,
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
                &endpoint,
                &stream_id,
                classify_reqwest_failure(&err),
                err,
            );
            return;
        }
    };

    if !response.status().is_success() {
        fail_stream(
            &inner,
            &app_handle,
            &endpoint,
            &stream_id,
            ChatFailureKind::Other,
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
                    &endpoint,
                    &stream_id,
                    classify_stream_read_failure(&err),
                    err,
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
    if let Err(err) = app_handle.emit("chat_stream_event", &event) {
        eprintln!(
            "failed to emit chat_stream_event (stream_id={}, type={}): {err}",
            event.stream_id, event.event_type
        );
    }
}

fn fail_stream(
    inner: &Arc<Mutex<ChatInner>>,
    app_handle: &AppHandle,
    endpoint: &str,
    stream_id: &str,
    kind: ChatFailureKind,
    error: impl Display,
) {
    let error = chat_failure_message(endpoint, kind, error);
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

fn chat_failure_message(endpoint: &str, kind: ChatFailureKind, error: impl Display) -> String {
    match kind {
        ChatFailureKind::Timeout => {
            format!("chat stream at {endpoint} timed out: {error}")
        }
        ChatFailureKind::Unavailable => {
            format!("chat stream at {endpoint} is unavailable: {error}")
        }
        ChatFailureKind::Other => format!("chat stream at {endpoint} failed: {error}"),
    }
}

fn classify_reqwest_failure(err: &reqwest::Error) -> ChatFailureKind {
    if err.is_timeout() {
        ChatFailureKind::Timeout
    } else if err.is_connect() {
        ChatFailureKind::Unavailable
    } else {
        ChatFailureKind::Other
    }
}

fn classify_stream_read_failure(err: &std::io::Error) -> ChatFailureKind {
    match err.kind() {
        ErrorKind::TimedOut => ChatFailureKind::Timeout,
        ErrorKind::ConnectionRefused
        | ErrorKind::ConnectionAborted
        | ErrorKind::ConnectionReset
        | ErrorKind::NotConnected
        | ErrorKind::BrokenPipe
        | ErrorKind::UnexpectedEof => ChatFailureKind::Unavailable,
        _ => ChatFailureKind::Other,
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
    use reqwest::blocking::Client;
    use std::io::Error;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

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
    fn validate_messages_rejects_mixed_valid_and_invalid_messages() {
        let messages = vec![
            message("system", "You are a helpful assistant."),
            message("user", "Hello"),
            message("assistant", "Hi there"),
            message("moderator", "This role is not allowed"),
        ];

        assert_eq!(
            validate_messages(&messages),
            Err("message.role must be one of: system, user, assistant".to_string())
        );
    }

    #[test]
    fn validate_messages_rejects_role_casing_variants() {
        let messages = vec![
            message("System", "Caps are not accepted"),
            message("USER", "Neither are uppercase roles"),
            message("assistant", "This one is valid"),
        ];

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

    #[test]
    fn set_state_ignores_unknown_stream_id_without_mutating_existing_streams() {
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
            "stream-unknown",
            ChatStreamState::Failed,
            Some("boom".to_string()),
            42,
        );

        let guard = inner.lock().expect("mutex should not be poisoned");
        let status = &guard.streams["stream-1"].status;

        assert_eq!(status.state, ChatStreamState::Streaming);
        assert_eq!(status.error, None);
        assert_eq!(status.bytes_received, 12);
        assert_eq!(status.stream_id, "stream-1");
        assert_eq!(status.model, "llama-3");
    }

    #[test]
    fn chat_failure_message_distinguishes_categories() {
        let endpoint = "http://127.0.0.1:8080/v1/chat/completions";

        assert_eq!(
            chat_failure_message(endpoint, ChatFailureKind::Timeout, "deadline exceeded"),
            "chat stream at http://127.0.0.1:8080/v1/chat/completions timed out: deadline exceeded"
        );
        assert_eq!(
            chat_failure_message(endpoint, ChatFailureKind::Unavailable, "connection refused"),
            "chat stream at http://127.0.0.1:8080/v1/chat/completions is unavailable: connection refused"
        );
        assert_eq!(
            chat_failure_message(endpoint, ChatFailureKind::Other, "unexpected EOF"),
            "chat stream at http://127.0.0.1:8080/v1/chat/completions failed: unexpected EOF"
        );
    }

    #[test]
    fn classify_stream_read_failure_maps_io_error_kinds() {
        assert_eq!(
            classify_stream_read_failure(&Error::new(ErrorKind::TimedOut, "timed out")),
            ChatFailureKind::Timeout
        );
        assert_eq!(
            classify_stream_read_failure(&Error::new(
                ErrorKind::ConnectionReset,
                "connection reset by peer"
            )),
            ChatFailureKind::Unavailable
        );
        assert_eq!(
            classify_stream_read_failure(&Error::new(ErrorKind::UnexpectedEof, "unexpected eof")),
            ChatFailureKind::Unavailable
        );
        assert_eq!(
            classify_stream_read_failure(&Error::new(ErrorKind::InvalidData, "bad response")),
            ChatFailureKind::Other
        );
    }

    #[test]
    fn classify_reqwest_failure_marks_connection_refused_unavailable() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("should bind ephemeral port");
        let port = listener.local_addr().expect("local addr available").port();
        drop(listener);

        let endpoint = format!("http://127.0.0.1:{port}/v1/chat/completions");
        let client = Client::builder().build().expect("client should build");
        let error = client
            .post(endpoint)
            .json(&serde_json::json!({
                "model": "llama-3.1",
                "messages": [message("user", "Hello")],
                "stream": true
            }))
            .send()
            .expect_err("request should fail against closed port");

        assert!(error.is_connect());
        assert_eq!(
            classify_reqwest_failure(&error),
            ChatFailureKind::Unavailable
        );
    }

    #[test]
    fn classify_reqwest_failure_marks_hanging_server_timeout() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("should bind ephemeral port");
        let port = listener.local_addr().expect("local addr available").port();

        let server = thread::spawn(move || {
            if let Ok((_socket, _addr)) = listener.accept() {
                thread::sleep(Duration::from_millis(400));
            }
        });

        let endpoint = format!("http://127.0.0.1:{port}/v1/chat/completions");
        let client = Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .expect("client should build");
        let error = client
            .post(endpoint)
            .json(&serde_json::json!({
                "model": "llama-3.1",
                "messages": [message("user", "Hello")],
                "stream": true
            }))
            .send()
            .expect_err("request should time out against hanging server");

        assert!(error.is_timeout());
        assert_eq!(classify_reqwest_failure(&error), ChatFailureKind::Timeout);
        server.join().expect("server thread should complete");
    }
}
