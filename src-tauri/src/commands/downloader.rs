use crate::core::models::DownloadStatus;
use crate::state::app_state::AppState;
use tauri::State;

#[tauri::command]
pub fn start_download(
    state: State<'_, AppState>,
    source_url: String,
    destination_path: String,
) -> Result<DownloadStatus, String> {
    let downloader = state.downloader.lock().map_err(|e| e.to_string())?;
    downloader.start_download(source_url, destination_path)
}

#[tauri::command]
pub fn cancel_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<DownloadStatus, String> {
    let downloader = state.downloader.lock().map_err(|e| e.to_string())?;
    downloader.cancel_download(&download_id)
}

#[tauri::command]
pub fn get_download_status(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<DownloadStatus, String> {
    let downloader = state.downloader.lock().map_err(|e| e.to_string())?;
    downloader.get_download_status(&download_id)
}

#[tauri::command]
pub fn get_download_statuses(state: State<'_, AppState>) -> Result<Vec<DownloadStatus>, String> {
    let downloader = state.downloader.lock().map_err(|e| e.to_string())?;
    downloader.get_download_statuses()
}
