//! Per-case filesystem and subprocess configuration for oracle evaluations.
//!
//! `OracleSandbox` is the single seam through which both GNU Emacs and
//! Neomacs receive their form, load roots, scratch directory, and explicit
//! environment overrides. Keeping that setup here prevents the two engine
//! runners from drifting apart.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{TempDir, TempPath};

pub(crate) struct OracleSandbox {
    form_path: TempPath,
    load_root: PathBuf,
    load_files: String,
    project_root: PathBuf,
    scratch_root: PathBuf,
    shared_tmpdir: Option<PathBuf>,
    extra_env: Vec<(OsString, OsString)>,
}

impl OracleSandbox {
    pub(crate) fn new(form: &str, load_files: &[&str], load_root: &Path) -> Result<Self, String> {
        let project_root = project_root();
        let scratch_root = scratch_root(&project_root)?;
        let form_path = write_form_file(&scratch_root, form)?;
        let load_files = load_files
            .iter()
            .map(|file| load_root.join(file).to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(Self {
            form_path,
            load_root: load_root.to_path_buf(),
            load_files,
            project_root,
            scratch_root,
            shared_tmpdir: None,
            extra_env: Vec::new(),
        })
    }

    pub(crate) fn with_shared_tmpdir(mut self, shared_tmpdir: Option<&Path>) -> Self {
        self.shared_tmpdir = shared_tmpdir.map(Path::to_path_buf);
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
            .env("NEOVM_ORACLE_FORM_FILE", self.form_path.as_os_str())
            .env("NEOVM_ORACLE_LOAD_ROOT", &self.load_root)
            .env("NEOVM_ORACLE_PROJECT_ROOT", &self.project_root)
            .env("NEOVM_ORACLE_SCRATCH_ROOT", &self.scratch_root)
            .env("NEOVM_ORACLE_LOAD_FILES", &self.load_files)
            .env("TMPDIR", &self.scratch_root);

        if let Some(shared_tmpdir) = &self.shared_tmpdir {
            command.env("NEOVM_ORACLE_TEST_TMPDIR", shared_tmpdir);
        }
        for (name, value) in &self.extra_env {
            command.env(name, value);
        }
    }

    pub(crate) fn create_case_tempdir() -> Result<TempDir, String> {
        let root = project_root();
        let scratch_root = scratch_root(&root)?;
        tempfile::Builder::new()
            // Keep this deliberately short: Unix-domain socket paths created
            // inside a case directory are limited to roughly 108 bytes.
            .prefix("case-")
            .tempdir_in(&scratch_root)
            .map_err(|error| {
                format!(
                    "failed to create oracle case directory in {}: {error}",
                    scratch_root.display()
                )
            })
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

fn scratch_root(project_root: &Path) -> Result<PathBuf, String> {
    let root = project_root.join("tmp/oracle");
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create oracle scratch directory {}: {error}",
            root.display()
        )
    })?;
    Ok(root)
}

fn write_form_file(scratch_root: &Path, form: &str) -> Result<TempPath, String> {
    let mut file = tempfile::Builder::new()
        .prefix("neovm-oracle-form-")
        .suffix(".el")
        .tempfile_in(scratch_root)
        .map_err(|error| format!("failed to create oracle form file: {error}"))?;
    file.write_all(form.as_bytes())
        .map_err(|error| format!("failed to write oracle form file: {error}"))?;
    file.flush()
        .map_err(|error| format!("failed to flush oracle form file: {error}"))?;
    Ok(file.into_temp_path())
}
