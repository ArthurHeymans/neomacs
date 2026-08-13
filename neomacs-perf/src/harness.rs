use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use neomacs_melpa_test_support::{
    CommandError, EmacsRuntime, MelpaSandbox, PreparedPackageSet, locked_melpa_sources,
    output_with_timeout, prepare_cached_tree_sitter_grammar,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    ArtifactFile, ArtifactKind, CorrectnessMismatch, Frontend, Measurement, MetricName, MetricUnit,
    RunArtifact, RunVerdict, ScenarioId, scenario,
};

const ARTIFACT_SCHEMA_VERSION: u32 = 1;
const SCENARIO_RESULT_SCHEMA_VERSION: u32 = 1;
const RUST_LSP_TYPING_OVERLAY_COUNT: u64 = 4;
const RUST_LSP_TYPING_DIAGNOSTIC_COUNT: u64 = 4;
const RUST_GRAMMAR_REPOSITORY: &str = "https://github.com/tree-sitter/tree-sitter-rust";
const RUST_GRAMMAR_REVISION: &str = "18b0515fca567f5a10aee9978c6d2640e878671a";
static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRequest {
    scenario: ScenarioId,
    editor: PathBuf,
    iterations: NonZeroU32,
    frontend: Option<Frontend>,
    timeout: Duration,
}

impl RunRequest {
    pub fn new(scenario: ScenarioId, editor: impl Into<PathBuf>, iterations: NonZeroU32) -> Self {
        Self {
            scenario,
            editor: editor.into(),
            iterations,
            frontend: None,
            timeout: Duration::from_secs(300),
        }
    }

