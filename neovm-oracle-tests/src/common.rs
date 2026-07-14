//! Shared oracle helpers for Elisp unit tests.
//!
//! These helpers are intentionally test-only. The default snapshot mode only
//! requires a Neomacs release binary at `target/release/neomacs` (or
//! `NEOVM_BINARY_PATH`). Live oracle modes also require GNU Emacs on PATH (or
//! via `NEOVM_FORCE_ORACLE_PATH`).

use colored::Colorize;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

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
    OracleMode::from_env() == OracleMode::Snapshot || oracle_emacs_available()
}

pub(crate) fn live_oracle_enabled() -> bool {
    OracleMode::from_env() != OracleMode::Snapshot && oracle_emacs_available()
}

fn oracle_timing_enabled() -> bool {
    std::env::var_os("NEOVM_ORACLE_TIMING").is_some()
}

/// Execution strategy for oracle tests that embed GNU Emacs expectations in
/// the Rust test source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OracleMode {
    /// Fast path: run only Neomacs and compare its result with the checked-in
    /// inline GNU expectation.
    Snapshot,
    /// Consistency check: run GNU Emacs, compare it with the inline
    /// expectation, then require Neomacs to match the same live GNU result.
    Verify,
    /// Maintenance path: run GNU Emacs and compare it with the inline
    /// expectation; with `UPDATE_EXPECT=1`, `expect-test` rewrites the source.
    Refresh,
    /// Legacy parity path: run GNU Emacs and Neomacs directly, ignoring the
    /// inline expectation.
    Live,
}

