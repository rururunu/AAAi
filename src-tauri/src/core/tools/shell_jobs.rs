use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use super::error::ToolError;

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

        let job = ShellJob {
            id: id.clone(),
            command,
            output: String::new(),
            done: false,
            exit_code: None,
            child: Some(child),
        };

        let store = Arc::clone(self);
        let job_id = id.clone();
        std::thread::spawn(move || {
            store.finish_job(&job_id);
        });

        if let Ok(mut guard) = self.jobs.lock() {
            guard.insert(id.clone(), job);
        }
        Ok(id)
    }

    fn finish_job(&self, job_id: &str) {
        let output = {
            let mut guard = match self.jobs.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            let job = match guard.get_mut(job_id) {
                Some(job) => job,
                None => return,
            };
            let mut child = match job.child.take() {
                Some(child) => child,
                None => return,
            };
            let mut stdout = String::new();
            if let Some(mut out) = child.stdout.take() {
                use std::io::Read;
                let _ = out.read_to_string(&mut stdout);
            }
            let mut stderr = String::new();
            if let Some(mut err) = child.stderr.take() {
                use std::io::Read;
                let _ = err.read_to_string(&mut stderr);
            }
            let status = child.wait().ok();
            job.output = format!("{stdout}{stderr}");
            job.done = true;
            job.exit_code = status.and_then(|s| s.code());
            job.output.clone()
        };
        let _ = output;
    }

    pub fn read_output(&self, job_id: &str) -> Result<String, ToolError> {
        let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        Ok(format!(
            "status: {}\nexit_code: {:?}\n{}",
            if job.done { "done" } else { "running" },
            job.exit_code,
            job.output
        ))
    }

    pub fn wait_job(&self, job_id: &str) -> Result<String, ToolError> {
        for _ in 0..100 {
            {
                let guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
                if let Some(job) = guard.get(job_id) {
                    if job.done {
                        return self.read_output(job_id);
                    }
                } else {
                    return Err(ToolError::new(format!("unknown job: {job_id}")));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        self.read_output(job_id)
    }

    pub fn kill(&self, job_id: &str) -> Result<String, ToolError> {
        let mut guard = self.jobs.lock().map_err(|_| ToolError::new("job lock"))?;
        let job = guard
            .get_mut(job_id)
            .ok_or_else(|| ToolError::new(format!("unknown job: {job_id}")))?;
        if let Some(mut child) = job.child.take() {
            let _ = child.kill();
        }
        job.done = true;
        Ok("killed".into())
    }
}

pub fn run_foreground(command: &str, cwd: Option<&std::path::Path>) -> Result<String, ToolError> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", command]);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(format!(
        "exit_code: {}\n{stdout}{stderr}",
        output.status.code().unwrap_or(-1)
    ))
}
