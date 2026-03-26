use crate::core::models::{HistoryEntry, Preset, Settings};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::PathBuf;

pub struct PersistenceService {
    data_dir: PathBuf,
}

impl PersistenceService {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn ensure_data_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.data_dir)
    }

    pub fn load_settings(&self) -> io::Result<Settings> {
        self.read_json_or_default("settings.json")
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        self.write_json("settings.json", settings)
    }

    pub fn load_presets(&self) -> io::Result<Vec<Preset>> {
        self.read_json_or_default("presets.json")
    }

    pub fn save_presets(&self, presets: &[Preset]) -> io::Result<()> {
        self.write_json("presets.json", presets)
    }

    pub fn load_history(&self) -> io::Result<Vec<HistoryEntry>> {
        self.read_json_or_default("history.json")
    }

    pub fn save_history(&self, history: &[HistoryEntry]) -> io::Result<()> {
        self.write_json("history.json", history)
    }

    fn read_json_or_default<T>(&self, file_name: &str) -> io::Result<T>
    where
        T: DeserializeOwned + Default,
    {
        let path = self.data_dir.join(file_name);
        if !path.exists() {
            return Ok(T::default());
        }

        let content = fs::read_to_string(path)?;
        let parsed = serde_json::from_str::<T>(&content).map_err(io::Error::other)?;
        Ok(parsed)
    }

    fn write_json<T>(&self, file_name: &str, value: &T) -> io::Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.ensure_data_dir()?;
        let path = self.data_dir.join(file_name);
        let tmp_path = self.data_dir.join(format!("{file_name}.tmp"));
        let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        fs::write(&tmp_path, json)?;
        fs::rename(tmp_path, path)
    }
}

#[cfg(test)]
mod tests {
    use super::PersistenceService;
    use crate::core::models::{HistoryEntry, Preset, Settings};
    use std::fs;

    fn temp_data_dir(test_name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be monotonic after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("llamacppdesk-{test_name}-{nanos}"))
    }

    #[test]
    fn settings_round_trip_works() {
        let data_dir = temp_data_dir("settings");
        let service = PersistenceService::new(data_dir.clone());

        let settings = Settings {
            server_url: "http://localhost:8081".to_string(),
            max_tokens: 1024,
            temperature: 0.5,
        };

        service
            .save_settings(&settings)
            .expect("save should succeed");
        let loaded = service.load_settings().expect("load should succeed");

        assert_eq!(loaded.server_url, settings.server_url);
        assert_eq!(loaded.max_tokens, settings.max_tokens);
        assert!((loaded.temperature - settings.temperature).abs() < f32::EPSILON);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn presets_and_history_round_trip_works() {
        let data_dir = temp_data_dir("collections");
        let service = PersistenceService::new(data_dir.clone());

        let presets = vec![Preset {
            id: "p1".to_string(),
            name: "Default".to_string(),
            system_prompt: "You are helpful.".to_string(),
            created_at: "2026-03-26T12:00:00Z".to_string(),
        }];
        let history = vec![HistoryEntry {
            id: "h1".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: "2026-03-26T12:01:00Z".to_string(),
        }];

        service
            .save_presets(&presets)
            .expect("save presets should succeed");
        service
            .save_history(&history)
            .expect("save history should succeed");

        let loaded_presets = service.load_presets().expect("load presets should succeed");
        let loaded_history = service.load_history().expect("load history should succeed");

        assert_eq!(loaded_presets.len(), 1);
        assert_eq!(loaded_presets[0].id, "p1");
        assert_eq!(loaded_history.len(), 1);
        assert_eq!(loaded_history[0].id, "h1");

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn malformed_json_returns_error() {
        let data_dir = temp_data_dir("invalid-json");
        fs::create_dir_all(&data_dir).expect("dir create should succeed");
        fs::write(data_dir.join("settings.json"), "{invalid").expect("write should succeed");

        let service = PersistenceService::new(data_dir.clone());
        let result = service.load_settings();
        assert!(result.is_err());

        let _ = fs::remove_dir_all(data_dir);
    }
}
