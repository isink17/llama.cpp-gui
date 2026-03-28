use crate::services::chat::ChatService;
use crate::services::downloader::DownloaderService;
use crate::services::model_resolver::ModelResolverService;
use crate::services::persistence::PersistenceService;
use crate::services::process_manager::ProcessManager;
use parking_lot::Mutex;

pub struct AppState {
    pub persistence: Mutex<PersistenceService>,
    pub process_manager: Mutex<ProcessManager>,
    pub downloader: Mutex<DownloaderService>,
    pub chat: Mutex<ChatService>,
    pub model_resolver: Mutex<ModelResolverService>,
}

impl AppState {
    pub fn new(
        persistence: PersistenceService,
        process_manager: ProcessManager,
        downloader: DownloaderService,
        chat: ChatService,
        model_resolver: ModelResolverService,
    ) -> Self {
        Self {
            persistence: Mutex::new(persistence),
            process_manager: Mutex::new(process_manager),
            downloader: Mutex::new(downloader),
            chat: Mutex::new(chat),
            model_resolver: Mutex::new(model_resolver),
        }
    }
}
