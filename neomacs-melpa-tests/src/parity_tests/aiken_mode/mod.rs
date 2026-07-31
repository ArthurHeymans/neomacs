use std::time::Duration;

use crate::{AIKEN_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod editing;
mod font_lock;
mod mode;
mod workflows;

const AIKEN_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aiken_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AIKEN_MODE_MELPA_PIN, "aiken-mode.el")
        .expect("prepare pinned aiken-mode source below ./tmp")
        .with_prelude(
            r##"
(require 'cl-lib)
(require 'compile)
(require 'project)
(require 'thingatpt)
"##,
        )
        .with_timeout(AIKEN_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aiken-mode parity test")
        .into()
}

pub(crate) fn assert_aiken_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aiken_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aiken-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_aiken_mode_parity` cases (2a).
pub(crate) fn assert_aiken_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(aiken_mode_oracle(), &name, "aiken_mode_parity", cases);
}
