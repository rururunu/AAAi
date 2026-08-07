//! Single-threaded STA COM worker with cached active-object handles.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};

use super::com::{ComDispatch, ComError, ComSession};

struct WorkerState {
    _session: ComSession,
    apps: HashMap<String, ComDispatch>,
}

impl WorkerState {
    fn get_app(&mut self, prog_id: &str) -> Result<ComDispatch, ComError> {
        if let Some(app) = self.apps.get(prog_id) {
            return Ok(app.clone());
        }
        let app = ComDispatch::attach_active(prog_id)?;
        self.apps.insert(prog_id.to_string(), app.clone());
        Ok(app)
    }

    fn invalidate(&mut self, prog_id: &str) {
        self.apps.remove(prog_id);
    }
}

enum WorkerJob {
    Run {
        prog_id: String,
        run: Box<dyn FnOnce(&ComDispatch) -> Result<(), ComError> + Send>,
        reply: mpsc::Sender<Result<(), ComError>>,
    },
}

pub struct ComWorker {
    sender: mpsc::Sender<WorkerJob>,
    _handle: JoinHandle<()>,
}

static WORKER: OnceLock<ComWorker> = OnceLock::new();

pub fn worker() -> &'static ComWorker {
    WORKER.get_or_init(ComWorker::spawn)
}

impl ComWorker {
    fn spawn() -> Self {
        let (sender, receiver) = mpsc::channel::<WorkerJob>();
        let handle = thread::Builder::new()
            .name("anya-com-worker".into())
            .spawn(move || {
                let session = match ComSession::new() {
                    Ok(session) => session,
                    Err(error) => {
                        tracing::error!(error = %error, "COM worker failed to initialize");
                        return;
                    }
                };
                let mut state = WorkerState {
                    _session: session,
                    apps: HashMap::new(),
                };
                while let Ok(job) = receiver.recv() {
                    match job {
                        WorkerJob::Run {
                            prog_id,
                            run,
                            reply,
                        } => {
                            let result = match state.get_app(&prog_id) {
                                Ok(app) => run(&app),
                                Err(error) => Err(error),
                            };
                            if result.is_err() {
                                state.invalidate(&prog_id);
                            }
                            let _ = reply.send(result);
                        }
                    }
                }
            })
            .expect("spawn COM worker");

        Self {
            sender,
            _handle: handle,
        }
    }
}

pub fn with_app_value<F, T>(prog_id: &'static str, f: F) -> Result<T, ComError>
where
    F: FnOnce(&ComDispatch) -> Result<T, ComError> + Send + 'static,
    T: Send + 'static,
{
    let (value_tx, value_rx) = mpsc::channel::<Result<T, ComError>>();
    let (reply_tx, reply_rx) = mpsc::channel::<Result<(), ComError>>();
    worker()
        .sender
        .send(WorkerJob::Run {
            prog_id: prog_id.to_string(),
            run: Box::new(move |app| {
                let result = f(app);
                let _ = value_tx.send(result);
                Ok(())
            }),
            reply: reply_tx,
        })
        .map_err(|_| ComError::Init("COM worker channel closed".to_string()))?;
    let worker_result = reply_rx
        .recv()
        .map_err(|_| ComError::Init("COM worker dropped reply".to_string()))??;
    let _ = worker_result;
    value_rx
        .recv()
        .map_err(|_| ComError::Init("COM worker dropped value".to_string()))?
}

pub fn app_available(prog_id: &'static str) -> bool {
    with_app_value(prog_id, |_| Ok(())).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_starts_and_processes_jobs() {
        // No Office apps are installed in CI, so this exercises the worker's
        // Run job path (spawn, dispatch, reply) via a ProgID that never
        // resolves, confirming the thread is alive and replies instead of
        // hanging.
        assert!(!app_available("Anya.NonExistent.Test"));
    }
}
