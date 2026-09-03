//! Compile-time-selected native file-notification adapters.
//!
//! GNU Emacs builds exactly one local file-notification implementation into an
//! executable.  Keeping the same property here makes a request for one OS
//! impossible to pass to another OS adapter.

#[cfg(target_os = "linux")]
pub(super) mod linux;

// The pure snapshot-diff model is also compiled by Linux unit tests.  The
// native kqueue implementation inside the module remains macOS-only.
#[cfg(any(target_os = "macos", all(test, target_os = "linux")))]
pub(super) mod macos;

#[cfg(any(target_os = "windows", all(test, target_os = "linux")))]
pub(super) mod windows;

std::cfg_select! {
    target_os = "linux" => {
        pub(super) type Backend = linux::InotifyBackend;
    }
    target_os = "macos" => {
        pub(super) type Backend = macos::KqueueBackend;
    }
    target_os = "windows" => {
        pub(super) type Backend = windows::W32NotifyBackend;
    }
    _ => {
        compile_error!("file notification has no backend for this target");
    }
}
