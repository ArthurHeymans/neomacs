//! GNU-style synchronous subprocess owner, corresponding to `callproc.c`.

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::error::{EvalResult, Flow, signal};
use super::intern::resolve_sym;
use super::value::{Value, ValueKind, VecLikeType, list_to_vec};
use crate::buffer::{BufferManager, EmacsByteRange};
use crate::heap_types::LispString;

/// Build a child `Command` already isolated into its own OS session.
///
/// Every pipe-stdio subprocess neomacs launches MUST go through this (instead
/// of bare `Command::new`) so that an interactive child (e.g. `bash -i` via
/// `shell-command-switch "-ic"`) cannot disrupt the editor. Such a child does
/// terminal job-control setup; without isolation that breaks neomacs two ways,
/// both reported under issue #132:
///   * suspend — the child's SIGTSTP/SIGTTOU reach neomacs's process group and
///     stop the whole editor;
///   * hang — left as a *background* process group on neomacs's controlling
///     terminal, the child is SIGTTOU/SIGTTIN-stopped during its own job-control
///     init and never exits, wedging a synchronous `call-process` wait forever.
///
/// On Unix we therefore `setsid` the child (new session: own process group AND
/// no controlling terminal), which fixes both. On Windows we give it its own
/// process group (`CREATE_NEW_PROCESS_GROUP`). Children that genuinely need a
/// controlling terminal (M-x shell/term) are spawned via portable_pty, which
/// sets up the pty as their controlling terminal — they do not use this path.
pub(crate) fn new_child_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    isolate_child_command(&mut command);
    command
}

/// Apply the platform's "own process group" isolation to an already-built
/// command. Split out so callers that need portable_pty (which already
/// `setsid`s the child) or a pre-configured command can opt in explicitly.
pub(crate) fn isolate_child_command(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // setsid() in the child before exec puts it in a brand-new session:
        // its own process group AND no controlling terminal. Two #132 reasons:
        //   * isolation — the child's SIGTSTP/SIGTTOU stay in its own group and
        //     can never stop neomacs (the original suspend);
        //   * no controlling tty — an interactive child (`bash -i` via
        //     `shell-command-switch "-ic"`) is otherwise a *background* process
        //     group on neomacs's controlling terminal, gets SIGTTOU/SIGTTIN-
        //     stopped during its job-control init, and wedges a synchronous
        //     `call-process` wait forever (the hang). With no controlling
        //     terminal bash degrades to "no job control" and runs to completion.
        // `setsid` subsumes `setpgid(0, 0)`. PTY children that *need* a
        // controlling terminal go through portable_pty instead, not this path.
        //
        // SAFETY: the closure runs in the forked child before exec and calls
        // only the async-signal-safe `setsid`. A freshly forked process is
        // never a process-group leader, so `setsid` cannot fail with EPERM.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_PROCESS_GROUP: the child does not receive console
        // Ctrl-C/Ctrl-Break aimed at neomacs.
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = command;
    }
}

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ));
    }
    Ok(())
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ));
    }
    Ok(())
}

