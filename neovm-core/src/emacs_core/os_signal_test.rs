//! Ledger 184: the OS signal dispositions this port never installed.
//!
//! GNU's `init_signals` (src/sysdep.c) ends with
//!
//! ```c
//!   #if !defined HAVE_ANDROID
//!   #ifdef SIGUSR1
//!     add_user_signal (SIGUSR1, "sigusr1");
//!   #endif
//!   #ifdef SIGUSR2
//!     add_user_signal (SIGUSR2, "sigusr2");
//!   #endif
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

#[cfg(windows)]
use super::{self as os_signal, HandledSignal};
#[cfg(unix)]
use super::{
    self as os_signal, HandledSignal, InstalledDisposition, PreviousDisposition, UserSignalAction,
};

/// Send SIG to this whole process, the way `kill -USR1 PID` does.
///
/// `libc::raise` targets the calling THREAD, which is a weaker question than
/// the one GNU answers: `deliver_process_signal` (src/sysdep.c:1729-1751)
/// exists precisely because "POSIX says any thread can receive a signal that
/// is associated with a process".
#[cfg(unix)]
fn kill_self(sig: libc::c_int) {
    // SAFETY: `kill` with the caller's own pid and a valid signal number.
    let rc = unsafe { libc::kill(libc::getpid(), sig) };
    assert_eq!(rc, 0, "kill(getpid(), {sig}) failed");
}

/// `kill -SIG PID`, then WAIT until the delivery has been recorded.
///
/// **The wait is the design's claim under test, not a workaround.**  `strace`
/// of this very test shows the `kill` issued from libtest's worker thread and
/// the signal delivered to the MAIN thread:
///
/// ```text
///   3682381 kill(3682377, SIGUSR1)  = 0
///   3682377 --- SIGUSR1 {si_signo=SIGUSR1, si_code=SI_USER, si_pid=3682377} ---
/// ```
///
/// POSIX promises delivery before `kill` returns only when the signal goes to
/// the CALLING thread, and here the kernel chose otherwise -- which is exactly
/// the case `deliver_process_signal` (src/sysdep.c:1729-1751) exists for in
/// GNU and that this port handles by making the handler correct on any thread.
/// `raise`/`pthread_kill` would take the wait away and the question with it.
#[cfg(unix)]
fn kill_self_and_wait(signal: HandledSignal) {
    let before = os_signal::pending_count(signal);
    kill_self(signal.number());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while (os_signal::pending_count(signal) == before || !os_signal::pending())
        && std::time::Instant::now() < deadline
    {
        std::thread::yield_now();
    }
}

