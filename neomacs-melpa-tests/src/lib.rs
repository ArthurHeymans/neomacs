//! MELPA package-install integration harness.
//!
//! Installs real packages from MELPA the way a user does — through neomacs's
//! **own `package-install`** over the network (archive refresh → dependency
//! resolution → download → autoload generation → byte-compilation →
//! activation). Nothing is fetched or staged from Rust; the whole point is to
//! exercise neomacs's real package manager end to end and confirm it behaves
//! like GNU Emacs.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// The path to the `neomacs` binary (override with `NEOMACS_BIN`).
pub fn neomacs_binary() -> PathBuf {
    std::env::var("NEOMACS_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            workspace_root()
                .join("target")
                .join("release")
                .join("neomacs")
        })
}

/// A fresh, isolated `$HOME` with an empty `~/.emacs.d` for one test, so each
/// `package-install` run starts from a clean slate (no shared package state).
pub fn fresh_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("create isolated HOME");
    std::fs::create_dir_all(home.path().join(".emacs.d")).expect("create .emacs.d");
    home
}

/// Elisp prologue that points `package.el` at GNU ELPA + MELPA and installs
/// `pkgs` through neomacs's own `package-install` (real network workflow,
/// including transitive dependency resolution and byte-compilation).
pub fn install_prologue(pkgs: &[&str]) -> String {
    let installs: String = pkgs
        .iter()
        .map(|p| format!("  (package-install '{p})\n"))
        .collect();
    format!(
        "  (require 'package)\n  \
         (setq package-archives '((\"gnu\" . \"https://elpa.gnu.org/packages/\")\n                           \
         (\"melpa\" . \"https://melpa.org/packages/\"))\n        \
         package-check-signature nil)\n  \
         (package-initialize)\n  \
         (package-refresh-contents)\n{installs}"
    )
}

/// Install `pkgs` from MELPA via neomacs's `package-install`, then evaluate
/// `usage`. The `usage` form should `(error ...)` on a failed check and end by
/// `princ`-ing a success marker the caller asserts on. Returns stdout, or an
/// error if neomacs exited non-zero (an `(error ...)` in `usage`, a failed
/// install, or a crash).
pub fn install_and_eval(pkgs: &[&str], usage: &str) -> Result<String, String> {
    let home = fresh_home();
    let elisp = format!("(progn\n{}{}\n)", install_prologue(pkgs), usage);
    let output = run_neomacs(home.path(), &elisp);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(format!(
            "install/eval of {pkgs:?} failed (exit {}):\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status
        ));
    }
    Ok(stdout)
}

/// Path to a hand-written `.el` script under `elisp/` documenting/driving a
/// real user workflow (used by the standalone workflow tests).
pub fn elisp_script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("elisp")
        .join(name)
}

/// Run NeoMacs in batch mode with the given HOME, loading an Elisp file.
pub fn run_neomacs_script(home: &Path, script: &Path) -> std::process::Output {
    Command::new(neomacs_binary())
        .env("HOME", home)
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "-l", &script.display().to_string()])
        .output()
        .expect("run neomacs script")
}

/// Run a NeoMacs Elisp script and check it exits successfully with no error
/// markers in its output.
pub fn run_neomacs_script_ok(home: &Path, script: &Path) -> Result<String, String> {
    let output = run_neomacs_script(home, script);
    check_output(&output)
}

/// Run NeoMacs in batch mode with the given HOME and Elisp forms.
pub fn run_neomacs(home: &Path, elisp: &str) -> std::process::Output {
    Command::new(neomacs_binary())
        .env("HOME", home)
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "--eval", elisp])
        .output()
        .expect("run neomacs")
}

/// Run NeoMacs and check it exits successfully with no error markers.
pub fn run_neomacs_ok(home: &Path, elisp: &str) -> Result<String, String> {
    let output = run_neomacs(home, elisp);
    check_output(&output)
}

fn check_output(output: &std::process::Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    for needle in &[
        "wrong-type-argument",
        "void-function",
        "file-missing",
        "invalid-read-syntax",
        "end-of-file",
        "Error:",
    ] {
        if stdout.contains(needle) || stderr.contains(needle) {
            return Err(format!(
                "neomacs emitted `{needle}`:\nstdout:\n{stdout}\nstderr:\n{stderr}"
            ));
        }
    }
    if !output.status.success() {
        return Err(format!(
            "neomacs exit status {}:\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status
        ));
    }
    Ok(stdout)
}