    pub fn with_frontend(mut self, frontend: Frontend) -> Self {
        self.frontend = Some(frontend);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub const fn scenario(&self) -> ScenarioId {
        self.scenario
    }

    pub fn editor(&self) -> &Path {
        &self.editor
    }

    pub const fn iterations(&self) -> NonZeroU32 {
        self.iterations
    }

    pub fn frontend(&self) -> Frontend {
        self.frontend.unwrap_or_else(|| {
            scenario(self.scenario)
                .expect("catalogued scenario")
                .default_frontend
        })
    }

    pub const fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunReport {
    pub artifact: RunArtifact,
    pub artifact_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum PerfError {
    #[error("failed to create performance artifact directory {path}: {source}")]
    CreateArtifactDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write performance artifact {path}: {source}")]
    WriteArtifact {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("scenario `{scenario}` emitted an invalid result document: {source}")]
    InvalidScenarioResult {
        scenario: ScenarioId,
        source: serde_json::Error,
    },
    #[error("failed to serialize performance artifact: {0}")]
    SerializeArtifact(serde_json::Error),
}

#[derive(Clone, Debug)]
pub struct PerfHarness {
    workspace_root: PathBuf,
}

impl PerfHarness {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn run(&self, request: &RunRequest) -> Result<RunReport, PerfError> {
        let context = RunContext::create(&self.workspace_root, request)?;
        if !request.editor.is_file() {
            return context.infrastructure_failure(
                format!("missing editor executable {}", request.editor.display()),
                Vec::new(),
            );
        }

        self.run_prepared_scenario(request, context)
    }

    /// Validate and persist a result produced by a frontend adapter.
    ///
    /// Out-of-process adapters publish the result file before the harness sees
    /// it. Keeping validation and artifact publication together prevents an
    /// adapter from turning a mismatch into a valid sample.
    #[cfg(test)]
    pub(crate) fn record_fixture_result(
        &self,
        request: &RunRequest,
        raw_result: &str,
    ) -> Result<RunReport, PerfError> {
        let result: RustLspTypingResult = serde_json::from_str(raw_result).map_err(|source| {
            PerfError::InvalidScenarioResult {
                scenario: request.scenario,
                source,
            }
        })?;
        let context = RunContext::create(&self.workspace_root, request)?;

        let scenario_result_path = context.directory.join("scenario-result.json");
        fs::write(&scenario_result_path, raw_result).map_err(|source| {
            PerfError::WriteArtifact {
                path: scenario_result_path.clone(),
                source,
            }
        })?;

        let verdict = result_verdict(request, &result, u128::from(result.elapsed_us));
        context.publish(
            u128::from(result.elapsed_us),
            verdict,
            vec![ArtifactFile {
                kind: ArtifactKind::ScenarioResult,
                path: PathBuf::from("scenario-result.json"),
            }],
        )
    }

    fn run_prepared_scenario(
        &self,
        request: &RunRequest,
        context: RunContext,
    ) -> Result<RunReport, PerfError> {
        let mut files = Vec::new();
        let prepared = match self.prepare_rust_lsp_typing(request, &context.directory) {
            Ok(prepared) => prepared,
            Err(message) => {
                return context.infrastructure_failure(message, files);
            }
        };
        files.extend(prepared.input_artifacts());

        let mut command = frontend_command(request, &self.workspace_root, &prepared);
        let process_started = Instant::now();
        let output = match output_with_timeout(&mut command, request.timeout) {
            Ok(output) => output,
            Err(error) => {
                let (message, output) = command_error_details(error, request.timeout);
                if let Some(output) = output {
                    files.extend(write_process_output(&context.directory, &output)?);
                }
                files.extend(frontend_artifacts_if_present(&prepared));
                return context.infrastructure_failure(message, files);
            }
        };
        let process_wall_us = process_started.elapsed().as_micros();
        files.extend(write_process_output(&context.directory, &output)?);
        files.extend(frontend_artifacts_if_present(&prepared));

        if !output.status.success() {
            return context.infrastructure_failure(
                format!(
                    "{} adapter exited with status {}",
                    frontend_name(request.frontend()),
                    output.status
                ),
                files,
            );
        }
        if !prepared.sentinel.is_file() {
            return context.infrastructure_failure(
                "scenario process exited without publishing its completion sentinel".to_string(),
                files,
            );
        }
        let raw_result = match fs::read_to_string(&prepared.result) {
            Ok(result) => result,
            Err(error) => {
                return context.infrastructure_failure(
                    format!(
                        "completed scenario did not publish a readable result {}: {error}",
                        prepared.result.display()
                    ),
                    files,
                );
            }
        };
        files.push(ArtifactFile {
            kind: ArtifactKind::ScenarioResult,
            path: relative_artifact_path(&prepared.result),
        });
        let result: RustLspTypingResult = match serde_json::from_str(&raw_result) {
            Ok(result) => result,
            Err(error) => {
                return context.infrastructure_failure(
                    format!("scenario emitted invalid result JSON: {error}"),
                    files,
                );
            }
        };
        let verdict = result_verdict(request, &result, process_wall_us);
        context.publish(context.elapsed_us(), verdict, files)
    }

    fn prepare_rust_lsp_typing(
        &self,
        request: &RunRequest,
        run_directory: &Path,
    ) -> Result<PreparedScenario, String> {
        let lsp_mode_source = locked_melpa_sources()?
            .into_iter()
            .find(|source| source.package().0 == "lsp-mode")
            .ok_or_else(|| "the MELPA source lock does not contain lsp-mode".to_string())?;
        let lsp_mode = lsp_mode_source.package();
        let packages = PreparedPackageSet::from_locked_melpa(
            &EmacsRuntime::gnu_emacs(),
            lsp_mode,
            "lsp-mode.el",
        )?;
        let cached_grammar = prepare_cached_tree_sitter_grammar(
            &EmacsRuntime::gnu_emacs(),
            "rust",
            RUST_GRAMMAR_REPOSITORY,
            RUST_GRAMMAR_REVISION,
        )?;
        let grammar_directory = run_directory.join("tree-sitter");
        let grammar_libraries =
            copy_grammar_libraries(&cached_grammar, &grammar_directory, "tree-sitter-rust")?;
        let sandbox = MelpaSandbox::new(&format!("perf-{}", request.scenario))?;
        let editor = collect_editor_provenance(request.editor(), &sandbox)?;
        let startup = packages.write_startup_file(run_directory)?;
        let fixture_root = self.workspace_root.join("neomacs-perf/fixtures");
        let fixture_source = fixture_root.join("rust-lsp-typing.el");
        let source_source = fixture_root.join("rust-lsp-typing.rs");
        let replay_source = fixture_root.join("rust-lsp-diagnostics.json");
        for required in [&fixture_source, &source_source, &replay_source] {
            if !required.is_file() {
                return Err(format!(
                    "missing committed performance fixture {}",
                    required.display()
                ));
            }
        }
        let fixture = run_directory.join("rust-lsp-typing.el");
        let source = run_directory.join("rust-lsp-typing.rs");
        let replay = run_directory.join("rust-lsp-diagnostics.json");
        for (input, output) in [
            (&fixture_source, &fixture),
            (&source_source, &source),
            (&replay_source, &replay),
        ] {
            fs::copy(input, output).map_err(|error| {
                format!(
                    "failed to copy performance fixture {} to {}: {error}",
                    input.display(),
                    output.display()
                )
            })?;
        }
        let provenance = run_directory.join("input-provenance.json");
        let provenance_manifest = InputProvenanceManifest {
            lsp_mode: PackageProvenance {
                name: lsp_mode.0,
                version: lsp_mode.1,
                repository: lsp_mode_source.repository(),
                revision: lsp_mode_source.revision(),
                upstream_repository: lsp_mode_source.upstream_repository(),
                upstream_revision: lsp_mode_source.upstream_revision(),
            },
            tree_sitter_grammar: GrammarProvenance {
                language: "rust",
                repository: RUST_GRAMMAR_REPOSITORY,
                revision: RUST_GRAMMAR_REVISION,
            },
            editor,
            workload_source: "neomacs-perf/fixtures/rust-lsp-typing.rs",
            workload_source_sha256: sha256_file(&source_source)?,
            environment_policy: "closed-v1",
            passthrough_environment: benchmark_passthrough_environment()
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string_lossy().into_owned()))
                .collect(),
        };
        let provenance_json = serde_json::to_vec_pretty(&provenance_manifest)
            .map_err(|error| format!("failed to serialize input provenance: {error}"))?;
        fs::write(&provenance, provenance_json).map_err(|error| {
            format!(
                "failed to write input provenance {}: {error}",
                provenance.display()
            )
        })?;
        let gui_runtime_directory = self
            .workspace_root
            .join("tmp/gui-runtime")
            .join(std::process::id().to_string());
        fs::create_dir_all(&gui_runtime_directory).map_err(|error| {
            format!(
                "failed to create short GUI runtime directory {}: {error}",
                gui_runtime_directory.display()
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&gui_runtime_directory, fs::Permissions::from_mode(0o700))
                .map_err(|error| {
                    format!(
                        "failed to secure GUI runtime directory {}: {error}",
                        gui_runtime_directory.display()
                    )
                })?;
        }
        Ok(PreparedScenario {
            startup,
            fixture,
            source,
            replay,
            provenance,
            result: run_directory.join("scenario-result.json"),
            sentinel: run_directory.join("completed"),
            terminal_bytes: run_directory.join("terminal.ansi"),
            gui_app_log: run_directory.join("gui-app.log"),
            gui_weston_log: run_directory.join("weston.log"),
            gui_runtime_directory,
            grammar_directory,
            grammar_libraries,
            packages,
            sandbox,
        })
    }
}

struct RunContext {
    request: RunRequest,
    run_id: String,
    started_unix_ms: u128,
    started: Instant,
    directory: PathBuf,
}

impl RunContext {
    fn create(workspace_root: &Path, request: &RunRequest) -> Result<Self, PerfError> {
        let started = Instant::now();
        let started_unix_ms = unix_time_ms();
        let run_id = next_run_id(request.scenario, started_unix_ms);
        let directory = workspace_root.join("tmp/perf").join(&run_id);
        fs::create_dir_all(&directory).map_err(|source| PerfError::CreateArtifactDirectory {
            path: directory.clone(),
            source,
        })?;
        Ok(Self {
            request: request.clone(),
            run_id,
            started_unix_ms,
            started,
            directory,
        })
    }

    fn elapsed_us(&self) -> u128 {
        self.started.elapsed().as_micros()
    }

    fn infrastructure_failure(
        &self,
        message: String,
        files: Vec<ArtifactFile>,
    ) -> Result<RunReport, PerfError> {
        self.publish(
            self.elapsed_us(),
            RunVerdict::InfrastructureFailure { message },
            files,
        )
    }

    fn publish(
        &self,
        total_elapsed_us: u128,
        verdict: RunVerdict,
        files: Vec<ArtifactFile>,
    ) -> Result<RunReport, PerfError> {
        let artifact = RunArtifact {
            schema_version: ARTIFACT_SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            scenario: self.request.scenario,
            frontend: self.request.frontend(),
            editor: self.request.editor.clone(),
            iterations: self.request.iterations.get(),
            started_unix_ms: self.started_unix_ms,
            total_elapsed_us,
            verdict,
            files,
        };
        let artifact_path = self.directory.join("artifact.json");
        write_json_atomically(&artifact_path, &artifact)?;
        Ok(RunReport {
            artifact,
            artifact_path,
        })
    }
}

struct PreparedScenario {
    startup: PathBuf,
    fixture: PathBuf,
    source: PathBuf,
    replay: PathBuf,
    provenance: PathBuf,
    result: PathBuf,
    sentinel: PathBuf,
    terminal_bytes: PathBuf,
    gui_app_log: PathBuf,
    gui_weston_log: PathBuf,
    gui_runtime_directory: PathBuf,
    grammar_directory: PathBuf,
    grammar_libraries: Vec<PathBuf>,
    packages: PreparedPackageSet,
    sandbox: MelpaSandbox,
}

impl PreparedScenario {
    fn input_artifacts(&self) -> Vec<ArtifactFile> {
        let mut artifacts = [
            (ArtifactKind::PackageStartup, &self.startup),
            (ArtifactKind::ScenarioFixture, &self.fixture),
            (ArtifactKind::SourceFixture, &self.source),
            (ArtifactKind::LspReplay, &self.replay),
            (ArtifactKind::InputProvenance, &self.provenance),
        ]
        .into_iter()
        .map(|(kind, path)| ArtifactFile {
            kind,
            path: relative_artifact_path(path),
        })
        .collect::<Vec<_>>();
        artifacts.extend(self.grammar_libraries.iter().map(|path| {
            ArtifactFile {
                kind: ArtifactKind::TreeSitterGrammar,
                path: PathBuf::from("tree-sitter").join(
                    path.file_name()
                        .expect("copied grammar library has a file name"),
                ),
            }
        }));
        artifacts
    }
}

fn frontend_command(
    request: &RunRequest,
    workspace_root: &Path,
    prepared: &PreparedScenario,
) -> Command {
    let frontend = request.frontend();
    let mut command = match frontend {
        Frontend::Batch => {
            let mut command = Command::new(request.editor());
            command.arg("--batch").arg("-Q");
            command
        }
        Frontend::Tui { .. } => {
            let mut command = Command::new("python3");
            command
                .arg(workspace_root.join("tools/bench/pty-run.py"))
                .arg(request.editor())
                .arg("-nw")
                .arg("-Q");
            command
        }
        Frontend::Gui { .. } => {
            let mut command = Command::new(workspace_root.join("tools/bench/gui-run.sh"));
            command.arg(request.editor()).arg("-Q");
            command
        }
    };
    configure_benchmark_environment(&mut command, &prepared.sandbox);
    match frontend {
        Frontend::Batch => {}
        Frontend::Tui { rows, columns } => {
            command
                .env("PTY_ROWS", rows.to_string())
                .env("PTY_COLS", columns.to_string())
                .env("PTY_TIMEOUT", request.timeout().as_secs().to_string())
                .env("PTY_OUTPUT", &prepared.terminal_bytes)
                // The package sandbox deliberately defaults to TERM=dumb for
                // batch tests. A real PTY owns its display capabilities.
                .env("TERM", "screen-256color");
        }
        Frontend::Gui { width, height } => {
            command
                .env("GUI_WIDTH", width.to_string())
                .env("GUI_HEIGHT", height.to_string())
                .env("GUI_TIMEOUT", request.timeout().as_secs().to_string())
                .env("GUI_APP_LOG", &prepared.gui_app_log)
                .env("GUI_WESTON_LOG", &prepared.gui_weston_log)
                .env("XDG_RUNTIME_DIR", &prepared.gui_runtime_directory);
        }
    }
    command
        .arg("--load")
        .arg(&prepared.startup)
        .arg("--load")
        .arg(&prepared.fixture)
        .current_dir(workspace_root);
    command
        .envs(prepared.packages.process_environment())
        .env_remove("EMACSLOADPATH")
        .env("SENTINEL", &prepared.sentinel)
        .env("NEOMACS_PERF_RESULT", &prepared.result)
        .env("NEOMACS_PERF_SOURCE", &prepared.source)
        .env("NEOMACS_PERF_LSP_REPLAY", &prepared.replay)
        .env("NEOMACS_PERF_TREE_SITTER_DIR", &prepared.grammar_directory)
        .env(
            "NEOMACS_PERF_ITERATIONS",
            request.iterations().get().to_string(),
        );
    command
}

const BENCHMARK_PASSTHROUGH_ENVIRONMENT: &[&str] = &[
    "PATH",
    "LD_LIBRARY_PATH",
    "DYLD_LIBRARY_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "SYSTEMROOT",
    "WINDIR",
];

pub(crate) fn configure_benchmark_environment(command: &mut Command, sandbox: &MelpaSandbox) {
    command.env_clear();
    command.envs(benchmark_passthrough_environment());
    command.envs(sandbox.process_environment());
}

fn benchmark_passthrough_environment() -> Vec<(&'static str, std::ffi::OsString)> {
    BENCHMARK_PASSTHROUGH_ENVIRONMENT
        .iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (*name, value)))
        .collect()
}

