use crate::core::models::{DownloadState, DownloadStatus};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

#[derive(Clone)]
pub struct DownloaderService {
    inner: Arc<Mutex<DownloaderInner>>,
}

struct DownloaderInner {
    next_download_id: u64,
    downloads: HashMap<String, DownloadRecord>,
}

struct DownloadRecord {
    status: DownloadStatus,
    cancel_requested: Arc<AtomicBool>,
}

impl DownloaderService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DownloaderInner {
                next_download_id: 1,
                downloads: HashMap::new(),
            })),
        }
    }

    pub fn start_download(
        &self,
        source_url: String,
        destination_path: String,
    ) -> Result<DownloadStatus, String> {
        let source_url = source_url.trim().to_string();
        if source_url.is_empty() {
            return Err("source_url cannot be empty".to_string());
        }
        if !source_url.starts_with("http://") && !source_url.starts_with("https://") {
            return Err("source_url must start with http:// or https://".to_string());
        }

        let destination_path = destination_path.trim().to_string();
        if destination_path.is_empty() {
            return Err("destination_path cannot be empty".to_string());
        }

        let destination_path = PathBuf::from(&destination_path);
        let destination_path_str = destination_path.to_string_lossy().to_string();
        if destination_path.exists() {
            return Err(format!(
                "destination_path already exists: {}",
                destination_path.display()
            ));
        }

        if let Some(parent) = destination_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!(
                        "failed to create destination directory {}: {e}",
                        parent.display()
                    )
                })?;
            }
        }

        let (download_id, temp_path, cancel_requested, started_status) = {
            let mut inner = self.inner.lock().map_err(|e| e.to_string())?;

            if inner.downloads.values().any(|record| {
                record.status.destination_path == destination_path_str
                    && !record.status.state.is_terminal()
            }) {
                return Err(format!(
                    "an active download already targets destination_path {}",
                    destination_path.display()
                ));
            }

            let download_id = format!("download-{}", inner.next_download_id);
            inner.next_download_id = inner
                .next_download_id
                .checked_add(1)
                .ok_or_else(|| "download id counter overflowed".to_string())?;

            let temp_path = build_temp_path(&destination_path, &download_id);
            let cancel_requested = Arc::new(AtomicBool::new(false));
            let status = DownloadStatus {
                download_id: download_id.clone(),
                source_url: source_url.clone(),
                destination_path: destination_path_str.clone(),
                state: DownloadState::Downloading,
                bytes_downloaded: 0,
                total_bytes: None,
                percent_complete: None,
                error: None,
            };
            let started_status = status.clone();

            inner.downloads.insert(
                download_id.clone(),
                DownloadRecord {
                    status: status.clone(),
                    cancel_requested: Arc::clone(&cancel_requested),
                },
            );

            (download_id, temp_path, cancel_requested, started_status)
        };

        let worker_inner = Arc::clone(&self.inner);
        let worker_url = source_url.clone();
        let worker_destination = destination_path.clone();
        thread::spawn(move || {
            run_download_worker(
                worker_inner,
                download_id,
                worker_url,
                worker_destination,
                temp_path,
                cancel_requested,
            );
        });

        Ok(started_status)
    }

    pub fn cancel_download(&self, download_id: &str) -> Result<DownloadStatus, String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        let record = inner
            .downloads
            .get_mut(download_id)
            .ok_or_else(|| format!("unknown download_id: {download_id}"))?;

        if record.status.state.is_terminal() {
            return Err(format!(
                "download {download_id} is already in state {:?}",
                record.status.state
            ));
        }

        record.cancel_requested.store(true, Ordering::SeqCst);
        Ok(record.status.clone())
    }

    pub fn get_download_status(&self, download_id: &str) -> Result<DownloadStatus, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        inner
            .downloads
            .get(download_id)
            .map(|record| record.status.clone())
            .ok_or_else(|| format!("unknown download_id: {download_id}"))
    }

    pub fn get_download_statuses(&self) -> Result<Vec<DownloadStatus>, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        let mut statuses: Vec<DownloadStatus> = inner
            .downloads
            .values()
            .map(|record| record.status.clone())
            .collect();
        statuses.sort_by(|a, b| a.download_id.cmp(&b.download_id));
        Ok(statuses)
    }
}

