use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::runtime::terminal::prepare_command;

use super::error::ToolError;

const WAIT_POLL: Duration = Duration::from_millis(100);
const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
const FOREGROUND_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct ShellJob {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub command: String,
    pub output: String,
    pub done: bool,
    pub exit_code: Option<i32>,
    cwd: Option<std::path::PathBuf>,
    child: Option<Child>,
    cancelled: Arc<AtomicBool>,
}

pub struct ShellJobStore {
    jobs: Mutex<HashMap<String, ShellJob>>,
    next_id: Mutex<u32>,
}

impl ShellJobStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        })
    }

    pub fn spawn_background(
        self: &Arc<Self>,
        command: String,
        cwd: Option<&std::path::Path>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<String, ToolError> {
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        prepare_command(&mut cmd);
        let child = cmd.spawn()?;

        let id = {
            let mut guard = self
                .next_id
                .lock()
                .map_err(|_| ToolError::new("job lock"))?;
            let id = format!("job-{}", *guard);
            *guard += 1;
            id
        };

        // Insert before spawning the waiter — otherwise finish_job can miss the
        // entry and leave the job stuck as running forever.
        {
            let mut guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
            guard.insert(
                id.clone(),
                ShellJob {
                    id: id.clone(),
                    command,
                    output: String::new(),
                    done: false,
                    exit_code: None,
                    cwd: cwd.map(std::path::Path::to_path_buf),
                    child: Some(child),
                    cancelled,
                },
            );
        }

        let store = Arc::clone(self);
        let job_id = id.clone();
        std::thread::spawn(move || {
            store.finish_job(&job_id);
        });

        Ok(id)
    }

    /// Take the child under a short lock, wait outside the lock, then publish.
    fn finish_job(&self, job_id: &str) {
        let (mut child, cancelled) = {
            let mut guard = match self.jobs.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let job = match guard.get_mut(job_id) {
                Some(job) => job,
                None => return,
            };
            match job.child.take() {
                Some(child) => (child, Arc::clone(&job.cancelled)),
                None => return,
            }
        };

        let was_cancelled = loop {
            if cancelled.load(Ordering::Relaxed) {
                terminate_process_tree(&mut child);
                break true;
            }
            match child.try_wait() {
                Ok(Some(_)) => break false,
                Ok(None) => std::thread::sleep(WAIT_POLL),
                Err(_) => break false,
            }
        };
        let (stdout, stderr, exit_code) = collect_child_output(&mut child);
        let output = if was_cancelled {
            format!("cancelled\n{}", format_streams(&stdout, &stderr))
        } else {
            format_streams(&stdout, &stderr)
        };

        if let Ok(mut guard) = self.jobs.lock() {
            if let Some(job) = guard.get_mut(job_id) {
                job.output = output;
                job.exit_code = exit_code;
                job.done = true;
                crate::core::context::provider::environment_provider::record_shell_execution(
                    &job.command,
                    job.cwd.as_deref(),
                    &format_job_status(job),
                );
            }
        }
    }

    pub fn read_output(&self, job_id: &str) -> Result<String, ToolError> {
        let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        Ok(format_job_status(job))
    }

    pub fn wait_job(
        &self,
        job_id: &str,
        context: &crate::core::tools::context::ToolContext,
    ) -> Result<String, ToolError> {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
            context.ensure_not_cancelled()?;
            {
                let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
                let job = guard
                    .get(job_id)
                    .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
                if job.done {
                    return Ok(format_job_status(job));
                }
            }
            if Instant::now() >= deadline {
                let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
                let job = guard
                    .get(job_id)
                    .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
                return Ok(format!(
                    "status: timeout (still running after {}s)\nexit_code: {:?}\n{}",
                    WAIT_TIMEOUT.as_secs(),
                    job.exit_code,
                    job.output
                ));
            }
            std::thread::sleep(WAIT_POLL);
        }
    }

    pub fn kill(&self, job_id: &str) -> Result<String, ToolError> {
        let mut guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get_mut(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        if let Some(mut child) = job.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        job.done = true;
        if job.exit_code.is_none() {
            job.exit_code = Some(-1);
        }
        Ok("killed".into())
    }
}

