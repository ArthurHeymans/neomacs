use std::time::Duration;

use crate::{AUREL_MELPA_PIN, BUI_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod filters;
mod parsing;
mod registry;
mod urls;
mod workflows;

const AUREL_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUREL_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(setq temporary-file-directory "/fixture/scratch/"
      aurel-download-directory "/fixture/downloads/"
      aurel-pacman-program "/fixture/bin/pacman"
      aurel-installed-packages-check nil
      aurel-debug-level 0)

(defun aurel-test-error-data
    (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))
"##;

fn aurel_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUREL_MELPA_PIN, "aurel.el")
        .expect("prepare pinned aurel source and dependencies below ./tmp")
        .with_melpa_dependency(BUI_MELPA_PIN)
        .expect("prepare pinned BUI dependency below ./tmp")
        .with_prelude(AUREL_TEST_PRELUDE)
        .with_timeout(AUREL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aurel parity test").into()
}

fn assert_aurel_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(AUREL_MELPA_PIN, source_file)
        .expect("prepare pinned aurel source and dependencies below ./tmp")
        .with_melpa_dependency(BUI_MELPA_PIN)
        .expect("prepare pinned BUI dependency below ./tmp")
        .with_prelude(AUREL_TEST_PRELUDE)
        .with_timeout(AUREL_TEST_TIMEOUT)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aurel parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aurel_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aurel_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aurel parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aurel_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_aurel_source_parity("aurel-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_aurel_autoload_parity` cases (2a).
pub(crate) fn assert_aurel_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        aurel_oracle(),
        &name,
        "aurel_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_aurel_parity` cases (2a).
pub(crate) fn assert_aurel_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        aurel_oracle(),
        &name,
        "aurel_parity",
        cases,
    );
}
