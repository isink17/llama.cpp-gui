use crate::services::chat::ChatService;
use crate::services::downloader::DownloaderService;
use crate::services::persistence::PersistenceService;
use crate::services::process_manager::ProcessManager;
use std::sync::Mutex;

pub struct AppState {
    pub persistence: Mutex<PersistenceService>,
    pub process_manager: Mutex<ProcessManager>,
    pub downloader: Mutex<DownloaderService>,
    pub chat: Mutex<ChatService>,
}

impl AppState {
    pub fn new(
        persistence: PersistenceService,
        process_manager: ProcessManager,
        downloader: DownloaderService,
        chat: ChatService,
    ) -> Self {
        Self {
            persistence: Mutex::new(persistence),
            process_manager: Mutex::new(process_manager),
            downloader: Mutex::new(downloader),
            chat: Mutex::new(chat),
        }
    }
}
