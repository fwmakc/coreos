//! Audio stream configuration types and constants.

/// Default sample rate for general audio playback and capture (48 kHz).
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Sample rate for automatic speech recognition pipelines (16 kHz).
pub const ASR_SAMPLE_RATE: u32 = 16_000;

/// Default buffer duration in milliseconds per callback period.
pub const DEFAULT_BUFFER_DURATION_MS: u32 = 10;

/// Default channel count (mono).
pub const DEFAULT_CHANNELS: u16 = 1;

/// Maximum buffer duration in milliseconds.
pub const MAX_BUFFER_DURATION_MS: u32 = 100;

/// Audio stream direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDirection {
    /// Microphone / line-in.
    Input,
    /// Speakers / headphones.
    Output,
}

/// Configuration for an audio stream.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo).
    pub channels: u16,
    /// Desired buffer duration per callback in milliseconds.
    pub buffer_duration_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            channels: DEFAULT_CHANNELS,
            buffer_duration_ms: DEFAULT_BUFFER_DURATION_MS,
        }
    }
}

impl AudioConfig {
    /// Configuration tuned for speech recognition (16 kHz mono).
    pub fn asr() -> Self {
        Self {
            sample_rate: ASR_SAMPLE_RATE,
            channels: DEFAULT_CHANNELS,
            buffer_duration_ms: DEFAULT_BUFFER_DURATION_MS,
        }
    }

    /// Stereo configuration at the default sample rate.
    pub fn stereo() -> Self {
        Self {
            channels: 2,
            ..Self::default()
        }
    }

    /// Buffer size in samples (per channel) for one callback period.
    pub fn buffer_size_samples(&self) -> usize {
        ((self.sample_rate as u64 * self.buffer_duration_ms as u64) / 1000) as usize
    }

    /// Ring buffer capacity in total interleaved samples.
    ///
    /// Sized to hold approximately 100 ms of audio to absorb timing jitter.
    pub fn ring_buffer_capacity(&self) -> usize {
        let samples_per_ms = self.sample_rate as usize * self.channels as usize / 1000;
        (samples_per_ms * 100).max(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = AudioConfig::default();
        assert_eq!(c.sample_rate, 48_000);
        assert_eq!(c.channels, 1);
        assert_eq!(c.buffer_duration_ms, 10);
    }

    #[test]
    fn asr_config() {
        let c = AudioConfig::asr();
        assert_eq!(c.sample_rate, 16_000);
        assert_eq!(c.channels, 1);
    }

    #[test]
    fn stereo_config() {
        let c = AudioConfig::stereo();
        assert_eq!(c.channels, 2);
        assert_eq!(c.sample_rate, 48_000);
    }

    #[test]
    fn buffer_size_samples() {
        let c = AudioConfig::default();
        assert_eq!(c.buffer_size_samples(), 480);
    }

    #[test]
    fn ring_buffer_capacity_minimum() {
        let c = AudioConfig::asr();
        assert!(c.ring_buffer_capacity() >= 1024);
    }
}
