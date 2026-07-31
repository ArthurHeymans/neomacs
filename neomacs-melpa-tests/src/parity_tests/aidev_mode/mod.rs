use std::time::Duration;

use crate::{AIDEV_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const AIDEV_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aidev_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AIDEV_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned aidev-mode source below ./tmp")
        .with_prelude(
            r##"(setenv
                 "AIDEV_OLLAMA_ADDRESS"
                 "http://frozen-ollama.invalid:11434")"##,
        )
        .with_timeout(AIDEV_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aidev-mode parity test")
        .into()
}

fn assert_aidev_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aidev_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aidev-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aidev_mode_parity(elisp_form: &str, expected: Expect) {
    assert_aidev_mode_source_parity("aidev-mode.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_aidev_mode_parity` cases (2a).
pub(crate) fn assert_aidev_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        aidev_mode_oracle("aidev-mode.el"),
        &name,
        "aidev_mode_parity",
        cases,
    );
}
