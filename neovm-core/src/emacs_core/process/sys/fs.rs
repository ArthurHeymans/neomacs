//! Filesystem access checks (per-facility platform module).
//!
//! `path_is_executable` answers "can this process execute this file", which is
//! an effective-access question: on Unix it must go through `access(path,
//! X_OK)` so the kernel honors effective uid/gid and ACLs -- a raw
//! permission-bit inspection would get this wrong for setuid Emacs or ACL'd
//! files, and it is exactly what GNU's `file-executable-p` does. Off Unix there
//! is no such probe here, so it degrades to an existence check.

use std::path::Path;

/// Whether `path` is executable by the current process.
#[cfg(unix)]
pub fn path_is_executable(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c_path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: `c_path` is a valid C string; `access` takes no other pointers.
    unsafe { libc::access(c_path.as_ptr(), libc::X_OK) == 0 }
}

#[cfg(not(unix))]
pub fn path_is_executable(path: &Path) -> bool {
    path.exists()
}
