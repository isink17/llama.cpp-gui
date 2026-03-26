use crate::core::models::{ChatStreamRequest, ChatStreamStatus};
use crate::state::app_state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn start_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChatStreamRequest,
) -> Result<ChatStreamStatus, String> {
    let chat = state.chat.lock().map_err(|e| e.to_string())?;
    chat.start_stream(app, request)
}

#[tauri::command]
pub fn cancel_chat_stream(
    state: State<'_, AppState>,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    let chat = state.chat.lock().map_err(|e| e.to_string())?;
    chat.cancel_stream(&stream_id)
}

#[tauri::command]
pub fn get_chat_stream_status(
    state: State<'_, AppState>,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    let chat = state.chat.lock().map_err(|e| e.to_string())?;
    chat.get_stream_status(&stream_id)
}

#[tauri::command]
pub fn get_chat_stream_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<ChatStreamStatus>, String> {
    let chat = state.chat.lock().map_err(|e| e.to_string())?;
    chat.get_stream_statuses()
}
