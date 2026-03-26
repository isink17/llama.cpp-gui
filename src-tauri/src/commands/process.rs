use crate::core::models::LlamaProcessStatus;
use crate::state::app_state::AppState;
use tauri::State;

const DEFAULT_LOG_LIMIT: usize = 200;
const MAX_LOG_LIMIT: usize = 2000;

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