impl OracleMode {
    fn from_env() -> Self {
        match std::env::var("NEOVM_ORACLE_MODE")
            .unwrap_or_else(|_| "snapshot".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "snapshot" | "snap" | "expected" => Self::Snapshot,
            "verify" => Self::Verify,
            "refresh" | "bless" | "update" => Self::Refresh,
            "live" => Self::Live,
            other => panic!(
                "unknown NEOVM_ORACLE_MODE={other:?}; expected snapshot, verify, refresh, or live"
            ),
        }
    }
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

fn oracle_emacs_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new(oracle_emacs_path())
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
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

fn apply_extra_env(cmd: &mut Command, extra_env: &[(&str, &str)]) {
    for (name, value) in extra_env {
        cmd.env(name, value);
    }
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
         ;; Opaque handles print with implementation-specific identities:
         ;; GNU uses addresses for threads/mutexes/condition variables, while
         ;; Neomacs uses simulated ids.  Normalize to stable semantic tokens
         ;; before generic cons/vector traversal can copy Neomacs handles.
         ;; Thread liveness is intentionally not part of the opaque thread
         ;; token: GNU `make-thread' returns before the worker necessarily
         ;; exits, so `(thread-live-p v)' is scheduler-sensitive for short
         ;; thread functions.  Tests that need liveness should call
         ;; `thread-live-p' explicitly.
         ((and (fboundp 'threadp) (threadp v))
          (list :thread
                (and (fboundp 'thread-name) (thread-name v))))
         ((and (fboundp 'mutexp) (mutexp v))
          (list :mutex
                (and (fboundp 'mutex-name) (mutex-name v))))
         ((and (fboundp 'condition-variable-p) (condition-variable-p v))
          (list :condition-variable
                (and (fboundp 'condition-name) (condition-name v))
                (and (fboundp 'condition-mutex)
                     (let ((m (condition-mutex v)))
                       (and (fboundp 'mutexp)
                            (mutexp m)
                            (list :mutex
                                  (and (fboundp 'mutex-name)
                                       (mutex-name m))))))))
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
         ;; org-element parses a timestamp into a plist whose sub-day fields
         ;; (:hour-start/:minute-start/:second-start and the -end variants) are
         ;; integers.  When the timestamp came from `current-time' (a live
         ;; `org-clock-in'/`org-insert-time-stamp'), those integers are the run
         ;; wall-clock and differ between record and replay.  Squash the sub-day
         ;; fields to 0 while leaving :year/:month/:day intact (dates are test
         ;; data or, at worst, only roll at midnight).
         ((and (consp v)
               (memq (car v) '(:hour-start :minute-start :second-start
                               :hour-end :minute-end :second-end))
               (integerp (car (cdr v))))
          (cons (car v) (cons 0 (neovm--oracle-normalize-1 (cdr (cdr v)) seen))))
         ;; `org-agenda' tags the line for the current day with an `org-today'
         ;; text property.  Which line carries it depends on the run date, so
         ;; across a midnight boundary between record and replay the property
         ;; moves.  Drop the `org-today PROP' pair from any plist.
         ((and (consp v) (eq (car v) 'org-today) (consp (cdr v)))
          (neovm--oracle-normalize-1 (cdr (cdr v)) seen))
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
         ;; Org clock, archive, and export output embed the wall-clock time of the run.
         ;; The Neomacs and GNU oracle processes start seconds apart, so
         ;; timestamps can differ even when the resulting structure matches.
         ;; Replace wall-clock timestamps with canonical placeholders while
         ;; preserving clock durations.
         ;; Use make-string to build the leading '#' without writing
         ;; the quote-hash sequence inside the Rust raw-string literal.
         ((stringp v)
          (let* ((hash (make-string 1 35))
                 ;; Today's date, computed live: the normalizer runs in both the
                 ;; recording and the replay process, so "today" is whatever day
                 ;; each runs on.  Agenda/capture/feed output embeds the run date
                 ;; (`%t' timestamps, datetree headings, feed dates, the current
                 ;; agenda day), which changes across a midnight boundary between
                 ;; record and replay.  Collapse today's date -- in the org
                 ;; timestamp, datetree, and bare ISO forms -- to fixed tokens.
                 ;; Fixed test dates (never equal to today on both days) are left
                 ;; intact.
                 (today (regexp-quote (format-time-string "%Y-%m-%d")))
                 ;; Frame/icon title product branding is a DELIBERATE Neomacs
                 ;; divergence: GNU titles read "%b - GNU Emacs at HOST" while
                 ;; Neomacs -- which must never advertise "GNU Emacs" -- reads
                 ;; "%b - NEO Emacs at HOST" (see frame_vars.rs). Canonicalize the
                 ;; product name on BOTH engines so the frame-title-format
                 ;; STRUCTURE stays a real parity lock while this one intentional
                 ;; brand difference is ignored.
                 (brand-normalized
                  (replace-regexp-in-string
                   "%b - \\(?:GNU\\|NEO\\) Emacs at "
                   "%b - [EMACS-PRODUCT] at "
                   v))
                 ;; `temporary-file-directory' / a bare $TMPDIR is the
                 ;; per-session nix-shell sandbox dir (/tmp/nix-shell.XXXXX): it
                 ;; differs between the recording and replay runs but is shared
                 ;; within a run, so it is not a real divergence. Squash the
                 ;; per-CASE `.../neovm-oracle-case-...' path FIRST (it starts
                 ;; with this same nix-shell prefix; the outer chain also squashes
                 ;; it, harmlessly, later), then squash any remaining bare session
                 ;; root -- so a case path becomes a single [ORACLE-TMPDIR] token
                 ;; rather than [SESSION-TMPDIR][ORACLE-TMPDIR].
                 (tmpdir-normalized
                  (replace-regexp-in-string
                   "/tmp/nix-shell\\.[A-Za-z0-9]+"
                   "[SESSION-TMPDIR]"
                   (replace-regexp-in-string
                    "/[^ \n\"]*neovm-oracle-case-[A-Za-z0-9]+"
                    "[ORACLE-TMPDIR]"
                    brand-normalized)))
                 (caption-normalized
                  (replace-regexp-in-string
                   (concat hash "\\+CAPTION: Clock summary at \\[[^]]+\\]")
                   (concat hash "+CAPTION: Clock summary at [FIXED-TIME]")
                   tmpdir-normalized)))
            (replace-regexp-in-string
             ;; Bare today ISO date (e.g. an Org feed's pubdate).
             today "[FIXED-TODAY-DATE]"
             (replace-regexp-in-string
              ;; Datetree heading / long form: today's ISO date + weekday name.
              (concat today " [A-Z][a-z]+day") "[FIXED-TODAY-DAY]"
              (replace-regexp-in-string
               ;; Org timestamp for today with a 3-letter weekday, no time
               ;; (e.g. a `%t' capture insert): [2026-07-12 Sun] / <...>.
               (concat "\\([][<>]\\)" today " [A-Z][a-z][a-z]\\([][<>]\\)")
               "\\1FIXED-TODAY\\2"
               (replace-regexp-in-string
                ;; Per-run tempdir is a random path (tempfile prefix
                ;; "neovm-oracle-case-" under $TMPDIR); GNU and Neomacs share it
                ;; within a run but it differs across recording/replay runs, so
                ;; it is not a real divergence.  Squash the whole absolute path.
                "/[^ \n\"]*neovm-oracle-case-[A-Za-z0-9]+"
                "[ORACLE-TMPDIR]"
             (replace-regexp-in-string
              ;; Bare Org timestamps carrying a wall-clock HH:MM (e.g. from a
              ;; `%U'/`%T' capture template or `org-insert-time-stamp') are
              ;; recorded seconds apart from replay; the date+time is the run
              ;; time, not test data.  Timestamps WITHOUT a time (SCHEDULED/
              ;; DEADLINE dates like <2026-05-27 Wed>) are left intact.
              "\\([][<>]\\)[0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\} [A-Z][a-z][a-z] [0-9]\\{2\\}:[0-9]\\{2\\}\\([][<>]\\)"
              "\\1FIXED-ORG-TIME\\2"
              (replace-regexp-in-string
               ":ARCHIVE_TIME: [^\n]+"
               ":ARCHIVE_TIME: [FIXED-ARCHIVE-TIME]"
               (replace-regexp-in-string
                "CLOCK: \\[[^]]+\\]--\\[[^]]+\\] => \\( *[0-9]+:[0-9][0-9]\\)"
                "CLOCK: [FIXED-CLOCK] => \\1"
                (replace-regexp-in-string
                 "Created: [0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\} [A-Z][a-z][a-z] [0-9]\\{2\\}:[0-9]\\{2\\}"
                 "Created: [FIXED-EXPORT-TIME]"
                 (replace-regexp-in-string
                  "<!-- [0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\} [A-Z][a-z][a-z] [0-9]\\{2\\}:[0-9]\\{2\\} -->"
                  "<!-- [FIXED-EXPORT-TIME] -->"
                  (replace-regexp-in-string
                   "% Created [0-9]\\{4\\}-[0-9]\\{2\\}-[0-9]\\{2\\} [A-Z][a-z][a-z] [0-9]\\{2\\}:[0-9]\\{2\\}"
                   "% Created [FIXED-EXPORT-TIME]"
                   caption-normalized))))))))))))
         (t v)))
      (defun neovm--oracle-normalize (v)
        (neovm--oracle-normalize-1 v (make-hash-table :test 'eq)))
    (let* ((coding-system-for-read 'utf-8-unix)
           (coding-system-for-write 'utf-8-unix)
           (_ (set-language-environment "UTF-8"))
           (_ (setq system-time-locale "C"))
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
             (_ (setq system-time-locale "C"))
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

const NATIVE_COMP_SUPPRESSION_PRELUDE: &str = "(setq native-comp-jit-compilation nil inhibit-automatic-native-compilation t native-comp-enable-subr-trampolines nil)";

// ---------------------------------------------------------------------------
// Oracle (GNU Emacs) subprocess evaluation
// ---------------------------------------------------------------------------

fn run_oracle_eval_inner_with_tmpdir(
    form: &str,
    load_files: &[&str],
    shared_tmpdir: Option<&Path>,
    extra_env: &[(&str, &str)],
    load_root: &Path,
) -> Result<String, String> {
    let form_path = write_oracle_form_file(form)?;
    let oracle_bin = oracle_emacs_path();
    let lisp_dir = load_root.to_path_buf();
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
            NATIVE_COMP_SUPPRESSION_PRELUDE,
            "--eval",
            EVAL_PROGRAM_WITH_NORMALIZER,
        ]);
    if let Some(dir) = shared_tmpdir {
        cmd.env("NEOVM_ORACLE_TEST_TMPDIR", dir.as_os_str());
    }
    apply_extra_env(&mut cmd, extra_env);

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
    run_oracle_eval_inner_with_tmpdir(form, load_files, None, &[], &project_lisp_dir())
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
            NATIVE_COMP_SUPPRESSION_PRELUDE,
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
    match OracleMode::from_env() {
        OracleMode::Snapshot => run_neomacs_binary_eval_inner(form, &[]),
        OracleMode::Verify | OracleMode::Refresh | OracleMode::Live => {
            run_oracle_eval_inner(form, &[])
        }
    }
}

pub(crate) fn run_oracle_eval_with_load(form: &str, load_files: &[&str]) -> Result<String, String> {
    match OracleMode::from_env() {
        OracleMode::Snapshot => run_neomacs_binary_eval_inner(form, load_files),
        OracleMode::Verify | OracleMode::Refresh | OracleMode::Live => {
            run_oracle_eval_inner(form, load_files)
        }
    }
}

pub(crate) fn run_oracle_eval_with_load_raw(
    form: &str,
    load_files: &[&str],
) -> Result<String, String> {
    match OracleMode::from_env() {
        OracleMode::Snapshot => run_neomacs_binary_eval_inner_raw(form, load_files),
        OracleMode::Verify | OracleMode::Refresh | OracleMode::Live => {
            run_oracle_eval_inner_raw(form, load_files)
        }
    }
}

/// Like `run_oracle_eval_with_load`, but loads files from an external
/// `load_root` (e.g. a third-party package checkout) instead of the project's
/// own `lisp/` tree. Used by the package-corpus oracle tests
/// (e.g. `emacsorphanage_*`) to exercise real-world Elisp against both GNU
/// Emacs and Neomacs from the same checkout.
pub(crate) fn run_oracle_eval_with_load_root(
    form: &str,
    load_files: &[&str],
    load_root: &Path,
) -> Result<String, String> {
    match OracleMode::from_env() {
        OracleMode::Snapshot => {
            run_neomacs_binary_eval_inner_with_tmpdir(form, load_files, None, &[], load_root)
        }
        OracleMode::Verify | OracleMode::Refresh | OracleMode::Live => {
            run_oracle_eval_inner_with_tmpdir(form, load_files, None, &[], load_root)
        }
    }
}

// ---------------------------------------------------------------------------
// Neomacs binary subprocess evaluation
// ---------------------------------------------------------------------------

fn run_neomacs_binary_eval_inner_with_tmpdir(
    form: &str,
    load_files: &[&str],
    shared_tmpdir: Option<&Path>,
    extra_env: &[(&str, &str)],
    load_root: &Path,
) -> Result<String, String> {
    let form_path = write_oracle_form_file(form)?;
    let lisp_dir = load_root.to_path_buf();
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
        .args([
            "--batch",
            "-Q",
            "--eval",
            NATIVE_COMP_SUPPRESSION_PRELUDE,
            "--eval",
            EVAL_PROGRAM_WITH_NORMALIZER,
        ]);
    if let Some(dir) = shared_tmpdir {
        cmd.env("NEOVM_ORACLE_TEST_TMPDIR", dir.as_os_str());
    }
    apply_extra_env(&mut cmd, extra_env);

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
    run_neomacs_binary_eval_inner_with_tmpdir(form, load_files, None, &[], &project_lisp_dir())
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
        .args([
            "--batch",
            "-Q",
            "--eval",
            NATIVE_COMP_SUPPRESSION_PRELUDE,
            "--eval",
            EVAL_PROGRAM_RAW,
        ]);

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

// Store inline oracle values in a Rust-debug representation. This keeps exact
// newlines, tabs, quotes, and trailing spaces testable without putting literal
// trailing whitespace or conflict-marker-looking lines in source files.
fn inline_expect_payload(value: &str) -> String {
    let source_safe = value.replace('\0', "\\0").replace('\r', "\\r");
    format!("{source_safe:?}")
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
    if OracleMode::from_env() == OracleMode::Snapshot {
        if log_timing {
            eprintln!(
                "oracle-timing: neomacs-binary-done {:.3?}",
                neomacs_t0.elapsed()
            );
        }
        return;
    }
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

fn assert_oracle_parity_expect_with_runners<N, O>(
    form: &str,
    expected: expect_test::Expect,
    run_neomacs: N,
    run_oracle: O,
) where
    N: FnOnce() -> Result<String, String>,
    O: FnOnce() -> Result<String, String>,
{
    ensure_nonempty_form(form).expect("form should not be empty");

    match OracleMode::from_env() {
        OracleMode::Snapshot => {
            let neovm = run_neomacs().expect("neomacs binary eval should run");
            expected.assert_eq(&inline_expect_payload(&neovm));
        }
        OracleMode::Verify => {
            let oracle = run_oracle().expect("oracle eval should run");
            let neovm = run_neomacs().expect("neomacs binary eval should run");
            expected.assert_eq(&inline_expect_payload(&oracle));
            assert_neovm_oracle_parity(&neovm, &oracle, form);
        }
        OracleMode::Refresh => {
            let oracle = run_oracle().expect("oracle eval should run");
            expected.assert_eq(&inline_expect_payload(&oracle));
        }
        OracleMode::Live => {
            let oracle = run_oracle().expect("oracle eval should run");
            let neovm = run_neomacs().expect("neomacs binary eval should run");
            assert_neovm_oracle_parity(&neovm, &oracle, form);
        }
    }
}

pub(crate) fn assert_oracle_parity_expect(form: &str, expected: expect_test::Expect) {
    assert_oracle_parity_expect_with_runners(
        form,
        expected,
        || run_neomacs_binary_eval_inner(form, &[]),
        || run_oracle_eval(form),
    );
}

pub(crate) fn assert_oracle_parity_with_shared_tempdir_expect(
    form: &str,
    expected: expect_test::Expect,
) {
    let tmpdir = tempfile::Builder::new()
        .prefix("neovm-oracle-case-")
        .tempdir()
        .expect("shared oracle tempdir should be created");
    assert_oracle_parity_expect_with_runners(
        form,
        expected,
        || {
            run_neomacs_binary_eval_inner_with_tmpdir(
                form,
                &[],
                Some(tmpdir.path()),
                &[],
                &project_lisp_dir(),
            )
        },
        || {
            run_oracle_eval_inner_with_tmpdir(
                form,
                &[],
                Some(tmpdir.path()),
                &[],
                &project_lisp_dir(),
            )
        },
    );
}

pub(crate) fn assert_oracle_parity_with_env_expect(
    form: &str,
    extra_env: &[(&str, &str)],
    expected: expect_test::Expect,
) {
    assert_oracle_parity_expect_with_runners(
        form,
        expected,
        || {
            run_neomacs_binary_eval_inner_with_tmpdir(
                form,
                &[],
                None,
                extra_env,
                &project_lisp_dir(),
            )
        },
        || run_oracle_eval_inner_with_tmpdir(form, &[], None, extra_env, &project_lisp_dir()),
    );
}

pub(crate) fn assert_oracle_parity_with_load_expect(
    form: &str,
    load_files: &[&str],
    expected: expect_test::Expect,
) {
    assert_oracle_parity_expect_with_runners(
        form,
        expected,
        || run_neomacs_binary_eval_inner(form, load_files),
        || run_oracle_eval_with_load(form, load_files),
    );
}

pub(crate) fn assert_oracle_parity_with_load_raw_expect(
    form: &str,
    load_files: &[&str],
    expected: expect_test::Expect,
) {
    assert_oracle_parity_expect_with_runners(
        form,
        expected,
        || run_neomacs_binary_eval_inner_raw(form, load_files),
        || run_oracle_eval_with_load_raw(form, load_files),
    );
}

pub(crate) fn assert_oracle_parity_with_shared_tempdir(form: &str) {
    ensure_nonempty_form(form).expect("form should not be empty");
    let tmpdir = tempfile::Builder::new()
        .prefix("neovm-oracle-case-")
        .tempdir()
        .expect("shared oracle tempdir should be created");
    let neovm = run_neomacs_binary_eval_inner_with_tmpdir(
        form,
        &[],
        Some(tmpdir.path()),
        &[],
        &project_lisp_dir(),
    )
    .expect("neomacs binary eval should run");
    if OracleMode::from_env() == OracleMode::Snapshot {
        return;
    }
    let oracle =
        run_oracle_eval_inner_with_tmpdir(form, &[], Some(tmpdir.path()), &[], &project_lisp_dir())
            .expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn assert_oracle_parity_with_env(form: &str, extra_env: &[(&str, &str)]) {
    ensure_nonempty_form(form).expect("form should not be empty");
    let neovm =
        run_neomacs_binary_eval_inner_with_tmpdir(form, &[], None, extra_env, &project_lisp_dir())
            .expect("neomacs binary eval should run");
    if OracleMode::from_env() == OracleMode::Snapshot {
        return;
    }
    let oracle = run_oracle_eval_inner_with_tmpdir(form, &[], None, extra_env, &project_lisp_dir())
        .expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn assert_oracle_parity_with_load(form: &str, load_files: &[&str]) {
    let neovm =
        run_neomacs_binary_eval_inner(form, load_files).expect("neomacs binary eval should run");
    if OracleMode::from_env() == OracleMode::Snapshot {
        return;
    }
    let oracle = run_oracle_eval_with_load(form, load_files).expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn assert_oracle_parity_with_load_raw(form: &str, load_files: &[&str]) {
    let neovm = run_neomacs_binary_eval_inner_raw(form, load_files)
        .expect("neomacs binary eval should run");
    if OracleMode::from_env() == OracleMode::Snapshot {
        return;
    }
    let oracle = run_oracle_eval_with_load_raw(form, load_files).expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

/// Snapshot/parity assertion that loads third-party files from an external
/// `load_root` (a package checkout) rather than the project `lisp/` tree.
/// In Snapshot mode only Neomacs runs against the inline expectation; in
/// Verify/Refresh/Live the GNU oracle is driven from the same checkout.
pub(crate) fn assert_oracle_parity_with_load_root_expect(
    form: &str,
    load_files: &[&str],
    load_root: &Path,
    expected: expect_test::Expect,
) {
    assert_oracle_parity_expect_with_runners(
        form,
        expected,
        || run_neomacs_binary_eval_inner_with_tmpdir(form, load_files, None, &[], load_root),
        || run_oracle_eval_with_load_root(form, load_files, load_root),
    );
}

/// Non-snapshot variant of `assert_oracle_parity_with_load_root_expect` for
/// cases where no inline GNU expectation is kept (pure live parity).
pub(crate) fn assert_oracle_parity_with_load_root(
    form: &str,
    load_files: &[&str],
    load_root: &Path,
) {
    let neovm = run_neomacs_binary_eval_inner_with_tmpdir(form, load_files, None, &[], load_root)
        .expect("neomacs binary eval should run");
    if OracleMode::from_env() == OracleMode::Snapshot {
        return;
    }
    let oracle = run_oracle_eval_with_load_root(form, load_files, load_root)
        .expect("oracle eval should run");
    assert_neovm_oracle_parity(&neovm, &oracle, form);
}

pub(crate) fn eval_oracle_and_neovm(form: &str) -> (String, String) {
    if OracleMode::from_env() == OracleMode::Snapshot {
        let neovm =
            run_neomacs_binary_eval_inner(form, &[]).expect("neomacs binary eval should run");
        return (neovm.clone(), neovm);
    }
    let neovm = run_neomacs_binary_eval_inner(form, &[]).expect("neomacs binary eval should run");
    let oracle = run_oracle_eval(form).expect("oracle eval should run");
    (oracle, neovm)
}

pub(crate) fn eval_oracle_and_neovm_expect(
    form: &str,
    expected: expect_test::Expect,
) -> (String, String) {
    ensure_nonempty_form(form).expect("form should not be empty");

    match OracleMode::from_env() {
        OracleMode::Snapshot => {
            let neovm =
                run_neomacs_binary_eval_inner(form, &[]).expect("neomacs binary eval should run");
            expected.assert_eq(&inline_expect_payload(&neovm));
            (neovm.clone(), neovm)
        }
        OracleMode::Verify => {
            let oracle = run_oracle_eval(form).expect("oracle eval should run");
            let neovm =
                run_neomacs_binary_eval_inner(form, &[]).expect("neomacs binary eval should run");
            expected.assert_eq(&inline_expect_payload(&oracle));
            assert_neovm_oracle_parity(&neovm, &oracle, form);
            (oracle, neovm)
        }
        OracleMode::Refresh => {
            let oracle = run_oracle_eval(form).expect("oracle eval should run");
            expected.assert_eq(&inline_expect_payload(&oracle));
            (oracle.clone(), oracle)
        }
        OracleMode::Live => eval_oracle_and_neovm(form),
    }
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
