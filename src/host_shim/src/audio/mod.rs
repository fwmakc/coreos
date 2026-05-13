//! Cross-platform audio subsystem.
//!
//! Provides device enumeration, playback (output), and capture (input) via
//! `cpal`. All audio data uses interleaved `f32` samples.

pub mod config;
pub mod device;
pub mod ring_buffer;
pub mod stream;

use cpal::traits::{DeviceTrait, HostTrait};

use config::{AudioConfig, AudioDirection};
use device::{AudioDevice, AudioDeviceId};
use stream::{AudioInputStream, AudioOutputStream, RecordingIndicatorFn};

/// Audio subsystem errors.
#[derive(Debug)]
pub enum AudioError {
    /// No audio device found.
    NoDeviceFound,
    /// Device is unavailable or in use.
    DeviceUnavailable(String),
    /// Stream configuration not supported by the device.
    ConfigNotSupported(String),
    /// Failed to open an audio stream.
    StreamOpenFailed(String),
    /// The audio host could not be initialized.
    HostInitFailed(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDeviceFound => write!(f, "no audio device found"),
            Self::DeviceUnavailable(msg) => write!(f, "audio device unavailable: {msg}"),
            Self::ConfigNotSupported(msg) => write!(f, "audio config not supported: {msg}"),
            Self::StreamOpenFailed(msg) => write!(f, "audio stream failed: {msg}"),
            Self::HostInitFailed(msg) => write!(f, "audio host init failed: {msg}"),
        }
    }
}

impl std::error::Error for AudioError {}

/// Cross-platform audio backend powered by `cpal`.
///
/// Enumerates devices and opens input/output streams. The `cpal` library
/// abstracts WASAPI (Windows), CoreAudio (macOS), PulseAudio/ALSA (Linux),
/// and AAudio/OpenSL ES (Android).
pub struct CpalBackend {
    host: cpal::Host,
}

impl CpalBackend {
    /// Create a new audio backend using the default cpal host.
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        Ok(Self { host })
    }

    /// Enumerate all available audio devices.
    pub fn list_devices(&self) -> Result<Vec<AudioDevice>, AudioError> {
        let mut devices = Vec::new();

        let cpal_devices = self
            .host
            .devices()
            .map_err(|e| AudioError::DeviceUnavailable(e.to_string()))?;

        for device in cpal_devices {
            let name = device.name().unwrap_or_else(|_| "<unknown>".into());

            if device.default_input_config().is_ok() {
                devices.push(AudioDevice::new(
                    name.clone(),
                    AudioDirection::Input,
                    AudioDeviceId(format!("input:{name}")),
                ));
            }
            if device.default_output_config().is_ok() {
                devices.push(AudioDevice::new(
                    name.clone(),
                    AudioDirection::Output,
                    AudioDeviceId(format!("output:{name}")),
                ));
            }
        }

        Ok(devices)
    }

    /// Get the default output device.
    pub fn default_output(&self) -> Result<AudioDevice, AudioError> {
        let device = self
            .host
            .default_output_device()
            .ok_or(AudioError::NoDeviceFound)?;
        let name = device.name().unwrap_or_else(|_| "<unknown>".into());
        Ok(AudioDevice::new(
            name,
            AudioDirection::Output,
            AudioDeviceId("default:output".into()),
        ))
    }

    /// Get the default input device.
    pub fn default_input(&self) -> Result<AudioDevice, AudioError> {
        let device = self
            .host
            .default_input_device()
            .ok_or(AudioError::NoDeviceFound)?;
        let name = device.name().unwrap_or_else(|_| "<unknown>".into());
        Ok(AudioDevice::new(
            name,
            AudioDirection::Input,
            AudioDeviceId("default:input".into()),
        ))
    }

    /// Open a playback (output) stream.
    ///
    /// If `device` is `None`, the default output device is used.
    pub fn open_output(
        &self,
        device: Option<&AudioDevice>,
        config: AudioConfig,
    ) -> Result<AudioOutputStream, AudioError> {
        let cpal_device = self.find_output_device(device)?;
        AudioOutputStream::new(cpal_device, config)
            .map_err(|e| AudioError::StreamOpenFailed(e.to_string()))
    }

    /// Open a capture (input) stream.
    ///
    /// If `device` is `None`, the default input device is used.
    /// The `indicator` callback is invoked when capture starts/stops.
    pub fn open_input(
        &self,
        device: Option<&AudioDevice>,
        config: AudioConfig,
        indicator: Option<RecordingIndicatorFn>,
    ) -> Result<AudioInputStream, AudioError> {
        let cpal_device = self.find_input_device(device)?;
        AudioInputStream::new(cpal_device, config, indicator)
            .map_err(|e| AudioError::StreamOpenFailed(e.to_string()))
    }

    fn find_output_device(&self, device: Option<&AudioDevice>) -> Result<cpal::Device, AudioError> {
        if device.is_none() {
            return self
                .host
                .default_output_device()
                .ok_or(AudioError::NoDeviceFound);
        }

        let target_name = device.unwrap().name.as_str();
        let mut cpal_devices = self
            .host
            .devices()
            .map_err(|e| AudioError::DeviceUnavailable(e.to_string()))?;

        cpal_devices
            .find(|d| d.name().map(|n| n == target_name).unwrap_or(false))
            .ok_or(AudioError::NoDeviceFound)
    }

    fn find_input_device(&self, device: Option<&AudioDevice>) -> Result<cpal::Device, AudioError> {
        if device.is_none() {
            return self
                .host
                .default_input_device()
                .ok_or(AudioError::NoDeviceFound);
        }

        let target_name = device.unwrap().name.as_str();
        let mut cpal_devices = self
            .host
            .devices()
            .map_err(|e| AudioError::DeviceUnavailable(e.to_string()))?;

        cpal_devices
            .find(|d| d.name().map(|n| n == target_name).unwrap_or(false))
            .ok_or(AudioError::NoDeviceFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_error_display() {
        assert!(!AudioError::NoDeviceFound.to_string().is_empty());
        assert!(AudioError::DeviceUnavailable("busy".into())
            .to_string()
            .contains("busy"));
        assert!(AudioError::StreamOpenFailed("oops".into())
            .to_string()
            .contains("oops"));
    }

    #[test]
    fn audio_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(AudioError::NoDeviceFound);
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn cpal_backend_new() {
        let backend = CpalBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    #[ignore = "requires audio devices (run with --ignored --test-threads=1)"]
    fn cpal_backend_default_output() {
        let backend = CpalBackend::new().unwrap();
        let result = backend.default_output();
        assert!(result.is_ok());
        let dev = result.unwrap();
        assert_eq!(dev.direction, AudioDirection::Output);
    }

    #[test]
    #[ignore = "requires audio devices (run with --ignored --test-threads=1)"]
    fn cpal_backend_default_input() {
        let backend = CpalBackend::new().unwrap();
        let result = backend.default_input();
        assert!(result.is_ok());
        let dev = result.unwrap();
        assert_eq!(dev.direction, AudioDirection::Input);
    }

    #[test]
    #[ignore = "requires audio devices (run with --ignored --test-threads=1)"]
    fn cpal_backend_list_devices() {
        let backend = CpalBackend::new().unwrap();
        let devices = backend.list_devices().unwrap();
        assert!(!devices.is_empty());
        let has_output = devices
            .iter()
            .any(|d| d.direction == AudioDirection::Output);
        let has_input = devices.iter().any(|d| d.direction == AudioDirection::Input);
        assert!(has_output);
        assert!(has_input);
    }
}
