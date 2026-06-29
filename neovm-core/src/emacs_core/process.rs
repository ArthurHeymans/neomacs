//! Process/subprocess management for the Elisp VM.
//!
//! Provides process abstractions: creating, killing, querying, and
//! communicating with subprocesses.  `start-process` creates a tracked
//! record; `call-process` and `shell-command-to-string` run real OS
//! commands via `std::process::Command`.
//!
//! ## Network processes
//!
//! `make-network-process` supports TCP streams, UDP datagrams, and Unix local
//! sockets on platforms that provide them. Network sockets are registered with
//! the process I/O poller so `accept-process-output` and `poll_process_output`
//! wake on incoming data.  Unix child pipes are also poller-backed; Windows
//! child pipes are deliberately kept on the synchronous service pass because
//! Windows waitable pipe support needs a separate reader-thread/event design,
//! as in GNU Emacs' w32 process layer.
//!
//! **TLS**: `gnutls-boot` upgrades a network process through the Neomacs TLS
//! facade. The `TcpStream` is moved into the backend-neutral
//! `Process.tls_stream`. Read/write/send automatically use the TLS layer when
//! present. Mozilla root certificates are used by the default rustls backend
//! for verification.

use num_enum::IntoPrimitive;
use socket2::{Domain, Protocol, SockAddr, SockRef, Socket, Type};
use std::collections::HashMap;
#[cfg(not(target_os = "windows"))]
use std::ffi::CStr;
#[cfg(unix)]
use std::ffi::CString;
use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{Read as IoRead, Write as IoWrite};
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
    UdpSocket,
};
#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(unix)]
use std::os::unix::net::{SocketAddr as UnixSocketAddr, UnixDatagram, UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use strum::{EnumString, IntoStaticStr};

use super::tls::{
    RustlsBackend, TlsBackendError, TlsClientBackend, TlsStream, gnutls_close_notify_result_value,
    gnutls_peer_status_to_value, parse_gnutls_boot_parameters,
};
use super::wait::ProcessOutputWaitOutcome;

/// OS socket owned by a network process.
///
/// GNU Emacs keeps the concrete socket type in the process record
/// (`socktype`, `is_server`, and fd slots).  Keep the Rust side explicit as
/// well, so listener-only operations and stream I/O cannot be confused.
#[derive(Debug)]
pub enum NetworkSocket {
    TcpStream(TcpStream),
    TcpListener(TcpListener),
    UdpSocket(UdpSocket),
    #[cfg(unix)]
    SeqpacketStream(Socket),
    #[cfg(unix)]
    SeqpacketListener(Socket),
    #[cfg(unix)]
    UnixStream(UnixStream),
    #[cfg(unix)]
    UnixListener(UnixListener),
    #[cfg(unix)]
    UnixDatagram(UnixDatagram),
}

/// GNU-compatible GnuTLS process initialization stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(i64)]
pub(crate) enum GnutlsInitStage {
    Empty = 0,
    CredAlloc = 1,
    Files = 2,
    Callbacks = 3,
    Init = 4,
    Priority = 5,
    CredSet = 6,
    TransportPointersSet = 7,
    HandshakeTried = 8,
    Ready = 9,
}

impl NetworkSocket {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::TcpStream(_) => "tcp-stream",
            Self::TcpListener(_) => "tcp-listener",
            Self::UdpSocket(_) => "udp-socket",
            #[cfg(unix)]
            Self::SeqpacketStream(_) => "seqpacket-stream",
            #[cfg(unix)]
            Self::SeqpacketListener(_) => "seqpacket-listener",
            #[cfg(unix)]
            Self::UnixStream(_) => "unix-stream",
            #[cfg(unix)]
            Self::UnixListener(_) => "unix-listener",
            #[cfg(unix)]
            Self::UnixDatagram(_) => "unix-datagram",
        }
    }

    fn register_readable(&self, poller: &polling::Poller, id: ProcessId) -> Result<(), String> {
        match self {
            Self::TcpStream(stream) => ProcessManager::register_readable_source(poller, stream, id),
            Self::TcpListener(listener) => {
                ProcessManager::register_readable_source(poller, listener, id)
            }
            Self::UdpSocket(socket) => ProcessManager::register_readable_source(poller, socket, id),
            #[cfg(unix)]
            Self::SeqpacketStream(socket) => {
                ProcessManager::register_readable_source(poller, socket, id)
            }
            #[cfg(unix)]
            Self::SeqpacketListener(socket) => {
                ProcessManager::register_readable_source(poller, socket, id)
            }
            #[cfg(unix)]
            Self::UnixStream(stream) => {
                ProcessManager::register_readable_source(poller, stream, id)
            }
            #[cfg(unix)]
            Self::UnixListener(listener) => {
                ProcessManager::register_readable_source(poller, listener, id)
            }
            #[cfg(unix)]
            Self::UnixDatagram(socket) => {
                ProcessManager::register_readable_source(poller, socket, id)
            }
        }
    }

    fn register_writable(&self, poller: &polling::Poller, id: ProcessId) -> Result<(), String> {
        match self {
            Self::TcpStream(stream) => ProcessManager::register_writable_source(poller, stream, id),
            Self::TcpListener(_) => Err("Listener sockets are not writable process sources".into()),
            Self::UdpSocket(socket) => ProcessManager::register_writable_source(poller, socket, id),
            #[cfg(unix)]
            Self::SeqpacketStream(socket) => {
                ProcessManager::register_writable_source(poller, socket, id)
            }
            #[cfg(unix)]
            Self::SeqpacketListener(_) => {
                Err("Listener sockets are not writable process sources".into())
            }
            #[cfg(unix)]
            Self::UnixStream(stream) => {
                ProcessManager::register_writable_source(poller, stream, id)
            }
            #[cfg(unix)]
            Self::UnixListener(_) => {
                Err("Listener sockets are not writable process sources".into())
            }
            #[cfg(unix)]
            Self::UnixDatagram(socket) => {
                ProcessManager::register_writable_source(poller, socket, id)
            }
        }
    }

    fn unregister_readable(&self, poller: &polling::Poller) {
        match self {
            Self::TcpStream(stream) => {
                let _ = poller.delete(stream);
            }
            Self::TcpListener(listener) => {
                let _ = poller.delete(listener);
            }
            Self::UdpSocket(socket) => {
                let _ = poller.delete(socket);
            }
            #[cfg(unix)]
            Self::SeqpacketStream(socket) => {
                let _ = poller.delete(socket);
            }
            #[cfg(unix)]
            Self::SeqpacketListener(socket) => {
                let _ = poller.delete(socket);
            }
            #[cfg(unix)]
            Self::UnixStream(stream) => {
                let _ = poller.delete(stream);
            }
            #[cfg(unix)]
            Self::UnixListener(listener) => {
                let _ = poller.delete(listener);
            }
            #[cfg(unix)]
            Self::UnixDatagram(socket) => {
                let _ = poller.delete(socket);
            }
        }
    }

    fn read_stream_output(&mut self, buf: &mut [u8]) -> Option<std::io::Result<usize>> {
        match self {
            Self::TcpStream(stream) => Some(stream.read(buf)),
            Self::TcpListener(_) => None,
            Self::UdpSocket(_) => None,
            #[cfg(unix)]
            Self::SeqpacketStream(socket) => Some(socket.read(buf)),
            #[cfg(unix)]
            Self::SeqpacketListener(_) => None,
            #[cfg(unix)]
            Self::UnixStream(stream) => Some(stream.read(buf)),
            #[cfg(unix)]
            Self::UnixListener(_) => None,
            #[cfg(unix)]
            Self::UnixDatagram(_) => None,
        }
    }

    fn write_stream_input(&mut self, bytes: &[u8]) -> Option<std::io::Result<()>> {
        match self {
            Self::TcpStream(stream) => Some(stream.write_all(bytes).and_then(|_| stream.flush())),
            Self::TcpListener(_) => None,
            Self::UdpSocket(_) => None,
            #[cfg(unix)]
            Self::SeqpacketStream(socket) => {
                Some(socket.write_all(bytes).and_then(|_| socket.flush()))
            }
            #[cfg(unix)]
            Self::SeqpacketListener(_) => None,
            #[cfg(unix)]
            Self::UnixStream(stream) => Some(stream.write_all(bytes).and_then(|_| stream.flush())),
            #[cfg(unix)]
            Self::UnixListener(_) => None,
            #[cfg(unix)]
            Self::UnixDatagram(_) => None,
        }
    }

    fn shutdown_write(&self) -> Option<std::io::Result<()>> {
        match self {
            Self::TcpStream(stream) => Some(stream.shutdown(Shutdown::Write)),
            Self::TcpListener(_) => None,
            Self::UdpSocket(_) => None,
            #[cfg(unix)]
            Self::SeqpacketStream(socket) => Some(socket.shutdown(Shutdown::Write)),
            #[cfg(unix)]
            Self::SeqpacketListener(_) => None,
            #[cfg(unix)]
            Self::UnixStream(stream) => Some(stream.shutdown(Shutdown::Write)),
            #[cfg(unix)]
            Self::UnixListener(_) => None,
            #[cfg(unix)]
            Self::UnixDatagram(_) => None,
        }
    }

    fn take_pending_connect_error(&self) -> Option<std::io::Result<Option<std::io::Error>>> {
        match self {
            Self::TcpStream(stream) => Some(stream.take_error()),
            #[cfg(unix)]
            Self::UnixStream(stream) => Some(stream.take_error()),
            _ => None,
        }
    }
}

use super::error::{EvalResult, Flow, signal};
use super::intern::{intern, resolve_sym};
use super::threads::ThreadManager;
use super::value::{
    StringTextPropertyRun, Value, ValueKind, VecLikeType, equal_value, list_to_vec, next_float_id,
};
use crate::buffer::{
    BufferId, BufferManager, EmacsByteLen, EmacsBytePos, EmacsByteRange, LispCharPos1,
};
use crate::gc_trace::GcTrace;
use crate::heap_types::LispString;
use crate::window::FrameManager;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Unique identifier for a process.
pub type ProcessId = u64;

thread_local! {
    /// Name registry keyed by process id, used by the printer to render
    /// `#<process NAME>` without threading a `ProcessManager` into the
    /// stateless print path (mirrors the terminal handle registry).  A process
    /// name never changes after creation and survives `delete-process`, so
    /// entries are inserted once and never removed.
    static PROCESS_NAME_REGISTRY: std::cell::RefCell<rustc_hash::FxHashMap<ProcessId, String>> =
        std::cell::RefCell::new(rustc_hash::FxHashMap::default());
}

/// Record a process id -> name mapping for the printer.
pub(crate) fn register_process_print_name(id: ProcessId, name: &str) {
    PROCESS_NAME_REGISTRY.with(|slot| {
        slot.borrow_mut().insert(id, name.to_string());
    });
}

/// Look up a process name for printing `#<process NAME>`.
///
/// Returns `None` only for an id that was never registered (it then prints as a
/// bare `#<process>` fallback).
pub(crate) fn print_process_handle(value: &Value) -> Option<String> {
    let id = value.as_process_id()?;
    let name = PROCESS_NAME_REGISTRY.with(|slot| slot.borrow().get(&id).cloned());
    Some(match name {
        Some(name) => format!("#<process {name}>"),
        None => "#<process>".to_string(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessOutputWaitTiming {
    Poll,
    For(Duration),
    Forever,
}

impl ProcessOutputWaitTiming {
    pub(crate) fn is_poll(self) -> bool {
        matches!(self, Self::Poll)
    }

    pub(crate) fn is_finite(self) -> bool {
        matches!(self, Self::For(_))
    }

    pub(crate) fn is_forever(self) -> bool {
        matches!(self, Self::Forever)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcessOutputWaitRequest {
    timing: ProcessOutputWaitTiming,
    target_process: Option<ProcessId>,
    just_this_one: bool,
    allow_timers: bool,
}

impl ProcessOutputWaitRequest {
    pub(crate) fn new(
        timing: ProcessOutputWaitTiming,
        target_process: Option<ProcessId>,
        just_this_one: bool,
        allow_timers: bool,
    ) -> Self {
        Self {
            timing,
            target_process,
            just_this_one,
            allow_timers,
        }
    }

    pub(crate) fn timing(self) -> ProcessOutputWaitTiming {
        self.timing
    }

    pub(crate) fn target_process(self) -> Option<ProcessId> {
        self.target_process
    }

    pub(crate) fn just_this_one(self) -> bool {
        self.just_this_one
    }

    pub(crate) fn allow_timers(self) -> bool {
        self.allow_timers
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessOutputServiceRequest {
    None,
    Any { target: Option<ProcessId> },
    TargetOnly(ProcessId),
}

impl ProcessOutputServiceRequest {
    pub(crate) fn none() -> Self {
        Self::None
    }

    pub(crate) fn any(target: Option<ProcessId>) -> Self {
        Self::Any { target }
    }

    pub(crate) fn target_only(target: ProcessId) -> Self {
        Self::TargetOnly(target)
    }

    fn target_process(self) -> Option<ProcessId> {
        match self {
            Self::None | Self::Any { target: None } => None,
            Self::Any {
                target: Some(target),
            }
            | Self::TargetOnly(target) => Some(target),
        }
    }

    fn live_processes(self, live_processes: Vec<ProcessId>) -> Vec<ProcessId> {
        match self {
            Self::None => Vec::new(),
            Self::Any { .. } => live_processes,
            Self::TargetOnly(target) => vec![target],
        }
    }

    fn ready_processes(self, ready_processes: Vec<ProcessId>) -> Vec<ProcessId> {
        match self {
            Self::None => Vec::new(),
            Self::Any { .. } => dedupe_process_ids(ready_processes),
            Self::TargetOnly(target) => ready_processes
                .contains(&target)
                .then_some(target)
                .into_iter()
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ProcessOutputServiceActivity {
    #[default]
    None,
    Any,
    Target,
}

impl ProcessOutputServiceActivity {
    fn record(self, target: bool) -> Self {
        if target {
            Self::Target
        } else if matches!(self, Self::Target) {
            Self::Target
        } else {
            Self::Any
        }
    }

    fn any(self) -> bool {
        matches!(self, Self::Any | Self::Target)
    }

    fn target(self) -> bool {
        matches!(self, Self::Target)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProcessOutputServiceOutcome {
    activity: ProcessOutputServiceActivity,
}

impl ProcessOutputServiceOutcome {
    pub(crate) fn record_activity(&mut self, target: bool) {
        self.activity = self.activity.record(target);
    }

    pub(crate) fn absorb(&mut self, other: Self) {
        if other.has_target_process_activity() {
            self.record_activity(true);
        } else if other.has_any_process_activity() {
            self.record_activity(false);
        }
    }

    pub(crate) fn has_any_process_activity(self) -> bool {
        self.activity.any()
    }

    pub(crate) fn has_target_process_activity(self) -> bool {
        self.activity.target()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProcessWaitEvents {
    input_wakeup: bool,
    ready_processes: Vec<ProcessId>,
    writable_processes: Vec<ProcessId>,
}

impl ProcessWaitEvents {
    pub(crate) fn from_sources(input_wakeup: bool, ready_processes: Vec<ProcessId>) -> Self {
        Self::from_sources_with_writable(input_wakeup, ready_processes, Vec::new())
    }

    pub(crate) fn from_sources_with_writable(
        input_wakeup: bool,
        ready_processes: Vec<ProcessId>,
        writable_processes: Vec<ProcessId>,
    ) -> Self {
        Self {
            input_wakeup,
            ready_processes,
            writable_processes,
        }
    }

    pub(crate) fn input_wakeup() -> Self {
        Self::from_sources(true, Vec::new())
    }

    pub(crate) fn ready_processes(processes: Vec<ProcessId>) -> Self {
        Self {
            input_wakeup: false,
            ready_processes: processes,
            writable_processes: Vec::new(),
        }
    }

    pub(crate) fn writable_processes(processes: Vec<ProcessId>) -> Self {
        Self::from_sources_with_writable(false, Vec::new(), processes)
    }

    pub(crate) fn has_input_wakeup(&self) -> bool {
        self.input_wakeup
    }

    pub(crate) fn has_ready_processes(&self) -> bool {
        !self.ready_processes.is_empty()
    }

    pub(crate) fn has_writable_processes(&self) -> bool {
        !self.writable_processes.is_empty()
    }

    pub(crate) fn has_ready_process(&self, process: ProcessId) -> bool {
        self.ready_processes.contains(&process)
    }

    pub(crate) fn has_writable_process(&self, process: ProcessId) -> bool {
        self.writable_processes.contains(&process)
    }

    pub(crate) fn is_empty(&self) -> bool {
        !self.input_wakeup && self.ready_processes.is_empty() && self.writable_processes.is_empty()
    }

    pub(crate) fn into_ready_processes(self) -> Vec<ProcessId> {
        self.ready_processes
    }

    pub(crate) fn ready_processes_ref(&self) -> &[ProcessId] {
        &self.ready_processes
    }

    pub(crate) fn writable_processes_ref(&self) -> &[ProcessId] {
        &self.writable_processes
    }
}

const INPUT_WAKEUP_EVENT_KEY: usize = 0;

/// Process family used by compatibility helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum ProcessKind {
    Real,
    Network,
    Pipe,
    Serial,
}

impl ProcessKind {
    fn name(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum ProcessStatusSymbol {
    Run,
    Stop,
    Exit,
    Signal,
    Open,
    Listen,
    Closed,
    Connect,
    Failed,
}

impl ProcessStatusSymbol {
    fn from_symbol_value(value: Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    fn from_status_value(status: Value) -> Option<Self> {
        Self::from_symbol_value(process_status_symbol_value(status))
    }

    fn value(self) -> Value {
        Value::symbol(self.name())
    }

    fn name(self) -> &'static str {
        self.into()
    }

    #[cfg(test)]
    fn gnu_public_domain() -> [Self; 9] {
        [
            Self::Run,
            Self::Stop,
            Self::Exit,
            Self::Signal,
            Self::Open,
            Self::Listen,
            Self::Closed,
            Self::Connect,
            Self::Failed,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum ProcessTtyStream {
    Stdin,
    Stdout,
    Stderr,
}

impl ProcessTtyStream {
    fn from_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    fn name(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(prefix = ":", serialize_all = "kebab-case")]
enum ProcessKeyword {
    Name,
    Type,
    Buffer,
    Command,
    Coding,
    Noquery,
    Stop,
    ConnectionType,
    Filter,
    Sentinel,
    Stderr,
    FileHandler,
    Host,
    Service,
    Family,
    Local,
    Remote,
    Server,
    Nowait,
    Log,
    TlsParameters,
    UseExternalSocket,
    Plist,
    Bindtodevice,
    Broadcast,
    Dontroute,
    Keepalive,
    Linger,
    Oobinline,
    Priority,
    Reuseaddr,
    Nodelay,
    Port,
    Speed,
    Process,
    Bytesize,
    Stopbits,
    Parity,
    Flowcontrol,
    Summary,
}

impl ProcessKeyword {
    fn keyword(self) -> &'static str {
        self.into()
    }

    fn value(self) -> Value {
        Value::keyword(self.keyword())
    }

    fn from_keyword(name: &str) -> Option<Self> {
        name.strip_prefix(':')?.parse().ok()
    }

    fn from_value(value: &Value) -> Option<Self> {
        Self::from_keyword(keyword_name(value)?)
    }
}

/// A tracked process record.
pub struct Process {
    pub id: ProcessId,
    pub name: Value,
    pub command: Value,
    pub executable: Option<LispString>,
    pub kind: ProcessKind,
    pub proc_type: Value,
    pub status: Value,
    pub buffer: Value,
    pub childp: Value,
    /// Queued input entries `(STRING . (OFFSET . LENGTH))`, matching GNU's `write_queue`.
    pub write_queue: Value,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Query-on-exit flag state.
    pub query_on_exit_flag: bool,
    /// Process filter callback (or default marker symbol).
    pub filter: Value,
    /// Process sentinel callback (or default marker symbol).
    pub sentinel: Value,
    /// Server process log callback.
    pub log: Value,
    /// Process plist state.
    pub plist: Value,
    /// Pipe process attached to standard error.
    pub stderrproc: Value,
    /// Current decoding coding-system.
    pub coding_decode: Value,
    /// Current encoding coding-system.
    pub coding_encode: Value,
    /// Inherit-coding-system flag.
    pub inherit_coding_system_flag: bool,
    /// Attached thread object.
    pub thread: Value,
    /// Last process-window-size columns value.
    pub window_cols: Option<i64>,
    /// Last process-window-size rows value.
    pub window_rows: Option<i64>,
    /// Terminal name reported by `process-tty-name`, when this process uses a tty.
    pub tty_name: Value,
    /// Whether stdin is tty-backed for this process.
    pub tty_stdin: bool,
    /// Whether stdout is tty-backed for this process.
    pub tty_stdout: bool,
    /// Whether stderr is tty-backed for this process.
    pub tty_stderr: bool,
    /// The child's real OS process id, captured at spawn time.  GNU's
    /// `Fprocess_id` returns this pid (`XPROCESS (process)->pid`); it is `None`
    /// for network/serial/pipe connections that have no OS child, and stays
    /// independent of the internal `ProcessId` used to key the manager.
    pub os_pid: Option<u32>,
    /// The actual OS child process, if spawned (pipe mode).
    #[allow(dead_code)]
    pub child: Option<Child>,
    /// OS-level stdout pipe for non-blocking reads (pipe mode).
    pub child_stdout: Option<std::process::ChildStdout>,
    /// OS-level stderr pipe for non-blocking reads (pipe mode).
    pub child_stderr: Option<std::process::ChildStderr>,
    /// PTY master handle for resize and I/O (PTY mode).
    pub pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    /// PTY child process handle (PTY mode).
    pub pty_child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    /// PTY reader for non-blocking reads from the master side.
    pub pty_reader: Option<Box<dyn IoRead + Send>>,
    /// PTY writer for sending input to the master side.
    pub pty_writer: Option<Box<dyn std::io::Write + Send>>,
    /// Current peer address for datagram network processes, as Lisp.
    pub datagram_address: Value,
    /// Current peer address for datagram network processes, as a Rust socket address.
    pub datagram_socket_addr: Option<SocketAddr>,
    /// Current peer address for Unix datagram network processes.
    #[cfg(unix)]
    pub datagram_unix_path: Option<PathBuf>,
    pub network_socket: Option<NetworkSocket>,
    pending_network_connect: Option<PendingNetworkConnect>,
    /// TLS-wrapped stream for encrypted network connections.
    /// When `Some`, reads/writes go through this instead of `socket`.
    pub(crate) tls_stream: Option<TlsStream>,
    /// GNU-compatible GnuTLS initialization stage for this process.
    pub(crate) gnutls_initstage: GnutlsInitStage,
    /// Deferred parameters set by `gnutls-asynchronous-parameters`.
    pub(crate) gnutls_boot_parameters: Value,
    /// End-of-output marker, matching GNU's `p->mark`.
    pub mark: Value,
    /// Working directory for the subprocess, derived from
    /// `default-directory` at the time the process was created.
    /// If `None`, the child inherits the Rust process's cwd.
    pub default_directory: Option<PathBuf>,
}

impl std::fmt::Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Process")
            .field("id", &self.id)
            .field("name", &process_name_runtime(self.name))
            .field("command", &self.command)
            .field("kind", &self.kind)
            .field("proc_type", &self.proc_type)
            .field("status", &self.status)
            .field("buffer", &self.buffer)
            .field("childp", &self.childp)
            .field("pty_master", &self.pty_master.as_ref().map(|_| ".."))
            .field("pty_child", &self.pty_child.is_some())
            .field("pty_reader", &self.pty_reader.as_ref().map(|_| ".."))
            .field("pty_writer", &self.pty_writer.as_ref().map(|_| ".."))
            .field(
                "network_socket",
                &self.network_socket.as_ref().map(NetworkSocket::kind_name),
            )
            .finish_non_exhaustive()
    }
}

/// Manages the set of live processes.
///
/// Uses `polling::Poller` for efficient I/O multiplexing (epoll on Linux,
/// kqueue on macOS, wepoll on Windows) instead of sleep-based polling.
pub struct ProcessManager {
    processes: HashMap<ProcessId, Process>,
    deleted_processes: HashMap<ProcessId, Process>,
    next_id: ProcessId,
    /// Environment variable overrides (for `setenv`/`getenv`).
    env_overrides: HashMap<LispString, Option<LispString>>,
    wait_backend: ProcessWaitBackend,
}

/// Opaque, thread-safe handle the render/frontend thread uses to wake a blocked
/// wait loop after delivering input, via the cross-platform `Poller::notify()`.
///
/// This is the platform-agnostic replacement for the Unix-only wakeup pipe: it
/// works identically on Linux/macOS/Windows (the `polling` crate maps `notify`
/// onto eventfd/pipe/IOCP as appropriate) and is one-shot + remembered if no
/// waiter is currently blocked, so `send`-then-`notify` never loses a wakeup.
#[derive(Clone)]
pub struct WaitNotifier {
    poller: Arc<polling::Poller>,
}

impl WaitNotifier {
    fn new(poller: Arc<polling::Poller>) -> Self {
        Self { poller }
    }

    /// Wake the current (or next) `poller.wait()` so the evaluator drains its
    /// input channel. Call this right after pushing an event to the input
    /// channel from the render/frontend thread.
    pub fn notify(&self) {
        let _ = self.poller.notify();
    }
}

struct ProcessWaitBackend {
    /// I/O multiplexer for process descriptors and render-thread input wakeups.
    ///
    /// Shared (`Arc`) so the render/frontend thread can wake a blocked
    /// `poller.wait()` via the cross-platform `Poller::notify()` — the basis for
    /// the unified single-poll wait loop (no per-OS wakeup pipe needed).
    poller: Option<Arc<polling::Poller>>,
    /// Render-thread input wakeup fd registered in the shared wait poller.
    #[cfg(unix)]
    input_wakeup_fd: Option<std::os::unix::io::RawFd>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessWaitBackendInterest {
    ProcessesOnly,
    InputWakeupOnly,
    InputWakeupAndProcesses,
}

impl ProcessWaitBackendInterest {
    fn wants_input_wakeup(self) -> bool {
        matches!(self, Self::InputWakeupOnly | Self::InputWakeupAndProcesses)
    }

    fn wants_processes(self) -> bool {
        matches!(self, Self::ProcessesOnly | Self::InputWakeupAndProcesses)
    }
}

impl ProcessWaitBackend {
    fn new() -> Self {
        Self {
            poller: polling::Poller::new().ok().map(Arc::new),
            #[cfg(unix)]
            input_wakeup_fd: None,
        }
    }

    fn poller(&self) -> Option<&polling::Poller> {
        self.poller.as_deref()
    }

    /// A shared handle the frontend uses to wake a blocked wait (cross-platform).
    fn notify_handle(&self) -> Option<WaitNotifier> {
        self.poller.clone().map(WaitNotifier::new)
    }

    #[cfg(unix)]
    fn register_input_wakeup_fd(&mut self, fd: std::os::unix::io::RawFd) {
        let Some(ref poller) = self.poller else {
            self.input_wakeup_fd = None;
            return;
        };

        if let Some(old_fd) = self.input_wakeup_fd.take() {
            let borrowed = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(old_fd) };
            let _ = poller.delete(borrowed);
        }

        // SAFETY: the fd is owned by the display communication layer and
        // remains valid for the evaluator lifetime after `init_input_system`.
        let registered = unsafe {
            poller.add_with_mode(
                fd,
                polling::Event::readable(INPUT_WAKEUP_EVENT_KEY),
                polling::PollMode::Level,
            )
        }
        .is_ok();

        if registered {
            self.input_wakeup_fd = Some(fd);
        }
    }

    #[cfg(not(unix))]
    fn register_input_wakeup_fd(&mut self, _fd: super::eval::WakeupFd) {}

    fn has_input_wakeup(&self) -> bool {
        // Cross-platform: any live poller can be woken via `Poller::notify()`,
        // so the unified input+process wait path is available on every OS — not
        // only where the Unix wakeup pipe (`input_wakeup_fd`) is registered.
        self.poller.is_some()
    }

    fn wait_for_events(
        &self,
        processes: &HashMap<ProcessId, Process>,
        timeout: std::time::Duration,
        interest: ProcessWaitBackendInterest,
    ) -> Option<ProcessWaitEvents> {
        if let Some(ref poller) = self.poller {
            let deadline = Instant::now() + timeout;
            loop {
                let now = Instant::now();
                let wait_time = if timeout.is_zero() {
                    Duration::ZERO
                } else {
                    deadline.saturating_duration_since(now)
                };
                let mut events = polling::Events::new();
                match poller.wait(&mut events, Some(wait_time)) {
                    Ok(_) => {
                        let mut input_wakeup = false;
                        let mut ready_processes = Vec::new();
                        let mut writable_processes = Vec::new();
                        for event in events.iter() {
                            if event.key == INPUT_WAKEUP_EVENT_KEY {
                                if interest.wants_input_wakeup() {
                                    input_wakeup = true;
                                }
                                continue;
                            }
                            if interest.wants_processes() {
                                let id = event.key as ProcessId;
                                let Some(process) = processes.get(&id) else {
                                    continue;
                                };
                                if event.readable && process_has_readable_process_io(process) {
                                    ready_processes.push(id);
                                }
                                if event.writable && process.pending_network_connect.is_some() {
                                    writable_processes.push(id);
                                }
                            }
                        }
                        // A cross-platform `Poller::notify()` wake (the frontend
                        // delivered input) carries no event — so any wake while
                        // input is of interest means "input may be ready": surface
                        // it and let the caller drain the input channel. This also
                        // makes the wait return promptly instead of yield-looping.
                        if interest.wants_input_wakeup() {
                            input_wakeup = true;
                        }
                        let backend = ProcessWaitEvents::from_sources_with_writable(
                            input_wakeup,
                            ready_processes,
                            writable_processes,
                        );
                        if backend.has_input_wakeup()
                            || backend.has_ready_processes()
                            || backend.has_writable_processes()
                            || timeout.is_zero()
                            || Instant::now() >= deadline
                        {
                            return Some(backend);
                        }
                        std::thread::yield_now();
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        }

        None
    }
}

struct AcceptedNetworkConnection {
    server_id: ProcessId,
    client_id: ProcessId,
    log: Value,
    sentinel: Value,
    log_message: String,
    sentinel_message: String,
}

fn accepted_network_process_name(server_name: &str, addr: SocketAddr) -> String {
    match addr {
        SocketAddr::V4(v4) => format!("{} <{}:{}>", server_name, v4.ip(), v4.port()),
        SocketAddr::V6(v6) => format!("{} <[{}]:{}>", server_name, v6.ip(), v6.port()),
    }
}

impl std::fmt::Debug for ProcessManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessManager")
            .field("processes", &self.processes)
            .field("next_id", &self.next_id)
            .finish()
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

fn process_name_value(name: &str) -> Value {
    Value::heap_string(super::builtins::plain_str_to_lisp_string(name, true))
}

fn process_name_lisp_value(name: &LispString) -> Value {
    Value::heap_string(name.clone())
}

fn process_name_runtime(name: Value) -> String {
    name.as_lisp_string()
        .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
        .unwrap_or_else(|| "<invalid-process-name>".to_string())
}

fn process_type_value(kind: &ProcessKind) -> Value {
    Value::symbol(kind.name())
}

fn make_process_command_lisp_value(
    kind: &ProcessKind,
    program: &LispString,
    args: &[LispString],
) -> Value {
    if *kind != ProcessKind::Real || program.is_empty() {
        return Value::NIL;
    }
    let mut items = Vec::with_capacity(args.len() + 1);
    items.push(Value::heap_string(program.clone()));
    items.extend(args.iter().cloned().map(Value::heap_string));
    Value::list(items)
}

fn process_command_lisp_argv(command: Value) -> Option<Vec<LispString>> {
    let items = list_to_vec(&command)?;
    items
        .iter()
        .map(|value| value.as_lisp_string().cloned())
        .collect::<Option<Vec<_>>>()
}

fn process_spawn_lisp_argv(proc: &Process) -> Option<Vec<LispString>> {
    let mut argv = process_command_lisp_argv(proc.command)?;
    if let (Some(executable), Some(program)) = (&proc.executable, argv.first_mut()) {
        *program = executable.clone();
    }
    Some(argv)
}

fn lisp_string_from_bytes(bytes: &[u8], multibyte: bool) -> LispString {
    if multibyte {
        LispString::from_emacs_bytes(bytes.to_vec())
    } else {
        LispString::from_unibyte(bytes.to_vec())
    }
}

fn lisp_bytes_to_os_string(bytes: &[u8], _multibyte: bool) -> OsString {
    // Issue #131: on Unix the OS path is the string's bytes verbatim — for a
    // unibyte string those are the raw bytes, for a multibyte string they are the
    // Emacs internal encoding (valid UTF-8 for ordinary text), matching the
    // byte-faithful boundary in `fileio::lisp_file_name_to_path_buf`. A raw
    // eight-bit byte therefore reaches the kernel as itself rather than as an
    // in-Unicode storage sentinel.
    #[cfg(unix)]
    {
        OsString::from_vec(bytes.to_vec())
    }

    #[cfg(not(unix))]
    {
        OsString::from(crate::emacs_core::emacs_char::to_utf8_lossy(bytes))
    }
}

fn lisp_string_to_os_string(string: &LispString) -> OsString {
    lisp_bytes_to_os_string(string.as_bytes(), string.is_multibyte())
}

fn executable_path_exists(path: &Path) -> bool {
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

#[derive(Clone, Copy)]
struct ProcessExecLookup<'a> {
    exec_path: Value,
    exec_suffixes: Value,
    default_directory: Option<&'a LispString>,
}

fn process_lookup_error(program: &LispString) -> Flow {
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

fn process_exec_suffixes(lookup: ProcessExecLookup<'_>) -> Result<Vec<LispString>, Flow> {
    if lookup.exec_suffixes.is_nil() {
        return Ok(vec![LispString::from_unibyte(Vec::new())]);
    }

    let suffix_values = list_to_vec(&lookup.exec_suffixes)
        .ok_or_else(|| signal_wrong_type_string(lookup.exec_suffixes))?;
    suffix_values
        .iter()
        .map(|value| super::builtins::expect_lisp_string(value).cloned())
        .collect()
}

fn process_program_is_absolute(program: &LispString) -> bool {
    Path::new(&lisp_string_to_os_string(program)).is_absolute()
}

fn resolve_async_process_program(
    lookup: ProcessExecLookup<'_>,
    program: &LispString,
) -> Result<LispString, Flow> {
    if process_program_is_absolute(program) {
        let path = PathBuf::from(lisp_string_to_os_string(program));
        if path.is_dir() {
            return Err(signal(
                "error",
                vec![Value::string(
                    "Specified program for new process is a directory",
                )],
            ));
        }
        return Ok(program.clone());
    }

    let path_entries = if lookup.exec_path.is_nil() {
        Vec::new()
    } else {
        list_to_vec(&lookup.exec_path).ok_or_else(|| process_lookup_error(program))?
    };
    let suffixes = process_exec_suffixes(lookup)?;
    let program_path = super::fileio::lisp_file_name_to_path_buf(program);

    for entry in path_entries {
        let Some(directory) = (match entry.kind() {
            ValueKind::Nil => lookup
                .default_directory
                .map(super::fileio::lisp_file_name_to_path_buf),
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
                return Ok(os_str_to_lisp_string(candidate.as_os_str()));
            }
        }
    }

    Err(process_lookup_error(program))
}

fn os_str_to_lisp_string(value: &OsStr) -> LispString {
    #[cfg(unix)]
    {
        LispString::from_unibyte(value.as_bytes().to_vec())
    }

    #[cfg(not(unix))]
    {
        LispString::from_utf8(value.to_string_lossy().as_ref())
    }
}

fn process_coding_symbol_name(value: Value) -> &'static str {
    match value.as_symbol_name() {
        Some(name) => name,
        None => "utf-8-unix",
    }
}

fn process_coding_is_binary(coding: Value) -> bool {
    coding.is_nil()
        || matches!(
            coding.as_symbol_name(),
            Some("binary" | "no-conversion" | "raw-text")
        )
}

/// Encode the data passed to `process-send-string`/`process-send-region`
/// through a process's ENCODE coding system, mirroring GNU `send_process`
/// (src/process.c).  A `binary`/`raw-text`/`no-conversion`/nil encode coding
/// (or an unset one) leaves the bytes untouched; every other coding goes through
/// the shared string encoder, which performs character-code conversion and the
/// EOL conversion the coding's eol_type requests.
fn encode_process_send_input(
    processes: &ProcessManager,
    id: ProcessId,
    input: &LispString,
) -> LispString {
    let coding = processes
        .get_any(id)
        .map(|proc| proc.coding_encode)
        .unwrap_or(Value::NIL);
    if process_coding_is_binary(coding) {
        return input.clone();
    }
    let bytes = crate::encoding::encode_lisp_string(input, process_coding_symbol_name(coding));
    LispString::from_unibyte(bytes)
}

fn decode_process_output_bytes(bytes: &[u8], coding: Value) -> LispString {
    if process_coding_is_binary(coding) {
        LispString::from_unibyte(bytes.to_vec())
    } else {
        // Issue #131: decode straight to Emacs bytes so process output keeps real
        // PUA glyphs and eight-bit raw bytes instead of round-tripping through the
        // lossy storage-string form (the old `from_utf8(decode_bytes(..))`).
        crate::encoding::decode_bytes_to_lisp_string(bytes, process_coding_symbol_name(coding))
    }
}

fn process_output_runtime_string(output: &LispString) -> String {
    // Issue #131: this only feeds the `proc.stdout: String` diagnostic mirror
    // (read solely via `get_output` in tests). The byte-faithful process output
    // that is actually inserted into the process buffer flows through
    // `read_process_output` as a `LispString`. A lossy UTF-8 rendering here
    // keeps real Unicode (incl. PUA) and avoids the buggy storage-string form.
    crate::emacs_core::emacs_char::to_utf8_lossy(output.as_bytes())
}

#[cfg(unix)]
fn configure_child_pty_tty(tty_name: &OsStr) -> Result<(), String> {
    use std::os::unix::io::RawFd;

    #[cfg(unix)]
    fn close_fd(fd: RawFd) {
        unsafe {
            libc::close(fd);
        }
    }

    #[cfg(unix)]
    fn set_cc(settings: &mut libc::termios, index: usize, value: u8) {
        if index < settings.c_cc.len() {
            settings.c_cc[index] = value;
        }
    }

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

fn env_var_name_bytes_eq(left: &[u8], right: &[u8]) -> bool {
    #[cfg(windows)]
    {
        left.eq_ignore_ascii_case(right)
    }

    #[cfg(not(windows))]
    {
        left == right
    }
}

fn split_process_environment_entry(entry: &LispString) -> (OsString, Option<OsString>) {
    let bytes = entry.as_bytes();
    let multibyte = entry.is_multibyte();
    if let Some(eq_pos) = bytes.iter().position(|&byte| byte == b'=') {
        (
            lisp_bytes_to_os_string(&bytes[..eq_pos], multibyte),
            Some(lisp_bytes_to_os_string(&bytes[eq_pos + 1..], multibyte)),
        )
    } else {
        (lisp_bytes_to_os_string(bytes, multibyte), None)
    }
}

pub(crate) fn process_environment_entries(
    process_environment: Option<Value>,
) -> Option<Vec<(OsString, Option<OsString>)>> {
    let env_list = process_environment?;
    if !env_list.is_cons() {
        return None;
    }
    list_to_vec(&env_list).map(|entries| {
        // GNU's `make_environment_block`/`add_env` (callproc.c) keeps the
        // FIRST definition of a given variable that appears in
        // `process-environment` and drops later duplicates. This applies to
        // both "VAR=value" entries and bare "VAR" unset entries: whichever
        // occurs first wins. Dedup by variable name preserving first-seen so
        // every spawn path (call-process and make-process/pipe/pty) gets the
        // same precedence.
        let mut seen: std::collections::HashSet<OsString> = std::collections::HashSet::new();
        entries
            .iter()
            .filter_map(|entry| entry.as_lisp_string().map(split_process_environment_entry))
            .filter(|(key, _)| seen.insert(key.clone()))
            .collect()
    })
}

fn update_process_mark(buffers: &mut BufferManager, proc: &mut Process) -> EvalResult {
    let Some(buffer_id) = proc.buffer.as_buffer_id() else {
        return super::marker::builtin_set_marker_in_buffers(buffers, vec![proc.mark, Value::NIL]);
    };
    let Some(buffer) = buffers.get(buffer_id) else {
        return super::marker::builtin_set_marker_in_buffers(buffers, vec![proc.mark, Value::NIL]);
    };
    let position = Value::fixnum(buffer.z_lisp_char_pos().as_i64());
    super::marker::builtin_set_marker_in_buffers(buffers, vec![proc.mark, position, proc.buffer])
}

fn process_status_run_value() -> Value {
    Value::symbol("run")
}

fn process_status_connect_value() -> Value {
    Value::symbol("connect")
}

fn write_queue_push(queue: Value, input_obj: Value, front: bool) -> Value {
    let len = input_obj
        .as_lisp_string()
        .map(|string| string.sbytes() as i64)
        .unwrap_or(0);
    let entry = Value::cons(input_obj, Value::cons(Value::fixnum(0), Value::fixnum(len)));
    let mut entries = list_to_vec(&queue).unwrap_or_default();
    if front {
        entries.insert(0, entry);
    } else {
        entries.push(entry);
    }
    Value::list(entries)
}

fn parse_make_network_tls_parameters(
    value: Value,
) -> Result<Option<super::tls::GnutlsBootParameters>, Flow> {
    if value.is_nil() {
        return Ok(None);
    }
    let items = list_to_vec(&value)
        .ok_or_else(|| signal("wrong-type-argument", vec![Value::symbol("listp"), value]))?;
    let Some((&credential_type, rest)) = items.split_first() else {
        return Ok(None);
    };
    parse_gnutls_boot_parameters(credential_type, Value::list(rest.to_vec())).map(Some)
}

fn process_status_stop_value(signal_num: i64) -> Value {
    Value::list(vec![Value::symbol("stop"), Value::fixnum(signal_num)])
}

fn process_status_exit_value(code: i32) -> Value {
    Value::list(vec![Value::symbol("exit"), Value::fixnum(code as i64)])
}

fn process_status_failed_value(code: i32) -> Value {
    Value::list(vec![Value::symbol("failed"), Value::fixnum(code as i64)])
}

/// Convert a finished `std::process::ExitStatus` to an Emacs process status:
/// `(exit CODE)` for a normal exit, `(signal N ...)` for signal death (GNU
/// distinguishes the two via `WIFSIGNALED`/`WTERMSIG`).
fn process_status_from_exit(status: &std::process::ExitStatus) -> Value {
    if let Some(code) = status.code() {
        return process_status_exit_value(code);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return process_status_signal_value(sig);
        }
    }
    process_status_exit_value(1)
}

fn process_status_signal_value(signal_num: i32) -> Value {
    Value::list(vec![
        Value::symbol("signal"),
        Value::fixnum(signal_num as i64),
        Value::NIL,
    ])
}

/// Map a `strsignal`-style description (as produced by `portable_pty::ExitStatus`)
/// back to a signal number by scanning the platform's signal table. Both the
/// PTY layer and this lookup call `strsignal`, so the descriptions match exactly.
#[cfg(unix)]
fn signal_number_from_description(name: &str) -> Option<i32> {
    // portable_pty falls back to "Signal N" when strsignal yields NULL.
    if let Some(rest) = name.strip_prefix("Signal ") {
        if let Ok(n) = rest.trim().parse::<i32>() {
            return Some(n);
        }
    }
    for signum in 1..=64i32 {
        let desc = unsafe {
            let p = libc::strsignal(signum);
            if p.is_null() {
                continue;
            }
            std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
        };
        if desc == name {
            return Some(signum);
        }
    }
    None
}

/// Convert a finished `portable_pty::ExitStatus` (PTY child) to an Emacs process
/// status. GNU distinguishes signal death from a normal exit via
/// `WIFSIGNALED`/`WTERMSIG`; portable_pty preserves this as `signal()`/`exit_code()`.
#[cfg(unix)]
fn process_status_from_pty_exit(status: &portable_pty::ExitStatus) -> Value {
    if let Some(sig_name) = status.signal() {
        let signum = signal_number_from_description(sig_name).unwrap_or(0);
        return process_status_signal_value(signum);
    }
    process_status_exit_value(status.exit_code() as i32)
}

#[cfg(not(unix))]
fn process_status_from_pty_exit(status: &portable_pty::ExitStatus) -> Value {
    if status.success() {
        process_status_exit_value(0)
    } else {
        process_status_exit_value(status.exit_code() as i32)
    }
}

/// GNU `status_message` (process.c): the human-readable sentinel/buffer message
/// for a finished process status. Signal/stop death reports the `strsignal`
/// description with its first character down-cased; a non-zero exit reports
/// "exited abnormally with code N"; a zero exit reports "finished".
fn gnu_process_status_message(status: Value) -> String {
    match ProcessStatusSymbol::from_status_value(status) {
        Some(ProcessStatusSymbol::Exit) => {
            let code = process_status_code_value(status);
            if code == 0 {
                "finished\n".to_string()
            } else {
                format!("exited abnormally with code {code}\n")
            }
        }
        Some(ProcessStatusSymbol::Failed) => {
            let code = process_status_code_value(status);
            format!("failed with code {code}\n")
        }
        Some(ProcessStatusSymbol::Signal) | Some(ProcessStatusSymbol::Stop) => {
            let code = process_status_code_value(status);
            let desc = signal_description(code as i32);
            format!("{desc}\n")
        }
        _ => "finished\n".to_string(),
    }
}

/// strsignal description with the first character down-cased, matching GNU's
/// `status_message`. Falls back to "unknown" when strsignal yields NULL.
fn signal_description(signum: i32) -> String {
    #[cfg(unix)]
    {
        let raw = unsafe {
            let p = libc::strsignal(signum);
            if p.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned())
            }
        };
        if let Some(s) = raw {
            let mut chars = s.chars();
            return match chars.next() {
                Some(first) => {
                    let lowered: String = first.to_lowercase().collect();
                    format!("{lowered}{}", chars.as_str())
                }
                None => s,
            };
        }
    }
    let _ = signum;
    "unknown".to_string()
}

fn process_status_symbol_value(status: Value) -> Value {
    list_to_vec(&status)
        .and_then(|items| items.first().copied())
        .unwrap_or(status)
}

fn process_status_code_value(status: Value) -> i64 {
    list_to_vec(&status)
        .and_then(|items| items.get(1).copied())
        .and_then(|value| value.as_fixnum())
        .unwrap_or(0)
}

/// Resolve the (decode . encode) coding pair for a network process, mirroring
/// GNU `set_network_socket_coding_system` (src/process.c:3291-3367).
///
/// * An explicit `:coding` value supplies both directions (its car/cdr if a
///   cons, else the whole symbol for both).
/// * When `:coding` is nil GNU does NOT fall back to `binary`.  Instead it
///   consults `coding-system-for-read/write` and, failing those, the buffer's
///   multibyteness:
///     - DECODE: if the process buffer (or, when absent, the default buffer)
///       is UNIBYTE, decode is left nil/raw so libraries receive bare CR LF;
///       otherwise decode comes from `find-operation-coding-system`, which for
///       a plain local socket has no alist entry and falls through to the car
///       of `default-process-coding-system` (`utf-8-unix`).
///     - ENCODE: comes from the cdr of `default-process-coding-system`.
///
/// `default_coding` is the runtime value of `default-process-coding-system`
/// (a cons `(decode . encode)`, normally `(utf-8-unix . utf-8-unix)`); a nil
/// or malformed value falls back to `binary`.  `buffer_multibyte` is the
/// multibyteness of the process buffer (true when there is no buffer, since
/// the default buffer is multibyte), matching GNU's `p->buffer` /
/// `buffer_defaults` test.
fn network_process_coding_pair(
    coding: Value,
    default_coding: Value,
    buffer_multibyte: bool,
) -> (Value, Value) {
    if coding.is_cons() {
        return (coding.cons_car(), coding.cons_cdr());
    }
    if !coding.is_nil() {
        return (coding, coding);
    }
    // `:coding` unspecified: derive from `default-process-coding-system`.
    // GNU `set_network_socket_coding_system` (process.c:3331-3336, 3361-3366)
    // uses the car/cdr of `Vdefault_process_coding_system` when it is a cons,
    // and otherwise falls back to `Qnil` (NOT `binary`) — so an unset default
    // yields a nil decode/encode, matching `(process-coding-system)` of `(nil)`.
    let (default_decode, default_encode) = if default_coding.is_cons() {
        (default_coding.cons_car(), default_coding.cons_cdr())
    } else {
        (Value::NIL, Value::NIL)
    };
    // GNU leaves the decode side as nil (raw) for a unibyte buffer so the
    // process receives bare bytes; a multibyte (or absent) buffer decodes via
    // the default process coding system.
    let decode = if buffer_multibyte {
        default_decode
    } else {
        Value::NIL
    };
    (decode, default_encode)
}

fn set_network_process_coding(
    proc: &mut Process,
    coding: Value,
    default_coding: Value,
    buffer_multibyte: bool,
) {
    let (decode, encode) = network_process_coding_pair(coding, default_coding, buffer_multibyte);
    proc.coding_decode = decode;
    proc.coding_encode = encode;
}

fn explicit_process_coding_pair(coding: Value) -> (Value, Value) {
    if coding.is_cons() {
        (coding.cons_car(), coding.cons_cdr())
    } else {
        (coding, coding)
    }
}

fn validate_process_coding_component(
    coding_systems: Option<&super::coding::CodingSystemManager>,
    value: Value,
) -> Result<(), Flow> {
    if let Some(coding_systems) = coding_systems {
        super::coding::builtin_check_coding_system(coding_systems, vec![value]).map(|_| ())
    } else if value.is_nil() || value.as_symbol_name().is_some() {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), value],
        ))
    }
}

fn validate_process_coding_value(
    coding_systems: Option<&super::coding::CodingSystemManager>,
    coding: Value,
) -> Result<(), Flow> {
    let (decode, encode) = explicit_process_coding_pair(coding);
    validate_process_coding_component(coding_systems, decode)?;
    validate_process_coding_component(coding_systems, encode)
}

fn set_explicit_process_coding(proc: &mut Process, coding: Value) {
    if coding.is_nil() {
        return;
    }
    let (decode, encode) = explicit_process_coding_pair(coding);
    proc.coding_decode = decode;
    proc.coding_encode = encode;
}

fn copy_process_plist(plist: Value) -> EvalResult {
    super::builtins::builtin_copy_sequence(vec![plist])
}

fn apply_connection_process_flags(proc: &mut Process, noquery: bool, stop: bool) {
    if noquery {
        proc.query_on_exit_flag = false;
    }
    if stop {
        proc.command = Value::T;
    }
}

#[derive(Debug)]
enum ProcessOutputRead {
    Data(LispString),
    WouldBlock,
    Eof,
    NoSource,
}

impl ProcessOutputRead {
    fn from_io_result(
        result: std::io::Result<usize>,
        bytes: &[u8],
        coding: Value,
    ) -> ProcessOutputRead {
        match result {
            Ok(0) => Self::Eof,
            Ok(n) => Self::Data(decode_process_output_bytes(&bytes[..n], coding)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Self::WouldBlock,
            Err(_) => Self::Eof,
        }
    }

    fn into_legacy_option(self) -> Option<LispString> {
        match self {
            Self::Data(data) => Some(data),
            Self::WouldBlock => Some(LispString::from_utf8("")),
            Self::Eof | Self::NoSource => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessOutputSource {
    Pty,
    ChildStdout,
    /// A stderr pipe-process (created for `make-process :stderr`) whose readable
    /// source is the child's separate stderr pipe.  GNU connects the stderr
    /// pipe-process's `READ_FROM_SUBPROCESS` fd to the child's stderr; here that
    /// read end lives in `child_stderr` on the stderr pipe-process record.
    ChildStderr,
    Network,
}

fn process_output_source(proc: &Process) -> Option<ProcessOutputSource> {
    if proc.pty_reader.is_some() {
        Some(ProcessOutputSource::Pty)
    } else if proc.child_stdout.is_some() {
        Some(ProcessOutputSource::ChildStdout)
    } else if proc.child_stderr.is_some() {
        Some(ProcessOutputSource::ChildStderr)
    } else if proc.tls_stream.is_some() || proc.network_socket.is_some() {
        Some(ProcessOutputSource::Network)
    } else {
        None
    }
}

fn process_status_is_run(status: &Value) -> bool {
    ProcessStatusSymbol::from_status_value(*status) == Some(ProcessStatusSymbol::Run)
}

fn process_status_allows_send(status: &Value) -> bool {
    matches!(
        ProcessStatusSymbol::from_status_value(*status),
        Some(ProcessStatusSymbol::Run | ProcessStatusSymbol::Open)
    )
}

fn process_status_is_connect(status: &Value) -> bool {
    ProcessStatusSymbol::from_status_value(*status) == Some(ProcessStatusSymbol::Connect)
}

fn process_status_has_readable_process_io(status: &Value) -> bool {
    matches!(
        ProcessStatusSymbol::from_status_value(*status),
        Some(
            ProcessStatusSymbol::Run
                | ProcessStatusSymbol::Open
                | ProcessStatusSymbol::Listen
                | ProcessStatusSymbol::Connect
        )
    )
}

fn process_stopped_for_io(proc: &Process) -> bool {
    proc.command == Value::T
}

fn process_has_readable_process_io(proc: &Process) -> bool {
    !process_stopped_for_io(proc) && process_status_has_readable_process_io(&proc.status)
}

impl super::eval::Context {
    fn wait_while_network_process_connecting(&mut self, id: ProcessId) -> Result<(), Flow> {
        while self.processes.get(id).is_some_and(|proc| {
            proc.kind == ProcessKind::Network && process_status_is_connect(&proc.status)
        }) {
            let _ = self.wait_for_process_output(ProcessOutputWaitRequest::new(
                ProcessOutputWaitTiming::For(Duration::from_millis(20)),
                Some(id),
                false,
                true,
            ))?;
        }
        Ok(())
    }
}

fn pending_network_connect_id(
    processes: &ProcessManager,
    process: Value,
) -> Result<Option<ProcessId>, Flow> {
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &process)?;
    Ok(processes
        .get(id)
        .is_some_and(|proc| {
            proc.kind == ProcessKind::Network && proc.pending_network_connect.is_some()
        })
        .then_some(id))
}

fn process_uses_contact_plist(proc: &Process) -> bool {
    matches!(
        proc.kind,
        ProcessKind::Network | ProcessKind::Pipe | ProcessKind::Serial
    )
}

fn process_contact_plist_get(contact: Value, key: Value) -> Value {
    super::builtins::builtin_plist_get(vec![contact, key]).unwrap_or(Value::NIL)
}

fn process_contact_plist_put(contact: Value, key: Value, value: Value) -> EvalResult {
    super::builtins::builtin_plist_put(vec![contact, key, value])
}

fn process_contact_server_p(proc: &Process) -> bool {
    process_contact_plist_get(proc.childp, ProcessKeyword::Server.value()).is_truthy()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkSocketOption {
    Bindtodevice,
    Broadcast,
    Dontroute,
    Keepalive,
    Linger,
    Oobinline,
    Priority,
    Reuseaddr,
    Nodelay,
}

#[derive(Clone, Copy, Debug)]
struct NetworkSocketOptionSpec {
    keyword: ProcessKeyword,
    option: NetworkSocketOption,
    value: Value,
}

#[derive(Clone, Debug)]
enum PendingNetworkConnect {
    Tcp {
        remaining_addrs: Vec<SocketAddr>,
        socket_options: Vec<NetworkSocketOptionSpec>,
    },
    #[cfg(unix)]
    Local,
}

#[derive(Debug)]
struct PendingNetworkConnectStarted {
    stream: TcpStream,
    remote_addr: SocketAddr,
    remaining_addrs: Vec<SocketAddr>,
}

#[derive(Debug)]
enum PendingNetworkConnectStart {
    Started(PendingNetworkConnectStarted),
    Failed(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingNetworkConnectCompletion {
    None,
    Retrying,
    Connected { sentinel: Value },
    Failed { sentinel: Value, code: i32 },
}

impl NetworkSocketOption {
    fn from_keyword(keyword: ProcessKeyword) -> Option<Self> {
        match keyword {
            ProcessKeyword::Bindtodevice => Some(Self::Bindtodevice),
            ProcessKeyword::Broadcast => Some(Self::Broadcast),
            ProcessKeyword::Dontroute => Some(Self::Dontroute),
            ProcessKeyword::Keepalive => Some(Self::Keepalive),
            ProcessKeyword::Linger => Some(Self::Linger),
            ProcessKeyword::Oobinline => Some(Self::Oobinline),
            ProcessKeyword::Priority => Some(Self::Priority),
            ProcessKeyword::Reuseaddr => Some(Self::Reuseaddr),
            ProcessKeyword::Nodelay => Some(Self::Nodelay),
            _ => None,
        }
    }
}

fn network_socket_options_include(
    options: &[NetworkSocketOptionSpec],
    option: NetworkSocketOption,
) -> bool {
    options.iter().any(|spec| spec.option == option)
}

fn collect_network_socket_options(args: &[Value]) -> Vec<NetworkSocketOptionSpec> {
    let mut options = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).cloned().unwrap_or(Value::NIL);
        if let Some(keyword) = ProcessKeyword::from_value(key)
            && let Some(option) = NetworkSocketOption::from_keyword(keyword)
        {
            options.push(NetworkSocketOptionSpec {
                keyword,
                option,
                value,
            });
        }
        i += 2;
    }
    options
}

fn network_server_backlog(server_value: Value) -> Result<i32, Flow> {
    if server_value == Value::T {
        return Ok(5);
    }
    match server_value.as_fixnum() {
        Some(backlog) => {
            i32::try_from(backlog).map_err(|_| signal_wrong_type_integerp(server_value))
        }
        None => Err(signal_wrong_type_integerp(server_value)),
    }
}

fn signal_bad_network_option_value(keyword: ProcessKeyword) -> Flow {
    signal(
        "error",
        vec![Value::string(format!(
            "Bad option value for {}",
            keyword.keyword()
        ))],
    )
}

fn signal_network_option_io_error(
    keyword: ProcessKeyword,
    value: Value,
    err: std::io::Error,
) -> Flow {
    signal(
        "file-error",
        vec![
            Value::string("Cannot set network option"),
            keyword.value(),
            value,
            Value::string(err.to_string()),
        ],
    )
}

fn network_option_i32_value(keyword: ProcessKeyword, value: Value) -> Result<i32, Flow> {
    match value.as_fixnum().and_then(|n| i32::try_from(n).ok()) {
        Some(n) => Ok(n),
        None => Err(signal_bad_network_option_value(keyword)),
    }
}

#[cfg(unix)]
fn setsockopt_raw<T>(
    fd: RawFd,
    level: libc::c_int,
    optname: libc::c_int,
    value: &T,
) -> std::io::Result<()> {
    let rc = unsafe {
        libc::setsockopt(
            fd,
            level,
            optname,
            (value as *const T).cast(),
            std::mem::size_of::<T>() as libc::socklen_t,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn set_socket_bool_option_raw(fd: RawFd, optname: libc::c_int, value: bool) -> std::io::Result<()> {
    let raw: libc::c_int = if value { 1 } else { 0 };
    setsockopt_raw(fd, libc::SOL_SOCKET, optname, &raw)
}

#[cfg(unix)]
fn set_socket_linger_option_raw(fd: RawFd, value: Value) -> std::io::Result<()> {
    let raw = libc::linger {
        l_onoff: if value.is_nil() { 0 } else { 1 },
        l_linger: value
            .as_fixnum()
            .and_then(|n| i32::try_from(n).ok())
            .unwrap_or(0) as libc::c_int,
    };
    setsockopt_raw(fd, libc::SOL_SOCKET, libc::SO_LINGER, &raw)
}

#[cfg(unix)]
fn set_socket_priority_option_raw(fd: RawFd, priority: i32) -> std::io::Result<()> {
    let raw = priority as libc::c_int;
    setsockopt_raw(fd, libc::SOL_SOCKET, libc::SO_PRIORITY, &raw)
}

#[cfg(unix)]
fn apply_network_socket_option_to_socket(
    socket: &Socket,
    spec: NetworkSocketOptionSpec,
) -> EvalResult {
    let value = spec.value;
    let result = match spec.option {
        NetworkSocketOption::Bindtodevice => {
            if value.is_nil() {
                socket.bind_device(None)
            } else if let Some(name) = value.as_lisp_string() {
                socket.bind_device(Some(name.as_bytes()))
            } else {
                return Err(signal_bad_network_option_value(spec.keyword));
            }
        }
        NetworkSocketOption::Broadcast => socket.set_broadcast(value.is_truthy()),
        NetworkSocketOption::Dontroute => {
            set_socket_bool_option_raw(socket.as_raw_fd(), libc::SO_DONTROUTE, value.is_truthy())
        }
        NetworkSocketOption::Keepalive => socket.set_keepalive(value.is_truthy()),
        NetworkSocketOption::Linger => set_socket_linger_option_raw(socket.as_raw_fd(), value),
        NetworkSocketOption::Oobinline => socket.set_out_of_band_inline(value.is_truthy()),
        NetworkSocketOption::Priority => {
            let priority = network_option_i32_value(spec.keyword, value)?;
            set_socket_priority_option_raw(socket.as_raw_fd(), priority)
        }
        NetworkSocketOption::Reuseaddr => socket.set_reuse_address(value.is_truthy()),
        NetworkSocketOption::Nodelay => socket.set_tcp_nodelay(value.is_truthy()),
    };

    result
        .map(|_| Value::T)
        .map_err(|err| signal_network_option_io_error(spec.keyword, value, err))
}

#[cfg(not(unix))]
fn apply_network_socket_option_to_socket(
    _socket: &Socket,
    spec: NetworkSocketOptionSpec,
) -> EvalResult {
    Err(signal(
        "error",
        vec![Value::string(format!(
            "Unsupported network option {}",
            spec.keyword.keyword()
        ))],
    ))
}

fn apply_network_socket_options(
    socket: &Socket,
    options: &[NetworkSocketOptionSpec],
) -> Result<(), Flow> {
    for spec in options.iter().copied() {
        apply_network_socket_option_to_socket(socket, spec)?;
    }
    Ok(())
}

fn apply_network_socket_option_to_process(
    proc: &mut Process,
    spec: NetworkSocketOptionSpec,
) -> EvalResult {
    if let Some(socket) = proc.network_socket.as_ref() {
        return match socket {
            NetworkSocket::TcpStream(stream) => {
                apply_network_socket_option_to_socket(&SockRef::from(stream), spec)
            }
            NetworkSocket::TcpListener(listener) => {
                apply_network_socket_option_to_socket(&SockRef::from(listener), spec)
            }
            NetworkSocket::UdpSocket(socket) => {
                apply_network_socket_option_to_socket(&SockRef::from(socket), spec)
            }
            #[cfg(unix)]
            NetworkSocket::SeqpacketStream(socket) => {
                apply_network_socket_option_to_socket(&SockRef::from(socket), spec)
            }
            #[cfg(unix)]
            NetworkSocket::SeqpacketListener(socket) => {
                apply_network_socket_option_to_socket(&SockRef::from(socket), spec)
            }
            #[cfg(unix)]
            NetworkSocket::UnixStream(stream) => {
                apply_network_socket_option_to_socket(&SockRef::from(stream), spec)
            }
            #[cfg(unix)]
            NetworkSocket::UnixListener(listener) => {
                apply_network_socket_option_to_socket(&SockRef::from(listener), spec)
            }
            #[cfg(unix)]
            NetworkSocket::UnixDatagram(socket) => {
                apply_network_socket_option_to_socket(&SockRef::from(socket), spec)
            }
        };
    }

    if let Some(tls) = proc.tls_stream.as_ref() {
        return apply_network_socket_option_to_socket(&SockRef::from(tls.tcp_stream()), spec);
    }

    Err(signal(
        "error",
        vec![Value::string("Process has no socket")],
    ))
}

fn tcp_socket_domain(addr: SocketAddr) -> Domain {
    if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    }
}

fn network_socket_io_error(message: &str, err: std::io::Error) -> Flow {
    signal(
        "file-error",
        vec![Value::string(message), Value::string(err.to_string())],
    )
}

fn bind_tcp_listener_socket(
    addr: SocketAddr,
    backlog: i32,
    options: &[NetworkSocketOptionSpec],
) -> Result<TcpListener, Flow> {
    let socket = Socket::new(tcp_socket_domain(addr), Type::STREAM, Some(Protocol::TCP))
        .map_err(|err| network_socket_io_error("Cannot create server socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::from(addr);
    socket
        .bind(&sock_addr)
        .map_err(|err| network_socket_io_error("Cannot bind server socket", err))?;
    socket
        .listen(backlog)
        .map_err(|err| network_socket_io_error("Cannot listen on server socket", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

fn connect_tcp_stream_socket(
    addr: SocketAddr,
    options: &[NetworkSocketOptionSpec],
) -> Result<TcpStream, Flow> {
    let socket = Socket::new(tcp_socket_domain(addr), Type::STREAM, Some(Protocol::TCP))
        .map_err(|err| network_socket_io_error("Cannot create client socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::from(addr);
    socket
        .connect(&sock_addr)
        .map_err(|err| network_socket_io_error("make client process failed", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

fn nonblocking_connect_is_pending(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }
    #[cfg(unix)]
    {
        matches!(
            err.raw_os_error(),
            Some(code)
                if code == libc::EINPROGRESS
                    || code == libc::EWOULDBLOCK
                    || code == libc::EALREADY
        )
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn io_error_status_code(err: &std::io::Error) -> i32 {
    err.raw_os_error().unwrap_or(1)
}

fn start_nonblocking_tcp_stream_socket(
    addr: SocketAddr,
    options: &[NetworkSocketOptionSpec],
) -> Result<Result<TcpStream, std::io::Error>, Flow> {
    let socket = Socket::new(tcp_socket_domain(addr), Type::STREAM, Some(Protocol::TCP))
        .map_err(|err| network_socket_io_error("Cannot create client socket", err))?;
    apply_network_socket_options(&socket, options)?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    let sock_addr = SockAddr::from(addr);
    match socket.connect(&sock_addr) {
        Ok(()) => Ok(Ok(socket.into())),
        Err(err) if nonblocking_connect_is_pending(&err) => Ok(Ok(socket.into())),
        Err(err) => Ok(Err(err)),
    }
}

fn start_pending_tcp_stream_connect(
    addrs: Vec<SocketAddr>,
    options: &[NetworkSocketOptionSpec],
) -> Result<PendingNetworkConnectStart, Flow> {
    let mut last_error_code = libc::ECONNREFUSED;
    let mut iter = addrs.into_iter();
    while let Some(addr) = iter.next() {
        match start_nonblocking_tcp_stream_socket(addr, options)? {
            Ok(stream) => {
                return Ok(PendingNetworkConnectStart::Started(
                    PendingNetworkConnectStarted {
                        stream,
                        remote_addr: addr,
                        remaining_addrs: iter.collect(),
                    },
                ));
            }
            Err(err) => {
                last_error_code = io_error_status_code(&err);
            }
        }
    }
    Ok(PendingNetworkConnectStart::Failed(last_error_code))
}

fn bind_udp_socket(
    addr: SocketAddr,
    options: &[NetworkSocketOptionSpec],
) -> Result<UdpSocket, Flow> {
    let socket = Socket::new(tcp_socket_domain(addr), Type::DGRAM, Some(Protocol::UDP))
        .map_err(|err| network_socket_io_error("Cannot create datagram socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::from(addr);
    socket
        .bind(&sock_addr)
        .map_err(|err| network_socket_io_error("Cannot bind datagram socket", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

fn udp_unspecified_addr_for(remote: SocketAddr) -> SocketAddr {
    match remote {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}

fn datagram_zero_address_for(addr: SocketAddr) -> Value {
    let family_len = std::mem::size_of::<libc::sa_family_t>();
    let raw_len = match addr {
        SocketAddr::V4(_) => std::mem::size_of::<libc::sockaddr_in>() - family_len,
        SocketAddr::V6(_) => std::mem::size_of::<libc::sockaddr_in6>() - family_len,
    };
    Value::cons(Value::fixnum(0), int_vector(&vec![0_i64; raw_len]))
}

#[cfg(unix)]
fn datagram_zero_unix_address() -> Value {
    let raw_len =
        std::mem::size_of::<libc::sockaddr_un>() - std::mem::size_of::<libc::sa_family_t>();
    Value::cons(Value::fixnum(0), int_vector(&vec![0_i64; raw_len]))
}

fn bind_udp_client_socket(
    remote: SocketAddr,
    options: &[NetworkSocketOptionSpec],
) -> Result<UdpSocket, Flow> {
    bind_udp_socket(udp_unspecified_addr_for(remote), options)
}

fn resolve_tcp_socket_addrs(
    host: &str,
    port: u16,
    family: Option<NetworkProcessFamilySymbol>,
    operation: &str,
) -> Result<Vec<SocketAddr>, Flow> {
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|err| network_socket_io_error(operation, err))?;
    let addrs: Vec<_> = addrs
        .filter(|addr| match family {
            Some(NetworkProcessFamilySymbol::Ipv4) => addr.is_ipv4(),
            Some(NetworkProcessFamilySymbol::Ipv6) => addr.is_ipv6(),
            _ => true,
        })
        .collect();
    if addrs.is_empty() {
        Err(signal(
            "file-error",
            vec![
                Value::string(operation),
                Value::string("No address associated with hostname"),
            ],
        ))
    } else {
        Ok(addrs)
    }
}

fn bind_udp_socket_host(
    host: &str,
    port: u16,
    family: Option<NetworkProcessFamilySymbol>,
    options: &[NetworkSocketOptionSpec],
) -> Result<UdpSocket, Flow> {
    let mut last_error = None;
    for addr in resolve_tcp_socket_addrs(host, port, family, "Cannot bind datagram socket")? {
        match bind_udp_socket(addr, options) {
            Ok(socket) => return Ok(socket),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        signal(
            "file-error",
            vec![Value::string("Cannot bind datagram socket")],
        )
    }))
}

fn connect_udp_socket_host(
    host: &str,
    port: u16,
    family: Option<NetworkProcessFamilySymbol>,
    options: &[NetworkSocketOptionSpec],
) -> Result<(UdpSocket, SocketAddr), Flow> {
    let mut last_error = None;
    for addr in resolve_tcp_socket_addrs(host, port, family, "make datagram process failed")? {
        match bind_udp_client_socket(addr, options) {
            Ok(socket) => return Ok((socket, addr)),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        signal(
            "file-error",
            vec![Value::string("make datagram process failed")],
        )
    }))
}

fn bind_tcp_listener_host(
    host: &str,
    port: u16,
    family: Option<NetworkProcessFamilySymbol>,
    backlog: i32,
    options: &[NetworkSocketOptionSpec],
) -> Result<TcpListener, Flow> {
    let mut last_error = None;
    for addr in resolve_tcp_socket_addrs(host, port, family, "Cannot bind server socket")? {
        match bind_tcp_listener_socket(addr, backlog, options) {
            Ok(listener) => return Ok(listener),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        signal(
            "file-error",
            vec![Value::string("Cannot bind server socket")],
        )
    }))
}

fn connect_tcp_stream_host(
    host: &str,
    port: u16,
    family: Option<NetworkProcessFamilySymbol>,
    options: &[NetworkSocketOptionSpec],
) -> Result<TcpStream, Flow> {
    let mut last_error = None;
    for addr in resolve_tcp_socket_addrs(host, port, family, "make client process failed")? {
        match connect_tcp_stream_socket(addr, options) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        signal(
            "file-error",
            vec![Value::string("make client process failed")],
        )
    }))
}

fn tcp_server_socket_options(options: &[NetworkSocketOptionSpec]) -> Vec<NetworkSocketOptionSpec> {
    let mut effective = options.to_vec();
    if !network_socket_options_include(&effective, NetworkSocketOption::Reuseaddr) {
        effective.push(NetworkSocketOptionSpec {
            keyword: ProcessKeyword::Reuseaddr,
            option: NetworkSocketOption::Reuseaddr,
            value: Value::T,
        });
    }
    effective
}

#[cfg(unix)]
fn bind_unix_listener_socket(
    path: &Path,
    backlog: i32,
    options: &[NetworkSocketOptionSpec],
) -> Result<UnixListener, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None)
        .map_err(|err| network_socket_io_error("Cannot create server socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::unix(path)
        .map_err(|err| network_socket_io_error("Cannot bind server socket", err))?;
    socket
        .bind(&sock_addr)
        .map_err(|err| network_socket_io_error("Cannot bind server socket", err))?;
    socket
        .listen(backlog)
        .map_err(|err| network_socket_io_error("Cannot listen on server socket", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

#[cfg(unix)]
fn connect_unix_stream_socket(
    path: &Path,
    options: &[NetworkSocketOptionSpec],
) -> Result<UnixStream, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None)
        .map_err(|err| network_socket_io_error("Cannot create client socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::unix(path)
        .map_err(|err| network_socket_io_error("make client process failed", err))?;
    socket
        .connect(&sock_addr)
        .map_err(|err| network_socket_io_error("make client process failed", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

#[cfg(unix)]
fn start_nonblocking_unix_stream_socket(
    path: &Path,
    options: &[NetworkSocketOptionSpec],
) -> Result<Result<UnixStream, std::io::Error>, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None)
        .map_err(|err| network_socket_io_error("Cannot create client socket", err))?;
    apply_network_socket_options(&socket, options)?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    let sock_addr = SockAddr::unix(path)
        .map_err(|err| network_socket_io_error("make client process failed", err))?;
    match socket.connect(&sock_addr) {
        Ok(()) => Ok(Ok(socket.into())),
        Err(err) if nonblocking_connect_is_pending(&err) => Ok(Ok(socket.into())),
        Err(err) => Ok(Err(err)),
    }
}

#[cfg(unix)]
fn bind_unix_seqpacket_listener_socket(
    path: &Path,
    backlog: i32,
    options: &[NetworkSocketOptionSpec],
) -> Result<Socket, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::SEQPACKET, None)
        .map_err(|err| network_socket_io_error("Cannot create server socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::unix(path)
        .map_err(|err| network_socket_io_error("Cannot bind server socket", err))?;
    socket
        .bind(&sock_addr)
        .map_err(|err| network_socket_io_error("Cannot bind server socket", err))?;
    socket
        .listen(backlog)
        .map_err(|err| network_socket_io_error("Cannot listen on server socket", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket)
}

#[cfg(unix)]
fn connect_unix_seqpacket_socket(
    path: &Path,
    options: &[NetworkSocketOptionSpec],
) -> Result<Socket, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::SEQPACKET, None)
        .map_err(|err| network_socket_io_error("Cannot create client socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::unix(path)
        .map_err(|err| network_socket_io_error("make client process failed", err))?;
    socket
        .connect(&sock_addr)
        .map_err(|err| network_socket_io_error("make client process failed", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket)
}

#[cfg(unix)]
fn bind_unix_datagram_socket(
    path: &Path,
    options: &[NetworkSocketOptionSpec],
) -> Result<UnixDatagram, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::DGRAM, None)
        .map_err(|err| network_socket_io_error("Cannot create datagram socket", err))?;
    apply_network_socket_options(&socket, options)?;
    let sock_addr = SockAddr::unix(path)
        .map_err(|err| network_socket_io_error("Cannot bind datagram socket", err))?;
    socket
        .bind(&sock_addr)
        .map_err(|err| network_socket_io_error("Cannot bind datagram socket", err))?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

#[cfg(unix)]
fn unbound_unix_datagram_socket(options: &[NetworkSocketOptionSpec]) -> Result<UnixDatagram, Flow> {
    let socket = Socket::new(Domain::UNIX, Type::DGRAM, None)
        .map_err(|err| network_socket_io_error("Cannot create datagram socket", err))?;
    apply_network_socket_options(&socket, options)?;
    socket
        .set_nonblocking(true)
        .map_err(|err| network_socket_io_error("set_nonblocking", err))?;
    Ok(socket.into())
}

impl ProcessManager {
    fn register_readable_source(
        poller: &polling::Poller,
        source: impl polling::AsRawSource,
        id: ProcessId,
    ) -> Result<(), String> {
        // SAFETY: ProcessManager only registers descriptors owned by the
        // corresponding Process record.  `unregister_process_poll_sources`
        // removes every registered descriptor from this poller before the
        // Process drops or replaces the descriptor.
        unsafe {
            poller
                .add_with_mode(
                    source,
                    polling::Event::readable(id as usize),
                    polling::PollMode::Level,
                )
                .map_err(|e| format!("Failed to register socket: {e}"))
        }
    }

    fn register_writable_source(
        poller: &polling::Poller,
        source: impl polling::AsRawSource,
        id: ProcessId,
    ) -> Result<(), String> {
        // SAFETY: ProcessManager only registers descriptors owned by the
        // corresponding Process record.  `unregister_process_poll_sources`
        // removes every registered descriptor from this poller before the
        // Process drops or replaces the descriptor.
        unsafe {
            poller
                .add_with_mode(
                    source,
                    polling::Event::writable(id as usize),
                    polling::PollMode::Level,
                )
                .map_err(|e| format!("Failed to register socket: {e}"))
        }
    }

    #[cfg(unix)]
    fn register_readable_raw_fd(
        poller: &polling::Poller,
        fd: std::os::unix::io::RawFd,
        id: ProcessId,
    ) -> Result<(), String> {
        // SAFETY: `fd` is borrowed from a process-owned descriptor that
        // remains alive until `unregister_process_poll_sources` removes it
        // from the poller.
        let borrowed = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(fd) };
        Self::register_readable_source(poller, &borrowed, id)
    }

    #[cfg(unix)]
    fn register_child_stdout_with_poller(
        poller: &polling::Poller,
        stdout: &std::process::ChildStdout,
        id: ProcessId,
    ) {
        use std::os::unix::io::AsRawFd;
        let fd = stdout.as_raw_fd();
        // Set non-blocking before registering.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        // Use process id as the event key so we know which process is ready.
        let _ = Self::register_readable_raw_fd(poller, fd, id);
    }

    #[cfg(not(unix))]
    fn register_child_stdout_with_poller(
        _poller: &polling::Poller,
        _stdout: &std::process::ChildStdout,
        _id: ProcessId,
    ) {
        // GNU Emacs does not pass Windows subprocess pipe handles to Winsock
        // select.  Its w32 layer uses a reader thread plus event objects.  Until
        // Neomacs has the same backend, child pipe output is serviced by the
        // regular non-blocking wait pass instead of the socket poller.
    }

    #[cfg(unix)]
    fn unregister_child_stdout_from_poller(
        poller: &polling::Poller,
        stdout: &std::process::ChildStdout,
    ) {
        use std::os::unix::io::AsRawFd;
        let fd = stdout.as_raw_fd();
        let borrowed = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(fd) };
        let _ = poller.delete(&borrowed);
    }

    #[cfg(not(unix))]
    fn unregister_child_stdout_from_poller(
        _poller: &polling::Poller,
        _stdout: &std::process::ChildStdout,
    ) {
        // See `register_child_stdout_with_poller`.
    }

    #[cfg(unix)]
    fn register_child_stderr_with_poller(
        poller: &polling::Poller,
        stderr: &std::process::ChildStderr,
        id: ProcessId,
    ) {
        use std::os::unix::io::AsRawFd;
        let fd = stderr.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        let _ = Self::register_readable_raw_fd(poller, fd, id);
    }

    #[cfg(not(unix))]
    fn register_child_stderr_with_poller(
        _poller: &polling::Poller,
        _stderr: &std::process::ChildStderr,
        _id: ProcessId,
    ) {
        // See `register_child_stdout_with_poller`.
    }

    #[cfg(unix)]
    fn unregister_child_stderr_from_poller(
        poller: &polling::Poller,
        stderr: &std::process::ChildStderr,
    ) {
        use std::os::unix::io::AsRawFd;
        let fd = stderr.as_raw_fd();
        let borrowed = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(fd) };
        let _ = poller.delete(&borrowed);
    }

    #[cfg(not(unix))]
    fn unregister_child_stderr_from_poller(
        _poller: &polling::Poller,
        _stderr: &std::process::ChildStderr,
    ) {
        // See `register_child_stdout_with_poller`.
    }

    fn unregister_process_poll_sources(poller: Option<&polling::Poller>, proc: &Process) {
        let Some(poller) = poller else {
            return;
        };

        if let Some(stdout) = proc.child_stdout.as_ref() {
            Self::unregister_child_stdout_from_poller(poller, stdout);
        }
        if let Some(stderr) = proc.child_stderr.as_ref() {
            Self::unregister_child_stderr_from_poller(poller, stderr);
        }
        if let Some(tls) = proc.tls_stream.as_ref() {
            let _ = poller.delete(tls.tcp_stream());
        }
        if let Some(socket) = proc.network_socket.as_ref() {
            socket.unregister_readable(poller);
        }
        #[cfg(unix)]
        if let Some(master) = proc
            .pty_master
            .as_ref()
            .and_then(|master| master.as_raw_fd())
        {
            let borrowed = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(master) };
            let _ = poller.delete(&borrowed);
        }
    }

    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            deleted_processes: HashMap::new(),
            next_id: 1,
            env_overrides: HashMap::new(),
            wait_backend: ProcessWaitBackend::new(),
        }
    }

    /// Create a new process record.  Returns the process id.
    pub fn create_process(
        &mut self,
        name: String,
        buffer: Value,
        command: String,
        args: Vec<String>,
    ) -> ProcessId {
        self.create_process_lisp(
            LispString::from_utf8(&name),
            buffer,
            LispString::from_utf8(&command),
            args.into_iter()
                .map(|arg| LispString::from_utf8(&arg))
                .collect(),
        )
    }

    pub fn create_process_lisp(
        &mut self,
        name: LispString,
        buffer: Value,
        command: LispString,
        args: Vec<LispString>,
    ) -> ProcessId {
        self.create_process_with_kind_lisp(name, buffer, command, args, ProcessKind::Real)
    }

    pub fn create_process_lisp_resolved(
        &mut self,
        name: LispString,
        buffer: Value,
        command: LispString,
        args: Vec<LispString>,
        executable: Option<LispString>,
    ) -> ProcessId {
        self.create_process_with_kind_lisp_resolved(
            name,
            buffer,
            command,
            args,
            ProcessKind::Real,
            executable,
        )
    }

    /// Create a new process record with an explicit process kind.
    pub fn create_process_with_kind(
        &mut self,
        name: String,
        buffer: Value,
        command: String,
        args: Vec<String>,
        kind: ProcessKind,
    ) -> ProcessId {
        self.create_process_with_kind_lisp(
            LispString::from_utf8(&name),
            buffer,
            LispString::from_utf8(&command),
            args.into_iter()
                .map(|arg| LispString::from_utf8(&arg))
                .collect(),
            kind,
        )
    }

    pub fn create_process_with_kind_lisp(
        &mut self,
        name: LispString,
        buffer: Value,
        command: LispString,
        args: Vec<LispString>,
        kind: ProcessKind,
    ) -> ProcessId {
        self.create_process_with_kind_lisp_resolved(name, buffer, command, args, kind, None)
    }

    pub fn create_process_with_kind_lisp_resolved(
        &mut self,
        name: LispString,
        buffer: Value,
        command: LispString,
        args: Vec<LispString>,
        kind: ProcessKind,
        executable: Option<LispString>,
    ) -> ProcessId {
        let id = self.next_id;
        self.next_id += 1;
        let (tty_name, tty_stdin, tty_stdout, tty_stderr) = match kind {
            ProcessKind::Real => {
                let tty_name = Value::string(default_process_tty_name());
                (tty_name, true, true, true)
            }
            ProcessKind::Network | ProcessKind::Pipe | ProcessKind::Serial => {
                (Value::NIL, false, false, false)
            }
        };
        let proc_type = process_type_value(&kind);
        let childp = if kind == ProcessKind::Real {
            Value::T
        } else {
            Value::NIL
        };
        let proc = Process {
            id,
            name: process_name_lisp_value(&name),
            command: make_process_command_lisp_value(&kind, &command, &args),
            executable,
            kind,
            proc_type,
            status: process_status_run_value(),
            buffer,
            childp,
            write_queue: Value::NIL,
            stdout: String::new(),
            stderr: String::new(),
            query_on_exit_flag: true,
            filter: Value::symbol(DEFAULT_PROCESS_FILTER_SYMBOL),
            sentinel: Value::symbol(DEFAULT_PROCESS_SENTINEL_SYMBOL),
            log: Value::NIL,
            plist: Value::NIL,
            stderrproc: Value::NIL,
            coding_decode: Value::symbol("utf-8-unix"),
            coding_encode: Value::symbol("utf-8-unix"),
            inherit_coding_system_flag: false,
            thread: Value::NIL,
            window_cols: None,
            window_rows: None,
            tty_name,
            tty_stdin,
            tty_stdout,
            tty_stderr,
            os_pid: None,
            child: None,
            child_stdout: None,
            child_stderr: None,
            pty_master: None,
            pty_child: None,
            pty_reader: None,
            pty_writer: None,
            datagram_address: Value::NIL,
            datagram_socket_addr: None,
            #[cfg(unix)]
            datagram_unix_path: None,
            network_socket: None,
            pending_network_connect: None,
            tls_stream: None,
            gnutls_initstage: GnutlsInitStage::Empty,
            gnutls_boot_parameters: Value::NIL,
            mark: super::marker::make_marker_value(None, None, false),
            default_directory: None,
        };
        register_process_print_name(id, &process_name_runtime(proc.name));
        self.processes.insert(id, proc);
        id
    }

    pub fn sync_process_mark(&mut self, buffers: &mut BufferManager, id: ProcessId) -> EvalResult {
        let proc = self
            .get_mut(id)
            .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;
        update_process_mark(buffers, proc)
    }

    /// Spawn an OS child process for a tracked process record.
    ///
    /// When `use_pty` is true (and on Unix), the child is spawned on a
    /// pseudo-terminal via `portable-pty`. Otherwise the traditional
    /// pipe-based `std::process::Command` path is used.
    pub fn spawn_child(&mut self, id: ProcessId, use_pty: bool) -> Result<(), String> {
        self.spawn_child_with_environment(id, use_pty, None)
    }

    pub fn spawn_child_with_environment(
        &mut self,
        id: ProcessId,
        use_pty: bool,
        process_environment: Option<Value>,
    ) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(&id)
            .ok_or_else(|| "Process not found".to_string())?;

        if proc.child.is_some() || proc.pty_child.is_some() {
            return Ok(()); // Already spawned
        }

        // Don't spawn non-real processes
        if proc.kind != ProcessKind::Real {
            return Ok(());
        }

        let Some(argv) = process_spawn_lisp_argv(proc) else {
            return Ok(()); // No program to run
        };
        if argv.is_empty()
            || argv[0].as_bytes().is_empty()
            || env_var_name_bytes_eq(argv[0].as_bytes(), b"nil")
        {
            return Ok(());
        }

        // Collect env overrides into a temporary Vec so we don't borrow
        // `self` across the mutable `proc` borrow below.
        let env_overrides: Vec<(LispString, Option<LispString>)> = self
            .env_overrides
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // PTY path (Unix only).
        #[cfg(unix)]
        if use_pty {
            return self.spawn_child_pty(id, process_environment, &env_overrides);
        }

        // Pipe path (all platforms, or when use_pty is false).
        self.spawn_child_pipe(id, process_environment, &env_overrides)
    }

    /// Pipe-based child spawn (traditional stdin/stdout/stderr pipes).
    fn spawn_child_pipe(
        &mut self,
        id: ProcessId,
        process_environment: Option<Value>,
        env_overrides: &[(LispString, Option<LispString>)],
    ) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(&id)
            .ok_or_else(|| "Process not found".to_string())?;

        let Some(argv) = process_spawn_lisp_argv(proc) else {
            return Ok(());
        };
        if argv.is_empty() {
            return Ok(());
        }

        // GNU's `create_process` :stderr path: a separate stderr pipe-process
        // captures the child's stderr stream.  Its read end is parked in the
        // stderr pipe-process's `child_stderr` slot after spawn; the main
        // process keeps stdout on its own buffer.  When there is no stderr
        // pipe-process the child's stderr is captured on the main process
        // record (current behaviour — merged conceptually with stdout).
        let stderr_pipe_id = process_value_to_id(&proc.stderrproc);

        let argv_os = argv
            .iter()
            .map(lisp_string_to_os_string)
            .collect::<Vec<OsString>>();

        let mut cmd = crate::emacs_core::callproc::new_child_command(&argv_os[0]);
        cmd.args(&argv_os[1..]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        if let Some(dir) = &proc.default_directory {
            cmd.current_dir(dir);
        }

        if let Some(entries) = process_environment_entries(process_environment) {
            cmd.env_clear();
            for (key, value) in entries {
                if let Some(value) = value {
                    cmd.env(&key, &value);
                } else {
                    cmd.env_remove(&key);
                }
            }
        }

        for (key, val) in env_overrides {
            let key_str = lisp_string_to_os_string(key);
            match val {
                Some(v) => {
                    let v_str = lisp_string_to_os_string(v);
                    cmd.env(&key_str, &v_str);
                }
                None => {
                    cmd.env_remove(&key_str);
                }
            }
        }

        let spawned = cmd.spawn();
        // End the `proc` borrow before touching other process records (the
        // stderr pipe-process) and the poller.
        let _ = proc;

        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                if let Some(proc) = self.processes.get_mut(&id) {
                    proc.status = process_status_exit_value(1);
                }
                return Err(format!("Failed to start process: {}", e));
            }
        };

        // GNU records the child's real OS pid (create_process sets
        // p->pid = pid). `std::process::Child::id` exposes it as a `u32`.
        let os_pid = Some(child.id());

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Register stdout with the poller where the platform exposes child
        // pipe descriptors as pollable sources.
        if let (Some(poller), Some(stdout)) = (self.wait_backend.poller(), &stdout) {
            Self::register_child_stdout_with_poller(poller, stdout, id);
        }

        if let Some(proc) = self.processes.get_mut(&id) {
            proc.child_stdout = stdout;
            proc.os_pid = os_pid;
            proc.child = Some(child);
            proc.status = process_status_run_value();
            // Pipe-mode processes don't have a real TTY.
            proc.tty_name = Value::NIL;
            proc.tty_stdin = false;
            proc.tty_stdout = false;
            proc.tty_stderr = false;
        }

        // Route the child's stderr.  With a separate stderr pipe-process
        // (make-process :stderr), the read end goes to that process's record
        // and is polled under its id, mirroring GNU's create_process which
        // connects the child's stderr fd to the stderr pipe-process's
        // READ_FROM_SUBPROCESS.  Otherwise it stays on the main process record.
        self.route_child_stderr_to_pipe_process(id, stderr_pipe_id, stderr);
        Ok(())
    }

    /// Wire a freshly spawned child's stderr handle into the stderr
    /// pipe-process record (`make-process :stderr`), mirroring GNU's
    /// `create_process`: the child's stderr fd (`forkerr`) is taken from the
    /// stderr pipe-process and that process reads the data into its own buffer,
    /// independently of the main process's stdout (`callproc.c` `emacs_spawn`
    /// uses the separate `forkerr` whenever `p->stderrproc` is non-nil, and
    /// `process.c:2231-2240` takes `forkerr` from the stderr pipe-process).
    ///
    /// `stderr_pipe_id` is the id resolved from the main process's `stderrproc`
    /// slot (None when there is no `:stderr`).  This is shared by both the pipe
    /// and PTY spawn paths so that stdout may use a PTY while stderr stays on a
    /// dedicated pipe, exactly as GNU does (the PTY-for-stdout decision is
    /// independent of the `:stderr` pipe — see `99bd67887`).
    fn route_child_stderr_to_pipe_process(
        &mut self,
        main_id: ProcessId,
        stderr_pipe_id: Option<ProcessId>,
        stderr: Option<std::process::ChildStderr>,
    ) {
        let stderr_target = stderr_pipe_id.filter(|sid| {
            *sid != main_id
                && matches!(
                    self.processes.get(sid).map(|p| p.kind),
                    Some(ProcessKind::Pipe)
                )
        });
        match stderr_target {
            Some(stderr_id) => {
                if let (Some(poller), Some(stderr)) = (self.wait_backend.poller(), &stderr) {
                    Self::register_child_stderr_with_poller(poller, stderr, stderr_id);
                }
                if let Some(stderr_proc) = self.processes.get_mut(&stderr_id) {
                    stderr_proc.child_stderr = stderr;
                    stderr_proc.status = process_status_run_value();
                }
            }
            None => {
                // No separate stderr pipe-process: drop the child's stderr
                // handle.  Parking it on the main (Real) process record would
                // make it look like a stderr pipe-process and would also be
                // surfaced as a readable source; neither is wanted here.  (GNU
                // merges stderr into the same stdout pipe in this case; that
                // pre-existing merge limitation is out of scope.)
                drop(stderr);
            }
        }
    }

    /// PTY-based child spawn via `portable-pty`.
    ///
    /// The child is attached to a pseudo-terminal. The master side provides
    /// a single combined I/O stream (PTY merges stdout and stderr) — UNLESS a
    /// separate stderr pipe-process is requested (`make-process :stderr`), in
    /// which case stdout stays on the PTY but stderr is routed to a dedicated
    /// pipe, exactly as GNU's `create_process` wires `forkin`/`forkout` to the
    /// pty and `forkerr` to the stderr pipe-process independently.
    #[cfg(unix)]
    fn spawn_child_pty(
        &mut self,
        id: ProcessId,
        process_environment: Option<Value>,
        env_overrides: &[(LispString, Option<LispString>)],
    ) -> Result<(), String> {
        let proc = self
            .processes
            .get_mut(&id)
            .ok_or_else(|| "Process not found".to_string())?;

        let rows = proc.window_rows.unwrap_or(24) as u16;
        let cols = proc.window_cols.unwrap_or(80) as u16;
        let stderrproc = proc.stderrproc;
        let default_directory = proc.default_directory.clone();
        let argv = process_spawn_lisp_argv(proc);
        // Release the `proc` borrow: the rest of this function reads other
        // process records (the stderr pipe-process) and re-borrows `id`.
        let _ = proc;

        // A separate stderr pipe-process (make-process :stderr) is wired here as
        // GNU does: stdout uses the PTY, stderr uses an independent pipe.  When
        // none is requested the PTY merges stdout and stderr as before.
        let stderr_pipe_id = process_value_to_id(&stderrproc).filter(|sid| {
            *sid != id
                && matches!(
                    self.processes.get(sid).map(|p| p.kind),
                    Some(ProcessKind::Pipe)
                )
        });

        let pty_system = portable_pty::native_pty_system();
        let pty_size = portable_pty::PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pty_pair = pty_system
            .openpty(pty_size)
            .map_err(|e| format!("Failed to create PTY: {}", e))?;

        let Some(argv) = argv else {
            return Ok(());
        };
        if argv.is_empty() {
            return Ok(());
        }

        let argv_os = argv
            .iter()
            .map(lisp_string_to_os_string)
            .collect::<Vec<OsString>>();

        // Obtain the TTY name from the master (which knows the slave path).
        let tty_name_path = pty_pair.master.tty_name();
        let tty_name = tty_name_path
            .as_ref()
            .map(|p| Value::heap_string(os_str_to_lisp_string(p.as_os_str())))
            .unwrap_or(Value::NIL);
        if let Some(tty_path) = tty_name_path.as_ref() {
            configure_child_pty_tty(tty_path.as_os_str())
                .map_err(|e| format!("Failed to configure PTY child tty: {e}"))?;
        }

        // With a separate stderr pipe-process we cannot use portable_pty's
        // `spawn_command` (it hardwires the child's stdin/stdout/stderr all to
        // the PTY slave).  Instead spawn the child ourselves, dup'ing the PTY
        // slave onto stdin/stdout and leaving stderr on an OS pipe, mirroring
        // GNU's `emacs_spawn` where `std_err` is the separate `forkerr` fd and
        // only merges into `std_out` when no stderr pipe-process exists.
        let stderr_routing = if let Some(stderr_id) = stderr_pipe_id {
            let tty_path = tty_name_path
                .clone()
                .ok_or_else(|| "PTY has no tty name for :stderr split spawn".to_string())?;
            let mut cmd = crate::emacs_core::callproc::new_child_command(&argv_os[0]);
            cmd.args(&argv_os[1..]);
            cmd.stderr(Stdio::piped());
            if let Some(dir) = &default_directory {
                cmd.current_dir(dir);
            }
            if let Some(entries) = process_environment_entries(process_environment) {
                cmd.env_clear();
                for (key, value) in entries {
                    match value {
                        Some(value) => {
                            cmd.env(&key, &value);
                        }
                        None => {
                            cmd.env_remove(&key);
                        }
                    }
                }
            }
            for (key, val) in env_overrides {
                let key_str = lisp_string_to_os_string(key);
                match val {
                    Some(v) => {
                        cmd.env(&key_str, lisp_string_to_os_string(v));
                    }
                    None => {
                        cmd.env_remove(&key_str);
                    }
                }
            }
            // `new_child_command` already installs a `pre_exec` that calls
            // `setsid` (own session, no controlling tty).  Chain a second
            // `pre_exec` that opens the PTY slave by path and makes it the
            // controlling terminal on fds 0/1, leaving fd 2 (stderr) on the
            // pipe `Command` set up — exactly GNU's forkin/forkout=pty_tty,
            // forkerr=stderr-pipe arrangement.
            let tty_cstr = std::ffi::CString::new(tty_path.as_os_str().as_bytes())
                .map_err(|_| "PTY tty name contains an interior NUL".to_string())?;
            unsafe {
                use std::os::unix::process::CommandExt;
                cmd.pre_exec(move || {
                    let slave = libc::open(tty_cstr.as_ptr(), libc::O_RDWR | libc::O_NOCTTY);
                    if slave < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    // Make the pty our controlling terminal (setsid already ran
                    // in the first pre_exec, so we are a session leader with no
                    // controlling tty).
                    #[allow(clippy::cast_lossless)]
                    if libc::ioctl(slave, libc::TIOCSCTTY as _, 0) == -1 {
                        let err = std::io::Error::last_os_error();
                        libc::close(slave);
                        return Err(err);
                    }
                    if libc::dup2(slave, libc::STDIN_FILENO) == -1
                        || libc::dup2(slave, libc::STDOUT_FILENO) == -1
                    {
                        let err = std::io::Error::last_os_error();
                        libc::close(slave);
                        return Err(err);
                    }
                    if slave > libc::STDERR_FILENO {
                        libc::close(slave);
                    }
                    Ok(())
                });
            }

            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to spawn PTY child: {}", e))?;
            // GNU records the child's real OS pid (create_process sets p->pid).
            let os_pid = Some(child.id());
            let child_stderr = child.stderr.take();
            if let Some(proc) = self.processes.get_mut(&id) {
                proc.os_pid = os_pid;
                proc.child = Some(child);
            }
            (Some(stderr_id), child_stderr)
        } else {
            let mut cmd = portable_pty::CommandBuilder::from_argv(argv_os);
            if let Some(dir) = &default_directory {
                cmd.cwd(dir);
            }
            if let Some(entries) = process_environment_entries(process_environment) {
                cmd.env_clear();
                for (key, value) in entries {
                    if let Some(value) = value {
                        cmd.env(&key, &value);
                    } else {
                        cmd.env_remove(&key);
                    }
                }
            }
            for (key, val) in env_overrides {
                let key_str = lisp_string_to_os_string(key);
                match val {
                    Some(v) => {
                        let v_str = lisp_string_to_os_string(v);
                        cmd.env(&key_str, &v_str);
                    }
                    None => {
                        cmd.env_remove(&key_str);
                    }
                }
            }

            let pty_child = pty_pair
                .slave
                .spawn_command(cmd)
                .map_err(|e| format!("Failed to spawn PTY child: {}", e))?;
            // GNU records the child's real OS pid; portable_pty exposes it via
            // `Child::process_id`.
            let os_pid = pty_child.process_id();
            if let Some(proc) = self.processes.get_mut(&id) {
                proc.os_pid = os_pid;
                proc.pty_child = Some(pty_child);
            }
            (None, None)
        };
        let (stderr_pipe_id, child_stderr) = stderr_routing;

        // Drop the slave end now that the child has it; otherwise the master
        // read never sees EOF after the child exits.
        drop(pty_pair.slave);

        let pty_read = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;
        let pty_write = pty_pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

        // Register the PTY master fd with the poller for non-blocking I/O.
        if let Some(master_fd) = pty_pair.master.as_raw_fd() {
            // Set non-blocking on the master fd.
            unsafe {
                let flags = libc::fcntl(master_fd, libc::F_GETFL);
                libc::fcntl(master_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
            if let Some(poller) = self.wait_backend.poller() {
                let _ = Self::register_readable_raw_fd(poller, master_fd, id);
            }
        }

        if let Some(proc) = self.processes.get_mut(&id) {
            proc.pty_master = Some(pty_pair.master);
            proc.pty_reader = Some(pty_read);
            proc.pty_writer = Some(Box::new(pty_write));
            proc.status = process_status_run_value();
            proc.tty_name = tty_name;
            proc.tty_stdin = true;
            proc.tty_stdout = true;
            // stderr is tty-backed only when it shares the PTY; with a separate
            // stderr pipe-process it is not (GNU's `Fprocess_tty_name` returns
            // nil for the stderr stream when `p->stderrproc` is set).
            proc.tty_stderr = stderr_pipe_id.is_none();
        }

        // Route the child's stderr to the stderr pipe-process record, shared
        // with the pipe spawn path (GNU `create_process` forkerr wiring).
        self.route_child_stderr_to_pipe_process(id, stderr_pipe_id, child_stderr);
        Ok(())
    }

    /// Check if a child process has exited and update its status.
    /// Returns true if the process exited (status changed).
    pub fn check_child_exit(&mut self, id: ProcessId) -> bool {
        let proc = match self.processes.get_mut(&id) {
            Some(p) => p,
            None => return false,
        };

        if !process_status_is_run(&proc.status) {
            return false;
        }

        // PTY child path.
        if let Some(ref mut pty_child) = proc.pty_child {
            match pty_child.try_wait() {
                Ok(Some(status)) => {
                    // Preserve the real exit code and signal-death status, as GNU
                    // does (status_notify decodes WIFSIGNALED/WEXITSTATUS); the
                    // previous `success ? 0 : 1` collapsed every failure to 1.
                    proc.status = process_status_from_pty_exit(&status);
                    return true;
                }
                Ok(None) => return false,
                Err(_) => {
                    proc.status = process_status_exit_value(1);
                    return true;
                }
            }
        }

        // Pipe child path.
        let child = match proc.child.as_mut() {
            Some(c) => c,
            None => return false,
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                proc.status = process_status_from_exit(&status);
                true
            }
            Ok(None) => false, // Still running
            Err(_) => {
                proc.status = process_status_exit_value(1);
                true
            }
        }
    }

    /// Read available output from a child process's stdout.
    /// Returns the data read (may be empty if nothing available).
    fn read_child_stdout_result(&mut self, id: ProcessId) -> ProcessOutputRead {
        let Some(proc) = self.processes.get_mut(&id) else {
            return ProcessOutputRead::NoSource;
        };
        let Some(stdout) = proc.child_stdout.as_mut() else {
            return ProcessOutputRead::NoSource;
        };

        // Use non-blocking read via set_nonblocking on Unix
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = stdout.as_raw_fd();
            // Set non-blocking
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        let mut buf = vec![0u8; 4096];
        let read =
            ProcessOutputRead::from_io_result(stdout.read(&mut buf), &buf, proc.coding_decode);
        if let ProcessOutputRead::Data(ref data) = read {
            proc.stdout.push_str(&process_output_runtime_string(data));
        }
        read
    }

    /// Read available output from a stderr pipe-process's child stderr fd.
    ///
    /// Mirrors GNU's `create_process` :stderr wiring: the stderr pipe-process
    /// reads from the child's separate stderr pipe.  The read end lives in this
    /// (the stderr pipe-process's) `child_stderr` slot.
    fn read_child_stderr_result(&mut self, id: ProcessId) -> ProcessOutputRead {
        let Some(proc) = self.processes.get_mut(&id) else {
            return ProcessOutputRead::NoSource;
        };
        let Some(stderr) = proc.child_stderr.as_mut() else {
            return ProcessOutputRead::NoSource;
        };

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = stderr.as_raw_fd();
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        let mut buf = vec![0u8; 4096];
        let read =
            ProcessOutputRead::from_io_result(stderr.read(&mut buf), &buf, proc.coding_decode);
        if let ProcessOutputRead::Data(ref data) = read {
            proc.stderr.push_str(&process_output_runtime_string(data));
        }
        read
    }

    /// Read available output from a PTY master reader.
    /// Returns the data read (may be empty if nothing available).
    /// PTY combines stdout and stderr into a single stream.
    fn read_pty_output_result(&mut self, id: ProcessId) -> ProcessOutputRead {
        let Some(proc) = self.processes.get_mut(&id) else {
            return ProcessOutputRead::NoSource;
        };
        let Some(reader) = proc.pty_reader.as_mut() else {
            return ProcessOutputRead::NoSource;
        };

        let mut buf = vec![0u8; 4096];
        let read =
            ProcessOutputRead::from_io_result(reader.read(&mut buf), &buf, proc.coding_decode);
        if let ProcessOutputRead::Data(ref data) = read {
            proc.stdout.push_str(&process_output_runtime_string(data));
        }
        read
    }

    fn read_network_output_result(&mut self, id: ProcessId) -> ProcessOutputRead {
        let Some(proc) = self.processes.get_mut(&id) else {
            return ProcessOutputRead::NoSource;
        };

        if let Some(ref mut tls) = proc.tls_stream {
            let mut buf = vec![0u8; 4096];
            let read = ProcessOutputRead::from_io_result(
                tls.read_process_output(&mut buf),
                &buf,
                proc.coding_decode,
            );
            if let ProcessOutputRead::Data(ref data) = read {
                proc.stdout.push_str(&process_output_runtime_string(data));
            }
            return read;
        }

        if let Some(socket) = proc.network_socket.as_mut() {
            let mut buf = vec![0u8; 4096];
            let read = match socket.read_stream_output(&mut buf) {
                Some(result) => ProcessOutputRead::from_io_result(result, &buf, proc.coding_decode),
                None => match socket {
                    NetworkSocket::UdpSocket(socket) => match socket.recv_from(&mut buf) {
                        Ok((n, addr)) => {
                            proc.datagram_socket_addr = Some(addr);
                            proc.datagram_address = socket_addr_to_lisp_value(addr);
                            ProcessOutputRead::Data(decode_process_output_bytes(
                                &buf[..n],
                                proc.coding_decode,
                            ))
                        }
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            ProcessOutputRead::WouldBlock
                        }
                        Err(_) => ProcessOutputRead::Eof,
                    },
                    #[cfg(unix)]
                    NetworkSocket::UnixDatagram(socket) => match socket.recv_from(&mut buf) {
                        Ok((n, addr)) => {
                            if let Some(path) = addr.as_pathname() {
                                let path = path.to_path_buf();
                                proc.datagram_unix_path = Some(path.clone());
                                proc.datagram_address = Value::heap_string(
                                    crate::emacs_core::fileio::path_to_lisp_file_name(&path),
                                );
                            }
                            ProcessOutputRead::Data(decode_process_output_bytes(
                                &buf[..n],
                                proc.coding_decode,
                            ))
                        }
                        Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            ProcessOutputRead::WouldBlock
                        }
                        Err(_) => ProcessOutputRead::Eof,
                    },
                    _ => ProcessOutputRead::WouldBlock,
                },
            };
            if let ProcessOutputRead::Data(ref data) = read {
                proc.stdout.push_str(&process_output_runtime_string(data));
            }
            return read;
        }

        ProcessOutputRead::NoSource
    }

    pub(crate) fn wait_for_process_events(
        &self,
        timeout: std::time::Duration,
    ) -> ProcessWaitEvents {
        if let Some(events) =
            self.wait_for_backend_events(timeout, ProcessWaitBackendInterest::ProcessesOnly)
        {
            return events;
        }

        // No poller available — sleep fallback
        std::thread::sleep(timeout.min(std::time::Duration::from_millis(10)));
        ProcessWaitEvents::ready_processes(self.live_process_ids())
    }

    #[cfg(unix)]
    pub(crate) fn register_wait_input_wakeup_fd(&mut self, fd: std::os::unix::io::RawFd) {
        self.wait_backend.register_input_wakeup_fd(fd);
    }

    #[cfg(not(unix))]
    pub(crate) fn register_wait_input_wakeup_fd(&mut self, fd: super::eval::WakeupFd) {
        self.wait_backend.register_input_wakeup_fd(fd);
    }

    pub(crate) fn has_wait_input_wakeup_backend(&self) -> bool {
        self.wait_backend.has_input_wakeup()
    }

    /// Cross-platform handle for the render/frontend thread to wake a blocked
    /// wait via `Poller::notify()` after delivering input. `None` if no poller
    /// could be created (e.g. headless/batch).
    pub(crate) fn wait_notifier(&self) -> Option<WaitNotifier> {
        self.wait_backend.notify_handle()
    }

    /// Block on the unified wait poller (input-wakeup fd and/or process fds,
    /// per `interest`) until something is ready or `timeout` elapses. This is
    /// the single GNU-`pselect`-style primitive the wait loop blocks on; see
    /// `Context::block_for_wait_request`.
    pub(crate) fn wait_for_backend_events(
        &self,
        timeout: std::time::Duration,
        interest: ProcessWaitBackendInterest,
    ) -> Option<ProcessWaitEvents> {
        self.wait_backend
            .wait_for_events(&self.processes, timeout, interest)
    }

    fn deactivate_network_process_io(&mut self, id: ProcessId) {
        if let Some(proc) = self.processes.get_mut(&id) {
            Self::unregister_process_poll_sources(self.wait_backend.poller(), proc);
            proc.tls_stream = None;
            proc.network_socket = None;
            proc.gnutls_initstage = GnutlsInitStage::Empty;
        }
    }

    /// Tear down a stderr pipe-process's readable I/O once its source EOFs.
    /// Removes the stderr fd from the poller and drops it so the descriptor is
    /// closed and the process stops being polled.
    fn deactivate_stderr_pipe_process_io(&mut self, id: ProcessId) {
        if let Some(proc) = self.processes.get_mut(&id) {
            Self::unregister_process_poll_sources(self.wait_backend.poller(), proc);
            proc.child_stderr = None;
        }
    }

    /// Kill (remove) a process by id.  Returns true if found.
    pub fn kill_process(&mut self, id: ProcessId) -> bool {
        if let Some(proc) = self.processes.get_mut(&id) {
            Self::unregister_process_poll_sources(self.wait_backend.poller(), proc);
            if let Some(child) = proc.child.as_mut() {
                let _ = child.kill();
            }
            if let Some(pty_child) = proc.pty_child.as_mut() {
                let _ = pty_child.kill();
            }
            proc.tls_stream.take();
            proc.gnutls_initstage = GnutlsInitStage::Empty;
            proc.gnutls_boot_parameters = Value::NIL;
            proc.network_socket.take();
            proc.status = process_status_signal_value(9);
            true
        } else {
            false
        }
    }

    /// Delete a process entirely.
    pub fn delete_process(&mut self, id: ProcessId) -> bool {
        if let Some(mut proc) = self.processes.remove(&id) {
            Self::unregister_process_poll_sources(self.wait_backend.poller(), &proc);
            if let Some(child) = proc.child.as_mut() {
                let _ = child.kill();
            }
            if let Some(pty_child) = proc.pty_child.as_mut() {
                let _ = pty_child.kill();
            }
            proc.tls_stream.take();
            proc.gnutls_initstage = GnutlsInitStage::Empty;
            proc.gnutls_boot_parameters = Value::NIL;
            proc.network_socket.take();
            proc.status = process_status_signal_value(9);
            self.deleted_processes.insert(id, proc);
            true
        } else {
            self.deleted_processes.contains_key(&id)
        }
    }

    /// GNU `remove_process` for an already-terminated process (called from
    /// `status_notify` when `delete-exited-processes' is non-nil): drop the
    /// process from the live process table (so `get-process'/`process-list' no
    /// longer return it) while keeping the object reachable for bindings that
    /// still hold its value.  Unlike `delete_process`, this does NOT kill or
    /// re-stamp the child — it has already exited and its recorded terminal
    /// status (exit/signal) must be preserved for `process-status' on the value.
    pub fn reap_exited_process(&mut self, id: ProcessId) {
        if let Some(mut proc) = self.processes.remove(&id) {
            Self::unregister_process_poll_sources(self.wait_backend.poller(), &proc);
            // Release OS resources the (already dead) process held; keep the
            // recorded status and identity intact.
            proc.child = None;
            proc.pty_child = None;
            proc.child_stdout = None;
            proc.child_stderr = None;
            self.deleted_processes.insert(id, proc);
        }
    }

    /// Get process status.
    pub fn process_status(&self, id: ProcessId) -> Option<&Value> {
        self.processes.get(&id).map(|p| &p.status)
    }

    /// Get process status for both live and stale process handles.
    pub fn process_status_any(&self, id: ProcessId) -> Option<&Value> {
        self.processes
            .get(&id)
            .map(|p| &p.status)
            .or_else(|| self.deleted_processes.get(&id).map(|p| &p.status))
    }

    /// Get a process by id.
    pub fn get(&self, id: ProcessId) -> Option<&Process> {
        self.processes.get(&id)
    }

    /// Get a process by id from either live or stale process tables.
    pub fn get_any(&self, id: ProcessId) -> Option<&Process> {
        self.processes
            .get(&id)
            .or_else(|| self.deleted_processes.get(&id))
    }

    /// Get a mutable process by id.
    pub fn get_mut(&mut self, id: ProcessId) -> Option<&mut Process> {
        self.processes.get_mut(&id)
    }

    /// Get a mutable process by id from either live or stale process tables.
    pub fn get_any_mut(&mut self, id: ProcessId) -> Option<&mut Process> {
        if self.processes.contains_key(&id) {
            self.processes.get_mut(&id)
        } else {
            self.deleted_processes.get_mut(&id)
        }
    }

    pub(crate) fn open_channel_for_module(&self, process: Value) -> Result<std::ffi::c_int, Flow> {
        let id = resolve_process_or_wrong_type_any_in_manager(self, &process)?;
        let proc = self
            .get_any(id)
            .ok_or_else(|| signal_wrong_type_processp(process))?;
        if proc.kind != ProcessKind::Pipe {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("pipe-process-p"), process],
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let stdout = proc.child_stdout.as_ref().ok_or_else(|| {
                signal(
                    "error",
                    vec![Value::string("Pipe process has no stdout file descriptor")],
                )
            })?;
            let fd = unsafe { libc::dup(stdout.as_raw_fd()) };
            if fd == -1 {
                return Err(signal(
                    "file-error",
                    vec![Value::string("Cannot duplicate file descriptor")],
                ));
            }
            Ok(fd)
        }
        #[cfg(not(unix))]
        {
            Err(signal(
                "file-error",
                vec![Value::string(
                    "Cannot duplicate file descriptor on this platform",
                )],
            ))
        }
    }

    /// List all process ids.
    pub fn list_processes(&self) -> Vec<ProcessId> {
        // GNU `process-list` is `(mapcar #'cdr Vprocess_alist)`, and a new
        // process is consed to the FRONT of `Vprocess_alist` (process.c:953), so
        // the list is newest-first. `ProcessId` is a monotonic counter, so
        // sorting by descending id reproduces GNU's order exactly (a deleted
        // process is removed from both the alist and the map).
        let mut ids: Vec<ProcessId> = self.processes.keys().copied().collect();
        ids.sort_unstable_by(|a, b| b.cmp(a));
        ids
    }

    /// Return IDs of processes that have a live OS child, PTY child, or network socket.
    pub fn live_process_ids(&self) -> Vec<ProcessId> {
        self.processes
            .iter()
            .filter(|(_, p)| {
                if !process_has_readable_process_io(p) {
                    return false;
                }
                if p.child.is_some() || p.pty_child.is_some() {
                    return true;
                }
                if p.network_socket.is_some() || p.tls_stream.is_some() {
                    return true;
                }
                // A stderr pipe-process (make-process :stderr) has no child of
                // its own; its readable source is the child's stderr fd parked
                // in `child_stderr`.  It must be serviced so its output is
                // drained and it reaches a terminal state on EOF.
                if p.child_stderr.is_some() {
                    return true;
                }
                false
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns true if this id has been allocated at least once.
    pub fn was_issued_id(&self, id: ProcessId) -> bool {
        id > 0 && id < self.next_id
    }

    /// Find a process by name.
    pub fn find_by_name(&self, name: &str) -> Option<ProcessId> {
        let wanted = process_name_value(name);
        self.processes
            .values()
            .find(|p| equal_value(&p.name, &wanted, 0))
            .map(|p| p.id)
    }

    /// Find a process associated with BUFFER-ID.
    pub fn find_by_buffer_id(&self, buffer_id: crate::buffer::BufferId) -> Option<ProcessId> {
        self.processes
            .values()
            .find(|p| p.buffer.as_buffer_id() == Some(buffer_id))
            .map(|p| p.id)
    }

    /// Queue input for a process.
    pub fn send_input(&mut self, id: ProcessId, input: &LispString) -> Result<bool, Flow> {
        if let Some(proc) = self.processes.get_mut(&id) {
            proc.write_queue =
                write_queue_push(proc.write_queue, Value::heap_string(input.clone()), false);
            let input_bytes = input.as_bytes();
            // Write to PTY master if this is a PTY process.
            if let Some(ref mut pty_writer) = proc.pty_writer {
                use std::io::Write;
                let _ = pty_writer.write_all(input_bytes);
                let _ = pty_writer.flush();
            } else if let Some(ref mut child) = proc.child {
                // Write to actual child stdin if available (pipe mode).
                if let Some(ref mut stdin) = child.stdin {
                    use std::io::Write;
                    let _ = stdin.write_all(input_bytes);
                    let _ = stdin.flush();
                }
            }
            // Write to TLS stream or plain socket for network processes.
            if let Some(ref mut tls) = proc.tls_stream {
                tls.write_all_process_input(input_bytes)
                    .map_err(|err| signal_process_io("Writing to process", None, err))?;
            } else if let Some(socket) = proc.network_socket.as_mut() {
                let datagram_address = proc.datagram_socket_addr;
                #[cfg(unix)]
                let datagram_unix_path = proc.datagram_unix_path.clone();
                match socket {
                    NetworkSocket::UdpSocket(socket) => {
                        let Some(addr) = datagram_address else {
                            return Err(signal(
                                "error",
                                vec![Value::string("No datagram address")],
                            ));
                        };
                        socket
                            .send_to(input_bytes, addr)
                            .map_err(|err| signal_process_io("Sending datagram", None, err))?;
                    }
                    #[cfg(unix)]
                    NetworkSocket::UnixDatagram(socket) => {
                        let Some(path) = datagram_unix_path else {
                            return Err(signal(
                                "error",
                                vec![Value::string("No datagram address")],
                            ));
                        };
                        socket
                            .send_to(input_bytes, path)
                            .map_err(|err| signal_process_io("Sending datagram", None, err))?;
                    }
                    _ => {
                        let _ = socket.write_stream_input(input_bytes);
                    }
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Register a network socket with the I/O poller so that
    /// `wait_for_output` wakes up when data arrives.
    pub fn register_socket_fd(&self, id: ProcessId) -> Result<(), String> {
        let proc = self.processes.get(&id).ok_or("Process not found")?;
        if let Some(poller) = self.wait_backend.poller() {
            if let Some(tls) = proc.tls_stream.as_ref() {
                Self::register_readable_source(poller, tls.tcp_stream(), id)?;
                return Ok(());
            }

            let socket = proc.network_socket.as_ref().ok_or("No socket")?;
            socket.register_readable(poller, id)?;
        }
        Ok(())
    }

    pub fn register_socket_writable_fd(&self, id: ProcessId) -> Result<(), String> {
        let proc = self.processes.get(&id).ok_or("Process not found")?;
        if let Some(poller) = self.wait_backend.poller() {
            let socket = proc.network_socket.as_ref().ok_or("No socket")?;
            socket.register_writable(poller, id)?;
        }
        Ok(())
    }

    fn update_tcp_client_contact(
        proc: &mut Process,
        remote_addr: SocketAddr,
        local_addr: Option<SocketAddr>,
    ) -> Result<(), Flow> {
        proc.childp = process_contact_plist_put(
            proc.childp,
            ProcessKeyword::Remote.value(),
            socket_addr_to_lisp_value(remote_addr),
        )?;
        if let Some(local_addr) = local_addr {
            proc.childp = process_contact_plist_put(
                proc.childp,
                ProcessKeyword::Local.value(),
                socket_addr_to_lisp_value(local_addr),
            )?;
        }
        Ok(())
    }

    fn start_next_pending_network_connect(
        &mut self,
        id: ProcessId,
        addrs: Vec<SocketAddr>,
        options: &[NetworkSocketOptionSpec],
    ) -> Result<Option<i32>, Flow> {
        let start = start_pending_tcp_stream_connect(addrs, options)?;
        let started = match start {
            PendingNetworkConnectStart::Started(started) => started,
            PendingNetworkConnectStart::Failed(code) => return Ok(Some(code)),
        };
        let local_addr = started.stream.local_addr().ok();
        if let Some(proc) = self.processes.get_mut(&id) {
            proc.network_socket = Some(NetworkSocket::TcpStream(started.stream));
            proc.pending_network_connect = Some(PendingNetworkConnect::Tcp {
                remaining_addrs: started.remaining_addrs,
                socket_options: options.to_vec(),
            });
            proc.status = process_status_connect_value();
            Self::update_tcp_client_contact(proc, started.remote_addr, local_addr)?;
        }
        self.register_socket_writable_fd(id).ok();
        Ok(None)
    }

    fn complete_pending_network_connect(
        &mut self,
        id: ProcessId,
    ) -> Result<PendingNetworkConnectCompletion, Flow> {
        let Some(proc) = self.processes.get(&id) else {
            return Ok(PendingNetworkConnectCompletion::None);
        };
        if proc.pending_network_connect.is_none() {
            return Ok(PendingNetworkConnectCompletion::None);
        }
        let connect_error = proc
            .network_socket
            .as_ref()
            .and_then(NetworkSocket::take_pending_connect_error)
            .transpose()
            .map_err(|err| signal_process_io("Checking network connection", None, err))?
            .flatten();

        if let Some(err) = connect_error {
            let pending = self
                .processes
                .get_mut(&id)
                .and_then(|proc| proc.pending_network_connect.take());
            let Some(pending) = pending else {
                return Ok(PendingNetworkConnectCompletion::None);
            };
            if let Some(proc) = self.processes.get(&id)
                && let Some(socket) = proc.network_socket.as_ref()
                && let Some(poller) = self.wait_backend.poller()
            {
                socket.unregister_readable(poller);
            }
            let code = io_error_status_code(&err);
            match pending {
                PendingNetworkConnect::Tcp {
                    remaining_addrs,
                    socket_options,
                } if !remaining_addrs.is_empty() => {
                    return match self.start_next_pending_network_connect(
                        id,
                        remaining_addrs,
                        &socket_options,
                    )? {
                        None => Ok(PendingNetworkConnectCompletion::Retrying),
                        Some(code) => {
                            let sentinel = self
                                .processes
                                .get(&id)
                                .map(|proc| proc.sentinel)
                                .unwrap_or(Value::NIL);
                            if let Some(proc) = self.processes.get_mut(&id) {
                                proc.status = process_status_failed_value(code);
                                proc.network_socket = None;
                                proc.pending_network_connect = None;
                            }
                            Ok(PendingNetworkConnectCompletion::Failed { sentinel, code })
                        }
                    };
                }
                _ => {}
            }

            let sentinel = self
                .processes
                .get(&id)
                .map(|proc| proc.sentinel)
                .unwrap_or(Value::NIL);
            if let Some(proc) = self.processes.get_mut(&id) {
                proc.status = process_status_failed_value(code);
                proc.network_socket = None;
                proc.pending_network_connect = None;
            }
            return Ok(PendingNetworkConnectCompletion::Failed { sentinel, code });
        }

        let sentinel = self
            .processes
            .get(&id)
            .map(|proc| proc.sentinel)
            .unwrap_or(Value::NIL);
        if let Some(proc) = self.processes.get(&id)
            && let Some(socket) = proc.network_socket.as_ref()
            && let Some(poller) = self.wait_backend.poller()
        {
            socket.unregister_readable(poller);
        }
        if let Some(proc) = self.processes.get_mut(&id) {
            proc.pending_network_connect = None;
            proc.status = process_status_run_value();
        }
        self.register_socket_fd(id).ok();
        Ok(PendingNetworkConnectCompletion::Connected { sentinel })
    }

    fn accept_network_server_connections(
        &mut self,
        id: ProcessId,
    ) -> Vec<AcceptedNetworkConnection> {
        enum AcceptedSocket {
            Tcp {
                stream: TcpStream,
                remote_addr: SocketAddr,
                local_addr: Option<SocketAddr>,
            },
            #[cfg(unix)]
            Seqpacket {
                socket: Socket,
                remote_addr: SockAddr,
                local_addr: Option<SockAddr>,
            },
            #[cfg(unix)]
            Unix {
                stream: UnixStream,
                remote_name: String,
                local_name: String,
            },
        }

        let mut accepted = Vec::new();

        loop {
            let accepted_socket = {
                let Some(server) = self.processes.get(&id) else {
                    return accepted;
                };
                match server.network_socket.as_ref() {
                    Some(NetworkSocket::TcpListener(listener)) => match listener.accept() {
                        Ok((stream, remote_addr)) => Ok(Some(AcceptedSocket::Tcp {
                            local_addr: stream.local_addr().ok(),
                            stream,
                            remote_addr,
                        })),
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
                        Err(_) => Ok(None),
                    },
                    #[cfg(unix)]
                    Some(NetworkSocket::SeqpacketListener(listener)) => match listener.accept() {
                        Ok((socket, remote_addr)) => Ok(Some(AcceptedSocket::Seqpacket {
                            local_addr: socket.local_addr().ok(),
                            socket,
                            remote_addr,
                        })),
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
                        Err(_) => Ok(None),
                    },
                    #[cfg(unix)]
                    Some(NetworkSocket::UnixListener(listener)) => match listener.accept() {
                        Ok((stream, _)) => Ok(Some(AcceptedSocket::Unix {
                            remote_name: unix_socket_addr_to_runtime_string(
                                stream.peer_addr().ok(),
                            ),
                            local_name: unix_socket_addr_to_runtime_string(
                                stream.local_addr().ok(),
                            ),
                            stream,
                        })),
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
                        Err(_) => Ok(None),
                    },
                    _ => Err(()),
                }
            };
            let accepted_socket = match accepted_socket {
                Ok(Some(socket)) => socket,
                Ok(None) => break,
                Err(()) => return accepted,
            };

            let (
                server_name,
                server_contact,
                server_buffer,
                server_filter,
                server_sentinel,
                server_log,
                server_plist,
                coding_decode,
                coding_encode,
                inherit_coding_system_flag,
                server_thread,
                query_on_exit_flag,
            ) = {
                let Some(server) = self.processes.get(&id) else {
                    return accepted;
                };
                (
                    process_name_runtime(server.name),
                    server.childp,
                    server.buffer,
                    server.filter,
                    server.sentinel,
                    server.log,
                    server.plist,
                    server.coding_decode,
                    server.coding_encode,
                    server.inherit_coding_system_flag,
                    server.thread,
                    server.query_on_exit_flag,
                )
            };

            let mut contact = super::builtins::builtin_copy_sequence(vec![server_contact])
                .unwrap_or(server_contact);
            contact =
                process_contact_plist_put(contact, ProcessKeyword::Server.value(), Value::NIL)
                    .unwrap_or(contact);

            let (client_name, socket, host_for_message) = match accepted_socket {
                AcceptedSocket::Tcp {
                    stream,
                    remote_addr,
                    local_addr,
                } => {
                    let _ = stream.set_nonblocking(true);
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Host.value(),
                        Value::string(remote_addr.ip().to_string()),
                    )
                    .unwrap_or(contact);
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Service.value(),
                        Value::fixnum(remote_addr.port() as i64),
                    )
                    .unwrap_or(contact);
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        socket_addr_to_lisp_value(remote_addr),
                    )
                    .unwrap_or(contact);
                    if let Some(local_addr) = local_addr {
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Local.value(),
                            socket_addr_to_lisp_value(local_addr),
                        )
                        .unwrap_or(contact);
                    }
                    (
                        accepted_network_process_name(&server_name, remote_addr),
                        NetworkSocket::TcpStream(stream),
                        remote_addr.ip().to_string(),
                    )
                }
                #[cfg(unix)]
                AcceptedSocket::Seqpacket {
                    socket,
                    remote_addr,
                    local_addr,
                } => {
                    let _ = socket.set_nonblocking(true);
                    if let Some(remote_addr) = remote_addr.as_socket() {
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Host.value(),
                            Value::string(remote_addr.ip().to_string()),
                        )
                        .unwrap_or(contact);
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Service.value(),
                            Value::fixnum(remote_addr.port() as i64),
                        )
                        .unwrap_or(contact);
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Remote.value(),
                            socket_addr_to_lisp_value(remote_addr),
                        )
                        .unwrap_or(contact);
                        if let Some(local_addr) = local_addr.and_then(|addr| addr.as_socket()) {
                            contact = process_contact_plist_put(
                                contact,
                                ProcessKeyword::Local.value(),
                                socket_addr_to_lisp_value(local_addr),
                            )
                            .unwrap_or(contact);
                        }
                        (
                            accepted_network_process_name(&server_name, remote_addr),
                            NetworkSocket::SeqpacketStream(socket),
                            remote_addr.ip().to_string(),
                        )
                    } else {
                        let remote_name =
                            socket2_unix_sockaddr_to_runtime_string(Some(&remote_addr));
                        let local_name =
                            socket2_unix_sockaddr_to_runtime_string(local_addr.as_ref());
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Host.value(),
                            Value::NIL,
                        )
                        .unwrap_or(contact);
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Remote.value(),
                            Value::string(&remote_name),
                        )
                        .unwrap_or(contact);
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Local.value(),
                            Value::string(&local_name),
                        )
                        .unwrap_or(contact);

                        let prefix = format!("{} <", server_name);
                        let sequence = self
                            .processes
                            .values()
                            .filter(|process| {
                                process_name_runtime(process.name).starts_with(&prefix)
                            })
                            .count()
                            + 1;
                        let host_for_message = if remote_name.is_empty() {
                            "-".to_string()
                        } else {
                            remote_name
                        };
                        (
                            format!("{} <{}>", server_name, sequence),
                            NetworkSocket::SeqpacketStream(socket),
                            host_for_message,
                        )
                    }
                }
                #[cfg(unix)]
                AcceptedSocket::Unix {
                    stream,
                    remote_name,
                    local_name,
                } => {
                    let _ = stream.set_nonblocking(true);
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Host.value(),
                        Value::NIL,
                    )
                    .unwrap_or(contact);
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        Value::string(&remote_name),
                    )
                    .unwrap_or(contact);
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        Value::string(&local_name),
                    )
                    .unwrap_or(contact);

                    let prefix = format!("{} <", server_name);
                    let sequence = self
                        .processes
                        .values()
                        .filter(|process| process_name_runtime(process.name).starts_with(&prefix))
                        .count()
                        + 1;
                    let host_for_message = if remote_name.is_empty() {
                        "-".to_string()
                    } else {
                        remote_name
                    };
                    (
                        format!("{} <{}>", server_name, sequence),
                        NetworkSocket::UnixStream(stream),
                        host_for_message,
                    )
                }
            };

            let client_id = self.create_process_with_kind_lisp(
                LispString::from_utf8(&client_name),
                server_buffer,
                LispString::from_utf8("network"),
                Vec::new(),
                ProcessKind::Network,
            );
            if let Some(client) = self.get_mut(client_id) {
                client.network_socket = Some(socket);
                client.status = process_status_run_value();
                client.childp = contact;
                client.filter = server_filter;
                client.sentinel = server_sentinel;
                client.plist = server_plist;
                client.coding_decode = coding_decode;
                client.coding_encode = coding_encode;
                client.inherit_coding_system_flag = inherit_coding_system_flag;
                client.thread = server_thread;
                client.query_on_exit_flag = query_on_exit_flag;
            }
            self.register_socket_fd(client_id).ok();

            accepted.push(AcceptedNetworkConnection {
                server_id: id,
                client_id,
                log: server_log,
                sentinel: server_sentinel,
                log_message: format!("accept from {}\n", host_for_message),
                sentinel_message: format!("open from {}\n", host_for_message),
            });
        }

        accepted
    }

    /// Read available output from a process — child stdout or network socket.
    /// Returns `Some(data)` with available data (possibly empty on WouldBlock),
    /// or `None` on EOF / connection closed.
    fn read_process_output_result(&mut self, id: ProcessId) -> ProcessOutputRead {
        let source = self.processes.get(&id).and_then(process_output_source);

        match source {
            Some(ProcessOutputSource::Pty) => self.read_pty_output_result(id),
            Some(ProcessOutputSource::ChildStdout) => self.read_child_stdout_result(id),
            Some(ProcessOutputSource::ChildStderr) => self.read_child_stderr_result(id),
            Some(ProcessOutputSource::Network) => self.read_network_output_result(id),
            None => ProcessOutputRead::NoSource,
        }
    }

    /// Read available output from a process — child stdout or network socket.
    /// Returns `Some(data)` with available data (possibly empty on WouldBlock),
    /// or `None` on EOF / connection closed.
    pub fn read_process_output(&mut self, id: ProcessId) -> Option<LispString> {
        self.read_process_output_result(id).into_legacy_option()
    }

    /// Get stdout output from a process.
    pub fn get_output(&self, id: ProcessId) -> Option<&str> {
        self.processes.get(&id).map(|p| p.stdout.as_str())
    }

    /// Get an environment variable (checking overrides first, then OS).
    pub fn getenv(&self, name: &str) -> Option<LispString> {
        let key = LispString::from_utf8(name);
        if let Some(override_val) = self.env_overrides.get(&key) {
            return override_val.clone();
        }
        std::env::var_os(name)
            .as_ref()
            .map(|value| os_str_to_lisp_string(value.as_os_str()))
    }

    /// Set an environment variable override.  If value is None, unset it.
    pub fn setenv(&mut self, name: LispString, value: Option<LispString>) {
        self.env_overrides.insert(name, value);
    }
}

const DEFAULT_PROCESS_FILTER_SYMBOL: &str = "internal-default-process-filter";
const DEFAULT_PROCESS_SENTINEL_SYMBOL: &str = "internal-default-process-sentinel";

fn dedupe_process_ids(process_ids: impl IntoIterator<Item = ProcessId>) -> Vec<ProcessId> {
    let mut unique = Vec::new();
    for id in process_ids {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    unique
}

impl super::eval::Context {
    /// Whether `delete-exited-processes` is non-nil (GNU
    /// `delete_exited_processes`, default `t`).  Controls whether a terminated
    /// process is removed from the process list once its status is reported.
    fn delete_exited_processes_enabled(&self) -> bool {
        self.visible_variable_value_or_nil("delete-exited-processes")
            .is_truthy()
    }

    pub(crate) fn service_pending_timers_with_wait_policy(
        &mut self,
        redisplay: bool,
    ) -> Result<bool, Flow> {
        self.flush_pending_safe_funcalls();
        let mut fired_any = false;

        // GNU runs Lisp timers from `timer_check` before servicing lower-level
        // atimer/process-fd callbacks in `wait_reading_process_output`.  A
        // non-local `throw` raised by a timer callback propagates out to the
        // matching outer `catch` (e.g. `jsonrpc-request`'s catch tag), so a
        // throw from `run_timer_callback_preserving_state` is returned to the
        // caller — and the remaining due timers are not run, exactly as in GNU
        // where the throw unwinds out of `timer_check`.
        while let Some(timer) = self.next_due_gnu_timer_snapshot() {
            fired_any = true;
            if timer.is_vector() {
                let _ = timer.set_vector_slot(0, Value::T);
            }
            self.run_timer_callback_preserving_state(
                Value::symbol("timer-event-handler"),
                vec![timer],
                "GNU Lisp timer",
            )?;
        }

        let now = Instant::now();
        let idle_dur = self.current_idle_duration();
        let fired = self.timers.fire_pending_timers(now, idle_dur);
        for (callback, args) in fired {
            fired_any = true;
            self.run_timer_callback_preserving_state(callback, args, "Rust timer")?;
        }

        if fired_any && redisplay {
            self.redisplay();
        }

        Ok(fired_any)
    }

    fn run_async_process_callback_preserving_state(
        &mut self,
        callback: Value,
        args: Vec<Value>,
        label: &str,
    ) -> Result<(), Flow> {
        let saved_match_data = self.match_data.clone();
        let saved_current_buffer = self.buffers.current_buffer_id();
        let saved_waiting_for_input = self.waiting_for_user_input();
        let saved_deactivate_mark = self.eval_symbol("deactivate-mark").unwrap_or(Value::NIL);
        let specpdl_count = self.specpdl.len();

        let gc_roots = self.save_specpdl_roots();
        self.push_specpdl_root(callback);
        for arg in &args {
            self.push_specpdl_root(*arg);
        }

        self.specbind(intern("inhibit-quit"), Value::T);
        self.specbind(intern("last-nonmenu-event"), Value::T);

        let result = self.apply(callback, args);
        self.match_data = saved_match_data;
        if let Some(buffer_id) = saved_current_buffer {
            self.restore_current_buffer_if_live(buffer_id);
        }
        self.set_waiting_for_user_input(saved_waiting_for_input);
        self.unbind_to(specpdl_count);
        self.assign("deactivate-mark", saved_deactivate_mark);
        self.restore_specpdl_roots(gc_roots);

        self.finish_callback_flow(result, label)
    }

    fn run_timer_callback_preserving_state(
        &mut self,
        callback: Value,
        args: Vec<Value>,
        label: &str,
    ) -> Result<(), Flow> {
        let saved_current_buffer = self.buffers.current_buffer_id();
        let saved_deactivate_mark = self.eval_symbol("deactivate-mark").unwrap_or(Value::NIL);
        let specpdl_count = self.specpdl.len();

        let gc_roots = self.save_specpdl_roots();
        self.push_specpdl_root(callback);
        for arg in &args {
            self.push_specpdl_root(*arg);
        }

        self.specbind(intern("inhibit-quit"), Value::T);

        let result = self.apply(callback, args);
        if let Some(buffer_id) = saved_current_buffer {
            self.restore_current_buffer_if_live(buffer_id);
        }
        self.unbind_to(specpdl_count);
        self.assign("deactivate-mark", saved_deactivate_mark);
        self.restore_specpdl_roots(gc_roots);

        self.finish_callback_flow(result, label)
    }

    /// Resolve the control flow that escaped a timer/process callback after the
    /// callback's own state (buffer/deactivate-mark/specpdl/gc-roots) has been
    /// restored.
    ///
    /// GNU runs timer callbacks through `lisp/emacs-lisp/timer.el`
    /// `timer-event-handler`, which wraps the call in
    /// `condition-case-unless-debug err … (error …)`; process filters/sentinels
    /// in `src/process.c` (`read_process_output`/`exec_sentinel`) run with no
    /// surrounding handler at all.  In both cases an `error`-class *signal* is
    /// caught (and logged), but a non-local `throw` is NOT an error, so it
    /// propagates past the callback boundary to the matching outer `catch`.
    ///
    /// Mirroring that, a `Flow::Signal` is caught and logged here, while a
    /// `Flow::Throw` is propagated to the caller so it can reach the `catch`
    /// that surrounds the wait (e.g. `jsonrpc-request`'s catch tag, completed by
    /// a zero-delay `run-at-time` timer).  A throw to a tag with no live catch
    /// still becomes a `no-catch` error at the eval/thread boundary, as in GNU.
    fn finish_callback_flow(&mut self, result: EvalResult, label: &str) -> Result<(), Flow> {
        match result {
            Ok(_) => Ok(()),
            Err(err @ Flow::Throw { .. }) => Err(err),
            Err(err @ Flow::Signal(_)) => {
                let rendered = super::error::format_flow_with_eval(self, &err);
                tracing::warn!("{label} callback error: {}", rendered);
                Ok(())
            }
        }
    }

    fn run_process_filter_callback(
        &mut self,
        pid: ProcessId,
        filter: Value,
        data: &LispString,
    ) -> Result<(), Flow> {
        let proc_val = Value::make_process(pid);
        let output_val = Value::heap_string(data.clone());
        if filter.is_nil() || filter.is_symbol_named(DEFAULT_PROCESS_FILTER_SYMBOL) {
            let callback = Value::symbol(DEFAULT_PROCESS_FILTER_SYMBOL);
            self.run_async_process_callback_preserving_state(
                callback,
                vec![proc_val, output_val],
                "process filter",
            )
        } else if filter.is_truthy() {
            self.run_async_process_callback_preserving_state(
                filter,
                vec![proc_val, output_val],
                "process filter",
            )
        } else {
            Ok(())
        }
    }

    fn run_process_sentinel_callback(
        &mut self,
        pid: ProcessId,
        sentinel: Value,
        message: &str,
    ) -> Result<(), Flow> {
        if sentinel.is_nil() {
            return Ok(());
        }

        let callback = if sentinel.is_symbol_named(DEFAULT_PROCESS_SENTINEL_SYMBOL) {
            Value::symbol(DEFAULT_PROCESS_SENTINEL_SYMBOL)
        } else {
            sentinel
        };

        self.run_async_process_callback_preserving_state(
            callback,
            vec![Value::make_process(pid), Value::string(message)],
            "process sentinel",
        )
    }

    fn run_process_log_callback(
        &mut self,
        log: Value,
        server_id: ProcessId,
        client_id: ProcessId,
        message: &str,
    ) -> Result<(), Flow> {
        if log.is_nil() {
            return Ok(());
        }

        self.run_async_process_callback_preserving_state(
            log,
            vec![
                Value::make_process(server_id),
                Value::make_process(client_id),
                Value::string(message),
            ],
            "process log",
        )
    }

    pub(crate) fn poll_process_output_for_service_request(
        &mut self,
        request: &ProcessOutputServiceRequest,
    ) -> Result<ProcessOutputServiceOutcome, Flow> {
        let target_process = request.target_process();
        let proc_ids = request.live_processes(self.processes.live_process_ids());
        self.poll_process_output_for_ids(proc_ids, target_process)
    }

    pub(crate) fn poll_ready_process_output_for_service_request(
        &mut self,
        events: ProcessWaitEvents,
        request: &ProcessOutputServiceRequest,
    ) -> Result<ProcessOutputServiceOutcome, Flow> {
        let target_process = request.target_process();
        let mut outcome = ProcessOutputServiceOutcome::default();

        let writable_processes = request.ready_processes(events.writable_processes_ref().to_vec());
        for pid in writable_processes {
            let is_target = target_process.map_or(true, |target| target == pid);
            match self.processes.complete_pending_network_connect(pid)? {
                PendingNetworkConnectCompletion::None => {}
                PendingNetworkConnectCompletion::Retrying => {
                    outcome.record_activity(is_target);
                }
                PendingNetworkConnectCompletion::Connected { sentinel } => {
                    outcome.record_activity(is_target);
                    self.run_process_sentinel_callback(pid, sentinel, "open\n")?;
                }
                PendingNetworkConnectCompletion::Failed { sentinel, code } => {
                    outcome.record_activity(is_target);
                    self.run_process_sentinel_callback(
                        pid,
                        sentinel,
                        &format!("failed with code {code}\n"),
                    )?;
                }
            }
        }

        let proc_ids = request.ready_processes(events.ready_processes_ref().to_vec());
        outcome.absorb(self.poll_process_output_for_ids(proc_ids, target_process)?);

        Ok(outcome)
    }

    fn poll_process_output_for_ids(
        &mut self,
        proc_ids: Vec<ProcessId>,
        target_process: Option<ProcessId>,
    ) -> Result<ProcessOutputServiceOutcome, Flow> {
        let proc_ids = dedupe_process_ids(proc_ids);

        if proc_ids.is_empty() {
            return Ok(ProcessOutputServiceOutcome::default());
        }

        let mut outcome = ProcessOutputServiceOutcome::default();

        for pid in proc_ids {
            if self
                .processes
                .get(pid)
                .is_some_and(|process| !process_has_readable_process_io(process))
            {
                continue;
            }
            if self.processes.get(pid).is_some_and(|process| {
                ProcessStatusSymbol::from_status_value(process.status)
                    == Some(ProcessStatusSymbol::Connect)
            }) {
                continue;
            }

            let is_target = target_process.map_or(true, |target| target == pid);
            let mut exited = self.processes.check_child_exit(pid);
            for event in self.processes.accept_network_server_connections(pid) {
                outcome.record_activity(is_target);
                self.run_process_log_callback(
                    event.log,
                    event.server_id,
                    event.client_id,
                    &event.log_message,
                )?;
                let sentinel = self
                    .processes
                    .get(event.client_id)
                    .map(|process| process.sentinel)
                    .unwrap_or(event.sentinel);
                self.run_process_sentinel_callback(
                    event.client_id,
                    sentinel,
                    &event.sentinel_message,
                )?;
            }

            let read_result = self.processes.read_process_output_result(pid);
            let is_network = self
                .processes
                .get(pid)
                .map(|p| p.kind == ProcessKind::Network)
                .unwrap_or(false);
            // A stderr pipe-process drains the child's separate stderr fd; its
            // readable source lives in `child_stderr` and it has no child of
            // its own (it is a `ProcessKind::Pipe` created for `:stderr`).  On
            // EOF it must reach a terminal state and run its sentinel, or
            // `accept-process-output` would block forever waiting on it.  This
            // must NOT match the main (Real) process, which owns the child.
            let is_stderr_pipe = self
                .processes
                .get(pid)
                .map(|p| {
                    p.kind == ProcessKind::Pipe && p.child_stderr.is_some() && p.child.is_none()
                })
                .unwrap_or(false);

            match read_result {
                ProcessOutputRead::Data(ref data) if !data.is_empty() => {
                    outcome.record_activity(is_target);

                    let filter = self
                        .processes
                        .get(pid)
                        .map(|p| p.filter)
                        .unwrap_or(Value::NIL);
                    self.run_process_filter_callback(pid, filter, data)?;
                }
                ProcessOutputRead::Eof if is_stderr_pipe => {
                    outcome.record_activity(is_target);

                    // Mirror GNU: when the child's stderr EOFs, the stderr
                    // pipe-process finishes (exit status 0) and runs its
                    // sentinel, which inserts "Process NAME stderr finished".
                    self.processes.deactivate_stderr_pipe_process_io(pid);
                    if let Some(proc) = self.processes.get_mut(pid) {
                        proc.status = process_status_exit_value(0);
                    }
                    let sentinel = self
                        .processes
                        .get(pid)
                        .map(|p| p.sentinel)
                        .unwrap_or(Value::NIL);
                    let exit_msg = self
                        .processes
                        .get(pid)
                        .map(|p| gnu_process_status_message(p.status))
                        .unwrap_or_else(|| "finished\n".to_string());
                    self.run_process_sentinel_callback(pid, sentinel, &exit_msg)?;

                    // GNU `status_notify`: a terminated process (including the
                    // implicit stderr pipe-process) is removed from
                    // `Vprocess_alist' when `delete-exited-processes' is non-nil
                    // (its default), so `get-process'/`get-buffer-process' no
                    // longer return it.  Without this the dead "<name> stderr"
                    // process lingers in the process list, diverging from GNU
                    // (which returns nil for `get-buffer-process' on the stderr
                    // buffer once the process has finished).
                    if self.delete_exited_processes_enabled() {
                        self.processes.reap_exited_process(pid);
                    }
                    continue;
                }
                ProcessOutputRead::Eof if is_network => {
                    outcome.record_activity(is_target);

                    if let Some(proc) = self.processes.get_mut(pid) {
                        proc.status = process_status_exit_value(0);
                    }
                    self.processes.deactivate_network_process_io(pid);
                    let sentinel = self
                        .processes
                        .get(pid)
                        .map(|p| p.sentinel)
                        .unwrap_or(Value::NIL);
                    self.run_process_sentinel_callback(
                        pid,
                        sentinel,
                        "connection broken by remote peer\n",
                    )?;
                    continue;
                }
                _ => {}
            }

            // GNU's wait request can observe process output and terminal status
            // in the same wake cycle. Re-check after reading so short-lived
            // children that exit immediately after flushing output do not
            // defer their sentinel to a second accept-process-output call.
            if !exited {
                exited = self.processes.check_child_exit(pid);
            }

            if exited {
                outcome.record_activity(is_target);

                // GNU `status_notify` (process.c) drains ALL remaining output
                // from a terminated process before reporting its status and
                // (when `delete-exited-processes' is non-nil) removing it from
                // `Vprocess_alist'.  Mirror the drain here so trailing bytes
                // buffered in the pipe after the child exited are not lost when
                // the process is reaped below.
                loop {
                    match self.processes.read_process_output_result(pid) {
                        ProcessOutputRead::Data(ref data) if !data.is_empty() => {
                            let filter = self
                                .processes
                                .get(pid)
                                .map(|p| p.filter)
                                .unwrap_or(Value::NIL);
                            self.run_process_filter_callback(pid, filter, data)?;
                        }
                        ProcessOutputRead::Data(_)
                        | ProcessOutputRead::WouldBlock
                        | ProcessOutputRead::Eof
                        | ProcessOutputRead::NoSource => break,
                    }
                }

                let sentinel = self
                    .processes
                    .get(pid)
                    .map(|p| p.sentinel)
                    .unwrap_or(Value::NIL);
                let exit_msg = self
                    .processes
                    .get(pid)
                    .map(|p| gnu_process_status_message(p.status))
                    .unwrap_or_else(|| "finished\n".to_string());
                self.run_process_sentinel_callback(pid, sentinel, &exit_msg)?;

                // GNU `status_notify`: a terminated process (status exit/signal/
                // closed) is removed from `Vprocess_alist' when
                // `delete-exited-processes' is non-nil (its default), so that
                // `get-process'/`process-list' no longer return it.  The process
                // object itself stays alive for any binding that still holds it
                // (e.g. `process-status' on the value), which neomacs models by
                // moving it into the deleted-process table that `get_any' reads.
                if self.delete_exited_processes_enabled() {
                    self.processes.reap_exited_process(pid);
                }
            }
        }

        Ok(outcome)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn process_owned_runtime_string(value: Value) -> String {
    value
        .as_lisp_string()
        .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
        .expect("ValueKind::String must carry LispString payload")
}

fn expect_sequence(value: &Value) -> Result<(), Flow> {
    if value.is_nil() || value.is_cons() || value.is_vector() || value.is_string() {
        Ok(())
    } else {
        Err(signal_wrong_type_sequence(*value))
    }
}

fn expect_list(value: &Value) -> Result<(), Flow> {
    if value.is_list() {
        Ok(())
    } else {
        Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("listp"), *value],
        ))
    }
}

fn signal_wrong_type_sequence(value: Value) -> Flow {
    signal(
        "wrong-type-argument",
        vec![Value::symbol("sequencep"), value],
    )
}

fn signal_wrong_type_character(value: Value) -> Flow {
    signal(
        "wrong-type-argument",
        vec![Value::symbol("characterp"), value],
    )
}

fn char_code_from_value(value: &Value) -> Result<u32, Flow> {
    match value.kind() {
        ValueKind::Fixnum(_) => Ok(super::builtins::expect_character_code(value)? as u32),
        _ => Err(signal_wrong_type_character(*value)),
    }
}

/// Append the Emacs-internal byte encoding of a single character code.
fn push_char_code_bytes(code: u32, bytes: &mut Vec<u8>) {
    let mut buf = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
    let len = crate::emacs_core::emacs_char::char_string(code, &mut buf);
    bytes.extend_from_slice(&buf[..len]);
}

/// Convert a string / character-code vector / character-code list into a
/// faithful multibyte `LispString`, encoding each character code directly to
/// Emacs bytes via `char_string`.
///
/// Issue #131: this replaces a storage-String round-trip that corrupted real
/// character codes in the PUA sentinel ranges — e.g. the nerd-font glyph
/// U+E0B0 was rewritten to the eight-bit code 0x3FFFB0. Building the bytes
/// directly keeps every code intact.
pub(crate) fn char_sequence_to_lisp_string(value: &Value) -> Result<LispString, Flow> {
    if let Some(string) = value.as_lisp_string() {
        return Ok(string.clone());
    }
    let mut bytes = Vec::new();
    match value.kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let vec = value.as_vector_data().unwrap().clone();
            for elt in vec.iter() {
                push_char_code_bytes(char_code_from_value(elt)?, &mut bytes);
            }
        }
        ValueKind::Cons | ValueKind::Nil => {
            let mut cursor = *value;
            loop {
                match cursor.kind() {
                    ValueKind::Nil => break,
                    ValueKind::Cons => {
                        let car = cursor.cons_car();
                        let cdr = cursor.cons_cdr();
                        push_char_code_bytes(char_code_from_value(&car)?, &mut bytes);
                        cursor = cdr;
                    }
                    _ => {
                        return Err(signal(
                            "wrong-type-argument",
                            vec![Value::symbol("listp"), cursor],
                        ));
                    }
                }
            }
        }
        _ => return Err(signal_wrong_type_sequence(*value)),
    }
    Ok(crate::heap_types::LispString::from_emacs_bytes(bytes))
}

pub(crate) fn expect_int_or_marker(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        ValueKind::Veclike(VecLikeType::Marker) => super::marker::marker_position_as_int(value),
        _ => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("integer-or-marker-p"), *value],
        )),
    }
}

pub(crate) fn checked_region_bytes(
    buf: &crate::buffer::Buffer,
    region: super::position::LispRegionArgs,
) -> Result<EmacsByteRange, Flow> {
    region.accessible_byte_range(buf)
}

fn file_error_symbol(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::NotFound => "file-missing",
        std::io::ErrorKind::AlreadyExists => "file-already-exists",
        std::io::ErrorKind::PermissionDenied => "permission-denied",
        _ => "file-error",
    }
}

pub(crate) fn signal_process_io(action: &str, target: Option<&str>, err: std::io::Error) -> Flow {
    let mut data = vec![Value::string(action), Value::string(err.to_string())];
    if let Some(target) = target {
        data.push(Value::string(target));
    }
    signal(file_error_symbol(err.kind()), data)
}

/// GNU `report_file_error (STRING, FILENAME)` (callproc.c/fileio.c) for a
/// subprocess file-open/IO failure: signal a file-error-family condition whose
/// DATA is `(STRING STRERROR FILENAME)`, deriving the error SYMBOL and the bare
/// `strerror` string (no Rust "(os error N)" suffix) from the underlying
/// `errno`.  Use this instead of `signal_process_io` whenever the failing
/// operation has a Lisp filename to report — GNU always includes it.
#[cfg(unix)]
pub(crate) fn signal_process_file_error(
    action: &str,
    filename: Value,
    err: std::io::Error,
) -> Flow {
    let errno = err.raw_os_error().unwrap_or(libc::EIO);
    signal_file_errno(action, filename, errno)
}

#[cfg(not(unix))]
pub(crate) fn signal_process_file_error(
    action: &str,
    filename: Value,
    err: std::io::Error,
) -> Flow {
    let mut data = vec![
        Value::string(action),
        Value::string(err.to_string()),
        filename,
    ];
    signal(file_error_symbol(err.kind()), data)
}

/// The bare strerror string for an errno, matching GNU's `emacs_strerror`
/// (e.g. ENOENT -> "No such file or directory").  Rust's
/// `io::Error::to_string()` appends "(os error N)", which GNU never emits, so
/// go through libc directly.
#[cfg(unix)]
fn errno_message(errno: libc::c_int) -> String {
    // SAFETY: strerror returns a pointer to a static (per-thread) C string.
    unsafe {
        let ptr = libc::strerror(errno);
        if ptr.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}

#[cfg(not(unix))]
fn errno_message(errno: libc::c_int) -> String {
    std::io::Error::from_raw_os_error(errno).to_string()
}

/// GNU `report_file_errno` (fileio.c): signal a file-error-family condition
/// whose DATA is `(STRING ERRNO-STRING . NAME-LIST)` and whose error SYMBOL is
/// derived from ERRNO (ENOENT -> `file-missing`, EEXIST -> `file-already-exists`,
/// EACCES -> `permission-denied`, else `file-error`).  NAME is wrapped in a
/// one-element list unless it is itself a list (or nil), exactly like
/// `get_file_errno_data`.
pub(crate) fn signal_file_errno(string: &str, name: Value, errno: libc::c_int) -> Flow {
    let symbol = match errno {
        libc::ENOENT => "file-missing",
        libc::EEXIST => "file-already-exists",
        libc::EACCES => "permission-denied",
        _ => "file-error",
    };
    let mut data = vec![Value::string(string), Value::string(errno_message(errno))];
    if name.is_cons() || name.is_nil() {
        if let Some(items) = super::value::list_to_vec(&name) {
            data.extend(items);
        }
    } else {
        data.push(name);
    }
    signal(symbol, data)
}

fn signal_wrong_type_string(value: Value) -> Flow {
    signal("wrong-type-argument", vec![Value::symbol("stringp"), value])
}

pub(crate) fn expect_string_strict(value: &Value) -> Result<String, Flow> {
    match value.kind() {
        ValueKind::String => Ok(process_owned_runtime_string(*value)),
        _ => Err(signal_wrong_type_string(*value)),
    }
}

fn expect_process_name_lisp_string(value: &Value) -> Result<LispString, Flow> {
    match value.kind() {
        ValueKind::String => Ok(value
            .as_lisp_string()
            .expect("ValueKind::String must carry LispString payload")
            .clone()),
        _ => Err(signal(
            "error",
            vec![Value::string(":name value not a string")],
        )),
    }
}

fn keyword_name(value: &Value) -> Option<&str> {
    match value.kind() {
        ValueKind::Symbol(k) => Some(resolve_sym(k)),
        _ => None,
    }
}
pub(crate) fn parse_string_args_strict(args: &[Value]) -> Result<Vec<String>, Flow> {
    args.iter().map(expect_string_strict).collect()
}

pub(crate) fn parse_lisp_string_args_strict(args: &[Value]) -> Result<Vec<LispString>, Flow> {
    args.iter()
        .map(|arg| {
            super::builtins::expect_lisp_string(arg)
                .cloned()
                .map_err(|_| signal_wrong_type_string(*arg))
        })
        .collect()
}

fn signal_wrong_type_processp(value: Value) -> Flow {
    signal(
        "wrong-type-argument",
        vec![Value::symbol("processp"), value],
    )
}

fn signal_process_does_not_exist(name: &str) -> Flow {
    signal(
        "error",
        vec![Value::string(format!("Process {name} does not exist"))],
    )
}

fn signal_process_not_active(eval: &super::eval::Context, id: ProcessId) -> Flow {
    signal_process_not_active_in_manager(&eval.processes, id)
}

fn signal_process_not_active_in_manager(processes: &ProcessManager, id: ProcessId) -> Flow {
    let name = processes
        .get_any(id)
        .map(|proc| process_name_runtime(proc.name))
        .unwrap_or_else(|| id.to_string());
    signal(
        "error",
        vec![Value::string(format!("Process {name} is not active"))],
    )
}

fn stale_process_not_running_reason(status: &Value) -> &'static str {
    match ProcessStatusSymbol::from_status_value(*status) {
        Some(ProcessStatusSymbol::Signal) => "killed",
        Some(ProcessStatusSymbol::Exit) => "finished",
        Some(ProcessStatusSymbol::Stop) => "stopped",
        Some(ProcessStatusSymbol::Run) => "inactive",
        Some(ProcessStatusSymbol::Connect) => "connect",
        Some(ProcessStatusSymbol::Failed) => "failed",
        _ => "inactive",
    }
}

fn signal_process_not_running(eval: &super::eval::Context, id: ProcessId) -> Flow {
    signal_process_not_running_in_manager(&eval.processes, id)
}

fn signal_process_not_running_in_manager(processes: &ProcessManager, id: ProcessId) -> Flow {
    let (name, reason) = processes
        .get_any(id)
        .map(|proc| {
            (
                process_name_runtime(proc.name),
                stale_process_not_running_reason(&proc.status),
            )
        })
        .unwrap_or_else(|| (id.to_string(), "inactive"));
    signal(
        "error",
        vec![Value::string(format!(
            "Process {name} not running: {reason}\n"
        ))],
    )
}

/// Decode a process designator into a raw `ProcessId` candidate.
///
/// This is the single root that maps a Lisp value to a process key.  Like GNU's
/// `get_process` / `CHECK_PROCESS`, only a genuine process object designates a
/// process by identity — a bare integer is NOT a process (GNU signals
/// `wrong-type-argument processp`).  It does NOT validate that the id still
/// names a live/known process; callers layer their own `get`/`get_any` checks
/// on top.  Name-string and nil (current-buffer) designators are handled by the
/// individual resolvers since they need manager/buffer state.
pub(crate) fn process_value_to_id(value: &Value) -> Option<ProcessId> {
    value.as_process_id()
}

fn resolve_process_or_wrong_type(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<ProcessId, Flow> {
    if let Some(id) = process_value_to_id(value) {
        return if eval.processes.get(id).is_some() {
            Ok(id)
        } else {
            Err(signal_wrong_type_processp(*value))
        };
    }
    match value.kind() {
        ValueKind::String => {
            let name = process_owned_runtime_string(*value);
            eval.processes
                .find_by_name(&name)
                .ok_or_else(|| signal_wrong_type_processp(*value))
        }
        _ => Err(signal_wrong_type_processp(*value)),
    }
}

fn resolve_process_or_wrong_type_any(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<ProcessId, Flow> {
    resolve_process_or_wrong_type_any_in_manager(&eval.processes, value)
}

fn resolve_process_or_wrong_type_any_in_manager(
    processes: &ProcessManager,
    value: &Value,
) -> Result<ProcessId, Flow> {
    if let Some(id) = process_value_to_id(value) {
        return if processes.get_any(id).is_some() {
            Ok(id)
        } else {
            Err(signal_wrong_type_processp(*value))
        };
    }
    match value.kind() {
        ValueKind::String => {
            let name = process_owned_runtime_string(*value);
            processes
                .find_by_name(&name)
                .ok_or_else(|| signal_wrong_type_processp(*value))
        }
        _ => Err(signal_wrong_type_processp(*value)),
    }
}

fn resolve_process_or_missing_error(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<ProcessId, Flow> {
    resolve_process_or_missing_error_in_manager(&eval.processes, value)
}

fn resolve_process_or_missing_error_in_manager(
    processes: &ProcessManager,
    value: &Value,
) -> Result<ProcessId, Flow> {
    match value.kind() {
        ValueKind::String => {
            let name = process_owned_runtime_string(*value);
            processes
                .find_by_name(&name)
                .ok_or_else(|| signal_process_does_not_exist(&name))
        }
        _ => resolve_process_or_wrong_type_any_in_manager(processes, value),
    }
}

fn resolve_process_or_missing_error_any(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<ProcessId, Flow> {
    resolve_process_or_missing_error_any_in_manager(&eval.processes, value)
}

fn resolve_process_or_missing_error_any_in_manager(
    processes: &ProcessManager,
    value: &Value,
) -> Result<ProcessId, Flow> {
    match value.kind() {
        ValueKind::String => {
            let name = process_owned_runtime_string(*value);
            processes
                .find_by_name(&name)
                .ok_or_else(|| signal_process_does_not_exist(&name))
        }
        _ => resolve_process_or_wrong_type_any_in_manager(processes, value),
    }
}

fn resolve_process_for_status(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<Option<ProcessId>, Flow> {
    if let Some(id) = process_value_to_id(value) {
        return if eval.processes.get_any(id).is_some() {
            Ok(Some(id))
        } else {
            Err(signal_wrong_type_processp(*value))
        };
    }
    match value.kind() {
        ValueKind::String => {
            let name = process_owned_runtime_string(*value);
            Ok(eval.processes.find_by_name(&name))
        }
        _ => Err(signal_wrong_type_processp(*value)),
    }
}

fn resolve_buffer_for_process_lookup_in_state(
    frames: &FrameManager,
    buffers: &BufferManager,
    value: &Value,
) -> Result<Option<crate::buffer::BufferId>, Flow> {
    match value.kind() {
        ValueKind::Nil => Ok(frames
            .selected_frame()
            .and_then(|frame| frame.selected_window())
            .and_then(|window| window.buffer_id())),
        ValueKind::String => {
            let name_str = process_owned_runtime_string(*value);
            Ok(buffers.find_buffer_by_name(&name_str))
        }
        ValueKind::Veclike(VecLikeType::Buffer) => {
            let bid = value.as_buffer_id().unwrap();
            Ok(buffers.get(bid).map(|_| bid))
        }
        _ => Err(signal_wrong_type_string(*value)),
    }
}

/// Resolve a live process designator for compatibility builtins.
///
/// NeoVM currently models process handles as integer ids.  These helpers treat
/// a live process id as a process designator for runtime parity surfaces.
fn resolve_live_process_designator(
    eval: &super::eval::Context,
    value: &Value,
) -> Option<ProcessId> {
    resolve_live_process_designator_in_manager(&eval.processes, value)
}

fn resolve_live_process_designator_in_manager(
    processes: &ProcessManager,
    value: &Value,
) -> Option<ProcessId> {
    let id = process_value_to_id(value)?;
    processes.get(id).map(|_| id)
}

fn resolve_live_process_or_wrong_type(
    eval: &super::eval::Context,
    value: &Value,
) -> Result<ProcessId, Flow> {
    resolve_live_process_or_wrong_type_in_manager(&eval.processes, value)
}

fn resolve_live_process_or_wrong_type_in_manager(
    processes: &ProcessManager,
    value: &Value,
) -> Result<ProcessId, Flow> {
    resolve_live_process_designator_in_manager(processes, value).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), *value],
        )
    })
}

fn current_thread_handle(threads: &ThreadManager) -> Value {
    threads
        .thread_handle(threads.current_thread_id())
        .unwrap_or(Value::NIL)
}

fn is_stale_process_id_designator(eval: &super::eval::Context, value: &Value) -> bool {
    is_stale_process_id_designator_in_manager(&eval.processes, value)
}

fn is_stale_process_id_designator_in_manager(processes: &ProcessManager, value: &Value) -> bool {
    match process_value_to_id(value) {
        Some(id) if id > 0 => {
            processes.get(id).is_none()
                && (processes.get_any(id).is_some() || processes.was_issued_id(id))
        }
        _ => false,
    }
}

fn resolve_optional_process_or_current_buffer(
    eval: &super::eval::Context,
    value: Option<&Value>,
) -> Result<ProcessId, Flow> {
    resolve_optional_process_or_current_buffer_in_state(&eval.processes, &eval.buffers, value)
}

fn resolve_optional_process_or_current_buffer_in_state(
    processes: &ProcessManager,
    buffers: &BufferManager,
    value: Option<&Value>,
) -> Result<ProcessId, Flow> {
    if let Some(v) = value {
        if !v.is_nil() {
            return resolve_process_or_missing_error_in_manager(processes, v);
        }
    }

    let current_buffer = buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;

    processes.find_by_buffer_id(current_buffer).ok_or_else(|| {
        signal(
            "error",
            vec![Value::string(format!(
                "Buffer {} has no process",
                buffers
                    .get(current_buffer)
                    .map(|buffer| buffer.name_runtime_string_owned())
                    .unwrap_or_else(|| "<deleted buffer>".to_string())
            ))],
        )
    })
}

fn process_live_status_value(process: &Process) -> Value {
    if process_stopped_for_io(process) {
        return Value::list(vec![Value::symbol("stop")]);
    }
    let status = process.status;
    let kind = process.kind;
    match ProcessStatusSymbol::from_status_value(status) {
        Some(ProcessStatusSymbol::Run) => match kind {
            ProcessKind::Network => Value::list(vec![
                Value::symbol("listen"),
                Value::symbol("connect"),
                Value::symbol("stop"),
            ]),
            ProcessKind::Pipe => Value::list(vec![
                Value::symbol("open"),
                Value::symbol("listen"),
                Value::symbol("connect"),
                Value::symbol("stop"),
            ]),
            _ => Value::list(vec![
                Value::symbol("run"),
                Value::symbol("open"),
                Value::symbol("listen"),
                Value::symbol("connect"),
                Value::symbol("stop"),
            ]),
        },
        Some(ProcessStatusSymbol::Stop) => Value::list(vec![Value::symbol("stop")]),
        Some(ProcessStatusSymbol::Open) => Value::list(vec![
            Value::symbol("open"),
            Value::symbol("listen"),
            Value::symbol("connect"),
            Value::symbol("stop"),
        ]),
        Some(ProcessStatusSymbol::Listen) => Value::list(vec![
            Value::symbol("listen"),
            Value::symbol("connect"),
            Value::symbol("stop"),
        ]),
        Some(ProcessStatusSymbol::Connect) => Value::list(vec![Value::symbol("connect")]),
        _ => Value::NIL,
    }
}

pub(crate) fn process_public_status_symbol(process: &Process) -> Value {
    if process_stopped_for_io(process) {
        return ProcessStatusSymbol::Stop.value();
    }
    match ProcessStatusSymbol::from_status_value(process.status) {
        Some(ProcessStatusSymbol::Run) => match process.kind {
            ProcessKind::Network => {
                if process_contact_server_p(process) {
                    Value::symbol("listen")
                } else {
                    Value::symbol("open")
                }
            }
            ProcessKind::Pipe => Value::symbol("open"),
            _ => Value::symbol("run"),
        },
        Some(ProcessStatusSymbol::Stop) => ProcessStatusSymbol::Stop.value(),
        Some(ProcessStatusSymbol::Exit) => match process.kind {
            ProcessKind::Real => ProcessStatusSymbol::Exit.value(),
            _ => ProcessStatusSymbol::Closed.value(),
        },
        Some(ProcessStatusSymbol::Signal) => match process.kind {
            ProcessKind::Real => Value::symbol("signal"),
            _ => Value::symbol("closed"),
        },
        Some(ProcessStatusSymbol::Open) => ProcessStatusSymbol::Open.value(),
        Some(ProcessStatusSymbol::Listen) => ProcessStatusSymbol::Listen.value(),
        Some(ProcessStatusSymbol::Closed) => ProcessStatusSymbol::Closed.value(),
        Some(ProcessStatusSymbol::Connect) => ProcessStatusSymbol::Connect.value(),
        Some(ProcessStatusSymbol::Failed) => ProcessStatusSymbol::Failed.value(),
        _ => Value::NIL,
    }
}

fn default_process_tty_name() -> String {
    // Fallback TTY name when the actual PTY slave path is not available.
    "/dev/pts/0".to_string()
}

// ---------------------------------------------------------------------------
// Bootstrap variables
// ---------------------------------------------------------------------------

pub fn register_bootstrap_vars(obarray: &mut super::symbol::Obarray) {
    obarray.set_symbol_value("process-connection-type", Value::T);
    obarray.make_special("process-connection-type");
    // GNU `process.c` `syms_of_process` DEFVAR_LISPs
    // `process-adaptive-read-buffering` (default nil); it controls the
    // short-read delay heuristic in `read_process_output` and is set per
    // process at `start-process'/`make-process' time
    // (`p->adaptive_read_buffering`).  It must be *bound* (to nil) so that
    // `(boundp 'process-adaptive-read-buffering)` is t and reading the
    // variable does not signal `void-variable`; e.g. `tramp-sh.el` binds it
    // with `(let ((process-adaptive-read-buffering nil)) ...)`.  Without this
    // DEFVAR, code that reads the variable before calling a (non-existent)
    // helper sees `void-variable` instead of reaching the real error.
    obarray.set_symbol_value("process-adaptive-read-buffering", Value::NIL);
    obarray.make_special("process-adaptive-read-buffering");
    // GNU `gnutls.c` provides this via `DEFVAR_INT ("gnutls-log-level",
    // global_gnutls_log_level)` (default 0).  `gnutls.el` only forward-declares
    // it (`(defvar gnutls-log-level)  ; gnutls.c`), so without the C-side
    // definition it is void and `gnutls-negotiate` errors on
    // `:loglevel ,gnutls-log-level` before it ever reaches the (working,
    // TLS-capable) `gnutls-boot` -- breaking every package download and
    // thus `use-package`.  See https://github.com/eval-exec/neomacs/issues/121.
    obarray.set_symbol_value("gnutls-log-level", Value::fixnum(0));
    obarray.make_special("gnutls-log-level");
    // GNU `gnutls.c` always DEFVAR_LISPs `libgnutls-version`; when Emacs is
    // built without libgnutls, the documented value is -1.  Neomacs exposes a
    // `gnutls-boot` compatibility API over Rust TLS rather than linking
    // libgnutls, so keep the variable bound without pretending to have a
    // libgnutls version.  `nsm.el` reads this during HTTPS package refresh.
    obarray.set_symbol_value("libgnutls-version", Value::fixnum(-1));
    obarray.make_special("libgnutls-version");
}

/// Check whether `process-connection-type` is truthy (non-nil).
///
/// GNU Emacs defaults this to `t`, meaning processes should use PTYs.
/// When nil, pipe-based I/O is used instead.
fn process_connection_type_is_pty(obarray: &super::symbol::Obarray) -> bool {
    match obarray.symbol_value("process-connection-type") {
        Some(v) if v.is_nil() => false,
        Some(_) => true,
        // Default is t (PTY) when the variable has not been set.
        None => true,
    }
}

fn signal_wrong_type_bufferp(value: Value) -> Flow {
    signal("wrong-type-argument", vec![Value::symbol("bufferp"), value])
}

fn signal_wrong_type_threadp(value: Value) -> Flow {
    signal("wrong-type-argument", vec![Value::symbol("threadp"), value])
}

fn signal_wrong_type_integerp(value: Value) -> Flow {
    signal(
        "wrong-type-argument",
        vec![Value::symbol("integerp"), value],
    )
}

fn signal_wrong_type_numberp(value: Value) -> Flow {
    signal("wrong-type-argument", vec![Value::symbol("numberp"), value])
}

fn signal_undefined_signal_name(name: &str) -> Flow {
    signal(
        "error",
        vec![Value::string(format!("Undefined signal name {name}"))],
    )
}

fn resolve_optional_process_with_explicit_return(
    eval: &super::eval::Context,
    value: Option<&Value>,
) -> Result<(ProcessId, Value), Flow> {
    resolve_optional_process_with_explicit_return_in_state(&eval.processes, &eval.buffers, value)
}

fn resolve_optional_process_with_explicit_return_in_state(
    processes: &ProcessManager,
    buffers: &BufferManager,
    value: Option<&Value>,
) -> Result<(ProcessId, Value), Flow> {
    if let Some(v) = value {
        if !v.is_nil() && is_stale_process_id_designator_in_manager(processes, v) {
            if let Some(id) = process_value_to_id(v) {
                return Err(signal_process_not_active_in_manager(processes, id));
            }
        }
    }
    if let Some(v) = value {
        if !v.is_nil() {
            let id = resolve_process_or_missing_error_in_manager(processes, v)?;
            return Ok((id, *v));
        }
    }
    let id = resolve_optional_process_or_current_buffer_in_state(processes, buffers, value)?;
    Ok((id, Value::NIL))
}

enum SignalProcessTarget {
    Process(ProcessId),
    MissingNamedProcess,
    Pid(i64),
}

fn resolve_signal_process_target(
    eval: &super::eval::Context,
    value: Option<&Value>,
) -> Result<SignalProcessTarget, Flow> {
    resolve_signal_process_target_in_state(&eval.processes, &eval.buffers, value)
}

fn resolve_signal_process_target_in_state(
    processes: &ProcessManager,
    buffers: &BufferManager,
    value: Option<&Value>,
) -> Result<SignalProcessTarget, Flow> {
    if let Some(v) = value {
        if !v.is_nil() {
            // A first-class process object designates that process while live;
            // once it has exited, GNU still signals the recorded OS pid.
            if let Some(id) = v.as_process_id() {
                return if processes.get(id).is_some() {
                    Ok(SignalProcessTarget::Process(id))
                } else {
                    Ok(SignalProcessTarget::Pid(id as i64))
                };
            }
            return match v.kind() {
                ValueKind::String => {
                    let name_str = process_owned_runtime_string(*v);
                    Ok(match processes.find_by_name(&name_str) {
                        Some(id) => SignalProcessTarget::Process(id),
                        None => SignalProcessTarget::MissingNamedProcess,
                    })
                }
                // GNU `Fsignal_process` treats a bare integer as a literal OS
                // PID, not a process-object id.
                ValueKind::Fixnum(pid) if pid >= 0 => {
                    let id = pid as ProcessId;
                    if processes.get(id).is_some() {
                        Ok(SignalProcessTarget::Process(id))
                    } else {
                        Ok(SignalProcessTarget::Pid(pid))
                    }
                }
                _ => Err(signal_wrong_type_processp(*v)),
            };
        }
    }

    let id = resolve_optional_process_or_current_buffer_in_state(processes, buffers, value)?;
    Ok(SignalProcessTarget::Process(id))
}

fn parse_signal_number(value: &Value) -> Result<i32, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n as i32),
        ValueKind::String => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), *value],
        )),
        _ => {
            // Borrow the symbol name before consuming it
            let sym_name = value.as_symbol_name().map(|s| s.to_owned());
            if let Some(name) = sym_name {
                Err(signal_undefined_signal_name(&name))
            } else {
                Err(signal_wrong_type_integerp(*value))
            }
        }
    }
}

fn pid_exists(pid: i64) -> bool {
    if pid < 0 {
        return false;
    }
    std::fs::metadata(format!("/proc/{pid}")).is_ok()
}

#[derive(Clone, Debug)]
struct ProcStatSnapshot {
    comm: String,
    state: String,
    ppid: i64,
    pgrp: i64,
    sess: i64,
    tpgid: i64,
    minflt: i64,
    majflt: i64,
    cminflt: i64,
    cmajflt: i64,
    utime_ticks: i64,
    stime_ticks: i64,
    cutime_ticks: i64,
    cstime_ticks: i64,
    pri: i64,
    nice: i64,
    thcount: i64,
    start_ticks: i64,
    vsize: i64,
    rss: i64,
    ttname: String,
}

impl ProcStatSnapshot {
    fn fallback(pid: i64) -> Self {
        Self {
            comm: String::new(),
            state: String::new(),
            ppid: 0,
            pgrp: 0,
            sess: 0,
            tpgid: 0,
            minflt: 0,
            majflt: 0,
            cminflt: 0,
            cmajflt: 0,
            utime_ticks: 0,
            stime_ticks: 0,
            cutime_ticks: 0,
            cstime_ticks: 0,
            pri: 0,
            nice: 0,
            thcount: 0,
            start_ticks: 0,
            vsize: 0,
            rss: 0,
            ttname: read_proc_tty_name(pid),
        }
    }
}

fn parse_stat_i64_field(fields: &[&str], index: usize) -> Option<i64> {
    fields.get(index)?.parse::<i64>().ok()
}

#[cfg(unix)]
fn page_size_kb() -> i64 {
    let page_size_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size_bytes <= 0 {
        4
    } else {
        ((page_size_bytes as i64) / 1024).max(1)
    }
}

#[cfg(not(unix))]
fn page_size_kb() -> i64 {
    4
}

#[cfg(not(target_os = "windows"))]
fn clock_ticks_per_second() -> i64 {
    // SAFETY: `sysconf(_SC_CLK_TCK)` has no additional preconditions.
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks <= 0 { 100 } else { ticks as i64 }
}

#[cfg(target_os = "windows")]
fn clock_ticks_per_second() -> i64 {
    100
}

fn read_proc_tty_name(pid: i64) -> String {
    std::fs::read_link(format!("/proc/{pid}/fd/0"))
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string())
}

fn parse_proc_cmdline(pid: i64) -> String {
    let bytes = match std::fs::read(format!("/proc/{pid}/cmdline")) {
        Ok(bytes) => bytes,
        Err(_) => return String::new(),
    };
    let mut args = Vec::new();
    for chunk in bytes.split(|b| *b == 0) {
        if chunk.is_empty() {
            continue;
        }
        args.push(String::from_utf8_lossy(chunk).into_owned());
    }
    args.join(" ")
}

fn parse_proc_boot_time_secs() -> Option<i64> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    for line in stat.lines() {
        if let Some(rest) = line.strip_prefix("btime ") {
            return rest.trim().parse::<i64>().ok();
        }
    }
    None
}

fn parse_total_memory_kb() -> Option<i64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb = rest.split_whitespace().next()?.parse::<i64>().ok()?;
            return Some(kb);
        }
    }
    None
}

fn ticks_to_secs_usecs(ticks: i64, hz: i64) -> (i64, i64) {
    if hz <= 0 {
        return (0, 0);
    }
    let secs = ticks.div_euclid(hz);
    let rem = ticks.rem_euclid(hz);
    let usecs = ((rem as i128) * 1_000_000i128 / (hz as i128)) as i64;
    (secs, usecs)
}

fn time_list_from_secs_usecs(secs: i64, usecs: i64) -> Value {
    let high = (secs >> 16) & 0xFFFF_FFFF;
    let low = secs & 0xFFFF;
    Value::list(vec![
        Value::fixnum(high),
        Value::fixnum(low),
        Value::fixnum(usecs.clamp(0, 999_999)),
        Value::fixnum(0),
    ])
}

fn time_list_from_ticks(ticks: i64, hz: i64) -> Value {
    let (secs, usecs) = ticks_to_secs_usecs(ticks, hz);
    time_list_from_secs_usecs(secs, usecs)
}

fn now_epoch_secs_usecs() -> Option<(i64, i64)> {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(dur) => Some((dur.as_secs() as i64, dur.subsec_micros() as i64)),
        Err(_) => None,
    }
}

fn nonnegative_time_diff(now: (i64, i64), then: (i64, i64)) -> (i64, i64) {
    let (now_secs, now_usecs) = now;
    let (then_secs, then_usecs) = then;
    if (now_secs, now_usecs) < (then_secs, then_usecs) {
        return (0, 0);
    }
    let mut secs = now_secs - then_secs;
    let mut usecs = now_usecs - then_usecs;
    if usecs < 0 {
        secs -= 1;
        usecs += 1_000_000;
    }
    (secs, usecs)
}

fn parse_proc_stat_snapshot(pid: i64) -> Option<ProcStatSnapshot> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let open_paren = stat.find('(')?;
    let close_paren = stat.rfind(')')?;
    if close_paren <= open_paren {
        return None;
    }

    let comm = stat.get((open_paren + 1)..close_paren)?.to_string();
    let trailing = stat.get((close_paren + 1)..)?.trim_start();
    let fields: Vec<&str> = trailing.split_whitespace().collect();
    if fields.len() < 22 {
        return None;
    }

    let state = fields[0].to_string();
    let ppid = parse_stat_i64_field(&fields, 1)?;
    let pgrp = parse_stat_i64_field(&fields, 2)?;
    let sess = parse_stat_i64_field(&fields, 3)?;
    let tpgid = parse_stat_i64_field(&fields, 5)?;
    let minflt = parse_stat_i64_field(&fields, 7)?;
    let cminflt = parse_stat_i64_field(&fields, 8)?;
    let majflt = parse_stat_i64_field(&fields, 9)?;
    let cmajflt = parse_stat_i64_field(&fields, 10)?;
    let utime_ticks = parse_stat_i64_field(&fields, 11)?;
    let stime_ticks = parse_stat_i64_field(&fields, 12)?;
    let cutime_ticks = parse_stat_i64_field(&fields, 13)?;
    let cstime_ticks = parse_stat_i64_field(&fields, 14)?;
    let pri = parse_stat_i64_field(&fields, 15)?;
    let nice = parse_stat_i64_field(&fields, 16)?;
    let thcount = parse_stat_i64_field(&fields, 17)?;
    let start_ticks = parse_stat_i64_field(&fields, 19)?;
    let vsize = parse_stat_i64_field(&fields, 20)?;
    let rss_pages = parse_stat_i64_field(&fields, 21)?;
    let rss = rss_pages.saturating_mul(page_size_kb());
    let ttname = read_proc_tty_name(pid);

    Some(ProcStatSnapshot {
        comm,
        state,
        ppid,
        pgrp,
        sess,
        tpgid,
        minflt,
        majflt,
        cminflt,
        cmajflt,
        utime_ticks,
        stime_ticks,
        cutime_ticks,
        cstime_ticks,
        pri,
        nice,
        thcount,
        start_ticks,
        vsize,
        rss,
        ttname,
    })
}

fn parse_effective_ids_from_proc_status(pid: i64) -> Option<(u32, u32)> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let mut euid = None;
    let mut egid = None;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let fields: Vec<&str> = rest.split_whitespace().collect();
            if fields.len() >= 2 {
                euid = fields[1].parse::<u32>().ok();
            }
        } else if let Some(rest) = line.strip_prefix("Gid:") {
            let fields: Vec<&str> = rest.split_whitespace().collect();
            if fields.len() >= 2 {
                egid = fields[1].parse::<u32>().ok();
            }
        }
        if euid.is_some() && egid.is_some() {
            break;
        }
    }
    Some((euid?, egid?))
}

#[cfg(not(target_os = "windows"))]
fn lookup_user_name(uid: u32) -> Option<String> {
    // SAFETY: libc returns either null or a valid passwd struct pointer.
    let user = unsafe { libc::getpwuid(uid as libc::uid_t) };
    if user.is_null() {
        return None;
    }
    // SAFETY: `user` is non-null and `pw_name` is a valid C string pointer.
    let name_ptr = unsafe { (*user).pw_name };
    if name_ptr.is_null() {
        return None;
    }
    // SAFETY: `name_ptr` is a valid NUL-terminated C string.
    Some(
        unsafe { CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(target_os = "windows")]
fn lookup_user_name(_uid: u32) -> Option<String> {
    None
}

#[cfg(not(target_os = "windows"))]
fn lookup_group_name(gid: u32) -> Option<String> {
    // SAFETY: libc returns either null or a valid group struct pointer.
    let group = unsafe { libc::getgrgid(gid as libc::gid_t) };
    if group.is_null() {
        return None;
    }
    // SAFETY: `group` is non-null and `gr_name` is a valid C string pointer.
    let name_ptr = unsafe { (*group).gr_name };
    if name_ptr.is_null() {
        return None;
    }
    // SAFETY: `name_ptr` is a valid NUL-terminated C string.
    Some(
        unsafe { CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(target_os = "windows")]
fn lookup_group_name(_gid: u32) -> Option<String> {
    None
}

fn parse_make_process_command(value: &Value) -> Result<Vec<LispString>, Flow> {
    let as_vec: Option<Vec<Value>> = match value.kind() {
        ValueKind::Veclike(VecLikeType::Vector) => Some(value.as_vector_data().unwrap().clone()),
        ValueKind::Cons | ValueKind::Nil => list_to_vec(value),
        _ => None,
    };

    let Some(items) = as_vec else {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("sequencep"), *value],
        ));
    };

    items
        .into_iter()
        .map(|item| {
            super::builtins::expect_lisp_string(&item)
                .cloned()
                .map_err(|_| signal_wrong_type_string(item))
        })
        .collect()
}

fn parse_make_process_buffer(
    eval: &mut super::eval::Context,
    value: &Value,
) -> Result<Value, Flow> {
    parse_make_process_buffer_in_state(&mut eval.buffers, value)
}

fn parse_make_process_buffer_in_state(
    buffers: &mut BufferManager,
    value: &Value,
) -> Result<Value, Flow> {
    match value.kind() {
        ValueKind::Nil => Ok(Value::NIL),
        ValueKind::String => {
            let name_str = process_owned_runtime_string(*value);
            let id = buffers
                .find_buffer_by_name(&name_str)
                .unwrap_or_else(|| buffers.create_buffer(&name_str));
            Ok(Value::make_buffer(id))
        }
        ValueKind::Veclike(VecLikeType::Buffer) => {
            let bid = value.as_buffer_id().unwrap();
            buffers
                .get(bid)
                .map(|_| *value)
                .ok_or_else(|| signal("error", vec![Value::string("Selecting deleted buffer")]))
        }
        _ => Err(signal_wrong_type_string(*value)),
    }
}

fn expect_integer(value: &Value) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) => Ok(n),
        _ => Err(signal_wrong_type_integerp(*value)),
    }
}

fn value_as_nonnegative_integer(value: &Value) -> Option<i64> {
    match value.kind() {
        ValueKind::Fixnum(n) if n >= 0 => Some(n),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
enum NetworkAddressFamily {
    #[strum(serialize = "ipv4")]
    Ipv4,
    #[strum(serialize = "ipv6")]
    Ipv6,
}

impl NetworkAddressFamily {
    fn from_symbol_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum NetworkProcessFamilySymbol {
    Local,
    Ipv4,
    Ipv6,
}

impl NetworkProcessFamilySymbol {
    fn from_symbol_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

fn network_loopback_host_for_family(family: Option<NetworkProcessFamilySymbol>) -> &'static str {
    match family {
        Some(NetworkProcessFamilySymbol::Ipv6) => "::1",
        _ => "127.0.0.1",
    }
}

fn parse_network_host(
    value: &Value,
    family: Option<NetworkProcessFamilySymbol>,
) -> Result<Option<String>, Flow> {
    if value.is_nil() {
        return Ok(None);
    }
    if value.as_symbol_name() == Some("local") {
        return Ok(Some(network_loopback_host_for_family(family).to_string()));
    }
    match value.kind() {
        ValueKind::String => Ok(Some(process_owned_runtime_string(*value))),
        _ => Err(signal_wrong_type_string(*value)),
    }
}

fn network_service_protocol(socket_type: NetworkSocketType) -> &'static str {
    match socket_type {
        NetworkSocketType::Datagram => "udp",
        _ => "tcp",
    }
}

#[cfg(unix)]
fn lookup_network_service_port(service: &str, protocol: &str) -> Option<u16> {
    let service = CString::new(service).ok()?;
    let protocol = CString::new(protocol).ok()?;
    let entry = unsafe { libc::getservbyname(service.as_ptr(), protocol.as_ptr()) };
    if entry.is_null() {
        None
    } else {
        Some(u16::from_be(unsafe { (*entry).s_port as u16 }))
    }
}

#[cfg(not(unix))]
fn lookup_network_service_port(_service: &str, _protocol: &str) -> Option<u16> {
    None
}

fn parse_network_service_port(
    value: &Value,
    server: bool,
    socket_type: NetworkSocketType,
) -> Result<u16, Flow> {
    match value.kind() {
        ValueKind::T if server => Ok(0),
        ValueKind::Fixnum(port) if (0..(1 << 16)).contains(&port) => Ok(port as u16),
        ValueKind::String => {
            let service = process_owned_runtime_string(*value);
            if let Ok(port) = service.parse::<u16>() {
                return Ok(port);
            }
            lookup_network_service_port(&service, network_service_protocol(socket_type)).ok_or_else(
                || {
                    signal(
                        "error",
                        vec![Value::string(format!("Unknown service: {}", service))],
                    )
                },
            )
        }
        _ => Err(signal_wrong_type_string(*value)),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NetworkAddressSpec {
    Inet(SocketAddr),
    #[cfg(unix)]
    Local(std::path::PathBuf),
}

fn parse_network_address_spec(value: &Value) -> Result<NetworkAddressSpec, Flow> {
    #[cfg(unix)]
    if matches!(value.kind(), ValueKind::String) {
        return Ok(NetworkAddressSpec::Local(
            crate::emacs_core::fileio::lisp_file_name_to_path_buf(
                super::builtins::expect_lisp_string(value)?,
            ),
        ));
    }

    let Some(items) = value.as_vector_data() else {
        return Err(signal("error", vec![Value::string("Malformed :address")]));
    };

    match items.len() {
        5 => {
            let a = parse_lisp_sockaddr_part(items[0], 255)?;
            let b = parse_lisp_sockaddr_part(items[1], 255)?;
            let c = parse_lisp_sockaddr_part(items[2], 255)?;
            let d = parse_lisp_sockaddr_part(items[3], 255)?;
            let port = parse_lisp_sockaddr_part(items[4], u16::MAX as i64)?;
            Ok(NetworkAddressSpec::Inet(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(a as u8, b as u8, c as u8, d as u8)),
                port as u16,
            )))
        }
        9 => {
            let mut segments = [0_u16; 8];
            for (idx, segment) in segments.iter_mut().enumerate() {
                *segment = parse_lisp_sockaddr_part(items[idx], u16::MAX as i64)? as u16;
            }
            let port = parse_lisp_sockaddr_part(items[8], u16::MAX as i64)?;
            Ok(NetworkAddressSpec::Inet(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(
                    segments[0],
                    segments[1],
                    segments[2],
                    segments[3],
                    segments[4],
                    segments[5],
                    segments[6],
                    segments[7],
                )),
                port as u16,
            )))
        }
        _ => Err(signal("error", vec![Value::string("Malformed :address")])),
    }
}

fn parse_lisp_sockaddr_part(value: Value, max: i64) -> Result<i64, Flow> {
    match value.kind() {
        ValueKind::Fixnum(n) if (0..=max).contains(&n) => Ok(n),
        _ => Err(signal("error", vec![Value::string("Malformed :address")])),
    }
}

fn socket_addr_to_lisp_value(addr: SocketAddr) -> Value {
    match addr {
        SocketAddr::V4(v4) => {
            let octets = v4.ip().octets();
            int_vector(&[
                octets[0] as i64,
                octets[1] as i64,
                octets[2] as i64,
                octets[3] as i64,
                v4.port() as i64,
            ])
        }
        SocketAddr::V6(v6) => {
            let segments = v6.ip().segments();
            let mut vals = [0_i64; 9];
            for (idx, &seg) in segments.iter().enumerate() {
                vals[idx] = seg as i64;
            }
            vals[8] = v6.port() as i64;
            int_vector(&vals)
        }
    }
}

#[cfg(unix)]
fn unix_socket_addr_to_runtime_string(addr: Option<UnixSocketAddr>) -> String {
    addr.and_then(|addr| {
        addr.as_pathname()
            .map(|path| path.as_os_str().to_string_lossy().into_owned())
    })
    .unwrap_or_default()
}

#[cfg(unix)]
fn socket2_unix_sockaddr_to_runtime_string(addr: Option<&SockAddr>) -> String {
    addr.and_then(|addr| {
        addr.as_pathname()
            .map(|path| path.as_os_str().to_string_lossy().into_owned())
    })
    .unwrap_or_default()
}

fn validate_network_process_family(value: &Value) -> Result<(), Flow> {
    if value.is_nil()
        || matches!(value.kind(), ValueKind::Fixnum(_))
        || NetworkProcessFamilySymbol::from_symbol_value(value).is_some()
    {
        Ok(())
    } else {
        Err(signal(
            "error",
            vec![Value::string("Unknown address family")],
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum NetworkLookupHint {
    Numeric,
}

impl NetworkLookupHint {
    fn from_symbol_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    fn addrinfo_flags(self) -> i32 {
        match self {
            Self::Numeric => ai_numerichost_flag(),
        }
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

#[cfg(unix)]
fn ai_numerichost_flag() -> i32 {
    libc::AI_NUMERICHOST
}

#[cfg(windows)]
fn ai_numerichost_flag() -> i32 {
    windows_sys::Win32::Networking::WinSock::AI_NUMERICHOST as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum NumProcessorsQuery {
    All,
    Current,
}

impl NumProcessorsQuery {
    fn from_symbol_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkSocketType {
    Stream,
    Datagram,
    #[cfg(unix)]
    Seqpacket,
}

fn parse_network_socket_type(value: &Value) -> Result<NetworkSocketType, Flow> {
    match value.as_symbol_name() {
        _ if value.is_nil() => Ok(NetworkSocketType::Stream),
        Some("datagram") => Ok(NetworkSocketType::Datagram),
        #[cfg(unix)]
        Some("seqpacket") => Ok(NetworkSocketType::Seqpacket),
        Some(_) | None => Err(signal(
            "error",
            vec![Value::string("Unsupported connection type")],
        )),
    }
}

fn validate_network_socket_type(value: &Value) -> Result<(), Flow> {
    parse_network_socket_type(value).map(|_| ())
}

fn network_socket_type_uses_stream_connect(socket_type: NetworkSocketType) -> bool {
    match socket_type {
        NetworkSocketType::Stream => true,
        NetworkSocketType::Datagram => false,
        #[cfg(unix)]
        NetworkSocketType::Seqpacket => true,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum ProcessConnectionType {
    Pipe,
    Pty,
}

impl ProcessConnectionType {
    fn from_symbol_value(value: &Value) -> Option<Self> {
        value.as_symbol_name()?.parse().ok()
    }

    fn uses_pty(self) -> bool {
        matches!(self, Self::Pty)
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        self.into()
    }
}

fn resolve_process_connection_type_use_pty(
    connection_type: Option<&Value>,
    default_use_pty: bool,
) -> Result<bool, Flow> {
    match connection_type {
        None => Ok(default_use_pty),
        Some(value) if value.is_nil() => Ok(default_use_pty),
        Some(value) => ProcessConnectionType::from_symbol_value(value)
            .map(ProcessConnectionType::uses_pty)
            .ok_or_else(|| {
                // GNU `is_pty_from_symbol` (process.c) signals this through
                // `report_file_error ("Unknown connection type", symbol)`, which
                // reads the live `errno`.  At this point in `make-process` (before
                // any program lookup) the residual errno is ENOENT, so GNU emits
                // `(file-missing "Unknown connection type" "No such file or
                // directory" SYMBOL)`.  Match that data list exactly.
                signal_file_errno("Unknown connection type", *value, libc::ENOENT)
            }),
    }
}

#[derive(Clone, Debug)]
struct HostInterfaceEntry {
    name: String,
    family: NetworkAddressFamily,
    address: Value,
    list_broadcast: Value,
    info_broadcast: Value,
    netmask: Value,
    hwaddr: Option<Value>,
    flags: Value,
}

fn vector_nonnegative_integers(value: &Value) -> Option<Vec<i64>> {
    if !value.is_vector() {
        return None;
    };
    let locked = value.as_vector_data().unwrap().clone();
    let mut out = Vec::with_capacity(locked.len());
    for item in locked.iter() {
        out.push(value_as_nonnegative_integer(item)?);
    }
    Some(out)
}

fn int_vector(values: &[i64]) -> Value {
    Value::vector(values.iter().map(|v| Value::fixnum(*v)).collect())
}

fn loopback_ipv4_address() -> Value {
    int_vector(&[127, 0, 0, 1, 0])
}

fn loopback_ipv4_broadcast() -> Value {
    int_vector(&[0, 0, 0, 0, 0])
}

fn loopback_ipv4_netmask() -> Value {
    int_vector(&[255, 0, 0, 0, 0])
}

fn loopback_ipv6_address() -> Value {
    int_vector(&[0, 0, 0, 0, 0, 0, 0, 1, 0])
}

fn loopback_ipv6_broadcast() -> Value {
    int_vector(&[0, 0, 0, 0, 0, 0, 0, 1, 0])
}

fn loopback_ipv6_netmask() -> Value {
    int_vector(&[65535, 65535, 65535, 65535, 65535, 65535, 65535, 65535, 0])
}

fn loopback_hwaddr() -> Value {
    Value::cons(Value::fixnum(772), int_vector(&[0, 0, 0, 0, 0, 0]))
}

fn loopback_flags() -> Value {
    Value::list(vec![
        Value::symbol("running"),
        Value::symbol("loopback"),
        Value::symbol("up"),
    ])
}

fn zero_network_address(family: NetworkAddressFamily) -> Value {
    match family {
        NetworkAddressFamily::Ipv4 => int_vector(&[0, 0, 0, 0, 0]),
        NetworkAddressFamily::Ipv6 => int_vector(&[0, 0, 0, 0, 0, 0, 0, 0, 0]),
    }
}

fn network_directed_broadcast(
    family: NetworkAddressFamily,
    address: &Value,
    netmask: &Value,
) -> Option<Value> {
    let address_items = vector_nonnegative_integers(address)?;
    let netmask_items = vector_nonnegative_integers(netmask)?;
    match family {
        NetworkAddressFamily::Ipv4 => {
            if address_items.len() != 5 || netmask_items.len() != 5 {
                return None;
            }
            let mut out = [0_i64; 5];
            for idx in 0..4 {
                let addr = u8::try_from(address_items[idx]).ok()?;
                let mask = u8::try_from(netmask_items[idx]).ok()?;
                out[idx] = (addr | !mask) as i64;
            }
            Some(int_vector(&out))
        }
        NetworkAddressFamily::Ipv6 => {
            if address_items.len() != 9 || netmask_items.len() != 9 {
                return None;
            }
            let mut out = [0_i64; 9];
            for idx in 0..8 {
                let addr = u16::try_from(address_items[idx]).ok()?;
                let mask = u16::try_from(netmask_items[idx]).ok()?;
                out[idx] = (addr | !mask) as i64;
            }
            Some(int_vector(&out))
        }
    }
}

fn derive_network_interface_list_broadcast(
    family: NetworkAddressFamily,
    address: &Value,
    netmask: &Value,
    raw_broadcast: &Value,
) -> Value {
    network_directed_broadcast(family, address, netmask).unwrap_or(*raw_broadcast)
}

fn derive_network_interface_info_broadcast(
    family: NetworkAddressFamily,
    address: &Value,
    raw_broadcast: &Value,
) -> Value {
    if raw_broadcast == address {
        zero_network_address(family)
    } else {
        *raw_broadcast
    }
}

fn ip_to_value(ip: IpAddr) -> (NetworkAddressFamily, Value) {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            (
                NetworkAddressFamily::Ipv4,
                int_vector(&[
                    octets[0] as i64,
                    octets[1] as i64,
                    octets[2] as i64,
                    octets[3] as i64,
                    0,
                ]),
            )
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            let mut vals = [0_i64; 9];
            for (idx, &seg) in segments.iter().enumerate() {
                vals[idx] = seg as i64;
            }
            (NetworkAddressFamily::Ipv6, int_vector(&vals))
        }
    }
}

fn resolve_network_lookup_addresses(
    name: &str,
    family: Option<NetworkAddressFamily>,
    hint: Option<NetworkLookupHint>,
) -> Vec<Value> {
    use dns_lookup::{AddrFamily, AddrInfoHints, SockType};

    // Emacs forwards names through C APIs where embedded NUL terminates the
    // effective hostname. Match that behavior instead of rejecting interior NUL.
    let normalized_name = name.split('\0').next().unwrap_or_default();

    let hints = AddrInfoHints {
        flags: hint.map_or(0, NetworkLookupHint::addrinfo_flags),
        socktype: SockType::DGram.into(),
        address: match family {
            Some(NetworkAddressFamily::Ipv4) => AddrFamily::Inet.into(),
            Some(NetworkAddressFamily::Ipv6) => AddrFamily::Inet6.into(),
            None => 0, // AF_UNSPEC
        },
        ..AddrInfoHints::default()
    };

    let addrs = match dns_lookup::getaddrinfo(Some(normalized_name), None, Some(hints)) {
        Ok(iter) => iter,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for result in addrs {
        let info = match result {
            Ok(info) => info,
            Err(_) => continue,
        };
        let (resolved_family, address) = ip_to_value(info.sockaddr.ip());
        let include = match family {
            Some(expected) => expected == resolved_family,
            None => true,
        };
        if include {
            out.push(address);
        }
    }

    out
}

fn parse_mac_addr(mac: &str) -> Option<Value> {
    let mut bytes = Vec::new();
    for part in mac.trim().split(':') {
        if part.is_empty() {
            continue;
        }
        let byte = u8::from_str_radix(part, 16).ok()?;
        bytes.push(Value::fixnum(byte as i64));
    }
    if bytes.is_empty() {
        return None;
    }
    // hatype 1 = ARPHRD_ETHER (Ethernet), the common case
    Some(Value::cons(Value::fixnum(1), Value::vector(bytes)))
}

fn host_interface_snapshot() -> Option<Vec<HostInterfaceEntry>> {
    use network_interface::{Addr, NetworkInterface, NetworkInterfaceConfig};

    let interfaces = NetworkInterface::show().ok()?;

    let mut entries = Vec::new();

    for iface in &interfaces {
        let hwaddr = iface
            .mac_addr
            .as_deref()
            .and_then(|mac| parse_mac_addr(mac));

        for addr in &iface.addr {
            let (family, address, netmask, raw_broadcast) = match addr {
                Addr::V4(v4) => {
                    let ip = v4.ip.octets();
                    let address =
                        int_vector(&[ip[0] as i64, ip[1] as i64, ip[2] as i64, ip[3] as i64, 0]);
                    let netmask = v4
                        .netmask
                        .map(|m| {
                            let o = m.octets();
                            int_vector(&[o[0] as i64, o[1] as i64, o[2] as i64, o[3] as i64, 0])
                        })
                        .unwrap_or_else(|| zero_network_address(NetworkAddressFamily::Ipv4));
                    let broadcast = v4
                        .broadcast
                        .map(|b| {
                            let o = b.octets();
                            int_vector(&[o[0] as i64, o[1] as i64, o[2] as i64, o[3] as i64, 0])
                        })
                        .unwrap_or_else(|| zero_network_address(NetworkAddressFamily::Ipv4));
                    (NetworkAddressFamily::Ipv4, address, netmask, broadcast)
                }
                Addr::V6(v6) => {
                    let segs = v6.ip.segments();
                    let mut vals = [0_i64; 9];
                    for (idx, &seg) in segs.iter().enumerate() {
                        vals[idx] = seg as i64;
                    }
                    let address = int_vector(&vals);
                    let netmask = v6
                        .netmask
                        .map(|m| {
                            let s = m.segments();
                            let mut v = [0_i64; 9];
                            for (idx, &seg) in s.iter().enumerate() {
                                v[idx] = seg as i64;
                            }
                            int_vector(&v)
                        })
                        .unwrap_or_else(|| zero_network_address(NetworkAddressFamily::Ipv6));
                    let broadcast = v6
                        .broadcast
                        .map(|b| {
                            let s = b.segments();
                            let mut v = [0_i64; 9];
                            for (idx, &seg) in s.iter().enumerate() {
                                v[idx] = seg as i64;
                            }
                            int_vector(&v)
                        })
                        .unwrap_or_else(|| zero_network_address(NetworkAddressFamily::Ipv6));
                    (NetworkAddressFamily::Ipv6, address, netmask, broadcast)
                }
            };

            let list_broadcast =
                derive_network_interface_list_broadcast(family, &address, &netmask, &raw_broadcast);
            let info_broadcast =
                derive_network_interface_info_broadcast(family, &address, &raw_broadcast);

            // Approximate flags from available information
            let is_loopback = match addr {
                Addr::V4(v4) => v4.ip.is_loopback(),
                Addr::V6(v6) => v6.ip.is_loopback(),
            };
            let has_broadcast = match addr {
                Addr::V4(v4) => v4.broadcast.is_some(),
                Addr::V6(v6) => v6.broadcast.is_some(),
            };
            let mut flags = vec![Value::symbol("running"), Value::symbol("up")];
            if is_loopback {
                flags.push(Value::symbol("loopback"));
            }
            if has_broadcast {
                flags.push(Value::symbol("broadcast"));
            }

            entries.push(HostInterfaceEntry {
                name: iface.name.clone(),
                family,
                address,
                list_broadcast,
                info_broadcast,
                netmask,
                hwaddr,
                flags: Value::list(flags),
            });
        }
    }

    if entries.is_empty() {
        return None;
    }

    Some(entries)
}

fn interface_entry(name: &str, address: Value, full: bool) -> Value {
    if !full {
        return Value::cons(Value::string(name), address);
    }

    let (broadcast, netmask) = match address.kind() {
        ValueKind::Veclike(VecLikeType::Vector) if address.as_vector_data().unwrap().len() == 9 => {
            (loopback_ipv6_broadcast(), loopback_ipv6_netmask())
        }
        _ => (loopback_ipv4_broadcast(), loopback_ipv4_netmask()),
    };

    Value::list(vec![Value::string(name), address, broadcast, netmask])
}

fn format_ipv4_network_address(items: &[i64], omit_port: bool) -> Option<String> {
    if items.len() != 4 && items.len() != 5 {
        return None;
    }
    let octets: Vec<u8> = items[..4]
        .iter()
        .map(|v| u8::try_from(*v).ok())
        .collect::<Option<Vec<_>>>()?;
    let addr = format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]);
    if items.len() == 5 && !omit_port {
        let port = u16::try_from(items[4]).ok()?;
        Some(format!("{addr}:{port}"))
    } else {
        Some(addr)
    }
}

fn format_ipv6_network_address(items: &[i64], omit_port: bool) -> Option<String> {
    if items.len() != 8 && items.len() != 9 {
        return None;
    }
    let mut segments = Vec::with_capacity(8);
    for value in &items[..8] {
        let segment = u16::try_from(*value).ok()?;
        segments.push(format!("{segment:x}"));
    }
    let addr = segments.join(":");
    if items.len() == 9 && !omit_port {
        let port = u16::try_from(items[8]).ok()?;
        Some(format!("[{addr}]:{port}"))
    } else {
        Some(addr)
    }
}

// ---------------------------------------------------------------------------
// Builtins (eval-dependent)
// ---------------------------------------------------------------------------

/// (clone-process PROCESS &optional NAME) -> process
pub(crate) fn builtin_clone_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("clone-process", &args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("clone-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let id = resolve_process_or_wrong_type_any(eval, &args[0])?;
    Ok(Value::make_process(id))
}

/// (internal-default-interrupt-process &optional PROCESS CURRENT-GROUP) -> process-or-nil
pub(crate) fn builtin_internal_default_interrupt_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_internal_default_interrupt_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_internal_default_interrupt_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("internal-default-interrupt-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let (id, ret) =
        resolve_optional_process_with_explicit_return_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        proc.status = process_status_signal_value(2);
    }
    Ok(ret)
}

/// (internal-default-signal-process PROCESS SIGNAL &optional CURRENT-GROUP) -> int-or-nil
pub(crate) fn builtin_internal_default_signal_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_internal_default_signal_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_internal_default_signal_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-default-signal-process", &args, 2)?;
    if args.len() > 3 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("internal-default-signal-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let signal_num = parse_signal_number(&args[1])?;
    match resolve_signal_process_target_in_state(processes, buffers, args.first())? {
        SignalProcessTarget::Process(id) => {
            if let Some(proc) = processes.get_mut(id) {
                proc.status = process_status_signal_value(signal_num);
            }
            Ok(Value::fixnum(0))
        }
        SignalProcessTarget::MissingNamedProcess => Ok(Value::NIL),
        SignalProcessTarget::Pid(pid) => Ok(Value::fixnum(if pid_exists(pid) { 0 } else { -1 })),
    }
}

fn process_mark_insert_emacs_byte_pos(
    buffers: &BufferManager,
    buf_id: BufferId,
    mark: Value,
) -> EmacsBytePos {
    match super::marker::marker_position_as_int_with_buffers(buffers, &mark) {
        Ok(pos) => buffers
            .get(buf_id)
            .map(|b| b.lisp_pos_to_full_buffer_emacs_byte_pos(LispCharPos1::new(pos)))
            .unwrap_or(EmacsBytePos::ZERO),
        Err(_) => buffers
            .get(buf_id)
            .map(|b| b.full_emacs_byte_range().end())
            .unwrap_or(EmacsBytePos::ZERO),
    }
}

fn adjusted_process_output_point(
    old_point: EmacsBytePos,
    insert_pos: EmacsBytePos,
    inserted_len: EmacsByteLen,
) -> EmacsBytePos {
    if old_point >= insert_pos {
        old_point.add_len(inserted_len)
    } else {
        old_point
    }
}

/// (internal-default-process-filter PROCESS STRING) -> nil
///
/// When no custom filter is set, insert output into the process's associated
/// buffer at the process mark position (or end of buffer when mark is None).
/// This matches GNU Emacs's `internal-default-process-filter` behavior.
pub(crate) fn builtin_internal_default_process_filter(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("internal-default-process-filter", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let text = match args[1].as_lisp_string() {
        Some(text) => text.clone(),
        None => return Err(signal_wrong_type_string(args[1])),
    };
    if text.is_empty() {
        return Ok(Value::NIL);
    }

    // Look up the process buffer and mark.
    let (buf_id, mark) = match eval.processes.get(id) {
        Some(proc) => (proc.buffer.as_buffer_id(), proc.mark),
        None => return Ok(Value::NIL),
    };
    let Some(buf_id) = buf_id else {
        return Ok(Value::NIL);
    };
    if eval.buffers.get(buf_id).is_none() {
        return Ok(Value::NIL);
    }

    // Get mark position or end of buffer (ZV in GNU terms).
    let insert_pos = process_mark_insert_emacs_byte_pos(&eval.buffers, buf_id, mark);

    // Save current point, move point to insert position, insert, then restore.
    let saved_pt = eval.buffers.get(buf_id).map(|b| b.point_emacs_byte_pos());
    let old_read_only = eval.buffers.get(buf_id).map(|b| b.get_read_only());

    // Temporarily clear read-only so process output can be inserted.
    if let Some(buf) = eval.buffers.get_mut(buf_id) {
        buf.set_read_only_value(false);
    }
    let _ = eval.buffers.goto_buffer_emacs_byte_pos(buf_id, insert_pos);

    // Insert text at point (which is now at the mark position).
    eval.buffers.insert_lisp_string_into_buffer(buf_id, &text);

    // The new mark is at point after insertion (insert advances point).
    // If the buffer vanished out from under us the fallback uses text.len()
    // as an approximation; the live buffer path reads the exact Emacs byte
    // position after insertion.
    let new_mark = eval
        .buffers
        .get(buf_id)
        .map(|b| b.point_emacs_byte_pos())
        .unwrap_or(insert_pos.add_len(EmacsByteLen::new(text.sbytes())));

    // Restore read-only flag.
    if let (Some(buf), Some(ro)) = (eval.buffers.get_mut(buf_id), old_read_only) {
        buf.set_read_only_value(ro);
    }

    // Restore original point, adjusted for the insertion.
    let text_byte_len = EmacsByteLen::new(new_mark.get().saturating_sub(insert_pos.get()));
    if let Some(old_pt) = saved_pt {
        let adjusted_pt = adjusted_process_output_point(old_pt, insert_pos, text_byte_len);
        let _ = eval.buffers.goto_buffer_emacs_byte_pos(buf_id, adjusted_pt);
    }

    // Advance the stored process marker.
    if let Some(proc) = eval.processes.get_mut(id) {
        let new_mark_pos = eval
            .buffers
            .get(buf_id)
            .map(|b| Value::fixnum(b.emacs_byte_pos_to_lisp_char_pos(new_mark).as_i64()))
            .unwrap_or(Value::NIL);
        let _ = super::marker::builtin_set_marker_in_buffers(
            &mut eval.buffers,
            vec![proc.mark, new_mark_pos, proc.buffer],
        )?;
    }

    Ok(Value::NIL)
}

/// (internal-default-process-sentinel PROCESS STRING) -> nil
pub(crate) fn builtin_internal_default_process_sentinel(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("internal-default-process-sentinel", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let msg = expect_string_strict(&args[1])?;

    let (buffer, mark, name, status_symbol) = match eval.processes.get_any(id) {
        Some(proc) => (
            proc.buffer,
            proc.mark,
            process_name_runtime(proc.name),
            ProcessStatusSymbol::from_status_value(proc.status),
        ),
        None => return Err(signal_wrong_type_processp(args[0])),
    };

    if status_symbol == Some(ProcessStatusSymbol::Run) {
        return Ok(Value::NIL);
    }

    let Some(buf_id) = buffer.as_buffer_id() else {
        return Ok(Value::NIL);
    };
    if eval.buffers.get(buf_id).is_none() {
        return Ok(Value::NIL);
    }

    let saved_current = eval.buffers.current_buffer_id();
    let insert_pos = process_mark_insert_emacs_byte_pos(&eval.buffers, buf_id, mark);
    let saved_pt = eval.buffers.get(buf_id).map(|b| b.point_emacs_byte_pos());
    let old_read_only = eval.buffers.get(buf_id).map(|b| b.get_read_only());

    eval.set_current_buffer_unrecorded(buf_id)?;
    if let Some(buf) = eval.buffers.get_mut(buf_id) {
        buf.set_read_only_value(false);
    }
    let _ = eval.buffers.goto_buffer_emacs_byte_pos(buf_id, insert_pos);

    let text = format!("\nProcess {name} {msg}");
    let _ = eval
        .buffers
        .insert_into_buffer_before_markers(buf_id, &text);

    let new_mark = eval
        .buffers
        .get(buf_id)
        .map(|b| b.point_emacs_byte_pos())
        .unwrap_or(insert_pos.add_len(EmacsByteLen::new(text.len())));

    if let (Some(buf), Some(ro)) = (eval.buffers.get_mut(buf_id), old_read_only) {
        buf.set_read_only_value(ro);
    }

    let text_byte_len = EmacsByteLen::new(new_mark.get().saturating_sub(insert_pos.get()));
    if let Some(old_pt) = saved_pt {
        let adjusted_pt = adjusted_process_output_point(old_pt, insert_pos, text_byte_len);
        let _ = eval.buffers.goto_buffer_emacs_byte_pos(buf_id, adjusted_pt);
    }

    if let Some(proc) = eval.processes.get_any_mut(id) {
        let new_mark_pos = eval
            .buffers
            .get(buf_id)
            .map(|b| Value::fixnum(b.emacs_byte_pos_to_lisp_char_pos(new_mark).as_i64()))
            .unwrap_or(Value::NIL);
        super::marker::builtin_set_marker_in_buffers(
            &mut eval.buffers,
            vec![proc.mark, new_mark_pos, proc.buffer],
        )?;
    }

    if let Some(saved_id) = saved_current {
        eval.restore_current_buffer_if_live(saved_id);
    }

    Ok(Value::NIL)
}

/// (gnutls-boot PROCESS TYPE PROPLIST) -> t or error
///
/// Upgrade a network process to TLS through the GNU-compatible `gnutls-boot` API.
/// PROCESS must be a network process with an open TCP socket.
/// TYPE is the credential type.  PROPLIST is a keyword plist; `:hostname`
/// supplies SNI and certificate hostname validation.
pub(crate) fn builtin_gnutls_boot(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-boot", &args, 3)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let parameters = parse_gnutls_boot_parameters(args[1], args[2])?;
    upgrade_process_to_tls::<RustlsBackend>(
        eval,
        id,
        &parameters.hostname,
        "gnutls-boot",
        signal_gnutls_boot_error,
    )?;

    Ok(Value::T)
}

pub(crate) fn builtin_gnutls_asynchronous_parameters(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("gnutls-asynchronous-parameters", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let proc = eval
        .processes
        .get_mut(id)
        .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;
    proc.gnutls_boot_parameters = args[1];
    Ok(Value::NIL)
}

pub(crate) fn builtin_gnutls_bye(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("gnutls-bye", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let proc = eval
        .processes
        .get_mut(id)
        .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;
    let Some(tls_stream) = proc.tls_stream.as_mut() else {
        return Ok(Value::NIL);
    };
    match tls_stream.send_close_notify(args[1].is_nil()) {
        Ok(result) => Ok(gnutls_close_notify_result_value(result)),
        Err(err) => Err(signal_process_io("gnutls-bye", None, err)),
    }
}

pub(crate) fn builtin_gnutls_deinit(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("gnutls-deinit", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let proc = eval
        .processes
        .get_mut(id)
        .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;
    if proc.tls_stream.take().is_some() {
        proc.gnutls_initstage = GnutlsInitStage::Callbacks;
        proc.gnutls_boot_parameters = Value::NIL;
        Ok(Value::T)
    } else {
        Ok(Value::NIL)
    }
}

pub(crate) fn builtin_gnutls_get_initstage(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("gnutls-get-initstage", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let proc = eval
        .processes
        .get(id)
        .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;
    Ok(Value::fixnum(i64::from(proc.gnutls_initstage)))
}

pub(crate) fn builtin_gnutls_peer_status(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("gnutls-peer-status", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &args[0])?;
    let proc = eval
        .processes
        .get(id)
        .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;
    if proc.gnutls_initstage == GnutlsInitStage::Ready {
        Ok(proc
            .tls_stream
            .as_ref()
            .map(|tls| gnutls_peer_status_to_value(&tls.peer_status()))
            .unwrap_or(Value::NIL))
    } else {
        Ok(Value::NIL)
    }
}

/// (neomacs-open-tls-stream NAME BUFFER HOST PORT) -> process
///
/// Open a TCP network process and immediately upgrade it through Neomacs'
/// native TLS backend. This is intentionally separate from GNU's `gnutls-*`
/// API: rustls provides TLS transport, not libgnutls semantics.
pub(crate) fn builtin_neomacs_open_tls_stream(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("neomacs-open-tls-stream", &args, 4)?;
    let host = expect_string_strict(&args[2])?;
    let process = builtin_make_network_process(
        eval,
        vec![
            Value::keyword(":name"),
            args[0],
            Value::keyword(":buffer"),
            args[1],
            Value::keyword(":host"),
            args[2],
            Value::keyword(":service"),
            args[3],
        ],
    )?;
    let id = resolve_process_or_wrong_type_any_in_manager(&eval.processes, &process)?;
    upgrade_process_to_tls::<RustlsBackend>(
        eval,
        id,
        &host,
        "neomacs-open-tls-stream",
        signal_neomacs_tls_error,
    )?;
    Ok(process)
}

fn upgrade_process_to_tls<B: TlsClientBackend>(
    eval: &mut super::eval::Context,
    id: ProcessId,
    host: &str,
    operation: &str,
    map_error: fn(TlsBackendError) -> Flow,
) -> Result<(), Flow> {
    let proc = eval
        .processes
        .get_mut(id)
        .ok_or_else(|| signal("error", vec![Value::string("Process not found")]))?;

    if proc.kind != ProcessKind::Network {
        return Err(signal(
            "error",
            vec![Value::string(format!("{operation}: not a network process"))],
        ));
    }

    // Take the plain TCP stream; it will be owned by the TLS stream.
    let tcp_stream = match proc.network_socket.take() {
        Some(NetworkSocket::TcpStream(stream)) => stream,
        Some(other) => {
            proc.network_socket = Some(other);
            return Err(signal(
                "error",
                vec![Value::string(format!(
                    "{operation}: process is not a TCP stream"
                ))],
            ));
        }
        None => {
            return Err(signal(
                "error",
                vec![Value::string(format!(
                    "{operation}: no socket (already TLS or closed)"
                ))],
            ));
        }
    };

    proc.gnutls_initstage = GnutlsInitStage::HandshakeTried;
    let tls_stream = B::connect_client(tcp_stream, host).map_err(map_error)?;

    // Store the TLS stream. The poller still watches the underlying fd
    // (which is the same fd that was registered for the plain socket).
    proc.tls_stream = Some(tls_stream);
    proc.gnutls_initstage = GnutlsInitStage::Ready;
    proc.gnutls_boot_parameters = Value::NIL;

    Ok(())
}

fn signal_gnutls_boot_error(err: TlsBackendError) -> Flow {
    match err {
        TlsBackendError::InvalidHostname(_) | TlsBackendError::Connect(_) => {
            signal("error", vec![Value::string(err.to_string())])
        }
        TlsBackendError::UnexpectedEof => signal(
            "gnutls-error",
            vec![
                Value::fixnum(-1),
                Value::string("TLS handshake: unexpected EOF"),
            ],
        ),
        TlsBackendError::Io(err) => signal(
            "gnutls-error",
            vec![
                Value::fixnum(-1),
                Value::string(format!("TLS handshake: {}", err)),
            ],
        ),
    }
}

fn signal_neomacs_tls_error(err: TlsBackendError) -> Flow {
    signal("error", vec![Value::string(err.to_string())])
}

/// (isearch-process-search-char CHAR &optional COUNT) -> nil
pub(crate) fn builtin_isearch_process_search_char(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("isearch-process-search-char", &args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("isearch-process-search-char"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    Ok(Value::NIL)
}

/// (isearch-process-search-string STRING MESSAGE) -> nil
pub(crate) fn builtin_isearch_process_search_string(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("isearch-process-search-string", &args, 2)?;
    Ok(Value::NIL)
}

/// (minibuffer--sort-preprocess-history HISTORY) -> nil
pub(crate) fn builtin_minibuffer_sort_preprocess_history(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("minibuffer--sort-preprocess-history", &args, 1)?;
    expect_sequence(&args[0])?;
    Ok(Value::NIL)
}

/// (print--preprocess OBJECT) -> nil
///
/// Extracts sharing info from OBJECT needed to print it: fills the
/// `print-number-table` hash when `print-circle' is non-nil, and does nothing
/// otherwise.  Mirrors GNU `Fprint_preprocess` (src/print.c).
pub(crate) fn builtin_print_preprocess(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("print--preprocess", &args, 1)?;
    let object = args[0];

    // GNU: does nothing if `print-circle' is nil.
    let print_circle = eval
        .obarray
        .symbol_value("print-circle")
        .is_some_and(|v| v.is_truthy());
    if !print_circle {
        return Ok(Value::NIL);
    }

    let print_gensym = eval
        .obarray
        .symbol_value("print-gensym")
        .is_some_and(|v| v.is_truthy());
    let print_continuous_numbering = eval
        .obarray
        .symbol_value("print-continuous-numbering")
        .is_some_and(|v| v.is_truthy());

    // GNU: `if (!HASH_TABLE_P (Vprint_number_table)) Vprint_number_table = make-hash-table :test eq`.
    let table_value = match eval.obarray.symbol_value("print-number-table") {
        Some(v) if v.is_hash_table() => *v,
        _ => {
            let table = Value::hash_table(super::value::HashTableTest::Eq);
            eval.set_variable("print-number-table", table);
            table
        }
    };

    // Root the object and table across the (allocation-heavy) traversal.
    let roots = eval.save_specpdl_roots();
    eval.push_specpdl_root(object);
    eval.push_specpdl_root(table_value);
    super::print::preprocess_print_number_table(
        &object,
        table_value,
        print_gensym,
        print_continuous_numbering,
    );
    eval.restore_specpdl_roots(roots);

    Ok(Value::NIL)
}

/// (syntax-propertize--in-process-p) -> nil
pub(crate) fn builtin_syntax_propertize_in_process_p(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("syntax-propertize--in-process-p", &args, 0)?;
    Ok(Value::NIL)
}

/// (window--adjust-process-windows) -> nil
pub(crate) fn builtin_window_adjust_process_windows(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("window--adjust-process-windows", &args, 0)?;
    Ok(Value::NIL)
}

/// (window--process-window-list) -> nil
pub(crate) fn builtin_window_process_window_list(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("window--process-window-list", &args, 0)?;
    Ok(Value::NIL)
}

/// (window-adjust-process-window-size PROCESS WINDOW) -> nil
pub(crate) fn builtin_window_adjust_process_window_size(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("window-adjust-process-window-size", &args, 2)?;
    expect_list(&args[1])?;
    Ok(Value::NIL)
}

/// (window-adjust-process-window-size-largest PROCESS WINDOW) -> nil
pub(crate) fn builtin_window_adjust_process_window_size_largest(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("window-adjust-process-window-size-largest", &args, 2)?;
    expect_list(&args[1])?;
    Ok(Value::NIL)
}

/// (window-adjust-process-window-size-smallest PROCESS WINDOW) -> nil
pub(crate) fn builtin_window_adjust_process_window_size_smallest(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("window-adjust-process-window-size-smallest", &args, 2)?;
    expect_list(&args[1])?;
    Ok(Value::NIL)
}

/// (format-network-address ADDRESS &optional OMIT-PORT) -> string-or-nil
pub(crate) fn builtin_format_network_address(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_format_network_address_impl(args)
}

pub(crate) fn builtin_format_network_address_impl(args: Vec<Value>) -> EvalResult {
    expect_min_args("format-network-address", &args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("format-network-address"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let omit_port = args.get(1).is_some_and(|v| v.is_truthy());
    match args[0].kind() {
        ValueKind::String => Ok(args[0]),
        ValueKind::Nil => Ok(Value::NIL),
        ValueKind::Veclike(VecLikeType::Vector) => {
            let Some(items) = vector_nonnegative_integers(&args[0]) else {
                return Ok(Value::NIL);
            };
            if let Some(ipv4) = format_ipv4_network_address(&items, omit_port) {
                return Ok(Value::string(ipv4));
            }
            if let Some(ipv6) = format_ipv6_network_address(&items, omit_port) {
                return Ok(Value::string(ipv6));
            }
            Ok(Value::NIL)
        }
        ValueKind::Cons => {
            let first = list_to_vec(&args[0])
                .and_then(|items| items.first().cloned())
                .and_then(|v| value_as_nonnegative_integer(&v));
            if let Some(family) = first {
                Ok(Value::string(format!("<Family {family}>")))
            } else {
                Ok(Value::NIL)
            }
        }
        _ => Ok(Value::NIL),
    }
}

/// (network-interface-list &optional FULL FAMILY) -> interface-list
pub(crate) fn builtin_network_interface_list(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_network_interface_list_impl(args)
}

pub(crate) fn builtin_network_interface_list_impl(args: Vec<Value>) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("network-interface-list"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let full = args.first().is_some_and(|v| v.is_truthy());
    let family = args.get(1).cloned().unwrap_or(Value::NIL);
    let requested_family = if family.is_nil() {
        None
    } else {
        Some(
            NetworkAddressFamily::from_symbol_value(&family).ok_or_else(|| {
                signal("error", vec![Value::string("Unsupported address family")])
            })?,
        )
    };
    let include_ipv4 = requested_family.is_none_or(|family| family == NetworkAddressFamily::Ipv4);
    let include_ipv6 = requested_family.is_none_or(|family| family == NetworkAddressFamily::Ipv6);

    let mut entries = Vec::new();
    if let Some(host_entries) = host_interface_snapshot() {
        for entry in host_entries.into_iter().rev() {
            let include = match entry.family {
                NetworkAddressFamily::Ipv4 => include_ipv4,
                NetworkAddressFamily::Ipv6 => include_ipv6,
            };
            if !include {
                continue;
            }

            if full {
                entries.push(Value::list(vec![
                    Value::string(entry.name),
                    entry.address,
                    entry.list_broadcast,
                    entry.netmask,
                ]));
            } else {
                entries.push(Value::cons(Value::string(entry.name), entry.address));
            }
        }
    }

    if entries.is_empty() {
        if include_ipv6 {
            entries.push(interface_entry("lo", loopback_ipv6_address(), full));
        }
        if include_ipv4 {
            entries.push(interface_entry("lo", loopback_ipv4_address(), full));
        }
    }
    Ok(Value::list(entries))
}

/// (network-interface-info IFNAME) -> interface-info-or-nil
pub(crate) fn builtin_network_interface_info(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_network_interface_info_impl(args)
}

pub(crate) fn builtin_network_interface_info_impl(args: Vec<Value>) -> EvalResult {
    expect_args("network-interface-info", &args, 1)?;
    let ifname_raw = expect_string_strict(&args[0])?;
    // Match C-string interface-name handling: embedded NUL truncates lookup.
    let ifname = ifname_raw.split('\0').next().unwrap_or_default();
    // Emacs applies IFNAMSIZ-style byte limits, not character counts.
    if ifname.len() >= 16 {
        return Err(signal(
            "error",
            vec![Value::string("interface name too long")],
        ));
    }

    if let Some(host_entries) = host_interface_snapshot() {
        let mut first_match: Option<HostInterfaceEntry> = None;
        let mut ipv4_match: Option<HostInterfaceEntry> = None;

        for entry in host_entries {
            if entry.name != ifname {
                continue;
            }
            if first_match.is_none() {
                first_match = Some(entry.clone());
            }
            if entry.family == NetworkAddressFamily::Ipv4 {
                ipv4_match = Some(entry);
                break;
            }
        }

        if let Some(entry) = ipv4_match.or(first_match) {
            return Ok(Value::list(vec![
                entry.address,
                entry.info_broadcast,
                entry.netmask,
                entry.hwaddr.unwrap_or(Value::NIL),
                entry.flags,
            ]));
        }
    }

    if ifname == "lo" {
        return Ok(Value::list(vec![
            loopback_ipv4_address(),
            loopback_ipv4_broadcast(),
            loopback_ipv4_netmask(),
            loopback_hwaddr(),
            loopback_flags(),
        ]));
    }

    Ok(Value::NIL)
}

/// (network-lookup-address-info NAME &optional FAMILY HINTS) -> address-list
pub(crate) fn builtin_network_lookup_address_info(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_network_lookup_address_info_impl(args)
}

pub(crate) fn builtin_network_lookup_address_info_impl(args: Vec<Value>) -> EvalResult {
    expect_min_args("network-lookup-address-info", &args, 1)?;
    if args.len() > 3 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("network-lookup-address-info"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let name = expect_string_strict(&args[0])?;

    let family = args.get(1).cloned().unwrap_or(Value::NIL);
    let hint_value = args.get(2).cloned().unwrap_or(Value::NIL);

    let lookup_family = if family.is_nil() {
        None
    } else {
        Some(
            NetworkAddressFamily::from_symbol_value(&family)
                .ok_or_else(|| signal("error", vec![Value::string("Unsupported family")]))?,
        )
    };
    let lookup_hint = if hint_value.is_nil() {
        None
    } else {
        Some(
            NetworkLookupHint::from_symbol_value(&hint_value)
                .ok_or_else(|| signal("error", vec![Value::string("Unsupported hints value")]))?,
        )
    };
    let entries = resolve_network_lookup_addresses(&name, lookup_family, lookup_hint);
    Ok(Value::list(entries))
}

/// (signal-names) -> list-of-signal-name-strings
pub(crate) fn builtin_signal_names(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_signal_names_impl(args)
}

pub(crate) fn builtin_signal_names_impl(args: Vec<Value>) -> EvalResult {
    expect_args("signal-names", &args, 0)?;
    let names = vec![
        "RTMAX", "RTMAX-1", "RTMAX-2", "RTMAX-3", "RTMAX-4", "RTMAX-5", "RTMAX-6", "RTMAX-7",
        "RTMAX-8", "RTMAX-9", "RTMAX-10", "RTMAX-11", "RTMAX-12", "RTMAX-13", "RTMAX-14",
        "RTMIN+15", "RTMIN+14", "RTMIN+13", "RTMIN+12", "RTMIN+11", "RTMIN+10", "RTMIN+9",
        "RTMIN+8", "RTMIN+7", "RTMIN+6", "RTMIN+5", "RTMIN+4", "RTMIN+3", "RTMIN+2", "RTMIN+1",
        "RTMIN", "SYS", "PWR", "POLL", "WINCH", "PROF", "VTALRM", "XFSZ", "XCPU", "URG", "TTOU",
        "TTIN", "TSTP", "STOP", "CONT", "CHLD", "STKFLT", "TERM", "ALRM", "PIPE", "USR2", "SEGV",
        "USR1", "KILL", "FPE", "BUS", "ABRT", "TRAP", "ILL", "QUIT", "INT", "HUP", "EXIT",
    ];
    Ok(Value::list(
        names.into_iter().map(Value::string).collect::<Vec<_>>(),
    ))
}

/// (list-system-processes) -> process-id-list
pub(crate) fn builtin_list_system_processes(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_list_system_processes_impl(args)
}

pub(crate) fn builtin_list_system_processes_impl(args: Vec<Value>) -> EvalResult {
    expect_args("list-system-processes", &args, 0)?;

    let mut pids: Vec<i64> = std::fs::read_dir("/proc")
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<i64>().ok())
        .collect();
    pids.sort_unstable();
    Ok(Value::list(pids.into_iter().map(Value::fixnum).collect()))
}

/// (num-processors &optional QUERY) -> integer
pub(crate) fn builtin_num_processors(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_num_processors_impl(args)
}

pub(crate) fn builtin_num_processors_impl(args: Vec<Value>) -> EvalResult {
    if args.len() > 1 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("num-processors"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let query = args.first().and_then(NumProcessorsQuery::from_symbol_value);
    Ok(Value::fixnum(num_processors_count(query) as i64))
}

fn num_processors_count(query: Option<NumProcessorsQuery>) -> u64 {
    match query {
        Some(NumProcessorsQuery::All) => all_processors_count(),
        Some(NumProcessorsQuery::Current) => current_processors_count(),
        None => current_processors_count_overridable(),
    }
}

#[cfg(unix)]
fn current_processors_count_overridable() -> u64 {
    let omp_threads = std::env::var_os("OMP_NUM_THREADS");
    let omp_limit = std::env::var_os("OMP_THREAD_LIMIT");
    current_processors_count_overridable_with_env(
        omp_threads.as_deref().map(OsStrExt::as_bytes),
        omp_limit.as_deref().map(OsStrExt::as_bytes),
        current_processors_count(),
    )
}

#[cfg(not(unix))]
fn current_processors_count_overridable() -> u64 {
    let omp_threads = std::env::var("OMP_NUM_THREADS").ok();
    let omp_limit = std::env::var("OMP_THREAD_LIMIT").ok();
    current_processors_count_overridable_with_env(
        omp_threads.as_deref().map(str::as_bytes),
        omp_limit.as_deref().map(str::as_bytes),
        current_processors_count(),
    )
}

fn current_processors_count_overridable_with_env(
    omp_threads: Option<&[u8]>,
    omp_limit: Option<&[u8]>,
    current_count: u64,
) -> u64 {
    let omp_threads = omp_threads.and_then(parse_openmp_threads).unwrap_or(0);
    let mut omp_limit = omp_limit.and_then(parse_openmp_threads).unwrap_or(u64::MAX);
    if omp_limit == 0 {
        omp_limit = u64::MAX;
    }

    if omp_threads != 0 {
        return omp_threads.min(omp_limit);
    }

    current_count.min(omp_limit).max(1)
}

fn parse_openmp_threads(bytes: &[u8]) -> Option<u64> {
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx == bytes.len() || !bytes[idx].is_ascii_digit() {
        return None;
    }

    let start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    let value = std::str::from_utf8(&bytes[start..idx])
        .ok()?
        .parse::<u64>()
        .ok()?;

    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    if idx == bytes.len() || bytes[idx] == b',' {
        Some(value)
    } else {
        None
    }
}

fn current_processors_count() -> u64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(1)
        .max(1)
}

fn all_processors_count() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
    let count = system.cpus().len() as u64;
    if count == 0 {
        current_processors_count()
    } else {
        count
    }
}

/// (list-processes &optional QUERY-ONLY BUFFER) -> nil
pub(crate) fn builtin_list_processes(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("list-processes"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    Ok(Value::NIL)
}

/// (list-processes--refresh) -> row-spec
pub(crate) fn builtin_list_processes_refresh(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("list-processes--refresh", &args, 0)?;
    let spacer = Value::string_with_text_properties(
        " ",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword(":align-to"),
                    Value::list(vec![
                        Value::symbol("+"),
                        Value::symbol("header-line-indent-width"),
                        Value::fixnum(0),
                    ]),
                ]),
            ]),
        }],
    );
    Ok(Value::list(vec![
        Value::string(""),
        Value::symbol("header-line-indent"),
        spacer,
    ]))
}

/// (make-network-process &rest ARGS) -> process-or-nil
pub(crate) fn builtin_make_network_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.is_empty() {
        return Ok(Value::NIL);
    }

    // ---- Parse all keyword arguments ----
    let mut name: Option<LispString> = None;
    let mut host_value = Value::NIL;
    let mut service: Option<Value> = None;
    let mut server = false;
    let mut server_value = Value::NIL;
    let mut family_value = Value::NIL;
    let mut local_address_value = Value::NIL;
    let mut remote_address_value = Value::NIL;
    let mut nowait = false;
    let mut socket_type = NetworkSocketType::Stream;
    let mut contact = Value::list(args.clone());
    let mut filter_val = Value::NIL;
    let mut sentinel_val = Value::NIL;
    let mut log_val = Value::NIL;
    let mut buffer_val = Value::NIL;
    let mut coding_val = Value::NIL;
    let mut tls_parameters_val = Value::NIL;
    let mut noquery = false;
    let mut plist_val = Value::NIL;
    let mut stop_val = Value::NIL;
    let socket_options = collect_network_socket_options(&args);

    let mut i = 0usize;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).cloned().unwrap_or(Value::NIL);
        let Some(keyword) = ProcessKeyword::from_value(key) else {
            i += 1;
            continue;
        };
        match keyword {
            ProcessKeyword::Name => name = Some(expect_process_name_lisp_string(&value)?),
            ProcessKeyword::Host => host_value = value,
            ProcessKeyword::Service => service = Some(value),
            ProcessKeyword::Server => {
                server = value.is_truthy();
                server_value = value;
            }
            ProcessKeyword::Family => family_value = value,
            ProcessKeyword::Type => socket_type = parse_network_socket_type(&value)?,
            ProcessKeyword::Nowait => nowait = value.is_truthy(),
            ProcessKeyword::Filter => filter_val = value,
            ProcessKeyword::Sentinel => sentinel_val = value,
            ProcessKeyword::Log => log_val = value,
            ProcessKeyword::Buffer => buffer_val = value,
            ProcessKeyword::Coding => coding_val = value,
            ProcessKeyword::TlsParameters => tls_parameters_val = value,
            ProcessKeyword::Noquery => noquery = value.is_truthy(),
            ProcessKeyword::Stop => stop_val = value,
            ProcessKeyword::Local => local_address_value = value,
            ProcessKeyword::Remote => remote_address_value = value,
            ProcessKeyword::Plist => plist_val = value,
            _ => {}
        }
        i += 2;
    }

    let Some(name) = name else {
        return Err(signal(
            "error",
            vec![Value::string("Missing :name keyword parameter")],
        ));
    };

    if server && nowait {
        return Err(signal(
            "error",
            vec![Value::string("`:server' is incompatible with `:nowait'")],
        ));
    }
    validate_process_coding_value(Some(&eval.coding_systems), coding_val)?;
    let plist_val = copy_process_plist(plist_val)?;
    let stop = stop_val.is_truthy();
    let server_backlog = if server {
        Some(network_server_backlog(server_value)?)
    } else {
        None
    };
    let tls_parameters = parse_make_network_tls_parameters(tls_parameters_val)?;

    // Resolve :buffer to a buffer name (creating buffer if needed).
    let buffer = if !buffer_val.is_nil() {
        parse_make_process_buffer(eval, &buffer_val)?
    } else {
        Value::NIL
    };

    // Default coding resolution for `:coding nil`, mirroring GNU
    // `set_network_socket_coding_system` (src/process.c:3291-3367): the decode
    // side depends on the process buffer's multibyteness (raw for a unibyte
    // buffer, `default-process-coding-system` otherwise) and a missing buffer
    // uses the default buffer's multibyteness (multibyte by default).
    let default_process_coding =
        eval.visible_variable_value_or_nil("default-process-coding-system");
    let network_buffer_multibyte =
        match resolve_buffer_for_process_lookup_in_state(&eval.frames, &eval.buffers, &buffer) {
            Ok(Some(bid)) => eval
                .buffers
                .get(bid)
                .map(|b| b.get_multibyte())
                .unwrap_or(true),
            // No buffer (or unresolved) -> default buffer is multibyte in GNU.
            _ => true,
        };

    let explicit_address = if server {
        local_address_value
    } else {
        remote_address_value
    };
    if !explicit_address.is_nil() {
        let address = parse_network_address_spec(&explicit_address)?;
        match address {
            NetworkAddressSpec::Inet(addr) => {
                if socket_type == NetworkSocketType::Datagram {
                    if server {
                        let effective_options = tcp_server_socket_options(&socket_options);
                        let socket = bind_udp_socket(addr, &effective_options)?;
                        let local_addr = socket.local_addr().map_err(|e| {
                            signal(
                                "file-error",
                                vec![Value::string(format!("getsockname: {}", e))],
                            )
                        })?;
                        let zero_datagram = datagram_zero_address_for(local_addr);
                        let (datagram_socket_addr, datagram_address) =
                            if !remote_address_value.is_nil() {
                                match parse_network_address_spec(&remote_address_value)? {
                                    NetworkAddressSpec::Inet(remote) => {
                                        (Some(remote), socket_addr_to_lisp_value(remote))
                                    }
                                    #[cfg(unix)]
                                    NetworkAddressSpec::Local(_) => (None, zero_datagram),
                                }
                            } else {
                                (None, zero_datagram)
                            };
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Service.value(),
                            Value::fixnum(local_addr.port() as i64),
                        )?;
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Local.value(),
                            socket_addr_to_lisp_value(local_addr),
                        )?;
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Remote.value(),
                            datagram_address,
                        )?;

                        let id = eval.processes.create_process_with_kind_lisp(
                            name,
                            buffer,
                            LispString::from_utf8("network"),
                            Vec::new(),
                            ProcessKind::Network,
                        );
                        eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                        if let Some(proc) = eval.processes.get_mut(id) {
                            proc.childp = contact;
                            set_network_process_coding(
                                proc,
                                coding_val,
                                default_process_coding,
                                network_buffer_multibyte,
                            );
                            proc.thread = current_thread_handle(&eval.threads);
                            proc.plist = plist_val;
                            proc.status = ProcessStatusSymbol::Open.value();
                            proc.network_socket = Some(NetworkSocket::UdpSocket(socket));
                            proc.datagram_socket_addr = datagram_socket_addr;
                            proc.datagram_address = datagram_address;
                            if !filter_val.is_nil() {
                                proc.filter = filter_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Filter.value(),
                                    proc.filter,
                                )?;
                            }
                            if !sentinel_val.is_nil() {
                                proc.sentinel = sentinel_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Sentinel.value(),
                                    proc.sentinel,
                                )?;
                            }
                            if !log_val.is_nil() {
                                proc.log = log_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Log.value(),
                                    proc.log,
                                )?;
                            }
                            if !buffer.is_nil() {
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Buffer.value(),
                                    buffer,
                                )?;
                            }
                            apply_connection_process_flags(proc, noquery, stop);
                        }
                        eval.processes.register_socket_fd(id).ok();
                        return Ok(Value::make_process(id));
                    }

                    let socket = bind_udp_client_socket(addr, &socket_options)?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        socket_addr_to_lisp_value(addr),
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        socket_addr_to_lisp_value(udp_unspecified_addr_for(addr)),
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.network_socket = Some(NetworkSocket::UdpSocket(socket));
                        proc.datagram_socket_addr = Some(addr);
                        proc.datagram_address = socket_addr_to_lisp_value(addr);
                        proc.status = ProcessStatusSymbol::Open.value();
                        proc.childp = contact;
                        proc.plist = plist_val;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    eval.processes.register_socket_fd(id).ok();
                    return Ok(Value::make_process(id));
                }

                #[cfg(unix)]
                if socket_type == NetworkSocketType::Seqpacket {
                    return Err(signal(
                        "error",
                        vec![Value::string("Unsupported connection type")],
                    ));
                }

                if server {
                    let effective_options = tcp_server_socket_options(&socket_options);
                    let listener = bind_tcp_listener_socket(
                        addr,
                        server_backlog.unwrap_or(5),
                        &effective_options,
                    )?;
                    let local_addr = listener.local_addr().map_err(|e| {
                        signal(
                            "file-error",
                            vec![Value::string(format!("getsockname: {}", e))],
                        )
                    })?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Service.value(),
                        Value::fixnum(local_addr.port() as i64),
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        socket_addr_to_lisp_value(local_addr),
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.childp = contact;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        proc.plist = plist_val;
                        proc.network_socket = Some(NetworkSocket::TcpListener(listener));
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !log_val.is_nil() {
                            proc.log = log_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Log.value(),
                                proc.log,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    eval.processes.register_socket_fd(id).ok();
                    return Ok(Value::make_process(id));
                }

                if nowait {
                    let start = start_pending_tcp_stream_connect(vec![addr], &socket_options)?;
                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.status = process_status_connect_value();
                        proc.childp = contact;
                        proc.plist = plist_val;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        match start {
                            PendingNetworkConnectStart::Started(started) => {
                                let local_addr = started.stream.local_addr().ok();
                                proc.network_socket =
                                    Some(NetworkSocket::TcpStream(started.stream));
                                proc.pending_network_connect = Some(PendingNetworkConnect::Tcp {
                                    remaining_addrs: started.remaining_addrs,
                                    socket_options: socket_options.clone(),
                                });
                                ProcessManager::update_tcp_client_contact(
                                    proc,
                                    started.remote_addr,
                                    local_addr,
                                )?;
                            }
                            PendingNetworkConnectStart::Failed(code) => {
                                proc.status = process_status_failed_value(code);
                            }
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    if eval
                        .processes
                        .get(id)
                        .is_some_and(|proc| proc.pending_network_connect.is_some())
                    {
                        eval.processes.register_socket_writable_fd(id).ok();
                    }
                    return Ok(Value::make_process(id));
                }

                let stream = connect_tcp_stream_socket(addr, &socket_options)?;
                let remote_addr = stream.peer_addr().ok();
                let local_addr = stream.local_addr().ok();

                let id = eval.processes.create_process_with_kind_lisp(
                    name,
                    buffer,
                    LispString::from_utf8("network"),
                    Vec::new(),
                    ProcessKind::Network,
                );
                eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                if let Some(addr) = remote_addr {
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        socket_addr_to_lisp_value(addr),
                    )?;
                }
                if let Some(addr) = local_addr {
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        socket_addr_to_lisp_value(addr),
                    )?;
                }
                if let Some(proc) = eval.processes.get_mut(id) {
                    proc.network_socket = Some(NetworkSocket::TcpStream(stream));
                    proc.status = process_status_run_value();
                    proc.childp = contact;
                    proc.plist = plist_val;
                    set_network_process_coding(
                        proc,
                        coding_val,
                        default_process_coding,
                        network_buffer_multibyte,
                    );
                    proc.thread = current_thread_handle(&eval.threads);
                    if !filter_val.is_nil() {
                        proc.filter = filter_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Filter.value(),
                            proc.filter,
                        )?;
                    }
                    if !sentinel_val.is_nil() {
                        proc.sentinel = sentinel_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Sentinel.value(),
                            proc.sentinel,
                        )?;
                    }
                    if !buffer.is_nil() {
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Buffer.value(),
                            buffer,
                        )?;
                    }
                    apply_connection_process_flags(proc, noquery, stop);
                }
                if let Some(parameters) = tls_parameters.clone() {
                    upgrade_process_to_tls::<RustlsBackend>(
                        eval,
                        id,
                        &parameters.hostname,
                        "make-network-process",
                        signal_gnutls_boot_error,
                    )?;
                }
                eval.processes.register_socket_fd(id).ok();
                let sentinel = eval
                    .processes
                    .get(id)
                    .map(|p| p.sentinel)
                    .unwrap_or(Value::NIL);
                if !stop {
                    eval.run_process_sentinel_callback(id, sentinel, "open\n")?;
                }
                return Ok(Value::make_process(id));
            }
            #[cfg(unix)]
            NetworkAddressSpec::Local(path) => {
                if socket_type == NetworkSocketType::Datagram {
                    let path_value = Value::heap_string(
                        crate::emacs_core::fileio::path_to_lisp_file_name(&path),
                    );
                    if server {
                        let socket = bind_unix_datagram_socket(&path, &socket_options)?;
                        let zero_datagram = datagram_zero_unix_address();
                        let (datagram_unix_path, datagram_address) =
                            if !remote_address_value.is_nil() {
                                match parse_network_address_spec(&remote_address_value)? {
                                    NetworkAddressSpec::Local(remote_path) => {
                                        let remote_value = Value::heap_string(
                                            crate::emacs_core::fileio::path_to_lisp_file_name(
                                                &remote_path,
                                            ),
                                        );
                                        (Some(remote_path), remote_value)
                                    }
                                    NetworkAddressSpec::Inet(_) => (None, zero_datagram),
                                }
                            } else {
                                (None, zero_datagram)
                            };
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Local.value(),
                            path_value,
                        )?;
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Remote.value(),
                            datagram_address,
                        )?;

                        let id = eval.processes.create_process_with_kind_lisp(
                            name,
                            buffer,
                            LispString::from_utf8("network"),
                            Vec::new(),
                            ProcessKind::Network,
                        );
                        eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                        if let Some(proc) = eval.processes.get_mut(id) {
                            proc.childp = contact;
                            set_network_process_coding(
                                proc,
                                coding_val,
                                default_process_coding,
                                network_buffer_multibyte,
                            );
                            proc.thread = current_thread_handle(&eval.threads);
                            proc.plist = plist_val;
                            proc.status = ProcessStatusSymbol::Open.value();
                            proc.network_socket = Some(NetworkSocket::UnixDatagram(socket));
                            proc.datagram_address = datagram_address;
                            proc.datagram_unix_path = datagram_unix_path;
                            if !filter_val.is_nil() {
                                proc.filter = filter_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Filter.value(),
                                    proc.filter,
                                )?;
                            }
                            if !sentinel_val.is_nil() {
                                proc.sentinel = sentinel_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Sentinel.value(),
                                    proc.sentinel,
                                )?;
                            }
                            if !log_val.is_nil() {
                                proc.log = log_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Log.value(),
                                    proc.log,
                                )?;
                            }
                            if !buffer.is_nil() {
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Buffer.value(),
                                    buffer,
                                )?;
                            }
                            apply_connection_process_flags(proc, noquery, stop);
                        }
                        eval.processes.register_socket_fd(id).ok();
                        return Ok(Value::make_process(id));
                    }

                    let socket = unbound_unix_datagram_socket(&socket_options)?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        path_value,
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        Value::string(""),
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.network_socket = Some(NetworkSocket::UnixDatagram(socket));
                        proc.status = ProcessStatusSymbol::Open.value();
                        proc.childp = contact;
                        proc.plist = plist_val;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        proc.datagram_address = path_value;
                        proc.datagram_unix_path = Some(path);
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    eval.processes.register_socket_fd(id).ok();
                    return Ok(Value::make_process(id));
                }

                if socket_type == NetworkSocketType::Seqpacket {
                    if server {
                        let listener = bind_unix_seqpacket_listener_socket(
                            &path,
                            server_backlog.unwrap_or(5),
                            &socket_options,
                        )?;
                        contact = process_contact_plist_put(
                            contact,
                            ProcessKeyword::Local.value(),
                            Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                                &path,
                            )),
                        )?;

                        let id = eval.processes.create_process_with_kind_lisp(
                            name,
                            buffer,
                            LispString::from_utf8("network"),
                            Vec::new(),
                            ProcessKind::Network,
                        );
                        eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                        if let Some(proc) = eval.processes.get_mut(id) {
                            proc.childp = contact;
                            set_network_process_coding(
                                proc,
                                coding_val,
                                default_process_coding,
                                network_buffer_multibyte,
                            );
                            proc.thread = current_thread_handle(&eval.threads);
                            proc.plist = plist_val;
                            proc.network_socket = Some(NetworkSocket::SeqpacketListener(listener));
                            if !filter_val.is_nil() {
                                proc.filter = filter_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Filter.value(),
                                    proc.filter,
                                )?;
                            }
                            if !sentinel_val.is_nil() {
                                proc.sentinel = sentinel_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Sentinel.value(),
                                    proc.sentinel,
                                )?;
                            }
                            if !log_val.is_nil() {
                                proc.log = log_val;
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Log.value(),
                                    proc.log,
                                )?;
                            }
                            if !buffer.is_nil() {
                                proc.childp = process_contact_plist_put(
                                    proc.childp,
                                    ProcessKeyword::Buffer.value(),
                                    buffer,
                                )?;
                            }
                            apply_connection_process_flags(proc, noquery, stop);
                        }
                        eval.processes.register_socket_fd(id).ok();
                        return Ok(Value::make_process(id));
                    }

                    let socket = connect_unix_seqpacket_socket(&path, &socket_options)?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                            &path,
                        )),
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        Value::string(""),
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.network_socket = Some(NetworkSocket::SeqpacketStream(socket));
                        proc.status = process_status_run_value();
                        proc.childp = contact;
                        proc.plist = plist_val;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }

                    eval.processes.register_socket_fd(id).ok();

                    let sentinel = eval
                        .processes
                        .get(id)
                        .map(|p| p.sentinel)
                        .unwrap_or(Value::NIL);
                    if !stop {
                        eval.run_process_sentinel_callback(id, sentinel, "open\n")?;
                    }

                    return Ok(Value::make_process(id));
                }

                if server {
                    let listener = bind_unix_listener_socket(
                        &path,
                        server_backlog.unwrap_or(5),
                        &socket_options,
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                            &path,
                        )),
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.childp = contact;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        proc.plist = plist_val;
                        proc.network_socket = Some(NetworkSocket::UnixListener(listener));
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !log_val.is_nil() {
                            proc.log = log_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Log.value(),
                                proc.log,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    eval.processes.register_socket_fd(id).ok();
                    return Ok(Value::make_process(id));
                }

                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Remote.value(),
                    Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(&path)),
                )?;
                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Local.value(),
                    Value::string(""),
                )?;

                if nowait {
                    let start = start_nonblocking_unix_stream_socket(&path, &socket_options)?;
                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.status = process_status_connect_value();
                        proc.childp = contact;
                        proc.plist = plist_val;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        match start {
                            Ok(stream) => {
                                proc.network_socket = Some(NetworkSocket::UnixStream(stream));
                                proc.pending_network_connect = Some(PendingNetworkConnect::Local);
                            }
                            Err(err) => {
                                proc.status =
                                    process_status_failed_value(io_error_status_code(&err));
                            }
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    if eval
                        .processes
                        .get(id)
                        .is_some_and(|proc| proc.pending_network_connect.is_some())
                    {
                        eval.processes.register_socket_writable_fd(id).ok();
                    }
                    return Ok(Value::make_process(id));
                }

                let stream = connect_unix_stream_socket(&path, &socket_options)?;
                let id = eval.processes.create_process_with_kind_lisp(
                    name,
                    buffer,
                    LispString::from_utf8("network"),
                    Vec::new(),
                    ProcessKind::Network,
                );
                eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                if let Some(proc) = eval.processes.get_mut(id) {
                    proc.network_socket = Some(NetworkSocket::UnixStream(stream));
                    proc.status = process_status_run_value();
                    proc.childp = contact;
                    proc.plist = plist_val;
                    set_network_process_coding(
                        proc,
                        coding_val,
                        default_process_coding,
                        network_buffer_multibyte,
                    );
                    proc.thread = current_thread_handle(&eval.threads);
                    if !filter_val.is_nil() {
                        proc.filter = filter_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Filter.value(),
                            proc.filter,
                        )?;
                    }
                    if !sentinel_val.is_nil() {
                        proc.sentinel = sentinel_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Sentinel.value(),
                            proc.sentinel,
                        )?;
                    }
                    if !buffer.is_nil() {
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Buffer.value(),
                            buffer,
                        )?;
                    }
                    apply_connection_process_flags(proc, noquery, stop);
                }

                eval.processes.register_socket_fd(id).ok();

                let sentinel = eval
                    .processes
                    .get(id)
                    .map(|p| p.sentinel)
                    .unwrap_or(Value::NIL);
                if !stop {
                    eval.run_process_sentinel_callback(id, sentinel, "open\n")?;
                }

                return Ok(Value::make_process(id));
            }
        }
    }

    if !family_value.is_nil() {
        validate_network_process_family(&family_value)?;
    }
    let family = NetworkProcessFamilySymbol::from_symbol_value(&family_value);
    let host = parse_network_host(&host_value, family)?;

    let service = service.unwrap_or(Value::NIL);
    if service.is_nil() {
        return Err(signal_wrong_type_string(Value::NIL));
    }

    if family == Some(NetworkProcessFamilySymbol::Local) {
        #[cfg(not(unix))]
        {
            return Err(signal(
                "error",
                vec![Value::string("Unknown address family")],
            ));
        }

        #[cfg(unix)]
        {
            let service_path = crate::emacs_core::fileio::lisp_file_name_to_path_buf(
                super::builtins::expect_lisp_string(&service)?,
            );
            if !host_value.is_nil() {
                contact =
                    process_contact_plist_put(contact, ProcessKeyword::Host.value(), Value::NIL)?;
            }

            if socket_type == NetworkSocketType::Datagram {
                let service_path_value = Value::heap_string(
                    crate::emacs_core::fileio::path_to_lisp_file_name(&service_path),
                );
                if server {
                    let socket = bind_unix_datagram_socket(&service_path, &socket_options)?;
                    let zero_datagram = datagram_zero_unix_address();
                    let (datagram_unix_path, datagram_address) = if !remote_address_value.is_nil() {
                        match parse_network_address_spec(&remote_address_value)? {
                            NetworkAddressSpec::Local(remote_path) => {
                                let remote_value = Value::heap_string(
                                    crate::emacs_core::fileio::path_to_lisp_file_name(&remote_path),
                                );
                                (Some(remote_path), remote_value)
                            }
                            NetworkAddressSpec::Inet(_) => (None, zero_datagram),
                        }
                    } else {
                        (None, zero_datagram)
                    };
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        service_path_value,
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Remote.value(),
                        datagram_address,
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.childp = contact;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        proc.plist = plist_val;
                        proc.status = ProcessStatusSymbol::Open.value();
                        proc.network_socket = Some(NetworkSocket::UnixDatagram(socket));
                        proc.datagram_address = datagram_address;
                        proc.datagram_unix_path = datagram_unix_path;
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !log_val.is_nil() {
                            proc.log = log_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Log.value(),
                                proc.log,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    eval.processes.register_socket_fd(id).ok();
                    return Ok(Value::make_process(id));
                }

                let socket = unbound_unix_datagram_socket(&socket_options)?;
                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Remote.value(),
                    service_path_value,
                )?;
                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Local.value(),
                    Value::string(""),
                )?;

                let id = eval.processes.create_process_with_kind_lisp(
                    name,
                    buffer,
                    LispString::from_utf8("network"),
                    Vec::new(),
                    ProcessKind::Network,
                );
                eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                if let Some(proc) = eval.processes.get_mut(id) {
                    proc.network_socket = Some(NetworkSocket::UnixDatagram(socket));
                    proc.status = ProcessStatusSymbol::Open.value();
                    proc.childp = contact;
                    proc.plist = plist_val;
                    set_network_process_coding(
                        proc,
                        coding_val,
                        default_process_coding,
                        network_buffer_multibyte,
                    );
                    proc.thread = current_thread_handle(&eval.threads);
                    proc.datagram_address = service_path_value;
                    proc.datagram_unix_path = Some(service_path);
                    if !filter_val.is_nil() {
                        proc.filter = filter_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Filter.value(),
                            proc.filter,
                        )?;
                    }
                    if !sentinel_val.is_nil() {
                        proc.sentinel = sentinel_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Sentinel.value(),
                            proc.sentinel,
                        )?;
                    }
                    if !buffer.is_nil() {
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Buffer.value(),
                            buffer,
                        )?;
                    }
                    apply_connection_process_flags(proc, noquery, stop);
                }

                eval.processes.register_socket_fd(id).ok();
                return Ok(Value::make_process(id));
            }

            if socket_type == NetworkSocketType::Seqpacket {
                if server {
                    let listener = bind_unix_seqpacket_listener_socket(
                        &service_path,
                        server_backlog.unwrap_or(5),
                        &socket_options,
                    )?;
                    contact = process_contact_plist_put(
                        contact,
                        ProcessKeyword::Local.value(),
                        Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                            &service_path,
                        )),
                    )?;

                    let id = eval.processes.create_process_with_kind_lisp(
                        name,
                        buffer,
                        LispString::from_utf8("network"),
                        Vec::new(),
                        ProcessKind::Network,
                    );
                    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                    if let Some(proc) = eval.processes.get_mut(id) {
                        proc.childp = contact;
                        set_network_process_coding(
                            proc,
                            coding_val,
                            default_process_coding,
                            network_buffer_multibyte,
                        );
                        proc.thread = current_thread_handle(&eval.threads);
                        proc.plist = plist_val;
                        proc.network_socket = Some(NetworkSocket::SeqpacketListener(listener));
                        if !filter_val.is_nil() {
                            proc.filter = filter_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Filter.value(),
                                proc.filter,
                            )?;
                        }
                        if !sentinel_val.is_nil() {
                            proc.sentinel = sentinel_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Sentinel.value(),
                                proc.sentinel,
                            )?;
                        }
                        if !log_val.is_nil() {
                            proc.log = log_val;
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Log.value(),
                                proc.log,
                            )?;
                        }
                        if !buffer.is_nil() {
                            proc.childp = process_contact_plist_put(
                                proc.childp,
                                ProcessKeyword::Buffer.value(),
                                buffer,
                            )?;
                        }
                        apply_connection_process_flags(proc, noquery, stop);
                    }
                    eval.processes.register_socket_fd(id).ok();
                    return Ok(Value::make_process(id));
                }

                let socket = connect_unix_seqpacket_socket(&service_path, &socket_options)?;
                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Remote.value(),
                    Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                        &service_path,
                    )),
                )?;
                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Local.value(),
                    Value::string(""),
                )?;

                let id = eval.processes.create_process_with_kind_lisp(
                    name,
                    buffer,
                    LispString::from_utf8("network"),
                    Vec::new(),
                    ProcessKind::Network,
                );
                eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                if let Some(proc) = eval.processes.get_mut(id) {
                    proc.network_socket = Some(NetworkSocket::SeqpacketStream(socket));
                    proc.status = process_status_run_value();
                    proc.childp = contact;
                    proc.plist = plist_val;
                    set_network_process_coding(
                        proc,
                        coding_val,
                        default_process_coding,
                        network_buffer_multibyte,
                    );
                    proc.thread = current_thread_handle(&eval.threads);
                    if !filter_val.is_nil() {
                        proc.filter = filter_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Filter.value(),
                            proc.filter,
                        )?;
                    }
                    if !sentinel_val.is_nil() {
                        proc.sentinel = sentinel_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Sentinel.value(),
                            proc.sentinel,
                        )?;
                    }
                    if !buffer.is_nil() {
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Buffer.value(),
                            buffer,
                        )?;
                    }
                    apply_connection_process_flags(proc, noquery, stop);
                }

                eval.processes.register_socket_fd(id).ok();

                let sentinel = eval
                    .processes
                    .get(id)
                    .map(|p| p.sentinel)
                    .unwrap_or(Value::NIL);
                if !stop {
                    eval.run_process_sentinel_callback(id, sentinel, "open\n")?;
                }

                return Ok(Value::make_process(id));
            }

            if server {
                let listener = bind_unix_listener_socket(
                    &service_path,
                    server_backlog.unwrap_or(5),
                    &socket_options,
                )?;
                contact = process_contact_plist_put(
                    contact,
                    ProcessKeyword::Local.value(),
                    Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                        &service_path,
                    )),
                )?;

                let id = eval.processes.create_process_with_kind_lisp(
                    name,
                    buffer,
                    LispString::from_utf8("network"),
                    Vec::new(),
                    ProcessKind::Network,
                );
                eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                if let Some(proc) = eval.processes.get_mut(id) {
                    proc.childp = contact;
                    set_network_process_coding(
                        proc,
                        coding_val,
                        default_process_coding,
                        network_buffer_multibyte,
                    );
                    proc.thread = current_thread_handle(&eval.threads);
                    proc.plist = plist_val;
                    proc.network_socket = Some(NetworkSocket::UnixListener(listener));
                    if !filter_val.is_nil() {
                        proc.filter = filter_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Filter.value(),
                            proc.filter,
                        )?;
                    }
                    if !sentinel_val.is_nil() {
                        proc.sentinel = sentinel_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Sentinel.value(),
                            proc.sentinel,
                        )?;
                    }
                    if !log_val.is_nil() {
                        proc.log = log_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Log.value(),
                            proc.log,
                        )?;
                    }
                    if !buffer.is_nil() {
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Buffer.value(),
                            buffer,
                        )?;
                    }
                    apply_connection_process_flags(proc, noquery, stop);
                }
                if let Some(parameters) = tls_parameters.clone() {
                    upgrade_process_to_tls::<RustlsBackend>(
                        eval,
                        id,
                        &parameters.hostname,
                        "make-network-process",
                        signal_gnutls_boot_error,
                    )?;
                }
                eval.processes.register_socket_fd(id).ok();
                return Ok(Value::make_process(id));
            }

            contact = process_contact_plist_put(
                contact,
                ProcessKeyword::Remote.value(),
                Value::heap_string(crate::emacs_core::fileio::path_to_lisp_file_name(
                    &service_path,
                )),
            )?;
            contact = process_contact_plist_put(
                contact,
                ProcessKeyword::Local.value(),
                Value::string(""),
            )?;

            if nowait {
                let start = start_nonblocking_unix_stream_socket(&service_path, &socket_options)?;
                let id = eval.processes.create_process_with_kind_lisp(
                    name,
                    buffer,
                    LispString::from_utf8("network"),
                    Vec::new(),
                    ProcessKind::Network,
                );
                eval.processes.sync_process_mark(&mut eval.buffers, id)?;
                if let Some(proc) = eval.processes.get_mut(id) {
                    proc.status = process_status_connect_value();
                    proc.childp = contact;
                    proc.plist = plist_val;
                    set_network_process_coding(
                        proc,
                        coding_val,
                        default_process_coding,
                        network_buffer_multibyte,
                    );
                    proc.thread = current_thread_handle(&eval.threads);
                    if !filter_val.is_nil() {
                        proc.filter = filter_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Filter.value(),
                            proc.filter,
                        )?;
                    }
                    if !sentinel_val.is_nil() {
                        proc.sentinel = sentinel_val;
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Sentinel.value(),
                            proc.sentinel,
                        )?;
                    }
                    if !buffer.is_nil() {
                        proc.childp = process_contact_plist_put(
                            proc.childp,
                            ProcessKeyword::Buffer.value(),
                            buffer,
                        )?;
                    }
                    match start {
                        Ok(stream) => {
                            proc.network_socket = Some(NetworkSocket::UnixStream(stream));
                            proc.pending_network_connect = Some(PendingNetworkConnect::Local);
                        }
                        Err(err) => {
                            proc.status = process_status_failed_value(io_error_status_code(&err));
                        }
                    }
                    apply_connection_process_flags(proc, noquery, stop);
                }
                if eval
                    .processes
                    .get(id)
                    .is_some_and(|proc| proc.pending_network_connect.is_some())
                {
                    eval.processes.register_socket_writable_fd(id).ok();
                }
                return Ok(Value::make_process(id));
            }

            let stream = connect_unix_stream_socket(&service_path, &socket_options)?;
            let id = eval.processes.create_process_with_kind_lisp(
                name,
                buffer,
                LispString::from_utf8("network"),
                Vec::new(),
                ProcessKind::Network,
            );
            eval.processes.sync_process_mark(&mut eval.buffers, id)?;
            if let Some(proc) = eval.processes.get_mut(id) {
                proc.network_socket = Some(NetworkSocket::UnixStream(stream));
                proc.status = process_status_run_value();
                proc.childp = contact;
                proc.plist = plist_val;
                set_network_process_coding(
                    proc,
                    coding_val,
                    default_process_coding,
                    network_buffer_multibyte,
                );
                proc.thread = current_thread_handle(&eval.threads);
                if !filter_val.is_nil() {
                    proc.filter = filter_val;
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Filter.value(),
                        proc.filter,
                    )?;
                }
                if !sentinel_val.is_nil() {
                    proc.sentinel = sentinel_val;
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Sentinel.value(),
                        proc.sentinel,
                    )?;
                }
                if !buffer.is_nil() {
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Buffer.value(),
                        buffer,
                    )?;
                }
                apply_connection_process_flags(proc, noquery, stop);
            }

            eval.processes.register_socket_fd(id).ok();

            let sentinel = eval
                .processes
                .get(id)
                .map(|p| p.sentinel)
                .unwrap_or(Value::NIL);
            if !stop {
                eval.run_process_sentinel_callback(id, sentinel, "open\n")?;
            }

            return Ok(Value::make_process(id));
        }
    }

    if socket_type == NetworkSocketType::Datagram {
        let port = parse_network_service_port(&service, server, socket_type)?;
        let host_str = host
            .clone()
            .unwrap_or_else(|| network_loopback_host_for_family(family).to_string());
        if server {
            let effective_options = tcp_server_socket_options(&socket_options);
            let socket = bind_udp_socket_host(host_str.as_str(), port, family, &effective_options)?;
            let local_addr = socket.local_addr().map_err(|e| {
                signal(
                    "file-error",
                    vec![Value::string(format!("getsockname: {}", e))],
                )
            })?;
            let zero_datagram = datagram_zero_address_for(local_addr);
            contact = process_contact_plist_put(
                contact,
                ProcessKeyword::Service.value(),
                Value::fixnum(local_addr.port() as i64),
            )?;
            contact = process_contact_plist_put(
                contact,
                ProcessKeyword::Local.value(),
                socket_addr_to_lisp_value(local_addr),
            )?;
            contact =
                process_contact_plist_put(contact, ProcessKeyword::Remote.value(), zero_datagram)?;

            let id = eval.processes.create_process_with_kind_lisp(
                name,
                buffer,
                LispString::from_utf8("network"),
                Vec::new(),
                ProcessKind::Network,
            );
            eval.processes.sync_process_mark(&mut eval.buffers, id)?;
            if let Some(proc) = eval.processes.get_mut(id) {
                proc.childp = contact;
                set_network_process_coding(
                    proc,
                    coding_val,
                    default_process_coding,
                    network_buffer_multibyte,
                );
                proc.thread = current_thread_handle(&eval.threads);
                proc.plist = plist_val;
                proc.status = ProcessStatusSymbol::Open.value();
                proc.network_socket = Some(NetworkSocket::UdpSocket(socket));
                proc.datagram_address = zero_datagram;
                proc.datagram_socket_addr = None;
                if !filter_val.is_nil() {
                    proc.filter = filter_val;
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Filter.value(),
                        proc.filter,
                    )?;
                }
                if !sentinel_val.is_nil() {
                    proc.sentinel = sentinel_val;
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Sentinel.value(),
                        proc.sentinel,
                    )?;
                }
                if !log_val.is_nil() {
                    proc.log = log_val;
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Log.value(),
                        proc.log,
                    )?;
                }
                if !buffer.is_nil() {
                    proc.childp = process_contact_plist_put(
                        proc.childp,
                        ProcessKeyword::Buffer.value(),
                        buffer,
                    )?;
                }
                apply_connection_process_flags(proc, noquery, stop);
            }
            eval.processes.register_socket_fd(id).ok();
            return Ok(Value::make_process(id));
        }

        let (socket, remote_addr) =
            connect_udp_socket_host(host_str.as_str(), port, family, &socket_options)?;
        contact = process_contact_plist_put(
            contact,
            ProcessKeyword::Remote.value(),
            socket_addr_to_lisp_value(remote_addr),
        )?;
        contact = process_contact_plist_put(
            contact,
            ProcessKeyword::Local.value(),
            socket_addr_to_lisp_value(udp_unspecified_addr_for(remote_addr)),
        )?;

        let id = eval.processes.create_process_with_kind_lisp(
            name,
            buffer,
            LispString::from_utf8("network"),
            Vec::new(),
            ProcessKind::Network,
        );
        eval.processes.sync_process_mark(&mut eval.buffers, id)?;
        if let Some(proc) = eval.processes.get_mut(id) {
            proc.network_socket = Some(NetworkSocket::UdpSocket(socket));
            proc.datagram_socket_addr = Some(remote_addr);
            proc.datagram_address = socket_addr_to_lisp_value(remote_addr);
            proc.status = ProcessStatusSymbol::Open.value();
            proc.childp = contact;
            proc.plist = plist_val;
            set_network_process_coding(
                proc,
                coding_val,
                default_process_coding,
                network_buffer_multibyte,
            );
            proc.thread = current_thread_handle(&eval.threads);
            if !filter_val.is_nil() {
                proc.filter = filter_val;
                proc.childp = process_contact_plist_put(
                    proc.childp,
                    ProcessKeyword::Filter.value(),
                    proc.filter,
                )?;
            }
            if !sentinel_val.is_nil() {
                proc.sentinel = sentinel_val;
                proc.childp = process_contact_plist_put(
                    proc.childp,
                    ProcessKeyword::Sentinel.value(),
                    proc.sentinel,
                )?;
            }
            if !buffer.is_nil() {
                proc.childp =
                    process_contact_plist_put(proc.childp, ProcessKeyword::Buffer.value(), buffer)?;
            }
            apply_connection_process_flags(proc, noquery, stop);
        }
        eval.processes.register_socket_fd(id).ok();
        return Ok(Value::make_process(id));
    }

    #[cfg(unix)]
    if socket_type == NetworkSocketType::Seqpacket {
        return Err(signal(
            "error",
            vec![Value::string("Unsupported connection type")],
        ));
    }

    if server {
        let port = parse_network_service_port(&service, true, socket_type)?;
        let host_str = host
            .clone()
            .unwrap_or_else(|| network_loopback_host_for_family(family).to_string());
        let effective_options = tcp_server_socket_options(&socket_options);
        let listener = bind_tcp_listener_host(
            host_str.as_str(),
            port,
            family,
            server_backlog.unwrap_or(5),
            &effective_options,
        )?;
        let local_addr = listener.local_addr().map_err(|e| {
            signal(
                "file-error",
                vec![Value::string(format!("getsockname: {}", e))],
            )
        })?;
        let local = socket_addr_to_lisp_value(local_addr);
        let actual_service = Value::fixnum(local_addr.port() as i64);
        contact =
            process_contact_plist_put(contact, ProcessKeyword::Service.value(), actual_service)?;
        contact = process_contact_plist_put(contact, ProcessKeyword::Local.value(), local)?;

        let id = eval.processes.create_process_with_kind_lisp(
            name,
            buffer,
            LispString::from_utf8("network"),
            Vec::new(),
            ProcessKind::Network,
        );
        eval.processes.sync_process_mark(&mut eval.buffers, id)?;
        if let Some(proc) = eval.processes.get_mut(id) {
            proc.childp = contact;
            set_network_process_coding(
                proc,
                coding_val,
                default_process_coding,
                network_buffer_multibyte,
            );
            proc.thread = current_thread_handle(&eval.threads);
            proc.plist = plist_val;
            proc.network_socket = Some(NetworkSocket::TcpListener(listener));
            if !filter_val.is_nil() {
                proc.filter = filter_val;
                proc.childp = process_contact_plist_put(
                    proc.childp,
                    ProcessKeyword::Filter.value(),
                    proc.filter,
                )?;
            }
            if !sentinel_val.is_nil() {
                proc.sentinel = sentinel_val;
                proc.childp = process_contact_plist_put(
                    proc.childp,
                    ProcessKeyword::Sentinel.value(),
                    proc.sentinel,
                )?;
            }
            if !log_val.is_nil() {
                proc.log = log_val;
                proc.childp =
                    process_contact_plist_put(proc.childp, ProcessKeyword::Log.value(), proc.log)?;
            }
            if !buffer.is_nil() {
                proc.childp =
                    process_contact_plist_put(proc.childp, ProcessKeyword::Buffer.value(), buffer)?;
            }
            apply_connection_process_flags(proc, noquery, stop);
        }
        eval.processes.register_socket_fd(id).ok();
        return Ok(Value::make_process(id));
    }

    // ---- Client mode: establish TCP connection ----
    let host_str = host.unwrap_or_else(|| network_loopback_host_for_family(family).to_string());
    let port = parse_network_service_port(&service, false, socket_type)?;

    if nowait {
        let addrs = resolve_tcp_socket_addrs(
            host_str.as_str(),
            port,
            family,
            "make client process failed",
        )?;
        let start = start_pending_tcp_stream_connect(addrs, &socket_options)?;

        let id = eval.processes.create_process_with_kind_lisp(
            name,
            buffer,
            LispString::from_utf8("network"),
            Vec::new(),
            ProcessKind::Network,
        );
        eval.processes.sync_process_mark(&mut eval.buffers, id)?;
        if let Some(proc) = eval.processes.get_mut(id) {
            proc.status = process_status_connect_value();
            proc.childp = contact;
            proc.plist = plist_val;
            set_network_process_coding(
                proc,
                coding_val,
                default_process_coding,
                network_buffer_multibyte,
            );
            proc.thread = current_thread_handle(&eval.threads);
            if !filter_val.is_nil() {
                proc.filter = filter_val;
                proc.childp = process_contact_plist_put(
                    proc.childp,
                    ProcessKeyword::Filter.value(),
                    proc.filter,
                )?;
            }
            if !sentinel_val.is_nil() {
                proc.sentinel = sentinel_val;
                proc.childp = process_contact_plist_put(
                    proc.childp,
                    ProcessKeyword::Sentinel.value(),
                    proc.sentinel,
                )?;
            }
            if !buffer.is_nil() {
                proc.childp =
                    process_contact_plist_put(proc.childp, ProcessKeyword::Buffer.value(), buffer)?;
            }
            match start {
                PendingNetworkConnectStart::Started(started) => {
                    let local_addr = started.stream.local_addr().ok();
                    proc.network_socket = Some(NetworkSocket::TcpStream(started.stream));
                    proc.pending_network_connect = Some(PendingNetworkConnect::Tcp {
                        remaining_addrs: started.remaining_addrs,
                        socket_options: socket_options.clone(),
                    });
                    ProcessManager::update_tcp_client_contact(
                        proc,
                        started.remote_addr,
                        local_addr,
                    )?;
                }
                PendingNetworkConnectStart::Failed(code) => {
                    proc.status = process_status_failed_value(code);
                }
            }
            apply_connection_process_flags(proc, noquery, stop);
        }

        if eval
            .processes
            .get(id)
            .is_some_and(|proc| proc.pending_network_connect.is_some())
        {
            eval.processes.register_socket_writable_fd(id).ok();
        }
        return Ok(Value::make_process(id));
    }

    let stream = connect_tcp_stream_host(host_str.as_str(), port, family, &socket_options)?;
    let remote_addr = stream.peer_addr().ok();
    let local_addr = stream.local_addr().ok();

    let id = eval.processes.create_process_with_kind_lisp(
        name,
        buffer,
        LispString::from_utf8("network"),
        Vec::new(),
        ProcessKind::Network,
    );
    eval.processes.sync_process_mark(&mut eval.buffers, id)?;
    if let Some(addr) = remote_addr {
        contact = process_contact_plist_put(
            contact,
            ProcessKeyword::Remote.value(),
            socket_addr_to_lisp_value(addr),
        )?;
    }
    if let Some(addr) = local_addr {
        contact = process_contact_plist_put(
            contact,
            ProcessKeyword::Local.value(),
            socket_addr_to_lisp_value(addr),
        )?;
    }
    if let Some(proc) = eval.processes.get_mut(id) {
        proc.network_socket = Some(NetworkSocket::TcpStream(stream));
        proc.status = process_status_run_value();
        proc.childp = contact;
        proc.plist = plist_val;
        set_network_process_coding(
            proc,
            coding_val,
            default_process_coding,
            network_buffer_multibyte,
        );
        proc.thread = current_thread_handle(&eval.threads);
        if !filter_val.is_nil() {
            proc.filter = filter_val;
            proc.childp = process_contact_plist_put(
                proc.childp,
                ProcessKeyword::Filter.value(),
                proc.filter,
            )?;
        }
        if !sentinel_val.is_nil() {
            proc.sentinel = sentinel_val;
            proc.childp = process_contact_plist_put(
                proc.childp,
                ProcessKeyword::Sentinel.value(),
                proc.sentinel,
            )?;
        }
        if !buffer.is_nil() {
            proc.childp =
                process_contact_plist_put(proc.childp, ProcessKeyword::Buffer.value(), buffer)?;
        }
        apply_connection_process_flags(proc, noquery, stop);
    }

    if let Some(parameters) = tls_parameters {
        upgrade_process_to_tls::<RustlsBackend>(
            eval,
            id,
            &parameters.hostname,
            "make-network-process",
            signal_gnutls_boot_error,
        )?;
    }

    eval.processes.register_socket_fd(id).ok();

    // Call sentinel with "open\n" to signal successful connection
    // (GNU Emacs calls the sentinel when a network connection opens).
    let sentinel = eval
        .processes
        .get(id)
        .map(|p| p.sentinel)
        .unwrap_or(Value::NIL);
    if !stop {
        eval.run_process_sentinel_callback(id, sentinel, "open\n")?;
    }

    Ok(Value::make_process(id))
}

/// (make-pipe-process &rest ARGS) -> process-or-nil
pub(crate) fn builtin_make_pipe_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_make_pipe_process_impl(
        &mut eval.processes,
        &mut eval.buffers,
        &eval.threads,
        Some(&eval.coding_systems),
        args,
    )
}

pub(crate) fn builtin_make_pipe_process_impl(
    processes: &mut ProcessManager,
    buffers: &mut BufferManager,
    threads: &ThreadManager,
    coding_systems: Option<&super::coding::CodingSystemManager>,
    args: Vec<Value>,
) -> EvalResult {
    if args.is_empty() {
        return Ok(Value::NIL);
    }

    let contact = Value::list(args.clone());
    let mut name: Option<LispString> = None;
    let mut buffer: Option<Value> = None;
    let mut filter = Value::NIL;
    let mut sentinel = Value::NIL;
    let mut coding = Value::NIL;
    let mut coding_present = false;
    let mut noquery = false;
    let mut stop = false;
    let mut plist = Value::NIL;

    let mut i = 0usize;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).cloned().unwrap_or(Value::NIL);
        let Some(keyword) = ProcessKeyword::from_value(key) else {
            i += 1;
            continue;
        };
        match keyword {
            ProcessKeyword::Name => {
                name = Some(expect_process_name_lisp_string(&value)?);
            }
            ProcessKeyword::Buffer => {
                buffer = Some(parse_make_process_buffer_in_state(buffers, &value)?);
            }
            ProcessKeyword::Filter => filter = value,
            ProcessKeyword::Sentinel => sentinel = value,
            ProcessKeyword::Coding => {
                coding = value;
                coding_present = true;
            }
            ProcessKeyword::Noquery => noquery = value.is_truthy(),
            ProcessKeyword::Stop => stop = value.is_truthy(),
            ProcessKeyword::Plist => plist = value,
            _ => {}
        }
        i += 2;
    }

    let Some(name) = name else {
        return Err(signal(
            "error",
            vec![Value::string("Missing :name keyword parameter")],
        ));
    };

    let resolved_buffer = match buffer {
        Some(explicit) => explicit,
        None => {
            // Issue #131: buffer-name lookup/creation takes a `&str`; a lossy
            // UTF-8 rendering is the right display form here and avoids the
            // buggy storage-string sentinels.
            let name_runtime = crate::emacs_core::emacs_char::to_utf8_lossy(name.as_bytes());
            let id = buffers
                .find_buffer_by_name(&name_runtime)
                .unwrap_or_else(|| buffers.create_buffer(&name_runtime));
            Value::make_buffer(id)
        }
    };
    if coding_present {
        validate_process_coding_value(coding_systems, coding)?;
    }
    let plist = copy_process_plist(plist)?;

    let id = processes.create_process_with_kind_lisp(
        name,
        resolved_buffer,
        LispString::from_utf8("pipe"),
        Vec::new(),
        ProcessKind::Pipe,
    );
    processes.sync_process_mark(buffers, id)?;
    if let Some(proc) = processes.get_mut(id) {
        proc.childp = contact;
        proc.thread = current_thread_handle(threads);
        proc.plist = plist;
        if !filter.is_nil() {
            proc.filter = filter;
        }
        if !sentinel.is_nil() {
            proc.sentinel = sentinel;
        }
        if coding_present {
            set_explicit_process_coding(proc, coding);
        }
        apply_connection_process_flags(proc, noquery, stop);
    }
    Ok(Value::make_process(id))
}

/// (make-serial-process &rest ARGS) -> process-or-nil
pub(crate) fn builtin_make_serial_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_make_serial_process_impl(
        &mut eval.processes,
        &mut eval.buffers,
        &eval.threads,
        Some(&eval.coding_systems),
        args,
    )
}

pub(crate) fn builtin_make_serial_process_impl(
    processes: &mut ProcessManager,
    buffers: &mut BufferManager,
    threads: &ThreadManager,
    coding_systems: Option<&super::coding::CodingSystemManager>,
    args: Vec<Value>,
) -> EvalResult {
    if args.is_empty() {
        return Ok(Value::NIL);
    }

    let contact = Value::list(args.clone());
    let mut name: Option<LispString> = None;
    let mut port: Option<Value> = None;
    let mut port_name: Option<LispString> = None;
    let mut speed: Option<Value> = None;
    let mut buffer: Option<Value> = None;
    let mut filter = Value::NIL;
    let mut sentinel = Value::NIL;
    let mut coding = Value::NIL;
    let mut coding_present = false;
    let mut noquery = false;
    let mut stop = false;
    let mut plist = Value::NIL;

    let mut i = 0usize;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).cloned().unwrap_or(Value::NIL);
        let Some(keyword) = ProcessKeyword::from_value(key) else {
            i += 1;
            continue;
        };
        match keyword {
            ProcessKeyword::Name => {
                name = Some(expect_process_name_lisp_string(&value)?);
            }
            ProcessKeyword::Port => {
                if value.is_nil() {
                    port = None;
                } else {
                    let string = super::builtins::expect_lisp_string(&value)?.clone();
                    port = Some(value);
                    port_name = Some(string);
                }
            }
            ProcessKeyword::Speed => {
                if !value.is_nil() && !value.is_fixnum() {
                    return Err(signal(
                        "wrong-type-argument",
                        vec![Value::symbol("fixnump"), value],
                    ));
                }
                speed = Some(value);
            }
            ProcessKeyword::Buffer => {
                buffer = Some(if value.is_nil() {
                    Value::NIL
                } else {
                    parse_make_process_buffer_in_state(buffers, &value)?
                });
            }
            ProcessKeyword::Filter => filter = value,
            ProcessKeyword::Sentinel => sentinel = value,
            ProcessKeyword::Coding => {
                coding = value;
                coding_present = true;
            }
            ProcessKeyword::Noquery => noquery = value.is_truthy(),
            ProcessKeyword::Stop => stop = value.is_truthy(),
            ProcessKeyword::Plist => plist = value,
            _ => {}
        }
        i += 2;
    }

    if port.is_none() {
        return Err(signal("error", vec![Value::string("No port specified")]));
    }
    if speed.is_none() {
        return Err(signal("error", vec![Value::string(":speed not specified")]));
    }
    if coding_present {
        validate_process_coding_value(coding_systems, coding)?;
    }
    let plist = copy_process_plist(plist)?;
    let name = name.unwrap_or_else(|| port_name.clone().expect("port is present after validation"));
    let resolved_buffer = match buffer {
        Some(explicit) if !explicit.is_nil() => explicit,
        _ => {
            let name_runtime = crate::emacs_core::emacs_char::to_utf8_lossy(name.as_bytes());
            let id = buffers
                .find_buffer_by_name(&name_runtime)
                .unwrap_or_else(|| buffers.create_buffer(&name_runtime));
            Value::make_buffer(id)
        }
    };

    let id = processes.create_process_with_kind_lisp(
        name,
        resolved_buffer,
        LispString::from_utf8("serial"),
        Vec::new(),
        ProcessKind::Serial,
    );
    processes.sync_process_mark(buffers, id)?;
    if let Some(proc) = processes.get_mut(id) {
        proc.childp = contact;
        proc.thread = current_thread_handle(threads);
        proc.status = ProcessStatusSymbol::Open.value();
        proc.plist = plist;
        if !filter.is_nil() {
            proc.filter = filter;
        }
        if !sentinel.is_nil() {
            proc.sentinel = sentinel;
        }
        if coding_present {
            set_explicit_process_coding(proc, coding);
        }
        apply_connection_process_flags(proc, noquery, stop);
    }
    Ok(Value::make_process(id))
}

/// (serial-process-configure &rest ARGS) -> nil
pub(crate) fn builtin_serial_process_configure(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_serial_process_configure_impl(&eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_serial_process_configure_impl(
    processes: &ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    let mut process_id: Option<ProcessId> = None;
    let mut i = 0usize;
    while i < args.len() {
        let key = &args[i];
        let Some(keyword) = ProcessKeyword::from_value(key) else {
            i += 1;
            continue;
        };
        let value = args.get(i + 1).cloned().unwrap_or(Value::NIL);
        match keyword {
            ProcessKeyword::Process => {
                if value.is_nil() {
                    process_id = None;
                } else {
                    process_id = Some(resolve_process_or_missing_error_in_manager(
                        processes, &value,
                    )?);
                }
            }
            ProcessKeyword::Name => match value.kind() {
                ValueKind::String => {
                    let name_str = process_owned_runtime_string(value);
                    process_id = Some(
                        processes
                            .find_by_name(&name_str)
                            .ok_or_else(|| signal_process_does_not_exist(&name_str))?,
                    );
                }
                _ => return Err(signal_wrong_type_processp(value)),
            },
            _ => {}
        }
        i += 2;
    }

    let id = match process_id {
        Some(id) => id,
        None => resolve_optional_process_or_current_buffer_in_state(processes, buffers, None)?,
    };
    let proc = processes
        .get(id)
        .ok_or_else(|| signal_wrong_type_processp(Value::make_process(id)))?;
    if proc.kind != ProcessKind::Serial {
        return Err(signal("error", vec![Value::string("Not a serial process")]));
    }
    Ok(Value::NIL)
}

/// (set-network-process-option PROCESS OPTION VALUE &optional NO-ERROR) -> nil
pub(crate) fn builtin_set_network_process_option(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if (3..=4).contains(&args.len())
        && let Some(id) = pending_network_connect_id(&eval.processes, args[0])?
    {
        eval.wait_while_network_process_connecting(id)?;
    }
    builtin_set_network_process_option_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_network_process_option_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() < 3 || args.len() > 4 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("set-network-process-option"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let id = resolve_live_process_or_wrong_type_in_manager(processes, &args[0])?;
    if args[1].as_symbol_name().is_none() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[1]],
        ));
    }
    let no_error = args.get(3).is_some_and(|v| v.is_truthy());
    let Some(keyword) = ProcessKeyword::from_value(&args[1]) else {
        return if no_error {
            Ok(Value::NIL)
        } else {
            Err(signal(
                "error",
                vec![Value::string("Unknown or unsupported option")],
            ))
        };
    };
    let Some(option) = NetworkSocketOption::from_keyword(keyword) else {
        return if no_error {
            Ok(Value::NIL)
        } else {
            Err(signal(
                "error",
                vec![Value::string("Unknown or unsupported option")],
            ))
        };
    };

    let proc = processes.get_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    if proc.kind != ProcessKind::Network {
        return Err(signal(
            "error",
            vec![Value::string("Process is not a network process")],
        ));
    }

    let spec = NetworkSocketOptionSpec {
        keyword,
        option,
        value: args[2],
    };
    apply_network_socket_option_to_process(proc, spec)?;
    proc.childp = process_contact_plist_put(proc.childp, args[1], args[2])?;
    Ok(Value::T)
}

/// (start-process NAME BUFFER PROGRAM &rest ARGS) -> process-id
pub(crate) fn builtin_start_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("start-process", &args, 3)?;
    let name = expect_process_name_lisp_string(&args[0])?;
    let buffer = parse_make_process_buffer(eval, &args[1])?;
    let program = if args[2].is_nil() {
        LispString::from_utf8("nil")
    } else {
        super::builtins::expect_lisp_string(&args[2])?.clone()
    };
    let proc_args = parse_lisp_string_args_strict(&args[3..])?;
    let default_directory =
        super::fileio::default_directory_lisp_in_state(&eval.obarray, &[], &eval.buffers);
    let lookup = ProcessExecLookup {
        exec_path: eval.visible_variable_value_or_nil("exec-path"),
        exec_suffixes: eval.visible_variable_value_or_nil("exec-suffixes"),
        default_directory: default_directory.as_ref(),
    };
    let process_environment = Some(eval.visible_variable_value_or_nil("process-environment"));
    let executable = if args[2].is_nil() {
        None
    } else {
        Some(resolve_async_process_program(lookup, &program)?)
    };

    let use_pty = process_connection_type_is_pty(&eval.obarray);
    let subprocess_cwd = super::callproc::subprocess_default_directory(eval);
    let id = eval
        .processes
        .create_process_lisp_resolved(name, buffer, program, proc_args, executable);
    if let Some(cwd) = &subprocess_cwd {
        if let Some(proc) = eval.processes.get_mut(id) {
            proc.default_directory = Some(cwd.clone());
        }
    }
    eval.processes.sync_process_mark(&mut eval.buffers, id)?;

    // Actually spawn the OS process.
    if let Err(e) = eval
        .processes
        .spawn_child_with_environment(id, use_pty, process_environment)
    {
        // Process creation failed — mark as exited but still return the id
        // (GNU Emacs signals file-error for missing programs)
        return Err(signal(
            "file-missing",
            vec![
                Value::string("Searching for program"),
                Value::string(e),
                args[2],
            ],
        ));
    }

    Ok(Value::make_process(id))
}

/// (start-process-shell-command NAME BUFFER COMMAND) -> process-id
pub(crate) fn builtin_start_process_shell_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("start-process-shell-command", &args, 3)?;
    let name = expect_process_name_lisp_string(&args[0])?;
    let buffer = parse_make_process_buffer(eval, &args[1])?;
    let command = super::builtins::expect_lisp_string(&args[2])?.clone();
    let use_pty = process_connection_type_is_pty(&eval.obarray);
    let id = eval.processes.create_process_lisp(
        name,
        buffer,
        LispString::from_utf8("sh"),
        vec![LispString::from_utf8("-c"), command],
    );
    eval.processes.sync_process_mark(&mut eval.buffers, id)?;

    // Honor the dynamically-bound `process-environment` (let-bindings), not
    // just the global value, matching GNU's `make_environment_block`.
    let process_environment = Some(eval.visible_variable_value_or_nil("process-environment"));
    // Actually spawn the OS process.
    if let Err(e) = eval
        .processes
        .spawn_child_with_environment(id, use_pty, process_environment)
    {
        return Err(signal(
            "file-error",
            vec![Value::string("Searching for program"), Value::string(e)],
        ));
    }

    Ok(Value::make_process(id))
}

/// (start-file-process NAME BUFFER PROGRAM &rest PROGRAM-ARGS) -> process-id
pub(crate) fn builtin_start_file_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("start-file-process", &args, 3)?;
    let name = expect_process_name_lisp_string(&args[0])?;
    let buffer = parse_make_process_buffer(eval, &args[1])?;
    let program = if args[2].is_nil() {
        LispString::from_utf8("nil")
    } else {
        super::builtins::expect_lisp_string(&args[2])?.clone()
    };
    let proc_args = parse_lisp_string_args_strict(&args[3..])?;
    let use_pty = process_connection_type_is_pty(&eval.obarray);
    let id = eval
        .processes
        .create_process_lisp(name, buffer, program, proc_args);
    eval.processes.sync_process_mark(&mut eval.buffers, id)?;

    // Honor the dynamically-bound `process-environment` (let-bindings).
    let process_environment = Some(eval.visible_variable_value_or_nil("process-environment"));
    // NeoVM has no Tramp/remote support, so behave like start-process.
    if let Err(e) = eval
        .processes
        .spawn_child_with_environment(id, use_pty, process_environment)
    {
        return Err(signal(
            "file-error",
            vec![
                Value::string("Searching for program"),
                Value::string(e),
                args[2],
            ],
        ));
    }

    Ok(Value::make_process(id))
}

/// (start-file-process-shell-command NAME BUFFER COMMAND) -> process-id
pub(crate) fn builtin_start_file_process_shell_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("start-file-process-shell-command", &args, 3)?;
    let name = expect_process_name_lisp_string(&args[0])?;
    let buffer = parse_make_process_buffer(eval, &args[1])?;
    let command = super::builtins::expect_lisp_string(&args[2])?.clone();
    let use_pty = process_connection_type_is_pty(&eval.obarray);
    let id = eval.processes.create_process_lisp(
        name,
        buffer,
        LispString::from_utf8("sh"),
        vec![LispString::from_utf8("-c"), command],
    );
    eval.processes.sync_process_mark(&mut eval.buffers, id)?;

    // Honor the dynamically-bound `process-environment` (let-bindings).
    let process_environment = Some(eval.visible_variable_value_or_nil("process-environment"));
    // NeoVM has no Tramp/remote support, so behave like start-process-shell-command.
    if let Err(e) = eval
        .processes
        .spawn_child_with_environment(id, use_pty, process_environment)
    {
        return Err(signal(
            "file-error",
            vec![Value::string("Searching for program"), Value::string(e)],
        ));
    }

    Ok(Value::make_process(id))
}

/// (call-process PROGRAM &optional INFILE DESTINATION DISPLAY &rest ARGS)
///
/// Runs the command synchronously using `std::process::Command`, captures
/// output.  Returns the exit code as an integer.
pub(crate) fn builtin_call_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_call_process(eval, args)
}

/// (call-process-shell-command COMMAND &optional INFILE DESTINATION DISPLAY &rest ARGS)
pub(crate) fn builtin_call_process_shell_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_call_process_shell_command(eval, args)
}

/// (process-file PROGRAM &optional INFILE DESTINATION DISPLAY &rest ARGS)
pub(crate) fn builtin_process_file(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_process_file(eval, args)
}

/// (process-file-shell-command COMMAND &optional INFILE DESTINATION DISPLAY &rest ARGS)
pub(crate) fn builtin_process_file_shell_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_process_file_shell_command(eval, args)
}

/// (process-lines PROGRAM &rest ARGS) -> list of lines
pub(crate) fn builtin_process_lines(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_process_lines(_eval, args)
}

/// (process-lines-ignore-status PROGRAM &rest ARGS) -> list of lines
pub(crate) fn builtin_process_lines_ignore_status(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_process_lines_ignore_status(_eval, args)
}

/// (process-lines-handling-status PROGRAM STATUS-HANDLER &rest ARGS) -> list of lines
pub(crate) fn builtin_process_lines_handling_status(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_process_lines_handling_status(eval, args)
}

/// (call-process-region START END PROGRAM &optional DELETE DESTINATION DISPLAY &rest ARGS)
///
/// Pipes buffer region from START to END through PROGRAM.
pub(crate) fn builtin_call_process_region(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    super::callproc::builtin_call_process_region(eval, args)
}

/// (delete-process PROCESS) -> nil
pub(crate) fn builtin_delete_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_delete_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_delete_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 1 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("delete-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let id = if let Some(process) = args.first() {
        if process.is_nil() {
            resolve_optional_process_or_current_buffer_in_state(processes, buffers, args.first())?
        } else {
            resolve_process_or_missing_error_any_in_manager(processes, process)?
        }
    } else {
        resolve_optional_process_or_current_buffer_in_state(processes, buffers, args.first())?
    };
    processes.delete_process(id);
    Ok(Value::NIL)
}

/// (continue-process &optional PROCESS CURRENT-GROUP) -> process-or-nil
pub(crate) fn builtin_continue_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_continue_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_continue_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("continue-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let (id, ret) =
        resolve_optional_process_with_explicit_return_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        if matches!(
            proc.kind,
            ProcessKind::Network | ProcessKind::Pipe | ProcessKind::Serial
        ) {
            proc.command = Value::NIL;
            if proc.kind == ProcessKind::Serial {
                proc.status = ProcessStatusSymbol::Open.value();
            }
        } else {
            // Send SIGCONT to resume the child process.
            #[cfg(unix)]
            if let Some(ref child) = proc.child {
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGCONT);
                }
            }
            proc.status = process_status_run_value();
        }
    }
    Ok(ret)
}

/// (interrupt-process &optional PROCESS CURRENT-GROUP) -> process-or-nil
pub(crate) fn builtin_interrupt_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_interrupt_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_interrupt_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("interrupt-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let (id, ret) =
        resolve_optional_process_with_explicit_return_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        // Send SIGINT to actual child process.
        #[cfg(unix)]
        if let Some(ref child) = proc.child {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
        }
        proc.status = process_status_signal_value(2);
    }
    Ok(ret)
}

/// (kill-process &optional PROCESS CURRENT-GROUP) -> process-or-nil
pub(crate) fn builtin_kill_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_kill_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_kill_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("kill-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let (id, ret) =
        resolve_optional_process_with_explicit_return_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        // Kill the actual child process.
        if let Some(child) = proc.child.as_mut() {
            let _ = child.kill();
        }
        proc.status = process_status_signal_value(9);
    }
    Ok(ret)
}

/// (signal-process PROCESS SIGNAL &optional CURRENT-GROUP) -> int-or-nil
pub(crate) fn builtin_signal_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_signal_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_signal_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("signal-process", &args, 2)?;
    if args.len() > 3 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("signal-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    if let Some(process) = args.first() {
        if !process.is_nil() && is_stale_process_id_designator_in_manager(processes, process) {
            return Ok(Value::fixnum(-1));
        }
    }

    let signal_num = parse_signal_number(&args[1])?;
    match resolve_signal_process_target_in_state(processes, buffers, args.first())? {
        SignalProcessTarget::Process(id) => {
            if let Some(proc) = processes.get_mut(id) {
                // Send actual OS signal to child process.
                #[cfg(unix)]
                if let Some(ref child) = proc.child {
                    let pid = child.id() as i32;
                    unsafe {
                        libc::kill(pid, signal_num);
                    }
                }
                proc.status = process_status_signal_value(signal_num);
            }
            Ok(Value::fixnum(0))
        }
        SignalProcessTarget::MissingNamedProcess => Ok(Value::NIL),
        SignalProcessTarget::Pid(pid) => {
            #[cfg(unix)]
            {
                let result = unsafe { libc::kill(pid as i32, signal_num) };
                Ok(Value::fixnum(result as i64))
            }
            #[cfg(not(unix))]
            {
                Ok(Value::fixnum(if pid_exists(pid) { 0 } else { -1 }))
            }
        }
    }
}

/// (stop-process &optional PROCESS CURRENT-GROUP) -> process-or-nil
pub(crate) fn builtin_stop_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_stop_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_stop_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("stop-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let (id, ret) =
        resolve_optional_process_with_explicit_return_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        if matches!(
            proc.kind,
            ProcessKind::Network | ProcessKind::Pipe | ProcessKind::Serial
        ) {
            proc.command = Value::T;
        } else {
            // Send SIGTSTP to stop the child process.
            #[cfg(unix)]
            if let Some(ref child) = proc.child {
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGTSTP);
                }
            }
            proc.status = process_status_stop_value(0);
        }
    }
    Ok(ret)
}

/// (quit-process &optional PROCESS CURRENT-GROUP) -> process-or-nil
pub(crate) fn builtin_quit_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_quit_process_impl(&mut eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_quit_process_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("quit-process"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let (id, ret) =
        resolve_optional_process_with_explicit_return_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        // Send SIGQUIT to the child process.
        #[cfg(unix)]
        if let Some(ref child) = proc.child {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGQUIT);
            }
        }
    }
    Ok(ret)
}

/// (process-attributes PID) -> alist-or-nil
pub(crate) fn builtin_process_attributes(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_attributes_impl(args)
}

pub(crate) fn builtin_process_attributes_impl(args: Vec<Value>) -> EvalResult {
    expect_args("process-attributes", &args, 1)?;
    let pid = match args[0].kind() {
        ValueKind::Fixnum(n) => n,
        _ => return Err(signal_wrong_type_numberp(args[0])),
    };
    if !pid_exists(pid) {
        return Ok(Value::NIL);
    }

    let mut attrs = Vec::new();
    if let Some((euid, egid)) = parse_effective_ids_from_proc_status(pid) {
        attrs.push(Value::cons(
            Value::symbol("group"),
            Value::string(lookup_group_name(egid).unwrap_or_else(|| egid.to_string())),
        ));
        attrs.push(Value::cons(
            Value::symbol("egid"),
            Value::fixnum(egid as i64),
        ));
        attrs.push(Value::cons(
            Value::symbol("user"),
            Value::string(lookup_user_name(euid).unwrap_or_else(|| euid.to_string())),
        ));
        attrs.push(Value::cons(
            Value::symbol("euid"),
            Value::fixnum(euid as i64),
        ));
    }

    let stat = parse_proc_stat_snapshot(pid).unwrap_or_else(|| ProcStatSnapshot::fallback(pid));
    attrs.push(Value::cons(Value::symbol("comm"), Value::string(stat.comm)));
    attrs.push(Value::cons(
        Value::symbol("state"),
        Value::string(stat.state),
    ));
    attrs.push(Value::cons(Value::symbol("ppid"), Value::fixnum(stat.ppid)));
    attrs.push(Value::cons(Value::symbol("pgrp"), Value::fixnum(stat.pgrp)));
    attrs.push(Value::cons(Value::symbol("sess"), Value::fixnum(stat.sess)));
    attrs.push(Value::cons(
        Value::symbol("tpgid"),
        Value::fixnum(stat.tpgid),
    ));
    attrs.push(Value::cons(
        Value::symbol("minflt"),
        Value::fixnum(stat.minflt),
    ));
    attrs.push(Value::cons(
        Value::symbol("majflt"),
        Value::fixnum(stat.majflt),
    ));
    attrs.push(Value::cons(
        Value::symbol("cminflt"),
        Value::fixnum(stat.cminflt),
    ));
    attrs.push(Value::cons(
        Value::symbol("cmajflt"),
        Value::fixnum(stat.cmajflt),
    ));
    attrs.push(Value::cons(
        Value::symbol("utime"),
        time_list_from_ticks(stat.utime_ticks, clock_ticks_per_second()),
    ));
    attrs.push(Value::cons(
        Value::symbol("stime"),
        time_list_from_ticks(stat.stime_ticks, clock_ticks_per_second()),
    ));
    let total_ticks = stat.utime_ticks.saturating_add(stat.stime_ticks);
    attrs.push(Value::cons(
        Value::symbol("time"),
        time_list_from_ticks(total_ticks, clock_ticks_per_second()),
    ));
    attrs.push(Value::cons(
        Value::symbol("cutime"),
        time_list_from_ticks(stat.cutime_ticks, clock_ticks_per_second()),
    ));
    attrs.push(Value::cons(
        Value::symbol("cstime"),
        time_list_from_ticks(stat.cstime_ticks, clock_ticks_per_second()),
    ));
    let total_child_ticks = stat.cutime_ticks.saturating_add(stat.cstime_ticks);
    attrs.push(Value::cons(
        Value::symbol("ctime"),
        time_list_from_ticks(total_child_ticks, clock_ticks_per_second()),
    ));
    attrs.push(Value::cons(Value::symbol("pri"), Value::fixnum(stat.pri)));
    attrs.push(Value::cons(Value::symbol("nice"), Value::fixnum(stat.nice)));
    attrs.push(Value::cons(
        Value::symbol("thcount"),
        Value::fixnum(stat.thcount),
    ));
    let hz = clock_ticks_per_second();
    let start_epoch_time = parse_proc_boot_time_secs().map(|boot_secs| {
        let (start_rel_secs, start_rel_usecs) = ticks_to_secs_usecs(stat.start_ticks, hz);
        (boot_secs.saturating_add(start_rel_secs), start_rel_usecs)
    });
    let (start_secs, start_usecs) = start_epoch_time.unwrap_or((0, 0));
    attrs.push(Value::cons(
        Value::symbol("start"),
        time_list_from_secs_usecs(start_secs, start_usecs),
    ));
    attrs.push(Value::cons(
        Value::symbol("vsize"),
        Value::fixnum(stat.vsize),
    ));
    attrs.push(Value::cons(Value::symbol("rss"), Value::fixnum(stat.rss)));
    let elapsed = match (now_epoch_secs_usecs(), start_epoch_time) {
        (Some(now), Some(start)) => nonnegative_time_diff(now, start),
        _ => (0, 0),
    };
    attrs.push(Value::cons(
        Value::symbol("etime"),
        time_list_from_secs_usecs(elapsed.0, elapsed.1),
    ));
    let elapsed_secs = elapsed.0 as f64 + (elapsed.1 as f64 / 1_000_000.0);
    let total_cpu_secs = if hz > 0 {
        (total_ticks as f64) / (hz as f64)
    } else {
        0.0
    };
    let pcpu = if elapsed_secs > 0.0 {
        (total_cpu_secs * 100.0) / elapsed_secs
    } else {
        0.0
    };
    attrs.push(Value::cons(
        Value::symbol("pcpu"),
        Value::make_float(if pcpu.is_finite() { pcpu.max(0.0) } else { 0.0 }),
    ));
    let pmem = parse_total_memory_kb()
        .filter(|mem_total_kb| *mem_total_kb > 0)
        .map(|mem_total_kb| (stat.rss as f64 * 100.0) / mem_total_kb as f64)
        .unwrap_or(0.0);
    attrs.push(Value::cons(
        Value::symbol("pmem"),
        Value::make_float(if pmem.is_finite() { pmem.max(0.0) } else { 0.0 }),
    ));
    attrs.push(Value::cons(
        Value::symbol("args"),
        Value::string(parse_proc_cmdline(pid)),
    ));
    attrs.push(Value::cons(
        Value::symbol("ttname"),
        Value::string(stat.ttname),
    ));

    Ok(Value::list(attrs))
}

/// (make-process &rest ARGS) -> process-or-nil
pub(crate) fn builtin_make_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let use_pty = process_connection_type_is_pty(&eval.obarray);
    let default_directory =
        super::fileio::default_directory_lisp_in_state(&eval.obarray, &[], &eval.buffers);
    let lookup = ProcessExecLookup {
        exec_path: eval.visible_variable_value_or_nil("exec-path"),
        exec_suffixes: eval.visible_variable_value_or_nil("exec-suffixes"),
        default_directory: default_directory.as_ref(),
    };
    let process_environment = Some(eval.visible_variable_value_or_nil("process-environment"));
    let subprocess_cwd = super::callproc::subprocess_default_directory(eval);
    builtin_make_process_impl_with_environment(
        &mut eval.processes,
        &mut eval.buffers,
        &eval.threads,
        args,
        use_pty,
        process_environment,
        Some(lookup),
        subprocess_cwd,
        Some(&eval.coding_systems),
    )
}

pub(crate) fn builtin_make_process_impl(
    processes: &mut ProcessManager,
    buffers: &mut BufferManager,
    threads: &ThreadManager,
    args: Vec<Value>,
    default_use_pty: bool,
) -> EvalResult {
    builtin_make_process_impl_with_environment(
        processes,
        buffers,
        threads,
        args,
        default_use_pty,
        None,
        None,
        None,
        None,
    )
}

fn builtin_make_process_impl_with_environment(
    processes: &mut ProcessManager,
    buffers: &mut BufferManager,
    threads: &ThreadManager,
    args: Vec<Value>,
    default_use_pty: bool,
    process_environment: Option<Value>,
    lookup: Option<ProcessExecLookup<'_>>,
    subprocess_cwd: Option<PathBuf>,
    coding_systems: Option<&super::coding::CodingSystemManager>,
) -> EvalResult {
    if args.is_empty() {
        return Ok(Value::NIL);
    }

    let mut name: Option<LispString> = None;
    let mut buffer: Option<Value> = None;
    let mut command: Option<Vec<LispString>> = None;
    let mut filter = Value::NIL;
    let mut sentinel = Value::NIL;
    let mut connection_type: Option<Value> = None;
    let mut stderr_target = Value::NIL;
    let mut coding_val: Option<Value> = None;
    let mut noquery = false;
    let mut stop_val = Value::NIL;

    let mut i = 0usize;
    while i < args.len() {
        let key = &args[i];
        let value = args.get(i + 1).cloned().unwrap_or(Value::NIL);
        match ProcessKeyword::from_value(key) {
            Some(ProcessKeyword::Name) => name = Some(expect_process_name_lisp_string(&value)?),
            Some(ProcessKeyword::Buffer) => {
                buffer = Some(parse_make_process_buffer_in_state(buffers, &value)?)
            }
            Some(ProcessKeyword::Command) => command = Some(parse_make_process_command(&value)?),
            Some(ProcessKeyword::Filter) => filter = value,
            Some(ProcessKeyword::Sentinel) => sentinel = value,
            Some(ProcessKeyword::ConnectionType) => connection_type = Some(value),
            Some(ProcessKeyword::Stderr) => stderr_target = value,
            Some(ProcessKeyword::Coding) => coding_val = Some(value),
            Some(ProcessKeyword::Noquery) => noquery = value.is_truthy(),
            Some(ProcessKeyword::Stop) => stop_val = value,
            _ => {}
        }
        i += 2;
    }

    let Some(name) = name else {
        return Err(signal(
            "error",
            vec![Value::string("Missing :name keyword parameter")],
        ));
    };

    // Determine PTY vs pipe exactly as GNU's `is_pty_from_symbol` does:
    // nil inherits `process-connection-type`; only `pipe` and `pty` are
    // accepted explicit symbols.
    //
    // GNU's `Fmake_process` (src/process.c) decides STDIN/STDOUT's pty-vs-pipe
    // *solely* from `:connection-type` / `process-connection-type` and stores it
    // in `pty_in`/`pty_out`.  Supplying `:stderr` only routes the child's
    // *stderr* to a separate pipe process (`stderrproc`); it does NOT flip
    // stdin/stdout to a pipe.  In `create_process`, the pty is allocated when
    // `pty_in || pty_out`, stdin/stdout use the pty channels, and the stderr
    // pipe is wired through a wholly separate `forkerr` fd.  Hence with the
    // default connection-type (pty) and `:stderr`, GNU reports
    // `(process-tty-name p 'stdout)` => "/dev/pts/N" and `'stderr` => nil.
    //
    // The previous code wrongly forced `use_pty = false` whenever `:stderr` was
    // given, downgrading stdout from a pty to a pipe and diverging from GNU.
    let use_pty =
        resolve_process_connection_type_use_pty(connection_type.as_ref(), default_use_pty)?;

    let command = command.unwrap_or_default();
    if !stop_val.is_nil() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("null"), stop_val],
        ));
    }
    if let Some(coding) = coding_val {
        validate_process_coding_value(coding_systems, coding)?;
    }
    let (program, argv) = if command.is_empty() {
        (LispString::from_utf8(""), Vec::new())
    } else {
        (command[0].clone(), command[1..].to_vec())
    };
    let executable = if program.is_empty() {
        None
    } else if let Some(lookup) = lookup {
        Some(resolve_async_process_program(lookup, &program)?)
    } else {
        None
    };
    let stderrproc = if stderr_target.is_nil() {
        Value::NIL
    } else if let Some(stderr_id) = process_value_to_id(&stderr_target) {
        // An existing process (object or legacy id) is reused as the stderr
        // pipe; GNU requires it to be a pipe process.
        let stderr_proc = processes
            .get_any(stderr_id)
            .ok_or_else(|| signal_wrong_type_processp(stderr_target))?;
        if stderr_proc.kind != ProcessKind::Pipe {
            return Err(signal(
                "error",
                vec![Value::string("Process is not a pipe process")],
            ));
        }
        Value::make_process(stderr_id)
    } else {
        builtin_make_pipe_process_impl(
            processes,
            buffers,
            threads,
            coding_systems,
            vec![
                ProcessKeyword::Name.value(),
                Value::heap_string(name.concat(&LispString::from_unibyte(b" stderr".to_vec()))),
                ProcessKeyword::Buffer.value(),
                stderr_target,
                ProcessKeyword::Noquery.value(),
                Value::bool_val(noquery),
            ],
        )?
    };
    let id = processes.create_process_lisp_resolved(
        name,
        buffer.unwrap_or(Value::NIL),
        program,
        argv,
        executable,
    );
    processes.sync_process_mark(buffers, id)?;

    // GNU `make_process` (src/process.c) initialises every process's locking
    // thread to the creating thread (`pset_thread (p, Fcurrent_thread ())`),
    // so `process-thread` returns that thread rather than nil.  The network /
    // serial / pipe creators already do this; the subprocess path must too.
    if let Some(proc) = processes.get_mut(id) {
        proc.thread = current_thread_handle(threads);
    }

    // Set filter and sentinel if provided.
    if !filter.is_nil() {
        if let Some(proc) = processes.get_mut(id) {
            proc.filter = filter;
        }
    }
    if !sentinel.is_nil() {
        if let Some(proc) = processes.get_mut(id) {
            proc.sentinel = sentinel;
        }
    }
    if !stderrproc.is_nil() {
        if let Some(proc) = processes.get_mut(id) {
            proc.stderrproc = stderrproc;
        }
    }
    if let Some(proc) = processes.get_mut(id) {
        proc.default_directory = subprocess_cwd;
        if noquery {
            proc.query_on_exit_flag = false;
        }
    }

    // GNU `make-process` (src/process.c): when :coding is a non-nil value, it
    // supplies the DECODE coding (its car if it is a cons, else the whole
    // value) and the ENCODE coding (its cdr if a cons, else the whole value);
    // a single symbol is used for both directions. make-process does NOT run
    // these through `coding_inherit_eol_type` (unlike set-process-coding-system).
    // When :coding is absent/nil we keep the process default
    // (utf-8-unix . utf-8-unix), matching GNU's `default-process-coding-system`.
    if let Some(coding) = coding_val {
        if !coding.is_nil() {
            let (decode, encode) = if coding.is_cons() {
                (coding.cons_car(), coding.cons_cdr())
            } else {
                (coding, coding)
            };
            if let Some(proc) = processes.get_mut(id) {
                proc.coding_decode = decode;
                proc.coding_encode = encode;
            }
        }
    }

    // Spawn the actual OS child process.
    if let Err(e) = processes.spawn_child_with_environment(id, use_pty, process_environment) {
        return Err(signal(
            "file-missing",
            vec![Value::string("Searching for program"), Value::string(e)],
        ));
    }

    Ok(Value::make_process(id))
}

#[derive(Clone, Copy, Debug)]
struct AcceptProcessOutputRequest {
    wait: ProcessOutputWaitRequest,
    target_process: Option<ProcessId>,
    just_this_one: bool,
}

impl AcceptProcessOutputRequest {
    fn wait_timing_is_poll(self) -> bool {
        self.wait.timing().is_poll()
    }

    fn wait_timing_is_finite(self) -> bool {
        self.wait.timing().is_finite()
    }

    fn wait_timing_is_forever(self) -> bool {
        self.wait.timing().is_forever()
    }

    fn completes_on_any_process_activity(self) -> bool {
        self.target_process.is_none()
    }

    fn completes_on_target_process_activity(self, process: ProcessId) -> bool {
        self.target_process == Some(process)
    }

    fn services_only_target_process_output(self) -> bool {
        self.just_this_one
    }

    fn target_process_for_follow_up(self) -> Option<ProcessId> {
        self.target_process
    }
}

fn parse_accept_process_output_request(
    processes: &mut ProcessManager,
    args: &[Value],
) -> Result<Option<AcceptProcessOutputRequest>, Flow> {
    if args.len() > 4 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("accept-process-output"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    if let Some(process) = args.first() {
        if !process.is_nil()
            && resolve_live_process_designator_in_manager(processes, process).is_none()
        {
            if is_stale_process_id_designator_in_manager(processes, process) {
                return Ok(None);
            }
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("processp"), *process],
            ));
        }
    }

    if let Some(seconds) = args.get(1) {
        if let Some(milliseconds) = args.get(2) {
            if !milliseconds.is_nil() && !milliseconds.is_fixnum() {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("fixnump"), *milliseconds],
                ));
            }
            if milliseconds.is_nil() {
                if !seconds.is_nil() && !seconds.is_number() {
                    return Err(signal(
                        "wrong-type-argument",
                        vec![Value::symbol("numberp"), *seconds],
                    ));
                }
            } else if !seconds.is_nil() && !seconds.is_fixnum() {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("fixnump"), *seconds],
                ));
            }
        } else if !seconds.is_nil() && !seconds.is_number() {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("numberp"), *seconds],
            ));
        }
    }

    let target_id = if let Some(process) = args.first() {
        if !process.is_nil() {
            resolve_live_process_designator_in_manager(processes, process)
        } else {
            None
        }
    } else {
        None
    };

    let just_this_one = target_id.is_some() && args.get(3).is_some_and(|value| value.is_truthy());
    let allow_timers = if target_id.is_some() {
        !args.get(3).map_or(false, |v| v.is_fixnum())
    } else {
        true
    };
    let milliseconds_supplied = args.get(2).is_some_and(|value| !value.is_nil());
    let positive_timeout = accept_process_output_positive_timeout(args);
    let timing = if let Some(timeout) = positive_timeout {
        ProcessOutputWaitTiming::For(timeout)
    } else if target_id.is_some()
        && !milliseconds_supplied
        && args.get(1).map_or(true, |value| value.is_nil())
    {
        ProcessOutputWaitTiming::Forever
    } else {
        ProcessOutputWaitTiming::Poll
    };
    Ok(Some(AcceptProcessOutputRequest {
        wait: ProcessOutputWaitRequest::new(timing, target_id, just_this_one, allow_timers),
        target_process: target_id,
        just_this_one,
    }))
}

fn accept_process_output_positive_timeout(args: &[Value]) -> Option<Duration> {
    let total_seconds = if let Some(milliseconds) = args.get(2).filter(|value| !value.is_nil()) {
        let milliseconds = milliseconds.as_fixnum().unwrap_or(0) as f64 / 1000.0;
        let seconds = args
            .get(1)
            .filter(|value| !value.is_nil())
            .and_then(|value| value.as_fixnum())
            .unwrap_or(0) as f64;
        seconds + milliseconds
    } else if let Some(seconds) = args.get(1).filter(|value| !value.is_nil()) {
        seconds
            .as_fixnum()
            .map(|value| value as f64)
            .or_else(|| seconds.as_float())
            .unwrap_or(0.0)
    } else {
        return None;
    };

    (total_seconds > 0.0).then(|| Duration::from_secs_f64(total_seconds))
}

/// (process-send-string PROCESS STRING) -> nil
pub(crate) fn builtin_process_send_string(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() == 2 {
        if let Some(id) = process_value_to_id(&args[0]) {
            eval.wait_while_network_process_connecting(id)?;
        } else if let Ok(id) =
            resolve_process_or_missing_error_in_manager(&eval.processes, &args[0])
        {
            eval.wait_while_network_process_connecting(id)?;
        }
    }
    builtin_process_send_string_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_process_send_string_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-send-string", &args, 2)?;
    let input = args[1]
        .as_lisp_string()
        .cloned()
        .ok_or_else(|| signal_wrong_type_string(args[1]))?;
    if let Some(id) = process_value_to_id(&args[0]) {
        if is_stale_process_id_designator_in_manager(processes, &args[0]) {
            return Err(signal_process_not_running_in_manager(processes, id));
        }
    }
    let id = resolve_process_or_missing_error_in_manager(processes, &args[0])?;
    if processes
        .get(id)
        .is_some_and(|proc| !process_status_allows_send(&proc.status))
    {
        return Err(signal_process_not_running_in_manager(processes, id));
    }
    // GNU `send_process` (src/process.c) encodes the data through the process's
    // ENCODE coding system before writing it to the child's fd, applying both
    // character-code and EOL conversion.  Encode the input here so a process
    // whose output coding requests CRLF/CR (e.g. `dos`/`mac`/`utf-8-dos`) sends
    // the converted bytes; binary/raw-text encode systems pass the bytes
    // through unchanged.
    let encoded = encode_process_send_input(processes, id, &input);
    if !processes.send_input(id, &encoded)? {
        return Err(signal("error", vec![Value::string("Process not found")]));
    }
    Ok(Value::NIL)
}

/// (process-status PROCESS) -> symbol
pub(crate) fn builtin_process_status(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_status_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_process_status_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-status", &args, 1)?;
    let id = if let Some(id) = process_value_to_id(&args[0]) {
        if processes.get_any(id).is_some() {
            id
        } else {
            return Err(signal_wrong_type_processp(args[0]));
        }
    } else {
        match args[0].kind() {
            ValueKind::String => {
                let name = process_owned_runtime_string(args[0]);
                match processes.find_by_name(&name) {
                    Some(id) => id,
                    None => return Ok(Value::NIL),
                }
            }
            _ => return Err(signal_wrong_type_processp(args[0])),
        }
    };
    // Match GNU `Fprocess_status` (`src/process.c`): this reports the stored
    // process status and does not synchronously reap the child. Short-lived
    // subprocesses therefore remain `run` here until the wait request (for
    // example `accept-process-output`) observes the exit and updates status.
    match processes.get_any(id) {
        Some(proc) => Ok(process_public_status_symbol(proc)),
        None => Ok(Value::NIL),
    }
}

/// (process-exit-status PROCESS) -> integer
pub(crate) fn builtin_process_exit_status(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_exit_status_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_exit_status_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-exit-status", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes
        .get_any(id)
        .ok_or_else(|| signal_wrong_type_processp(args[0]))?;
    match ProcessStatusSymbol::from_status_value(proc.status) {
        Some(ProcessStatusSymbol::Exit) => {
            Ok(Value::fixnum(process_status_code_value(proc.status)))
        }
        Some(ProcessStatusSymbol::Failed) => {
            Ok(Value::fixnum(process_status_code_value(proc.status)))
        }
        Some(ProcessStatusSymbol::Signal) => {
            if proc.kind == ProcessKind::Real {
                Ok(Value::fixnum(process_status_code_value(proc.status)))
            } else {
                Ok(Value::fixnum(0))
            }
        }
        _ => Ok(Value::fixnum(0)),
    }
}

/// (process-list) -> list of process ids
pub(crate) fn builtin_process_list(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_list_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_list_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-list", &args, 0)?;
    let ids = processes.list_processes();
    let values: Vec<Value> = ids.iter().map(|id| Value::make_process(*id)).collect();
    Ok(Value::list(values))
}

/// (process-name PROCESS) -> string
pub(crate) fn builtin_process_name(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_name_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_name_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-name", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    match processes.get_any(id) {
        Some(proc) => Ok(proc.name),
        None => Err(signal_wrong_type_processp(args[0])),
    }
}

/// (process-buffer PROCESS) -> buffer or nil
pub(crate) fn builtin_process_buffer(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_buffer_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_buffer_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-buffer", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    match processes.get_any(id) {
        Some(proc) => Ok(proc.buffer),
        None => Err(signal_wrong_type_processp(args[0])),
    }
}

/// (process-coding-system PROCESS) -> (decode . encode)
pub(crate) fn builtin_process_coding_system(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_coding_system_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_coding_system_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-coding-system", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(Value::cons(proc.coding_decode, proc.coding_encode))
}

/// (process-datagram-address PROCESS) -> address-or-nil
pub(crate) fn builtin_process_datagram_address(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() == 1
        && let Some(id) = pending_network_connect_id(&eval.processes, args[0])?
    {
        eval.wait_while_network_process_connecting(id)?;
    }
    builtin_process_datagram_address_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_datagram_address_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-datagram-address", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let Some(proc) = processes.get_any(id) else {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        ));
    };
    let is_datagram = matches!(
        proc.network_socket.as_ref(),
        Some(NetworkSocket::UdpSocket(_))
    );
    #[cfg(unix)]
    let is_datagram = is_datagram
        || matches!(
            proc.network_socket.as_ref(),
            Some(NetworkSocket::UnixDatagram(_))
        );
    if is_datagram {
        Ok(proc.datagram_address)
    } else {
        Ok(Value::NIL)
    }
}

/// (process-inherit-coding-system-flag PROCESS) -> bool
pub(crate) fn builtin_process_inherit_coding_system_flag(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_inherit_coding_system_flag_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_inherit_coding_system_flag_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-inherit-coding-system-flag", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(Value::bool_val(proc.inherit_coding_system_flag))
}

/// (set-process-buffer PROCESS BUFFER) -> BUFFER
pub(crate) fn builtin_set_process_buffer(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_buffer_impl(&mut eval.processes, &mut eval.buffers, args)
}

pub(crate) fn builtin_set_process_buffer_impl(
    processes: &mut ProcessManager,
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-buffer", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    match args[1].kind() {
        ValueKind::Nil => {}
        ValueKind::Veclike(VecLikeType::Buffer) => {
            let bid = args[1].as_buffer_id().unwrap();
            let _ = buffers
                .get(bid)
                .ok_or_else(|| signal("error", vec![Value::string("Selecting deleted buffer")]))?;
        }
        _ => return Err(signal_wrong_type_bufferp(args[1])),
    };
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    if proc.buffer != args[1] {
        proc.buffer = args[1];
        update_process_mark(buffers, proc)?;
    }
    if process_uses_contact_plist(proc) {
        proc.childp =
            process_contact_plist_put(proc.childp, ProcessKeyword::Buffer.value(), args[1])?;
    }
    Ok(args[1])
}

/// (set-process-coding-system PROCESS &optional DECODING ENCODING) -> nil
pub(crate) fn builtin_set_process_coding_system(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_coding_system_impl(&mut eval.processes, &eval.coding_systems, args)
}

pub(crate) fn builtin_set_process_coding_system_impl(
    processes: &mut ProcessManager,
    coding_systems: &super::coding::CodingSystemManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("set-process-coding-system", &args, 1)?;
    if args.len() > 3 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("set-process-coding-system"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    // GNU `Fset_process_coding_system` (src/process.c): CHECK_PROCESS first,
    // then DECODING and ENCODING (both defaulting to nil) are validated, then
    // ENCODING (only) is passed through `coding_inherit_eol_type` so a
    // nil/undecided-EOL encode coding normalizes (e.g. nil -> raw-text-unix,
    // utf-8 -> utf-8-unix). DECODING is stored as-is (nil stays nil).
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let decoding = args.get(1).cloned().unwrap_or(Value::NIL);
    let encoding = args.get(2).cloned().unwrap_or(Value::NIL);
    super::coding::builtin_check_coding_system(coding_systems, vec![decoding])?;
    super::coding::builtin_check_coding_system(coding_systems, vec![encoding])?;
    let encoding = super::coding::coding_inherit_eol_type_unix(coding_systems, encoding);

    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.coding_decode = decoding;
    proc.coding_encode = encoding;
    Ok(Value::NIL)
}

/// (set-buffer-process-coding-system DECODING ENCODING) -> nil
pub(crate) fn builtin_set_buffer_process_coding_system(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-buffer-process-coding-system", &args, 2)?;
    let id = resolve_optional_process_or_current_buffer(eval, None)?;
    let proc = eval.processes.get_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), Value::make_process(id)],
        )
    })?;
    proc.coding_decode = args[0];
    proc.coding_encode = args[1];
    Ok(Value::NIL)
}

/// (set-process-datagram-address PROCESS ADDRESS) -> nil
pub(crate) fn builtin_set_process_datagram_address(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() == 2
        && let Some(id) = pending_network_connect_id(&eval.processes, args[0])?
    {
        eval.wait_while_network_process_connecting(id)?;
    }
    builtin_set_process_datagram_address_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_datagram_address_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-datagram-address", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let Some(proc) = processes.get_any_mut(id) else {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        ));
    };
    match proc.network_socket.as_ref() {
        Some(NetworkSocket::UdpSocket(_)) => {
            let Ok(NetworkAddressSpec::Inet(addr)) = parse_network_address_spec(&args[1]) else {
                return Ok(Value::NIL);
            };
            proc.datagram_socket_addr = Some(addr);
            proc.datagram_address = args[1];
            Ok(args[1])
        }
        #[cfg(unix)]
        Some(NetworkSocket::UnixDatagram(_)) => {
            let Ok(NetworkAddressSpec::Local(path)) = parse_network_address_spec(&args[1]) else {
                return Ok(Value::NIL);
            };
            proc.datagram_unix_path = Some(path);
            proc.datagram_address = args[1];
            Ok(args[1])
        }
        _ => Ok(Value::NIL),
    }
}

/// (set-process-inherit-coding-system-flag PROCESS FLAG) -> FLAG
pub(crate) fn builtin_set_process_inherit_coding_system_flag(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_inherit_coding_system_flag_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_inherit_coding_system_flag_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-inherit-coding-system-flag", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.inherit_coding_system_flag = args[1].is_truthy();
    Ok(args[1])
}

/// (set-process-thread PROCESS THREAD) -> thread-or-nil
pub(crate) fn builtin_set_process_thread(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_thread_impl(&mut eval.processes, &eval.threads, args)
}

pub(crate) fn builtin_set_process_thread_impl(
    processes: &mut ProcessManager,
    threads: &ThreadManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-thread", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let value = if args[1].is_nil() {
        Value::NIL
    } else if threads.thread_id_from_handle(&args[1]).is_some() {
        args[1]
    } else {
        return Err(signal_wrong_type_threadp(args[1]));
    };
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.thread = value;
    Ok(value)
}

/// (set-process-window-size PROCESS COLS ROWS) -> t
pub(crate) fn builtin_set_process_window_size(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_window_size_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_window_size_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-window-size", &args, 3)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let cols = expect_integer(&args[1])?;
    let rows = expect_integer(&args[2])?;
    let is_live = processes.get(id).is_some();
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.window_cols = Some(cols);
    proc.window_rows = Some(rows);
    // If the process has a PTY master, resize it.
    if let Some(ref pty_master) = proc.pty_master {
        let pty_size = portable_pty::PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        };
        let _ = pty_master.resize(pty_size);
    }
    Ok(if is_live { Value::T } else { Value::NIL })
}

/// (process-kill-buffer-query-function) -> bool
pub(crate) fn builtin_process_kill_buffer_query_function(args: Vec<Value>) -> EvalResult {
    expect_args("process-kill-buffer-query-function", &args, 0)?;
    Ok(Value::T)
}

/// (process-menu-delete-process) -> nil
pub(crate) fn builtin_process_menu_delete_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-menu-delete-process", &args, 0)?;
    let current_buffer_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    if eval
        .processes
        .find_by_buffer_id(current_buffer_id)
        .is_some()
    {
        return Err(signal(
            "error",
            vec![Value::string(
                "Buffer does not seem to be associated with any file",
            )],
        ));
    }
    let _ = resolve_optional_process_or_current_buffer(eval, None)?;
    Ok(Value::NIL)
}

/// (process-menu-visit-buffer LINE) -> nil
pub(crate) fn builtin_process_menu_visit_buffer(
    _eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-menu-visit-buffer", &args, 1)?;
    let _line = expect_int_or_marker(&args[0])?;
    Err(signal(
        "wrong-type-argument",
        vec![Value::symbol("stringp"), Value::NIL],
    ))
}

/// (process-tty-name PROCESS &optional STREAM) -> string-or-nil
pub(crate) fn builtin_process_tty_name(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_tty_name_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_tty_name_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-tty-name", &args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("process-tty-name"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    let stream = args.get(1).cloned().unwrap_or(Value::NIL);
    let tty_value = || proc.tty_name;

    match ProcessTtyStream::from_value(&stream) {
        None if stream.is_nil() => Ok(tty_value()),
        Some(ProcessTtyStream::Stdin) => {
            if proc.tty_stdin {
                Ok(tty_value())
            } else {
                Ok(Value::NIL)
            }
        }
        Some(ProcessTtyStream::Stdout) => {
            if proc.tty_stdout {
                Ok(tty_value())
            } else {
                Ok(Value::NIL)
            }
        }
        Some(ProcessTtyStream::Stderr) => {
            if proc.tty_stderr && proc.stderrproc.is_nil() {
                Ok(tty_value())
            } else {
                Ok(Value::NIL)
            }
        }
        None => Err(signal(
            "error",
            vec![Value::string("Unknown stream"), stream],
        )),
    }
}

/// (process-mark PROCESS) -> marker
pub(crate) fn builtin_process_mark(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_mark_impl(&eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_process_mark_impl(
    processes: &ProcessManager,
    _buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-mark", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.mark)
}

/// (process-type PROCESS) -> symbol
pub(crate) fn builtin_process_type(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_type_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_type_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-type", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.proc_type)
}

/// (process-thread PROCESS) -> object-or-nil
pub(crate) fn builtin_process_thread(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_thread_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_thread_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-thread", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.thread)
}

/// (process-send-region PROCESS START END) -> nil
pub(crate) fn builtin_process_send_region(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() == 3 {
        if let Some(id) = process_value_to_id(&args[0]) {
            eval.wait_while_network_process_connecting(id)?;
        } else if let Ok(id) =
            resolve_process_or_missing_error_in_manager(&eval.processes, &args[0])
        {
            eval.wait_while_network_process_connecting(id)?;
        }
    }
    builtin_process_send_region_impl(&mut eval.processes, &mut eval.buffers, args)
}

pub(crate) fn builtin_process_send_region_impl(
    processes: &mut ProcessManager,
    buffers: &mut BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-send-region", &args, 3)?;

    if let Some(id) = process_value_to_id(&args[0]) {
        if is_stale_process_id_designator_in_manager(processes, &args[0]) {
            let _ = super::position::LispRegionArgs::from_values(&*buffers, args[1], args[2])?;
            return Err(signal_process_not_running_in_manager(processes, id));
        }
    }

    let id =
        resolve_optional_process_or_current_buffer_in_state(processes, buffers, Some(&args[0]))?;
    if processes
        .get(id)
        .is_some_and(|proc| !process_status_allows_send(&proc.status))
    {
        return Err(signal_process_not_running_in_manager(processes, id));
    }
    let region_args = super::position::LispRegionArgs::from_values(&*buffers, args[1], args[2])?;

    let region_text = {
        let buf = buffers
            .current_buffer()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let region = checked_region_bytes(buf, region_args)?;
        buf.buffer_substring_lisp_string_range(region)
    };

    // Encode the region text through the process's ENCODE coding system, exactly
    // like `process-send-string` (GNU `send_process`).
    let encoded = encode_process_send_input(processes, id, &region_text);
    if !processes.send_input(id, &encoded)? {
        return Err(signal("error", vec![Value::string("Process not found")]));
    }
    Ok(Value::NIL)
}

/// (process-send-eof &optional PROCESS) -> process-or-nil
pub(crate) fn builtin_process_send_eof(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() <= 1 {
        let maybe_id = match args.first() {
            Some(process) if !process.is_nil() => {
                resolve_process_or_missing_error_in_manager(&eval.processes, process).ok()
            }
            _ => resolve_optional_process_or_current_buffer_in_state(
                &eval.processes,
                &eval.buffers,
                args.first(),
            )
            .ok(),
        };
        if let Some(id) = maybe_id
            && eval.processes.get(id).is_some_and(|proc| {
                proc.kind == ProcessKind::Network && proc.pending_network_connect.is_some()
            })
        {
            eval.wait_while_network_process_connecting(id)?;
        }
    }
    builtin_process_send_eof_impl(&mut eval.processes, &eval.buffers, args)
}

fn send_eof_to_process(proc: &mut Process) -> EvalResult {
    if let Some(tls) = proc.tls_stream.as_mut() {
        tls.send_close_notify(false)
            .map(|_| ())
            .map_err(|err| signal_process_io("Sending EOF to process", None, err))?;
        return Ok(Value::NIL);
    }

    if let Some(socket) = proc.network_socket.as_ref() {
        if let Some(result) = socket.shutdown_write() {
            result.map_err(|err| signal_process_io("Sending EOF to process", None, err))?;
        }
        return Ok(Value::NIL);
    }

    if let Some(ref mut child) = proc.child {
        drop(child.stdin.take());
    }
    Ok(Value::NIL)
}

pub(crate) fn builtin_process_send_eof_impl(
    processes: &mut ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 1 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("process-send-eof"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    if let Some(process) = args.first() {
        if !process.is_nil() {
            if let Some(id) = process_value_to_id(process) {
                if is_stale_process_id_designator_in_manager(processes, process) {
                    return Err(signal_process_not_running_in_manager(processes, id));
                }
            }
            let id = resolve_process_or_missing_error_in_manager(processes, process)?;
            if let Some(proc) = processes.get_mut(id) {
                send_eof_to_process(proc)?;
            }
            return Ok(*process);
        }
    }
    let id = resolve_optional_process_or_current_buffer_in_state(processes, buffers, args.first())?;
    if let Some(proc) = processes.get_mut(id) {
        send_eof_to_process(proc)?;
    }
    Ok(Value::NIL)
}

/// (process-running-child-p &optional PROCESS) -> bool
pub(crate) fn builtin_process_running_child_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_running_child_p_impl(&eval.processes, &eval.buffers, args)
}

pub(crate) fn builtin_process_running_child_p_impl(
    processes: &ProcessManager,
    buffers: &BufferManager,
    args: Vec<Value>,
) -> EvalResult {
    if args.len() > 1 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("process-running-child-p"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    if let Some(process) = args.first() {
        if let Some(id) = process_value_to_id(process) {
            if is_stale_process_id_designator_in_manager(processes, process) {
                return Err(signal_process_not_active_in_manager(processes, id));
            }
        }
    }
    let _id =
        resolve_optional_process_or_current_buffer_in_state(processes, buffers, args.first())?;
    Ok(Value::NIL)
}

/// (accept-process-output &optional PROCESS SECONDS MILLISECS JUST-THIS-ONE) -> bool
pub(crate) fn builtin_accept_process_output(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let Some(request) = parse_accept_process_output_request(&mut eval.processes, &args)? else {
        return Ok(Value::NIL);
    };

    match eval.wait_for_process_output(request.wait)? {
        ProcessOutputWaitOutcome::ProcessActivity => {
            accept_process_output_run_target_follow_up(eval, request)?;
            Ok(Value::T)
        }
        ProcessOutputWaitOutcome::NoProcessActivity => Ok(Value::NIL),
    }
}

fn accept_process_output_run_target_follow_up(
    eval: &mut super::eval::Context,
    request: AcceptProcessOutputRequest,
) -> Result<(), Flow> {
    let Some(target_id) = request.target_process_for_follow_up() else {
        return Ok(());
    };

    // GNU's wait_reading_process_output keeps a target-process
    // accept-process-output call alive for a minimum follow-up cycle after
    // reading bytes, so a child that exits immediately after flushing output
    // can run its sentinel before we return.
    let mut idle_follow_up_polls = 0usize;
    loop {
        let events = eval.processes.wait_for_process_events(Duration::ZERO);
        let target_activity = if events.has_ready_processes() {
            eval.service_process_output_wait_source_events_have_target_process_activity(
                request.wait,
                events,
            )?
        } else {
            eval.service_process_output_wait_once_has_target_process_activity(request.wait)?
        };
        if target_activity {
            idle_follow_up_polls = 0;
            continue;
        }

        let target_still_running = eval
            .processes
            .get(target_id)
            .is_some_and(process_has_readable_process_io);
        if !target_still_running {
            break;
        }

        idle_follow_up_polls += 1;
        if idle_follow_up_polls >= 4 {
            break;
        }
        std::thread::yield_now();
    }

    Ok(())
}

/// (get-process NAME) -> process-or-nil
pub(crate) fn builtin_get_process(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_get_process_impl(&eval.processes, args)
}

pub(crate) fn builtin_get_process_impl(processes: &ProcessManager, args: Vec<Value>) -> EvalResult {
    expect_args("get-process", &args, 1)?;
    // GNU `Fget_process`: a process object is returned unchanged; otherwise the
    // argument must be a name string.
    if args[0].is_process() {
        return Ok(args[0]);
    }
    let name = expect_string_strict(&args[0])?;
    match processes.find_by_name(&name) {
        Some(id) => Ok(Value::make_process(id)),
        None => Ok(Value::NIL),
    }
}

/// (get-buffer-process BUFFER-OR-NAME) -> process-or-nil
pub(crate) fn builtin_get_buffer_process(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_get_buffer_process_impl(&eval.frames, &eval.buffers, &eval.processes, args)
}

pub(crate) fn builtin_get_buffer_process_impl(
    frames: &FrameManager,
    buffers: &BufferManager,
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("get-buffer-process", &args, 1)?;
    let Some(buffer_id) = resolve_buffer_for_process_lookup_in_state(frames, buffers, &args[0])?
    else {
        return Ok(Value::NIL);
    };
    match processes.find_by_buffer_id(buffer_id) {
        Some(id) => Ok(Value::make_process(id)),
        None => Ok(Value::NIL),
    }
}

/// (processp OBJECT) -> bool
pub(crate) fn builtin_processp(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_processp_impl(&eval.processes, args)
}

pub(crate) fn builtin_processp_impl(_processes: &ProcessManager, args: Vec<Value>) -> EvalResult {
    expect_args("processp", &args, 1)?;
    // GNU `Fprocessp` is purely structural: any process object is `t`, even
    // after it has exited (it stays a process object).  A bare integer is not a
    // process.
    Ok(Value::bool_val(args[0].is_process()))
}

/// (process-live-p PROCESS) -> list-or-nil
pub(crate) fn builtin_process_live_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_live_p_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_live_p_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-live-p", &args, 1)?;
    let Some(id) = process_value_to_id(&args[0]).and_then(|id| processes.get(id).map(|_| id))
    else {
        return Ok(Value::NIL);
    };
    let proc = processes.get(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(process_live_status_value(proc))
}

/// (process-id PROCESS) -> integer
pub(crate) fn builtin_process_id(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_process_id_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_id_impl(processes: &ProcessManager, args: Vec<Value>) -> EvalResult {
    expect_args("process-id", &args, 1)?;
    // GNU `Fprocess_id` uses CHECK_PROCESS — it requires a genuine process
    // object (no name-string designator), so resolve structurally only.
    let id = process_value_to_id(&args[0])
        .filter(|id| processes.get_any(*id).is_some())
        .ok_or_else(|| signal_wrong_type_processp(args[0]))?;
    let proc = processes
        .get_any(id)
        .ok_or_else(|| signal_wrong_type_processp(args[0]))?;
    // GNU `Fprocess_id` returns the child's real OS pid as an integer
    // (`XPROCESS (process)->pid`), or nil when there is none (pid == 0), as
    // for network/serial/pipe connections.  The internal `ProcessId` used to
    // key the manager is kept separate and never exposed here.
    match proc.os_pid {
        Some(pid) => Ok(Value::fixnum(i64::from(pid))),
        None => Ok(Value::NIL),
    }
}

/// (process-query-on-exit-flag PROCESS) -> bool
pub(crate) fn builtin_process_query_on_exit_flag(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_query_on_exit_flag_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_query_on_exit_flag_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-query-on-exit-flag", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(Value::bool_val(proc.query_on_exit_flag))
}

/// (set-process-query-on-exit-flag PROCESS FLAG) -> FLAG
pub(crate) fn builtin_set_process_query_on_exit_flag(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_query_on_exit_flag_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_query_on_exit_flag_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-query-on-exit-flag", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let flag = args[1].is_truthy();
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.query_on_exit_flag = flag;
    Ok(args[1])
}

/// (process-command PROCESS) -> list
pub(crate) fn builtin_process_command(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_command_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_command_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-command", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.command)
}

/// (process-contact PROCESS &optional KEY NO-BLOCK) -> value
pub(crate) fn builtin_process_contact(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if (1..=3).contains(&args.len()) {
        let no_block = args.get(2).is_some_and(|value| value.is_truthy());
        if let Some(id) = pending_network_connect_id(&eval.processes, args[0])? {
            if no_block {
                return Ok(Value::NIL);
            }
            eval.wait_while_network_process_connecting(id)?;
        }
    }
    builtin_process_contact_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_contact_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("process-contact", &args, 1)?;
    if args.len() > 3 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("process-contact"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    let key = args.get(1).copied().unwrap_or(Value::NIL);
    let contact = proc.childp;
    match proc.proc_type.as_symbol_name() {
        Some("network") => {
            if key == Value::T {
                Ok(contact)
            } else if key.is_nil() {
                Ok(Value::list(vec![
                    process_contact_plist_get(contact, ProcessKeyword::Host.value()),
                    process_contact_plist_get(contact, ProcessKeyword::Service.value()),
                ]))
            } else {
                Ok(process_contact_plist_get(contact, key))
            }
        }
        Some("serial") => {
            if key == Value::T {
                Ok(contact)
            } else if key.is_nil() {
                Ok(Value::list(vec![
                    process_contact_plist_get(contact, ProcessKeyword::Port.value()),
                    process_contact_plist_get(contact, ProcessKeyword::Speed.value()),
                ]))
            } else {
                Ok(process_contact_plist_get(contact, key))
            }
        }
        Some("pipe") => {
            if key == Value::T {
                Ok(contact)
            } else if key.is_nil() {
                Ok(Value::T)
            } else {
                Ok(process_contact_plist_get(contact, key))
            }
        }
        _ => Ok(contact),
    }
}

/// (process-filter PROCESS) -> function
pub(crate) fn builtin_process_filter(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_filter_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_filter_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-filter", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.filter)
}

/// (set-process-filter PROCESS FILTER) -> FILTER
pub(crate) fn builtin_set_process_filter(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_filter_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_filter_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-filter", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let stored = if args[1].is_nil() {
        Value::symbol(DEFAULT_PROCESS_FILTER_SYMBOL)
    } else {
        args[1]
    };
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.filter = stored;
    if process_uses_contact_plist(proc) {
        proc.childp =
            process_contact_plist_put(proc.childp, ProcessKeyword::Filter.value(), stored)?;
    }
    Ok(stored)
}

/// (process-sentinel PROCESS) -> function
pub(crate) fn builtin_process_sentinel(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_sentinel_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_sentinel_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-sentinel", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.sentinel)
}

/// (set-process-sentinel PROCESS SENTINEL) -> SENTINEL
pub(crate) fn builtin_set_process_sentinel(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_sentinel_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_sentinel_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-sentinel", &args, 2)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let stored = if args[1].is_nil() {
        Value::symbol(DEFAULT_PROCESS_SENTINEL_SYMBOL)
    } else {
        args[1]
    };
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.sentinel = stored;
    if process_uses_contact_plist(proc) {
        proc.childp =
            process_contact_plist_put(proc.childp, ProcessKeyword::Sentinel.value(), stored)?;
    }
    Ok(stored)
}

/// (process-plist PROCESS) -> plist
pub(crate) fn builtin_process_plist(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_process_plist_impl(&eval.processes, args)
}

pub(crate) fn builtin_process_plist_impl(
    processes: &ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("process-plist", &args, 1)?;
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    Ok(proc.plist)
}

/// (set-process-plist PROCESS PLIST) -> plist
pub(crate) fn builtin_set_process_plist(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    builtin_set_process_plist_impl(&mut eval.processes, args)
}

pub(crate) fn builtin_set_process_plist_impl(
    processes: &mut ProcessManager,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-process-plist", &args, 2)?;
    if !args[1].is_list() {
        return Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("listp"), args[1]],
        ));
    }
    let id = resolve_process_or_wrong_type_any_in_manager(processes, &args[0])?;
    let proc = processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.plist = args[1];
    Ok(proc.plist)
}

/// (process-put PROCESS PROP VALUE) -> plist
pub(crate) fn builtin_process_put(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("process-put", &args, 3)?;
    let id = resolve_process_or_wrong_type_any(eval, &args[0])?;
    let current_plist = eval
        .processes
        .get_any(id)
        .ok_or_else(|| {
            signal(
                "wrong-type-argument",
                vec![Value::symbol("processp"), args[0]],
            )
        })?
        .plist;
    let new_plist = super::builtins::builtin_plist_put(vec![current_plist, args[1], args[2]])?;
    let proc = eval.processes.get_any_mut(id).ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("processp"), args[0]],
        )
    })?;
    proc.plist = new_plist;
    Ok(new_plist)
}

/// (process-get PROCESS PROP) -> value
pub(crate) fn builtin_process_get(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_args("process-get", &args, 2)?;
    let id = resolve_process_or_wrong_type_any(eval, &args[0])?;
    let plist = eval
        .processes
        .get_any(id)
        .ok_or_else(|| {
            signal(
                "wrong-type-argument",
                vec![Value::symbol("processp"), args[0]],
            )
        })?
        .plist;
    super::builtins::builtin_plist_get(vec![plist, args[1]])
}

// ---------------------------------------------------------------------------
// Builtins (pure — no evaluator needed)
// ---------------------------------------------------------------------------

/// (shell-command-to-string COMMAND) -> string
///
/// Runs COMMAND via the system shell and returns captured stdout.
pub(crate) fn builtin_shell_command_to_string(args: Vec<Value>) -> EvalResult {
    expect_args("shell-command-to-string", &args, 1)?;
    let command = lisp_string_to_os_string(super::builtins::expect_lisp_string(&args[0])?);

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let output = crate::emacs_core::callproc::new_child_command(&shell)
        .arg("-c")
        .arg(&command)
        .output()
        .map_err(|e| signal_process_io("Shell command failed", Some(&shell), e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(Value::string(stdout))
}

fn getenv_impl(name: &str, args: &[Value]) -> EvalResult {
    expect_min_args(name, args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ));
    }
    if let Some(frame) = args.get(1) {
        if !frame.is_nil() {
            return Err(signal(
                "wrong-type-argument",
                vec![Value::symbol("framep"), *frame],
            ));
        }
    }
    let name = super::builtins::expect_lisp_string(&args[0])?;
    match std::env::var_os(lisp_string_to_os_string(name)) {
        Some(val) => Ok(Value::heap_string(os_str_to_lisp_string(val.as_os_str()))),
        None => Ok(Value::NIL),
    }
}

/// (getenv VARIABLE) -> string or nil
pub(crate) fn builtin_getenv(args: Vec<Value>) -> EvalResult {
    getenv_impl("getenv", &args)
}

/// (getenv-internal VARIABLE &optional ENV) -> string or nil
///
/// GNU-compatible: checks process-environment first, then falls back
/// to the real OS environment (matching callproc.c:getenv_internal).
/// When ENV is a list, searches that list instead.
pub(crate) fn builtin_getenv_internal(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("getenv-internal", &args, 1)?;
    if args.len() > 2 {
        return Err(signal(
            "wrong-number-of-arguments",
            vec![
                Value::symbol("getenv-internal"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }
    let varname = super::builtins::expect_lisp_string(&args[0])?;

    // If ENV arg is a list, search it directly (GNU behavior).
    if let Some(env_list) = args.get(1) {
        if env_list.is_cons() {
            return Ok(match getenv_from_list(varname, *env_list) {
                EnvLookup::Value(value) => value,
                EnvLookup::Negative => Value::T,
                EnvLookup::Missing => Value::NIL,
            });
        }
    }

    // Check process-environment first (GNU callproc.c:1720 getenv_internal,
    // which consults Vprocess_environment — i.e. the dynamic binding, so
    // honor let-bindings rather than only the global value).
    let proc_env = eval.visible_variable_value_or_nil("process-environment");
    if proc_env.is_cons() {
        match getenv_from_list(varname, proc_env) {
            EnvLookup::Value(value) => return Ok(value),
            EnvLookup::Negative => return Ok(Value::NIL),
            EnvLookup::Missing => {}
        }
    }

    // Fall back to real OS environment.
    match std::env::var_os(lisp_string_to_os_string(varname)) {
        Some(val) => Ok(Value::heap_string(os_str_to_lisp_string(val.as_os_str()))),
        None => Ok(Value::NIL),
    }
}

enum EnvLookup {
    Value(Value),
    Negative,
    Missing,
}

/// Search a process-environment-style list for VARIABLE, matching GNU
/// `getenv_internal_1`: scan cons cells, ignore non-string entries, and treat
/// a bare "VARIABLE" string as an explicit negative entry.
fn getenv_from_list(varname: &LispString, env_list: Value) -> EnvLookup {
    let var_bytes = varname.as_bytes();
    let mut env = env_list;
    while env.is_cons() {
        let entry = env.cons_car();
        if let Some(s) = entry.as_lisp_string() {
            let bytes = s.as_bytes();
            if bytes.len() >= var_bytes.len()
                && env_var_name_bytes_eq(&bytes[..var_bytes.len()], var_bytes)
            {
                if bytes.len() > var_bytes.len() && bytes[var_bytes.len()] == b'=' {
                    return EnvLookup::Value(Value::heap_string(lisp_string_from_bytes(
                        &bytes[var_bytes.len() + 1..],
                        s.is_multibyte(),
                    )));
                }
                if bytes.len() == var_bytes.len() {
                    return EnvLookup::Negative;
                }
            }
        }
        env = env.cons_cdr();
    }
    EnvLookup::Missing
}

pub(crate) fn make_network_process_subfeatures() -> Value {
    // Advertise only behavior that this runtime actually implements.  GNU's
    // surface is still broader (full inet `:type seqpacket`), but packages use
    // `featurep' to choose code paths.  Keep this list tied to backed behavior,
    // not parser acceptance.
    Value::list(vec![
        Value::keyword("nodelay"),
        Value::keyword("reuseaddr"),
        Value::keyword("priority"),
        Value::keyword("oobinline"),
        Value::keyword("linger"),
        Value::keyword("keepalive"),
        Value::keyword("dontroute"),
        Value::keyword("broadcast"),
        Value::keyword("bindtodevice"),
        Value::list(vec![Value::keyword("family"), Value::symbol("local")]),
        Value::list(vec![Value::keyword("family"), Value::symbol("ipv4")]),
        Value::list(vec![Value::keyword("family"), Value::symbol("ipv6")]),
        Value::list(vec![Value::keyword("service"), Value::T]),
        Value::list(vec![Value::keyword("server"), Value::T]),
        Value::list(vec![Value::keyword("nowait"), Value::T]),
        Value::list(vec![Value::keyword("type"), Value::symbol("datagram")]),
    ])
}

/// (set-binary-mode STREAM MODE) -> t
///
/// Batch/runtime compatibility path. Accepts stdin/stdout/stderr symbols.
pub(crate) fn builtin_set_binary_mode(args: Vec<Value>) -> EvalResult {
    expect_args("set-binary-mode", &args, 2)?;
    let stream = args[0].as_symbol_name().ok_or_else(|| {
        signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), args[0]],
        )
    })?;

    match stream {
        "stdin" | "stdout" | "stderr" => Ok(Value::T),
        _ => Err(signal(
            "error",
            vec![Value::string("unsupported stream"), args[0]],
        )),
    }
}

impl GcTrace for ProcessManager {
    fn trace_roots(&self, roots: &mut Vec<Value>) {
        for process in self
            .processes
            .values()
            .chain(self.deleted_processes.values())
        {
            roots.push(process.name);
            roots.push(process.proc_type);
            roots.push(process.buffer);
            roots.push(process.mark);
            roots.push(process.command);
            roots.push(process.childp);
            roots.push(process.status);
            roots.push(process.tty_name);
            roots.push(process.write_queue);
            roots.push(process.filter);
            roots.push(process.sentinel);
            roots.push(process.log);
            roots.push(process.plist);
            roots.push(process.stderrproc);
            roots.push(process.datagram_address);
            roots.push(process.coding_decode);
            roots.push(process.coding_encode);
            roots.push(process.thread);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "process_raw_bytes_test.rs"]
mod raw_bytes_tests;
#[cfg(test)]
#[path = "process_test.rs"]
mod tests;
