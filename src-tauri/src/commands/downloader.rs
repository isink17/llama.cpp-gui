use crate::core::models::DownloadStatus;
use crate::state::app_state::AppState;
use std::sync::{Mutex, MutexGuard};
use tauri::State;

trait DownloadCommandBackend {
    fn start_download(
        &self,
        source_url: String,
        destination_path: String,
    ) -> Result<DownloadStatus, String>;

    fn cancel_download(&self, download_id: &str) -> Result<DownloadStatus, String>;

    fn get_download_status(&self, download_id: &str) -> Result<DownloadStatus, String>;

    fn get_download_statuses(&self) -> Result<Vec<DownloadStatus>, String>;
}

impl DownloadCommandBackend for crate::services::downloader::DownloaderService {
    fn start_download(
        &self,
        source_url: String,
        destination_path: String,
    ) -> Result<DownloadStatus, String> {
        Self::start_download(self, source_url, destination_path)
    }

    fn cancel_download(&self, download_id: &str) -> Result<DownloadStatus, String> {
        Self::cancel_download(self, download_id)
    }

    fn get_download_status(&self, download_id: &str) -> Result<DownloadStatus, String> {
        Self::get_download_status(self, download_id)
    }

    fn get_download_statuses(&self) -> Result<Vec<DownloadStatus>, String> {
        Self::get_download_statuses(self)
    }
}

fn lock_downloader(
    downloader: &Mutex<crate::services::downloader::DownloaderService>,
) -> Result<MutexGuard<'_, crate::services::downloader::DownloaderService>, String> {
    downloader.lock().map_err(|e| e.to_string())
}

fn start_download_with(
    downloader: &impl DownloadCommandBackend,
    source_url: String,
    destination_path: String,
) -> Result<DownloadStatus, String> {
    downloader.start_download(source_url, destination_path)
}

fn cancel_download_with(
    downloader: &impl DownloadCommandBackend,
    download_id: String,
) -> Result<DownloadStatus, String> {
    downloader.cancel_download(&download_id)
}

fn get_download_status_with(
    downloader: &impl DownloadCommandBackend,
    download_id: String,
) -> Result<DownloadStatus, String> {
    downloader.get_download_status(&download_id)
}

fn get_download_statuses_with(
    downloader: &impl DownloadCommandBackend,
) -> Result<Vec<DownloadStatus>, String> {
    downloader.get_download_statuses()
}

#[tauri::command]
pub fn start_download(
    state: State<'_, AppState>,
    source_url: String,
    destination_path: String,
) -> Result<DownloadStatus, String> {
    let downloader = lock_downloader(&state.inner().downloader)?;
    start_download_with(&*downloader, source_url, destination_path)
}

