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

/// One case id paired with its parsed editor outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkedBatchOutcome {
    pub id: String,
    pub outcome: EvalOutcome,
}

/// Extract every batch outcome line `MARKER<id>:<OK|ERR> …` in order.
///
/// Case ids must not contain `:`. Duplicate ids in the same stdout are an error.
pub fn extract_marked_batch_outcomes(
    stdout: &str,
    marker: &str,
) -> Result<Vec<MarkedBatchOutcome>, String> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in stdout.lines() {
        let Some(rest) = line.split_once(marker).map(|(_, rest)| rest.trim()) else {
            continue;
        };
        // Skip non-batch single-outcome lines that lack `id:`.
        let Some((id, encoded)) = rest.split_once(':') else {
            continue;
        };
        let id = id.trim();
        if id.is_empty() {
            return Err(format!(
                "editor output contained an empty batch outcome id after `{marker}`"
            ));
        }
        if !seen.insert(id.to_string()) {
            return Err(format!(
                "editor output contained duplicate batch outcome id `{id}`"
            ));
        }
        results.push(MarkedBatchOutcome {
            id: id.to_string(),
            outcome: EvalOutcome::parse(encoded.trim())?,
        });
    }
    if results.is_empty() {
        return Err(format!(
            "editor output did not contain any batch outcome markers `{marker}<id>:`"
        ));
    }
    Ok(results)
}

/// Lisp that defines the shared normalizer used by single- and multi-probe wrappers.
fn oracle_normalizer_elisp() -> &'static str {
    r##"(defun neomacs--test-oracle-normalize-string (value)
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
               ;; Walk the cdr chain iteratively.  This function's recursion
               ;; depth must track how deeply a value nests, not how long a
               ;; list is: recursing on the cdr cost one frame per cons, so a
               ;; flat list of 316 elements exhausted `max-lisp-eval-depth'
               ;; and the overflow surfaced from inside the oracle's own
               ;; error handler, indistinguishable from the package under
               ;; test signalling.  Each cons is still registered in SEEN
               ;; before its car is normalized, so shared structure and
               ;; cycles resolve exactly as before.
               (or (gethash value seen)
                   (let* ((copy (cons nil nil))
                          (tail copy)
                          (rest (cdr value)))
                     (puthash value copy seen)
                     (setcar
                      copy
                      (neomacs--test-oracle-normalize (car value) seen))
                     (while (and (consp rest) (not (gethash rest seen)))
                       (let ((next (cons nil nil)))
                         (puthash rest next seen)
                         (setcar
                          next
                          (neomacs--test-oracle-normalize (car rest) seen))
                         (setcdr tail next)
                         (setq tail next
                               rest (cdr rest))))
                     (setcdr
                      tail
                      (neomacs--test-oracle-normalize rest seen))
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
              (make-hash-table :test 'eq)))"##
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
                  (print-level nil)
                  (print-escape-newlines t)
                  (print-escape-control-characters t))
           {normalizer}
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
                neomacs--oracle-error)))))"##,
        normalizer = oracle_normalizer_elisp(),
        setup = setup,
        probe = probe,
        marker = marker,
    )
}

/// One named probe embedded in a multi-probe batch process.
#[derive(Clone, Copy, Debug)]
pub struct BatchProbe<'a> {
    /// Stable case id. Must be non-empty and must not contain `:`.
    pub id: &'a str,
    /// Elisp forms evaluated after shared setup; final value is the outcome.
    pub probe: &'a str,
}

/// Wrap shared setup plus many named probes for one editor process.
///
/// Setup runs once. Each probe is wrapped in its own `condition-case` so a
/// signal in one case does not stop later cases. Emitted lines look like:
///
/// ```text
/// <marker><id>:OK …
/// <marker><id>:ERR …
/// ```
pub fn wrap_elisp_batch_outcomes(
    setup: &str,
    cases: &[BatchProbe<'_>],
    marker: &str,
) -> Result<String, String> {
    if cases.is_empty() {
        return Err("batch outcomes require at least one probe".into());
    }
    let marker_lit = elisp_string(marker);
    let mut body = String::new();
    body.push_str(setup);
    body.push('\n');
    let mut seen = std::collections::HashSet::new();
    for case in cases {
        validate_batch_case_id(case.id)?;
        if !seen.insert(case.id) {
            return Err(format!("duplicate batch case id `{}`", case.id));
        }
        let id_lit = elisp_string(case.id);
        body.push_str(&format!(
            r##"
           (condition-case neomacs--oracle-error
               (let ((neomacs--oracle-result
                      (progn
                        {probe})))
                 (princ "\n")
                 (princ {marker})
                 (princ {id})
                 (princ ":")
                 (princ "OK ")
                 (prin1
                  (neomacs--test-oracle-normalized
                   neomacs--oracle-result)))
             (error
              (princ "\n")
              (princ {marker})
              (princ {id})
              (princ ":")
              (princ "ERR ")
              (prin1
               (neomacs--test-oracle-normalized
                neomacs--oracle-error))))
"##,
            probe = case.probe,
            marker = marker_lit,
            id = id_lit,
        ));
    }

    Ok(format!(
        r##"(let ((print-circle t)
                  (print-length nil)
                  (print-level nil)
                  (print-escape-newlines t)
                  (print-escape-control-characters t))
           {normalizer}
           (progn
             {body}))"##,
        normalizer = oracle_normalizer_elisp(),
        body = body,
    ))
}

/// Reject empty ids and ids that would break the `MARKER<id>:` wire format.
pub fn validate_batch_case_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("batch case id must not be empty".into());
    }
    if id.contains(':') {
        return Err(format!(
            "batch case id `{id}` must not contain ':' (reserved by the batch outcome protocol)"
        ));
    }
    if id.chars().any(|c| c.is_whitespace()) {
        return Err(format!("batch case id `{id}` must not contain whitespace"));
    }
    Ok(())
}

fn elisp_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
