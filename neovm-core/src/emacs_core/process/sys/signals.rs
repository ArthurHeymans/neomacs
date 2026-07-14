//! Signal-name → signal-number mapping for `signal-process` / `kill`.
//!
//! A per-facility platform module. The POSIX table is shared by every Unix
//! target (Linux and macOS alike), with a few Linux/Android-only signals
//! (`SIGPOLL`, `SIGPWR`, and the `SIGRTMIN..SIGRTMAX` realtime range) gated
//! inside; non-Unix (Windows) has only the synthetic `"EXIT"` (0). This mirrors
//! GNU Emacs's per-symbol `#ifdef` in `process.c`'s signal handling.
//!
//! Note the split here is Unix-vs-Windows, NOT linux-vs-rest like the
//! `child_status`/`interface` backends -- so this facility lives in its own
//! module with internal cfg, the way `std::sys` organizes per facility.

#[cfg(unix)]
pub fn signal_name_number(name: &str) -> Option<i32> {
    let name = name
        .strip_prefix("SIG")
        .or_else(|| name.strip_prefix("sig"))
        .unwrap_or(name);
    let name = name.to_ascii_uppercase();
    match name.as_str() {
        "EXIT" => Some(0),
        "HUP" => Some(libc::SIGHUP),
        "INT" => Some(libc::SIGINT),
        "QUIT" => Some(libc::SIGQUIT),
        "ILL" => Some(libc::SIGILL),
        "TRAP" => Some(libc::SIGTRAP),
        "ABRT" | "IOT" => Some(libc::SIGABRT),
        "BUS" => Some(libc::SIGBUS),
        "FPE" => Some(libc::SIGFPE),
        "KILL" => Some(libc::SIGKILL),
        "USR1" => Some(libc::SIGUSR1),
        "SEGV" => Some(libc::SIGSEGV),
        "USR2" => Some(libc::SIGUSR2),
        "PIPE" => Some(libc::SIGPIPE),
        "ALRM" => Some(libc::SIGALRM),
        "TERM" => Some(libc::SIGTERM),
        "CHLD" | "CLD" => Some(libc::SIGCHLD),
        "CONT" => Some(libc::SIGCONT),
        "STOP" => Some(libc::SIGSTOP),
        "TSTP" => Some(libc::SIGTSTP),
        "TTIN" => Some(libc::SIGTTIN),
        "TTOU" => Some(libc::SIGTTOU),
        "URG" => Some(libc::SIGURG),
        "XCPU" => Some(libc::SIGXCPU),
        "XFSZ" => Some(libc::SIGXFSZ),
        "VTALRM" => Some(libc::SIGVTALRM),
        "PROF" => Some(libc::SIGPROF),
        "WINCH" => Some(libc::SIGWINCH),
        #[cfg(any(target_os = "linux", target_os = "android"))]
        "POLL" | "IO" => Some(libc::SIGPOLL),
        #[cfg(any(target_os = "linux", target_os = "android"))]
        "PWR" => Some(libc::SIGPWR),
        "SYS" => Some(libc::SIGSYS),
        _ => realtime_signal_name_number(&name),
    }
}

#[cfg(unix)]
fn realtime_signal_name_number(name: &str) -> Option<i32> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let min = libc::SIGRTMIN();
        let max = libc::SIGRTMAX();
        if name == "RTMIN" {
            return Some(min);
        }
        if name == "RTMAX" {
            return Some(max);
        }
        if let Some(offset) = name
            .strip_prefix("RTMIN+")
            .and_then(|value| value.parse::<i32>().ok())
        {
            let signal = min + offset;
            return (signal <= max).then_some(signal);
        }
        if let Some(offset) = name
            .strip_prefix("RTMAX-")
            .and_then(|value| value.parse::<i32>().ok())
        {
            let signal = max - offset;
            return (signal >= min).then_some(signal);
        }
    }
    None
}

#[cfg(not(unix))]
pub fn signal_name_number(name: &str) -> Option<i32> {
    match name
        .strip_prefix("SIG")
        .or_else(|| name.strip_prefix("sig"))
        .unwrap_or(name)
        .to_ascii_uppercase()
        .as_str()
    {
        "EXIT" => Some(0),
        _ => None,
    }
}
