use crate::core::models::LlamaProcessStatus;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ProcessManager {
    child: Option<Child>,
    logs: Arc<Mutex<VecDeque<String>>>,
    max_logs: usize,
    last_exit_code: Option<i32>,
}

impl ProcessManager {
    pub fn new(max_logs: usize) -> Self {
        Self {
            child: None,
            logs: Arc::new(Mutex::new(VecDeque::new())),
            max_logs,
            last_exit_code: None,
        }
    }

    pub fn start(
        &mut self,
        executable_path: String,
        args: Vec<String>,
    ) -> Result<LlamaProcessStatus, String> {
        self.refresh_status();
        if self.child.is_some() {
            return Err("llama-server is already running".to_string());
        }

        let executable_path = executable_path.trim().to_string();
        if executable_path.is_empty() {
            return Err("executable_path cannot be empty".to_string());
        }

        let mut command = Command::new(&executable_path);
        command.args(args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|e| format!("failed to start llama-server: {e}"))?;

        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        self.last_exit_code = None;
        self.push_log(format!(
            "started llama-server pid={pid} path={executable_path}"
        ));

        if let Some(out) = stdout {
            spawn_log_reader(out, Arc::clone(&self.logs), self.max_logs, "stdout");
        }
        if let Some(err) = stderr {
            spawn_log_reader(err, Arc::clone(&self.logs), self.max_logs, "stderr");
        }

        self.child = Some(child);
        Ok(self.status())
    }

    pub fn stop(&mut self) -> Result<LlamaProcessStatus, String> {
        self.refresh_status();
        if self.child.is_none() {
            return Ok(self.status());
        }

        if let Some(child) = self.child.as_mut() {
            child
                .kill()
                .map_err(|e| format!("failed to stop llama-server: {e}"))?;
            let status = child
                .wait()
                .map_err(|e| format!("failed waiting for llama-server stop: {e}"))?;
            self.last_exit_code = status.code();
            self.push_log(format!(
                "stopped llama-server exit_code={:?}",
                status.code()
            ));
        }

        self.child = None;
        Ok(self.status())
    }

    pub fn status(&mut self) -> LlamaProcessStatus {
        self.refresh_status();
        let pid = self.child.as_ref().map(Child::id);
        LlamaProcessStatus {
            running: self.child.is_some(),
            pid,
            last_exit_code: self.last_exit_code,
        }
    }

    pub fn logs(&self, limit: usize) -> Vec<String> {
        let capped = limit.clamp(1, self.max_logs);
        if let Ok(logs) = self.logs.lock() {
            let len = logs.len();
            let start = len.saturating_sub(capped);
            logs.iter().skip(start).cloned().collect()
        } else {
            vec!["failed to read logs due to poisoned lock".to_string()]
        }
    }

    pub fn clear_logs(&mut self) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.clear();
        }
    }

    fn refresh_status(&mut self) {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.last_exit_code = status.code();
                    self.push_log(format!(
                        "llama-server exited exit_code={:?}",
                        self.last_exit_code
                    ));
                    self.child = None;
                }
                Ok(None) => {}
                Err(e) => {
                    self.push_log(format!("failed to read llama-server status: {e}"));
                }
            }
        }
    }

    fn push_log(&mut self, line: String) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.push_back(line);
            while logs.len() > self.max_logs {
                logs.pop_front();
            }
        }
    }
}

fn spawn_log_reader<R: std::io::Read + Send + 'static>(
    reader: R,
    logs: Arc<Mutex<VecDeque<String>>>,
    max_logs: usize,
    source: &'static str,
) {
    thread::spawn(move || {
        let buffered = BufReader::new(reader);
        for line in buffered.lines() {
            match line {
                Ok(text) => {
                    if let Ok(mut queue) = logs.lock() {
                        queue.push_back(format!("[{source}] {text}"));
                        while queue.len() > max_logs {
                            queue.pop_front();
                        }
                    }
                }
                Err(err) => {
                    if let Ok(mut queue) = logs.lock() {
                        queue.push_back(format!("[{source}] log read error: {err}"));
                        while queue.len() > max_logs {
                            queue.pop_front();
                        }
                    }
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::ProcessManager;

    #[test]
    fn initial_status_is_not_running() {
        let mut manager = ProcessManager::new(3);

        let status = manager.status();

        assert!(!status.running);
        assert_eq!(status.pid, None);
        assert_eq!(status.last_exit_code, None);
    }

    #[test]
    fn start_rejects_empty_executable_path() {
        let mut manager = ProcessManager::new(3);

        let empty_err = manager
            .start(String::new(), vec![])
            .expect_err("empty executable path should fail");
        assert_eq!(empty_err, "executable_path cannot be empty");

        let whitespace_err = manager
            .start("   ".to_string(), vec![])
            .expect_err("whitespace executable path should fail");
        assert_eq!(whitespace_err, "executable_path cannot be empty");
    }

    #[test]
    fn logs_clamp_low_limit_to_one_entry() {
        let mut manager = ProcessManager::new(3);

        manager.push_log("first".to_string());
        manager.push_log("second".to_string());
        manager.push_log("third".to_string());

        assert_eq!(manager.logs(0), vec!["third".to_string()]);
        assert_eq!(manager.logs(1), vec!["third".to_string()]);
    }

    #[test]
    fn logs_clamp_high_limit_to_max_capacity() {
        let mut manager = ProcessManager::new(3);

        manager.push_log("first".to_string());
        manager.push_log("second".to_string());
        manager.push_log("third".to_string());
        manager.push_log("fourth".to_string());

        assert_eq!(
            manager.logs(10),
            vec![
                "second".to_string(),
                "third".to_string(),
                "fourth".to_string()
            ]
        );
        assert_eq!(
            manager.logs(usize::MAX),
            vec![
                "second".to_string(),
                "third".to_string(),
                "fourth".to_string()
            ]
        );
    }

    #[test]
    fn clear_logs_does_not_change_status() {
        let mut manager = ProcessManager::new(3);

        manager.push_log("first".to_string());
        manager.push_log("second".to_string());

        let status_before_clear = manager.status();

        manager.clear_logs();

        let status_after_clear = manager.status();

        assert_eq!(status_before_clear.running, status_after_clear.running);
        assert_eq!(status_before_clear.pid, status_after_clear.pid);
        assert_eq!(
            status_before_clear.last_exit_code,
            status_after_clear.last_exit_code
        );
        assert!(manager.logs(10).is_empty());
    }

    #[test]
    fn stop_without_running_process_returns_status() {
        let mut manager = ProcessManager::new(3);

        let status = manager.stop().expect("stop should not error when idle");

        assert!(!status.running);
        assert_eq!(status.pid, None);
        assert_eq!(status.last_exit_code, None);
    }
}
