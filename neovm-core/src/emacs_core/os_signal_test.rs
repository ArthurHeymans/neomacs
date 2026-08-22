//! Ledger 184: the OS signal dispositions this port never installed.
//!
//! GNU's `init_signals` (src/sysdep.c) ends with
//!
//! ```c
//!   #ifdef SIGUSR1
//!     add_user_signal (SIGUSR1, "sigusr1");
//!   #endif
//!   #ifdef SIGUSR2
//!     add_user_signal (SIGUSR2, "sigusr2");
//!   #endif
//! ```
//!
//! and `add_user_signal` (src/keyboard.c:8464-8483) ends with
//! `emacs_sigaction_init (&action, deliver_user_signal); sigaction (sig,
//! &action, 0);`.  Without that install the kernel's default disposition for
//! both signals is `Term`, so the editor DIES.  Measured, `-Q --batch`,
//! `kill -USR1` / `kill -USR2` at a process spinning in pure Lisp:
//!
//! ```text
//!              GNU 31.0.90                    this port, before
//!   SIGUSR2    rc=0, debug-on-quit t          rc=140, killed
//!   SIGUSR1    rc=0, nothing armed            rc=138, killed
//! ```

use super::os_signal::{
    self, HandledSignal, InstalledDisposition, PreviousDisposition, UserSignalAction,
};

/// Send SIG to this whole process, the way `kill -USR1 PID` does.
///
/// `libc::raise` targets the calling THREAD, which is a weaker question than
/// the one GNU answers: `deliver_process_signal` (src/sysdep.c:1729-1751)
/// exists precisely because "POSIX says any thread can receive a signal that
/// is associated with a process".
fn kill_self(sig: libc::c_int) {
    // SAFETY: `kill` with the caller's own pid and a valid signal number.
    let rc = unsafe { libc::kill(libc::getpid(), sig) };
    assert_eq!(rc, 0, "kill(getpid(), {sig}) failed");
}

/// The red this entry started from: with no handler installed, this test's
/// process is TERMINATED by the signal and nextest reports it killed rather
/// than failed.  It survives only because [`os_signal::install`] ran.
#[test]
fn a_user_signal_does_not_terminate_this_process_like_gnu() {
    // RED RUN (ledger 184): the install is commented out, which is the tree
    // as it stood before this entry.
    // let report = os_signal::install();
    kill_self(libc::SIGUSR1);
    kill_self(libc::SIGUSR2);

    // Reaching this line at all is the assertion GNU's `rc=0` column makes.
    let pending = os_signal::take_pending();
    assert_eq!(
        pending[HandledSignal::Sigusr1 as usize], 1,
        "SIGUSR1 was survived but not recorded: {pending:?}"
    );
    assert_eq!(
        pending[HandledSignal::Sigusr2 as usize], 1,
        "SIGUSR2 was survived but not recorded: {pending:?}"
    );
}

/// GNU installs these two and nothing else wanted them: the previous
/// disposition of both is `SIG_DFL`.
///
/// This is the control on the install itself.  GNU works around exactly one
/// library that claims a signal it also wants (`lib_child_handler`,
/// src/process.c:7654-7660, for Glib's SIGCHLD); there is no such library for
/// SIGUSR1/2 on this platform, and GNU's own reason for skipping them --
/// `#if !defined HAVE_ANDROID`, because `android_select` uses them -- does not
/// apply here.
#[test]
fn the_two_user_signals_were_unclaimed_before_this_port_installed_them() {
    let report = os_signal::install();
    for signal in HandledSignal::ALL {
        assert_eq!(
            report.previous(signal),
            PreviousDisposition::Default,
            "{signal:?} was already claimed by something else"
        );
    }
}

