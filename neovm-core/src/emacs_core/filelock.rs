//! File locking primitives.
//!
//! GNU Emacs owns these in `filelock.c`, and `buffer.c` drives them from
//! `restore-buffer-modified-p` when a file-visiting buffer changes between
//! modified and unmodified states.

use crate::emacs_core::error::LispCondition;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::error::{EvalResult, Flow, signal};
use super::fileio::{lisp_file_name_to_path_buf, resolve_filename_lisp_for_eval};
use super::value::{Value, ValueKind};
use crate::buffer::BufferId;
use crate::heap_types::LispString;

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_range_args(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), Flow> {
    if args.len() < min || args.len() > max {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn file_lock_error(context: &str, filename: &LispString, err: io::Error) -> Flow {
    signal(
        LispCondition::FileError,
        vec![
            Value::string(context),
            Value::heap_string(filename.clone()),
            Value::string(err.to_string()),
        ],
    )
}

fn current_user_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn current_host_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

fn current_lock_info_string() -> String {
    format!(
        "{}@{}.{}",
        current_user_name(),
        current_host_name(),
        std::process::id()
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedLockInfo {
    user: String,
    host: String,
    pid: u32,
}

fn parse_lock_info(contents: &str) -> Option<ParsedLockInfo> {
    let trimmed = contents.trim();
    let (user, rest) = trimmed.split_once('@')?;
    let (host, pid_and_boot) = rest.rsplit_once('.')?;
    let pid_str = pid_and_boot.split(':').next()?;
    let pid = pid_str.parse().ok()?;
    Some(ParsedLockInfo {
        user: user.to_string(),
        host: host.to_string(),
        pid,
    })
}

enum LockOwner {
    None,
    Current,
    Other(String),
}

/// GNU's `make-lock-file-name` (files.el) prepends ".#" to the non-directory
/// part of FILENAME.  Compute it byte-faithfully on the encoded path so raw
/// unibyte file names survive intact.
fn fallback_make_lock_file_name(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?;
    let mut lock_name = std::ffi::OsString::from(".#");
    lock_name.push(name);
    let mut out = PathBuf::new();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        out.push(parent);
    }
    out.push(lock_name);
    Some(out)
}

fn make_lock_file_name(
    eval: &mut super::eval::Context,
    filename: &LispString,
) -> Result<Option<PathBuf>, Flow> {
    let file = Value::heap_string(filename.clone());
    match eval.apply(Value::symbol("make-lock-file-name"), vec![file]) {
        Ok(v) if v.is_nil() => Ok(None),
        Ok(v) if v.is_string() => Ok(Some(lisp_file_name_to_path_buf(
            v.as_lisp_string()
                .expect("ValueKind::String must carry LispString payload"),
        ))),
        Ok(other) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), other],
        )),
        Err(_) => Ok(fallback_make_lock_file_name(&lisp_file_name_to_path_buf(
            filename,
        ))),
    }
}

fn read_lock_contents(lock_path: &Path) -> io::Result<String> {
    match fs::read_link(lock_path) {
        Ok(target) => Ok(target.to_string_lossy().into_owned()),
        Err(link_err) => match fs::read_to_string(lock_path) {
            Ok(contents) => Ok(contents),
            Err(_) => Err(link_err),
        },
    }
}

fn current_lock_owner(lock_path: &Path) -> Result<LockOwner, io::Error> {
    match fs::symlink_metadata(lock_path) {
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(LockOwner::None),
        Err(err) => return Err(err),
    }

    let contents = read_lock_contents(lock_path)?;
    let Some(info) = parse_lock_info(&contents) else {
        let owner = contents
            .split_once('@')
            .map(|(user, _)| user.to_string())
            .unwrap_or(contents);
        return Ok(LockOwner::Other(owner));
    };

    let ours = info.user == current_user_name()
        && info.host == current_host_name()
        && info.pid == std::process::id();
    if ours {
        Ok(LockOwner::Current)
    } else {
        Ok(LockOwner::Other(info.user))
    }
}

