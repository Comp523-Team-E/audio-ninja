use std::sync::{
    mpsc::{self, SyncSender},
    Arc,
};
use rtrb::RingBuffer;

use crate::error::{AppError, Result};
use super::control::{ControlMsg, PlaybackState};
use super::decoder::{probe_file, run_decoder};
use super::output::{open_cpal_sink, AudioSink};
#[cfg(any(test, feature = "test-audio"))]
use super::output::NullSink;

/// Ring buffer capacity in samples.
/// At 44100 Hz stereo this is ~0.37 seconds of buffered audio.
const RING_BUFFER_CAPACITY: usize = 32_768;

/// Metadata extracted from a media file on open.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub file_name: String,
    pub file_path: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

/// The live audio engine for a single loaded file.
///
/// Owns the decoder thread and the cpal output stream for the lifetime of the
/// loaded file. Dropping it sends a `Stop` message and joins the decoder thread.
pub struct AudioEngine {
    control_tx: SyncSender<ControlMsg>,
    pub state: Arc<PlaybackState>,
    pub metadata: FileMetadata,
    decoder_handle: Option<std::thread::JoinHandle<()>>,
    /// Kept alive so cpal keeps streaming; dropped when the engine is dropped.
    _sink: Box<dyn AudioSink + Send>,
}

impl AudioEngine {
    /// Open `path`, start the decoder thread and cpal output, return the engine.
    /// The engine starts in a paused state; call `play()` to begin playback.
    pub fn open(path: &str) -> Result<Self> {
        // Probe the file for metadata.
        let init = probe_file(path).map_err(AppError::Decode)?;

        let file_name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();

        let metadata = FileMetadata {
            file_name,
            file_path: path.to_string(),
            duration_ms: init.duration_ms,
            sample_rate: init.sample_rate,
            channels: init.channels,
        };

        let state = Arc::new(PlaybackState::new());
        state.set_duration_ms(init.duration_ms as f64);
        // Start paused.
        state.set_is_playing(false);

        // Build the SPSC ring buffer shared between decoder (producer) and cpal (consumer).
        let (producer, consumer) = RingBuffer::<f32>::new(RING_BUFFER_CAPACITY);

        // Control channel: bounded so the UI cannot queue unbounded messages.
        let (control_tx, control_rx) = mpsc::sync_channel::<ControlMsg>(8);

        // Start the cpal output stream.
        let sink = open_cpal_sink(
            consumer,
            init.sample_rate,
            init.channels,
            Arc::clone(&state),
        )?;

        let path_owned = path.to_string();
        let state_for_decoder = Arc::clone(&state);

        let handle = std::thread::Builder::new()
            .name("audio-decoder".to_string())
            .spawn(move || {
                run_decoder(path_owned, producer, control_rx, state_for_decoder, 1.0);
            })
            .map_err(|e| AppError::AudioOutput(e.to_string()))?;

        Ok(AudioEngine {
            control_tx,
            state,
            metadata,
            decoder_handle: Some(handle),
            _sink: Box::new(sink),
        })
    }

    /// Like `open()` but uses `NullSink` instead of opening audio hardware.
    /// Available in tests and when the `test-audio` feature is enabled.
    #[cfg(any(test, feature = "test-audio"))]
    #[allow(dead_code)]
    pub fn open_with_null_sink(path: &str) -> Result<Self> {
        let init = probe_file(path).map_err(AppError::Decode)?;

        let file_name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();

        let metadata = FileMetadata {
            file_name,
            file_path: path.to_string(),
            duration_ms: init.duration_ms,
            sample_rate: init.sample_rate,
            channels: init.channels,
        };

        let state = Arc::new(PlaybackState::new());
        state.set_duration_ms(init.duration_ms as f64);
        state.set_is_playing(false);

        // Small ring buffer (256 < typical file size) creates back-pressure so the
        // decoder stays in its write loop with is_playing=true. The cpal consumer
        // is absent (NullSink), so samples are never drained. This is intentional
        // for tests — Drop sends Stop to unblock the decoder.
        let (producer, _consumer) = RingBuffer::<f32>::new(256);
        let (control_tx, control_rx) = mpsc::sync_channel::<ControlMsg>(8);

        let sink = NullSink::new(init.sample_rate, init.channels);

        let path_owned = path.to_string();
        let state_for_decoder = Arc::clone(&state);

        let handle = std::thread::Builder::new()
            .name("audio-decoder-test".to_string())
            .spawn(move || {
                run_decoder(path_owned, producer, control_rx, state_for_decoder, 1.0);
            })
            .map_err(|e| AppError::AudioOutput(e.to_string()))?;

        Ok(AudioEngine {
            control_tx,
            state,
            metadata,
            decoder_handle: Some(handle),
            _sink: Box::new(sink),
        })
    }

    pub fn play(&self) -> Result<()> {
        self.send(ControlMsg::Play)
    }