pub(crate) fn collect_editor_provenance(
    editor: &Path,
    sandbox: &MelpaSandbox,
) -> Result<EditorProvenance, String> {
    let metadata = fs::metadata(editor)
        .map_err(|error| format!("failed to inspect editor {}: {error}", editor.display()))?;
    let canonical_path = fs::canonicalize(editor).map_err(|error| {
        format!(
            "failed to resolve editor executable {}: {error}",
            editor.display()
        )
    })?;
    Ok(EditorProvenance {
        path: canonical_path.to_string_lossy().into_owned(),
        executable_sha256: sha256_file(editor)?,
        executable_size_bytes: metadata.len(),
        pdump_fingerprint: editor_identity_value(editor, "--fingerprint", sandbox)?,
        version: editor_identity_value(editor, "--version", sandbox)?,
    })
}

fn editor_identity_value(
    editor: &Path,
    argument: &str,
    sandbox: &MelpaSandbox,
) -> Result<String, String> {
    let mut command = Command::new(editor);
    configure_benchmark_environment(&mut command, sandbox);
    command.arg(argument);
    let output = command.output().map_err(|error| {
        format!(
            "failed to query editor {} with {argument}: {error}",
            editor.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "editor {} {argument} exited with {}: {}",
            editor.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value = String::from_utf8(output.stdout)
        .map_err(|error| format!("editor {argument} output was not UTF-8: {error}"))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(format!(
            "editor {} {argument} returned an empty identity",
            editor.display()
        ));
    }
    Ok(value.to_string())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(hex)
}

fn copy_grammar_libraries(
    cached_directory: &Path,
    run_directory: &Path,
    library_stem: &str,
) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(run_directory).map_err(|error| {
        format!(
            "failed to create run-local Tree-sitter directory {}: {error}",
            run_directory.display()
        )
    })?;
    let entries = fs::read_dir(cached_directory).map_err(|error| {
        format!(
            "failed to enumerate cached Tree-sitter directory {}: {error}",
            cached_directory.display()
        )
    })?;
    let mut copied = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read cached Tree-sitter entry below {}: {error}",
                cached_directory.display()
            )
        })?;
        if !entry.file_type().is_ok_and(|file_type| file_type.is_file())
            || !entry.file_name().to_string_lossy().contains(library_stem)
        {
            continue;
        }
        let destination = run_directory.join(entry.file_name());
        fs::copy(entry.path(), &destination).map_err(|error| {
            format!(
                "failed to copy Tree-sitter grammar {} to {}: {error}",
                entry.path().display(),
                destination.display()
            )
        })?;
        copied.push(destination);
    }
    if copied.is_empty() {
        return Err(format!(
            "cached Tree-sitter directory {} contains no `{library_stem}` library",
            cached_directory.display()
        ));
    }
    copied.sort();
    Ok(copied)
}

