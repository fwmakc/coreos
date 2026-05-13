//! Audio input and output streams backed by cpal.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, StreamTrait};
use tracing::{info, warn};

use crate::audio::config::AudioConfig;
use crate::audio::ring_buffer::{RingBufferInner, SharedRingBuffer};

/// Callback invoked when the recording indicator state changes.
///
/// `true` = capture started, `false` = capture stopped.
pub type RecordingIndicatorFn = Box<dyn Fn(bool) + Send + Sync>;

/// Playback stream: writes samples to a ring buffer consumed by the cpal
/// output callback.
pub struct AudioOutputStream {
    _stream: cpal::Stream,
    ring: SharedRingBuffer,
    underrun_count: Arc<AtomicU64>,
    config: AudioConfig,
}

impl AudioOutputStream {
    /// Create a playback stream on the given cpal device.
    pub(crate) fn new(device: cpal::Device, config: AudioConfig) -> Result<Self, AudioStreamError> {
        let stream_config = build_stream_config(&device, &config, Direction::Output)?;
        let ring = Arc::new(RingBufferInner::new(config.ring_buffer_capacity()));
        let underrun_count = Arc::new(AtomicU64::new(0));

        let ring_clone = ring.clone();
        let underrun_clone = underrun_count.clone();
        let channels = config.channels as usize;

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    if channels == 1 {
                        let underran = ring_clone.read_or_silence(data);
                        if underran {
                            underrun_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        let frames = data.len() / channels;
                        let mut tmp = vec![0.0f32; frames];
                        let underran = ring_clone.read_or_silence(&mut tmp);
                        if underran {
                            underrun_clone.fetch_add(1, Ordering::Relaxed);
                        }
                        for (frame_idx, sample) in tmp.into_iter().enumerate() {
                            for ch in 0..channels {
                                data[frame_idx * channels + ch] = sample;
                            }
                        }
                    }
                },
                |err| {
                    warn!("audio output error: {err}");
                },
                None,
            )
            .map_err(|e| AudioStreamError::OpenFailed(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioStreamError::OpenFailed(e.to_string()))?;

        info!(
            "audio output opened: {} Hz, {} ch, buffer {} samples",
            config.sample_rate,
            config.channels,
            config.ring_buffer_capacity(),
        );

        Ok(Self {
            _stream: stream,
            ring,
            underrun_count,
            config,
        })
    }

    /// Write interleaved f32 samples to the playback buffer.
    ///
    /// Returns the number of samples actually written (may be less than `data`
    /// if the ring buffer is near capacity).
    pub fn write_samples(&self, data: &[f32]) -> usize {
        self.ring.write(data)
    }

    /// Number of buffer underruns since stream creation.
    pub fn underrun_count(&self) -> u64 {
        self.underrun_count.load(Ordering::Relaxed)
    }

    /// Sample rate of this stream.
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Channel count of this stream.
    pub fn channels(&self) -> u16 {
        self.config.channels
    }
}

/// Capture stream: cpal input callback writes to a ring buffer that the
/// consumer reads via `read_samples`.
pub struct AudioInputStream {
    stream: Option<cpal::Stream>,
    ring: SharedRingBuffer,
    overrun_count: Arc<AtomicU64>,
    config: AudioConfig,
    active: Arc<AtomicBool>,
    indicator: Option<RecordingIndicatorFn>,
}

