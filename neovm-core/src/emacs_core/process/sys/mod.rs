//! Platform abstraction layer (PAL) for OS-specific subprocess facilities.
//!
//! This is the neomacs analogue of GNU Emacs's `sysdep.c` / `w32proc.c` split
//! and of Rust std's `sys::pal`: the portable process logic in the parent
//! `process` module calls this small, stable interface, and exactly one backend
//! module implements it for the target platform. The platform choice is made
//! ONCE here, at the module boundary -- no `cfg` leaks into the caller.
//!
//! Extension points: add a `macos` backend (kqueue `EVFILT_PROC`) or a
//! `windows` backend (job-object / process-handle wait) and route to it from
//! the `cfg_select!` below, exactly as GNU adds an `#elif`/`w32proc.c` path.
//! The `fallback` backend is the portable poll-only path used until then.

use crate::emacs_core::process::{HostInterfaceEntry, ProcessId};

mod signals;
pub use signals::signal_name_number;

mod process_status;
pub use process_status::process_is_alive;

#[cfg(unix)]
mod tty;
#[cfg(unix)]
pub use tty::configure_child_pty_tty;

cfg_select! {
    target_os = "linux" => {
        mod linux;
        use self::linux as backend;
    }
    _ => {
        mod fallback;
        use self::fallback as backend;
    }
}

/// A poll-able "this child changed state (exited)" edge for one subprocess.
///
/// GNU Emacs waits for subprocess I/O and child status in a single primitive:
/// Unix multiplexes a SIGCHLD self-pipe into `wait_reading_process_output`,
/// while w32 waits on subprocess handles alongside its pipe-reader events.
/// Linux `pidfd`s give the same per-child edge as a plain readable descriptor,
/// so the existing wait poller wakes on child exit exactly as it wakes on
/// process output. Platforms without a native source yet degrade to the
/// explicit periodic status poll (see the `fallback` backend).
pub struct ChildStatusSource(backend::Source);

impl ChildStatusSource {
    /// Open a status source for `pid`, or `None` when this platform has no
    /// native pollable source (the caller then keeps using the periodic poll).
    pub fn open(pid: u32) -> Option<Self> {
        backend::open(pid).map(Self)
    }

    /// Register this source's readable edge with the wait `poller`. A no-op
    /// when there is no poller or no native source.
    pub fn register_with_poller(&self, poller: Option<&polling::Poller>, id: ProcessId) {
        if let Some(poller) = poller {
            self.0.register(poller, id);
        }
    }

    /// Remove this source from the wait `poller`.
    pub fn unregister_from_poller(&self, poller: &polling::Poller) {
        self.0.unregister(poller);
    }
}

/// Snapshot of the host's network interfaces for `network-interface-list` /
/// `network-interface-info`. Linux uses native `getifaddrs`+ioctls; other
/// platforms use the portable `network_interface` crate (see the backends).
pub fn interface_snapshot() -> Option<Vec<HostInterfaceEntry>> {
    backend::interface_snapshot()
}
