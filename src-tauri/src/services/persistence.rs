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
        let parsed = serde_json::from_str::<T>(&content).unwrap_or_default();
        Ok(parsed)
    }

    fn write_json<T>(&self, file_name: &str, value: &T) -> io::Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.ensure_data_dir()?;
        let path = self.data_dir.join(file_name);
        let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        fs::write(path, json)
    }
}
