use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::Duration;

const EMACS_VERSION: &str = "31.0.50";

#[derive(Debug, Default)]
struct Options {
    nowait: bool,
    quiet: bool,
    suppress_output: bool,
    eval: bool,
    create_frame: bool,
    tty: bool,
    reuse_frame: bool,
    socket_name: Option<String>,
    server_file: Option<String>,
    alternate_editor: Option<String>,
    timeout: Option<Duration>,
    tramp_prefix: Option<String>,
    display: Option<String>,
    parent_id: Option<String>,
    frame_parameters: Option<String>,
    args: Vec<String>,
}

fn main() {
    let code = match run(env::args_os().collect()) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("*ERROR*: {err}");
            1
        }
    };
    process::exit(code);
}

fn run(argv: Vec<OsString>) -> Result<(), String> {
    let prog = argv
        .first()
        .and_then(|arg| arg.to_str())
        .unwrap_or("neomacsclient")
        .to_string();
    let options = parse_options(&prog, argv.into_iter().skip(1))?;

    if !(options.eval || options.create_frame || !options.args.is_empty()) {
        return Err(format!(
            "{prog}: file name or argument required\nTry '{prog} --help' for more information"
        ));
    }

    run_client(&prog, options)
}

fn parse_options(prog: &str, args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options::default();
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let mut i = 0usize;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            options.args.extend(args[i + 1..].iter().cloned());
            break;
        }
        if !arg.starts_with('-') || arg == "-" {
            options.args.push(arg.clone());
            i += 1;
            continue;
        }

        match arg.as_str() {
            "-n" | "--no-wait" => options.nowait = true,
            "-q" | "--quiet" => options.quiet = true,
            "-u" | "--suppress-output" => options.suppress_output = true,
            "-e" | "--eval" => options.eval = true,
            "-V" | "--version" => {
                println!("neomacsclient {EMACS_VERSION}");
                process::exit(0);
            }
            "-H" | "--help" => {
                print_help(prog);
                process::exit(0);
            }
            "-t" | "-nw" | "--tty" | "--nw" | "--no-window-system" => {
                options.create_frame = true;
                options.tty = true;
            }
            "-c" | "--create-frame" => options.create_frame = true,
            "-r" | "--reuse-frame" => {
                options.create_frame = true;
                options.reuse_frame = true;
            }
            _ => {
                if let Some(value) = option_value(arg, "--socket-name", "-s", &args, &mut i)? {
                    options.socket_name = Some(value);
                } else if let Some(value) = option_value(arg, "--server-file", "-f", &args, &mut i)?
                {
                    options.server_file = Some(value);
                } else if let Some(value) =
                    option_value(arg, "--alternate-editor", "-a", &args, &mut i)?
                {
                    options.alternate_editor = Some(value);
                } else if let Some(value) = option_value(arg, "--timeout", "-w", &args, &mut i)? {
                    let seconds = value
                        .parse::<u64>()
                        .map_err(|_| format!("Invalid timeout: \"{value}\""))?;
                    options.timeout = Some(Duration::from_secs(seconds));
                } else if let Some(value) = option_value(arg, "--tramp", "-T", &args, &mut i)? {
                    options.tramp_prefix = Some(value);
                } else if let Some(value) = option_value(arg, "--display", "-d", &args, &mut i)? {
                    options.display = Some(value);
                } else if let Some(value) = option_value(arg, "--parent-id", "", &args, &mut i)? {
                    options.parent_id = Some(value);
                } else if let Some(value) =
                    option_value(arg, "--frame-parameters", "-F", &args, &mut i)?
                {
                    options.frame_parameters = Some(value);
                } else {
                    return Err(format!(
                        "{prog}: unrecognized option '{arg}'\nTry '{prog} --help' for more information"
                    ));
                }
            }
        }
        i += 1;
    }

    Ok(options)
}

fn option_value(
    arg: &str,
    long: &str,
    short: &str,
    args: &[String],
    index: &mut usize,
) -> Result<Option<String>, String> {
    if !long.is_empty() {
        if arg == long {
            *index += 1;
            return args
                .get(*index)
                .cloned()
                .map(Some)
                .ok_or_else(|| format!("{long} requires an argument"));
        }
        if let Some(value) = arg.strip_prefix(&format!("{long}=")) {
            return Ok(Some(value.to_string()));
        }
    }

    if !short.is_empty() && arg == short {
        *index += 1;
        return args
            .get(*index)
            .cloned()
            .map(Some)
            .ok_or_else(|| format!("{short} requires an argument"));
    }

    Ok(None)
}

fn print_help(prog: &str) {
    println!(
        "\
Usage: {prog} [OPTIONS] FILE...
Tell a Neomacs server to visit files or evaluate forms.

Options:
  -V, --version              Print version info and return
  -H, --help                 Print this help
  -n, --no-wait              Do not wait for the server to return
  -e, --eval                 Treat FILE arguments as Elisp expressions
  -q, --quiet                Do not display success messages
  -u, --suppress-output      Do not display return values
  -s, --socket-name SOCKET   Use a local Unix server socket
-f, --server-file FILE     Use a TCP authentication file
  -a, --alternate-editor CMD Run CMD if the server is not available
  -w, --timeout SECONDS      Wait this many seconds for server replies
  -T, --tramp PREFIX         Prefix absolute file names for Tramp
"
    );
}