/// Every handled signal names its GNU install site, its Lisp name and its
/// disposition, and the table cannot be emptied.
///
/// The shape is ledger 177's `post_image_init.rs` and ledger 180's
/// `child_status.rs`: `ALL` is declared with length `COUNT`, derived from the
/// last discriminant, so a variant that is not listed is a compile error, and
/// an emptied table fails here rather than passing over nothing.
#[test]
fn every_handled_signal_carries_its_gnu_citation_and_disposition() {
    assert_eq!(
        HandledSignal::COUNT,
        2,
        "GNU's init_signals installs a user-signal handler for exactly SIGUSR1 \
         and SIGUSR2 (src/sysdep.c); a third needs its own citation"
    );
    assert_eq!(HandledSignal::ALL.len(), HandledSignal::COUNT);

    for signal in HandledSignal::ALL {
        assert!(
            signal.gnu().starts_with("src/"),
            "{signal:?} has no GNU citation"
        );
        assert!(signal.number() > 0, "{signal:?} has no signal number");
        let InstalledDisposition::UserSignal { lisp_name } = signal.disposition();
        assert!(
            !lisp_name.is_empty(),
            "{signal:?} has no `add_user_signal' NAME"
        );
    }

    assert_eq!(HandledSignal::Sigusr1.number(), libc::SIGUSR1);
    assert_eq!(HandledSignal::Sigusr2.number(), libc::SIGUSR2);
    let InstalledDisposition::UserSignal { lisp_name } = HandledSignal::Sigusr1.disposition();
    assert_eq!(lisp_name, "sigusr1");
    let InstalledDisposition::UserSignal { lisp_name } = HandledSignal::Sigusr2.disposition();
    assert_eq!(lisp_name, "sigusr2");
}

/// GNU's `handle_user_signal` decides between two arms by comparing
/// `Vdebug_on_event`'s symbol name with the signal's `add_user_signal` name
/// (src/keyboard.c:8487-8508).  Here that comparison runs on the Lisp thread
/// at the safe point, because BOTH arms touch Lisp state.
#[test]
fn debug_on_event_selects_the_debugger_arm_by_name_like_gnu() {
    // `debug-on-event' defaults to `sigusr2' (src/keyboard.c:14358-14367).
    assert_eq!(
        UserSignalAction::for_signal(HandledSignal::Sigusr2, Some("sigusr2")),
        UserSignalAction::EnterDebugger
    );
    assert_eq!(
        UserSignalAction::for_signal(HandledSignal::Sigusr1, Some("sigusr2")),
        UserSignalAction::QueueEvent {
            lisp_name: "sigusr1"
        }
    );
    // `if (SYMBOLP (Vdebug_on_event))' (:8492): a non-symbol selects no arm.
    assert_eq!(
        UserSignalAction::for_signal(HandledSignal::Sigusr2, None),
        UserSignalAction::QueueEvent {
            lisp_name: "sigusr2"
        }
    );
}

