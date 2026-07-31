use std::time::Duration;

use crate::{AUTO_ORG_MD_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod export;
mod lifecycle;
mod registry;
mod workflows;

const AUTO_ORG_MD_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_ORG_MD_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'org)
(require 'ox-md)

(defun auto-org-md-test-error (thunk)
  (condition-case error-data
      (list :value (funcall thunk))
    (error
     (list :signal (car error-data) (cdr error-data)))))

(defun auto-org-md-test-hook-count (function hook)
  (let ((count 0))
    (dolist (entry hook count)
      (when (eq entry function)
        (setq count (1+ count))))))

(defun auto-org-md-test-reset-state ()
  (put 'auto-org-md-mode 'state nil))

(defun auto-org-md-test-root (name)
  (let ((root
         (expand-file-name
          (concat "tmp/melpa-parity/auto-org-md/" name)
          (getenv "CARGO_WORKSPACE_DIR"))))
    (when (file-exists-p root)
      (delete-directory root t))
    (make-directory root t)
    root))

(defun auto-org-md-test-read-file (path)
  (with-temp-buffer
    (insert-file-contents path)
    (replace-regexp-in-string
     "org[[:xdigit:]]\\{7\\}"
     "org-ID"
     (buffer-string)
     t t)))
"##;

fn auto_org_md_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_ORG_MD_MELPA_PIN, source_file)
        .expect("prepare pinned auto-org-md source below ./tmp")
        .with_prelude(AUTO_ORG_MD_TEST_PRELUDE)
        .with_timeout(AUTO_ORG_MD_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-org-md parity test")
        .into()
}

fn assert_auto_org_md_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_org_md_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auto-org-md parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_org_md_parity(elisp_form: &str, expected: Expect) {
    assert_auto_org_md_source_parity("auto-org-md.el", elisp_form, expected);
}

pub(crate) fn assert_auto_org_md_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_org_md_source_parity("auto-org-md-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_auto_org_md_autoload_parity` cases (2a).
pub(crate) fn assert_auto_org_md_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_org_md_oracle("auto-org-md-autoloads.el"),
        &name,
        "auto_org_md_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_org_md_parity` cases (2a).
pub(crate) fn assert_auto_org_md_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_org_md_oracle("auto-org-md.el"),
        &name,
        "auto_org_md_parity",
        cases,
    );
}
