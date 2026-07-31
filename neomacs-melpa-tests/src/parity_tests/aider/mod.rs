use std::time::Duration;

use crate::{AIDER_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod core;
mod editing;
mod files;
mod registry;
mod workflows;

const AIDER_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn aider_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AIDER_MELPA_PIN, source_file)
        .expect("prepare pinned aider source below ./tmp")
        .with_timeout(AIDER_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed aider parity test").into()
}

fn assert_aider_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aider_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aider parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aider_parity(elisp_form: &str, expected: Expect) {
    assert_aider_source_parity("aider.el", elisp_form, expected);
}

pub(crate) fn assert_aider_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_aider_source_parity("aider-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_aider_helm_parity(elisp_form: &str, expected: Expect) {
    assert_aider_source_parity("aider-helm.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_aider_autoload_parity` cases (2a).
pub(crate) fn assert_aider_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        aider_oracle("aider-autoloads.el"),
        &name,
        "aider_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_aider_parity` cases (2a).
pub(crate) fn assert_aider_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(aider_oracle("aider.el"), &name, "aider_parity", cases);
}

/// Multi-probe batch for `assert_aider_helm_parity` cases (2a).
pub(crate) fn assert_aider_helm_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        aider_oracle("aider-helm.el"),
        &name,
        "aider_helm_parity",
        cases,
    );
}
