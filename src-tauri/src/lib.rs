mod commands;
mod core;
mod services;
mod state;

use commands::migration_data::{
    append_history, clear_history, delete_preset, get_history, get_presets, get_settings,
    save_preset, save_settings,
};
use services::persistence::PersistenceService;
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

            app.manage(AppState::new(service));

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
            clear_history
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Tauri app");
}