fn create_lock_file(lock_path: &Path, contents: &str, force: bool) -> io::Result<()> {
    if force {
        match fs::remove_file(lock_path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    #[cfg(unix)]
    {
        match std::os::unix::fs::symlink(contents, lock_path) {
            Ok(()) => return Ok(()),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::AlreadyExists
                        | io::ErrorKind::Unsupported
                        | io::ErrorKind::PermissionDenied
                ) => {}
            Err(err) => return Err(err),
        }
    }

    fs::write(lock_path, contents)
}

pub(crate) fn lock_file_resolved(
    eval: &mut super::eval::Context,
    filename: &LispString,
) -> Result<Value, Flow> {
    if !eval
        .visible_variable_value_or_nil("create-lockfiles")
        .is_truthy()
    {
        return Ok(Value::NIL);
    }

    let Some(lock_path) = make_lock_file_name(eval, filename)? else {
        return Ok(Value::NIL);
    };

    // Supersession check: before locking a file-visiting buffer,
    // verify that the file hasn't been modified on disk since we
    // last read it.  Mirrors GNU's `lock_file` (filelock.c:603-608).
    if eval
        .buffers
        .current_buffer()
        .and_then(|b| b.file_name_value().as_lisp_string())
        .is_some_and(|fname| fname.as_bytes() == filename.as_bytes())
        && eval
            .apply(Value::symbol("verify-visited-file-modtime"), vec![])
            .is_ok_and(|v| v.is_nil())
    {
        let _ = eval.apply(
            Value::symbol("userlock--ask-user-about-supersession-threat"),
            vec![Value::heap_string(filename.clone())],
        );
    }

    match current_lock_owner(&lock_path)
        .map_err(|err| file_lock_error("Testing file lock", filename, err))?
    {
        LockOwner::None => {}
        // GNU's `lock_if_free' treats our existing lock as success.  This is
        // important for `write-region': modifying a visiting buffer normally
        // acquired the lock already, and the save path locks the same file
        // again before opening it.
        LockOwner::Current => return Ok(Value::NIL),
        LockOwner::Other(owner) => {
            let attack = eval
                .apply(
                    Value::symbol("ask-user-about-lock"),
                    vec![Value::heap_string(filename.clone()), Value::string(owner)],
                )
                .unwrap_or(Value::NIL);
            if !attack.is_truthy() {
                return Ok(Value::NIL);
            }
            create_lock_file(&lock_path, &current_lock_info_string(), true)
                .map_err(|err| file_lock_error("Locking file", filename, err))?;
            return Ok(Value::NIL);
        }
    }

    create_lock_file(&lock_path, &current_lock_info_string(), false)
        .map_err(|err| file_lock_error("Locking file", filename, err))?;
    Ok(Value::NIL)
}

pub(crate) fn unlock_file_resolved(
    eval: &mut super::eval::Context,
    filename: &LispString,
) -> Result<Value, Flow> {
    let Some(lock_path) = make_lock_file_name(eval, filename)? else {
        return Ok(Value::NIL);
    };

    match current_lock_owner(&lock_path)
        .map_err(|err| file_lock_error("Unlocking file", filename, err))?
    {
        LockOwner::None | LockOwner::Other(_) => Ok(Value::NIL),
        LockOwner::Current => match fs::remove_file(&lock_path) {
            Ok(()) => Ok(Value::NIL),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Value::NIL),
            Err(err) => Err(file_lock_error("Unlocking file", filename, err)),
        },
    }
}

fn current_buffer_file_lock_target(
    eval: &super::eval::Context,
    buffer_id: BufferId,
) -> Option<LispString> {
    let root_id = eval.buffers.modified_state_root_id(buffer_id)?;
    let buffer = eval.buffers.get(root_id)?;
    let file_name = buffer.buffer_local_value("buffer-file-name")?;
    let file_truename = buffer.buffer_local_value("buffer-file-truename")?;
    match (file_name.kind(), file_truename.kind()) {
        (ValueKind::String, ValueKind::String) => file_truename.as_lisp_string().cloned(),
        _ => None,
    }
}

