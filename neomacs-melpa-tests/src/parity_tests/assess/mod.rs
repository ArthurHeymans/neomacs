use std::time::Duration;

use crate::{ASSESS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod buffers;
mod call_capture;
mod comparison;
mod discovery;
mod faces;
mod filesystem;
mod indentation;
mod registry;
mod robot;

const ASSESS_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ASSESS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun assess-test-path (filename)
  (expand-file-name
   filename
   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))

(defun assess-test-read-file (filename)
  (with-temp-buffer
    (insert-file-contents-literally filename)
    (buffer-string)))
"##;

fn assess_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASSESS_MELPA_PIN, source_file)
        .expect("prepare pinned assess source below ./tmp")
        .with_prelude(ASSESS_TEST_PRELUDE)
        .with_timeout(ASSESS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed assess parity test").into()
}

fn assert_assess_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = assess_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("assess parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_assess_parity(elisp_form: &str, expected: Expect) {
    assert_assess_source_parity("assess.el", elisp_form, expected);
}

pub(crate) fn assert_assess_call_parity(elisp_form: &str, expected: Expect) {
    assert_assess_source_parity("assess-call.el", elisp_form, expected);
}

pub(crate) fn assert_assess_discover_parity(elisp_form: &str, expected: Expect) {
    assert_assess_source_parity("assess-discover.el", elisp_form, expected);
}

pub(crate) fn assert_assess_robot_parity(elisp_form: &str, expected: Expect) {
    assert_assess_source_parity("assess-robot.el", elisp_form, expected);
}

pub(crate) fn assert_assess_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_assess_source_parity("assess-autoloads.el", elisp_form, expected);
}