fn maybe_redisplay_sync_output(
    eval: &mut super::eval::Context,
    destination: &Value,
    display: bool,
) -> Result<(), Flow> {
    if display && destination_writes_to_buffer_in_state(&eval.buffers, destination)? {
        eval.redisplay();
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum OutputTarget {
    Discard,
    Buffer(Value),
    File(LispString),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StderrTarget {
    Discard,
    ToStdoutTarget,
    File,
}

#[derive(Clone, Debug)]
struct DestinationSpec {
    stdout: OutputTarget,
    stderr: StderrTarget,
    stderr_file: Option<LispString>,
    no_wait: bool,
}

fn signal_wrong_type_string(value: Value) -> Flow {
    signal("wrong-type-argument", vec![Value::symbol("stringp"), value])
}

fn callproc_owned_runtime_string(value: Value) -> String {
    // The sole caller looks up a destination buffer by name; names are
    // ASCII/Unicode, for which to_utf8_lossy is exact.
    value
        .as_lisp_string()
        .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
        .expect("ValueKind::String must carry LispString payload")
}

fn lisp_string_to_os_string(string: &LispString) -> OsString {
    #[cfg(unix)]
    {
        // Byte-faithful: a multibyte arg drops to unibyte bytes (eight-bit chars
        // become their raw byte), like Emacs `string-as-unibyte`; the unibyte
        // branch already passes raw bytes through.
        if string.is_multibyte() {
            OsString::from_vec(crate::emacs_core::emacs_char::str_as_unibyte(
                string.as_bytes(),
            ))
        } else {
            OsString::from_vec(string.as_bytes().to_vec())
        }
    }

    #[cfg(not(unix))]
    {
        OsString::from(crate::emacs_core::emacs_char::to_utf8_lossy(
            string.as_bytes(),
        ))
    }
}

fn lisp_string_to_output_path(string: &LispString) -> std::path::PathBuf {
    super::fileio::lisp_file_name_to_path_buf(string)
}

fn executable_path_exists(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        let Ok(c_path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        unsafe { libc::access(c_path.as_ptr(), libc::X_OK) == 0 }
    }

    #[cfg(not(unix))]
    {
        path.exists()
    }
}

fn call_process_lookup_error(program: &LispString) -> Flow {
    signal(
        "file-missing",
        vec![
            Value::string("Searching for program"),
            Value::string(crate::emacs_core::emacs_char::to_utf8_lossy(
                program.as_bytes(),
            )),
        ],
    )
}

fn exec_suffixes(eval: &super::eval::Context) -> Result<Vec<LispString>, Flow> {
    let value = eval.visible_variable_value_or_nil("exec-suffixes");
    if value.is_nil() {
        return Ok(vec![LispString::from_unibyte(Vec::new())]);
    }

    let suffix_values = list_to_vec(&value).ok_or_else(|| signal_wrong_type_string(value))?;
    suffix_values
        .iter()
        .map(|value| super::builtins::expect_lisp_string(value).cloned())
        .collect()
}

fn resolve_call_process_program(
    eval: &super::eval::Context,
    program: &LispString,
) -> Result<OsString, Flow> {
    let program_path = lisp_string_to_output_path(program);

    // Mirror resolve_async_process_program (process.rs:584-595) and GNU
    // openp (lread.c:2027-2028): if the program is an absolute path,
    // check it directly and return immediately — no exec-path search.
    if program_path.is_absolute() {
        if program_path.is_dir() {
            return Err(signal(
                "error",
                vec![Value::string(
                    "Specified program for new process is a directory",
                )],
            ));
        }
        if executable_path_exists(&program_path) {
            return Ok(program_path.into_os_string());
        }
        return Err(call_process_lookup_error(program));
    }

    // Relative program name — search exec-path with suffixes.
    // Mirror GNU openp's just_use_str sentinel: when exec-path is nil,
    // expand against default-directory and try the program directly.
    let exec_path = eval.visible_variable_value_or_nil("exec-path");
    let path_entries: Vec<Value> = if exec_path.is_nil() {
        // GNU openp (lread.c:1797-1808): when path is nil, use a
        // sentinel that causes filename=str to be tried directly,
        // expanded against default-directory.
        vec![Value::NIL]
    } else {
        list_to_vec(&exec_path).ok_or_else(|| call_process_lookup_error(program))?
    };
    let suffixes = exec_suffixes(eval)?;

    for entry in &path_entries {
        let Some(directory) = (match entry.kind() {
            ValueKind::Nil => subprocess_default_directory(eval),
            ValueKind::String => entry
                .as_lisp_string()
                .map(super::fileio::lisp_file_name_to_path_buf),
            _ => None,
        }) else {
            continue;
        };

        for suffix in &suffixes {
            let mut candidate = directory.join(&program_path);
            if !suffix.as_bytes().is_empty() {
                let mut os = candidate.into_os_string();
                #[cfg(unix)]
                {
                    os.push(std::ffi::OsStr::from_bytes(suffix.as_bytes()));
                }
                #[cfg(not(unix))]
                {
                    os.push(crate::emacs_core::emacs_char::to_utf8_lossy(
                        suffix.as_bytes(),
                    ));
                }
                candidate = PathBuf::from(os);
            }
            if executable_path_exists(&candidate) {
                return Ok(candidate.into_os_string());
            }
        }
    }

    Err(call_process_lookup_error(program))
}

fn fallback_subprocess_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
}

pub(super) fn subprocess_default_directory(eval: &super::eval::Context) -> Option<PathBuf> {
    let default_dir =
        super::fileio::default_directory_lisp_in_state(&eval.obarray, &[], &eval.buffers)?;
    let path = super::fileio::lisp_file_name_to_path_buf(&default_dir);
    if path.is_dir() {
        Some(path)
    } else {
        fallback_subprocess_directory()
    }
}

fn configure_subprocess_current_dir(eval: &super::eval::Context, command: &mut Command) {
    if let Some(dir) = subprocess_default_directory(eval) {
        command.current_dir(dir);
    }
}

fn is_file_keyword(value: &Value) -> bool {
    value.as_keyword_id().map_or(false, |k| {
        let n = resolve_sym(k);
        n == ":file" || n == "file"
    })
}

fn parse_file_target(items: &[Value]) -> Result<OutputTarget, Flow> {
    let file_value = items.get(1).unwrap_or(&Value::NIL);
    let file = super::builtins::expect_lisp_string(file_value)?.clone();
    Ok(OutputTarget::File(file))
}

fn parse_real_buffer_destination_in_state(
    buffers: &BufferManager,
    value: &Value,
) -> Result<(OutputTarget, bool), Flow> {
    match value.kind() {
        ValueKind::Fixnum(_) => Ok((OutputTarget::Discard, true)),
        ValueKind::Nil => Ok((OutputTarget::Discard, false)),
        ValueKind::T | ValueKind::String => Ok((OutputTarget::Buffer(*value), false)),
        ValueKind::Veclike(VecLikeType::Buffer) => {
            if buffers.get(value.as_buffer_id().unwrap()).is_none() {
                Err(signal(
                    "error",
                    vec![Value::string("Selecting deleted buffer")],
                ))
            } else {
                Ok((OutputTarget::Buffer(*value), false))
            }
        }
        ValueKind::Cons => {
            let items = list_to_vec(value).ok_or_else(|| signal_wrong_type_string(*value))?;
            let first = items.first().cloned().unwrap_or(Value::NIL);
            if is_file_keyword(&first) {
                Ok((parse_file_target(&items)?, false))
            } else {
                Err(signal_wrong_type_string(first))
            }
        }
        other => Err(signal_wrong_type_string(*value)),
    }
}

fn parse_stderr_destination(value: &Value) -> Result<(StderrTarget, Option<LispString>), Flow> {
    match value.kind() {
        ValueKind::Nil => Ok((StderrTarget::Discard, None)),
        ValueKind::T => Ok((StderrTarget::ToStdoutTarget, None)),
        ValueKind::String => Ok((
            StderrTarget::File,
            Some(
                value
                    .as_lisp_string()
                    .expect("ValueKind::String must carry LispString payload")
                    .clone(),
            ),
        )),
        other => Err(signal_wrong_type_string(*value)),
    }
}

fn parse_call_process_destination(
    buffers: &BufferManager,
    destination: &Value,
) -> Result<DestinationSpec, Flow> {
    if destination.is_cons() {
        let items =
            list_to_vec(destination).ok_or_else(|| signal_wrong_type_string(*destination))?;
        let first = items.first().cloned().unwrap_or(Value::NIL);
        if is_file_keyword(&first) {
            let stdout = parse_file_target(&items)?;
            return Ok(DestinationSpec {
                stdout,
                stderr: StderrTarget::ToStdoutTarget,
                stderr_file: None,
                no_wait: false,
            });
        }
        let second = items.get(1).cloned().unwrap_or(Value::NIL);
        let (stdout, no_wait) = parse_real_buffer_destination_in_state(buffers, &first)?;
        let (stderr, stderr_file) = parse_stderr_destination(&second)?;
        return Ok(DestinationSpec {
            stdout,
            stderr,
            stderr_file,
            no_wait,
        });
    }

    let (stdout, no_wait) = parse_real_buffer_destination_in_state(buffers, destination)?;
    let stderr = match destination.kind() {
        ValueKind::Nil | ValueKind::Fixnum(_) => StderrTarget::Discard,
        _ => StderrTarget::ToStdoutTarget,
    };
    Ok(DestinationSpec {
        stdout,
        stderr,
        stderr_file: None,
        no_wait,
    })
}

fn destination_writes_to_buffer_in_state(
    buffers: &BufferManager,
    destination: &Value,
) -> Result<bool, Flow> {
    let spec = parse_call_process_destination(buffers, destination)?;
    Ok(matches!(spec.stdout, OutputTarget::Buffer(_)))
}

fn insert_process_output_in_state(
    buffers: &mut BufferManager,
    destination: &Value,
    output: &crate::heap_types::LispString,
) -> Result<(), Flow> {
    match destination.kind() {
        ValueKind::String => {
            let name_str = callproc_owned_runtime_string(*destination);
            let id = buffers
                .find_buffer_by_name(&name_str)
                .unwrap_or_else(|| buffers.create_buffer(&name_str));
            buffers
                .insert_lisp_string_into_buffer(id, output)
                .ok_or_else(|| {
                    signal(
                        "error",
                        vec![Value::string("No such live buffer for process output")],
                    )
                })?;
            Ok(())
        }
        ValueKind::Veclike(VecLikeType::Buffer) => {
            buffers
                .insert_lisp_string_into_buffer(destination.as_buffer_id().unwrap(), output)
                .ok_or_else(|| signal("error", vec![Value::string("Selecting deleted buffer")]))?;
            Ok(())
        }
        _ => {
            if let Some(current_id) = buffers.current_buffer_id() {
                let _ = buffers.insert_lisp_string_into_buffer(current_id, output);
            }
            Ok(())
        }
    }
}

fn write_output_target_in_state(
    buffers: &mut BufferManager,
    target: &OutputTarget,
    output: &[u8],
    append: bool,
) -> Result<(), Flow> {
    match target {
        OutputTarget::Discard => Ok(()),
        OutputTarget::Buffer(destination) => {
            // Issue #131: decode to Emacs bytes + insert via the LispString path so
            // process output keeps real PUA glyphs / eight-bit bytes (the old
            // decode_bytes->insert_into_buffer storage path corrupted them).
            let text = crate::encoding::decode_bytes_to_lisp_string(output, "utf-8-unix");
            insert_process_output_in_state(buffers, destination, &text)
        }
        OutputTarget::File(path) => {
            let path_buf = lisp_string_to_output_path(path);
            if append {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path_buf)
                    .map_err(|e| {
                        super::process::signal_process_io("Writing process output", None, e)
                    })?;
                file.write_all(output).map_err(|e| {
                    super::process::signal_process_io("Writing process output", None, e)
                })
            } else {
                std::fs::write(&path_buf, output).map_err(|e| {
                    super::process::signal_process_io("Writing process output", None, e)
                })
            }
        }
    }
}

fn route_captured_output_in_state(
    buffers: &mut BufferManager,
    destination: &DestinationSpec,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), Flow> {
    write_output_target_in_state(buffers, &destination.stdout, stdout, false)?;
    match destination.stderr {
        StderrTarget::Discard => Ok(()),
        StderrTarget::ToStdoutTarget => {
            write_output_target_in_state(buffers, &destination.stdout, stderr, true)
        }
        StderrTarget::File => {
            let path = destination
                .stderr_file
                .as_ref()
                .ok_or_else(|| signal("error", vec![Value::string("Missing stderr file target")]))?
                .clone();
            write_output_target_in_state(buffers, &OutputTarget::File(path), stderr, false)
        }
    }
}

#[cfg(unix)]
fn signal_description(signal: i32) -> String {
    let ptr = unsafe { libc::strsignal(signal) };
    if ptr.is_null() {
        "unknown".to_string()
    } else {
        unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

fn call_process_status_value(status: std::process::ExitStatus) -> Value {
    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return Value::string(signal_description(signal));
    }

    Value::fixnum(status.code().unwrap_or(-1) as i64)
}

fn configure_call_process_stdin(
    command: &mut Command,
    infile: Option<&LispString>,
) -> Result<(), Flow> {
    match infile {
        None => {
            command.stdin(Stdio::null());
            Ok(())
        }
        Some(path) => {
            let file = std::fs::File::open(lisp_string_to_output_path(path)).map_err(|e| {
                super::process::signal_process_io("Opening process input file", None, e)
            })?;
            command.stdin(Stdio::from(file));
            Ok(())
        }
    }
}

fn encode_call_process_region_string_input(input: &LispString) -> Vec<u8> {
    crate::encoding::encode_lisp_string(input, "utf-8-unix")
}

fn encode_call_process_region_buffer_text(emacs_bytes: Vec<u8>) -> Vec<u8> {
    crate::emacs_core::emacs_char::str_as_unibyte(&emacs_bytes)
}

fn run_process_command_in_state(
    eval: &mut super::eval::Context,
    program: &LispString,
    infile: Option<LispString>,
    destination: &Value,
    cmd_args: &[LispString],
) -> EvalResult {
    let destination_spec = parse_call_process_destination(&mut eval.buffers, destination)?;
    let program_os = resolve_call_process_program(eval, program)?;
    let cmd_args_os = cmd_args
        .iter()
        .map(lisp_string_to_os_string)
        .collect::<Vec<OsString>>();

    if destination_spec.no_wait {
        let mut command = new_child_command(&program_os);
        command.args(&cmd_args_os).stdout(Stdio::null());
        configure_subprocess_current_dir(eval, &mut command);
        configure_call_process_stdin(&mut command, infile.as_ref())?;
        match destination_spec.stderr {
            StderrTarget::Discard | StderrTarget::ToStdoutTarget => {
                command.stderr(Stdio::null());
            }
            StderrTarget::File => {
                let path = destination_spec.stderr_file.as_ref().ok_or_else(|| {
                    signal("error", vec![Value::string("Missing stderr file target")])
                })?;
                let path_buf = lisp_string_to_output_path(path);
                let file = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&path_buf)
                    .map_err(|e| {
                        super::process::signal_process_io("Writing process output", None, e)
                    })?;
                command.stderr(Stdio::from(file));
            }
        };

        let mut child = command
            .spawn()
            .map_err(|e| super::process::signal_process_io("Searching for program", None, e))?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        return Ok(Value::NIL);
    }

    let mut command = new_child_command(&program_os);
    command
        .args(&cmd_args_os)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_subprocess_current_dir(eval, &mut command);
    configure_call_process_stdin(&mut command, infile.as_ref())?;
    let output = command
        .output()
        .map_err(|e| super::process::signal_process_io("Searching for program", None, e))?;

    route_captured_output_in_state(
        &mut eval.buffers,
        &destination_spec,
        &output.stdout,
        &output.stderr,
    )?;
    Ok(call_process_status_value(output.status))
}

fn run_process_capture_output(
    eval: &super::eval::Context,
    program: &LispString,
    cmd_args: &[LispString],
) -> Result<(i32, Vec<u8>), Flow> {
    let mut command = new_child_command(resolve_call_process_program(eval, program)?);
    command
        .args(cmd_args.iter().map(lisp_string_to_os_string))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    configure_subprocess_current_dir(eval, &mut command);
    let output = command
        .output()
        .map_err(|e| super::process::signal_process_io("Searching for program", None, e))?;
    Ok((output.status.code().unwrap_or(-1), output.stdout))
}

fn parse_optional_infile(args: &[Value], index: usize) -> Result<Option<LispString>, Flow> {
    if args.len() > index && !args[index].is_nil() {
        Ok(Some(
            super::builtins::expect_lisp_string(&args[index])?.clone(),
        ))
    } else {
        Ok(None)
    }
}

fn obarray_lisp_string_variable(
    obarray: &super::symbol::Obarray,
    name: &str,
    fallback: &str,
) -> Result<LispString, Flow> {
    let value = obarray.symbol_value(name).copied().unwrap_or(Value::NIL);
    if value.is_nil() {
        Ok(LispString::from_utf8(fallback))
    } else {
        Ok(super::builtins::expect_lisp_string(&value)?.clone())
    }
}

fn signal_process_lines_status_error(program: &LispString, status: i32) -> Flow {
    signal(
        "error",
        vec![Value::string(format!(
            "{} exited with status {status}",
            crate::emacs_core::emacs_char::to_utf8_lossy(program.as_bytes())
        ))],
    )
}

fn shell_command_fragment(value: &Value) -> Result<LispString, Flow> {
    if let Some(string) = value.as_lisp_string() {
        return Ok(string.clone());
    }

    let runtime = super::process::sequence_value_to_env_string(value)?;
    Ok(super::builtins::runtime_string_to_lisp_string(
        &runtime, true,
    ))
}

fn mapconcat_identity_lisp_strings(strings: &[LispString], separator: &[u8]) -> LispString {
    if strings.is_empty() {
        return LispString::from_unibyte(Vec::new());
    }

    let multibyte = strings.iter().any(LispString::is_multibyte);
    let separator_bytes = separator
        .len()
        .saturating_mul(strings.len().saturating_sub(1));
    let total_len = strings.iter().map(LispString::sbytes).sum::<usize>() + separator_bytes;
    let mut bytes = Vec::with_capacity(total_len);

    for (index, string) in strings.iter().enumerate() {
        if index != 0 {
            bytes.extend_from_slice(separator);
        }
        bytes.extend_from_slice(string.as_bytes());
    }

    if multibyte {
        LispString::from_emacs_bytes(bytes)
    } else {
        LispString::from_unibyte(bytes)
    }
}

fn shell_command_with_legacy_args(command: &Value, args: &[Value]) -> Result<LispString, Flow> {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_command_fragment(command)?);
    for arg in args {
        parts.push(shell_command_fragment(arg)?);
    }
    Ok(mapconcat_identity_lisp_strings(&parts, b" "))
}

