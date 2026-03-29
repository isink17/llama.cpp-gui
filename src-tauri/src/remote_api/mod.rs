use crate::commands::process::{check_llama_server_health, wait_for_server_ready};
use crate::core::models::{
    ChatStreamRequest, ChatStreamStatus, DownloadStatus, HistoryEntry, LlamaProcessStatus,
    LlamaServerHealthStatus, Preset, Settings,
};
use crate::state::app_state::SharedAppState;
use crate::state::remote_events::{RemoteEvent, RemoteEventKind};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get, post};
use axum::Router;
use futures_util::stream::Stream;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tokio_stream::wrappers::BroadcastStream;

const REMOTE_BIND_ENV: &str = "LLAMACPPDESK_REMOTE_BIND";
const REMOTE_TOKEN_ENV: &str = "LLAMACPPDESK_REMOTE_TOKEN";

#[derive(Clone)]
struct RemoteState {
    app_state: SharedAppState,
    bearer_token: Arc<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteApiConfig {
    pub bind_addr: SocketAddr,
    pub bearer_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteStartRequest {
    executable_path: String,
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteDownloadRequest {
    source_url: String,
    destination_path: String,
    request_headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatStreamStartRequest {
    request: ChatStreamRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamIdRequest {
    stream_id: String,
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HealthRequest {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadyRequest {
    server_url: String,
    timeout_secs: Option<u64>,
}

impl RemoteApiConfig {
    pub fn from_env() -> Result<Option<Self>, String> {
        let bind_value = match std::env::var(REMOTE_BIND_ENV) {
            Ok(value) => value.trim().to_string(),
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(err) => {
                return Err(format!("failed to read {REMOTE_BIND_ENV}: {err}"));
            }
        };

        if bind_value.is_empty() {
            return Ok(None);
        }

        let bind_addr = bind_value
            .parse::<SocketAddr>()
            .map_err(|e| format!("invalid {REMOTE_BIND_ENV} value '{bind_value}': {e}"))?;

        let bearer_token = std::env::var(REMOTE_TOKEN_ENV)
            .map_err(|_| {
                format!("{REMOTE_TOKEN_ENV} must be set when {REMOTE_BIND_ENV} is configured")
            })?
            .trim()
            .to_string();

        if bearer_token.is_empty() {
            return Err(format!("{REMOTE_TOKEN_ENV} cannot be empty"));
        }

        Ok(Some(Self {
            bind_addr,
            bearer_token,
        }))
    }
}

pub fn spawn_remote_api_server(
    app_state: SharedAppState,
    config: RemoteApiConfig,
) -> Result<(), String> {
    let RemoteApiConfig {
        bind_addr,
        bearer_token,
    } = config;

    eprintln!("{}", startup_access_message(bind_addr));

    let state = RemoteState {
        app_state,
        bearer_token: Arc::new(bearer_token),
    };

    thread::Builder::new()
        .name("llamacppdesk-remote-api".to_string())
        .spawn(move || {
            let runtime = match Builder::new_multi_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(err) => {
                    eprintln!("failed to build remote API runtime: {err}");
                    return;
                }
            };

            runtime.block_on(async move {
                if let Err(err) = run_server(state, bind_addr).await {
                    eprintln!("remote API server stopped: {err}");
                }
            });
        })
        .map_err(|e| format!("failed to spawn remote API server: {e}"))?;

    Ok(())
}

fn startup_access_message(bind_addr: SocketAddr) -> String {
    if bind_addr.ip().is_loopback() {
        format!("remote API listening on local address {bind_addr}")
    } else {
        format!(
            "warning: remote API listening on {bind_addr}; keep the bearer token private and only expose it to trusted LAN clients"
        )
    }
}

async fn run_server(state: RemoteState, bind_addr: SocketAddr) -> Result<(), String> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/presets", get(get_presets).post(post_preset))
        .route("/api/presets/{preset_id}", delete(delete_preset))
        .route(
            "/api/history",
            get(get_history).post(post_history).delete(clear_history),
        )
        .route("/api/downloads", get(get_downloads).post(start_download))
        .route("/api/downloads/{download_id}", delete(cancel_download))
        .route("/api/chat/statuses", get(get_chat_statuses))
        .route("/api/chat/start", post(start_chat))
        .route("/api/chat/cancel", post(cancel_chat))
        .route("/api/process/status", get(get_process_status))
        .route("/api/process/logs", get(get_process_logs))
        .route("/api/process/logs", delete(clear_process_logs))
        .route("/api/process/health", post(check_process_health))
        .route("/api/process/wait-ready", post(wait_process_ready))
        .route("/api/process/start", post(start_process))
        .route("/api/process/stop", post(stop_process))
        .route("/api/process/restart", post(restart_process))
        .route("/api/events", get(events))
        .with_state(state);

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|e| format!("failed to bind remote API on {bind_addr}: {e}"))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("remote API server error: {e}"))
}

async fn health(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    authorize(&state, &headers)?;
    Ok(Json(json!({ "ok": true })))
}

async fn get_settings(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<Settings>, ApiError> {
    authorize(&state, &headers)?;
    let settings = state
        .app_state
        .persistence
        .lock()
        .load_settings()
        .map_err(|e| ApiError::internal(format!("failed to load settings: {e}")))?;
    Ok(Json(settings))
}

async fn put_settings(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(settings): Json<Settings>,
) -> Result<Json<Settings>, ApiError> {
    authorize(&state, &headers)?;
    let persistence = state.app_state.persistence.lock();
    persistence
        .save_settings(&settings)
        .map_err(|e| ApiError::internal(format!("failed to save settings: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Settings, settings.clone());
    Ok(Json(settings))
}

async fn get_presets(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Preset>>, ApiError> {
    authorize(&state, &headers)?;
    let presets = state
        .app_state
        .persistence
        .lock()
        .load_presets()
        .map_err(|e| ApiError::internal(format!("failed to load presets: {e}")))?;
    Ok(Json(presets))
}

async fn post_preset(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(preset): Json<Preset>,
) -> Result<Json<Vec<Preset>>, ApiError> {
    authorize(&state, &headers)?;
    let persistence = state.app_state.persistence.lock();
    let mut presets = persistence
        .load_presets()
        .map_err(|e| ApiError::internal(format!("failed to load presets: {e}")))?;

    if let Some(existing) = presets.iter_mut().find(|item| item.id == preset.id) {
        *existing = preset;
    } else {
        presets.push(preset);
    }

    persistence
        .save_presets(&presets)
        .map_err(|e| ApiError::internal(format!("failed to save presets: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Presets, presets.clone());
    Ok(Json(presets))
}

async fn delete_preset(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Path(preset_id): Path<String>,
) -> Result<Json<Vec<Preset>>, ApiError> {
    authorize(&state, &headers)?;
    let preset_id = preset_id.trim().to_string();
    if preset_id.is_empty() {
        return Err(ApiError::bad_request("preset_id cannot be empty"));
    }

    let persistence = state.app_state.persistence.lock();
    let mut presets = persistence
        .load_presets()
        .map_err(|e| ApiError::internal(format!("failed to load presets: {e}")))?;
    presets.retain(|item| item.id != preset_id);
    persistence
        .save_presets(&presets)
        .map_err(|e| ApiError::internal(format!("failed to save presets: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Presets, presets.clone());
    Ok(Json(presets))
}

async fn get_history(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<Vec<HistoryEntry>>, ApiError> {
    authorize(&state, &headers)?;
    let history = state
        .app_state
        .persistence
        .lock()
        .load_history()
        .map_err(|e| ApiError::internal(format!("failed to load history: {e}")))?;
    Ok(Json(history))
}

async fn post_history(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(entry): Json<HistoryEntry>,
) -> Result<Json<Vec<HistoryEntry>>, ApiError> {
    authorize(&state, &headers)?;
    let persistence = state.app_state.persistence.lock();
    let mut history = persistence
        .load_history()
        .map_err(|e| ApiError::internal(format!("failed to load history: {e}")))?;
    history.push(entry);
    persistence
        .save_history(&history)
        .map_err(|e| ApiError::internal(format!("failed to save history: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::History, history.clone());
    Ok(Json(history))
}

async fn clear_history(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    authorize(&state, &headers)?;
    let persistence = state.app_state.persistence.lock();
    let history = Vec::<HistoryEntry>::new();
    persistence
        .save_history(&history)
        .map_err(|e| ApiError::internal(format!("failed to clear history: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::History, history);
    Ok(StatusCode::NO_CONTENT)
}

async fn get_downloads(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DownloadStatus>>, ApiError> {
    authorize(&state, &headers)?;
    let downloader = state.app_state.downloader.lock();
    let statuses = downloader
        .get_download_statuses()
        .map_err(|e| ApiError::internal(format!("failed to read download statuses: {e}")))?;
    Ok(Json(statuses))
}

async fn start_download(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<RemoteDownloadRequest>,
) -> Result<Json<DownloadStatus>, ApiError> {
    authorize(&state, &headers)?;
    let downloader = state.app_state.downloader.lock();
    let status = downloader
        .start_download(
            request.source_url,
            request.destination_path,
            request.request_headers,
        )
        .map_err(|e| ApiError::internal(format!("failed to start download: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Downloads, status.clone());
    Ok(Json(status))
}

async fn cancel_download(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Path(download_id): Path<String>,
) -> Result<Json<DownloadStatus>, ApiError> {
    authorize(&state, &headers)?;
    let download_id = download_id.trim().to_string();
    if download_id.is_empty() {
        return Err(ApiError::bad_request("download_id cannot be empty"));
    }

    let downloader = state.app_state.downloader.lock();
    let status = downloader
        .cancel_download(&download_id)
        .map_err(|e| ApiError::internal(format!("failed to cancel download: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Downloads, status.clone());
    Ok(Json(status))
}

async fn get_chat_statuses(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ChatStreamStatus>>, ApiError> {
    authorize(&state, &headers)?;
    let chat = state.app_state.chat.lock();
    let statuses = chat
        .get_stream_statuses()
        .map_err(|e| ApiError::internal(format!("failed to read chat statuses: {e}")))?;
    Ok(Json(statuses))
}

async fn start_chat(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<ChatStreamStartRequest>,
) -> Result<Json<ChatStreamStatus>, ApiError> {
    authorize(&state, &headers)?;
    let chat = state.app_state.chat.lock();
    let status = chat
        .start_stream(state.app_state.remote_events.clone(), request.request)
        .map_err(|e| ApiError::internal(format!("failed to start chat stream: {e}")))?;
    Ok(Json(status))
}

async fn cancel_chat(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<StreamIdRequest>,
) -> Result<Json<ChatStreamStatus>, ApiError> {
    authorize(&state, &headers)?;
    let stream_id = request.stream_id.trim().to_string();
    if stream_id.is_empty() {
        return Err(ApiError::bad_request("stream_id cannot be empty"));
    }

    let chat = state.app_state.chat.lock();
    let status = chat
        .cancel_stream(&stream_id)
        .map_err(|e| ApiError::internal(format!("failed to cancel chat stream: {e}")))?;
    Ok(Json(status))
}

async fn get_process_status(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<LlamaProcessStatus>, ApiError> {
    authorize(&state, &headers)?;
    let mut manager = state.app_state.process_manager.lock();
    let status = manager
        .status()
        .map_err(|e| ApiError::internal(format!("failed to read process status: {e}")))?;
    Ok(Json(status))
}

async fn get_process_logs(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    authorize(&state, &headers)?;
    let manager = state.app_state.process_manager.lock();
    let limit = query.limit.unwrap_or(200).clamp(1, 2000);
    Ok(Json(manager.logs(limit)))
}

async fn clear_process_logs(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    authorize(&state, &headers)?;
    let mut manager = state.app_state.process_manager.lock();
    manager.clear_logs();
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Logs, Vec::<String>::new());
    Ok(StatusCode::NO_CONTENT)
}

async fn check_process_health(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<HealthRequest>,
) -> Result<Json<LlamaServerHealthStatus>, ApiError> {
    authorize(&state, &headers)?;
    let health = check_llama_server_health(request.url)
        .map_err(|e| ApiError::internal(format!("failed to check llama-server health: {e}")))?;
    Ok(Json(health))
}

async fn wait_process_ready(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<ReadyRequest>,
) -> Result<Json<LlamaServerHealthStatus>, ApiError> {
    authorize(&state, &headers)?;
    let health = wait_for_server_ready(request.server_url, request.timeout_secs)
        .map_err(|e| ApiError::internal(format!("failed waiting for server readiness: {e}")))?;
    Ok(Json(health))
}

async fn start_process(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<RemoteStartRequest>,
) -> Result<Json<LlamaProcessStatus>, ApiError> {
    authorize(&state, &headers)?;
    let mut manager = state.app_state.process_manager.lock();
    let status = manager
        .start(request.executable_path, request.args)
        .map_err(|e| ApiError::internal(format!("failed to start llama-server: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Process, status.clone());
    Ok(Json(status))
}

async fn stop_process(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Json<LlamaProcessStatus>, ApiError> {
    authorize(&state, &headers)?;
    let mut manager = state.app_state.process_manager.lock();
    let status = manager
        .stop()
        .map_err(|e| ApiError::internal(format!("failed to stop llama-server: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Process, status.clone());
    Ok(Json(status))
}

async fn restart_process(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(request): Json<RemoteStartRequest>,
) -> Result<Json<LlamaProcessStatus>, ApiError> {
    authorize(&state, &headers)?;
    let mut manager = state.app_state.process_manager.lock();
    let _ = manager
        .stop()
        .map_err(|e| ApiError::internal(format!("failed to stop llama-server: {e}")))?;
    let status = manager
        .start(request.executable_path, request.args)
        .map_err(|e| ApiError::internal(format!("failed to start llama-server: {e}")))?;
    state
        .app_state
        .publish_remote_event(RemoteEventKind::Process, status.clone());
    Ok(Json(status))
}

async fn events(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    authorize(&state, &headers)?;
    let receiver = state.app_state.remote_events.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|item| async move {
        match item {
            Ok(event) => Some(Ok::<Event, Infallible>(serialize_event(event))),
            Err(_) => None,
        }
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

fn serialize_event(event: RemoteEvent) -> Event {
    let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
    Event::default().event(event.kind.as_str()).data(data)
}

fn authorize(state: &RemoteState, headers: &HeaderMap) -> Result<(), ApiError> {
    let header_value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::unauthorized("missing Authorization header"))?;
    let header_value = header_value
        .to_str()
        .map_err(|_| ApiError::unauthorized("invalid Authorization header"))?
        .trim();
    let expected = format!("Bearer {}", state.bearer_token);
    if header_value != expected {
        return Err(ApiError::unauthorized("invalid bearer token"));
    }
    Ok(())
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::startup_access_message;

    #[test]
    fn startup_access_message_marks_loopback_bind_as_local() {
        let message = startup_access_message("127.0.0.1:8080".parse().unwrap());
        assert!(message.contains("local address"));
        assert!(!message.contains("warning:"));
    }

    #[test]
    fn startup_access_message_warns_for_lan_bind() {
        let message = startup_access_message("0.0.0.0:8080".parse().unwrap());
        assert!(message.contains("warning:"));
        assert!(message.contains("trusted LAN clients"));
    }
}