/// The red this entry started from: with no handler installed, this test's
/// process is TERMINATED by the signal and nextest reports it killed rather
/// than failed.  It survives only because [`os_signal::install`] ran.
#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn a_user_signal_does_not_terminate_this_process_like_gnu() {
    let report = os_signal::install();
    assert!(
        report.installed_count() > 0,
        "install() reported no dispositions: {report:?}"
    );

    kill_self_and_wait(HandledSignal::Sigusr1);
    kill_self_and_wait(HandledSignal::Sigusr2);

    // Reaching this line at all is the assertion GNU's `rc=0` column makes.
    let pending = os_signal::take_pending();
    assert_eq!(
        pending[HandledSignal::Sigusr1 as usize],
        1,
        "SIGUSR1 was survived but not recorded: {pending:?}"
    );
    assert_eq!(
        pending[HandledSignal::Sigusr2 as usize],
        1,
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
#[cfg(all(unix, not(target_os = "android")))]
fn the_two_user_signals_were_unclaimed_before_this_port_installed_them() {
    let report = os_signal::install();
    for signal in [HandledSignal::Sigusr1, HandledSignal::Sigusr2] {
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
#[cfg(unix)]
fn every_handled_signal_carries_its_gnu_citation_and_disposition() {
    assert_eq!(
        HandledSignal::COUNT,
        2,
        "GNU installs a user-signal handler for exactly SIGUSR1 and SIGUSR2 \
         (src/sysdep.c, init_signals); a third needs its own citation.  GNU's \
         SIGCHLD install (catch_child_signal, src/process.c:8650) is NOT one \
         of them here: ledger 200 measured that it woke nobody and changed no \
         Lisp answer, because this port registers a pidfd per child with the \
         wait poller -- which is the WINDOWSNT case GNU's own comment at \
         :7548-7552 exempts from the facility"
    );
    assert_eq!(HandledSignal::ALL.len(), HandledSignal::COUNT);

    for signal in HandledSignal::ALL {
        assert!(
            signal.gnu().starts_with("src/"),
            "{signal:?} has no GNU citation"
        );
        assert!(signal.number() > 0, "{signal:?} has no signal number");
        // Exhaustive on purpose: a disposition added without an arm here is a
        // compile error rather than a silently unclassified signal.
        match signal.disposition() {
            InstalledDisposition::UserSignal { lisp_name } => assert!(
                !lisp_name.is_empty(),
                "{signal:?} has no `add_user_signal' NAME"
            ),
        }
    }

    assert_eq!(HandledSignal::Sigusr1.number(), libc::SIGUSR1);
    assert_eq!(HandledSignal::Sigusr2.number(), libc::SIGUSR2);
    assert_eq!(
        HandledSignal::Sigusr1.disposition(),
        InstalledDisposition::UserSignal {
            lisp_name: "sigusr1"
        }
    );
    assert_eq!(
        HandledSignal::Sigusr2.disposition(),
        InstalledDisposition::UserSignal {
            lisp_name: "sigusr2"
        }
    );
}

/// GNU leaves both user signals to `android_select` in
/// `src/sysdep.c:init_signals`, so Android has no disposition left to claim.
///
/// GNU also installs a SIGCHLD handler there (`catch_child_signal`,
/// src/process.c:8650) and this port does not, on any target -- ledger 200.
/// Android is the target where that costs the most and still costs nothing
/// measurable: its `ChildStatusSource` backend is `fallback` rather than
/// `linux` (`cfg_select!` in process/sys/mod.rs keys on `target_os = "linux"`),
/// so there is no `pidfd` there -- but the trigger woke nobody on any target
/// (the self-pipe read end is registered with no poller and
/// `polling::Poller::wait` swallows `EINTR`), and the whole-alist walk it used
/// to arm now runs unconditionally at GNU's own `status_notify` sites.
#[test]
#[cfg(target_os = "android")]
fn android_advertises_no_signal_because_gnu_reserves_both_user_signals() {
    assert!(os_signal::supported_signals().is_empty());
}

#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn posix_hosts_advertise_the_complete_logical_signal_set() {
    assert_eq!(os_signal::supported_signals(), &HandledSignal::ALL);
}

/// GNU's `handle_user_signal` decides between two arms by comparing
/// `Vdebug_on_event`'s symbol name with the signal's `add_user_signal` name
/// (src/keyboard.c:8487-8508).  Here that comparison runs on the Lisp thread
/// at the safe point, because BOTH arms touch Lisp state.
#[test]
#[cfg(all(unix, not(target_os = "android")))]
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
#[cfg(unix)]
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
/// written, but the fd is **not yet registered with the wait poller**.
///
/// **So the byte wakes nobody, and ledger 200 measured that rather than
/// assuming it.**  An earlier version of this docstring said a delivery during
/// a block is "noticed through `epoll_wait`'s EINTR (which signal(7) says is
/// never restarted)"; that is true of the syscall and false of this port's
/// call site, because `polling::Poller::wait` catches `ErrorKind::Interrupted`
/// and re-enters the wait itself (polling-3.11.0/src/lib.rs:751-764).
/// Measured: a confirmed SIGCHLD delivered 200ms into a 3s block left it
/// running the full 3.000038747s, while a real child's `pidfd` returned the
/// same block at once.  Registering this fd is what would make the byte a wake
/// (GNU `add_read_fd`s it in `child_signal_init`, src/process.c:7590-7595, and
/// clears it by hand at :5537-5543 so the notify is not starved by its own
/// wake) -- and ledger 200 §9 is the measurement of what that would then be
/// worth, which on a `pidfd` backend is nothing.
#[test]
#[cfg(all(unix, not(target_os = "android")))]
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

    kill_self_and_wait(HandledSignal::Sigusr1);
    let _ = os_signal::take_pending();

    // The read RETRIES, and that is not tidiness: `kill_self_and_wait`
    // returns as soon as the counter moves, and the counter bump is the
    // handler's FIRST operation -- GNU's own order, `p->npending++;
    // pending_signals = true;` and only then the notify (src/keyboard.c:
    // 8511-8512, src/process.c:7766-7767).  The handler may be running on
    // another thread, which is the property this whole module is built
    // around, so between the bump and the `write` there is a real window.
    // Reading once caught this suite with EAGAIN.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let (mut n, mut errno);
    loop {
        // SAFETY: as above.
        n = unsafe { libc::read(read_fd, sink.as_mut_ptr().cast(), sink.len()) };
        errno = std::io::Error::last_os_error();
        if n >= 1 || std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        n >= 1,
        "the handler wrote no wake byte to the self-pipe \
         (read fd {read_fd} returned {n}, errno {errno:?})"
    );
}

/// GNU creates both pipe ends close-on-exec and nonblocking.  Nonblocking on
/// the write end is a correctness property: a full pipe must coalesce wakes,
/// never suspend inside the async signal handler.
#[test]
#[cfg(unix)]
fn the_wake_pipe_has_gnus_descriptor_flags_on_both_ends() {
    let report = os_signal::install();
    let fds = report
        .self_pipe_fds()
        .expect("install created GNU's self-pipe");

    for fd in fds {
        // SAFETY: `fd` is an open descriptor owned by the process-lifetime
        // install report; F_GETFL/F_GETFD do not mutate it.
        let status_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        let descriptor_flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert_ne!(status_flags, -1, "F_GETFL failed for wake fd {fd}");
        assert_ne!(descriptor_flags, -1, "F_GETFD failed for wake fd {fd}");
        assert_ne!(
            status_flags & libc::O_NONBLOCK,
            0,
            "wake fd {fd} can block in signal context"
        );
        assert_ne!(
            descriptor_flags & libc::FD_CLOEXEC,
            0,
            "wake fd {fd} leaks through exec"
        );
    }
}

/// GNU's forwarding handler saves and restores errno so an asynchronous
/// delivery cannot corrupt the interrupted operation's failure state
/// (src/sysdep.c:1733-1750).  Using `raise` here deliberately exercises the
/// handler on this thread so the before/after errno slot is the same one.
#[test]
#[cfg(all(unix, not(target_os = "android")))]
fn the_handler_preserves_errno_like_gnu() {
    let _ = os_signal::install();
    let expected = errno::Errno(libc::EBUSY);
    errno::set_errno(expected);

    // SAFETY: SIGUSR1 is installed by the call above and `raise` targets this
    // thread synchronously.
    let rc = unsafe { libc::raise(HandledSignal::Sigusr1.number()) };
    assert_eq!(rc, 0, "raise(SIGUSR1) failed");
    assert_eq!(errno::errno(), expected);
    let _ = os_signal::take_pending();
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
#[cfg(unix)]
fn the_pending_counters_are_lock_free() {
    // `Atomic*::is_lock_free` is still unstable, and `target_has_atomic` is
    // the stable spelling of the same fact: rustc sets it only for widths the
    // target implements natively, and for any other width `core` would fall
    // back to a lock -- which is the state this asserts against.
    assert!(
        cfg!(target_has_atomic = "32"),
        "AtomicU32 is not native on this target, so the pending-signal counter \
         would take a lock in signal context"
    );
    assert!(
        cfg!(target_has_atomic = "8"),
        "AtomicBool is not native on this target, so the pending-signal flag \
         would take a lock in signal context"
    );
}
/// The trigger's ENGAGEMENT counter, and the previous disposition it replaced.

/// A delivered SIGCHLD does NOT reach `maybe_quit`, and the drain there says
/// so with a number.
///
/// GNU's two handlers end on two different flags, and the difference is this
/// entry's finding rather than trivia:
///
/// * `handle_user_signal` ends `p->npending++; pending_signals = true;`
///   (src/keyboard.c:8511-8512), and `pending_signals` is what `maybe_quit`
///   tests (src/lisp.h:3896-3900);
/// * `handle_child_signal` ends `if (changed) child_signal_notify ();`
///   (src/process.c:7766-7767), one `emacs_write` to a self-pipe -- and never
///   assigns `pending_signals`: `grep -n 'pending_signals = ' src/*.c` returns
///   eleven lines and **not one is in `process.c`**.
///
/// **SIGCHLD is left at its default disposition here, and GNU's own comment
/// says why this port may.**
///
/// GNU installs a handler (`catch_child_signal`, src/process.c:8645-8660) for
/// one reason, stated at :7539-7552:
///
/// ```text
///    To avoid a deadlock when receiving SIGCHLD while
///    'wait_reading_process_output' is in 'pselect', the SIGCHLD handler
///    will notify the `pselect' using a self-pipe.  ...
///
///    WINDOWSNT doesn't need this facility because its 'pselect'
///    emulation (see 'sys_select' in w32proc.c) waits on a subprocess
///    handle, which becomes signaled when the process exits, ...
/// ```
///
/// This port waits on a subprocess handle too: `sys::ChildStatusSource` opens
/// a `pidfd` per child and registers it with the wait poller.  Ledger 193
/// installed a handler anyway; ledger 200 measured what it bought and the
/// answer was nothing --
///
/// * **no wake.**  Its self-pipe read end is registered with no poller (ledger
///   184's declared residual, ledger 198 §9.2) and `polling::Poller::wait`
///   swallows `EINTR` and re-enters the wait, so a confirmed delivery left a
///   3s block running the full 3.000038747s, while a real child's `pidfd`
///   returned the same block at once.
/// * **no Lisp answer.**  The same probe run armed and disarmed in one process
///   returned the same string, including all three of ledger 198 §4's re-pinned
///   rows.
///
/// so it was removed, and the whole-alist walk it used to arm now runs
/// unconditionally where GNU runs `status_notify`.
///
/// **This pin is where a future entry that wants the handler back has to
/// come.**  It also states the thing that would have to be decided first, and
/// which ledger 193 landed without exercising: GNU chains to whatever handler
/// was already installed (`lib_child_handler`, src/process.c:7657, called at
/// :7769) because Glib may own SIGCHLD, and GNU works out whether it does in
/// `init_process_emacs` (:8705-8731).  With no handler installed there is
/// nothing to chain and the question does not arise.
#[cfg(unix)]
#[test]
fn sigchld_is_left_at_its_default_disposition_and_gnu_says_why_this_port_may() {
    let report = os_signal::install();
    assert_eq!(
        report.installed_count(),
        os_signal::supported_signals().len(),
        "install() did not install every disposition: {report:?}"
    );

    // SAFETY: querying the current disposition writes only through `old`.
    let mut old: libc::sigaction = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut old) };
    assert_eq!(rc, 0, "sigaction could not report SIGCHLD's disposition");
    assert_eq!(
        old.sa_sigaction,
        libc::SIG_DFL,
        "SIGCHLD has a disposition in this process.  If this port installed \
         it, ledger 200 is the entry that removed the last one and the \
         measurements it has to beat are in §3-§5; if a linked library \
         installed it, GNU's answer is `lib_child_handler' \
         (src/process.c:7657, 8656-8659) and this port has no chain any more."
    );

    assert!(
        !HandledSignal::ALL
            .iter()
            .any(|signal| signal.number() == libc::SIGCHLD),
        "SIGCHLD is back in HandledSignal::ALL; see this test's docstring"
    );
}

/// Windows has no POSIX SIGUSR dispositions, so this module must expose no
/// signal entries and must not create an install-time wake mechanism.
#[cfg(windows)]
#[test]
fn windows_does_not_advertise_or_install_user_signals() {
    assert!(os_signal::supported_signals().is_empty());

    let report = os_signal::install();
    assert_eq!(report.installed_count(), 0);
    assert_eq!(report.self_pipe_read_fd(), None);
    assert!(!os_signal::pending());
}
