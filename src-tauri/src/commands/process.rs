use crate::core::models::{LlamaProcessStatus, LlamaServerHealthStatus};
use crate::state::app_state::SharedAppState;
use crate::state::remote_events::RemoteEventKind;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::time::{Duration, Instant};
use tauri::State;

const DEFAULT_LOG_LIMIT: usize = 200;
const MAX_LOG_LIMIT: usize = 2000;
const DEFAULT_HEALTH_URL: &str = "http://127.0.0.1:8080/health";
const DEFAULT_READY_TIMEOUT_SECS: u64 = 45;
const READY_POLL_INITIAL_MS: u64 = 100;
const READY_POLL_MAX_MS: u64 = 600;
const READY_REQUEST_TIMEOUT_SECS: u64 = 5;

#[tauri::command]
pub fn start_llama_server(
    state: State<'_, SharedAppState>,
    executable_path: String,
    args: Vec<String>,
) -> Result<LlamaProcessStatus, String> {
    let mut manager = state.process_manager.lock();
    let result = manager.start(executable_path, args);
    if let Ok(status) = &result {
        state.publish_remote_event(RemoteEventKind::Process, status.clone());
    }
    result
}

#[tauri::command]
pub fn stop_llama_server(state: State<'_, SharedAppState>) -> Result<LlamaProcessStatus, String> {
    let mut manager = state.process_manager.lock();
    let result = manager.stop();
    if let Ok(status) = &result {
        state.publish_remote_event(RemoteEventKind::Process, status.clone());
    }
    result
}

#[tauri::command]
pub fn get_llama_server_status(state: State<'_, SharedAppState>) -> Result<LlamaProcessStatus, String> {
    let mut manager = state.process_manager.lock();
    manager.status()
}

#[tauri::command]
pub fn get_llama_server_logs(
    state: State<'_, SharedAppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let manager = state.process_manager.lock();
    let limit = limit.unwrap_or(DEFAULT_LOG_LIMIT).clamp(1, MAX_LOG_LIMIT);
    Ok(manager.logs(limit))
}

#[tauri::command]
pub fn clear_llama_server_logs(state: State<'_, SharedAppState>) -> Result<(), String> {
    let mut manager = state.process_manager.lock();
    manager.clear_logs();
    state.publish_remote_event(RemoteEventKind::Logs, Vec::<String>::new());
    Ok(())
}

fn normalize_health_url(url: Option<String>) -> String {
    url.map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| DEFAULT_HEALTH_URL.to_string())
}

