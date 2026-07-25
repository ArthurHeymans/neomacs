//! Shared result protocol for differential GNU Emacs/Neomacs tests.
//!
//! Editor-specific sandboxes remain adapters owned by their test crates. This
//! crate defines the small common interface at the comparison seam: an
//! evaluation either returns a normalized printed Lisp value or signals
//! normalized printed Lisp error data.

use std::fmt;

/// A normalized, comparable editor evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalOutcome {
    Value(String),
    Signal(String),
}

impl EvalOutcome {
    /// Parse the `OK …` / `ERR …` protocol emitted by an editor adapter.
    pub fn parse(encoded: &str) -> Result<Self, String> {
        let encoded = encoded.trim();
        if let Some(value) = encoded.strip_prefix("OK ") {
            return Ok(Self::Value(value.to_string()));
        }
        if let Some(signal) = encoded.strip_prefix("ERR ") {
            return Ok(Self::Signal(signal.to_string()));
        }
        Err(format!(
            "expected an oracle outcome beginning with `OK ` or `ERR `, got `{encoded}`"
        ))
    }

    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }
}

impl fmt::Display for EvalOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(value) => write!(formatter, "OK {value}"),
            Self::Signal(signal) => write!(formatter, "ERR {signal}"),
        }
    }
}

/// Extract the last marked outcome from noisy editor stdout.
pub fn extract_marked_outcome(stdout: &str, marker: &str) -> Result<EvalOutcome, String> {
    let encoded = stdout
        .lines()
        .filter_map(|line| line.split_once(marker).map(|(_, encoded)| encoded.trim()))
        .next_back()
        .ok_or_else(|| format!("editor output did not contain outcome marker `{marker}`"))?;
    EvalOutcome::parse(encoded)
}

/// Wrap setup and probe forms in the shared result protocol.
///
/// Both inputs may contain multiple forms. The value of the probe's final
/// form is recursively normalized and printed with `prin1`; ordinary Lisp
/// errors are caught and their complete signal data is normalized and printed
/// instead. Workspace and per-engine sandbox roots come from the
/// `NEOMACS_TEST_WORKSPACE_ROOT` and `NEOMACS_TEST_SANDBOX_ROOT` environment
/// variables.
pub fn wrap_elisp_outcome(setup: &str, probe: &str, marker: &str) -> String {
    let marker = elisp_string(marker);
    format!(
        r##"(let ((print-circle t)
                  (print-length nil)
                  (print-level nil))
           (defun neomacs--test-oracle-normalize-string (value)
             (dolist
                 (root
                  (list
                   (cons (getenv "HOME") "[ORACLE-HOME]")
                   (cons (getenv "TMPDIR") "[ORACLE-TMPDIR]")
                   (cons (getenv "XDG_CONFIG_HOME") "[ORACLE-XDG-CONFIG]")
                   (cons (getenv "XDG_CACHE_HOME") "[ORACLE-XDG-CACHE]")
                   (cons (getenv "XDG_DATA_HOME") "[ORACLE-XDG-DATA]")
                   (cons (getenv "XDG_STATE_HOME") "[ORACLE-XDG-STATE]")
                   (cons (getenv "NEOMACS_TEST_SANDBOX_ROOT")
                         "[ORACLE-SANDBOX]")
                   (cons (getenv "NEOMACS_TEST_WORKSPACE_ROOT")
                         "[ORACLE-WORKSPACE]")))
               (when (and (stringp (car root))
                          (> (length (car root)) 1))
                 (setq value
                       (replace-regexp-in-string
                        (regexp-quote
                         (directory-file-name (car root)))
                        (cdr root)
                        value t t))))
             value)
           (defun neomacs--test-oracle-normalize (value seen)
             (cond
              ((stringp value)
               (neomacs--test-oracle-normalize-string value))
              ;; Some Neomacs runtime handles currently use integer IDs, so
              ;; predicates such as `windowp' can also accept ordinary small
              ;; integers. Preserve numeric values before probing opaque
              ;; runtime object predicates.
              ((numberp value) value)
              ((and (fboundp 'bufferp) (bufferp value))
               (list :buffer (buffer-name value)))
              ((and (fboundp 'markerp) (markerp value))
               (list :marker
                     (marker-position value)
                     (let ((buffer (marker-buffer value)))
                       (and buffer (buffer-name buffer)))))
              ((and (fboundp 'processp) (processp value))
               (list :process
                     (process-name value)
                     (process-status value)))
              ((and (fboundp 'windowp) (windowp value))
               (list :window
                     (let ((buffer (window-buffer value)))
                       (and buffer (buffer-name buffer)))))
              ((and (fboundp 'framep) (framep value))
               (list :frame
                     (frame-parameter value 'name)))
              ((consp value)
               (or (gethash value seen)
                   (let ((copy (cons nil nil)))
                     (puthash value copy seen)
                     (setcar
                      copy
                      (neomacs--test-oracle-normalize (car value) seen))
                     (setcdr
                      copy
                      (neomacs--test-oracle-normalize (cdr value) seen))
                     copy)))
              ((vectorp value)
               (or (gethash value seen)
                   (let* ((length (length value))
                          (copy (make-vector length nil)))
                     (puthash value copy seen)
                     (dotimes (index length)
                       (aset
                        copy index
                        (neomacs--test-oracle-normalize
                         (aref value index) seen)))
                     copy)))
              (t value)))
           (defun neomacs--test-oracle-normalized (value)
             (neomacs--test-oracle-normalize
              value
              (make-hash-table :test 'eq)))
           (condition-case neomacs--oracle-error
               (let ((neomacs--oracle-result
                      (progn
                        {setup}
                        {probe})))
                 (princ "\n")
                 (princ {marker})
                 (princ "OK ")
                 (prin1
                  (neomacs--test-oracle-normalized
                   neomacs--oracle-result)))
             (error
              (princ "\n")
              (princ {marker})
              (princ "ERR ")
              (prin1
               (neomacs--test-oracle-normalized
                neomacs--oracle-error)))))"##
    )
}

fn elisp_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
