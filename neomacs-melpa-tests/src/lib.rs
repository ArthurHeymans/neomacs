//! Package ecosystem compatibility harness for Neomacs.
//!
//! A scenario installs packages into an isolated, workspace-local sandbox,
//! exits the editor, and launches a fresh process to probe the installed
//! packages. The same scenario can run against Neomacs or GNU Emacs and
//! against either a frozen package archive or the live package ecosystem.

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use wait_timeout::ChildExt;

const RESULT_MARKER: &str = "NEOMACS-MELPA-RESULT:";
const INSTALLED_MARKER: &str = "NEOMACS-MELPA-INSTALLED:";
const DEFAULT_PROCESS_TIMEOUT: Duration = Duration::from_secs(300);

/// Resolve the checkout used by a normal Cargo run or an extracted Nextest
/// archive.
pub fn workspace_root() -> PathBuf {
    if let Some(root) = std::env::var_os("NEXTEST_WORKSPACE_ROOT") {
        return PathBuf::from(root);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("neomacs-melpa-tests is a workspace member")
        .to_path_buf()
}

/// Per-scenario filesystem and subprocess isolation.
pub struct MelpaSandbox {
    case_root: tempfile::TempDir,
    home: PathBuf,
    tmp: PathBuf,
    xdg_config: PathBuf,
    xdg_cache: PathBuf,
    xdg_data: PathBuf,
    xdg_state: PathBuf,
}

impl MelpaSandbox {
    /// Create a sandbox below `<workspace>/tmp/melpa`.
    pub fn new(label: &str) -> Result<Self, String> {
        let base = workspace_root().join("tmp/melpa");
        fs::create_dir_all(&base).map_err(|error| {
            format!(
                "failed to create MELPA scratch directory {}: {error}",
                base.display()
            )
        })?;
        let prefix = format!("{}-", sanitize_label(label));
        let case_root = tempfile::Builder::new()
            .prefix(&prefix)
            .tempdir_in(&base)
            .map_err(|error| {
                format!(
                    "failed to create MELPA scenario directory in {}: {error}",
                    base.display()
                )
            })?;
        let home = case_root.path().join("home");
        let tmp = case_root.path().join("tmp");
        let xdg_config = case_root.path().join("xdg/config");
        let xdg_cache = case_root.path().join("xdg/cache");
        let xdg_data = case_root.path().join("xdg/data");
        let xdg_state = case_root.path().join("xdg/state");
        for directory in [&home, &tmp, &xdg_config, &xdg_cache, &xdg_data, &xdg_state] {
            fs::create_dir_all(directory).map_err(|error| {
                format!(
                    "failed to create MELPA sandbox directory {}: {error}",
                    directory.display()
                )
            })?;
        }
        fs::create_dir_all(home.join(".emacs.d"))
            .map_err(|error| format!("failed to create isolated .emacs.d: {error}"))?;

        Ok(Self {
            case_root,
            home,
            tmp,
            xdg_config,
            xdg_cache,
            xdg_data,
            xdg_state,
        })
    }

    pub fn root(&self) -> &Path {
        self.case_root.path()
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn tmp_dir(&self) -> &Path {
        &self.tmp
    }

    /// Apply the deterministic process environment shared by install and
    /// restart/probe processes.
    pub fn configure(&self, command: &mut Command) {
        command
            .current_dir(self.root())
            .env("HOME", &self.home)
            .env("TMPDIR", &self.tmp)
            .env("TMP", &self.tmp)
            .env("TEMP", &self.tmp)
            .env("XDG_CONFIG_HOME", &self.xdg_config)
            .env("XDG_CACHE_HOME", &self.xdg_cache)
            .env("XDG_DATA_HOME", &self.xdg_data)
            .env("XDG_STATE_HOME", &self.xdg_state)
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8")
            .env("TZ", "UTC")
            .env("USER", "melpa-test")
            .env("LOGNAME", "melpa-test")
            .env("HOSTNAME", "melpa-host")
            .env("EMAIL", "melpa-test@melpa-host")
            .env("TERM", "dumb")
            .env_remove("EMACSLOADPATH")
            .env("GIT_CEILING_DIRECTORIES", workspace_root());
    }
}

fn sanitize_label(label: &str) -> String {
    let sanitized = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "scenario".to_string()
    } else {
        sanitized
    }
}

/// An editor executable that can run a package scenario.
#[derive(Clone, Debug)]
pub struct EmacsRuntime {
    pub name: String,
    pub executable: PathBuf,
    extra_env: Vec<(OsString, OsString)>,
    timeout: Duration,
}

impl EmacsRuntime {
    pub fn new(name: impl Into<String>, executable: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            executable: executable.into(),
            extra_env: Vec::new(),
            timeout: DEFAULT_PROCESS_TIMEOUT,
        }
    }

    pub fn neomacs() -> Self {
        Self::new("neomacs", neomacs_binary())
    }

    /// GNU Emacs oracle selected explicitly by environment, then from the
    /// developer's adjacent source checkout, and finally from `PATH`.
    pub fn gnu_emacs() -> Self {
        for variable in [
            "NEOMACS_MELPA_ORACLE_EMACS",
            "NEOVM_ORACLE_EMACS",
            "ORACLE_EMACS",
        ] {
            if let Some(path) = std::env::var_os(variable) {
                return Self::new("gnu-emacs", PathBuf::from(path));
            }
        }
        let source_checkout =
            PathBuf::from("/home/exec/Projects/github.com/emacs-mirror/emacs/src/emacs");
        if source_checkout.is_file() {
            return Self::new("gnu-emacs", source_checkout);
        }
        Self::new("gnu-emacs", "emacs")
    }

    pub fn with_env(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.extra_env.push((name.into(), value.into()));
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        for (name, value) in &self.extra_env {
            command.env(name, value);
        }
        command
    }
}

