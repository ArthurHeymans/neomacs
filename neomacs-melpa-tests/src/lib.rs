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

use neomacs_test_oracle::{EvalOutcome, extract_marked_outcome, wrap_elisp_outcome};
use wait_timeout::ChildExt;

const RESULT_MARKER: &str = "NEOMACS-MELPA-RESULT:";
const OUTCOME_MARKER: &str = "NEOMACS-MELPA-OUTCOME:";
const INSTALLED_MARKER: &str = "NEOMACS-MELPA-INSTALLED:";
const DEFAULT_PROCESS_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy)]
struct PackageArchiveSpec {
    cache_directory: &'static str,
    label: &'static str,
    name: &'static str,
    url: &'static str,
}

const MELPA_ARCHIVE: PackageArchiveSpec = PackageArchiveSpec {
    cache_directory: "package-cache",
    label: "MELPA",
    name: "melpa",
    url: "https://melpa.org/packages/",
};

const GNU_ELPA_ARCHIVE: PackageArchiveSpec = PackageArchiveSpec {
    cache_directory: "package-cache-gnu-elpa",
    label: "GNU ELPA",
    name: "gnu",
    url: "https://elpa.gnu.org/packages/",
};

/// The exact Async release selected from GNU ELPA by the comprehensive API
/// parity corpus.
pub const ASYNC_GNU_ELPA_PIN: (&str, &str) = ("async", "1.9.9");

/// The exact Dash package selected by the live lifecycle and comprehensive
/// API parity corpora.
pub const DASH_MELPA_PIN: (&str, &str) = ("dash", "20260221.1346");

/// The exact Bind-Key release selected from GNU ELPA by the comprehensive API
/// parity corpus.
pub const BIND_KEY_GNU_ELPA_PIN: (&str, &str) = ("bind-key", "2.4.1");

/// The exact Compat release selected from GNU ELPA by the comprehensive API
/// parity corpus.
pub const COMPAT_GNU_ELPA_PIN: (&str, &str) = ("compat", "31.0.0.2");

/// The exact f package selected by the comprehensive API parity corpus.
pub const F_MELPA_PIN: (&str, &str) = ("f", "20241003.1131");

/// The exact Magit package containing the Git-Commit source selected by the
/// comprehensive API parity corpus.
pub const GIT_COMMIT_MELPA_PIN: (&str, &str) = ("magit", "20260724.2338");

/// The exact General package selected by the comprehensive API parity corpus.
pub const GENERAL_MELPA_PIN: (&str, &str) = ("general", "20250612.2309");

/// The exact Magit package selected by the comprehensive API parity corpus.
pub const MAGIT_MELPA_PIN: (&str, &str) = ("magit", "20260724.2338");

/// The exact magit-section package selected by the comprehensive API parity
/// corpus.
pub const MAGIT_SECTION_MELPA_PIN: (&str, &str) = ("magit-section", "20260722.2131");

/// The exact Projectile package selected by the comprehensive API parity
/// corpus.
pub const PROJECTILE_MELPA_PIN: (&str, &str) = ("projectile", "20260725.1657");

/// The exact s package selected by the live lifecycle and comprehensive API
/// parity corpora.
pub const S_MELPA_PIN: (&str, &str) = ("s", "20220902.1511");

/// The exact Transient package selected by the comprehensive API parity
/// corpus.
pub const TRANSIENT_MELPA_PIN: (&str, &str) = ("transient", "20260725.1105");

/// The exact Use-Package release selected from GNU ELPA by the comprehensive
/// API parity corpus.
pub const USE_PACKAGE_GNU_ELPA_PIN: (&str, &str) = ("use-package", "2.4.6");

/// The exact With-Editor package selected by the comprehensive API parity
/// corpus.
pub const WITH_EDITOR_MELPA_PIN: (&str, &str) = ("with-editor", "20260701.1252");

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
        configure_process_environment(command, self.root(), &self.home, &self.tmp);
    }
}

