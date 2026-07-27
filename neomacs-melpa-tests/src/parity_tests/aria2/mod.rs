use std::time::Duration;

use crate::{ARIA2_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod api;
mod commands;
mod controller;
mod entries;
mod registry;
mod timers;
mod utils;

const ARIA2_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ARIA2_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun aria2-test-path
    (filename)
  (expand-file-name
   filename
   (getenv
    "NEOMACS_TEST_SANDBOX_ROOT")))

(defun aria2-test-controller
    (&optional request-id pid)
  (make-instance
   'aria2-controller
   "aria2-test-controller"
   :file
   (aria2-test-path
    "controller.eieio")
   :request-id
   (or request-id 0)
   :rcp-url
   "http://fixture.invalid:6800/jsonrpc"
   :secret
   "fixture-secret"
   :pid
   (or pid -1)))

(defun aria2-test-kill-buffers ()
  (dolist (name
           (list
            aria2-list-buffer-name
            aria2-url-list-buffer-name
            "*aria2-response*"))
    (when-let ((buffer
                (get-buffer name)))
      (with-current-buffer buffer
        (set-buffer-modified-p nil))
      (kill-buffer buffer))))
"##;

fn aria2_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARIA2_MELPA_PIN, source_file)
        .expect("prepare pinned aria2 source below ./tmp")
        .with_prelude(ARIA2_TEST_PRELUDE)
        .with_timeout(ARIA2_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aria2 parity test").into()
}

fn assert_aria2_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aria2_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aria2 parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aria2_parity(elisp_form: &str, expected: Expect) {
    assert_aria2_source_parity("aria2.el", elisp_form, expected);
}

pub(crate) fn assert_aria2_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_aria2_source_parity("aria2-autoloads.el", elisp_form, expected);
}
