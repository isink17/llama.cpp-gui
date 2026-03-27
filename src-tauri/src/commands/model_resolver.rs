use crate::core::models::ResolvedModelDownload;
use crate::state::app_state::AppState;
use std::sync::{Mutex, MutexGuard};
use tauri::State;

fn lock_model_resolver(
    resolver: &Mutex<crate::services::model_resolver::ModelResolverService>,
) -> Result<MutexGuard<'_, crate::services::model_resolver::ModelResolverService>, String> {
    resolver.lock().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_model_reference(
    state: State<'_, AppState>,
    source: String,
    input: String,
    hf_token: Option<String>,
) -> Result<ResolvedModelDownload, String> {
    let resolver = lock_model_resolver(&state.inner().model_resolver)?;
    resolver.resolve(&source, &input, hf_token.as_deref())
}

#[tauri::command]
pub fn list_hugging_face_files(
    state: State<'_, AppState>,
    input: String,
    hf_token: Option<String>,
) -> Result<Vec<String>, String> {
    let resolver = lock_model_resolver(&state.inner().model_resolver)?;
    resolver.list_hugging_face_gguf_files(&input, hf_token.as_deref())
}

#[tauri::command]
pub fn list_ollama_tags(
    state: State<'_, AppState>,
    input: String,
) -> Result<Vec<String>, String> {
    let resolver = lock_model_resolver(&state.inner().model_resolver)?;
    resolver.list_ollama_tags(&input)
}

#[cfg(test)]
mod tests {
    use super::lock_model_resolver;
    use crate::services::model_resolver::ModelResolverService;
    use std::sync::{Arc, Mutex};

    #[test]
    fn model_resolver_lock_errors_are_stringified() {
        let resolver = Arc::new(Mutex::new(ModelResolverService::new()));
        let poisoned_resolver = Arc::clone(&resolver);
        let _ = std::panic::catch_unwind(move || {
            let _guard = poisoned_resolver.lock().unwrap();
            panic!("poison the model_resolver mutex");
        });

        let err = match lock_model_resolver(resolver.as_ref()) {
            Ok(_) => panic!("expected poisoned model_resolver mutex lock to fail"),
            Err(err) => err,
        };
        assert!(err.contains("poisoned"));
    }
}
