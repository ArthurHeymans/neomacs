//! Emacs-based macro expansion.
//!
//! Shells out to Emacs for `macroexpand-all`, producing correctly expanded
//!//! S-expressions for the compiler pipeline. Use this for macros that the
//! built-in mini-evaluator (`expand_eval`) cannot handle (e.g., `require`
//! chains, `define-derived-mode`, complex `cl-lib` macros).
//!
//! Known limitation: Emacs's `macroexpand-all` can produce variable name
//! collisions in `cl-loop` expansions (e.g., `--cl-var--` reused for both
//! iteration and accumulation). For `cl-loop` heavy code, the built-in
//! mini-evaluator may produce more correct results.

use std::io::Write;
use std::process::Command;

use crate::diagnostic::Diagnostic;
use crate::reader;
use crate::source::{SourceFile, SourceId};
use crate::surface::SurfaceForm;

pub struct EmacsExpandOutput {
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn expand_with_emacs(
    source_text: &str,
    file_name: &str,
    emacs_path: &str,
) -> Result<EmacsExpandOutput, String> {
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();

    let source_path = temp_dir.join(format!("neovm_src_{pid}.el"));
    {
        let mut f =
            std::fs::File::create(&source_path).map_err(|e| format!("create temp source: {e}"))?;
        f.write_all(source_text.as_bytes())
            .map_err(|e| format!("write temp source: {e}"))?;
    }

    let source_path_str = source_path.to_string_lossy().to_string();

    let eval_expr = format!(
        r#"(progn
  (require 'cl-lib)
  (let* ((file {source_path_str_quoted})
         (buf-data nil)
         (buf-lex nil)
         (pos 0)
         (forms nil))
    (with-temp-buffer
      (insert-file-contents file)
      (setq buf-data (buffer-string))
      (setq buf-lex (not (null (string-match "-\\*-.*lexical-binding:\\s-*t.*-\\*-" buf-data))))
      (setq-local lexical-binding buf-lex)
      (condition-case nil
          (while t
            (let ((result (read-from-string buf-data pos)))
              (push (car result) forms)
              (setq pos (cdr result))))
        (error nil))
      (dolist (form (nreverse forms))
        (when (memq (car-safe form)
                    '(require defmacro define-derived-mode
                      defalias fset autoload declare-function
                      defvar defcustom defgroup defface eval-when-compile))
          (condition-case nil (eval form buf-lex) (error nil)))
        (let ((expanded (condition-case nil (macroexpand-all form) (error form))))
          (prin1 expanded)
          (terpri)))))
  (kill-emacs 0))"#,
        source_path_str_quoted = double_quote_elisp_string(&source_path_str),
    );

    let script_path = temp_dir.join(format!("neovm_expand_{pid}.el"));
    {
        let mut f =
            std::fs::File::create(&script_path).map_err(|e| format!("create temp script: {e}"))?;
        f.write_all(eval_expr.as_bytes())
            .map_err(|e| format!("write temp script: {e}"))?;
    }

    let output = Command::new(emacs_path)
        .arg("--batch")
        .arg("--no-site-file")
        .arg("--no-splash")
        .arg("-l")
        .arg(&script_path)
        .output()
        .map_err(|e| format!("failed to run emacs at '{emacs_path}': {e}"))?;

    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&script_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit = output
            .status
            .code()
            .map_or_else(|| "signal".to_string(), |c| c.to_string());
        return Err(format!("emacs exited with code {exit}:\n{stderr}"));
    }

    let expanded_text =
        String::from_utf8(output.stdout).map_err(|e| format!("emacs output not utf-8: {e}"))?;

    let expanded_source = SourceFile::new(
        SourceId::new(0),
        Some(format!("{file_name} (expanded)")),
        expanded_text,
    );
    let reader_output = reader::read_source(&expanded_source);

    Ok(EmacsExpandOutput {
        forms: reader_output.forms,
        diagnostics: reader_output.diagnostics,
    })
}

fn double_quote_elisp_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
