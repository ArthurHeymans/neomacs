#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LinuxDrmDevice {
    major: u32,
    minor: u32,
}

impl LinuxDrmDevice {
    pub(crate) const fn from_device_numbers(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub(crate) fn from_path(path: &std::path::Path) -> Option<Self> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};

        let metadata = std::fs::metadata(path).ok()?;
        if !metadata.file_type().is_char_device() {
            return None;
        }
        let device = metadata.rdev();
        let major = libc::major(device) as u32;
        let minor = libc::minor(device) as u32;
        (major == 226 && minor >= 128).then_some(Self { major, minor })
    }
}
