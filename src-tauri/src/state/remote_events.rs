use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteEventKind {
    Settings,
    Presets,
    History,
    Process,
    Logs,
    Downloads,
    Chat,
}

impl RemoteEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Settings => "settings",
            Self::Presets => "presets",
            Self::History => "history",
            Self::Process => "process",
            Self::Logs => "logs",
            Self::Downloads => "downloads",
            Self::Chat => "chat",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEvent {
    pub kind: RemoteEventKind,
    pub payload: Value,
}

#[derive(Clone)]
pub struct RemoteEventHub {
    sender: broadcast::Sender<RemoteEvent>,
}

impl RemoteEventHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, kind: RemoteEventKind, payload: Value) {
        let _ = self.sender.send(RemoteEvent { kind, payload });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RemoteEvent> {
        self.sender.subscribe()
    }
}
