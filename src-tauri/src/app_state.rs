use std::sync::Arc;

use crate::adapters::TauriEventBus;
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
