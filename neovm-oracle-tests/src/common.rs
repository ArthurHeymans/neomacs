//! Shared oracle helpers for Elisp unit tests.
//!
//! These helpers are intentionally test-only and require GNU Emacs
//! available on PATH (or via `NEOVM_FORCE_ORACLE_PATH`) and a Neomacs
//! release binary at `target/release/neomacs` (or `NEOVM_BINARY_PATH`).

use colored::Colorize;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum virtual address space (in bytes) for each spawned oracle Emacs
/// process.  This prevents runaway evaluations from consuming unbounded
/// memory and triggering the system OOM killer.
/// Overridable via `NEOVM_ORACLE_MEM_LIMIT_MB` (default: 500 MB).
fn oracle_mem_limit_bytes() -> u64 {
    let mb: u64 = std::env::var("NEOVM_ORACLE_MEM_LIMIT_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    mb * 1024 * 1024
}

/// Optional virtual address space cap for spawned release Neomacs binary
/// checks. Unlike the GNU oracle process, release Neomacs can legitimately
/// map several gigabytes while running exhaustive recursive parity cases, and
/// some nextest child processes cannot raise a lower inherited hard limit.
///
/// Set `NEOVM_NEOMACS_BINARY_MEM_LIMIT_MB` to enable an extra cap.
fn neomacs_binary_mem_limit_bytes() -> Option<u64> {
    let mb: u64 = std::env::var("NEOVM_NEOMACS_BINARY_MEM_LIMIT_MB")
        .ok()
        .and_then(|v| v.parse().ok())?;
    Some(mb * 1024 * 1024)
}

pub(crate) const ORACLE_PROP_CASES: u32 = 10;

pub(crate) fn oracle_prop_enabled() -> bool {
    true
}

fn oracle_timing_enabled() -> bool {
    std::env::var_os("NEOVM_ORACLE_TIMING").is_some()
}

macro_rules! return_if_neovm_enable_oracle_proptest_not_set {
    () => {
        if !$crate::common::oracle_prop_enabled() {
            tracing::info!(
                "skipping {}:{}: set NEOVM_FORCE_ORACLE_PATH=/path/to/emacs",
                module_path!(),
                line!()
            );
            return;
        }
    };
    ($ret:expr) => {
        if !$crate::common::oracle_prop_enabled() {
            tracing::info!(
                "skipping {}:{}: set NEOVM_FORCE_ORACLE_PATH=/path/to/emacs",
                module_path!(),
                line!()
            );
            return $ret;
        }
    };
}

pub(crate) use return_if_neovm_enable_oracle_proptest_not_set;

fn oracle_emacs_path() -> String {
    if let Ok(path) = std::env::var("NEOVM_FORCE_ORACLE_PATH") {
        return path;
    }
    "emacs".to_string()
}

fn neomacs_binary_path() -> String {
    std::env::var("NEOVM_BINARY_PATH").unwrap_or_else(|_| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("project root")
            .join("target/release/neomacs")
            .to_string_lossy()
            .into_owned()
    })
}

fn write_temp_elisp_file(
    prefix: &str,
    suffix: &str,
    content: &str,
) -> Result<tempfile::TempPath, String> {
    let mut file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile()
        .map_err(|e| format!("failed to create oracle form file: {e}"))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to write oracle form file: {e}"))?;
    file.flush()
        .map_err(|e| format!("failed to flush oracle form file: {e}"))?;
    Ok(file.into_temp_path())
}

fn write_oracle_form_file(form: &str) -> Result<tempfile::TempPath, String> {
    write_temp_elisp_file("neovm-oracle-form-", ".el", form)
}

fn project_lisp_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("project root").join("lisp")
}

fn ensure_nonempty_form(form: &str) -> Result<(), String> {
    if form.trim().is_empty() {
        Err("no form parsed".to_string())
    } else {
        Ok(())
    }
}