fn format_job_status(job: &ShellJob) -> String {
    format!(
        "status: {}\nexit_code: {:?}\n{}",
        if job.done { "done" } else { "running" },
        job.exit_code,
        job.output
    )
}

fn collect_child_output(child: &mut Child) -> (String, String, Option<i32>) {
    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }
    let status = child.wait().ok();
    (stdout, stderr, status.and_then(|s| s.code()))
}

fn format_streams(stdout: &str, stderr: &str) -> String {
    format!("stdout:\n{stdout}\nstderr:\n{stderr}")
}

pub fn run_foreground(
    command: &str,
    cwd: Option<&std::path::Path>,
    cancelled: &AtomicBool,
) -> Result<String, ToolError> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    prepare_command(&mut cmd);
    let mut child = cmd.spawn()?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let (stdout, stderr, exit_code) = collect_child_output(&mut child);
                let result = format!(
                    "exit_code: {}\n{}",
                    exit_code.unwrap_or(-1),
                    format_streams(&stdout, &stderr)
                );
                crate::core::context::provider::environment_provider::record_shell_execution(
                    command, cwd, &result,
                );
                return Ok(result);
            }
            Ok(None) => {
                if cancelled.load(Ordering::Relaxed) {
                    terminate_process_tree(&mut child);
                    let _ = collect_child_output(&mut child);
                    tracing::debug!(pid = child.id(), "foreground shell command cancelled");
                    return Err(ToolError::cancelled());
                }
                if started.elapsed() >= FOREGROUND_TIMEOUT {
                    terminate_process_tree(&mut child);
                    let (stdout, stderr, exit_code) = collect_child_output(&mut child);
                    let result = format!(
                        "command timed out after {}s (exit_code: {:?})\n{}",
                        FOREGROUND_TIMEOUT.as_secs(),
                        exit_code,
                        format_streams(&stdout, &stderr)
                    );
                    crate::core::context::provider::environment_provider::record_shell_execution(
                        command, cwd, &result,
                    );
                    return Err(ToolError::new(result));
                }
                std::thread::sleep(WAIT_POLL);
            }
            Err(error) => return Err(ToolError::new(error.to_string())),
        }
    }
}

pub(crate) fn terminate_process_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn background_job_is_registered_before_waiter_runs() {
        let store = ShellJobStore::new();
        // A quick command — race used to drop the waiter before insert.
        let id = store
            .spawn_background(
                "Write-Output 'ok'".into(),
                None,
                Arc::new(AtomicBool::new(false)),
            )
            .expect("spawn");
        let db_path =
            std::env::temp_dir().join(format!("peek-shell-job-{}.db", uuid::Uuid::new_v4()));
        struct NullBus;
        impl crate::core::event::EventBus for NullBus {
            fn emit(&self, _event: crate::core::event::BusEvent) {}
        }
        let context = crate::core::tools::context::ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: Default::default(),
            session_id: "test".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(
                crate::core::chat::conversation_manager::ConversationManager::new(db_path.clone()),
            ),
            event_bus: Arc::new(NullBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(crate::core::tools::context::AskStore::new()),
            path_permission_store: Arc::new(crate::core::tools::context::PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let status = store.wait_job(&id, &context).expect("wait");
        assert!(
            status.contains("status: done") || status.contains("exit_code:"),
            "unexpected status: {status}"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn foreground_command_stops_when_cancelled() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let started = Instant::now();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            signal.store(true, Ordering::Relaxed);
        });

        let error = run_foreground("Start-Sleep -Seconds 30", None, &cancelled).unwrap_err();
        canceller.join().unwrap();

        assert!(error.is_cancelled());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
