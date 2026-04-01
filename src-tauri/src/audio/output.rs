use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream, StreamConfig,
};
use rtrb::Consumer;
use std::sync::Arc;

use crate::error::{AppError, Result};
use super::control::PlaybackState;

/// Abstraction over the audio hardware output.
///
/// The real implementation routes samples through cpal/CoreAudio.
/// In tests, `NullSink` discards all samples so no audio hardware is required.
#[allow(dead_code)]
pub trait AudioSink: Send + 'static {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
}

// ---------------------------------------------------------------------------
// Real cpal sink
// ---------------------------------------------------------------------------

/// Wraps a live `cpal::Stream`. Dropping this stops audio output.
pub struct CpalSink {
    _stream: Stream,
    #[allow(dead_code)]
    sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
}

impl AudioSink for CpalSink {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn channels(&self) -> u16 {
        self.channels
    }
}

// Safety: `cpal::Stream` on CoreAudio wraps an Arc<Mutex<StreamInner>> where
// StreamInner contains a Box<dyn FnMut()> that is not automatically Send.
// We never call the stream's callback directly — CoreAudio calls it on its own
// thread — and we only access `CpalSink` through a `parking_lot::Mutex` in
// `AppState`, ensuring exclusive access. No raw pointers are shared across
// thread boundaries by our code.
unsafe impl Send for CpalSink {}
unsafe impl Sync for CpalSink {}

/// Open the default output device and start streaming samples from `consumer`.
///
/// `state` is shared with the decoder thread; the callback reads `is_playing`
/// to decide whether to drain the ring buffer or fill with silence.
pub fn open_cpal_sink(
    consumer: Consumer<f32>,
    sample_rate: u32,
    channels: u16,
    state: Arc<PlaybackState>,
) -> Result<CpalSink> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AppError::AudioOutput("No default output device found".into()))?;

    let config = StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let mut consumer = consumer;

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                if !state.get_is_playing() {
                    for s in data.iter_mut() {
                        *s = 0.0;
                    }
                    return;
                }
                for sample in data.iter_mut() {
                    *sample = consumer.pop().unwrap_or(0.0);
                }
            },
            |err| eprintln!("[cpal] Stream error: {err}"),
            None,
        )
        .map_err(|e| AppError::AudioOutput(e.to_string()))?;

    stream.play().map_err(|e| AppError::AudioOutput(e.to_string()))?;

    Ok(CpalSink { _stream: stream, sample_rate, channels })
}

// ---------------------------------------------------------------------------
// NullSink — for unit tests (never calls cpal)
// ---------------------------------------------------------------------------

/// A no-op sink that satisfies the `AudioSink` trait without opening any
/// hardware device. Used exclusively in tests.
#[cfg(any(test, feature = "test-audio"))]
#[allow(dead_code)]
pub struct NullSink {
    sample_rate: u32,
    channels: u16,
}

#[cfg(any(test, feature = "test-audio"))]
impl NullSink {
    #[allow(dead_code)]
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self { sample_rate, channels }
    }
}

#[cfg(any(test, feature = "test-audio"))]
impl AudioSink for NullSink {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn channels(&self) -> u16 {
        self.channels
    }
}