/// Package archive used by a scenario.
#[derive(Clone, Debug)]
pub enum PackageSource {
    Frozen { archive_dir: PathBuf },
    LiveMelpa,
}

impl PackageSource {
    pub fn frozen(archive_dir: impl Into<PathBuf>) -> Self {
        Self::Frozen {
            archive_dir: archive_dir.into(),
        }
    }

    pub fn live_melpa() -> Self {
        Self::LiveMelpa
    }

    fn archive_form(&self) -> String {
        match self {
            Self::Frozen { archive_dir } => {
                let directory = archive_dir
                    .canonicalize()
                    .unwrap_or_else(|_| archive_dir.clone());
                let directory = format!("{}/", directory.display());
                format!(r##"'(("frozen" . {}))"##, elisp_string(&directory))
            }
            Self::LiveMelpa => r##"'(("gnu" . "https://elpa.gnu.org/packages/")
                      ("melpa" . "https://melpa.org/packages/"))"##
                .to_string(),
        }
    }
}

/// Packages and the post-restart Elisp probe that define one compatibility
/// scenario.
#[derive(Clone, Debug)]
pub struct PackageScenario {
    pub name: String,
    pub packages: Vec<String>,
    pub probe: String,
}

impl PackageScenario {
    pub fn new<I, P>(name: impl Into<String>, packages: I, probe: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        Self {
            name: name.into(),
            packages: packages.into_iter().map(Into::into).collect(),
            probe: probe.into(),
        }
    }

