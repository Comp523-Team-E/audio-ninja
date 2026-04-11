use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

/// A thin wrapper around a rubato `SincFixedIn` resampler.
///
/// Used to apply pitch-preserving time-stretch when playback speed ≠ 1.0.
/// At exactly 1.0x speed the wrapper is bypassed entirely in the decoder loop.
pub struct SpeedResampler {
    inner: SincFixedIn<f32>,
    /// Number of input frames consumed per process call.
    chunk_size: usize,
    channels: usize,
    /// Staging buffer for partial input (per channel).
    partial: Vec<Vec<f32>>,
}

impl SpeedResampler {
    /// Create a resampler for `channels` channels at the given `speed`.
    ///
    /// `speed = 2.0` means the output is half the length (playing twice as fast).
    /// The resampler ratio is therefore `1.0 / speed`.
    pub fn new(channels: usize, speed: f64) -> Self {
        let ratio = 1.0 / speed;
        // chunk_size is the number of input frames per process call.
        // 1024 is a reasonable default that balances latency and quality.
        let chunk_size = 1024usize;

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: WindowFunction::BlackmanHarris2,
        };

        let inner = SincFixedIn::<f32>::new(
            ratio,
            2.0, // max_resample_ratio_relative
            params,
            chunk_size,
            channels,
        )
        .expect("Failed to create resampler");

        Self {
            inner,
            chunk_size,
            channels,
            partial: vec![Vec::new(); channels],
        }
    }

    /// Feed interleaved `f32` samples and return resampled interleaved output.
    ///
    /// Internally buffers partial chunks until a full `chunk_size` worth of
    /// frames has accumulated, then runs the resampler.
    pub fn process_interleaved(&mut self, input: &[f32]) -> Vec<f32> {
        // De-interleave into per-channel staging buffers.
        for (i, &sample) in input.iter().enumerate() {
            let ch = i % self.channels;
            self.partial[ch].push(sample);
        }

        let frames_available = self.partial[0].len();
        if frames_available < self.chunk_size {
            return Vec::new(); // not enough data yet
        }

        // Take exactly chunk_size frames from each channel.
        let chunk: Vec<Vec<f32>> = self
            .partial
            .iter_mut()
            .map(|ch_buf| {
                let taken: Vec<f32> = ch_buf.drain(..self.chunk_size).collect();
                taken
            })
            .collect();

        match self.inner.process(&chunk, None) {
            Ok(output_channels) => interleave(&output_channels),
            Err(_) => Vec::new(),
        }
    }

    /// Discard any buffered partial input (call when seeking or changing speed).
    pub fn flush(&mut self) {
        for ch in &mut self.partial {
            ch.clear();
        }
    }
}

/// Interleave per-channel `Vec<f32>` into a single flat `Vec<f32>`.
fn interleave(channels: &[Vec<f32>]) -> Vec<f32> {
    if channels.is_empty() {
        return Vec::new();
    }
    let frames = channels[0].len();
    let ch_count = channels.len();
    let mut out = Vec::with_capacity(frames * ch_count);
    for f in 0..frames {
        for ch in channels {
            if f < ch.len() {
                out.push(ch[f]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_speed_output_is_approximately_half_input_length() {
        // 2.0x speed → ratio 0.5 → output ≈ half the frames
        let mut r = SpeedResampler::new(1, 2.0);
        // Feed enough frames to trigger at least one process call.
        let input: Vec<f32> = (0..2048).map(|i| (i as f32).sin()).collect();
        let output = r.process_interleaved(&input);
        // Expect output roughly half the input (within a generous tolerance).
        assert!(!output.is_empty(), "Expected non-empty output");
        assert!(
            output.len() < input.len(),
            "Expected output shorter than input at 2x speed, got {} vs {}",
            output.len(),
            input.len()
        );
    }

    #[test]
    fn half_speed_produces_more_output_than_input_after_warmup() {
        let mut r = SpeedResampler::new(1, 0.5);
        // The sinc resampler has startup latency; feed many chunks so the filter
        // warms up and the cumulative output/input ratio approaches 2.0.
        let chunk = vec![0.0f32; 1024];
        let mut total_output = 0usize;
        let iterations = 20;
        for _ in 0..iterations {
            total_output += r.process_interleaved(&chunk).len();
        }
        let total_input = 1024 * iterations;
        // After warmup, total output should exceed total input for 0.5× speed.
        assert!(
            total_output > total_input,
            "After warmup, output ({total_output}) should exceed input ({total_input}) at 0.5x speed"
        );
    }

    #[test]
    fn flush_clears_partial_buffer() {
        let mut r = SpeedResampler::new(1, 2.0);
        // Feed a small amount (not a full chunk).
        let partial_input: Vec<f32> = vec![0.0f32; 100];
        r.process_interleaved(&partial_input);
        assert_eq!(r.partial[0].len(), 100);
        r.flush();
        assert_eq!(r.partial[0].len(), 0);
    }

    #[test]
    fn interleave_two_channels() {
        let ch0 = vec![1.0f32, 2.0, 3.0];
        let ch1 = vec![4.0f32, 5.0, 6.0];
        let result = interleave(&[ch0, ch1]);
        assert_eq!(result, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn interleave_empty_channels_returns_empty() {
        let result = interleave(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn interleave_mismatched_channel_lengths() {
        // ch0 has 3 frames, ch1 has only 1 — exercises the `if f < ch.len()` guard
        let ch0 = vec![1.0f32, 2.0, 3.0];
        let ch1 = vec![4.0f32];
        let result = interleave(&[ch0, ch1]);
        // frame 0: ch0[0]=1.0, ch1[0]=4.0
        // frame 1: ch0[1]=2.0, ch1 skipped (f=1 >= ch1.len()=1)
        // frame 2: ch0[2]=3.0, ch1 skipped
        assert_eq!(result, vec![1.0, 4.0, 2.0, 3.0]);
    }
}