fn run_client(prog: &str, options: Options) -> Result<(), String> {
    if let Some(server_file) = options
        .server_file
        .clone()
        .or_else(|| env::var("EMACS_SERVER_FILE").ok())
    {
        return run_tcp_client(prog, options, &server_file);
    }

    #[cfg(unix)]
    {
        run_unix_client(prog, options)
    }

    #[cfg(not(unix))]
    {
        Err(format!(
            "{prog}: local socket mode is unsupported on this platform; use --server-file"
        ))
    }
}

#[cfg(unix)]
fn run_unix_client(prog: &str, options: Options) -> Result<(), String> {
    let socket = resolve_socket_path(&options)?;
    let mut stream = match std::os::unix::net::UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(err) => {
            return fail_or_alternate(
                prog,
                &options,
                &format!("can't connect to {}: {err}", socket.display()),
            );
        }
    };
    if let Some(timeout) = options.timeout {
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|err| format!("failed to set socket timeout: {err}"))?;
    }

    let request = build_request(&options)?;
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("failed to send request to server: {err}"))?;

    read_responses(&mut stream, &options)
}

fn run_tcp_client(prog: &str, options: Options, server_file: &str) -> Result<(), String> {
    let config = match read_tcp_server_config(server_file) {
        Ok(config) => config,
        Err(err) => return fail_or_alternate(prog, &options, &err),
    };
    let mut stream = match TcpStream::connect((&*config.host, config.port)) {
        Ok(stream) => stream,
        Err(err) => {
            return fail_or_alternate(
                prog,
                &options,
                &format!("can't connect to {}:{}: {err}", config.host, config.port),
            );
        }
    };
    if let Some(timeout) = options.timeout {
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|err| format!("failed to set socket timeout: {err}"))?;
    }

    let mut request = String::new();
    push_arg_command(&mut request, "-auth", &config.auth_key);
    request.push_str(&build_request(&options)?);
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("failed to send request to server: {err}"))?;

    read_responses(&mut stream, &options)
}

struct TcpServerConfig {
    host: String,
    port: u16,
    auth_key: String,
}

fn read_tcp_server_config(server_file: &str) -> Result<TcpServerConfig, String> {
    let path = resolve_tcp_server_file(server_file)
        .ok_or_else(|| format!("can't find server file: {server_file}"))?;
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("cannot read server file {}: {err}", path.display()))?;
    let mut lines = contents.lines();
    let endpoint = lines
        .next()
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| format!("invalid server file: {}", path.display()))?;
    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| format!("invalid server address in {}", path.display()))?;
    let port = port
        .parse::<u16>()
        .map_err(|_| format!("invalid server port in {}", path.display()))?;
    let auth_key = lines
        .next()
        .ok_or_else(|| format!("cannot read authentication info from {}", path.display()))?
        .trim_end_matches(['\r', '\n'])
        .to_string();
    if auth_key.is_empty() {
        return Err(format!(
            "empty authentication info in server file {}",
            path.display()
        ));
    }

    Ok(TcpServerConfig {
        host: host.to_string(),
        port,
        auth_key,
    })
}

fn resolve_tcp_server_file(server_file: &str) -> Option<PathBuf> {
    let path = Path::new(server_file);
    if path.is_absolute() {
        return path.exists().then(|| path.to_path_buf());
    }

    if let Some(home) = env::var_os("HOME") {
        let emacs_d = PathBuf::from(&home)
            .join(".emacs.d")
            .join("server")
            .join(server_file);
        if emacs_d.exists() {
            return Some(emacs_d);
        }
    }

    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        let xdg_path = PathBuf::from(xdg)
            .join("emacs")
            .join("server")
            .join(server_file);
        if xdg_path.exists() {
            return Some(xdg_path);
        }
    } else if let Some(home) = env::var_os("HOME") {
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("emacs")
            .join("server")
            .join(server_file);
        if config_path.exists() {
            return Some(config_path);
        }
    }

    None
}

#[cfg(unix)]
fn resolve_socket_path(options: &Options) -> Result<PathBuf, String> {
    if let Some(socket) = options
        .socket_name
        .clone()
        .or_else(|| env::var("EMACS_SOCKET_NAME").ok())
    {
        return Ok(socket_path_from_name(&socket));
    }

    Ok(socket_path_from_name("server"))
}

#[cfg(unix)]
fn socket_path_from_name(name: &str) -> PathBuf {
    let path = Path::new(name);
    if path.components().count() > 1 || path.is_absolute() {
        return path.to_path_buf();
    }

    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return Path::new(&runtime_dir).join("emacs").join(name);
    }

    let tmp = env::var_os("TMPDIR").unwrap_or_else(|| OsString::from("/tmp"));
    PathBuf::from(tmp)
        .join(format!("emacs{}", effective_uid()))
        .join(name)
}

#[cfg(unix)]
fn effective_uid() -> u32 {
    unsafe { libc::geteuid() }
}

