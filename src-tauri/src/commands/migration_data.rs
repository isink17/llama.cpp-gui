use crate::core::models::{HistoryEntry, Preset, Settings};
use crate::state::app_state::AppState;
use tauri::State;

const MAX_TEMPERATURE: f32 = 2.0;
const MAX_TOKENS_LIMIT: u32 = 8192;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let persistence = state.persistence.lock().map_err(|e| e.to_string())?;
    persistence.load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    validate_settings(&settings)?;
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
    validate_preset(&preset)?;
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
    let preset_id = preset_id.trim().to_string();
    if preset_id.is_empty() {
        return Err("preset_id cannot be empty".to_string());
    }

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
    validate_history_entry(&entry)?;
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

fn validate_settings(settings: &Settings) -> Result<(), String> {
    if settings.server_url.trim().is_empty() {
        return Err("server_url cannot be empty".to_string());
    }

    if settings.max_tokens == 0 || settings.max_tokens > MAX_TOKENS_LIMIT {
        return Err(format!(
            "max_tokens must be in range 1..={MAX_TOKENS_LIMIT}"
        ));
    }

    if !(0.0..=MAX_TEMPERATURE).contains(&settings.temperature) {
        return Err(format!(
            "temperature must be in range 0.0..={MAX_TEMPERATURE}"
        ));
    }

    Ok(())
}

fn validate_preset(preset: &Preset) -> Result<(), String> {
    if preset.id.trim().is_empty() {
        return Err("preset.id cannot be empty".to_string());
    }

    if preset.name.trim().is_empty() {
        return Err("preset.name cannot be empty".to_string());
    }

    if preset.created_at.trim().is_empty() {
        return Err("preset.created_at cannot be empty".to_string());
    }

    Ok(())
}

fn validate_history_entry(entry: &HistoryEntry) -> Result<(), String> {
    if entry.id.trim().is_empty() {
        return Err("history.id cannot be empty".to_string());
    }

    if entry.content.trim().is_empty() {
        return Err("history.content cannot be empty".to_string());
    }

    if entry.timestamp.trim().is_empty() {
        return Err("history.timestamp cannot be empty".to_string());
    }

    match entry.role.trim() {
        "system" | "user" | "assistant" => Ok(()),
        _ => Err("history.role must be one of: system, user, assistant".to_string()),
    }
}
