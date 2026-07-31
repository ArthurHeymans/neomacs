use std::time::Duration;

use crate::{ASYNC_MELPA_PIN, ASYNC_STATUS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod files;
mod items;
mod registry;
mod rendering;
mod workflows;

const ASYNC_STATUS_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ASYNC_STATUS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

;; Keep indicator tests independent of graphical-frame support.  Individual
;; cases inspect the complete call contracts exposed through these seams.
(defvar async-status-test-posframe-calls nil)
(defvar async-status-test-svg-calls nil)

(require 'posframe)

(defun posframe-show
    (&rest arguments)
  (push
   (cons 'show arguments)
   async-status-test-posframe-calls)
  :shown)

(defun posframe-hide
    (&rest arguments)
  (push
   (cons 'hide arguments)
  async-status-test-posframe-calls)
  :hidden)

(require 'svg-lib)

(setq svg-lib-style-default
      '(:background "test-background"
        :foreground "test-foreground"))

(defun svg-lib-progress-bar
    (&rest arguments)
  (push arguments async-status-test-svg-calls)
  (list 'image
        :type 'svg
        :progress (car arguments)))

(defun async-status-test-error
    (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun async-status-test-id-summary
    (id)
  (list
   (string-prefix-p async-status--file-prefix id)
   (and
    (string-match-p
     "\\`async-status-.*-[[:alnum:]]+\\'"
     id)
    t)
   (file-exists-p
    (async-status--get-absolute-path-by-id id))
   (async-status--get-msg-val id)))

(defun async-status-test-item-summary
    (item)
  (list
   (async-status--item-msg-id item)
   (async-status--item-fs-watcher-id item)
   (and
    (async-status--item-file-path item)
    (file-name-nondirectory
     (async-status--item-file-path item)))
   (async-status--item-progress item)
   (async-status--item-label item)))
"##;

fn async_status_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASYNC_STATUS_MELPA_PIN, source_file)
        .expect("prepare pinned async-status source and dependencies below ./tmp")
        .with_melpa_dependency(ASYNC_MELPA_PIN)
        .expect("prepare pinned async dependency below ./tmp")
        .with_prelude(ASYNC_STATUS_TEST_PRELUDE)
        .with_timeout(ASYNC_STATUS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed async-status parity test")
        .into()
}

fn assert_async_status_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async_status_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("async-status parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_async_status_parity(elisp_form: &str, expected: Expect) {
    assert_async_status_source_parity("async-status.el", elisp_form, expected);
}

pub(crate) fn assert_async_status_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_async_status_source_parity("async-status-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_async_status_autoload_parity` cases (2a).
pub(crate) fn assert_async_status_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_status_oracle("async-status-autoloads.el"),
        &name,
        "async_status_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_status_parity` cases (2a).
pub(crate) fn assert_async_status_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_status_oracle("async-status.el"),
        &name,
        "async_status_parity",
        cases,
    );
}
