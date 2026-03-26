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
    let preset_id = normalize_preset_id(preset_id)?;

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

fn normalize_preset_id(preset_id: String) -> Result<String, String> {
    let preset_id = preset_id.trim().to_string();
    if preset_id.is_empty() {
        return Err("preset_id cannot be empty".to_string());
    }

    Ok(preset_id)
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

#[cfg(test)]
mod tests {
    use super::{
        normalize_preset_id, validate_history_entry, validate_preset, validate_settings,
        MAX_TEMPERATURE, MAX_TOKENS_LIMIT,
    };
    use crate::core::models::{HistoryEntry, Preset, Settings};

    #[test]
    fn accepts_valid_settings() {
        let settings = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 1024,
            temperature: 0.8,
        };
        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn rejects_invalid_settings() {
        let empty_url = Settings {
            server_url: "".to_string(),
            max_tokens: 1024,
            temperature: 0.8,
        };
        assert!(validate_settings(&empty_url).is_err());

        let bad_tokens = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: MAX_TOKENS_LIMIT + 1,
            temperature: 0.8,
        };
        assert!(validate_settings(&bad_tokens).is_err());

        let bad_temp = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 512,
            temperature: MAX_TEMPERATURE + 0.1,
        };
        assert!(validate_settings(&bad_temp).is_err());
    }

    #[test]
    fn accepts_settings_boundary_values() {
        let min_values = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 1,
            temperature: 0.0,
        };
        assert!(validate_settings(&min_values).is_ok());

        let max_values = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: MAX_TOKENS_LIMIT,
            temperature: MAX_TEMPERATURE,
        };
        assert!(validate_settings(&max_values).is_ok());
    }

    #[test]
    fn rejects_settings_outside_boundary_values() {
        let zero_tokens = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 0,
            temperature: 0.8,
        };
        assert!(validate_settings(&zero_tokens).is_err());

        let below_zero_temp = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 512,
            temperature: -0.1,
        };
        assert!(validate_settings(&below_zero_temp).is_err());

        let above_max_temp = Settings {
            server_url: "http://127.0.0.1:8080".to_string(),
            max_tokens: 512,
            temperature: MAX_TEMPERATURE + 0.01,
        };
        assert!(validate_settings(&above_max_temp).is_err());
    }

    #[test]
    fn rejects_invalid_preset() {
        let preset = Preset {
            id: "".to_string(),
            name: "Preset".to_string(),
            system_prompt: "You are helpful.".to_string(),
            created_at: "2026-03-26T12:00:00Z".to_string(),
        };
        assert!(validate_preset(&preset).is_err());
    }

    #[test]
    fn trims_delete_preset_id_and_rejects_blank_values() {
        assert_eq!(
            normalize_preset_id("  preset-1  ".to_string()).unwrap(),
            "preset-1"
        );
        assert!(normalize_preset_id("   ".to_string()).is_err());
    }

    #[test]
    fn accepts_trimmed_history_role_and_rejects_blank_content() {
        let trimmed_role = HistoryEntry {
            id: "h1".to_string(),
            role: "  user  ".to_string(),
            content: "  Hello there  ".to_string(),
            timestamp: "2026-03-26T12:00:00Z".to_string(),
        };
        assert!(validate_history_entry(&trimmed_role).is_ok());

        let blank_content = HistoryEntry {
            content: "   ".to_string(),
            ..trimmed_role.clone()
        };
        assert!(validate_history_entry(&blank_content).is_err());
    }

    #[test]
    fn rejects_blank_history_role_and_content() {
        let blank_role = HistoryEntry {
            id: "h2".to_string(),
            role: "   ".to_string(),
            content: "Hello".to_string(),
            timestamp: "2026-03-26T12:00:00Z".to_string(),
        };
        assert!(validate_history_entry(&blank_role).is_err());

        let blank_content = HistoryEntry {
            content: "\t".to_string(),
            ..blank_role
        };
        assert!(validate_history_entry(&blank_content).is_err());
    }

    #[test]
    fn accepts_and_rejects_history_roles() {
        let valid = HistoryEntry {
            id: "h1".to_string(),
            role: "assistant".to_string(),
            content: "Hi".to_string(),
            timestamp: "2026-03-26T12:00:00Z".to_string(),
        };
        assert!(validate_history_entry(&valid).is_ok());

        let invalid = HistoryEntry {
            role: "bot".to_string(),
            ..valid
        };
        assert!(validate_history_entry(&invalid).is_err());
    }
}