fn health_status_from_response(url: String, status: StatusCode) -> LlamaServerHealthStatus {
    let healthy = status.is_success();
    let message = if healthy {
        Some("llama-server is healthy".to_string())
    } else {
        Some(format!("health check failed with HTTP status {status}"))
    };

    let reason = if healthy {
        None
    } else {
        Some("unhealthy_status".to_string())
    };

    LlamaServerHealthStatus {
        healthy,
        status_code: Some(status.as_u16()),
        message,
        reason,
        url,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HealthFailureKind {
    Timeout,
    Unavailable,
    Other,
}

fn health_error_message(url: &str, kind: HealthFailureKind, err: impl std::fmt::Display) -> String {
    match kind {
        HealthFailureKind::Timeout => {
            format!("timed out waiting for llama-server at {url}: {err}")
        }
        HealthFailureKind::Unavailable => {
            format!("llama-server at {url} is unavailable: {err}")
        }
        HealthFailureKind::Other => format!("failed to reach llama-server at {url}: {err}"),
    }
}

fn reason_from_failure_kind(kind: HealthFailureKind) -> String {
    match kind {
        HealthFailureKind::Timeout => "timeout".to_string(),
        HealthFailureKind::Unavailable => "network_unreachable".to_string(),
        HealthFailureKind::Other => "unknown".to_string(),
    }
}

fn health_status_from_error(
    url: String,
    kind: HealthFailureKind,
    err: impl std::fmt::Display,
) -> LlamaServerHealthStatus {
    LlamaServerHealthStatus {
        healthy: false,
        status_code: None,
        message: Some(health_error_message(&url, kind, err)),
        reason: Some(reason_from_failure_kind(kind)),
        url,
    }
}

fn classify_health_failure(err: &reqwest::Error) -> HealthFailureKind {
    if err.is_timeout() {
        HealthFailureKind::Timeout
    } else if err.is_connect() {
        HealthFailureKind::Unavailable
    } else {
        HealthFailureKind::Other
    }
}

#[tauri::command]
pub fn check_llama_server_health(url: Option<String>) -> Result<LlamaServerHealthStatus, String> {
    let url = normalize_health_url(url);

    let client = Client::builder()
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    match client.get(&url).send() {
        Ok(response) => Ok(health_status_from_response(url, response.status())),
        Err(err) => {
            let kind = classify_health_failure(&err);
            Ok(health_status_from_error(url, kind, err))
        }
    }
}

fn validate_server_url(url: &str) -> Result<(), String> {
    if url
        .chars()
        .any(|c| c.is_whitespace() || c == ';' || c == '&' || c == '|')
    {
        return Err("invalid server URL".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn wait_for_server_ready(
    server_url: String,
    timeout_secs: Option<u64>,
) -> Result<LlamaServerHealthStatus, String> {
    validate_server_url(&server_url)?;
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_READY_TIMEOUT_SECS));
    let mut interval = Duration::from_millis(READY_POLL_INITIAL_MS);
    let base = server_url.trim_end_matches('/');
    let health_url = format!("{base}/health");
    let models_url = format!("{base}/v1/models");

    let client = Client::builder()
        .timeout(Duration::from_secs(READY_REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let start = Instant::now();

    while start.elapsed() < timeout {
        if let Ok(resp) = client.get(&health_url).send() {
            if resp.status().is_success() {
                return Ok(LlamaServerHealthStatus {
                    healthy: true,
                    status_code: Some(resp.status().as_u16()),
                    message: Some("Server is ready".to_string()),
                    reason: None,
                    url: health_url,
                });
            }
        }

        if let Ok(resp) = client.get(&models_url).send() {
            if resp.status().is_success() {
                return Ok(LlamaServerHealthStatus {
                    healthy: true,
                    status_code: Some(resp.status().as_u16()),
                    message: Some("Server is ready".to_string()),
                    reason: None,
                    url: models_url,
                });
            }
        }

        std::thread::sleep(interval);
        interval = (interval * 2).min(Duration::from_millis(READY_POLL_MAX_MS));
    }

    Err(format!(
        "Timed out waiting for server to be ready ({}s)",
        timeout.as_secs()
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        classify_health_failure, health_error_message, health_status_from_error,
        health_status_from_response, normalize_health_url, HealthFailureKind, DEFAULT_HEALTH_URL,
    };
    use reqwest::blocking::Client;
    use reqwest::StatusCode;
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn normalize_health_url_defaults_when_missing_or_blank() {
        assert_eq!(normalize_health_url(None), DEFAULT_HEALTH_URL);
        assert_eq!(
            normalize_health_url(Some("   ".to_string())),
            DEFAULT_HEALTH_URL
        );
        assert_eq!(
            normalize_health_url(Some("\n\t  ".to_string())),
            DEFAULT_HEALTH_URL
        );
    }

    #[test]
    fn normalize_health_url_trims_non_empty_input() {
        assert_eq!(
            normalize_health_url(Some(
                "\n\t  http://127.0.0.1:9000/health?foo=bar  \t".to_string()
            )),
            "http://127.0.0.1:9000/health?foo=bar"
        );
    }

    #[test]
    fn health_status_from_response_maps_success_and_failure() {
        let healthy =
            health_status_from_response("http://127.0.0.1:9000/health".to_string(), StatusCode::OK);
        assert!(healthy.healthy);
        assert_eq!(healthy.status_code, Some(200));
        assert_eq!(healthy.message.as_deref(), Some("llama-server is healthy"));
        assert_eq!(healthy.reason, None);
        assert_eq!(healthy.url, "http://127.0.0.1:9000/health");

        let unhealthy = health_status_from_response(
            "http://127.0.0.1:9000/health".to_string(),
            StatusCode::SERVICE_UNAVAILABLE,
        );
        assert!(!unhealthy.healthy);
        assert_eq!(unhealthy.status_code, Some(503));
        assert_eq!(
            unhealthy.message.as_deref(),
            Some("health check failed with HTTP status 503 Service Unavailable")
        );
        assert_eq!(unhealthy.reason.as_deref(), Some("unhealthy_status"));
        assert_eq!(unhealthy.url, "http://127.0.0.1:9000/health");
    }

    #[test]
    fn health_status_from_error_marks_unreachable_server_unhealthy() {
        let status = health_status_from_error(
            "http://127.0.0.1:8080/health".to_string(),
            HealthFailureKind::Unavailable,
            "connection refused",
        );

        assert!(!status.healthy);
        assert_eq!(status.status_code, None);
        assert_eq!(
            status.message.as_deref(),
            Some("llama-server at http://127.0.0.1:8080/health is unavailable: connection refused")
        );
        assert_eq!(status.reason.as_deref(), Some("network_unreachable"));
        assert_eq!(status.url, "http://127.0.0.1:8080/health");
    }

    #[test]
    fn health_error_message_distinguishes_timeout_and_unavailable_states() {
        let url = "http://127.0.0.1:8080/health";

        assert_eq!(
            health_error_message(url, HealthFailureKind::Timeout, "deadline exceeded"),
            "timed out waiting for llama-server at http://127.0.0.1:8080/health: deadline exceeded"
        );
        assert_eq!(
            health_error_message(url, HealthFailureKind::Unavailable, "connection refused"),
            "llama-server at http://127.0.0.1:8080/health is unavailable: connection refused"
        );
        assert_eq!(
            health_error_message(url, HealthFailureKind::Other, "tls handshake failed"),
            "failed to reach llama-server at http://127.0.0.1:8080/health: tls handshake failed"
        );
    }

    #[test]
    fn classify_health_failure_marks_connection_refused_unavailable() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("should bind ephemeral port");
        let port = listener.local_addr().expect("local addr available").port();
        drop(listener);

        let url = format!("http://127.0.0.1:{port}/health");
        let client = Client::builder().build().expect("client should build");
        let error = client
            .get(&url)
            .send()
            .expect_err("request should fail against closed port");

        assert!(error.is_connect());
        assert_eq!(
            classify_health_failure(&error),
            HealthFailureKind::Unavailable
        );
    }

    #[test]
    fn classify_health_failure_marks_hanging_server_timeout() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("should bind ephemeral port");
        let port = listener.local_addr().expect("local addr available").port();

        let server = thread::spawn(move || {
            if let Ok((_socket, _addr)) = listener.accept() {
                thread::sleep(Duration::from_millis(400));
            }
        });

        let url = format!("http://127.0.0.1:{port}/health");
        let client = Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .expect("client should build");
        let error = client
            .get(&url)
            .send()
            .expect_err("request should time out against hanging server");

        assert!(error.is_timeout());
        assert_eq!(classify_health_failure(&error), HealthFailureKind::Timeout);
        server.join().expect("server thread should complete");
    }
}
