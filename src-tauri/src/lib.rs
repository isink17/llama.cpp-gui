mod commands;
mod core;
mod services;
mod state;

use commands::downloader::{
    cancel_download, get_download_status, get_download_statuses, start_download,
};
use commands::migration_data::{
    append_history, clear_history, delete_preset, get_history, get_presets, get_settings,
    save_preset, save_settings,
};
use commands::process::{
    clear_llama_server_logs, get_llama_server_logs, get_llama_server_status, start_llama_server,
    stop_llama_server,
};
use services::downloader::DownloaderService;
use services::persistence::PersistenceService;
use services::process_manager::ProcessManager;
use state::app_state::AppState;
use tauri::Manager;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("migration-data");

            let service = PersistenceService::new(data_dir);
            service.ensure_data_dir().map_err(|e| e.to_string())?;
            let process_manager = ProcessManager::new(2_000);
            let downloader = DownloaderService::new();

            app.manage(AppState::new(service, process_manager, downloader));

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("LlamaCppDesk");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_settings,
            save_settings,
            get_presets,
            save_preset,
            delete_preset,
            get_history,
            append_history,
            clear_history,
            start_llama_server,
            stop_llama_server,
            get_llama_server_status,
            get_llama_server_logs,
            clear_llama_server_logs,
            start_download,
            cancel_download,
            get_download_status,
            get_download_statuses
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Tauri app");
}