    pub fn from_probe_file<I, P>(
        name: impl Into<String>,
        packages: I,
        probe_path: impl AsRef<Path>,
    ) -> Result<Self, String>
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        let probe_path = probe_path.as_ref();
        let probe = fs::read_to_string(probe_path).map_err(|error| {
            format!(
                "failed to read package probe {}: {error}",
                probe_path.display()
            )
        })?;
        Ok(Self::new(name, packages, probe))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioPhase {
    Install,
    RestartProbe,
}

#[derive(Debug)]
pub struct PhaseReport {
    pub phase: ScenarioPhase,
    pub duration: Duration,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub struct ScenarioReport {
    pub runtime: String,
    pub scenario: String,
    pub phases: Vec<PhaseReport>,
    pub installed_packages: Vec<InstalledPackage>,
    pub result: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}

impl fmt::Display for ScenarioReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{} scenario `{}` installed: {}",
            self.runtime,
            self.scenario,
            format_installed_packages(&self.installed_packages)
        )?;
        for phase in &self.phases {
            writeln!(
                formatter,
                "{:?}: status {:?}, {:.2?}",
                phase.phase, phase.status_code, phase.duration
            )?;
        }
        write!(formatter, "result: {}", self.result)
    }
}

/// Install a scenario's packages, exit the editor, and probe them in a fresh
/// process using the same isolated home.
pub fn run_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<ScenarioReport, String> {
    let sandbox = MelpaSandbox::new(&scenario.name)?;
    let install_form = install_form(source, &scenario.packages);
    let probe_form = probe_form(&scenario.probe);
    let mut phases = Vec::with_capacity(2);

    let install = run_phase(
        runtime,
        &sandbox,
        &scenario.name,
        ScenarioPhase::Install,
        &install_form,
    )?;
    let installed_packages = extract_installed_packages(&install.stdout).map_err(|error| {
        format!(
            "{} scenario `{}` emitted an invalid installed-package report during Install: {error}\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, install.stdout, install.stderr
        )
    })?;
    phases.push(install);

    let probe = run_phase(
        runtime,
        &sandbox,
        &scenario.name,
        ScenarioPhase::RestartProbe,
        &probe_form,
    )
    .map_err(|error| {
        format!(
            "{error}\ninstalled packages: {}",
            format_installed_packages(&installed_packages)
        )
    })?;
    let result = extract_marker(&probe.stdout, RESULT_MARKER).ok_or_else(|| {
        format!(
            "{} scenario `{}` did not emit `{RESULT_MARKER}` during RestartProbe\ninstalled packages: {}\nstdout:\n{}\nstderr:\n{}",
            runtime.name,
            scenario.name,
            format_installed_packages(&installed_packages),
            probe.stdout,
            probe.stderr
        )
    })?;
    phases.push(probe);

    Ok(ScenarioReport {
        runtime: runtime.name.clone(),
        scenario: scenario.name.clone(),
        phases,
        installed_packages,
        result,
    })
}

fn run_phase(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
) -> Result<PhaseReport, String> {
    let mut command = runtime.command();
    sandbox.configure(&mut command);
    command
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "--quick", "--eval", form]);
    let started = Instant::now();
    let output =
        output_with_timeout(&mut command, runtime.timeout).map_err(|error| match error {
            CommandError::Launch(error) => format!(
                "failed to launch {} for {phase:?} in scenario `{scenario_name}` sandbox {}: {error}",
                runtime.name,
                sandbox.root().display()
            ),
            CommandError::TimedOut => format!(
                "{} scenario `{scenario_name}` timed out during {phase:?} after {:?} in sandbox {}",
                runtime.name,
                runtime.timeout,
                sandbox.root().display()
            ),
            CommandError::Capture(error) => format!(
                "failed to capture {} scenario `{scenario_name}` output during {phase:?}: {error}",
                runtime.name
            ),
        })?;
    let report = phase_report(phase, started.elapsed(), output);
    if report.status_code != Some(0) {
        return Err(format!(
            "{} scenario `{scenario_name}` failed during {phase:?} (exit {:?})\nstdout:\n{}\nstderr:\n{}",
            runtime.name, report.status_code, report.stdout, report.stderr
        ));
    }
    check_error_markers(&report.stdout, &report.stderr).map_err(|error| {
        format!(
            "{} scenario `{scenario_name}` failed during {phase:?}: {error}",
            runtime.name
        )
    })?;
    Ok(report)
}