impl AudioInputStream {
    /// Create a capture stream on the given cpal device.
    pub(crate) fn new(
        device: cpal::Device,
        config: AudioConfig,
        indicator: Option<RecordingIndicatorFn>,
    ) -> Result<Self, AudioStreamError> {
        let stream_config = build_stream_config(&device, &config, Direction::Input)?;
        let ring = Arc::new(RingBufferInner::new(config.ring_buffer_capacity()));
        let overrun_count = Arc::new(AtomicU64::new(0));
        let active = Arc::new(AtomicBool::new(true));

        let ring_clone = ring.clone();
        let overrun_clone = overrun_count.clone();
        let active_clone = active.clone();
        let channels = config.channels as usize;

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    if !active_clone.load(Ordering::Relaxed) {
                        return;
                    }
                    if channels == 1 {
                        let overwritten = ring_clone.write_overwrite(data);
                        if overwritten > 0 {
                            overrun_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        let frames = data.len() / channels;
                        let mut mono = Vec::with_capacity(frames);
                        for frame in 0..frames {
                            mono.push(data[frame * channels]);
                        }
                        let overwritten = ring_clone.write_overwrite(&mono);
                        if overwritten > 0 {
                            overrun_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                },
                |err| {
                    warn!("audio input error: {err}");
                },
                None,
            )
            .map_err(|e| AudioStreamError::OpenFailed(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioStreamError::OpenFailed(e.to_string()))?;

        if let Some(ref cb) = indicator {
            cb(true);
        }

        info!(
            "audio input opened: {} Hz, {} ch, buffer {} samples",
            config.sample_rate,
            config.channels,
            config.ring_buffer_capacity(),
        );

        Ok(Self {
            stream: Some(stream),
            ring,
            overrun_count,
            config,
            active,
            indicator,
        })
    }

    /// Read captured samples into `buffer`.
    ///
    /// Returns the number of samples actually read (may be less than the buffer
    /// length if fewer samples are available).
    pub fn read_samples(&self, buffer: &mut [f32]) -> usize {
        self.ring.read(buffer)
    }

    /// Number of buffer overruns since stream creation.
    pub fn overrun_count(&self) -> u64 {
        self.overrun_count.load(Ordering::Relaxed)
    }

    /// Sample rate of this stream.
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Channel count of this stream.
    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    /// Stop capture, zeroize buffers, and release resources.
    pub fn stop(mut self) {
        self.active.store(false, Ordering::SeqCst);
        self.stream.take();

        if let Some(ref cb) = self.indicator {
            cb(false);
        }

        if let Some(inner) = Arc::get_mut(&mut self.ring) {
            use zeroize::Zeroize;
            inner.zeroize();
            info!("audio input buffer zeroized");
        }
    }
}

/// Errors that can occur when opening an audio stream.
#[derive(Debug)]
pub enum AudioStreamError {
    /// No device supports the requested configuration.
    ConfigNotSupported(String),
    /// The stream could not be opened.
    OpenFailed(String),
}

impl std::fmt::Display for AudioStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigNotSupported(msg) => write!(f, "audio config not supported: {msg}"),
            Self::OpenFailed(msg) => write!(f, "audio stream open failed: {msg}"),
        }
    }
}

impl std::error::Error for AudioStreamError {}

#[derive(Clone, Copy)]
enum Direction {
    Input,
    Output,
}

/// Build a `cpal::StreamConfig` for the requested parameters.
///
/// Tries to find a supported config matching the requested sample rate and
/// channels. If the exact rate is not listed but falls within a supported
/// range, the OS audio subsystem typically handles resampling transparently
/// (WASAPI shared mode, PulseAudio, CoreAudio).
fn build_stream_config(
    device: &cpal::Device,
    config: &AudioConfig,
    direction: Direction,
) -> Result<cpal::StreamConfig, AudioStreamError> {
    let supported_configs: Vec<_> = match direction {
        Direction::Input => device
            .supported_input_configs()
            .map_err(|e| AudioStreamError::ConfigNotSupported(e.to_string()))?
            .filter(|c| {
                c.channels() == config.channels && c.sample_format() == cpal::SampleFormat::F32
            })
            .collect(),
        Direction::Output => device
            .supported_output_configs()
            .map_err(|e| AudioStreamError::ConfigNotSupported(e.to_string()))?
            .filter(|c| {
                c.channels() == config.channels && c.sample_format() == cpal::SampleFormat::F32
            })
            .collect(),
    };

    if supported_configs.is_empty() {
        return Err(AudioStreamError::ConfigNotSupported(format!(
            "no f32 / {} ch config on '{}'",
            config.channels,
            device.name().unwrap_or_default()
        )));
    }

    let rate = cpal::SampleRate(config.sample_rate);
    let exact = supported_configs
        .iter()
        .find(|c| c.min_sample_rate().0 <= rate.0 && c.max_sample_rate().0 >= rate.0);

    if let Some(cfg) = exact {
        return Ok(cfg.with_sample_rate(rate).config());
    }

    let fallback = &supported_configs[0];
    let fallback_rate = fallback.max_sample_rate();
    warn!(
        "requested {} Hz not in supported range [{}, {}], falling back to {} Hz (OS resampling)",
        config.sample_rate,
        fallback.min_sample_rate().0,
        fallback.max_sample_rate().0,
        fallback_rate.0,
    );
    Ok(fallback.with_sample_rate(fallback_rate).config())
}
