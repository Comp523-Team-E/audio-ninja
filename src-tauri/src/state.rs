use parking_lot::Mutex;

use crate::audio::AudioEngine;
use crate::markers::MarkerStore;

/// Application-wide state managed by Tauri.
///
/// Wrapped in `Mutex` for interior mutability; Tauri's `manage()` wraps it in
/// an `Arc` internally so every command handler receives the same instance.
pub struct AppState {
    /// The active audio engine, or `None` if no file is loaded.
    pub engine: Mutex<Option<AudioEngine>>,
    /// In-memory marker store for the current session.
    pub markers: Mutex<MarkerStore>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
            markers: Mutex::new(MarkerStore::new()),
        }
    }
}