const EVAL_PROGRAM_WITH_NORMALIZER: &str = r#"(condition-case err
    (progn
      (defun neovm--oracle-normalize-1 (v seen)
        (cond
         ((and (functionp v) (eq (type-of v) 'interpreted-function))
          (let ((args (aref v 0))
                (body (aref v 1))
                (env (aref v 2)))
            (if (null env)
                (cons 'lambda
                      (cons (neovm--oracle-normalize-1 args seen)
                            (neovm--oracle-normalize-1 body seen)))
              (cons 'closure
                    (cons (neovm--oracle-normalize-1 env seen)
                          (cons (neovm--oracle-normalize-1 args seen)
                                (neovm--oracle-normalize-1 body seen)))))))
         ((consp v)
          (or (gethash v seen)
              (let ((out (cons nil nil)))
                (puthash v out seen)
                (setcar out (neovm--oracle-normalize-1 (car v) seen))
                (setcdr out (neovm--oracle-normalize-1 (cdr v) seen))
                out)))
         ((vectorp v)
          (or (gethash v seen)
              (let* ((len (length v))
                     (out (make-vector len nil)))
                (puthash v out seen)
                (dotimes (i len)
                  (aset out i (neovm--oracle-normalize-1 (aref v i) seen)))
                out)))
         ;; Large fixnums in error data are implementation artefacts:
         ;; Neomacs uses a hardcoded sentinel for unfilled concat slots in
         ;; mapconcat, while GNU reuses uninitialised stack memory.  Both are
         ;; non-deterministic across builds, so squash them to 0 for parity.
         ((fixnump v) (if (> (abs v) 1000000000000) 0 v))
         (t v)))
      (defun neovm--oracle-normalize (v)
        (neovm--oracle-normalize-1 v (make-hash-table :test 'eq)))
    (let* ((coding-system-for-read 'utf-8-unix)
           (coding-system-for-write 'utf-8-unix)
           (_ (set-language-environment "UTF-8"))
           (load-root (getenv "NEOVM_ORACLE_LOAD_ROOT"))
           (load-files (split-string (or (getenv "NEOVM_ORACLE_LOAD_FILES") "") "\n" t))
           (form-file (getenv "NEOVM_ORACLE_FORM_FILE"))
           (result
            (let ((source-buf (generate-new-buffer " *neovm-oracle-form*")))
              (unwind-protect
                  (progn
                    (when load-root
                      (let ((extra-load-path nil))
                        (dolist (sub '("" "emacs-lisp" "progmodes" "language"
                                       "international" "textmodes" "vc" "leim"
                                       "org"))
                          (let ((dir (if (equal sub "")
                                         load-root
                                       (expand-file-name sub load-root))))
                            (when (file-directory-p dir)
                              (push dir extra-load-path))))
                        (setq load-path (append (nreverse extra-load-path) load-path))))
                    (dolist (file load-files)
                      (load file nil t nil t))
                    (with-current-buffer source-buf
                      (insert-file-contents form-file)
                      (goto-char (point-min)))
                    (let ((last nil))
                      (condition-case nil
                          (while t
                            (setq last (eval (read source-buf) t)))
                        (end-of-file last))))
                (when (buffer-live-p source-buf)
                  (kill-buffer source-buf))))))
      (princ (concat "OK " (prin1-to-string (neovm--oracle-normalize result))))))
  (error
   (princ
    (concat "ERR "
            (prin1-to-string
             (neovm--oracle-normalize (cons (car err) (cdr err))))))))"#;

const EVAL_PROGRAM_RAW: &str = r#"(condition-case err
    (progn
      (let* ((coding-system-for-read 'utf-8-unix)
             (coding-system-for-write 'utf-8-unix)
             (_ (set-language-environment "UTF-8"))
             (load-root (getenv "NEOVM_ORACLE_LOAD_ROOT"))
             (load-files (split-string (or (getenv "NEOVM_ORACLE_LOAD_FILES") "") "\n" t))
             (form-file (getenv "NEOVM_ORACLE_FORM_FILE"))
             (result
              (let ((source-buf (generate-new-buffer " *neovm-oracle-form*")))
                (unwind-protect
                    (progn
                      (when load-root
                        (let ((extra-load-path nil))
                          (dolist (sub '("" "emacs-lisp" "progmodes" "language"
                                         "international" "textmodes" "vc" "leim"
                                         "org"))
                            (let ((dir (if (equal sub "")
                                           load-root
                                         (expand-file-name sub load-root))))
                              (when (file-directory-p dir)
                                (push dir extra-load-path))))
                          (setq load-path (append (nreverse extra-load-path) load-path))))
                      (dolist (file load-files)
                        (load file nil t nil t))
                      (with-current-buffer source-buf
                        (insert-file-contents form-file)
                        (goto-char (point-min)))
                      (let ((last nil))
                        (condition-case nil
                            (while t
                              (setq last (eval (read source-buf) t)))
                          (end-of-file last))))
                  (when (buffer-live-p source-buf)
                    (kill-buffer source-buf))))))
        (princ (concat "OK " (prin1-to-string result)))))
  (error
   (princ (concat "ERR " (prin1-to-string err)))))"#;

// ---------------------------------------------------------------------------
// Oracle (GNU Emacs) subprocess evaluation
// ---------------------------------------------------------------------------

fn run_oracle_eval_inner_with_tmpdir(
    form: &str,
    load_files: &[&str],
    shared_tmpdir: Option<&Path>,
) -> Result<String, String> {
    let form_path = write_oracle_form_file(form)?;
    let oracle_bin = oracle_emacs_path();
    let lisp_dir = project_lisp_dir();
    let oracle_load_files = load_files
        .iter()
        .map(|file| lisp_dir.join(file).to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");

    let mem_limit = oracle_mem_limit_bytes();
    let mut cmd = Command::new(&oracle_bin);
    cmd.env("NEOVM_ORACLE_FORM_FILE", form_path.as_os_str())
        .env("NEOVM_ORACLE_LOAD_ROOT", &lisp_dir)
        .env("NEOVM_ORACLE_LOAD_FILES", oracle_load_files)
        .env("EMACSNATIVELOADPATH", "/dev/null")
        .args([
            "--batch",
            "-Q",
            "--eval",
            "(setq native-comp-jit-compilation nil inhibit-automatic-native-compilation t native-comp-enable-subr-trampolines nil)",
            "--eval",
            EVAL_PROGRAM_WITH_NORMALIZER,
        ]);
    if let Some(dir) = shared_tmpdir {
        cmd.env("NEOVM_ORACLE_TEST_TMPDIR", dir.as_os_str());
    }

    unsafe {
        cmd.pre_exec(move || {
            let rlim = libc::rlimit {
                rlim_cur: mem_limit as libc::rlim_t,
                rlim_max: mem_limit as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run oracle Emacs: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "oracle Emacs failed: status={}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_oracle_eval_inner(form: &str, load_files: &[&str]) -> Result<String, String> {
    run_oracle_eval_inner_with_tmpdir(form, load_files, None)
}

fn run_oracle_eval_inner_raw(form: &str, load_files: &[&str]) -> Result<String, String> {
    let form_path = write_oracle_form_file(form)?;
    let oracle_bin = oracle_emacs_path();
    let lisp_dir = project_lisp_dir();
    let oracle_load_files = load_files
        .iter()
        .map(|file| lisp_dir.join(file).to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");

    let mem_limit = oracle_mem_limit_bytes();
    let mut cmd = Command::new(&oracle_bin);
    cmd.env("NEOVM_ORACLE_FORM_FILE", form_path.as_os_str())
        .env("NEOVM_ORACLE_LOAD_ROOT", &lisp_dir)
        .env("NEOVM_ORACLE_LOAD_FILES", oracle_load_files)
        .env("EMACSNATIVELOADPATH", "/dev/null")
        .args([
            "--batch",
            "-Q",
            "--eval",
            "(setq native-comp-jit-compilation nil inhibit-automatic-native-compilation t native-comp-enable-subr-trampolines nil)",
            "--eval",
            EVAL_PROGRAM_RAW,
        ]);

    unsafe {
        cmd.pre_exec(move || {
            let rlim = libc::rlimit {
                rlim_cur: mem_limit as libc::rlim_t,
                rlim_max: mem_limit as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run oracle Emacs: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "oracle Emacs failed: status={}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn run_oracle_eval(form: &str) -> Result<String, String> {
    run_oracle_eval_inner(form, &[])
}

pub(crate) fn run_oracle_eval_with_load(form: &str, load_files: &[&str]) -> Result<String, String> {
    run_oracle_eval_inner(form, load_files)
}

pub(crate) fn run_oracle_eval_with_load_raw(
    form: &str,
    load_files: &[&str],
) -> Result<String, String> {
    run_oracle_eval_inner_raw(form, load_files)
}

// ---------------------------------------------------------------------------
// Neomacs binary subprocess evaluation
// ---------------------------------------------------------------------------

fn run_neomacs_binary_eval_inner_with_tmpdir(
    form: &str,
    load_files: &[&str],
    shared_tmpdir: Option<&Path>,
) -> Result<String, String> {
    let form_path = write_oracle_form_file(form)?;
    let lisp_dir = project_lisp_dir();
    let neomacs_bin = neomacs_binary_path();
    let load_files_str = load_files
        .iter()
        .map(|file| lisp_dir.join(file).to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");

    let mut cmd = Command::new(&neomacs_bin);
    cmd.env("NEOVM_ORACLE_FORM_FILE", form_path.as_os_str())
        .env("NEOVM_ORACLE_LOAD_ROOT", &lisp_dir)
        .env("NEOVM_ORACLE_LOAD_FILES", load_files_str)
        .args(["--batch", "-Q", "--eval", EVAL_PROGRAM_WITH_NORMALIZER]);
    if let Some(dir) = shared_tmpdir {
        cmd.env("NEOVM_ORACLE_TEST_TMPDIR", dir.as_os_str());
    }

    if let Some(mem_limit) = neomacs_binary_mem_limit_bytes() {
        unsafe {
            cmd.pre_exec(move || {
                let rlim = libc::rlimit {
                    rlim_cur: mem_limit as libc::rlim_t,
                    rlim_max: mem_limit as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run Neomacs binary: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Neomacs binary failed: status={}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_neomacs_binary_eval_inner(form: &str, load_files: &[&str]) -> Result<String, String> {
    run_neomacs_binary_eval_inner_with_tmpdir(form, load_files, None)
}

fn run_neomacs_binary_eval_inner_raw(form: &str, load_files: &[&str]) -> Result<String, String> {
    let form_path = write_oracle_form_file(form)?;
    let lisp_dir = project_lisp_dir();
    let neomacs_bin = neomacs_binary_path();
    let load_files_str = load_files
        .iter()
        .map(|file| lisp_dir.join(file).to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");

    let mut cmd = Command::new(&neomacs_bin);
    cmd.env("NEOVM_ORACLE_FORM_FILE", form_path.as_os_str())
        .env("NEOVM_ORACLE_LOAD_ROOT", &lisp_dir)
        .env("NEOVM_ORACLE_LOAD_FILES", load_files_str)
        .args(["--batch", "-Q", "--eval", EVAL_PROGRAM_RAW]);

    if let Some(mem_limit) = neomacs_binary_mem_limit_bytes() {
        unsafe {
            cmd.pre_exec(move || {
                let rlim = libc::rlimit {
                    rlim_cur: mem_limit as libc::rlim_t,
                    rlim_max: mem_limit as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run Neomacs binary: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Neomacs binary failed: status={}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn run_neovm_eval(form: &str) -> Result<String, String> {
    run_neomacs_binary_eval_inner(form, &[])
}

pub(crate) fn run_neovm_eval_with_load(form: &str, load_files: &[&str]) -> Result<String, String> {
    run_neomacs_binary_eval_inner(form, load_files)
}

pub(crate) fn run_neovm_eval_with_load_raw(
    form: &str,
    load_files: &[&str],
) -> Result<String, String> {
    run_neomacs_binary_eval_inner_raw(form, load_files)
}

// ---------------------------------------------------------------------------
// Internal parity helper
// ---------------------------------------------------------------------------

fn assert_neovm_oracle_parity(neovm: &str, oracle: &str, form: &str) {
    if neovm == oracle {
        return;
    }
    let neo_label = "NEO Emacs:".red().bold().to_string();
    let gnu_label = "GNU Emacs:".green().bold().to_string();
    panic!(
        "oracle parity mismatch for form: {form}\n  {neo_label}  {neovm}\n  {gnu_label}  {oracle}\n  NEO debug (len={}): {:?}\n  GNU debug (len={}): {:?}",
        neovm.len(),
        neovm,
        oracle.len(),
        oracle
    );
}

// ---------------------------------------------------------------------------
// Public parity assertions
// ---------------------------------------------------------------------------

pub(crate) fn assert_oracle_parity(form: &str) {
    let t0 = std::time::Instant::now();
    let log_timing = oracle_timing_enabled();

    ensure_nonempty_form(form).expect("form should not be empty");

    if log_timing {
        eprintln!("oracle-timing: neomacs-binary-start");
    }
    let neomacs_t0 = std::time::Instant::now();
    let neovm = run_neomacs_binary_eval_inner(form, &[]).expect("neomacs binary eval should run");
    if log_timing {
        eprintln!(
            "oracle-timing: neomacs-binary-done {:.3?}",
            neomacs_t0.elapsed()
        );
        eprintln!("oracle-timing: oracle-start");
    }
    let oracle_t0 = std::time::Instant::now();
    let oracle = run_oracle_eval(form).expect("oracle eval should run");
    if log_timing {
        eprintln!("oracle-timing: oracle-done {:.3?}", oracle_t0.elapsed());
    }
    eprintln!("total: {:.3?}", t0.elapsed());
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn assert_oracle_parity_with_shared_tempdir(form: &str) {
    ensure_nonempty_form(form).expect("form should not be empty");
    let tmpdir = tempfile::Builder::new()
        .prefix("neovm-oracle-case-")
        .tempdir()
        .expect("shared oracle tempdir should be created");
    let neovm = run_neomacs_binary_eval_inner_with_tmpdir(form, &[], Some(tmpdir.path()))
        .expect("neomacs binary eval should run");
    let oracle = run_oracle_eval_inner_with_tmpdir(form, &[], Some(tmpdir.path()))
        .expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn assert_oracle_parity_with_load(form: &str, load_files: &[&str]) {
    let neovm =
        run_neomacs_binary_eval_inner(form, load_files).expect("neomacs binary eval should run");
    let oracle = run_oracle_eval_with_load(form, load_files).expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn assert_oracle_parity_with_load_raw(form: &str, load_files: &[&str]) {
    let neovm = run_neomacs_binary_eval_inner_raw(form, load_files)
        .expect("neomacs binary eval should run");
    let oracle = run_oracle_eval_with_load_raw(form, load_files).expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn eval_oracle_and_neovm(form: &str) -> (String, String) {
    let neovm = run_neomacs_binary_eval_inner(form, &[]).expect("neomacs binary eval should run");
    let oracle = run_oracle_eval(form).expect("oracle eval should run");
    (oracle, neovm)
}

pub(crate) fn assert_ok_eq(expected_payload: &str, oracle: &str, neovm: &str) {
    let expected = format!("OK {expected_payload}");
    assert_eq!(oracle, expected, "GNU Emacs should match expected payload");
    assert_eq!(neovm, expected, "Neomacs should match expected payload");
    assert_neovm_oracle_parity(neovm, oracle, "assert_ok_eq");
}

pub(crate) fn assert_err_kind(oracle: &str, neovm: &str, err_kind: &str) {
    assert!(
        oracle.starts_with("ERR "),
        "oracle should return an error: {oracle}"
    );
    assert!(
        neovm.starts_with("ERR "),
        "neovm should return an error: {neovm}"
    );

    let oracle_payload = oracle
        .strip_prefix("ERR ")
        .expect("oracle payload should have ERR prefix")
        .trim();
    let neovm_payload = neovm
        .strip_prefix("ERR ")
        .expect("neovm payload should have ERR prefix")
        .trim();

    assert!(
        !oracle_payload.is_empty(),
        "oracle error should include a message"
    );
    assert!(
        !neovm_payload.is_empty(),
        "neovm error should include a message"
    );
    assert!(
        oracle_payload.contains(err_kind),
        "oracle error kind should contain '{err_kind}': {oracle_payload}"
    );
    assert!(
        neovm_payload.contains(err_kind),
        "neovm error kind should contain '{err_kind}': {neovm_payload}"
    );
}
