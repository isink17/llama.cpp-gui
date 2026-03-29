use crate::core::models::{DownloadState, DownloadStatus};
use reqwest::blocking::Client;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
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

const MAX_CONCURRENT_DOWNLOADS: usize = 5;

struct DownloaderInner {
    next_download_id: u64,
    downloads: BTreeMap<String, DownloadRecord>,
}

struct DownloadRecord {
    status: DownloadStatus,
    cancel_requested: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadFailureKind {
    Timeout,
    Unavailable,
    Other,
}

impl DownloaderService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DownloaderInner {
                next_download_id: 1,
                downloads: BTreeMap::new(),
            })),
        }
    }

    pub fn start_download(
        &self,
        source_url: String,
        destination_path: String,
        request_headers: Option<HashMap<String, String>>,
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

        for component in destination_path.components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err(
                    "destination_path must not contain '..' path traversal components".to_string(),
                );
            }
        }

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

            let active_count = inner
                .downloads
                .values()
                .filter(|record| !record.status.state.is_terminal())
                .count();
            if active_count >= MAX_CONCURRENT_DOWNLOADS {
                return Err(format!(
                    "maximum concurrent downloads reached ({})",
                    MAX_CONCURRENT_DOWNLOADS
                ));
            }

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
                    status,
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
                request_headers,
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
        let statuses: Vec<DownloadStatus> = inner
            .downloads
            .values()
            .map(|record| record.status.clone())
            .collect();
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
    request_headers: Option<HashMap<String, String>>,
) {
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(err) => {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(download_failure_message(
                    &source_url,
                    DownloadFailureKind::Other,
                    format!("failed to build HTTP client: {err}"),
                ));
            });
            return;
        }
    };

    let mut request = client.get(&source_url);
    if let Some(headers) = request_headers {
        for (name, value) in headers {
            request = request.header(name, value);
        }
    }

    let mut response = match request.send() {
        Ok(response) => response,
        Err(err) => {
            update_status(&inner, &download_id, |status| {
                status.state = DownloadState::Failed;
                status.error = Some(download_failure_message(
                    &source_url,
                    classify_reqwest_failure(&err),
                    err,
                ));
            });
            return;
        }
    };

    if !response.status().is_success() {
        update_status(&inner, &download_id, |status| {
            status.state = DownloadState::Failed;
            status.error = Some(download_failure_message(
                &source_url,
                DownloadFailureKind::Other,
                format!(
                    "download request failed with HTTP status {}",
                    response.status()
                ),
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
    let mut buffer = [0u8; 128 * 1024];

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
                    status.error = Some(download_failure_message(
                        &source_url,
                        classify_read_failure(&err),
                        err,
                    ));
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

fn download_failure_message(
    source_url: &str,
    kind: DownloadFailureKind,
    error: impl Display,
) -> String {
    match kind {
        DownloadFailureKind::Timeout => {
            format!("download from {source_url} timed out: {error}")
        }
        DownloadFailureKind::Unavailable => {
            format!("download from {source_url} is unavailable: {error}")
        }
        DownloadFailureKind::Other => format!("download from {source_url} failed: {error}"),
    }
}

fn classify_reqwest_failure(err: &reqwest::Error) -> DownloadFailureKind {
    if err.is_timeout() {
        DownloadFailureKind::Timeout
    } else if err.is_connect() {
        DownloadFailureKind::Unavailable
    } else {
        DownloadFailureKind::Other
    }
}

fn classify_read_failure(err: &std::io::Error) -> DownloadFailureKind {
    match err.kind() {
        ErrorKind::TimedOut => DownloadFailureKind::Timeout,
        ErrorKind::ConnectionRefused
        | ErrorKind::ConnectionAborted
        | ErrorKind::ConnectionReset
        | ErrorKind::NotConnected
        | ErrorKind::BrokenPipe
        | ErrorKind::UnexpectedEof => DownloadFailureKind::Unavailable,
        _ => DownloadFailureKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::blocking::Client;
    use std::io::Error;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

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
            .start_download(
                "   ".to_string(),
                destination.to_string_lossy().to_string(),
                None,
            )
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
                None,
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
                None,
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
                None,
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

    #[test]
    fn cancel_download_returns_error_for_unknown_id() {
        let service = DownloaderService::new();

        let err = service
            .cancel_download("download-missing")
            .expect_err("expected unknown download_id to fail");

        assert_eq!(err, "unknown download_id: download-missing");
    }

    #[test]
    fn start_download_rejects_existing_destination_path() {
        let service = DownloaderService::new();
        let temp_dir = unique_temp_dir("downloader-existing-destination");
        let destination = temp_dir.join("model.bin");
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        File::create(&destination).expect("failed to create destination file");

        let err = service
            .start_download(
                "https://example.com/model.bin".to_string(),
                destination.to_string_lossy().to_string(),
                None,
            )
            .expect_err("expected existing destination_path to fail");

        assert_eq!(
            err,
            format!("destination_path already exists: {}", destination.display())
        );
    }

    #[test]
    fn build_temp_path_uses_destination_file_name_and_download_id() {
        let destination = PathBuf::from(r"C:\tmp\models\model.bin");
        let temp_path = build_temp_path(&destination, "download-42");

        assert_eq!(
            temp_path,
            PathBuf::from(r"C:\tmp\models\model.bin.download-42.part")
        );
    }

    #[test]
    fn download_failure_message_distinguishes_categories() {
        let source_url = "https://example.com/model.bin";

        assert_eq!(
            download_failure_message(
                source_url,
                DownloadFailureKind::Timeout,
                "deadline exceeded"
            ),
            "download from https://example.com/model.bin timed out: deadline exceeded"
        );
        assert_eq!(
            download_failure_message(
                source_url,
                DownloadFailureKind::Unavailable,
                "connection refused"
            ),
            "download from https://example.com/model.bin is unavailable: connection refused"
        );
        assert_eq!(
            download_failure_message(source_url, DownloadFailureKind::Other, "bad response"),
            "download from https://example.com/model.bin failed: bad response"
        );
    }

    #[test]
    fn classify_read_failure_maps_io_error_kinds() {
        assert_eq!(
            classify_read_failure(&Error::new(ErrorKind::TimedOut, "timed out")),
            DownloadFailureKind::Timeout
        );
        assert_eq!(
            classify_read_failure(&Error::new(
                ErrorKind::ConnectionReset,
                "connection reset by peer"
            )),
            DownloadFailureKind::Unavailable
        );
        assert_eq!(
            classify_read_failure(&Error::new(ErrorKind::UnexpectedEof, "unexpected eof")),
            DownloadFailureKind::Unavailable
        );
        assert_eq!(
            classify_read_failure(&Error::new(ErrorKind::InvalidData, "bad response")),
            DownloadFailureKind::Other
        );
    }

    #[test]
    fn classify_reqwest_failure_marks_connection_refused_unavailable() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("should bind ephemeral port");
        let port = listener.local_addr().expect("local addr available").port();
        drop(listener);

        let source_url = format!("http://127.0.0.1:{port}/model.bin");
        let client = Client::builder().build().expect("client should build");
        let error = client
            .get(&source_url)
            .send()
            .expect_err("request should fail against closed port");

        assert!(error.is_connect());
        assert_eq!(
            classify_reqwest_failure(&error),
            DownloadFailureKind::Unavailable
        );
    }

    #[test]
    fn classify_reqwest_failure_marks_hanging_server_timeout() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("should bind ephemeral port");
        let port = listener.local_addr().expect("local addr available").port();

        let server = thread::spawn(move || {
            if let Ok((_socket, _addr)) = listener.accept() {
                thread::sleep(Duration::from_millis(400));
            }
        });

        let source_url = format!("http://127.0.0.1:{port}/model.bin");
        let client = Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .expect("client should build");
        let error = client
            .get(&source_url)
            .send()
            .expect_err("request should time out against hanging server");

        assert!(error.is_timeout());
        assert_eq!(
            classify_reqwest_failure(&error),
            DownloadFailureKind::Timeout
        );
        server.join().expect("server thread should complete");
    }
}
