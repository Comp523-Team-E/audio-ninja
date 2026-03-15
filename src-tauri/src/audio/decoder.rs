use std::sync::{mpsc::Receiver, Arc};

use rtrb::Producer;
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};
use symphonia::default::get_probe;

use super::control::{ControlMsg, PlaybackState};
use super::resampler::SpeedResampler;

/// Information extracted from the file before decoding begins.
pub struct DecoderInit {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: u64,
}

/// Open a media file at `path` and return decoder init info without starting
/// full playback. Used by `AudioEngine::open` to extract metadata quickly.
pub fn probe_file(path: &str) -> Result<DecoderInit, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "No supported audio track found".to_string())?;

    let params = &track.codec_params;
    let sample_rate = params.sample_rate.unwrap_or(44100);
    let channels = params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);
    let duration_ms = params
        .n_frames
        .map(|n| n * 1000 / sample_rate as u64)
        .unwrap_or(0);

    Ok(DecoderInit { sample_rate, channels, duration_ms })
}

/// Entry point for the decoder thread.
///
/// Decodes `path`, writes interleaved `f32` samples to `producer`, and
/// handles `ControlMsg` events (seek, pause, speed change, stop).
pub fn run_decoder(
    path: String,
    mut producer: Producer<f32>,
    control_rx: Receiver<ControlMsg>,
    state: Arc<PlaybackState>,
    initial_speed: f64,
) {
    if let Err(e) = decode_loop(&path, &mut producer, &control_rx, &state, initial_speed) {
        eprintln!("[decoder] Fatal error: {e}");
    }
    state.set_is_playing(false);
}

fn decode_loop(
    path: &str,
    producer: &mut Producer<f32>,
    control_rx: &Receiver<ControlMsg>,
    state: &Arc<PlaybackState>,
    initial_speed: f64,
) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "No supported audio track".to_string())?
        .clone();

    let track_id = track.id;
    let params = &track.codec_params;
    let sample_rate = params.sample_rate.unwrap_or(44100);
    let channels = params.channels.map(|c| c.count()).unwrap_or(2);
    let n_frames = params.n_frames;

    if let Some(n) = n_frames {
        state.set_duration_ms(n as f64 * 1000.0 / sample_rate as f64);
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut speed = initial_speed;
    let mut looping = false;
    let mut paused = false;
    let mut resampler: Option<SpeedResampler> = if (speed - 1.0).abs() > 1e-6 {
        Some(SpeedResampler::new(channels, speed))
    } else {
        None
    };

    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        // --- Handle control messages (non-blocking) ---
        loop {
            match control_rx.try_recv() {
                Ok(ControlMsg::Play) => {
                    paused = false;
                    state.set_is_playing(true);
                }
                Ok(ControlMsg::Pause) => {
                    paused = true;
                    state.set_is_playing(false);
                }
                Ok(ControlMsg::Seek(ms)) => {
                    let time = Time::from(ms as f64 / 1000.0);
                    let _ = probed.format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(track_id) });
                    decoder.reset();
                    if let Some(r) = &mut resampler {
                        r.flush();
                    }
                    state.set_position_ms(ms as f64);
                }
                Ok(ControlMsg::SetSpeed(new_speed)) => {
                    speed = new_speed;
                    if (speed - 1.0).abs() < 1e-6 {
                        resampler = None;
                    } else {
                        let r = SpeedResampler::new(channels, speed);
                        if let Some(old) = &mut resampler {
                            old.flush();
                        }
                        resampler = Some(r);
                    }
                }
                Ok(ControlMsg::SetLoop(enabled)) => {
                    looping = enabled;
                }
                Ok(ControlMsg::Stop) => return Ok(()),
                Err(_) => break, // no more messages
            }
        }

        // --- If paused, wait for a control message (blocking) ---
        if paused {
            match control_rx.recv() {
                Ok(ControlMsg::Play) => {
                    paused = false;
                    state.set_is_playing(true);
                }
                Ok(ControlMsg::Stop) => return Ok(()),
                Ok(msg) => {
                    // Re-handle other messages by re-queueing via a local vec would
                    // require mpsc trickery; instead handle them here directly.
                    match msg {
                        ControlMsg::Seek(ms) => {
                            let time = Time::from(ms as f64 / 1000.0);
                            let _ = probed.format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(track_id) });
                            decoder.reset();
                            if let Some(r) = &mut resampler { r.flush(); }
                            state.set_position_ms(ms as f64);
                        }
                        ControlMsg::SetSpeed(s) => {
                            speed = s;
                            resampler = if (speed - 1.0).abs() < 1e-6 { None }
                            else { Some(SpeedResampler::new(channels, speed)) };
                        }
                        ControlMsg::SetLoop(e) => looping = e,
                        _ => {}
                    }
                }
                Err(_) => return Ok(()), // channel closed
            }
            continue;
        }

        // --- Decode the next packet ---
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of stream.
                if looping {
                    let time = Time::from(0.0f64);
                    let _ = probed.format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(track_id) });
                    decoder.reset();
                    if let Some(r) = &mut resampler { r.flush(); }
                    state.set_position_ms(0.0);
                    continue;
                } else {
                    state.set_is_playing(false);
                    // Wait for a Play/Stop/Seek rather than spinning.
                    match control_rx.recv() {
                        Ok(ControlMsg::Stop) | Err(_) => return Ok(()),
                        Ok(ControlMsg::Play) => {
                            // Replay from beginning.
                            let time = Time::from(0.0f64);
                            let _ = probed.format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(track_id) });
                            decoder.reset();
                            state.set_position_ms(0.0);
                            state.set_is_playing(true);
                            paused = false;
                        }
                        Ok(ControlMsg::Seek(ms)) => {
                            let time = Time::from(ms as f64 / 1000.0);
                            let _ = probed.format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(track_id) });
                            decoder.reset();
                            state.set_position_ms(ms as f64);
                        }
                        Ok(_) => {}
                    }
                    continue;
                }
            }
            Err(e) => {
                eprintln!("[decoder] Packet error: {e}");
                continue;
            }
        };

        // Skip packets from other tracks.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet to raw samples.
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(e)) => {
                eprintln!("[decoder] Decode error (skipping): {e}");
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };

        // Convert to interleaved f32.
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;
        if sample_buf.is_none() {
            sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
        }
        let sb = sample_buf.as_mut().unwrap();
        sb.copy_interleaved_ref(decoded);
        let raw: &[f32] = sb.samples();

        // Update playback position.
        let pts_ms = packet.ts() as f64 * 1000.0 / sample_rate as f64;
        state.set_position_ms(pts_ms);

        // Optionally time-stretch via rubato.
        let samples_to_write: &[f32];
        let resampled: Vec<f32>;
        if let Some(r) = &mut resampler {
            resampled = r.process_interleaved(raw);
            samples_to_write = &resampled;
        } else {
            samples_to_write = raw;
        }

        // Write to the ring buffer, back-pressuring if full.
        let mut offset = 0;
        while offset < samples_to_write.len() {
            let free = producer.slots();
            if free == 0 {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            let to_write = (samples_to_write.len() - offset).min(free);
            let chunk = &samples_to_write[offset..offset + to_write];
            for &s in chunk {
                let _ = producer.push(s);
            }
            offset += to_write;
        }
    }
}