pub(crate) fn sync_modified_buffer_file_lock(
    eval: &mut super::eval::Context,
    buffer_id: BufferId,
    was_modified: bool,
    flag: Value,
) -> Result<(), Flow> {
    let Some(filename) = current_buffer_file_lock_target(eval, buffer_id) else {
        return Ok(());
    };

    let filename = resolve_filename_lisp_for_eval(eval, &filename);
    if !was_modified && !flag.is_nil() {
        let _ = lock_file_resolved(eval, &filename)?;
    } else if was_modified && flag.is_nil() {
        let _ = unlock_file_resolved(eval, &filename)?;
    }
    Ok(())
}

pub(crate) fn builtin_lock_file(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("lock-file", &args, 1)?;
    let filename = super::builtins::expect_lisp_string(&args[0])?;
    let filename = resolve_filename_lisp_for_eval(eval, filename);
    lock_file_resolved(eval, &filename)
}

pub(crate) fn builtin_unlock_file(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("unlock-file", &args, 1)?;
    let filename = super::builtins::expect_lisp_string(&args[0])?;
    let filename = resolve_filename_lisp_for_eval(eval, filename);
    unlock_file_resolved(eval, &filename)
}

pub(crate) fn builtin_file_locked_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("file-locked-p", &args, 1)?;
    let filename = super::builtins::expect_lisp_string(&args[0])?;
    let filename = resolve_filename_lisp_for_eval(eval, filename);
    let Some(lock_path) = make_lock_file_name(eval, &filename)? else {
        return Ok(Value::NIL);
    };

    match current_lock_owner(&lock_path)
        .map_err(|err| file_lock_error("Testing file lock", &filename, err))?
    {
        LockOwner::None => Ok(Value::NIL),
        LockOwner::Current => Ok(Value::T),
        LockOwner::Other(user) => Ok(Value::string(user)),
    }
}

pub(crate) fn builtin_lock_buffer(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_range_args("lock-buffer", &args, 0, 1)?;
    let filename = if let Some(filename) = args.first() {
        if filename.is_nil() {
            None
        } else {
            let filename = super::builtins::expect_lisp_string(filename)?;
            Some(resolve_filename_lisp_for_eval(eval, filename))
        }
    } else {
        let current = eval
            .buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        current
            .buffer_local_value("buffer-file-truename")
            .and_then(|value| match value.kind() {
                ValueKind::String => value.as_lisp_string().cloned(),
                _ => None,
            })
            .map(|filename| resolve_filename_lisp_for_eval(eval, &filename))
    };

    let modified = eval
        .buffers
        .current_buffer()
        .is_some_and(|buffer| buffer.modified_state_value().is_truthy());
    if modified && let Some(filename) = filename {
        let _ = lock_file_resolved(eval, &filename)?;
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_unlock_buffer(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("unlock-buffer", &args, 0)?;
    let Some(current) = eval.buffers.current_buffer() else {
        return Ok(Value::NIL);
    };
    if current.modified_state_value().is_truthy()
        && let Some(truename) = current.buffer_local_value("buffer-file-truename")
        && truename.is_string()
    {
        let filename = truename
            .as_lisp_string()
            .expect("ValueKind::String must carry LispString payload");
        let filename = resolve_filename_lisp_for_eval(eval, filename);
        let _ = unlock_file_resolved(eval, &filename)?;
    }
    Ok(Value::NIL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn current_lock_owner_recognizes_dangling_symlink_lockfiles() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join(".#probe");
        std::os::unix::fs::symlink(current_lock_info_string(), &lock_path)
            .expect("create lock symlink");

        assert!(matches!(
            current_lock_owner(&lock_path).expect("read lock owner"),
            LockOwner::Current
        ));
    }
}
