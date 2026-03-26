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
