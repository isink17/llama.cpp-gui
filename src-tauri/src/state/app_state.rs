use crate::services::persistence::PersistenceService;
use std::sync::Mutex;

pub struct AppState {
    pub persistence: Mutex<PersistenceService>,
}

impl AppState {
    pub fn new(persistence: PersistenceService) -> Self {
        Self {
            persistence: Mutex::new(persistence),
        }
    }
}
