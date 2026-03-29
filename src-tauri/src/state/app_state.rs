use crate::services::chat::ChatService;
use crate::services::downloader::DownloaderService;
use crate::services::model_resolver::ModelResolverService;
use crate::services::persistence::PersistenceService;
use crate::services::process_manager::ProcessManager;
use crate::state::remote_events::{RemoteEventHub, RemoteEventKind};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;

pub type SharedAppState = Arc<AppState>;

pub struct AppState {
    pub persistence: Mutex<PersistenceService>,
    pub process_manager: Mutex<ProcessManager>,
    pub downloader: Mutex<DownloaderService>,
    pub chat: Mutex<ChatService>,
    pub model_resolver: Mutex<ModelResolverService>,
    pub remote_events: RemoteEventHub,
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
            remote_events: RemoteEventHub::new(64),
        }
    }

    pub fn publish_remote_event<T>(&self, kind: RemoteEventKind, payload: T)
    where
        T: Serialize,
    {
        if let Ok(payload) = serde_json::to_value(payload) {
            self.remote_events.publish(kind, payload);
        }
    }

}
