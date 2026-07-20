//! Per-case filesystem and subprocess configuration for oracle evaluations.
//!
//! `OracleSandbox` is the single seam through which both GNU Emacs and
//! Neomacs receive their form, load roots, scratch directory, and explicit
//! environment overrides. Keeping that setup here prevents the two engine
//! runners from drifting apart.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{NamedTempFile, TempDir};

/// Locale used when recording the checked-in GNU Emacs expectations.
///
/// Collation is locale-sensitive, so inheriting the developer's locale would
/// make snapshot mode non-deterministic. Per-case overrides are applied after
/// this default and can still select another locale explicitly.
const SNAPSHOT_LOCALE: &str = "en_US.UTF-8";

pub(crate) struct OracleSandbox {
    case_root: TempDir,
    form_path: PathBuf,
    load_root: PathBuf,
    load_files: Vec<PathBuf>,
    project_root: PathBuf,
    expose_case_root: bool,
    use_case_working_dir: bool,
    extra_env: Vec<(OsString, OsString)>,
}

impl OracleSandbox {
    pub(crate) fn new(form: &str, load_files: &[&str], load_root: &Path) -> Result<Self, String> {
        let project_root = project_root();
        let case_root = create_case_tempdir_in(&project_root)?;
        let form_path = write_form_file(case_root.path(), form)?;
        let load_files = load_files
            .iter()
            .map(|file| load_root.join(file))
            .collect::<Vec<_>>();

        Ok(Self {
            case_root,
            form_path,
            load_root: load_root.to_path_buf(),
            load_files,
            project_root,
            expose_case_root: false,
            use_case_working_dir: false,
            extra_env: Vec::new(),
        })
    }

    pub(crate) fn expose_case_root_as_test_tmpdir(mut self) -> Self {
        self.expose_case_root = true;
        self
    }

    pub(crate) fn with_case_working_dir(mut self) -> Self {
        self.use_case_working_dir = true;
        self
    }

    pub(crate) fn with_extra_env(mut self, extra_env: &[(&str, &str)]) -> Self {
        self.extra_env.extend(
            extra_env
                .iter()
                .map(|(name, value)| (OsString::from(name), OsString::from(value))),
        );
        self
    }

    pub(crate) fn configure(&self, command: &mut Command) {
        command
            .env("LANG", SNAPSHOT_LOCALE)
            .env("LC_ALL", SNAPSHOT_LOCALE);

        for (name, value) in &self.extra_env {
            command.env(name, value);
        }

        let scratch_root = self.case_root.path();
        let session_tmpdir = self
            .extra_env
            .iter()
            .rev()
            .find(|(name, _)| name.as_os_str() == OsStr::new("TMPDIR"))
            .map(|(_, value)| value.clone())
            .or_else(|| std::env::var_os("TMPDIR"));
        let load_files = self
            .load_files
            .iter()
            .map(|file| file.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n");
        command
            .env("NEOVM_ORACLE_FORM_FILE", &self.form_path)
            .env("NEOVM_ORACLE_LOAD_ROOT", &self.load_root)
            .env("NEOVM_ORACLE_PROJECT_ROOT", &self.project_root)
            .env("NEOVM_ORACLE_SCRATCH_ROOT", scratch_root)
            .env("NEOVM_ORACLE_LOAD_FILES", load_files);

        if let Some(session_tmpdir) = session_tmpdir {
            command.env("NEOVM_ORACLE_SESSION_TMPDIR", session_tmpdir);
        } else {
            command.env_remove("NEOVM_ORACLE_SESSION_TMPDIR");
        }

        if self.use_case_working_dir {
            command.current_dir(scratch_root);
        }

        if self.expose_case_root {
            command.env("NEOVM_ORACLE_TEST_TMPDIR", scratch_root);
        } else {
            command.env_remove("NEOVM_ORACLE_TEST_TMPDIR");
        }
    }

    pub(crate) fn create_fixture_tempdir() -> Result<TempDir, String> {
        create_case_tempdir_in(&project_root())
    }
}

pub(crate) fn project_root() -> PathBuf {
    if let Some(root) = std::env::var_os("NEXTEST_WORKSPACE_ROOT") {
        return PathBuf::from(root);
    }
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    manifest.parent().expect("project root").to_path_buf()
}

fn scratch_base(project_root: &Path) -> Result<PathBuf, String> {
    let root = project_root.join("tmp/oracle");
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create oracle scratch directory {}: {error}",
            root.display()
        )
    })?;
    ensure_dir_locals_barrier(&root)?;
    Ok(root)
}

fn ensure_dir_locals_barrier(scratch_root: &Path) -> Result<(), String> {
    let barrier = scratch_root.join(".dir-locals.el");
    if barrier.is_file() {
        return Ok(());
    }

    let mut staged = NamedTempFile::new_in(scratch_root)
        .map_err(|error| format!("failed to stage oracle dir-locals barrier: {error}"))?;
    staged
        .write_all(b"((nil . nil))\n")
        .and_then(|()| staged.flush())
        .map_err(|error| format!("failed to write oracle dir-locals barrier: {error}"))?;
    match staged.persist_noclobber(&barrier) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(format!(
            "failed to install oracle dir-locals barrier {}: {}",
            barrier.display(),
            error.error
        )),
    }
}

fn create_case_tempdir_in(project_root: &Path) -> Result<TempDir, String> {
    let scratch_base = scratch_base(project_root)?;
    tempfile::Builder::new()
        // Keep this deliberately short: some cases create relative Unix-domain
        // socket names inside this directory.
        .prefix("case-")
        .tempdir_in(&scratch_base)
        .map_err(|error| {
            format!(
                "failed to create oracle case directory in {}: {error}",
                scratch_base.display()
            )
        })
}

fn write_form_file(case_root: &Path, form: &str) -> Result<PathBuf, String> {
    let form_path = case_root.join("form.el");
    let mut file = fs::File::create(&form_path)
        .map_err(|error| format!("failed to create oracle form file: {error}"))?;
    file.write_all(form.as_bytes())
        .map_err(|error| format!("failed to write oracle form file: {error}"))?;
    file.flush()
        .map_err(|error| format!("failed to flush oracle form file: {error}"))?;
    Ok(form_path)
}
