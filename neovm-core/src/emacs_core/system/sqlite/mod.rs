//! SQLite capability facade matching GNU `src/sqlite.c`.
//!
//! The public surface exists in every build.  The backend and its operational
//! subrs are compiled only with the `sqlite` feature; feature-disabled builds
//! retain GNU's `sqlitep` and `sqlite-available-p` capability probes.

#[cfg(feature = "sqlite")]
mod backend;
#[cfg(not(feature = "sqlite"))]
mod disabled;
mod subrs;

#[cfg(feature = "sqlite")]
pub(crate) use backend::*;
#[cfg(not(feature = "sqlite"))]
pub(crate) use disabled::*;
pub(crate) use subrs::register_subrs;

pub(super) fn reset_sqlite_thread_locals() {
    #[cfg(feature = "sqlite")]
    backend::reset_thread_locals();
}

#[cfg(all(test, not(feature = "sqlite")))]
#[path = "tests/disabled.rs"]
mod tests;