fn run_download_worker(
    inner: Arc<Mutex<DownloaderInner>>,
    download_id: String,
    source_url: String,
    destination_path: PathBuf,
    temp_path: PathBuf,
    cancel_requested: Arc<AtomicBool>,
) {
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(err) => {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(format!("failed to build HTTP client: {err}"));
            });
            return;
        }
    };

    let mut response = match client.get(&source_url).send() {
        Ok(response) => response,
        Err(err) => {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(format!("failed to start download: {err}"));
            });
            return;
        }
    };

    if !response.status().is_success() {
        update_status(&inner, &download_id, |status| {
            status.state = DownloadState::Failed;
            status.error = Some(format!(
                "download request failed with HTTP status {}",
                response.status()
            ));
        });
        let _ = fs::remove_file(&temp_path);
        return;
    }

    let total_bytes = response.content_length();
    update_status(&inner, &download_id, |status| {
        status.total_bytes = total_bytes;
        status.percent_complete = if total_bytes.is_some() {
            Some(0.0)
        } else {
            None
        };
    });

    let mut file = match File::create(&temp_path) {
        Ok(file) => file,
        Err(err) => {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(format!(
                    "failed to create temporary file {}: {err}",
                    temp_path.display()
                ));
            });
            return;
        }
    };

    let mut downloaded_bytes: u64 = 0;
    let mut buffer = [0u8; 16 * 1024];

    loop {
        if cancel_requested.load(Ordering::SeqCst) {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Cancelled;
                status.error = Some("download cancelled by user".to_string());
            });
            let _ = fs::remove_file(&temp_path);
            return;
        }

        let bytes_read = match response.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(err) => {
                update_status(&inner, &download_id, |status| {
                    status.state = DownloadState::Failed;
                    status.error = Some(format!("failed while reading response body: {err}"));
                });
                let _ = fs::remove_file(&temp_path);
                return;
            }
        };

        if bytes_read == 0 {
            break;
        }

        if let Err(err) = file.write_all(&buffer[..bytes_read]) {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(format!(
                    "failed while writing temporary file {}: {err}",
                    temp_path.display()
                ));
            });
            let _ = fs::remove_file(&temp_path);
            return;
        }

        downloaded_bytes = downloaded_bytes.saturating_add(bytes_read as u64);
        update_status(&inner, &download_id, |status| {
            status.bytes_downloaded = downloaded_bytes;
            if let Some(total) = status.total_bytes {
                if total > 0 {
                    status.percent_complete =
                        Some(((downloaded_bytes as f64 / total as f64) * 100.0).clamp(0.0, 100.0));
                }
            }
        });
    }

    if let Err(err) = file.flush() {
        update_status(&inner, &download_id, |status| {
            status.state = DownloadState::Failed;
            status.error = Some(format!("failed to flush temporary file: {err}"));
        });
        let _ = fs::remove_file(&temp_path);
        return;
    }

    if let Some(total) = total_bytes {
        if downloaded_bytes != total {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(format!(
                    "download ended early: expected {total} bytes, received {downloaded_bytes}"
                ));
            });
            let _ = fs::remove_file(&temp_path);
            return;
        }
    }

    if cancel_requested.load(Ordering::SeqCst) {
        update_status(&inner, &download_id, |status| {
            status.state = DownloadState::Cancelled;
            status.error = Some("download cancelled by user".to_string());
        });
        let _ = fs::remove_file(&temp_path);
        return;
    }

    if let Err(err) = fs::rename(&temp_path, &destination_path) {
        update_status(&inner, &download_id, |status| {
            status.state = DownloadState::Failed;
            status.error = Some(format!(
                "failed to move temporary file to {}: {err}",
                destination_path.display()
            ));
        });
        let _ = fs::remove_file(&temp_path);
        return;
    }

    update_status(&inner, &download_id, |status| {
        status.state = DownloadState::Completed;
        status.bytes_downloaded = downloaded_bytes;
        status.total_bytes = total_bytes.or(Some(downloaded_bytes));
        status.percent_complete = Some(100.0);
        status.error = None;
    });
}

