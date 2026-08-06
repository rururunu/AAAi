use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::runtime::encoding::decode_process_bytes;
use crate::runtime::terminal::{prepare_command, prepare_powershell};

use super::error::ToolError;

const WAIT_POLL: Duration = Duration::from_millis(100);
const WAIT_TIMEOUT: Duration = Duration::from_secs(120);
const BACKGROUND_OUTPUT_MAX_CHARS: usize = 256 * 1024;
const BACKGROUND_OUTPUT_MAX_BYTES: usize = BACKGROUND_OUTPUT_MAX_CHARS * 4;

#[derive(Debug)]
pub struct ShellJob {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub command: String,
    pub output: String,
    raw_output: Vec<u8>,
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
        prepare_powershell(&mut cmd, &command);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        if crate::core::tools::sandbox::restricted_shell() {
            crate::core::tools::sandbox::scrub_sensitive_env(&mut cmd);
        }
        let mut child = cmd.spawn()?;
        if crate::core::tools::sandbox::restricted_shell() {
            crate::core::tools::sandbox::assign_restricted_job(&mut child);
        }
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

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
                    raw_output: Vec::new(),
                    done: false,
                    exit_code: None,
                    cwd: cwd.map(std::path::Path::to_path_buf),
                    child: Some(child),
                    cancelled,
                },
            );
        }

        let stdout_reader =
            stdout.map(|stream| spawn_output_reader(Arc::clone(self), id.clone(), stream));
        let stderr_reader =
            stderr.map(|stream| spawn_output_reader(Arc::clone(self), id.clone(), stream));
        let store = Arc::clone(self);
        let job_id = id.clone();
        std::thread::spawn(move || {
            store.finish_job(&job_id, stdout_reader, stderr_reader);
        });

        Ok(id)
    }

    /// Take the child under a short lock, wait outside the lock, then publish.
    fn finish_job(
        &self,
        job_id: &str,
        stdout_reader: Option<std::thread::JoinHandle<()>>,
        stderr_reader: Option<std::thread::JoinHandle<()>>,
    ) {
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
        let exit_code = child.wait().ok().and_then(|status| status.code());
        if let Some(reader) = stdout_reader {
            let _ = reader.join();
        }
        if let Some(reader) = stderr_reader {
            let _ = reader.join();
        }

        if let Ok(mut guard) = self.jobs.lock() {
            if let Some(job) = guard.get_mut(job_id) {
                if was_cancelled {
                    append_bounded(&mut job.output, "\n[cancelled]\n");
                }
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

    pub fn read_output_limited(
        &self,
        job_id: &str,
        tail_lines: Option<usize>,
        max_chars: Option<usize>,
    ) -> Result<String, ToolError> {
        let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        let mut output = job.output.clone();
        if let Some(lines) = tail_lines.filter(|value| *value > 0) {
            let all: Vec<&str> = output.lines().collect();
            output = all[all.len().saturating_sub(lines)..].join("\n");
        }
        if let Some(limit) = max_chars.filter(|value| *value > 0) {
            output = take_tail_chars(&output, limit);
        }
        Ok(format_job_status_with_output(job, &output))
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
    format_job_status_with_output(job, &job.output)
}

fn format_job_status_with_output(job: &ShellJob, output: &str) -> String {
    format!(
        "status: {}\nexit_code: {:?}\n{}",
        if job.done { "done" } else { "running" },
        job.exit_code,
        output
    )
}

fn spawn_output_reader<R: Read + Send + 'static>(
    store: Arc<ShellJobStore>,
    job_id: String,
    mut stream: R,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if let Ok(mut jobs) = store.jobs.lock() {
                        if let Some(job) = jobs.get_mut(&job_id) {
                            append_raw_bounded(job, &buffer[..read]);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn append_raw_bounded(job: &mut ShellJob, chunk: &[u8]) {
    job.raw_output.extend_from_slice(chunk);
    if job.raw_output.len() > BACKGROUND_OUTPUT_MAX_BYTES {
        let keep = BACKGROUND_OUTPUT_MAX_BYTES;
        let drain = job.raw_output.len() - keep;
        job.raw_output.drain(..drain);
    }
    job.output = decode_process_bytes(&job.raw_output);
    let count = job.output.chars().count();
    if count > BACKGROUND_OUTPUT_MAX_CHARS {
        job.output = take_tail_chars(&job.output, BACKGROUND_OUTPUT_MAX_CHARS);
    }
}

fn append_bounded(output: &mut String, chunk: &str) {
    output.push_str(chunk);
    let count = output.chars().count();
    if count > BACKGROUND_OUTPUT_MAX_CHARS {
        *output = take_tail_chars(output, BACKGROUND_OUTPUT_MAX_CHARS);
    }
}

fn take_tail_chars(value: &str, limit: usize) -> String {
    let count = value.chars().count();
    value.chars().skip(count.saturating_sub(limit)).collect()
}

/// Only commands expected to stay alive are allowed to allocate a background
/// job. This prevents routine reads, tests, builds, Git, and Docker inspection
/// from being turned into noisy `job-n` handles merely to avoid waiting.
pub fn background_allowed(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    let persistent_markers = [
        "get-content -wait",
        "tail -f",
        "docker logs -f",
        "docker logs --follow",
        "docker compose logs -f",
        "docker compose logs --follow",
        "docker-compose logs -f",
        "docker-compose logs --follow",
        "npm run dev",
        "pnpm dev",
        "yarn dev",
        "bun run dev",
        "vite --host",
        "webpack --watch",
        "cargo watch",
        "dotnet watch",
    ];
    if persistent_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    let trimmed = normalized.trim();
    (trimmed.starts_with("docker compose up")
        || trimmed.starts_with("docker-compose up")
        || trimmed.starts_with("docker run"))
        && !normalized
            .split_whitespace()
            .any(|part| part == "-d" || part == "--detach")
}

fn collect_child_output(child: &mut Child) -> (String, String, Option<i32>) {
    let mut stdout_bytes = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout_bytes);
    }
    let mut stderr_bytes = Vec::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_bytes);
    }
    let status = child.wait().ok();
    (
        decode_process_bytes(&stdout_bytes),
        decode_process_bytes(&stderr_bytes),
        status.and_then(|s| s.code()),
    )
}

fn format_streams(stdout: &str, stderr: &str) -> String {
    format!("stdout:\n{stdout}\nstderr:\n{stderr}")
}

pub fn run_foreground(
    command: &str,
    cwd: Option<&std::path::Path>,
    cancelled: &AtomicBool,
) -> Result<String, ToolError> {
    let restricted = crate::core::tools::sandbox::restricted_shell();
    let timeout = Duration::from_secs(crate::core::tools::sandbox::shell_timeout_secs());
    let mut cmd = Command::new("powershell");
    prepare_powershell(&mut cmd, command);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    if restricted {
        crate::core::tools::sandbox::scrub_sensitive_env(&mut cmd);
    }
    let mut child = cmd.spawn()?;
    if restricted {
        crate::core::tools::sandbox::assign_restricted_job(&mut child);
    }

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
                if started.elapsed() >= timeout {
                    terminate_process_tree(&mut child);
                    let (stdout, stderr, exit_code) = collect_child_output(&mut child);
                    let result = format!(
                        "command timed out after {}s (exit_code: {:?})\n{}",
                        timeout.as_secs(),
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
        let mut command = Command::new("taskkill");
        command
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        prepare_command(&mut command);
        let _ = command.status();
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

    #[test]
    fn finite_commands_are_not_allowed_in_background() {
        assert!(!background_allowed("git status"));
        assert!(!background_allowed("pnpm build"));
        assert!(!background_allowed("docker compose ps"));
        assert!(!background_allowed("docker compose logs --tail 100"));
        assert!(background_allowed("docker compose logs -f --tail 100"));
        assert!(background_allowed("Get-Content -Wait -Tail 100 app.log"));
    }

    #[test]
    fn background_output_is_readable_before_process_exits() {
        let store = ShellJobStore::new();
        let id = store
            .spawn_background(
                "Write-Output 'first'; Start-Sleep -Milliseconds 800; Write-Output 'second'".into(),
                None,
                Arc::new(AtomicBool::new(false)),
            )
            .expect("spawn");

        let deadline = Instant::now() + Duration::from_secs(3);
        let running = loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("first") {
                break status;
            }
            assert!(Instant::now() < deadline, "first log line was not streamed");
            std::thread::sleep(Duration::from_millis(25));
        };
        assert!(running.contains("status: running"));

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("status: done") {
                assert!(status.contains("first"));
                assert!(status.contains("second"));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background command did not finish"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