fn command_error_details(error: CommandError, timeout: Duration) -> (String, Option<Output>) {
    match error {
        CommandError::Launch(error) => {
            (format!("failed to launch frontend adapter: {error}"), None)
        }
        CommandError::TimedOut(output) => (
            format!("frontend adapter timed out after {timeout:?}"),
            Some(output),
        ),
        CommandError::Capture(error) => (
            format!("failed to capture frontend adapter output: {error}"),
            None,
        ),
    }
}

fn write_process_output(
    run_directory: &Path,
    output: &Output,
) -> Result<Vec<ArtifactFile>, PerfError> {
    let outputs = [
        (ArtifactKind::Stdout, "stdout.log", output.stdout.as_slice()),
        (ArtifactKind::Stderr, "stderr.log", output.stderr.as_slice()),
    ];
    let mut files = Vec::with_capacity(outputs.len());
    for (kind, name, bytes) in outputs {
        let path = run_directory.join(name);
        fs::write(&path, bytes).map_err(|source| PerfError::WriteArtifact {
            path: path.clone(),
            source,
        })?;
        files.push(ArtifactFile {
            kind,
            path: PathBuf::from(name),
        });
    }
    Ok(files)
}

fn frontend_artifacts_if_present(prepared: &PreparedScenario) -> Vec<ArtifactFile> {
    [
        (ArtifactKind::TerminalByteStream, &prepared.terminal_bytes),
        (ArtifactKind::FrontendLog, &prepared.gui_app_log),
        (ArtifactKind::CompositorLog, &prepared.gui_weston_log),
    ]
    .into_iter()
    .filter(|(_, path)| path.is_file())
    .map(|(kind, path)| ArtifactFile {
        kind,
        path: relative_artifact_path(path),
    })
    .collect()
}

