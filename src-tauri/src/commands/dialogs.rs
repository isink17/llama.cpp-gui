use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub fn pick_file(
    app: tauri::AppHandle,
    title: String,
    extensions: Vec<String>,
) -> Result<Option<String>, String> {
    let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();

    let file_path = app
        .dialog()
        .file()
        .set_title(&title)
        .add_filter("Allowed files", &ext_refs)
        .blocking_pick_file();

    Ok(file_path.map(|fp| fp.to_string()))
}

#[tauri::command]
pub fn pick_folder(
    app: tauri::AppHandle,
    title: String,
) -> Result<Option<String>, String> {
    let folder_path = app
        .dialog()
        .file()
        .set_title(&title)
        .blocking_pick_folder();

    Ok(folder_path.map(|fp| fp.to_string()))
}
