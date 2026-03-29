use crate::core::models::{ChatStreamRequest, ChatStreamStatus};
use crate::state::app_state::SharedAppState;
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
    state: State<'_, SharedAppState>,
    request: ChatStreamRequest,
) -> Result<ChatStreamStatus, String> {
    let chat = state.inner().chat.lock();
    start_chat_stream_with(&*chat, app, request)
}

#[tauri::command]
pub fn cancel_chat_stream(
    state: State<'_, SharedAppState>,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    let chat = state.inner().chat.lock();
    cancel_chat_stream_with(&*chat, stream_id)
}

#[tauri::command]
pub fn get_chat_stream_status(
    state: State<'_, SharedAppState>,
    stream_id: String,
) -> Result<ChatStreamStatus, String> {
    let chat = state.inner().chat.lock();
    get_chat_stream_status_with(&*chat, stream_id)
}

#[tauri::command]
pub fn get_chat_stream_statuses(
    state: State<'_, SharedAppState>,
) -> Result<Vec<ChatStreamStatus>, String> {
    let chat = state.inner().chat.lock();
    get_chat_stream_statuses_with(&*chat)
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_chat_stream_with, get_chat_stream_status_with, get_chat_stream_statuses_with,
        start_chat_stream_with, ChatCommandBackend,
    };
    use crate::core::models::{ChatMessage, ChatStreamRequest, ChatStreamState, ChatStreamStatus};
    use std::cell::RefCell;

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
}