fn builtin_call_process_impl(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("call-process", &args, 1)?;
    let program = super::builtins::expect_lisp_string(&args[0])?.clone();
    let infile = parse_optional_infile(&args, 1)?;
    let destination = args.get(2).unwrap_or(&Value::NIL);
    let cmd_args = if args.len() > 4 {
        super::process::parse_lisp_string_args_strict(&args[4..])?
    } else {
        Vec::new()
    };
    run_process_command_in_state(eval, &program, infile, destination, &cmd_args)
}

fn builtin_call_process_region_impl(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("call-process-region", &args, 3)?;
    let program = super::builtins::expect_lisp_string(&args[2])?.clone();
    let program_os = resolve_call_process_program(eval, &program)?;
    let subprocess_dir = subprocess_default_directory(eval);
    let buffers = &mut eval.buffers;

    let delete = args.len() > 3 && args[3].is_truthy();
    let destination = if args.len() > 4 {
        &args[4]
    } else {
        &Value::NIL
    };
    let destination_spec = parse_call_process_destination(buffers, destination)?;

    let cmd_args = if args.len() > 6 {
        super::process::parse_lisp_string_args_strict(&args[6..])?
    } else {
        Vec::new()
    };

    let region_text = match args[0].kind() {
        ValueKind::Nil => {
            let (text, maybe_delete_range) = {
                let buf = buffers
                    .current_buffer()
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
                (
                    encode_call_process_region_buffer_text(
                        buf.buffer_substring_bytes_range(buf.full_emacs_byte_range()),
                    ),
                    buf.full_emacs_byte_range(),
                )
            };
            if delete {
                let current_id = buffers
                    .current_buffer_id()
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
                let _ = buffers.delete_buffer_emacs_byte_range(current_id, maybe_delete_range);
            }
            text
        }
        ValueKind::String => {
            if delete {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("integer-or-marker-p"), args[0]],
                ));
            }
            encode_call_process_region_string_input(
                args[0]
                    .as_lisp_string()
                    .expect("ValueKind::String must carry LispString payload"),
            )
        }
        _ => {
            let region_args =
                super::position::LispRegionArgs::from_values(&*buffers, args[0], args[1])?;
            let (text, region) = {
                let buf = buffers
                    .current_buffer()
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
                let region = super::process::checked_region_bytes(buf, region_args)?;
                (
                    encode_call_process_region_buffer_text(
                        buf.buffer_substring_bytes_range(region),
                    ),
                    region,
                )
            };

            if delete {
                let current_id = buffers
                    .current_buffer_id()
                    .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
                let _ = buffers.delete_buffer_emacs_byte_range(current_id, region);
            }

            text
        }
    };

    if destination_spec.no_wait {
        let mut command = new_child_command(&program_os);
        if let Some(dir) = &subprocess_dir {
            command.current_dir(dir);
        }
        command
            .args(cmd_args.iter().map(lisp_string_to_os_string))
            .stdin(Stdio::piped())
            .stdout(Stdio::null());
        match destination_spec.stderr {
            StderrTarget::Discard | StderrTarget::ToStdoutTarget => {
                command.stderr(Stdio::null());
            }
            StderrTarget::File => {
                let path = destination_spec.stderr_file.as_ref().ok_or_else(|| {
                    signal("error", vec![Value::string("Missing stderr file target")])
                })?;
                let file = OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(lisp_string_to_output_path(path))
                    .map_err(|e| {
                        super::process::signal_process_io("Writing process output", None, e)
                    })?;
                command.stderr(Stdio::from(file));
            }
        };

        let mut child = command
            .spawn()
            .map_err(|e| super::process::signal_process_io("Searching for program", None, e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&region_text);
        }

        std::thread::spawn(move || {
            let _ = child.wait();
        });

        return Ok(Value::NIL);
    }

    let mut command = new_child_command(&program_os);
    if let Some(dir) = &subprocess_dir {
        command.current_dir(dir);
    }
    let mut child = command
        .args(cmd_args.iter().map(lisp_string_to_os_string))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| super::process::signal_process_io("Searching for program", None, e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&region_text);
    }

    let output = child
        .wait_with_output()
        .map_err(|e| super::process::signal_process_io("Process error", None, e))?;

    route_captured_output_in_state(buffers, &destination_spec, &output.stdout, &output.stderr)?;
    Ok(call_process_status_value(output.status))
}

