use std::time::Duration;

use crate::{ANKI_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const ANKI_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anki_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANKI_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned anki-mode source below ./tmp")
        .with_timeout(ANKI_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anki-mode parity test")
        .into()
}

fn assert_anki_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anki_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anki-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_anki_mode_parity(elisp_form: &str, expected: Expect) {
    assert_anki_mode_source_parity("anki-mode.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_anki_mode_parity` cases (2a).
pub(crate) fn assert_anki_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        anki_mode_oracle("anki-mode.el"),
        &name,
        "anki_mode_parity",
        cases,
    );
}