fn relative_artifact_path(path: &Path) -> PathBuf {
    path.file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf())
}

fn frontend_name(frontend: Frontend) -> &'static str {
    match frontend {
        Frontend::Batch => "batch",
        Frontend::Tui { .. } => "TUI",
        Frontend::Gui { .. } => "GUI",
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "RustLspTypingResultWire")]
struct RustLspTypingResult {
    schema_version: u32,
    scenario: ScenarioId,
    outcome: ScenarioOutcome,
    iterations: u32,
    elapsed_us: u64,
    major_mode: String,
    lsp_mode_loaded: bool,
    treesit_parser_language: String,
    text_unchanged: bool,
    point_unchanged: bool,
    overlay_count: u64,
    lsp_diagnostic_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RustLspTypingResultWire {
    schema_version: u32,
    scenario: ScenarioId,
    status: ScenarioStatus,
    iterations: u32,
    elapsed_us: u64,
    major_mode: String,
    lsp_mode_loaded: bool,
    treesit_parser_language: String,
    text_unchanged: bool,
    point_unchanged: bool,
    overlay_count: u64,
    lsp_diagnostic_count: u64,
    #[serde(deserialize_with = "deserialize_optional_error", rename = "error")]
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ScenarioStatus {
    Ok,
    Error,
}

#[derive(Debug, Eq, PartialEq)]
enum ScenarioOutcome {
    Ok,
    Error(String),
}

impl std::fmt::Display for ScenarioOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => formatter.write_str("ok"),
            Self::Error(message) => write!(formatter, "error: {message}"),
        }
    }
}