pub(crate) fn builtin_call_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let destination = args.get(2).copied().unwrap_or(Value::NIL);
    let display = args.get(3).is_some_and(|v| v.is_truthy());
    let result = builtin_call_process_impl(eval, args)?;
    maybe_redisplay_sync_output(eval, &destination, display)?;
    Ok(result)
}

pub(crate) fn builtin_call_process_shell_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("call-process-shell-command", &args, 1)?;
    let infile = parse_optional_infile(&args, 1)?;
    let destination = args.get(2).copied().unwrap_or(Value::NIL);
    let display = args.get(3).is_some_and(|v| v.is_truthy());
    let shell_command = shell_command_with_legacy_args(&args[0], args.get(4..).unwrap_or(&[]))?;
    let shell_program = obarray_lisp_string_variable(eval.obarray(), "shell-file-name", "sh")?;
    let shell_switch = obarray_lisp_string_variable(eval.obarray(), "shell-command-switch", "-c")?;
    let shell_args = vec![shell_switch, shell_command];
    let result =
        run_process_command_in_state(eval, &shell_program, infile, &destination, &shell_args)?;
    maybe_redisplay_sync_output(eval, &destination, display)?;
    Ok(result)
}

pub(crate) fn builtin_process_file(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-file", &args, 1)?;
    let program = super::builtins::expect_lisp_string(&args[0])?.clone();
    let infile = parse_optional_infile(&args, 1)?;
    let destination = args.get(2).copied().unwrap_or(Value::NIL);
    let display = args.get(3).is_some_and(|v| v.is_truthy());
    let cmd_args = if args.len() > 4 {
        super::process::parse_lisp_string_args_strict(&args[4..])?
    } else {
        Vec::new()
    };
    let result = run_process_command_in_state(eval, &program, infile, &destination, &cmd_args)?;
    maybe_redisplay_sync_output(eval, &destination, display)?;
    Ok(result)
}

