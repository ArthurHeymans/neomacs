//! GUI visual test harness for Neomacs.
//!
//! The crate keeps GUI checks text-first for automation: each scenario has an
//! explicit display backend and writes stable JSON/PNG/log artifacts under
//! `target/neomacs-gui-tests`.

use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiBackend {
    LinuxX11,
    LinuxWayland,
    Macos,
    Windows,
}

impl GuiBackend {
    pub fn runner_kind(self) -> RunnerKind {
        match self {
            Self::LinuxX11 => RunnerKind::Xvfb,
            Self::LinuxWayland => RunnerKind::WestonHeadless,
            Self::Macos | Self::Windows => RunnerKind::CurrentDesktopSession,
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::LinuxX11 => "linux-x11",
            Self::LinuxWayland => "linux-wayland",
            Self::Macos => "macos",
            Self::Windows => "windows",
        }
    }

    fn winit_unix_backend(self) -> Option<&'static str> {
        match self {
            Self::LinuxX11 => Some("x11"),
            Self::LinuxWayland => Some("wayland"),
            Self::Macos | Self::Windows => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerKind {
    Xvfb,
    WestonHeadless,
    CurrentDesktopSession,
}

impl RunnerKind {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Xvfb => "xvfb",
            Self::WestonHeadless => "weston-headless",
            Self::CurrentDesktopSession => "current-desktop-session",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayHarness {
    backend: GuiBackend,
}

impl DisplayHarness {
    pub fn for_backend(backend: GuiBackend) -> Self {
        Self { backend }
    }

    pub fn required_env(&self) -> &'static [&'static str] {
        match self.backend {
            GuiBackend::LinuxX11 => &["DISPLAY"],
            GuiBackend::LinuxWayland => &["XDG_RUNTIME_DIR", "WAYLAND_DISPLAY"],
            GuiBackend::Macos | GuiBackend::Windows => &[],
        }
    }

    pub fn start_session(&self, artifact_root: impl AsRef<Path>) -> io::Result<DisplaySession> {
        match self.backend {
            GuiBackend::LinuxWayland => start_weston_headless(artifact_root.as_ref()),
            GuiBackend::LinuxX11 => start_xvfb(),
            GuiBackend::Macos | GuiBackend::Windows => Ok(DisplaySession {
                child: None,
                env: Vec::new(),
                cleanup_dir: None,
            }),
        }
    }
}

#[derive(Debug)]
pub struct DisplaySession {
    child: Option<Child>,
    env: Vec<(String, String)>,
    cleanup_dir: Option<PathBuf>,
}

impl DisplaySession {
    pub fn env(&self) -> &[(String, String)] {
        &self.env
    }
}

impl Drop for DisplaySession {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(path) = self.cleanup_dir.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiArtifactSet {
    pub json: PathBuf,
    pub png: PathBuf,
    pub stderr: PathBuf,
    pub gui_state: PathBuf,
}

impl GuiArtifactSet {
    pub fn new(root: impl Into<PathBuf>, backend: GuiBackend, scenario_name: &str) -> Self {
        let dir = root.into().join(backend.slug());
        Self {
            json: dir.join(format!("{scenario_name}.json")),
            png: dir.join(format!("{scenario_name}.png")),
            stderr: dir.join(format!("{scenario_name}.stderr.log")),
            gui_state: dir.join(format!("{scenario_name}.gui-state.json")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiScenario {
    pub name: String,
    pub script: PathBuf,
}

impl GuiScenario {
    pub fn new(name: impl Into<String>, script: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            script: script.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiTestPlan {
    backend: GuiBackend,
    workspace_root: PathBuf,
    artifact_root: PathBuf,
    scenario: GuiScenario,
    program: Option<PathBuf>,
    extra_env: Vec<(String, String)>,
}

impl GuiTestPlan {
    pub fn new(
        backend: GuiBackend,
        workspace_root: impl Into<PathBuf>,
        artifact_root: impl Into<PathBuf>,
        scenario: GuiScenario,
    ) -> Self {
        Self {
            backend,
            workspace_root: workspace_root.into(),
            artifact_root: artifact_root.into(),
            scenario,
            program: None,
            extra_env: Vec::new(),
        }
    }

    pub fn with_program(mut self, program: impl Into<PathBuf>) -> Self {
        self.program = Some(program.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_env.push((key.into(), value.into()));
        self
    }

    pub fn command_spec(&self) -> CommandSpec {
        let artifacts = GuiArtifactSet::new(
            &self.artifact_root,
            self.backend,
            self.scenario.name.as_str(),
        );
        let mut command = CommandSpec {
            program: self
                .program
                .clone()
                .unwrap_or_else(|| self.workspace_root.join("target/release/neomacs")),
            args: vec![
                "-Q".to_string(),
                "-l".to_string(),
                path_to_string(&self.scenario.script),
            ],
            env: vec![
                (
                    "NEOMACS_DEBUG_FIRST_FRAME_READBACK".to_string(),
                    "1".to_string(),
                ),
                (
                    "NEOMACS_DEBUG_SURFACE_READBACK".to_string(),
                    "1".to_string(),
                ),
                (
                    "NEOMACS_DEBUG_SURFACE_READBACK_PNG".to_string(),
                    path_to_string(&artifacts.png),
                ),
                (
                    "NEOMACS_GUI_STATE_JSON".to_string(),
                    path_to_string(&artifacts.gui_state),
                ),
            ],
        };

        if let Some(winit_backend) = self.backend.winit_unix_backend() {
            command
                .env
                .push(("WINIT_UNIX_BACKEND".to_string(), winit_backend.to_string()));
        }
        command.env.extend(self.extra_env.iter().cloned());

        command
    }

    pub fn write_manifest(&self) -> io::Result<GuiArtifactSet> {
        let artifacts = GuiArtifactSet::new(
            &self.artifact_root,
            self.backend,
            self.scenario.name.as_str(),
        );
        let command = self.command_spec();
        self.write_manifest_json(&artifacts, planned_manifest(self, &command, &artifacts))?;
        Ok(artifacts)
    }

    pub fn run_with(
        &self,
        runner: &mut impl GuiCommandRunner,
        options: GuiRunOptions,
    ) -> io::Result<GuiRunResult> {
        let artifacts = self.write_manifest()?;
        let command = self.command_spec();
        let output = runner.run(&command, &artifacts, &options)?;

        if let Some(parent) = artifacts.stderr.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&artifacts.stderr, &output.stderr)?;

        let png_bytes = fs::metadata(&artifacts.png)
            .ok()
            .map(|metadata| metadata.len())
            .filter(|len| *len > 0);
        let status = if png_bytes.is_some() && (output.exit_code == Some(0) || output.timed_out) {
            GuiRunStatus::Passed
        } else if output.timed_out {
            GuiRunStatus::TimedOut
        } else {
            GuiRunStatus::Failed
        };
        let stderr_bytes = output.stderr.len() as u64;
        let stdout_bytes = output.stdout.len() as u64;
        let gui_state = read_gui_state(&artifacts.gui_state)?;
        let gui_state_bytes = fs::metadata(&artifacts.gui_state)
            .ok()
            .map(|metadata| metadata.len())
            .filter(|len| *len > 0);
        let readback_diagnostics = readback_diagnostics(&output.stderr);
        let failure_reason = failure_reason(status, output.exit_code, png_bytes);
        let result = GuiRunResult {
            artifacts: artifacts.clone(),
            status,
            exit_code: output.exit_code,
            timed_out: output.timed_out,
            png_bytes,
            gui_state_bytes,
            stderr_bytes,
            stdout_bytes,
            gui_state,
            readback_diagnostics,
            failure_reason,
        };

        self.write_manifest_json(&artifacts, result_manifest(self, &command, &result))?;
        Ok(result)
    }

    fn write_manifest_json(
        &self,
        artifacts: &GuiArtifactSet,
        manifest: serde_json::Value,
    ) -> io::Result<()> {
        if let Some(parent) = artifacts.json.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&artifacts.json, format!("{}\n", manifest))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    pub fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value.as_str()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiRunOptions {
    pub timeout: Duration,
}

impl GuiRunOptions {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for GuiRunOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiRunStatus {
    Passed,
    Failed,
    TimedOut,
}

impl GuiRunStatus {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::TimedOut => "timed-out",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiCommandOutput {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait GuiCommandRunner {
    fn run(
        &mut self,
        command: &CommandSpec,
        artifacts: &GuiArtifactSet,
        options: &GuiRunOptions,
    ) -> io::Result<GuiCommandOutput>;
}

#[derive(Debug, Default)]
pub struct ProcessGuiCommandRunner;

impl GuiCommandRunner for ProcessGuiCommandRunner {
    fn run(
        &mut self,
        command: &CommandSpec,
        _artifacts: &GuiArtifactSet,
        options: &GuiRunOptions,
    ) -> io::Result<GuiCommandOutput> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .envs(command.env.iter().map(|(key, value)| (key, value)))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout pipe should be captured");
        let stderr = child.stderr.take().expect("stderr pipe should be captured");
        let stdout_reader = thread::spawn(move || read_pipe(stdout));
        let stderr_reader = thread::spawn(move || read_pipe(stderr));

        let start = Instant::now();
        let (exit_code, timed_out) = loop {
            if let Some(status) = child.try_wait()? {
                break (status.code(), false);
            }
            if start.elapsed() >= options.timeout {
                let _ = child.kill();
                let status = child.wait()?;
                break (status.code(), true);
            }
            thread::sleep(Duration::from_millis(50));
        };

        let stdout = stdout_reader.join().unwrap_or_else(|_| Ok(String::new()))?;
        let stderr = stderr_reader.join().unwrap_or_else(|_| Ok(String::new()))?;

        Ok(GuiCommandOutput {
            exit_code,
            timed_out,
            stdout,
            stderr,
        })
    }
}

fn start_weston_headless(artifact_root: &Path) -> io::Result<DisplaySession> {
    let runtime_dir =
        std::env::temp_dir().join(format!("neomacs-gui-tests-{}", std::process::id()));
    fs::create_dir_all(&runtime_dir)?;
    set_owner_only_dir_permissions(&runtime_dir)?;
    fs::create_dir_all(artifact_root)?;
    let log_path = artifact_root.join("weston-headless.log");

    let socket = format!("neomacs-gui-tests-{}", std::process::id());
    let mut child = Command::new("weston")
        .arg("--backend=headless")
        .arg("--renderer=pixman")
        .arg(format!("--socket={socket}"))
        .arg("--idle-time=0")
        .arg("--no-config")
        .arg("--width=1280")
        .arg("--height=800")
        .arg("--fake-seat")
        .arg(format!("--log={}", log_path.display()))
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let socket_path = runtime_dir.join(&socket);
    if wait_for_path(&socket_path, Duration::from_secs(5)) {
        Ok(DisplaySession {
            child: Some(child),
            env: vec![
                ("XDG_RUNTIME_DIR".to_string(), path_to_string(&runtime_dir)),
                ("WAYLAND_DISPLAY".to_string(), socket),
            ],
            cleanup_dir: Some(runtime_dir),
        })
    } else {
        let _ = child.kill();
        let _ = child.wait();
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "weston did not create Wayland socket {}; log: {}",
                socket_path.display(),
                read_log_tail(&log_path)
            ),
        ))
    }
}

fn start_xvfb() -> io::Result<DisplaySession> {
    let display_number = 90 + (std::process::id() % 1000);
    let display = format!(":{display_number}");
    let mut child = Command::new("Xvfb")
        .arg(&display)
        .arg("-screen")
        .arg("0")
        .arg("1280x800x24")
        .arg("-nolisten")
        .arg("tcp")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let socket_path = PathBuf::from(format!("/tmp/.X11-unix/X{display_number}"));
    if wait_for_path(&socket_path, Duration::from_secs(5)) {
        Ok(DisplaySession {
            child: Some(child),
            env: vec![("DISPLAY".to_string(), display)],
            cleanup_dir: None,
        })
    } else {
        let _ = child.kill();
        let _ = child.wait();
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("Xvfb did not create X11 socket {}", socket_path.display()),
        ))
    }
}

fn wait_for_path(path: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if path.exists() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    path.exists()
}

fn set_owner_only_dir_permissions(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn read_log_tail(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" | "),
        Err(err) => format!("failed to read {}: {err}", path.display()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiRunResult {
    pub artifacts: GuiArtifactSet,
    pub status: GuiRunStatus,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub png_bytes: Option<u64>,
    pub gui_state_bytes: Option<u64>,
    pub stderr_bytes: u64,
    pub stdout_bytes: u64,
    pub gui_state: Option<serde_json::Value>,
    pub readback_diagnostics: Vec<String>,
    pub failure_reason: Option<String>,
}

fn read_pipe(mut pipe: impl Read) -> io::Result<String> {
    let mut output = String::new();
    pipe.read_to_string(&mut output)?;
    Ok(output)
}

fn planned_manifest(
    plan: &GuiTestPlan,
    command: &CommandSpec,
    artifacts: &GuiArtifactSet,
) -> serde_json::Value {
    serde_json::json!({
        "status": "planned",
        "scenario": plan.scenario.name,
        "backend": plan.backend.slug(),
        "runner": plan.backend.runner_kind().slug(),
        "command": command_json(command),
        "expected_artifacts": artifacts_json(artifacts),
        "ai_agent_note": "Read this JSON and the stderr log first. Open the PNG only when visual inspection is needed.",
    })
}

fn result_manifest(
    plan: &GuiTestPlan,
    command: &CommandSpec,
    result: &GuiRunResult,
) -> serde_json::Value {
    serde_json::json!({
        "status": result.status.slug(),
        "scenario": plan.scenario.name,
        "backend": plan.backend.slug(),
        "runner": plan.backend.runner_kind().slug(),
        "command": command_json(command),
        "expected_artifacts": artifacts_json(&result.artifacts),
        "observed_artifacts": {
            "png_exists": result.png_bytes.is_some(),
            "png_bytes": result.png_bytes,
            "gui_state_exists": result.gui_state_bytes.is_some(),
            "gui_state_bytes": result.gui_state_bytes,
            "stderr_bytes": result.stderr_bytes,
            "stdout_bytes": result.stdout_bytes,
        },
        "gui_state": result.gui_state,
        "process": {
            "exit_code": result.exit_code,
            "timed_out": result.timed_out,
        },
        "readback_diagnostics": result.readback_diagnostics,
        "failure_reason": result.failure_reason,
        "ai_agent_note": "Use status, observed_artifacts, stderr, and readback_diagnostics for automated checks. The PNG is supplemental visual evidence.",
    })
}

fn command_json(command: &CommandSpec) -> serde_json::Value {
    serde_json::json!({
        "program": path_to_string(&command.program),
        "args": command.args,
        "env": command.env.iter().cloned().collect::<std::collections::BTreeMap<_, _>>(),
    })
}

fn artifacts_json(artifacts: &GuiArtifactSet) -> serde_json::Value {
    serde_json::json!({
        "json": path_to_string(&artifacts.json),
        "png": path_to_string(&artifacts.png),
        "stderr": path_to_string(&artifacts.stderr),
        "gui_state": path_to_string(&artifacts.gui_state),
    })
}

fn read_gui_state(path: &Path) -> io::Result<Option<serde_json::Value>> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map(Some)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn readback_diagnostics(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("surface readback") || lower.contains("bottom_band_avg")
        })
        .map(str::to_string)
        .collect()
}

fn failure_reason(
    status: GuiRunStatus,
    exit_code: Option<i32>,
    png_bytes: Option<u64>,
) -> Option<String> {
    match (status, exit_code, png_bytes) {
        (GuiRunStatus::Passed, _, _) => None,
        (GuiRunStatus::TimedOut, _, _) => Some("GUI command timed out".to_string()),
        (GuiRunStatus::Failed, Some(code), None) if code == 0 => {
            Some("PNG artifact was not generated".to_string())
        }
        (GuiRunStatus::Failed, Some(code), _) => {
            Some(format!("GUI command exited with status {code}"))
        }
        (GuiRunStatus::Failed, None, _) => Some("GUI command exited without a status".to_string()),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