impl TryFrom<RustLspTypingResultWire> for RustLspTypingResult {
    type Error = String;

    fn try_from(wire: RustLspTypingResultWire) -> Result<Self, Self::Error> {
        let outcome = match (wire.status, wire.error) {
            (ScenarioStatus::Ok, None) => ScenarioOutcome::Ok,
            (ScenarioStatus::Ok, Some(_)) => {
                return Err("status `ok` requires a null error".to_string());
            }
            (ScenarioStatus::Error, Some(message)) if !message.trim().is_empty() => {
                ScenarioOutcome::Error(message)
            }
            (ScenarioStatus::Error, Some(_)) => {
                return Err("status `error` requires a non-empty error".to_string());
            }
            (ScenarioStatus::Error, None) => {
                return Err("status `error` requires a non-null error".to_string());
            }
        };
        Ok(Self {
            schema_version: wire.schema_version,
            scenario: wire.scenario,
            outcome,
            iterations: wire.iterations,
            elapsed_us: wire.elapsed_us,
            major_mode: wire.major_mode,
            lsp_mode_loaded: wire.lsp_mode_loaded,
            treesit_parser_language: wire.treesit_parser_language,
            text_unchanged: wire.text_unchanged,
            point_unchanged: wire.point_unchanged,
            overlay_count: wire.overlay_count,
            lsp_diagnostic_count: wire.lsp_diagnostic_count,
        })
    }
}