fn build_request(options: &Options) -> Result<String, String> {
    let mut request = String::new();
    let cwd = env::current_dir().map_err(|err| format!("cannot get current directory: {err}"))?;
    let mut cwd = cwd.to_string_lossy().into_owned();
    if !cwd.ends_with('/') {
        cwd.push('/');
    }
    let display = effective_display(options);

    push_command(&mut request, "-dir");
    if let Some(prefix) = &options.tramp_prefix {
        request.push_str(&quote_argument(prefix));
    }
    request.push_str(&quote_argument(&cwd));
    request.push(' ');

    if options.nowait {
        push_flag(&mut request, "-nowait");
    }
    if !options.create_frame || options.reuse_frame {
        push_flag(&mut request, "-current-frame");
    }
    if let Some(display) = &display {
        push_arg_command(&mut request, "-display", display);
    }
    if let Some(parent_id) = &options.parent_id {
        push_arg_command(&mut request, "-parent-id", parent_id);
    }
    if let Some(frame_parameters) = &options.frame_parameters {
        push_arg_command(&mut request, "-frame-parameters", frame_parameters);
    }
    if options.create_frame && !options.tty && display.is_some() {
        push_flag(&mut request, "-window-system");
    }

    if options.eval {
        if options.args.is_empty() {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|err| format!("failed to read stdin: {err}"))?;
            for line in input.lines() {
                push_arg_command(&mut request, "-eval", line);
            }
        } else {
            for arg in &options.args {
                push_arg_command(&mut request, "-eval", arg);
            }
        }
    } else {
        for arg in &options.args {
            if is_position_arg(arg) {
                push_arg_command(&mut request, "-position", arg);
            } else {
                push_command(&mut request, "-file");
                if let Some(prefix) = &options.tramp_prefix
                    && Path::new(arg).is_absolute()
                {
                    request.push_str(&quote_argument(prefix));
                }
                request.push_str(&quote_argument(arg));
                request.push(' ');
            }
        }
    }

    request.push('\n');
    Ok(request)
}

fn effective_display(options: &Options) -> Option<String> {
    if let Some(display) = options
        .display
        .as_ref()
        .filter(|display| !display.is_empty())
    {
        return Some(display.clone());
    }
    if options.create_frame && !options.tty {
        return env::var("WAYLAND_DISPLAY")
            .ok()
            .filter(|display| !display.is_empty())
            .or_else(|| {
                env::var("DISPLAY")
                    .ok()
                    .filter(|display| !display.is_empty())
            });
    }
    None
}

fn push_flag(request: &mut String, flag: &str) {
    request.push_str(flag);
    request.push(' ');
}

fn push_command(request: &mut String, command: &str) {
    request.push_str(command);
    request.push(' ');
}

fn push_arg_command(request: &mut String, command: &str, arg: &str) {
    push_command(request, command);
    request.push_str(&quote_argument(arg));
    request.push(' ');
}

fn is_position_arg(arg: &str) -> bool {
    let Some(rest) = arg.strip_prefix('+') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == ':')
}

fn quote_argument(arg: &str) -> String {
    let mut quoted = String::with_capacity(arg.len() * 2);
    if arg.starts_with('-') {
        quoted.push('&');
    }
    for ch in arg.chars() {
        match ch {
            ' ' => quoted.push_str("&_"),
            '\n' => quoted.push_str("&n"),
            '&' => quoted.push_str("&&"),
            _ => quoted.push(ch),
        }
    }
    quoted
}

fn unquote_argument(arg: &str) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut chars = arg.chars();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            match chars.next() {
                Some('_') => out.push(' '),
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => out.push('&'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn read_responses(stream: &mut impl Read, options: &Options) -> Result<(), String> {
    let mut buffer = Vec::new();
    stream
        .read_to_end(&mut buffer)
        .map_err(|err| format!("failed to read server response: {err}"))?;
    let text = String::from_utf8_lossy(&buffer);
    let mut ok = true;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("-print ") {
            if !options.suppress_output {
                print!("{}", unquote_argument(value));
            }
        } else if let Some(value) = line.strip_prefix("-print-nonl ") {
            if !options.suppress_output {
                print!("{}", unquote_argument(value));
            }
        } else if let Some(value) = line.strip_prefix("-error ") {
            eprintln!("*ERROR*: {}", unquote_argument(value));
            ok = false;
        }
    }

    if ok {
        Ok(())
    } else {
        Err("server reported an error".to_string())
    }
}

fn fail_or_alternate(prog: &str, options: &Options, message: &str) -> Result<(), String> {
    let Some(alternate) = &options.alternate_editor else {
        return Err(format!("{prog}: {message}"));
    };
    if alternate.is_empty() {
        return Err(format!(
            "{prog}: automatic daemon startup is not implemented in neomacsclient yet"
        ));
    }

    let status = Command::new("sh")
        .arg("-c")
        .arg(alternate)
        .args(&options.args)
        .status()
        .map_err(|err| format!("{prog}: failed to run alternate editor: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{prog}: alternate editor exited with {status}"))
    }
}
