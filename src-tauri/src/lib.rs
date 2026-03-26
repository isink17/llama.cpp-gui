use tauri::Manager;

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("LlamaCppDesk");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Tauri app");
}