#[derive(Serialize)]
struct InputProvenanceManifest<'a> {
    lsp_mode: PackageProvenance<'a>,
    tree_sitter_grammar: GrammarProvenance<'a>,
    editor: EditorProvenance,
    workload_source: &'a str,
    workload_source_sha256: String,
    environment_policy: &'a str,
    passthrough_environment: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub(crate) struct EditorProvenance {
    pub(crate) path: String,
    pub(crate) executable_sha256: String,
    pub(crate) executable_size_bytes: u64,
    pub(crate) pdump_fingerprint: String,
    pub(crate) version: String,
}

#[derive(Serialize)]
struct PackageProvenance<'a> {
    name: &'a str,
    version: &'a str,
    repository: &'a str,
    revision: &'a str,
    upstream_repository: &'a str,
    upstream_revision: &'a str,
}

#[derive(Serialize)]
struct GrammarProvenance<'a> {
    language: &'a str,
    repository: &'a str,
    revision: &'a str,
}

fn deserialize_optional_error<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

fn validate_rust_lsp_typing_result(
    request: &RunRequest,
    result: &RustLspTypingResult,
) -> Vec<CorrectnessMismatch> {
    let mut mismatches = Vec::new();
    mismatch(
        &mut mismatches,
        "scenario-result-schema",
        SCENARIO_RESULT_SCHEMA_VERSION,
        result.schema_version,
    );
    mismatch(
        &mut mismatches,
        "scenario-id",
        request.scenario,
        result.scenario,
    );
    mismatch(
        &mut mismatches,
        "scenario-outcome",
        &ScenarioOutcome::Ok,
        &result.outcome,
    );
    mismatch(
        &mut mismatches,
        "iterations",
        request.iterations.get(),
        result.iterations,
    );
    mismatch(
        &mut mismatches,
        "major-mode",
        "rust-ts-mode",
        result.major_mode.as_str(),
    );
    mismatch(
        &mut mismatches,
        "lsp-mode-loaded",
        true,
        result.lsp_mode_loaded,
    );
    mismatch(
        &mut mismatches,
        "treesit-parser-language",
        "rust",
        result.treesit_parser_language.as_str(),
    );
    mismatch(
        &mut mismatches,
        "final-buffer-text",
        true,
        result.text_unchanged,
    );
    mismatch(&mut mismatches, "final-point", true, result.point_unchanged);
    mismatch(
        &mut mismatches,
        "overlay-count",
        RUST_LSP_TYPING_OVERLAY_COUNT,
        result.overlay_count,
    );
    mismatch(
        &mut mismatches,
        "lsp-diagnostic-count",
        RUST_LSP_TYPING_DIAGNOSTIC_COUNT,
        result.lsp_diagnostic_count,
    );
    mismatches
}

