mod commands;
mod core;
mod services;
mod state;

use commands::chat::{
    cancel_chat_stream, get_chat_stream_status, get_chat_stream_statuses, start_chat_stream,
};
use commands::dialogs::{pick_file, pick_folder};
use commands::downloader::{
    cancel_download, get_download_status, get_download_statuses, start_download,
};
use commands::migration_data::{
    append_history, clear_history, delete_preset, get_history, get_presets, get_settings,
    save_preset, save_settings,
};
use commands::model_resolver::{
    list_hugging_face_files, list_ollama_tags, resolve_model_reference,
};
use commands::process::{
    check_llama_server_health, clear_llama_server_logs, get_llama_server_logs,
    get_llama_server_status, start_llama_server, stop_llama_server, wait_for_server_ready,
};
use services::chat::ChatService;
use services::downloader::DownloaderService;
use services::model_resolver::ModelResolverService;
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
            /// Maximum number of log lines retained in memory for the llama-server process.
            const PROCESS_MAX_LOG_LINES: usize = 2_000;

            let process_manager = ProcessManager::new(PROCESS_MAX_LOG_LINES);
            let downloader = DownloaderService::new();
            let chat = ChatService::new();
            let model_resolver = ModelResolverService::new().map_err(|e| e.to_string())?;

            app.manage(AppState::new(
                service,
                process_manager,
                downloader,
                chat,
                model_resolver,
            ));

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("LlamaCppDesk");
            }

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
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
            check_llama_server_health,
            wait_for_server_ready,
            start_download,
            cancel_download,
            get_download_status,
            get_download_statuses,
            start_chat_stream,
            cancel_chat_stream,
            get_chat_stream_status,
            get_chat_stream_statuses,
            pick_file,
            pick_folder,
            resolve_model_reference,
            list_hugging_face_files,
            list_ollama_tags
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Tauri app");
}
