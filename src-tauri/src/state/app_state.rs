use crate::services::persistence::PersistenceService;
use crate::services::process_manager::ProcessManager;
use std::sync::Mutex;

pub struct AppState {
    pub persistence: Mutex<PersistenceService>,
    pub process_manager: Mutex<ProcessManager>,
}

impl AppState {
    pub fn new(persistence: PersistenceService, process_manager: ProcessManager) -> Self {
        Self {
            persistence: Mutex::new(persistence),
            process_manager: Mutex::new(process_manager),
        }
    }
}