fn configure_process_environment(command: &mut Command, root: &Path, home: &Path, tmp: &Path) {
    command
        .current_dir(root)
        .env("HOME", home)
        .env("TMPDIR", tmp)
        .env("TMP", tmp)
        .env("TEMP", tmp)
        .env("XDG_CONFIG_HOME", root.join("xdg/config"))
        .env("XDG_CACHE_HOME", root.join("xdg/cache"))
        .env("XDG_DATA_HOME", root.join("xdg/data"))
        .env("XDG_STATE_HOME", root.join("xdg/state"))
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .env("USER", "melpa-test")
        .env("LOGNAME", "melpa-test")
        .env("HOSTNAME", "melpa-host")
        .env("EMAIL", "melpa-test@melpa-host")
        .env("TERM", "dumb")
        .env("NEOMACS_TEST_SANDBOX_ROOT", root)
        .env("NEOMACS_TEST_WORKSPACE_ROOT", workspace_root())
        .env_remove("EMACSLOADPATH")
        .env("GIT_CEILING_DIRECTORIES", workspace_root());
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
pub struct PackageSource {
    archives: Vec<(String, PathBuf)>,
}

impl PackageSource {
    pub fn frozen(archive_dir: impl Into<PathBuf>) -> Self {
        Self {
            archives: vec![("frozen".to_string(), archive_dir.into())],
        }
    }

    pub fn local<I, N, P>(archives: I) -> Self
    where
        I: IntoIterator<Item = (N, P)>,
        N: Into<String>,
        P: Into<PathBuf>,
    {
        Self {
            archives: archives
                .into_iter()
                .map(|(name, path)| (name.into(), path.into()))
                .collect(),
        }
    }

    fn archive_form(&self) -> String {
        let entries = self
            .archives
            .iter()
            .map(|(name, directory)| {
                let directory = directory
                    .canonicalize()
                    .unwrap_or_else(|_| directory.clone());
                let directory = format!("{}/", directory.display());
                format!("({} . {})", elisp_string(name), elisp_string(&directory))
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("'({entries})")
    }
}

/// Packages and the post-restart Elisp probe that define one compatibility
/// scenario.
#[derive(Clone, Debug)]
pub struct PackageScenario {
    pub name: String,
    packages: PackageSelection,
    pub probe: String,
}

#[derive(Clone, Debug)]
enum PackageSelection {
    Unversioned(Vec<String>),
    Versioned(Vec<PackagePin>),
}

/// An exact package name/version selected for a live archive scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagePin {
    pub name: String,
    pub version: String,
}

impl PackageScenario {
    pub fn new<I, P>(name: impl Into<String>, packages: I, probe: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        Self {
            name: name.into(),
            packages: PackageSelection::Unversioned(packages.into_iter().map(Into::into).collect()),
            probe: probe.into(),
        }
    }

    /// Define a scenario whose selected third-party packages have exact
    /// versions.
    pub fn versioned<I, N, V>(
        name: impl Into<String>,
        packages: I,
        probe: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = (N, V)>,
        N: Into<String>,
        V: Into<String>,
    {
        Self {
            name: name.into(),
            packages: PackageSelection::Versioned(
                packages
                    .into_iter()
                    .map(|(name, version)| PackagePin {
                        name: name.into(),
                        version: version.into(),
                    })
                    .collect(),
            ),
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

    /// Build a package-agnostic probe of the post-restart autoload surface.
    ///
    /// This is the scalable baseline for a package corpus: it does not guess
    /// arguments or invoke arbitrary package commands. It inventories
    /// autoloaded functions/macros, custom variables, and emitted bytecode for
    /// the complete dependency graph. Curated probes can be added separately
    /// when meaningful behavior and inputs are known.
    pub fn autoload_surface<I, P>(name: impl Into<String>, packages: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        let packages = packages.into_iter().map(Into::into).collect::<Vec<_>>();
        let package_strings = packages
            .iter()
            .map(|package| elisp_string(package))
            .collect::<Vec<_>>()
            .join(" ");
        let probe = format!(
            r##"(let* ((requested
                         (mapcar #'intern '({package_strings})))
                       (libraries (make-hash-table :test 'equal))
                       (known-library-p
                        (lambda (library)
                          (and
                           (stringp library)
                           (or (gethash library libraries)
                               (gethash
                                (file-name-sans-extension library)
                                libraries)
                               (gethash
                                (file-name-base library)
                                libraries)))))
                       (autoloads nil)
                       (customs nil)
                       (bytecode nil))
                  (dolist (package requested)
                    (unless (package-installed-p package)
                      (error "requested package was not installed: %S" package)))
                  (dolist (entry package-alist)
                    (let* ((description (cadr entry))
                           (directory (package-desc-dir description))
                           (files
                            (and directory
                                 (file-directory-p directory)
                                 (directory-files-recursively
                                  directory "\\.elc?\\'")))
                           (compiled nil))
                      (dolist (file files)
                        (let* ((relative
                                (file-relative-name file directory))
                               (library
                                (file-name-sans-extension relative)))
                          (puthash library t libraries)
                          (puthash (file-name-base library) t libraries)
                          (when (string-suffix-p ".elc" relative)
                            (push relative compiled))))
                      (push
                       (list
                        (car entry)
                        (package-version-join
                         (package-desc-version description))
                        (sort compiled #'string<))
                       bytecode)))
                  (mapatoms
                   (lambda (symbol)
                     (let ((definition
                            (and (fboundp symbol)
                                 (symbol-function symbol))))
                       (when (and (autoloadp definition)
                                  (funcall known-library-p (nth 1 definition)))
                         (push
                          (list symbol
                                (nth 1 definition)
                                (if (eq (nth 4 definition) 'macro)
                                    'macro
                                  (if (nth 3 definition)
                                      'command
                                    'function)))
                          autoloads)))
                     (let ((custom-libraries nil))
                       (dolist (library (get symbol 'custom-loads))
                         (let ((library-name
                                (cond
                                 ((stringp library) library)
                                 ((symbolp library) (symbol-name library)))))
                           (when (and library-name
                                      (funcall known-library-p library-name))
                             (push library-name custom-libraries))))
                       (when custom-libraries
                         (push
                          (list symbol
                                (sort custom-libraries #'string<))
                          customs)))))
                  (list
                   :autoloads
                   (sort autoloads
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right)))))
                   :customs
                   (sort customs
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right)))))
                   :bytecode
                   (sort bytecode
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right)))))))"##
        );
        Self::new(name, packages, probe)
    }

    fn package_names(&self) -> Vec<&str> {
        match &self.packages {
            PackageSelection::Unversioned(packages) => {
                packages.iter().map(String::as_str).collect()
            }
            PackageSelection::Versioned(packages) => packages
                .iter()
                .map(|package| package.name.as_str())
                .collect(),
        }
    }

    fn package_pins(&self) -> Option<&[PackagePin]> {
        match &self.packages {
            PackageSelection::Unversioned(_) => None,
            PackageSelection::Versioned(packages) => Some(packages),
        }
    }
}

/// One ERT selector loaded from an Emacs Lisp test file.
#[derive(Clone, Debug)]
pub struct ErtScenario {
    pub name: String,
    pub test_file: PathBuf,
    pub selector: String,
}

impl ErtScenario {
    pub fn new(
        name: impl Into<String>,
        test_file: impl Into<PathBuf>,
        selector: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            test_file: test_file.into(),
            selector: selector.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioPhase {
    PrepareArchive,
    Install,
    RestartProbe,
    QuickstartProbe,
    VcInstall,
    VcRestart,
    VcUpgrade,
    VcDelete,
    VcRestartAfterDelete,
    Ert,
}

/// A current package transaction downloaded once for both editor adapters.
///
/// The owning sandbox keeps catalogs and package payloads alive below
/// `<workspace>/tmp/melpa` for the duration of the oracle comparison.
pub struct PreparedPackageSource {
    _sandbox: MelpaSandbox,
    source: PackageSource,
}

impl PreparedPackageSource {
    pub fn source(&self) -> &PackageSource {
        &self.source
    }
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
    pub outcome: EvalOutcome,
}

/// GNU Emacs and Neomacs reports after package lifecycle parity is verified.
#[derive(Debug)]
pub struct OracleScenarioReport {
    pub neomacs: ScenarioReport,
    pub gnu_emacs: ScenarioReport,
}

/// GNU Emacs and Neomacs outcomes for one direct Elisp form.
#[derive(Debug)]
pub struct ElispOracleReport {
    pub neomacs: EvalOutcome,
    pub gnu_emacs: EvalOutcome,
}

/// Differential oracle for one exact package cached below `./tmp`.
pub struct CachedPackageOracle {
    package_name: String,
    package_user_dir: PathBuf,
    source_file: PathBuf,
    prelude: String,
    timeout: Duration,
}

/// MELPA-focused name retained for package-specific parity modules.
pub type CachedMelpaOracle = CachedPackageOracle;

impl CachedPackageOracle {
    /// Prepare the pinned package and select the Elisp source file to load.
    pub fn new(package: (&str, &str), source_file_name: &str) -> Result<Self, String> {
        validate_cached_source_file_name(MELPA_ARCHIVE.label, source_file_name)?;
        let package_dir = prepare_cached_melpa_package(&EmacsRuntime::gnu_emacs(), package)?;
        Self::from_package_dir(package, source_file_name, package_dir)
    }

    /// Prepare one pinned GNU ELPA package and select its Elisp source file.
    pub fn new_from_gnu_elpa(
        package: (&str, &str),
        source_file_name: &str,
    ) -> Result<Self, String> {
        validate_cached_source_file_name(GNU_ELPA_ARCHIVE.label, source_file_name)?;
        let package_dir = prepare_cached_gnu_elpa_package(&EmacsRuntime::gnu_emacs(), package)?;
        Self::from_package_dir(package, source_file_name, package_dir)
    }

    fn from_package_dir(
        package: (&str, &str),
        source_file_name: &str,
        package_dir: PathBuf,
    ) -> Result<Self, String> {
        let source_file = package_dir.join(source_file_name);
        if !source_file.is_file() {
            return Err(format!(
                "cached {} source `{source_file_name}` is missing below {}",
                package.0,
                package_dir.display()
            ));
        }
        let package_user_dir = package_dir
            .parent()
            .expect("cached package directory is below an ELPA directory")
            .to_path_buf();
        Ok(Self {
            package_name: package.0.to_string(),
            package_user_dir,
            source_file,
            prelude: String::new(),
            timeout: DEFAULT_PROCESS_TIMEOUT,
        })
    }

    /// Evaluate an additional setup form before loading the package source.
    pub fn with_prelude(mut self, prelude: impl Into<String>) -> Self {
        self.prelude = prelude.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Run a parity case that must complete with a value in both editors.
    pub fn run_value(&self, name: &str, probe: &str) -> Result<ElispOracleReport, String> {
        self.run_expected(name, probe, true)
    }

    /// Run a parity case that must signal in both editors.
    pub fn run_signal(&self, name: &str, probe: &str) -> Result<ElispOracleReport, String> {
        self.run_expected(name, probe, false)
    }

    fn run_expected(
        &self,
        name: &str,
        probe: &str,
        expect_value: bool,
    ) -> Result<ElispOracleReport, String> {
        let neomacs = EmacsRuntime::neomacs()
            .with_env(
                "NEOMACS_PACKAGE_USER_DIR",
                self.package_user_dir.as_os_str(),
            )
            .with_env("NEOMACS_PACKAGE_SOURCE", self.source_file.as_os_str())
            .with_timeout(self.timeout);
        let gnu_emacs = EmacsRuntime::gnu_emacs()
            .with_env(
                "NEOMACS_PACKAGE_USER_DIR",
                self.package_user_dir.as_os_str(),
            )
            .with_env("NEOMACS_PACKAGE_SOURCE", self.source_file.as_os_str())
            .with_timeout(self.timeout);
        let setup = format!(
            r##"(progn
                   (require 'package)
                   (setq package-user-dir
                         (getenv "NEOMACS_PACKAGE_USER_DIR")
                         load-suffixes '(".el"))
                   (package-initialize)
                   {}
                   (load
                    (getenv "NEOMACS_PACKAGE_SOURCE")
                    nil t t))"##,
            self.prelude
        );
        let report = run_elisp_oracle(&neomacs, &gnu_emacs, name, &setup, probe)?;
        if report.neomacs.is_value() != expect_value {
            let expected = if expect_value { "a value" } else { "a signal" };
            return Err(format!(
                "{} parity case `{name}` expected {expected}, got {}",
                self.package_name, report.neomacs
            ));
        }
        Ok(report)
    }
}

fn validate_cached_source_file_name(
    archive_label: &str,
    source_file_name: &str,
) -> Result<(), String> {
    let mut components = Path::new(source_file_name).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(format!(
            "cached {archive_label} source must be one file name, got `{source_file_name}`"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErtSummary {
    pub total: usize,
    pub expected: usize,
    pub unexpected: usize,
    pub skipped: usize,
}

#[derive(Debug)]
pub struct ErtReport {
    pub runtime: String,
    pub scenario: String,
    pub phase: PhaseReport,
    pub summary: ErtSummary,
}

#[derive(Debug)]
pub struct PackageVcReport {
    pub runtime: String,
    pub phases: Vec<PhaseReport>,
    pub checkpoints: Vec<String>,
}

struct PackageVcProgress {
    phases: Vec<PhaseReport>,
    checkpoints: Vec<String>,
}

impl PackageVcProgress {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            phases: Vec::with_capacity(capacity),
            checkpoints: Vec::with_capacity(capacity),
        }
    }
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
        write!(formatter, "outcome: {}", self.outcome)
    }
}

impl fmt::Display for ErtReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ERT scenario `{}`: {} total, {} expected, {} unexpected, {} skipped ({:.2?})",
            self.runtime,
            self.scenario,
            self.summary.total,
            self.summary.expected,
            self.summary.unexpected,
            self.summary.skipped,
            self.phase.duration
        )
    }
}

impl fmt::Display for PackageVcReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{} package-vc lifecycle: {}",
            self.runtime,
            self.checkpoints.join(" -> ")
        )?;
        for phase in &self.phases {
            writeln!(
                formatter,
                "{:?}: status {:?}, {:.2?}",
                phase.phase, phase.status_code, phase.duration
            )?;
        }
        Ok(())
    }
}

/// Load an Emacs Lisp test file and run one ERT selector inside an isolated
/// editor process.
pub fn run_ert_scenario(
    runtime: &EmacsRuntime,
    scenario: &ErtScenario,
) -> Result<ErtReport, String> {
    if !scenario.test_file.is_file() {
        return Err(format!(
            "ERT scenario `{}` test file does not exist: {}",
            scenario.name,
            scenario.test_file.display()
        ));
    }

    let sandbox = MelpaSandbox::new(&scenario.name)?;
    let load_directory = scenario
        .test_file
        .parent()
        .expect("ERT test files have a parent directory");
    let eval = format!(r##"(ert-run-tests-batch {})"##, scenario.selector);
    let mut command = runtime.command();
    sandbox.configure(&mut command);
    command
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "--quick", "-L"])
        .arg(load_directory)
        .arg("-l")
        .arg(&scenario.test_file)
        .args(["--eval", &eval]);

    let started = Instant::now();
    let output = output_with_timeout(&mut command, runtime.timeout).map_err(|error| {
        command_error_message(error, runtime, &sandbox, &scenario.name, ScenarioPhase::Ert)
    })?;
    let phase = phase_report(ScenarioPhase::Ert, started.elapsed(), output);
    if phase.status_code != Some(0) {
        return Err(format!(
            "{} ERT scenario `{}` failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, phase.status_code, phase.stdout, phase.stderr
        ));
    }
    let summary = extract_ert_summary(&phase.stdout, &phase.stderr).ok_or_else(|| {
        format!(
            "{} ERT scenario `{}` did not emit an ERT summary\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, phase.stdout, phase.stderr
        )
    })?;
    if summary.unexpected != 0 {
        return Err(format!(
            "{} ERT scenario `{}` reported {} unexpected result(s)\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, summary.unexpected, phase.stdout, phase.stderr
        ));
    }

    Ok(ErtReport {
        runtime: runtime.name.clone(),
        scenario: scenario.name.clone(),
        phase,
        summary,
    })
}

/// Install a scenario's packages, exit the editor, and probe them in a fresh
/// process using the same isolated home.
pub fn run_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<ScenarioReport, String> {
    run_install_and_probe(
        runtime,
        scenario,
        install_form(source, &scenario.package_names(), ""),
        probe_form(&scenario.probe),
        ScenarioPhase::RestartProbe,
    )
}

/// Install one exact MELPA package into a validated, cross-process cache.
///
/// The returned package directory stays below
/// `<workspace>/tmp/melpa/package-cache`. Package payloads remain runtime
/// artifacts and are never copied into the source tree.
pub fn prepare_cached_melpa_package(
    gnu_emacs: &EmacsRuntime,
    package: (&str, &str),
) -> Result<PathBuf, String> {
    prepare_cached_package(gnu_emacs, package, MELPA_ARCHIVE)
}

/// Install one exact GNU ELPA package into a validated, cross-process cache.
///
/// Like the MELPA cache, this remains a workspace-local runtime artifact.
pub fn prepare_cached_gnu_elpa_package(
    gnu_emacs: &EmacsRuntime,
    package: (&str, &str),
) -> Result<PathBuf, String> {
    prepare_cached_package(gnu_emacs, package, GNU_ELPA_ARCHIVE)
}

fn prepare_cached_package(
    gnu_emacs: &EmacsRuntime,
    package: (&str, &str),
    archive: PackageArchiveSpec,
) -> Result<PathBuf, String> {
    let (name, version) = package;
    if name.is_empty()
        || version.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        || !version.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '+')
        })
    {
        return Err(format!(
            "cached {} package must have a safe hard-coded name and version, got `{name}` `{version}`",
            archive.label
        ));
    }

    let root = workspace_root()
        .join("tmp/melpa")
        .join(archive.cache_directory)
        .join(name)
        .join(version);
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create package cache root {}: {error}",
            root.display()
        )
    })?;
    let lock_path = root.join("prepare.lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            format!(
                "failed to open package cache lock {}: {error}",
                lock_path.display()
            )
        })?;
    fs4::FileExt::lock(&lock)
        .map_err(|error| format!("failed to lock package cache {}: {error}", root.display()))?;

    let home = root.join("home");
    let tmp = root.join("tmp");
    let package_dir = home.join(".emacs.d/elpa").join(format!("{name}-{version}"));
    let descriptor = package_dir.join(format!("{name}-pkg.el"));
    let ready_marker = root.join("ready");
    let expected_marker = format!("{name}\t{version}\n");
    let cache_is_ready = descriptor.is_file()
        && fs::read_to_string(&ready_marker).is_ok_and(|contents| contents == expected_marker);
    if cache_is_ready {
        return Ok(package_dir);
    }

    if home.exists() {
        fs::remove_dir_all(&home).map_err(|error| {
            format!(
                "failed to remove incomplete package cache {}: {error}",
                home.display()
            )
        })?;
    }
    if ready_marker.exists() {
        fs::remove_file(&ready_marker).map_err(|error| {
            format!(
                "failed to remove invalid package cache marker {}: {error}",
                ready_marker.display()
            )
        })?;
    }
    for directory in [
        home.join(".emacs.d"),
        tmp.clone(),
        root.join("xdg/config"),
        root.join("xdg/cache"),
        root.join("xdg/data"),
        root.join("xdg/state"),
    ] {
        fs::create_dir_all(&directory).map_err(|error| {
            format!(
                "failed to create package cache directory {}: {error}",
                directory.display()
            )
        })?;
    }

    let name_string = elisp_string(name);
    let version_string = elisp_string(version);
    let archive_name_string = elisp_string(archive.name);
    let archive_url_string = elisp_string(archive.url);
    let form = format!(
        r##"(progn
               (require 'package)
               (setq package-user-dir
                     (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                     package-check-signature nil
                     package-archives
                     (list
                      (cons {archive_name_string}
                            {archive_url_string})))
               (package-refresh-contents)
               (let* ((package-name {name_string})
                      (expected-version {version_string})
                      (package-symbol (intern package-name))
                      (description
                       (cadr
                        (assq package-symbol package-archive-contents)))
                      (archive-version
                       (and description
                            (package-version-join
                             (package-desc-version description)))))
                 (unless description
                   (error "Package is absent from selected archive: %s"
                          package-name))
                 (unless (equal archive-version expected-version)
                   (error
                    "Package version changed: %s expected %s, current %s"
                    package-name expected-version archive-version))
                 (package-install description)
                 (package-initialize)
                 (let* ((installed
                         (cadr (assq package-symbol package-alist)))
                        (installed-version
                         (and installed
                              (package-version-join
                               (package-desc-version installed))))
                        (directory
                         (and installed (package-desc-dir installed)))
                        (descriptor
                         (and directory
                              (expand-file-name
                               (concat package-name "-pkg.el")
                               directory))))
                   (unless (equal installed-version expected-version)
                     (error
                      "Installed package version mismatch: %s expected %s, got %s"
                      package-name expected-version installed-version))
                   (unless (and descriptor (file-readable-p descriptor))
                     (error
                      "Installed package descriptor is unreadable: %s"
                      descriptor))))
               (princ "NEOMACS-PACKAGE-CACHE:ready"))"##
    );
    let mut command = gnu_emacs.command();
    configure_process_environment(&mut command, &root, &home, &tmp);
    command.args(["--batch", "--quick", "--eval", &form]);
    let output =
        output_with_timeout(&mut command, gnu_emacs.timeout).map_err(|error| match error {
            CommandError::Launch(error) => format!(
                "failed to launch {} for cached package `{name}` in {}: {error}",
                gnu_emacs.name,
                root.display()
            ),
            CommandError::TimedOut => format!(
                "{} cached package `{name}` timed out after {:?} in {}",
                gnu_emacs.name,
                gnu_emacs.timeout,
                root.display()
            ),
            CommandError::Capture(error) => format!(
                "failed to capture {} cached package `{name}` output: {error}",
                gnu_emacs.name
            ),
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        || !stdout.contains("NEOMACS-PACKAGE-CACHE:ready")
        || !descriptor.is_file()
    {
        return Err(format!(
            "failed to prepare cached {} package {name} {version} below {}\nstatus: {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            archive.label,
            root.display(),
            output.status.code()
        ));
    }

    let marker_tmp = root.join(format!("ready.{}.tmp", std::process::id()));
    fs::write(&marker_tmp, &expected_marker).map_err(|error| {
        format!(
            "failed to write package cache marker {}: {error}",
            marker_tmp.display()
        )
    })?;
    fs::rename(&marker_tmp, &ready_marker).map_err(|error| {
        format!(
            "failed to publish package cache marker {}: {error}",
            ready_marker.display()
        )
    })?;
    Ok(package_dir)
}

/// Build one local package transaction under `./tmp` for both editors.
///
/// GNU Emacs reads the live GNU ELPA/MELPA catalogs, verifies every selected
/// package's hard-coded version, computes the dependency closure, and
/// downloads that closure into local archives. The later oracle runs cannot
/// contact the remote archives because their `package-archives` contains only
/// these local directories.
pub fn prepare_shared_package_source(
    gnu_emacs: &EmacsRuntime,
    scenario: &PackageScenario,
) -> Result<PreparedPackageSource, String> {
    validate_versioned_scenario(scenario)?;
    let sandbox = MelpaSandbox::new(&format!("{}-shared-archive", scenario.name))?;
    let archive_root = sandbox.root().join("archive");
    let gnu_archive = archive_root.join("gnu");
    let melpa_archive = archive_root.join("melpa");
    fs::create_dir_all(&gnu_archive)
        .and_then(|()| fs::create_dir_all(&melpa_archive))
        .map_err(|error| {
            format!(
                "failed to create shared package archives below {}: {error}",
                archive_root.display()
            )
        })?;

    let requested = scenario
        .package_names()
        .iter()
        .map(|package| elisp_string(package))
        .collect::<Vec<_>>()
        .join(" ");
    let expected = scenario
        .package_pins()
        .expect("validated versioned scenario")
        .iter()
        .map(|package| {
            format!(
                "({} . {})",
                elisp_string(&package.name),
                elisp_string(&package.version)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let archive_root = format!("{}/", archive_root.display());
    let form = format!(
        r##"(progn
           (require 'package)
           (require 'url)
           (setq package-user-dir
                 (expand-file-name ".emacs.d/mirror-builder-elpa" (getenv "HOME"))
                 package-check-signature nil)
           (let* ((remote-archives
                   '(("gnu" . "https://elpa.gnu.org/packages/")
                     ("melpa" . "https://melpa.org/packages/")))
                  (mirror-root {})
                  (local-archives
                   (mapcar
                    (lambda (archive)
                      (let ((directory
                             (expand-file-name
                              (concat (car archive) "/")
                              mirror-root)))
                        (make-directory directory t)
                        (url-copy-file
                         (concat (cdr archive) "archive-contents")
                         (expand-file-name "archive-contents" directory)
                         t)
                        (cons (car archive) directory)))
                    remote-archives))
                  (expected '({expected}))
                  (requested
                   (mapcar #'intern '({requested}))))
             (setq package-archives local-archives)
             (package-initialize)
             (package-refresh-contents)
             (dolist (entry expected)
               (let* ((name (intern (car entry)))
                      (description
                       (cadr (assq name package-archive-contents)))
                      (actual
                       (and description
                            (package-version-join
                             (package-desc-version description)))))
                 (unless description
                   (error "package absent from current archives: %s"
                          (car entry)))
                 (unless (equal actual (cdr entry))
                   (error
                    "package version changed: %s expected %s, current %s"
                    (car entry) (cdr entry) actual))))
             (let ((transaction
                    (package-compute-transaction
                     nil
                     (mapcar (lambda (name) (list name)) requested))))
               (dolist (description transaction)
                 (let* ((archive
                         (package-desc-archive description))
                        (remote
                         (cdr (assoc archive remote-archives)))
                        (local
                         (cdr (assoc archive local-archives)))
                        (filename
                         (concat
                          (package-desc-full-name description)
                          (package-desc-suffix description))))
                   (unless (and remote local)
                     (error "unknown package archive: %S" archive))
                   (url-copy-file
                    (concat remote filename)
                    (expand-file-name filename local)
                    t))))
             (princ "NEOMACS-MELPA-ARCHIVE:ready")))"##,
        elisp_string(&archive_root)
    );
    let report = run_phase(
        gnu_emacs,
        &sandbox,
        &scenario.name,
        ScenarioPhase::PrepareArchive,
        &form,
    )?;
    if !report.stdout.contains("NEOMACS-MELPA-ARCHIVE:ready") {
        return Err(format!(
            "{} scenario `{}` did not finish shared archive preparation\nstdout:\n{}\nstderr:\n{}",
            gnu_emacs.name, scenario.name, report.stdout, report.stderr
        ));
    }

    Ok(PreparedPackageSource {
        _sandbox: sandbox,
        source: PackageSource::local([("gnu", gnu_archive), ("melpa", melpa_archive)]),
    })
}

fn validate_versioned_scenario(scenario: &PackageScenario) -> Result<(), String> {
    let Some(packages) = scenario.package_pins() else {
        let package = scenario
            .package_names()
            .into_iter()
            .next()
            .unwrap_or("<missing package>");
        return Err(format!(
            "shared package scenario `{}` must hard-code exactly one version for `{package}`",
            scenario.name
        ));
    };
    for (index, package) in packages.iter().enumerate() {
        if package.name.is_empty()
            || package.version.is_empty()
            || packages[..index]
                .iter()
                .any(|earlier| earlier.name == package.name)
        {
            return Err(format!(
                "shared package scenario `{}` must hard-code exactly one version for `{}`",
                scenario.name, package.name
            ));
        }
    }
    Ok(())
}

/// Run the same package lifecycle and probe against GNU Emacs and Neomacs.
///
/// The editors receive separate homes but the same package source and probe.
/// Package/version graph differences and normalized value/signal differences
/// are both oracle failures.
pub fn run_oracle_scenario(
    neomacs: &EmacsRuntime,
    gnu_emacs: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<OracleScenarioReport, String> {
    let gnu_report = run_scenario(gnu_emacs, source, scenario)
        .map_err(|error| format!("GNU Emacs baseline failed: {error}"))?;
    let neomacs_report = run_scenario(neomacs, source, scenario)
        .map_err(|error| format!("Neomacs comparison failed: {error}"))?;

    if neomacs_report.installed_packages != gnu_report.installed_packages {
        return Err(format!(
            "package graph mismatch for scenario `{}`\n  Neomacs: {}\n  GNU Emacs: {}",
            scenario.name,
            format_installed_packages(&neomacs_report.installed_packages),
            format_installed_packages(&gnu_report.installed_packages)
        ));
    }
    if neomacs_report.outcome != gnu_report.outcome {
        return Err(format!(
            "oracle outcome mismatch for scenario `{}`\n  Neomacs: {}\n  GNU Emacs: {}",
            scenario.name, neomacs_report.outcome, gnu_report.outcome
        ));
    }

    Ok(OracleScenarioReport {
        neomacs: neomacs_report,
        gnu_emacs: gnu_report,
    })
}

/// Run the same setup and Elisp form in isolated GNU Emacs and Neomacs
/// processes without installing a package.
///
/// This is useful for dense behavioral corpora that load one previously
/// prepared package source while the package lifecycle remains covered by a
/// separate scenario.
pub fn run_elisp_oracle(
    neomacs: &EmacsRuntime,
    gnu_emacs: &EmacsRuntime,
    name: &str,
    setup: &str,
    probe: &str,
) -> Result<ElispOracleReport, String> {
    fn evaluate(
        runtime: &EmacsRuntime,
        name: &str,
        setup: &str,
        probe: &str,
    ) -> Result<EvalOutcome, String> {
        let sandbox = MelpaSandbox::new(name)?;
        let form = wrap_elisp_outcome(setup, probe, OUTCOME_MARKER);
        let phase = run_outcome_phase(runtime, &sandbox, name, ScenarioPhase::RestartProbe, &form)?;
        extract_marked_outcome(&phase.stdout, OUTCOME_MARKER).map_err(|error| {
            format!(
                "{} direct oracle `{name}` emitted an invalid outcome: {error}\nstdout:\n{}\nstderr:\n{}",
                runtime.name, phase.stdout, phase.stderr
            )
        })
    }

    let gnu_outcome = evaluate(gnu_emacs, name, setup, probe)
        .map_err(|error| format!("GNU Emacs baseline failed: {error}"))?;
    let neomacs_outcome = evaluate(neomacs, name, setup, probe)
        .map_err(|error| format!("Neomacs comparison failed: {error}"))?;
    if neomacs_outcome != gnu_outcome {
        return Err(format!(
            "oracle outcome mismatch for direct form `{name}`\n  Neomacs: {neomacs_outcome}\n  GNU Emacs: {gnu_outcome}"
        ));
    }

    Ok(ElispOracleReport {
        neomacs: neomacs_outcome,
        gnu_emacs: gnu_outcome,
    })
}

/// Install packages, generate `package-quickstart-file`, then load that file
/// and probe package activation in a fresh editor process.
pub fn run_quickstart_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<ScenarioReport, String> {
    let quickstart_setup = r##"
           (setq package-quickstart t
                 package-quickstart-file
                 (expand-file-name ".emacs.d/package-quickstart.el"
                                   (getenv "HOME")))
           (package-quickstart-refresh)
           (unless (file-exists-p package-quickstart-file)
             (error "package quickstart file was not generated"))"##;
    let quickstart_probe = format!(
        r##"(progn
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                 package-quickstart t
                 package-quickstart-file
                 (expand-file-name ".emacs.d/package-quickstart.el"
                                   (getenv "HOME")))
           (load package-quickstart-file nil nil t)
           {})"##,
        scenario.probe
    );
    run_install_and_probe(
        runtime,
        scenario,
        install_form(source, &scenario.package_names(), quickstart_setup),
        wrap_elisp_outcome("", &quickstart_probe, OUTCOME_MARKER),
        ScenarioPhase::QuickstartProbe,
    )
}

/// Install packages, delete one archive package, then verify the resulting
/// package state in a fresh editor process.
pub fn run_delete_and_probe_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
    package_to_delete: &str,
) -> Result<ScenarioReport, String> {
    let delete_setup = format!(
        r##"
           (let* ((name (intern {}))
                  (description (cadr (assq name package-alist))))
             (unless description
               (error "package selected for deletion was not installed"))
             (package-delete description t)
             (when (package-installed-p name)
               (error "archive package remained installed after delete")))"##,
        elisp_string(package_to_delete)
    );
    run_install_and_probe(
        runtime,
        scenario,
        install_form(source, &scenario.package_names(), &delete_setup),
        probe_form(&scenario.probe),
        ScenarioPhase::RestartProbe,
    )
}

/// Exercise `package-vc` against a local Git repository through install,
/// restart, upgrade, delete, and restart-after-delete.
pub fn run_package_vc_lifecycle(runtime: &EmacsRuntime) -> Result<PackageVcReport, String> {
    let scenario_name = "offline-package-vc-lifecycle";
    let sandbox = MelpaSandbox::new(scenario_name)?;
    let repository = sandbox.root().join("neo-vc-fixture-remote");
    fs::create_dir_all(&repository).map_err(|error| {
        format!(
            "failed to create package-vc fixture repository {}: {error}",
            repository.display()
        )
    })?;
    let fixture_root = workspace_root().join("neomacs-melpa-tests/fixtures/package-vc");
    let package_file = repository.join("neo-vc-fixture.el");
    fs::copy(fixture_root.join("neo-vc-fixture-v1.el"), &package_file)
        .map_err(|error| format!("failed to seed package-vc v1 fixture: {error}"))?;
    initialize_git_fixture(&sandbox, &repository)?;

    let repository_string = elisp_string(&repository.to_string_lossy());
    let package_setup = r##"
           (require 'package)
           (require 'package-vc)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                 package-archives nil
                 package-vc--archive-data-alist '((offline)))
           (package-initialize)"##;
    let install_form = format!(
        r##"(progn
           {package_setup}
           (package-vc-install
            '(neo-vc-fixture :url {repository_string} :vc-backend Git))
           (let* ((description (cadr (assq 'neo-vc-fixture package-alist)))
                  (directory (and description (package-desc-dir description)))
                  (bytecode (and directory
                                 (expand-file-name "neo-vc-fixture.elc" directory))))
             (unless (and description bytecode (file-exists-p bytecode))
               (error "package-vc did not install and compile v1")))
           (princ "{RESULT_MARKER}installed-v1"))"##
    );
    let restart_v1_form = format!(
        r##"(progn
           {package_setup}
           (unless (and (package-installed-p 'neo-vc-fixture)
                        (fboundp 'neo-vc-fixture-version)
                        (string= (neo-vc-fixture-version) "v1"))
             (error "package-vc v1 did not survive restart"))
           (princ "{RESULT_MARKER}restarted-v1"))"##
    );

    let mut progress = PackageVcProgress::with_capacity(5);
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcInstall,
        &install_form,
        &mut progress,
    )?;
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcRestart,
        &restart_v1_form,
        &mut progress,
    )?;

    fs::copy(fixture_root.join("neo-vc-fixture-v2.el"), &package_file)
        .map_err(|error| format!("failed to update package-vc v2 fixture: {error}"))?;
    git(&sandbox, &repository, ["add", "neo-vc-fixture.el"])?;
    git(&sandbox, &repository, ["commit", "-m", "fixture v2"])?;

    let upgrade_form = format!(
        r##"(progn
           {package_setup}
           (package-vc-upgrade
            (cadr (assq 'neo-vc-fixture package-alist)))
           (let ((deadline (+ (float-time) 30)))
             (while (and
                     (not
                      (equal
                       (package-desc-version
                        (cadr (assq 'neo-vc-fixture package-alist)))
                       '(2 0)))
                     (< (float-time) deadline))
               (accept-process-output nil 0.05)))
           (unless (equal
                    (package-desc-version
                     (cadr (assq 'neo-vc-fixture package-alist)))
                    '(2 0))
             (error "package-vc upgrade did not install v2"))
           (princ "{RESULT_MARKER}upgraded-v2"))"##
    );
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcUpgrade,
        &upgrade_form,
        &mut progress,
    )?;

    let delete_form = format!(
        r##"(progn
           {package_setup}
           (unless (and (fboundp 'neo-vc-fixture-version)
                        (string= (neo-vc-fixture-version) "v2"))
             (error "package-vc v2 did not survive restart"))
           (package-delete (cadr (assq 'neo-vc-fixture package-alist)) t)
           (when (package-installed-p 'neo-vc-fixture)
             (error "package-vc package remained installed after delete"))
           (princ "{RESULT_MARKER}deleted"))"##
    );
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcDelete,
        &delete_form,
        &mut progress,
    )?;

    let absent_form = format!(
        r##"(progn
           {package_setup}
           (when (or (package-installed-p 'neo-vc-fixture)
                     (fboundp 'neo-vc-fixture-version))
             (error "deleted package-vc package reappeared after restart"))
           (princ "{RESULT_MARKER}absent-after-restart"))"##
    );
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcRestartAfterDelete,
        &absent_form,
        &mut progress,
    )?;

    Ok(PackageVcReport {
        runtime: runtime.name.clone(),
        phases: progress.phases,
        checkpoints: progress.checkpoints,
    })
}

fn initialize_git_fixture(sandbox: &MelpaSandbox, repository: &Path) -> Result<(), String> {
    git(sandbox, repository, ["init", "--initial-branch=main"])?;
    git(
        sandbox,
        repository,
        ["config", "user.email", "melpa-test@example.invalid"],
    )?;
    git(sandbox, repository, ["config", "user.name", "MELPA Test"])?;
    git(sandbox, repository, ["add", "neo-vc-fixture.el"])?;
    git(sandbox, repository, ["commit", "-m", "fixture v1"])
}

fn git<const N: usize>(
    sandbox: &MelpaSandbox,
    repository: &Path,
    args: [&str; N],
) -> Result<(), String> {
    let mut command = Command::new("git");
    sandbox.configure(&mut command);
    let output = command
        .current_dir(repository)
        .args(args)
        .output()
        .map_err(|error| format!("failed to launch git in {}: {error}", repository.display()))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "git failed in {} (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        repository.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn run_checkpoint(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
    progress: &mut PackageVcProgress,
) -> Result<(), String> {
    let report = run_phase(runtime, sandbox, scenario_name, phase, form)?;
    let checkpoint = extract_marker(&report.stdout, RESULT_MARKER).ok_or_else(|| {
        format!(
            "{} scenario `{scenario_name}` did not emit `{RESULT_MARKER}` during {phase:?}\nstdout:\n{}\nstderr:\n{}",
            runtime.name, report.stdout, report.stderr
        )
    })?;
    progress.phases.push(report);
    progress.checkpoints.push(checkpoint);
    Ok(())
}

fn run_install_and_probe(
    runtime: &EmacsRuntime,
    scenario: &PackageScenario,
    install_form: String,
    probe_form: String,
    probe_phase: ScenarioPhase,
) -> Result<ScenarioReport, String> {
    let sandbox = MelpaSandbox::new(&scenario.name)?;
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
    if let Some(expected_packages) = scenario.package_pins() {
        for expected in expected_packages {
            let actual = installed_packages
                .iter()
                .find(|installed| installed.name == expected.name);
            if actual.map(|installed| installed.version.as_str()) != Some(expected.version.as_str())
            {
                return Err(format!(
                    "{} scenario `{}` installed an unexpected version of `{}`: expected {}, got {}",
                    runtime.name,
                    scenario.name,
                    expected.name,
                    expected.version,
                    actual
                        .map(|installed| installed.version.as_str())
                        .unwrap_or("<not installed>")
                ));
            }
        }
    }
    phases.push(install);

    let probe = run_outcome_phase(runtime, &sandbox, &scenario.name, probe_phase, &probe_form)
        .map_err(|error| {
            format!(
                "{error}\ninstalled packages: {}",
                format_installed_packages(&installed_packages)
            )
        })?;
    let outcome = extract_marked_outcome(&probe.stdout, OUTCOME_MARKER).map_err(|error| {
        format!(
            "{} scenario `{}` emitted an invalid oracle outcome during {probe_phase:?}: {error}\ninstalled packages: {}\nstdout:\n{}\nstderr:\n{}",
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
        outcome,
    })
}

fn run_phase(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
) -> Result<PhaseReport, String> {
    run_phase_with_validation(runtime, sandbox, scenario_name, phase, form, true)
}

fn run_outcome_phase(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
) -> Result<PhaseReport, String> {
    run_phase_with_validation(runtime, sandbox, scenario_name, phase, form, false)
}

fn run_phase_with_validation(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
    check_editor_error_output: bool,
) -> Result<PhaseReport, String> {
    let mut command = runtime.command();
    sandbox.configure(&mut command);
    command
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "--quick", "--eval", form]);
    let started = Instant::now();
    let output = output_with_timeout(&mut command, runtime.timeout)
        .map_err(|error| command_error_message(error, runtime, sandbox, scenario_name, phase))?;
    let report = phase_report(phase, started.elapsed(), output);
    if report.status_code != Some(0) {
        return Err(format!(
            "{} scenario `{scenario_name}` failed during {phase:?} (exit {:?})\nstdout:\n{}\nstderr:\n{}",
            runtime.name, report.status_code, report.stdout, report.stderr
        ));
    }
    if check_editor_error_output {
        check_error_markers(&report.stdout, &report.stderr).map_err(|error| {
            format!(
                "{} scenario `{scenario_name}` failed during {phase:?}: {error}",
                runtime.name
            )
        })?;
    }
    Ok(report)
}

fn command_error_message(
    error: CommandError,
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
) -> String {
    match error {
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
    }
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

fn extract_ert_summary(stdout: &str, stderr: &str) -> Option<ErtSummary> {
    stdout
        .lines()
        .chain(stderr.lines())
        .filter_map(parse_ert_summary_line)
        .next_back()
}

fn parse_ert_summary_line(line: &str) -> Option<ErtSummary> {
    let fields = line
        .trim()
        .trim_end_matches(')')
        .split_once("Ran ")?
        .1
        .split_whitespace()
        .map(|field| field.trim_end_matches(','))
        .collect::<Vec<_>>();
    if fields.get(1) != Some(&"tests") || fields.get(3..6) != Some(&["results", "as", "expected"]) {
        return None;
    }
    Some(ErtSummary {
        total: fields.first()?.parse().ok()?,
        expected: fields.get(2)?.parse().ok()?,
        unexpected: count_before(&fields, "unexpected").unwrap_or(0),
        skipped: count_before(&fields, "skipped").unwrap_or(0),
    })
}

fn count_before(fields: &[&str], label: &str) -> Option<usize> {
    let index = fields.iter().position(|field| *field == label)?;
    fields.get(index.checked_sub(1)?)?.parse().ok()
}

fn install_form(source: &PackageSource, packages: &[&str], post_install: &str) -> String {
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
        installs,
        post_install
    )
}

fn probe_form(probe: &str) -> String {
    let setup = r##"
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
           (package-initialize)"##;
    wrap_elisp_outcome(setup, probe, OUTCOME_MARKER)
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

#[cfg(test)]
mod parity_tests;
