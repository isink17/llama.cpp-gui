use crate::core::models::{ChatStreamRequest, ChatStreamStatus};
use crate::state::app_state::AppState;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, State};

trait ChatCommandBackend {
    type App;

    fn start_stream(
        &self,
        app: Self::App,
        request: ChatStreamRequest,
    ) -> Result<ChatStreamStatus, String>;

    fn cancel_stream(&self, stream_id: &str) -> Result<ChatStreamStatus, String>;

    fn get_stream_status(&self, stream_id: &str) -> Result<ChatStreamStatus, String>;

    fn get_stream_statuses(&self) -> Result<Vec<ChatStreamStatus>, String>;
}

impl ChatCommandBackend for crate::services::chat::ChatService {
    type App = AppHandle;

    fn start_stream(
        &self,
        app: AppHandle,
        request: ChatStreamRequest,
    ) -> Result<ChatStreamStatus, String> {
        Self::start_stream(self, app, request)
    }

    fn cancel_stream(&self, stream_id: &str) -> Result<ChatStreamStatus, String> {
        Self::cancel_stream(self, stream_id)
    }

    fn get_stream_status(&self, stream_id: &str) -> Result<ChatStreamStatus, String> {
        Self::get_stream_status(self, stream_id)
    }

    fn get_stream_statuses(&self) -> Result<Vec<ChatStreamStatus>, String> {
        Self::get_stream_statuses(self)
    }
}

fn lock_chat(
    chat: &Mutex<crate::services::chat::ChatService>,
) -> Result<MutexGuard<'_, crate::services::chat::ChatService>, String> {
    chat.lock().map_err(|e| e.to_string())
}

fn start_chat_stream_with<B>(
    chat: &B,
    app: B::App,
    request: ChatStreamRequest,
) -> Result<ChatStreamStatus, String>
where
    B: ChatCommandBackend,
{
    chat.start_stream(app, request)
}

fn cancel_chat_stream_with(
    chat: &impl ChatCommandBackend,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    chat.cancel_stream(&stream_id)
}

fn get_chat_stream_status_with(
    chat: &impl ChatCommandBackend,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    chat.get_stream_status(&stream_id)
}

fn get_chat_stream_statuses_with(
    chat: &impl ChatCommandBackend,
) -> Result<Vec<ChatStreamStatus>, String> {
    chat.get_stream_statuses()
}

#[tauri::command]
pub fn start_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChatStreamRequest,
) -> Result<ChatStreamStatus, String> {
    let chat = lock_chat(&state.inner().chat)?;
    start_chat_stream_with(&*chat, app, request)
}

#[tauri::command]
pub fn cancel_chat_stream(
    state: State<'_, AppState>,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    let chat = lock_chat(&state.inner().chat)?;
    cancel_chat_stream_with(&*chat, stream_id)
}

#[tauri::command]
pub fn get_chat_stream_status(
    state: State<'_, AppState>,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    let chat = lock_chat(&state.inner().chat)?;
    get_chat_stream_status_with(&*chat, stream_id)
}

#[tauri::command]
pub fn get_chat_stream_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<ChatStreamStatus>, String> {
    let chat = lock_chat(&state.inner().chat)?;
    get_chat_stream_statuses_with(&*chat)
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_chat_stream_with, get_chat_stream_status_with, get_chat_stream_statuses_with,
        lock_chat, start_chat_stream_with, ChatCommandBackend,
    };
    use crate::core::models::{ChatMessage, ChatStreamRequest, ChatStreamState, ChatStreamStatus};
    use crate::services::chat::ChatService;
    use std::cell::RefCell;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeChatService {
        start_calls: RefCell<Vec<ChatStreamRequest>>,
        cancel_calls: RefCell<Vec<String>>,
        status_calls: RefCell<Vec<String>>,
        statuses_calls: RefCell<usize>,
    }

    impl ChatCommandBackend for FakeChatService {
        type App = ();

        fn start_stream(
            &self,
            _app: (),
            request: ChatStreamRequest,
        ) -> Result<ChatStreamStatus, String> {
            self.start_calls.borrow_mut().push(request.clone());
            Ok(ChatStreamStatus {
                stream_id: "stream-9".to_string(),
                state: ChatStreamState::Streaming,
                model: request.model,
                bytes_received: 0,
                error: None,
            })
        }

        fn cancel_stream(&self, stream_id: &str) -> Result<ChatStreamStatus, String> {
            self.cancel_calls.borrow_mut().push(stream_id.to_string());
            Ok(status(stream_id))
        }

        fn get_stream_status(&self, stream_id: &str) -> Result<ChatStreamStatus, String> {
            self.status_calls.borrow_mut().push(stream_id.to_string());
            Ok(status(stream_id))
        }

        fn get_stream_statuses(&self) -> Result<Vec<ChatStreamStatus>, String> {
            *self.statuses_calls.borrow_mut() += 1;
            Ok(vec![status("stream-1"), status("stream-2")])
        }
    }

    fn status(stream_id: &str) -> ChatStreamStatus {
        ChatStreamStatus {
            stream_id: stream_id.to_string(),
            state: ChatStreamState::Streaming,
            model: "llama-3.1".to_string(),
            bytes_received: 0,
            error: None,
        }
    }

    fn request() -> ChatStreamRequest {
        ChatStreamRequest {
            server_url: Some("http://127.0.0.1:8080".to_string()),
            model: "llama-3.1".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: Some(128),
            temperature: Some(0.4),
        }
    }

    #[test]
    fn chat_helpers_forward_requests_and_ids() {
        let chat = FakeChatService::default();

        let start_status = start_chat_stream_with(&chat, (), request()).unwrap();
        let cancel_status = cancel_chat_stream_with(&chat, "stream-9".to_string()).unwrap();
        let status = get_chat_stream_status_with(&chat, "stream-9".to_string()).unwrap();
        let statuses = get_chat_stream_statuses_with(&chat).unwrap();

        assert_eq!(chat.start_calls.borrow().len(), 1);
        assert_eq!(chat.start_calls.borrow()[0].model, "llama-3.1");
        assert_eq!(chat.start_calls.borrow()[0].messages[0].content, "Hello");
        assert_eq!(
            chat.cancel_calls.borrow().as_slice(),
            &["stream-9".to_string()]
        );
        assert_eq!(
            chat.status_calls.borrow().as_slice(),
            &["stream-9".to_string()]
        );
        assert_eq!(*chat.statuses_calls.borrow(), 1);
        assert_eq!(start_status.stream_id, "stream-9");
        assert_eq!(cancel_status.stream_id, "stream-9");
        assert_eq!(status.stream_id, "stream-9");
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn chat_lock_errors_are_stringified() {
        let chat = Arc::new(Mutex::new(ChatService::new()));
        let poisoned_chat = Arc::clone(&chat);
        let _ = std::panic::catch_unwind(move || {
            let _guard = poisoned_chat.lock().unwrap();
            panic!("poison the chat mutex");
        });

        let err = lock_chat(chat.as_ref()).unwrap_err();
        assert!(err.contains("poisoned"));
    }
}