pub(crate) fn builtin_process_file_shell_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-file-shell-command", &args, 1)?;
    let infile = parse_optional_infile(&args, 1)?;
    let destination = args.get(2).copied().unwrap_or(Value::NIL);
    let display = args.get(3).is_some_and(|v| v.is_truthy());
    let shell_command = shell_command_with_legacy_args(&args[0], args.get(4..).unwrap_or(&[]))?;
    let shell_program = obarray_lisp_string_variable(eval.obarray(), "shell-file-name", "sh")?;
    let shell_switch = obarray_lisp_string_variable(eval.obarray(), "shell-command-switch", "-c")?;
    let shell_args = vec![shell_switch, shell_command];
    let result =
        run_process_command_in_state(eval, &shell_program, infile, &destination, &shell_args)?;
    maybe_redisplay_sync_output(eval, &destination, display)?;
    Ok(result)
}

pub(crate) fn builtin_process_lines(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-lines", &args, 1)?;
    let program = super::builtins::expect_lisp_string(&args[0])?.clone();
    let cmd_args = super::process::parse_lisp_string_args_strict(&args[1..])?;
    let (status, stdout) = run_process_capture_output(eval, &program, &cmd_args)?;
    if status != 0 {
        return Err(signal_process_lines_status_error(&program, status));
    }
    Ok(parse_output_lines(&stdout))
}

