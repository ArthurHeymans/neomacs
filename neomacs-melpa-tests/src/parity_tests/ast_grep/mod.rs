use std::time::Duration;

use crate::{AST_GREP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod backends;
mod candidates;
mod commands;
mod outline;
mod registry;
mod rewrite;
mod sync;

const AST_GREP_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AST_GREP_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'subr-x)

(defun ast-grep-test-path (filename)
  (expand-file-name filename (getenv "NEOMACS_TEST_SANDBOX_ROOT")))

(defun ast-grep-test-write-file (filename content)
  (let ((path (ast-grep-test-path filename)))
    (make-directory (file-name-directory path) t)
    (with-temp-file path
      (insert content))
    path))

(defun ast-grep-test-read-file (filename)
  (with-temp-buffer
    (insert-file-contents-literally filename)
    (buffer-string)))

(defun ast-grep-test-make-executable (name body)
  (let ((path (ast-grep-test-write-file
               (concat "bin/" name)
               (concat "#!/bin/sh\nset -eu\n" body "\n"))))
    (set-file-modes path #o755)
    path))

(defun ast-grep-test-error-data (thunk)
  (condition-case error-data
      (list :ok (funcall thunk))
    (error (list :error (car error-data) (cdr error-data)))))

(defun ast-grep-test-match-summary (candidate)
  (let ((match (ast-grep--candidate-match candidate)))
    (and match
         (list
          (plist-get match :file)
          (plist-get match :start-line)
          (plist-get match :start-column)
          (plist-get match :end-line)
          (plist-get match :end-column)
          (plist-get match :text)
          (plist-get match :replacement)))))

(defun ast-grep-test-kill-file-buffer (file)
  (when-let ((buffer (find-buffer-visiting file)))
    (with-current-buffer buffer
      (set-buffer-modified-p nil))
    (kill-buffer buffer)))
"##;

fn ast_grep_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AST_GREP_MELPA_PIN, source_file)
        .expect("prepare pinned ast-grep source below ./tmp")
        .with_prelude(AST_GREP_TEST_PRELUDE)
        .with_timeout(AST_GREP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ast-grep parity test")
        .into()
}

fn assert_ast_grep_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ast_grep_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ast-grep parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ast_grep_parity(elisp_form: &str, expected: Expect) {
    assert_ast_grep_source_parity("ast-grep.el", elisp_form, expected);
}

pub(crate) fn assert_ast_grep_consult_parity(elisp_form: &str, expected: Expect) {
    assert_ast_grep_source_parity("ast-grep-consult.el", elisp_form, expected);
}

pub(crate) fn assert_ast_grep_ivy_parity(elisp_form: &str, expected: Expect) {
    assert_ast_grep_source_parity("ast-grep-ivy.el", elisp_form, expected);
}

pub(crate) fn assert_ast_grep_helm_parity(elisp_form: &str, expected: Expect) {
    assert_ast_grep_source_parity("ast-grep-helm.el", elisp_form, expected);
}