    pub fn pause(&self) -> Result<()> {
        self.send(ControlMsg::Pause)
    }

    pub fn seek(&self, position_ms: u64) -> Result<()> {
        self.send(ControlMsg::Seek(position_ms))
    }

    pub fn set_speed(&self, speed: f64) -> Result<()> {
        self.send(ControlMsg::SetSpeed(speed))
    }

    pub fn set_loop(&self, enabled: bool) -> Result<()> {
        self.send(ControlMsg::SetLoop(enabled))
    }

    fn send(&self, msg: ControlMsg) -> Result<()> {
        self.control_tx
            .send(msg)
            .map_err(|_| AppError::AudioOutput("Decoder thread has exited".into()))
    }

    /// Force the decoder thread to exit. Used in tests to trigger the
    /// "send after decoder exit" error path.
    #[cfg(test)]
    pub fn kill_decoder_for_test(&mut self) {
        let _ = self.control_tx.send(ControlMsg::Stop);
        if let Some(handle) = self.decoder_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.control_tx.send(ControlMsg::Stop);
        if let Some(handle) = self.decoder_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Re-use the WAV writer from decoder tests.
    fn write_test_wav(path: &std::path::Path, sample_rate: u32, channels: u16, num_frames: u32) {
        use std::io::Write;
        let bits_per_sample: u16 = 16;
        let block_align = channels * bits_per_sample / 8;
        let byte_rate = sample_rate * block_align as u32;
        let data_size = num_frames * block_align as u32;
        let riff_size = 36u32 + data_size;
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(b"RIFF").unwrap();
        f.write_all(&riff_size.to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap();
        f.write_all(&channels.to_le_bytes()).unwrap();
        f.write_all(&sample_rate.to_le_bytes()).unwrap();
        f.write_all(&byte_rate.to_le_bytes()).unwrap();
        f.write_all(&block_align.to_le_bytes()).unwrap();
        f.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&data_size.to_le_bytes()).unwrap();
        for i in 0..num_frames {
            for _ch in 0..channels {
                let s = ((i as f64 * 2.0 * std::f64::consts::PI / sample_rate as f64 * 440.0).sin()
                    * i16::MAX as f64) as i16;
                f.write_all(&s.to_le_bytes()).unwrap();
            }
        }
        f.flush().unwrap();
    }

    fn wait_until(predicate: impl Fn() -> bool, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if predicate() { return true; }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn open_with_null_sink_valid_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        assert_eq!(engine.metadata.sample_rate, 44100);
        assert_eq!(engine.metadata.channels, 1);
        assert!(engine.metadata.duration_ms > 0);
        assert_eq!(engine.metadata.file_name, "t.wav");
    }

    #[test]
    fn open_with_null_sink_missing_file_returns_err() {
        let result = AudioEngine::open_with_null_sink("/nonexistent/file.wav");
        assert!(matches!(result, Err(AppError::Decode(_))));
    }

    #[test]
    fn open_with_null_sink_path_without_filename() {
        // A path whose file_name() is None — use a bare stem like "/" — falls back
        // to using the full path string as file_name.
        // Use a known-missing path so probe_file returns an Err; we just check
        // that the fallback path is reachable by looking at what probe_file returns.
        // (A real "/" directory would panic Symphonia; use a missing path with no stem.)
        let result = AudioEngine::open_with_null_sink("/");
        // We only care that the code path compiles and runs without panic.
        // On most systems this returns an Err because "/" is not an audio file.
        let _ = result;
    }

    #[test]
    fn play_sends_play_message() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        engine.play().unwrap();
        assert!(wait_until(|| engine.state.get_is_playing(), Duration::from_secs(2)));
    }

    #[test]
    fn pause_after_play() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        engine.play().unwrap();
        assert!(wait_until(|| engine.state.get_is_playing(), Duration::from_secs(2)));
        engine.pause().unwrap();
        assert!(wait_until(|| !engine.state.get_is_playing(), Duration::from_secs(2)));
    }

    #[test]
    fn seek_within_range() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        assert!(engine.seek(50).is_ok());
    }

    #[test]
    fn set_speed_valid() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        assert!(engine.set_speed(1.5).is_ok());
    }

    #[test]
    fn set_loop_enabled() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        assert!(engine.set_loop(true).is_ok());
    }

    #[test]
    fn send_after_decoder_exit_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let mut engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        engine.kill_decoder_for_test();
        // Now the decoder thread has exited; sending any message should return an error.
        let result = engine.play();
        assert!(matches!(result, Err(AppError::AudioOutput(_))));
    }

    #[test]
    fn drop_joins_without_deadlock() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("t.wav");
        write_test_wav(&path, 44100, 1, 4410);
        let engine = AudioEngine::open_with_null_sink(path.to_str().unwrap()).unwrap();
        drop(engine); // Must complete without blocking.
    }
}
