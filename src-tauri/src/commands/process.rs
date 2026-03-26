use crate::core::models::{LlamaProcessStatus, LlamaServerHealthStatus};
use crate::state::app_state::AppState;
use reqwest::blocking::Client;
use tauri::State;

const DEFAULT_LOG_LIMIT: usize = 200;
const MAX_LOG_LIMIT: usize = 2000;
const DEFAULT_HEALTH_URL: &str = "http://127.0.0.1:8080/health";

#[tauri::command]
pub fn start_llama_server(
    state: State<'_, AppState>,
    executable_path: String,
    args: Vec<String>,
) -> Result<LlamaProcessStatus, String> {
    let mut manager = state.process_manager.lock().map_err(|e| e.to_string())?;
    manager.start(executable_path, args)
}

#[tauri::command]
pub fn stop_llama_server(state: State<'_, AppState>) -> Result<LlamaProcessStatus, String> {
    let mut manager = state.process_manager.lock().map_err(|e| e.to_string())?;
    manager.stop()
}

#[tauri::command]
pub fn get_llama_server_status(state: State<'_, AppState>) -> Result<LlamaProcessStatus, String> {
    let mut manager = state.process_manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.status())
}

#[tauri::command]
pub fn get_llama_server_logs(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
    let manager = state.process_manager.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(DEFAULT_LOG_LIMIT).clamp(1, MAX_LOG_LIMIT);
    Ok(manager.logs(limit))
}

#[tauri::command]
pub fn clear_llama_server_logs(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.process_manager.lock().map_err(|e| e.to_string())?;
    manager.clear_logs();
    Ok(())
}

fn normalize_health_url(url: Option<String>) -> String {
    url.map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| DEFAULT_HEALTH_URL.to_string())
}

#[tauri::command]
pub fn check_llama_server_health(url: Option<String>) -> Result<LlamaServerHealthStatus, String> {
    let url = normalize_health_url(url);

    let client = Client::builder()
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    match client.get(&url).send() {
        Ok(response) => {
            let status = response.status();
            let healthy = status.is_success();
            let message = if healthy {
                Some("llama-server is healthy".to_string())
            } else {
                Some(format!("health check failed with HTTP status {status}"))
            };

            Ok(LlamaServerHealthStatus {
                healthy,
                status_code: Some(status.as_u16()),
                message,
                url,
            })
        }
        Err(err) => Ok(LlamaServerHealthStatus {
            healthy: false,
            status_code: None,
            message: Some(format!("failed to reach llama-server: {err}")),
            url,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_health_url, DEFAULT_HEALTH_URL};

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
            normalize_health_url(Some("  http://127.0.0.1:9000/health  ".to_string())),
            "http://127.0.0.1:9000/health"
        );
    }
}
