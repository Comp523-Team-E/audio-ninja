use std::sync::atomic::{AtomicBool, Ordering};
use atomic_float::AtomicF64;

/// Messages sent from Tauri command handlers to the decoder thread.
#[derive(Debug)]
pub enum ControlMsg {
    Play,
    Pause,
    /// Seek to the given position in milliseconds.
    Seek(u64),
    /// Set playback speed. Values outside [0.1, 4.0] will be clamped by the engine.
    SetSpeed(f64),
    /// Enable or disable looping.
    SetLoop(bool),
    /// Tear down the pipeline and exit the decoder thread.
    Stop,
}

/// Lock-free playback state updated by the audio pipeline and read by IPC polling.
pub struct PlaybackState {
    pub position_ms: AtomicF64,
    pub is_playing: AtomicBool,
    pub duration_ms: AtomicF64,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self {
            position_ms: AtomicF64::new(0.0),
            is_playing: AtomicBool::new(false),
            duration_ms: AtomicF64::new(0.0),
        }
    }

    pub fn get_position_ms(&self) -> f64 {
        self.position_ms.load(Ordering::Relaxed)
    }

    pub fn get_is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    pub fn get_duration_ms(&self) -> f64 {
        self.duration_ms.load(Ordering::Relaxed)
    }

    pub fn set_position_ms(&self, ms: f64) {
        self.position_ms.store(ms, Ordering::Relaxed);
    }

    pub fn set_is_playing(&self, playing: bool) {
        self.is_playing.store(playing, Ordering::Relaxed);
    }

    pub fn set_duration_ms(&self, ms: f64) {
        self.duration_ms.store(ms, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_new_defaults() {
        let s = PlaybackState::new();
        assert_eq!(s.get_position_ms(), 0.0);
        assert!(!s.get_is_playing());
        assert_eq!(s.get_duration_ms(), 0.0);
    }

    #[test]
    fn set_and_get_position_ms() {
        let s = PlaybackState::new();
        s.set_position_ms(1234.5);
        assert_eq!(s.get_position_ms(), 1234.5);
    }

    #[test]
    fn set_and_get_is_playing() {
        let s = PlaybackState::new();
        s.set_is_playing(true);
        assert!(s.get_is_playing());
        s.set_is_playing(false);
        assert!(!s.get_is_playing());
    }

    #[test]
    fn set_and_get_duration_ms() {
        let s = PlaybackState::new();
        s.set_duration_ms(99999.0);
        assert_eq!(s.get_duration_ms(), 99999.0);
    }

    #[test]
    fn control_msg_debug_variants() {
        let _ = format!("{:?}", ControlMsg::Play);
        let _ = format!("{:?}", ControlMsg::Pause);
        let _ = format!("{:?}", ControlMsg::Seek(0));
        let _ = format!("{:?}", ControlMsg::SetSpeed(1.5));
        let _ = format!("{:?}", ControlMsg::SetLoop(true));
        let _ = format!("{:?}", ControlMsg::Stop);
    }
}
