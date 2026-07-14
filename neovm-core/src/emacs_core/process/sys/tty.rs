//! Child PTY terminal setup (per-facility platform module, Unix only).
//!
//! `configure_child_pty_tty` puts a freshly-allocated child PTY into the mode
//! GNU Emacs uses for subprocesses (`child_setup_tty`, sysdep.c): output
//! post-processing on, NL->CR-NL off, echo off, signals on, canonical mode with
//! erase/kill disabled and EOF = C-d. The case-mapping (`IUCLC`/`OLCUC`) and
//! tab-expansion (`TAB3`) flags exist only on Linux/Android, gated inside
//! exactly as GNU's `#ifdef IUCLC` / `#ifdef OLCUC`. This is a Unix-only
//! facility (Windows has no termios/PTY here), so the whole module is
//! `#[cfg(unix)]`.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;

fn close_fd(fd: RawFd) {
    unsafe {
        libc::close(fd);
    }
}

fn set_cc(settings: &mut libc::termios, index: usize, value: u8) {
    if index < settings.c_cc.len() {
        settings.c_cc[index] = value;
    }
}

pub fn configure_child_pty_tty(tty_name: &OsStr) -> Result<(), String> {
    let path = std::ffi::CString::new(tty_name.as_bytes())
        .map_err(|_| "PTY tty name contains an interior NUL".to_string())?;
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }

    let mut settings = unsafe {
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(fd, settings.as_mut_ptr()) != 0 {
            let err = std::io::Error::last_os_error().to_string();
            close_fd(fd);
            return Err(err);
        }
        settings.assume_init()
    };

    settings.c_oflag |= libc::OPOST;
    settings.c_oflag &= !libc::ONLCR;
    settings.c_lflag &= !libc::ECHO;
    settings.c_lflag |= libc::ISIG;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        settings.c_iflag &= !libc::IUCLC;
        settings.c_oflag &= !libc::OLCUC;
    }
    settings.c_iflag &= !libc::ISTRIP;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        settings.c_oflag &= !libc::TAB3;
    }
    settings.c_cflag = (settings.c_cflag & !libc::CSIZE) | libc::CS8;
    set_cc(&mut settings, libc::VERASE, 0);
    set_cc(&mut settings, libc::VKILL, 0);
    settings.c_lflag |= libc::ICANON;
    set_cc(&mut settings, libc::VEOF, 4);

    let result = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &settings) };
    let err = if result != 0 {
        Some(std::io::Error::last_os_error().to_string())
    } else {
        None
    };
    close_fd(fd);
    err.map_or(Ok(()), Err)
}
