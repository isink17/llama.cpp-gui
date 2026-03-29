use crate::core::models::ResolvedModelDownload;
use crate::state::app_state::SharedAppState;
use tauri::State;

#[tauri::command]
pub fn resolve_model_reference(
    state: State<'_, SharedAppState>,
    source: String,
    input: String,
    hf_token: Option<String>,
) -> Result<ResolvedModelDownload, String> {
    let resolver = state.inner().model_resolver.lock();
    resolver.resolve(&source, &input, hf_token.as_deref())
}

#[tauri::command]
pub fn list_hugging_face_files(
    state: State<'_, SharedAppState>,
    input: String,
    hf_token: Option<String>,
) -> Result<Vec<String>, String> {
    let resolver = state.inner().model_resolver.lock();
    resolver.list_hugging_face_gguf_files(&input, hf_token.as_deref())
}

#[tauri::command]
pub fn list_ollama_tags(state: State<'_, SharedAppState>, input: String) -> Result<Vec<String>, String> {
    let resolver = state.inner().model_resolver.lock();
    resolver.list_ollama_tags(&input)
}