#[tauri::command]
pub fn cancel_download(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<DownloadStatus, String> {
    let downloader = lock_downloader(&state.inner().downloader)?;
    cancel_download_with(&*downloader, download_id)
}

#[tauri::command]
pub fn get_download_status(
    state: State<'_, AppState>,
    download_id: String,
) -> Result<DownloadStatus, String> {
    let downloader = lock_downloader(&state.inner().downloader)?;
    get_download_status_with(&*downloader, download_id)
}

#[tauri::command]
pub fn get_download_statuses(state: State<'_, AppState>) -> Result<Vec<DownloadStatus>, String> {
    let downloader = lock_downloader(&state.inner().downloader)?;
    get_download_statuses_with(&*downloader)
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_download_with, get_download_status_with, get_download_statuses_with,
        lock_downloader, start_download_with, DownloadCommandBackend,
    };
    use crate::core::models::{DownloadState, DownloadStatus};
    use crate::services::downloader::DownloaderService;
    use std::cell::RefCell;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeDownloader {
        start_calls: RefCell<Vec<(String, String)>>,
        cancel_calls: RefCell<Vec<String>>,
        status_calls: RefCell<Vec<String>>,
        statuses_calls: RefCell<usize>,
    }

    impl DownloadCommandBackend for FakeDownloader {
        fn start_download(
            &self,
            source_url: String,
            destination_path: String,
        ) -> Result<DownloadStatus, String> {
            self.start_calls
                .borrow_mut()
                .push((source_url.clone(), destination_path.clone()));

            Ok(DownloadStatus {
                download_id: "download-7".to_string(),
                source_url,
                destination_path,
                state: DownloadState::Downloading,
                bytes_downloaded: 0,
                total_bytes: None,
                percent_complete: None,
                error: None,
            })
        }

        fn cancel_download(&self, download_id: &str) -> Result<DownloadStatus, String> {
            self.cancel_calls.borrow_mut().push(download_id.to_string());
            Ok(status(download_id))
        }

        fn get_download_status(&self, download_id: &str) -> Result<DownloadStatus, String> {
            self.status_calls.borrow_mut().push(download_id.to_string());
            Ok(status(download_id))
        }

        fn get_download_statuses(&self) -> Result<Vec<DownloadStatus>, String> {
            *self.statuses_calls.borrow_mut() += 1;
            Ok(vec![status("download-1"), status("download-2")])
        }
    }

    struct ErrorDownloader;

    impl DownloadCommandBackend for ErrorDownloader {
        fn start_download(
            &self,
            source_url: String,
            destination_path: String,
        ) -> Result<DownloadStatus, String> {
            Err(format!(
                "start failed for {source_url} -> {destination_path}"
            ))
        }

        fn cancel_download(&self, download_id: &str) -> Result<DownloadStatus, String> {
            Err(format!("cancel failed for {download_id}"))
        }

        fn get_download_status(&self, download_id: &str) -> Result<DownloadStatus, String> {
            Err(format!("status failed for {download_id}"))
        }

        fn get_download_statuses(&self) -> Result<Vec<DownloadStatus>, String> {
            Err("status list failed".to_string())
        }
    }

    fn status(download_id: &str) -> DownloadStatus {
        DownloadStatus {
            download_id: download_id.to_string(),
            source_url: "https://example.com/model.bin".to_string(),
            destination_path: "C:/models/model.bin".to_string(),
            state: DownloadState::Downloading,
            bytes_downloaded: 0,
            total_bytes: None,
            percent_complete: None,
            error: None,
        }
    }

    #[test]
    fn downloader_helpers_forward_arguments_and_ids() {
        let downloader = FakeDownloader::default();

        let start_status = start_download_with(
            &downloader,
            "https://example.com/model.bin".to_string(),
            "C:/models/model.bin".to_string(),
        )
        .unwrap();
        let cancel_status = cancel_download_with(&downloader, "download-7".to_string()).unwrap();
        let status = get_download_status_with(&downloader, "download-7".to_string()).unwrap();
        let statuses = get_download_statuses_with(&downloader).unwrap();

        assert_eq!(
            downloader.start_calls.borrow().as_slice(),
            &[(
                "https://example.com/model.bin".to_string(),
                "C:/models/model.bin".to_string(),
            )]
        );
        assert_eq!(
            downloader.cancel_calls.borrow().as_slice(),
            &["download-7".to_string()]
        );
        assert_eq!(
            downloader.status_calls.borrow().as_slice(),
            &["download-7".to_string()]
        );
        assert_eq!(*downloader.statuses_calls.borrow(), 1);
        assert_eq!(start_status.download_id, "download-7");
        assert_eq!(cancel_status.download_id, "download-7");
        assert_eq!(status.download_id, "download-7");
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn downloader_helpers_propagate_backend_errors_verbatim() {
        let downloader = ErrorDownloader;

        let start_err = start_download_with(
            &downloader,
            "https://example.com/model.bin".to_string(),
            "C:/models/model.bin".to_string(),
        )
        .expect_err("expected start helper to forward backend error");
        let cancel_err = cancel_download_with(&downloader, "download-7".to_string())
            .expect_err("expected cancel helper to forward backend error");
        let status_err = get_download_status_with(&downloader, "download-7".to_string())
            .expect_err("expected status helper to forward backend error");
        let statuses_err = get_download_statuses_with(&downloader)
            .expect_err("expected statuses helper to forward backend error");

        assert_eq!(
            start_err,
            "start failed for https://example.com/model.bin -> C:/models/model.bin"
        );
        assert_eq!(cancel_err, "cancel failed for download-7");
        assert_eq!(status_err, "status failed for download-7");
        assert_eq!(statuses_err, "status list failed");
    }

    #[test]
    fn downloader_lock_errors_are_stringified() {
        let downloader = Arc::new(Mutex::new(DownloaderService::new()));
        let poisoned_downloader = Arc::clone(&downloader);
        let _ = std::panic::catch_unwind(move || {
            let _guard = poisoned_downloader.lock().unwrap();
            panic!("poison the downloader mutex");
        });

        let err = match lock_downloader(downloader.as_ref()) {
            Ok(_) => panic!("expected poisoned downloader mutex lock to fail"),
            Err(err) => err,
        };
        assert!(err.contains("poisoned"));
    }
}