/// **Why rows 2 and 3 of ledger 184 are declined**, measured rather than
/// argued.
///
/// `(process-attributes pid)` answers `"Z"` here and `nil` in GNU, and
/// `(signal-process p 0)` answers `0` here and `-1` in GNU, because GNU's
/// SIGCHLD handler REAPS: `child_status_changed` is `waitpid`
/// (src/process.c:7741-7742), so GNU's exited child is gone from the OS within
/// microseconds and this port's is a zombie until something waits.
///
/// Both rows therefore need a reaper that runs with nobody waiting -- ledger
/// 180 §9.1's "dedicated reaper" -- and its cost is not the thread:
///
/// > `waitpid` must then have exactly ONE owner, and today every
/// > `try_wait`/`poll_child_status` path reaps on the Lisp thread, so a second
/// > reaper is a double-reap hazard across the whole file.
///
/// This is that hazard as a measurement.  `std::process::Child` owns its
/// child's reap; a second reaper that gets there first takes the status the
/// owner would have reported, and the owner is left with `ECHILD` and no exit
/// code -- which is exactly the `(exit . 7)` this port's `process-status`
/// answers from.  Five call sites reach a reap today (`process.rs:975`,
/// `:984`, `:6340`, `:6359`, and `sys::poll_child_status`), three of them
/// through `std::process::Child`.
#[test]
fn a_second_reaper_takes_the_exit_status_the_owner_would_have_reported() {
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg("exit 7")
        .spawn()
        .expect("spawn /bin/sh");
    let pid = child.id() as libc::pid_t;

    // The reaper GNU's SIGCHLD handler is, arriving first.
    let mut raw: libc::c_int = 0;
    // SAFETY: a blocking `waitpid` on a child of this process.
    let reaped = unsafe { libc::waitpid(pid, &mut raw, 0) };
    assert_eq!(reaped, pid, "the second reaper did not get the child");
    assert!(libc::WIFEXITED(raw));
    assert_eq!(
        libc::WEXITSTATUS(raw),
        7,
        "the second reaper holds the exit code"
    );

    // And the owner can no longer report it.  This is the whole reason rows 2
    // and 3 are their own entry: closing them means giving `waitpid` ONE owner
    // across every site above, not adding a thread.
    let owner_says = child.try_wait();
    assert!(
        !matches!(owner_says, Ok(Some(_))),
        "the owning std::process::Child still reported a status ({owner_says:?}); \
         if that ever becomes true the double-reap hazard is gone and ledger \
         184's rows 2 and 3 can be reconsidered"
    );
}

/// The handler's wake is GNU's, and the fd it needs exists.
///
/// GNU's `child_signal_init` (src/process.c:7580-7597) makes a nonblocking
/// pipe, `add_read_fd`s the read end, and `child_signal_notify` writes one
/// byte to the write end from signal context -- the ONE thing left in GNU's
/// handler after `emacs_perror` had to be deleted for reaching `malloc`
/// through `strerror_l` (:7630-7649).
///
/// This asserts the mechanism rather than the wiring, and the difference is
/// ledger 184's declared residual: the read end is created and the byte is
/// written, but the fd is **not yet registered with the wait poller**, so a
/// signal delivered while the Lisp thread is blocked in `poller.wait` is
/// noticed through `epoll_wait`'s EINTR (which signal(7) says is never
/// restarted) rather than through a readable fd.
#[test]
fn the_handler_has_gnus_self_pipe_and_it_carries_a_byte() {
    let report = os_signal::install();
    let read_fd = report
        .self_pipe_read_fd()
        .expect("install created GNU's self-pipe");

    // Drain anything an earlier test in this process left behind.
    let mut sink = [0u8; 64];
    loop {
        // SAFETY: a nonblocking read of the pipe's own read end.
        let n = unsafe { libc::read(read_fd, sink.as_mut_ptr().cast(), sink.len()) };
        if n <= 0 {
            break;
        }
    }

    kill_self(libc::SIGUSR1);
    let _ = os_signal::take_pending();

    // SAFETY: as above.
    let n = unsafe { libc::read(read_fd, sink.as_mut_ptr().cast(), sink.len()) };
    assert!(
        n >= 1,
        "the handler wrote no wake byte to the self-pipe (read returned {n})"
    );
}

/// The counter the handler bumps must be lock-free, or the handler is not
/// async-signal-safe no matter what it is written in.
///
/// GNU's is a plain `int` (`p->npending`, src/keyboard.c:8456) reached only
/// from the thread `deliver_process_signal` forwarded to.  This port's
/// handler runs on whatever thread the kernel picked, so the counter has to
/// be an atomic -- and an atomic that fell back to a lock would put a lock in
/// signal context, which is exactly the state this module exists to exclude.
#[test]
fn the_pending_counters_are_lock_free() {
    assert!(
        std::sync::atomic::AtomicU32::is_lock_free(),
        "the pending-signal counter would take a lock in signal context"
    );
    assert!(
        std::sync::atomic::AtomicBool::is_lock_free(),
        "the pending-signal flag would take a lock in signal context"
    );
}
