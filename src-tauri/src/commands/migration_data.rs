use crate::core::models::{HistoryEntry, Preset, Settings};
use crate::state::app_state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    persistence.load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    persistence
        .save_settings(&settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_presets(state: State<'_, AppState>) -> Result<Vec<Preset>, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    persistence.load_presets().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_preset(state: State<'_, AppState>, preset: Preset) -> Result<Vec<Preset>, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    let mut presets = persistence.load_presets().map_err(|e| e.to_string())?;

    if let Some(existing) = presets.iter_mut().find(|item| item.id == preset.id) {
        *existing = preset;
    } else {
        presets.push(preset);
    }

    persistence
        .save_presets(&presets)
        .map_err(|e| e.to_string())?;
    Ok(presets)
}

#[tauri::command]
pub fn delete_preset(state: State<'_, AppState>, preset_id: String) -> Result<Vec<Preset>, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    let mut presets = persistence.load_presets().map_err(|e| e.to_string())?;

    presets.retain(|item| item.id != preset_id);
    persistence
        .save_presets(&presets)
        .map_err(|e| e.to_string())?;

    Ok(presets)
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    persistence.load_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn append_history(
    state: State<'_, AppState>,
    entry: HistoryEntry,
) -> Result<Vec<HistoryEntry>, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    let mut history = persistence.load_history().map_err(|e| e.to_string())?;
    history.push(entry);
    persistence
        .save_history(&history)
        .map_err(|e| e.to_string())?;
    Ok(history)
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    persistence
        .save_history(&Vec::<HistoryEntry>::new())
        .map_err(|e| e.to_string())
}
