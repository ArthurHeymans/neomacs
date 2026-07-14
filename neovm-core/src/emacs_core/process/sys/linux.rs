//! Linux `ChildStatusSource` backend.
//!
//! `pidfd_open(2)` (Linux 5.3+) returns a descriptor that becomes readable when
//! the target process changes state, so the wait poller can wake on child exit
//! with a plain readable registration -- the GNU SIGCHLD edge, but per-child and
//! pollable without installing a signal handler. `open` returns `None` on older
//! kernels, so the caller degrades to the periodic status poll.

use std::os::fd::{FromRawFd, OwnedFd, RawFd};

use crate::emacs_core::process::{ProcessId, ProcessManager};

pub struct Source {
    pidfd: OwnedFd,
}

pub fn open(pid: u32) -> Option<Source> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) };
    if fd < 0 {
        return None;
    }
    // SAFETY: `pidfd_open` returned a fresh owned descriptor.
    Some(Source {
        pidfd: unsafe { OwnedFd::from_raw_fd(fd as RawFd) },
    })
}

impl Source {
    pub fn register(&self, poller: &polling::Poller, id: ProcessId) {
        // Reuse the ProcessManager registration policy (level-triggered
        // readable, keyed by process id) so pidfd sources and process-output
        // sources are registered identically.
        let _ = ProcessManager::register_readable_source(poller, &self.pidfd, id);
    }

    pub fn unregister(&self, poller: &polling::Poller) {
        let _ = poller.delete(&self.pidfd);
    }
}
