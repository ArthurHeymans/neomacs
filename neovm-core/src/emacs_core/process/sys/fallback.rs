//! Fallback `ChildStatusSource` backend for platforms without a native pollable
//! child-status edge yet (macOS, Windows, other Unix).
//!
//! `open` returns `None`, so the caller keeps using the explicit periodic
//! status poll. A native source -- kqueue `EVFILT_PROC` on macOS, a
//! job-object / process-handle wait on Windows -- would replace this with a
//! dedicated `macos` / `windows` backend selected from `sys::mod`'s
//! `cfg_select!`, mirroring GNU's per-platform `w32proc.c` implementation.

use crate::emacs_core::process::ProcessId;

/// Uninhabited: this backend never has a source, which is enforced by the type
/// system (`open` only ever returns `None`).
pub enum Source {}

pub fn open(_pid: u32) -> Option<Source> {
    None
}

impl Source {
    pub fn register(&self, _poller: &polling::Poller, _id: ProcessId) {
        match *self {}
    }

    pub fn unregister(&self, _poller: &polling::Poller) {
        match *self {}
    }
}
