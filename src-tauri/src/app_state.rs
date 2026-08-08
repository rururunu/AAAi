use std::sync::Arc;

use crate::adapters::TauriEventBus;
use crate::core::chat::ChatService;
use crate::core::PeekCore;

pub struct AppState {
    pub core: PeekCore,
}

impl AppState {
    pub fn new(app: tauri::AppHandle) -> Self {
        let event_bus = Arc::new(TauriEventBus::new(app.clone()));
        let core = PeekCore::new(app, event_bus);

        Self { core }
    }
}

/// Run a closure against ChatService when AppState is available.
#[allow(dead_code)]
pub fn with_chat<T>(
    app: &tauri::AppHandle,
    f: impl FnOnce(&ChatService) -> T,
) -> Result<T, String> {
    use tauri::Manager;
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "app state unavailable".to_string())?;
    Ok(f(state.core.chat()))
}