pub(crate) fn builtin_process_lines_ignore_status(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-lines-ignore-status", &args, 1)?;
    let program = super::builtins::expect_lisp_string(&args[0])?.clone();
    let cmd_args = super::process::parse_lisp_string_args_strict(&args[1..])?;
    let (_, stdout) = run_process_capture_output(eval, &program, &cmd_args)?;
    Ok(parse_output_lines(&stdout))
}

pub(crate) fn builtin_process_lines_handling_status(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-lines-handling-status", &args, 2)?;
    let program = super::builtins::expect_lisp_string(&args[0])?.clone();
    let status_handler = args[1];
    let cmd_args = super::process::parse_lisp_string_args_strict(&args[2..])?;
    let (status, stdout) = run_process_capture_output(eval, &program, &cmd_args)?;
    let lines = parse_output_lines(&stdout);

    if !status_handler.is_nil() {
        let _ = eval.apply(status_handler, vec![Value::fixnum(status as i64)])?;
    } else if status != 0 {
        return Err(signal_process_lines_status_error(&program, status));
    }

    Ok(lines)
}

pub(crate) fn builtin_call_process_region(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("call-process-region", &args, 3)?;
    let destination = args.get(4).copied().unwrap_or(Value::NIL);
    let display = args.get(5).is_some_and(|v| v.is_truthy());
    let result = builtin_call_process_region_impl(eval, args)?;
    maybe_redisplay_sync_output(eval, &destination, display)?;
    Ok(result)
}

