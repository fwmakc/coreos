//! Audio device enumeration and identification.

use crate::audio::config::AudioDirection;

/// Opaque device identifier. Internally stores the cpal device name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioDeviceId(pub String);

/// Describes an audio device (input or output).
#[derive(Debug, Clone)]
pub struct AudioDevice {
    /// Human-readable device name (e.g. "Speakers (Realtek HD Audio)").
    pub name: String,
    /// Whether this device is an input or output.
    pub direction: AudioDirection,
    /// Opaque identifier for use with `AudioBackend::open_*`.
    pub id: AudioDeviceId,
}

impl AudioDevice {
    /// Create a new device descriptor.
    pub fn new(name: impl Into<String>, direction: AudioDirection, id: AudioDeviceId) -> Self {
        Self {
            name: name.into(),
            direction,
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_new() {
        let dev = AudioDevice::new(
            "Speakers",
            AudioDirection::Output,
            AudioDeviceId("speakers".into()),
        );
        assert_eq!(dev.name, "Speakers");
        assert_eq!(dev.direction, AudioDirection::Output);
        assert_eq!(dev.id, AudioDeviceId("speakers".into()));
    }

    #[test]
    fn device_id_equality() {
        let a = AudioDeviceId("mic".into());
        let b = AudioDeviceId("mic".into());
        let c = AudioDeviceId("speakers".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