fn update_status(
    inner: &Arc<Mutex<DownloaderInner>>,
    download_id: &str,
    update: impl FnOnce(&mut DownloadStatus),
) {
    if let Ok(mut guard) = inner.lock() {
        if let Some(record) = guard.downloads.get_mut(download_id) {
            update(&mut record.status);
        }
    }
}

fn build_temp_path(destination_path: &PathBuf, download_id: &str) -> PathBuf {
    let mut temp_path = destination_path.clone();
    let temp_file_name = destination_path
        .file_name()
        .map(|name| format!("{}.{}.part", name.to_string_lossy(), download_id))
        .unwrap_or_else(|| format!("{download_id}.part"));
    temp_path.set_file_name(temp_file_name);
    temp_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static UNIQUE_SUFFIX: AtomicU64 = AtomicU64::new(1);

    fn unique_temp_dir(name: &str) -> PathBuf {
        let suffix = UNIQUE_SUFFIX.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("{name}-{suffix}"))
    }

    fn active_download_status(destination_path: String) -> DownloadStatus {
        DownloadStatus {
            download_id: "download-1".to_string(),
            source_url: "https://example.com/model.bin".to_string(),
            destination_path,
            state: DownloadState::Downloading,
            bytes_downloaded: 0,
            total_bytes: None,
            percent_complete: None,
            error: None,
        }
    }

    #[test]
    fn start_download_rejects_empty_source_url() {
        let service = DownloaderService::new();
        let destination = unique_temp_dir("downloader-empty-source").join("model.bin");

        let err = service
            .start_download("   ".to_string(), destination.to_string_lossy().to_string())
            .expect_err("expected empty source_url to fail");

        assert_eq!(err, "source_url cannot be empty");
    }

    #[test]
    fn start_download_rejects_non_http_source_url() {
        let service = DownloaderService::new();
        let destination = unique_temp_dir("downloader-bad-url").join("model.bin");

        let err = service
            .start_download(
                "ftp://example.com/model.bin".to_string(),
                destination.to_string_lossy().to_string(),
            )
            .expect_err("expected non-http source_url to fail");

        assert_eq!(err, "source_url must start with http:// or https://");
    }

    #[test]
    fn start_download_rejects_empty_destination_path() {
        let service = DownloaderService::new();

        let err = service
            .start_download(
                "https://example.com/model.bin".to_string(),
                "   ".to_string(),
            )
            .expect_err("expected empty destination_path to fail");

        assert_eq!(err, "destination_path cannot be empty");
    }

    #[test]
    fn start_download_rejects_duplicate_active_destination() {
        let service = DownloaderService::new();
        let destination = unique_temp_dir("downloader-duplicate").join("model.bin");
        let destination_str = destination.to_string_lossy().to_string();

        {
            let mut inner = service.inner.lock().expect("mutex poisoned");
            inner.downloads.insert(
                "download-99".to_string(),
                DownloadRecord {
                    status: active_download_status(destination_str.clone()),
                    cancel_requested: Arc::new(AtomicBool::new(false)),
                },
            );
        }

        let err = service
            .start_download(
                "https://example.com/model.bin".to_string(),
                destination_str.clone(),
            )
            .expect_err("expected duplicate destination to fail");

        assert_eq!(
            err,
            format!(
                "an active download already targets destination_path {}",
                destination.display()
            )
        );
    }

    #[test]
    fn get_download_status_returns_error_for_unknown_id() {
        let service = DownloaderService::new();

        let err = service
            .get_download_status("download-missing")
            .expect_err("expected unknown download_id to fail");

        assert_eq!(err, "unknown download_id: download-missing");
    }
}