fn parse_output_lines(stdout: &[u8]) -> Value {
    let mut text = String::from_utf8_lossy(stdout).into_owned();
    if text.ends_with('\n') {
        text.pop();
    }
    if text.is_empty() {
        Value::NIL
    } else {
        Value::list(text.split('\n').map(Value::string).collect())
    }
}

#[cfg(test)]
#[path = "callproc_raw_bytes_test.rs"]
mod raw_bytes_tests;

#[cfg(all(test, unix))]
mod child_isolation_tests {
    use super::new_child_command;
    use std::process::Stdio;

    /// Regression test for issue #132: every spawned pipe-stdio child must live
    /// in its own *session* (`setsid`) — its own process group AND no
    /// controlling terminal. The process group stops a child's SIGTSTP/SIGTTOU
    /// from suspending the editor (the suspend); the lack of a controlling
    /// terminal stops an interactive `bash -i` from being SIGTTOU/SIGTTIN-
    /// stopped as a background process group, which would wedge a synchronous
    /// `call-process` forever (the hang).
    #[test]
    fn child_runs_in_its_own_session() {
        let parent_pgid = unsafe { libc::getpgrp() };
        let parent_sid = unsafe { libc::getsid(0) };
        let mut child = new_child_command("sh")
            .arg("-c")
            .arg("sleep 1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn child");
        let pid = child.id() as libc::pid_t;
        // Read the child's process group + session while it is still alive.
        let child_pgid = unsafe { libc::getpgid(pid) };
        let child_sid = unsafe { libc::getsid(pid) };
        let _ = child.kill();
        let _ = child.wait();

        assert!(child_pgid > 0, "getpgid failed for live child");
        assert_ne!(
            child_pgid, parent_pgid,
            "child shares the editor's process group; its SIGTSTP/SIGTTOU could suspend neomacs (#132 suspend)"
        );
        assert_eq!(
            child_pgid, pid,
            "isolated child should lead its own process group"
        );
        // setsid makes the child a session leader (sid == pid) in a session
        // distinct from the editor's, so it has no controlling terminal and an
        // interactive shell cannot get SIGTTOU/SIGTTIN-stopped (#132 hang).
        assert!(child_sid > 0, "getsid failed for live child");
        assert_eq!(
            child_sid, pid,
            "isolated child should lead its own session (setsid)"
        );
        assert_ne!(
            child_sid, parent_sid,
            "child shares the editor's session/controlling terminal (#132 hang)"
        );
    }
}