fn result_verdict(
    request: &RunRequest,
    result: &RustLspTypingResult,
    process_wall_us: u128,
) -> RunVerdict {
    let mismatches = validate_rust_lsp_typing_result(request, result);
    if mismatches.is_empty() {
        RunVerdict::Valid {
            measurements: valid_measurements(result, process_wall_us),
        }
    } else {
        RunVerdict::CorrectnessMismatch { mismatches }
    }
}

fn mismatch<T>(mismatches: &mut Vec<CorrectnessMismatch>, invariant: &str, expected: T, actual: T)
where
    T: PartialEq + std::fmt::Display,
{
    if expected != actual {
        mismatches.push(CorrectnessMismatch {
            invariant: invariant.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
}

fn valid_measurements(result: &RustLspTypingResult, wall_elapsed_us: u128) -> Vec<Measurement> {
    let edits = u64::from(result.iterations) * 2;
    vec![
        Measurement {
            name: MetricName::ProcessWallTime,
            value: wall_elapsed_us as f64,
            unit: MetricUnit::Microseconds,
        },
        Measurement {
            name: MetricName::WorkloadCpuTime,
            value: result.elapsed_us as f64,
            unit: MetricUnit::Microseconds,
        },
        Measurement {
            name: MetricName::PerEditCpuTime,
            value: result.elapsed_us as f64 / edits.max(1) as f64,
            unit: MetricUnit::MicrosecondsPerEdit,
        },
        Measurement {
            name: MetricName::Edits,
            value: edits as f64,
            unit: MetricUnit::Count,
        },
        Measurement {
            name: MetricName::Redisplays,
            value: edits as f64,
            unit: MetricUnit::Count,
        },
        Measurement {
            name: MetricName::Iterations,
            value: f64::from(result.iterations),
            unit: MetricUnit::Count,
        },
        Measurement {
            name: MetricName::OverlayCount,
            value: result.overlay_count as f64,
            unit: MetricUnit::Count,
        },
        Measurement {
            name: MetricName::LspDiagnosticCount,
            value: result.lsp_diagnostic_count as f64,
            unit: MetricUnit::Count,
        },
    ]
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn next_run_id(scenario: ScenarioId, unix_ms: u128) -> String {
    let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{unix_ms}-{}-{sequence}",
        scenario.as_str(),
        std::process::id()
    )
}

fn write_json_atomically(path: &Path, artifact: &RunArtifact) -> Result<(), PerfError> {
    let json = serde_json::to_vec_pretty(artifact).map_err(PerfError::SerializeArtifact)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json).map_err(|source| PerfError::WriteArtifact {
        path: temporary.clone(),
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| PerfError::WriteArtifact {
        path: path.to_path_buf(),
        source,
    })
}
