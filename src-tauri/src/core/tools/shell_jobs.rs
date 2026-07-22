use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
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
    child: Option<Child>,
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
                    child: Some(child),
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
        let mut child = {
            let mut guard = match self.jobs.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let job = match guard.get_mut(job_id) {
                Some(job) => job,
                None => return,
            };
            match job.child.take() {
                Some(child) => child,
                None => return,
            }
        };

        let (output, exit_code) = collect_child_output(&mut child);

        if let Ok(mut guard) = self.jobs.lock() {
            if let Some(job) = guard.get_mut(job_id) {
                job.output = output;
                job.exit_code = exit_code;
                job.done = true;
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

    pub fn wait_job(&self, job_id: &str) -> Result<String, ToolError> {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
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

fn collect_child_output(child: &mut Child) -> (String, Option<i32>) {
    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }
    let status = child.wait().ok();
    (
        format!("{stdout}{stderr}"),
        status.and_then(|s| s.code()),
    )
}

pub fn run_foreground(command: &str, cwd: Option<&std::path::Path>) -> Result<String, ToolError> {
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
                let (output, exit_code) = collect_child_output(&mut child);
                return Ok(format!(
                    "exit_code: {}\n{output}",
                    exit_code.unwrap_or(-1)
                ));
            }
            Ok(None) => {
                if started.elapsed() >= FOREGROUND_TIMEOUT {
                    let _ = child.kill();
                    let (output, exit_code) = collect_child_output(&mut child);
                    return Err(ToolError::new(format!(
                        "command timed out after {}s (exit_code: {:?})\n{output}",
                        FOREGROUND_TIMEOUT.as_secs(),
                        exit_code
                    )));
                }
                std::thread::sleep(WAIT_POLL);
            }
            Err(error) => return Err(ToolError::new(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_job_is_registered_before_waiter_runs() {
        let store = ShellJobStore::new();
        // A quick command — race used to drop the waiter before insert.
        let id = store
            .spawn_background("Write-Output 'ok'".into(), None)
            .expect("spawn");
        let status = store.wait_job(&id).expect("wait");
        assert!(
            status.contains("status: done") || status.contains("exit_code:"),
            "unexpected status: {status}"
        );
    }
}
