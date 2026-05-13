//! Removable media detection.
//!
//! Enumerates USB drives, memory cards, and other removable storage on
//! the host OS. Each platform uses its own detection mechanism:
//!
//! - **Windows:** `GetLogicalDriveStrings` + `GetDriveType`
//! - **macOS:** `/Volumes` directory scanning
//! - **Linux:** `/media`, `/mnt`, and `udev` parsing
//! - **Android:** `StorageManager` via JNI (placeholder)

use std::path::PathBuf;

/// A detected storage device.
#[derive(Debug, Clone)]
pub struct StorageDevice {
    /// Human-readable label (volume name or mount point basename).
    pub label: String,
    /// Mount point on the host filesystem.
    pub mount_point: PathBuf,
    /// Kind of device.
    pub device_type: StorageDeviceType,
    /// Filesystem type (e.g. "NTFS", "exFAT", "vfat").
    pub file_system: String,
    /// Total capacity in bytes.
    pub total_bytes: u64,
    /// Available (free) bytes.
    pub available_bytes: u64,
    /// Whether the device is read-only.
    pub is_readonly: bool,
}

/// Classification of storage devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDeviceType {
    /// USB flash drive or external HDD/SSD.
    Usb,
    /// Optical disc (CD/DVD).
    CdRom,
    /// Internal fixed disk.
    Fixed,
    /// Network-mounted share.
    Network,
}

impl std::fmt::Display for StorageDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usb => write!(f, "USB"),
            Self::CdRom => write!(f, "CD-ROM"),
            Self::Fixed => write!(f, "Fixed"),
            Self::Network => write!(f, "Network"),
        }
    }
}

/// List all removable storage devices currently attached to the host.
pub fn list_removable() -> Vec<StorageDevice> {
    platform::detect_removable()
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    pub fn detect_removable() -> Vec<StorageDevice> {
        let mut devices = Vec::new();
        let drive_bits = unsafe { winapi_get_logical_drives() };
        if drive_bits == 0 {
            return devices;
        }

        for i in 0..26u32 {
            if drive_bits & (1 << i) == 0 {
                continue;
            }
            let letter = b"A"[0] + i as u8;
            let root = format!("{}:\\", letter as char);
            let root_path = PathBuf::from(&root);

            let drive_type = unsafe { winapi_get_drive_type(&root) };
            if drive_type != 2 {
                continue;
            }

            devices.push(build_device(&root_path, StorageDeviceType::Usb));
        }

        devices
    }

    fn build_device(mount: &PathBuf, dtype: StorageDeviceType) -> StorageDevice {
        let label = mount
            .to_str()
            .unwrap_or("Removable")
            .trim_end_matches('\\')
            .to_string();

        let (total, available, readonly) = disk_stats(mount);

        StorageDevice {
            label,
            mount_point: mount.clone(),
            device_type: dtype,
            file_system: String::new(),
            total_bytes: total,
            available_bytes: available,
            is_readonly: readonly,
        }
    }

    fn disk_stats(mount: &PathBuf) -> (u64, u64, bool) {
        match std::fs::metadata(mount) {
            Ok(meta) => {
                let readonly = meta.permissions().readonly();
                (0, 0, readonly)
            }
            Err(_) => (0, 0, true),
        }
    }

    unsafe fn winapi_get_logical_drives() -> u32 {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetLogicalDrives() -> u32;
        }
        GetLogicalDrives()
    }

    unsafe fn winapi_get_drive_type(root: &str) -> u32 {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
        }
        let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
        GetDriveTypeW(wide.as_ptr())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    const VOLUMES_DIR: &str = "/Volumes";

    pub fn detect_removable() -> Vec<StorageDevice> {
        let volumes = match std::fs::read_dir(VOLUMES_DIR) {
            Ok(rd) => rd,
            Err(_) => return Vec::new(),
        };

        let mut devices = Vec::new();
        for entry in volumes.flatten() {
            let mount = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "Macintosh HD" {
                continue;
            }
            devices.push(StorageDevice {
                label: name,
                mount_point: mount.clone(),
                device_type: StorageDeviceType::Usb,
                file_system: String::new(),
                total_bytes: 0,
                available_bytes: 0,
                is_readonly: false,
            });
        }
        devices
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    const MEDIA_DIR: &str = "/media";
    const MNT_DIR: &str = "/mnt";

    pub fn detect_removable() -> Vec<StorageDevice> {
        let mut devices = Vec::new();
        scan_dir(MEDIA_DIR, &mut devices);
        scan_dir(MNT_DIR, &mut devices);
        devices
    }

    fn scan_dir(dir: &str, devices: &mut Vec<StorageDevice>) {
        let rd = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        for entry in rd.flatten() {
            let mount = entry.path();
            if !mount.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            devices.push(StorageDevice {
                label: name,
                mount_point: mount,
                device_type: StorageDeviceType::Usb,
                file_system: String::new(),
                total_bytes: 0,
                available_bytes: 0,
                is_readonly: false,
            });
        }
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::*;

    pub fn detect_removable() -> Vec<StorageDevice> {
        Vec::new()
    }
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "android"
)))]
mod platform {
    use super::*;

    pub fn detect_removable() -> Vec<StorageDevice> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_device_type_display() {
        assert_eq!(StorageDeviceType::Usb.to_string(), "USB");
        assert_eq!(StorageDeviceType::CdRom.to_string(), "CD-ROM");
        assert_eq!(StorageDeviceType::Fixed.to_string(), "Fixed");
        assert_eq!(StorageDeviceType::Network.to_string(), "Network");
    }

    #[test]
    fn list_removable_does_not_panic() {
        let devices = list_removable();
        for dev in &devices {
            assert!(!dev.label.is_empty());
            assert!(dev.mount_point.is_absolute() || dev.mount_point.starts_with("/"));
        }
    }

    #[test]
    fn storage_device_fields() {
        let dev = StorageDevice {
            label: "USB-DRIVE".into(),
            mount_point: PathBuf::from("/mnt/usb"),
            device_type: StorageDeviceType::Usb,
            file_system: "exFAT".into(),
            total_bytes: 32 * 1024 * 1024 * 1024,
            available_bytes: 16 * 1024 * 1024 * 1024,
            is_readonly: false,
        };
        assert_eq!(dev.label, "USB-DRIVE");
        assert_eq!(dev.device_type, StorageDeviceType::Usb);
        assert!(!dev.is_readonly);
    }
}