enum CommandError {
    Launch(std::io::Error),
    TimedOut,
    Capture(String),
}

fn output_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output, CommandError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(CommandError::Launch)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CommandError::Capture("stdout pipe was not created".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CommandError::Capture("stderr pipe was not created".to_string()))?;
    let stdout_reader = thread::spawn(move || read_pipe(stdout));
    let stderr_reader = thread::spawn(move || read_pipe(stderr));

    let status = match child.wait_timeout(timeout).map_err(CommandError::Launch)? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(CommandError::TimedOut);
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| CommandError::Capture("stdout reader panicked".to_string()))?
        .map_err(|error| CommandError::Capture(format!("failed to read stdout: {error}")))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| CommandError::Capture("stderr reader panicked".to_string()))?
        .map_err(|error| CommandError::Capture(format!("failed to read stderr: {error}")))?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_pipe(mut pipe: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn phase_report(phase: ScenarioPhase, duration: Duration, output: Output) -> PhaseReport {
    PhaseReport {
        phase,
        duration,
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn install_form(source: &PackageSource, packages: &[String]) -> String {
    let installs = packages
        .iter()
        .map(|package| format!("(package-install '{package})"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r##"(progn
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                 package-archives {}
                 package-check-signature nil)
           (package-initialize)
           (package-refresh-contents)
           {}
           (let ((installed
                  (mapcar
                   (lambda (entry)
                     (cons (car entry)
                           (package-version-join
                            (package-desc-version (cadr entry)))))
                   package-alist)))
             (setq installed
                   (sort installed
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right))))))
             (dolist (entry installed)
               (princ "\n{INSTALLED_MARKER}")
               (princ (symbol-name (car entry)))
               (princ "\t")
               (princ (cdr entry)))))"##,
        source.archive_form(),
        installs
    )
}

fn probe_form(probe: &str) -> String {
    format!(
        r##"(progn
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
           (package-initialize)
           {})"##,
        probe
    )
}

fn extract_marker(stdout: &str, marker: &str) -> Option<String> {
    stdout
        .lines()
        .filter_map(|line| line.split_once(marker).map(|(_, value)| value.trim()))
        .next_back()
        .map(str::to_string)
}

fn extract_installed_packages(stdout: &str) -> Result<Vec<InstalledPackage>, String> {
    let mut installed = Vec::new();
    for value in stdout
        .lines()
        .filter_map(|line| line.split_once(INSTALLED_MARKER).map(|(_, value)| value))
    {
        let (name, version) = value.trim().split_once('\t').ok_or_else(|| {
            format!(r##"expected `{INSTALLED_MARKER}<name>\t<version>`, got `{value}`"##)
        })?;
        installed.push(InstalledPackage {
            name: name.to_string(),
            version: version.to_string(),
        });
    }
    if installed.is_empty() {
        return Err(format!("did not emit `{INSTALLED_MARKER}`"));
    }
    Ok(installed)
}

fn format_installed_packages(installed: &[InstalledPackage]) -> String {
    installed
        .iter()
        .map(|package| format!("{}@{}", package.name, package.version))
        .collect::<Vec<_>>()
        .join(", ")
}

fn elisp_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn check_error_markers(stdout: &str, stderr: &str) -> Result<(), String> {
    for needle in [
        "wrong-type-argument",
        "void-function",
        "file-missing",
        "invalid-read-syntax",
        "end-of-file",
        "Error:",
    ] {
        if stdout.contains(needle) || stderr.contains(needle) {
            return Err(format!(
                "editor emitted `{needle}`:\nstdout:\n{stdout}\nstderr:\n{stderr}"
            ));
        }
    }
    Ok(())
}

/// The path to the `neomacs` binary (override with `NEOMACS_BIN`).
pub fn neomacs_binary() -> PathBuf {
    std::env::var_os("NEOMACS_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target/release/neomacs"))
}
