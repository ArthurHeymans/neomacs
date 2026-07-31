use std::time::Duration;

use crate::{ARSCRIPT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const ARSCRIPT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arscript_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARSCRIPT_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned arscript-mode source below ./tmp")
        .with_timeout(ARSCRIPT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arscript-mode parity test")
        .into()
}

fn assert_arscript_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arscript_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arscript-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arscript_mode_parity(elisp_form: &str, expected: Expect) {
    assert_arscript_mode_source_parity("arscript-mode.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_arscript_mode_parity` cases (2a).
pub(crate) fn assert_arscript_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        arscript_mode_oracle("arscript-mode.el"),
        &name,
        "arscript_mode_parity",
        cases,
    );
}
