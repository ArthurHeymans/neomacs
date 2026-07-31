use std::time::Duration;

use crate::{ANKI_VOCABULARY_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ANKI_VOCABULARY_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anki_vocabulary_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANKI_VOCABULARY_MELPA_PIN, source_file)
        .expect("prepare pinned anki-vocabulary source below ./tmp")
        .with_timeout(ANKI_VOCABULARY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anki-vocabulary parity test")
        .into()
}

fn assert_anki_vocabulary_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anki_vocabulary_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anki-vocabulary parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_anki_vocabulary_parity(elisp_form: &str, expected: Expect) {
    assert_anki_vocabulary_source_parity("anki-vocabulary.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_anki_vocabulary_parity` cases (2a).
pub(crate) fn assert_anki_vocabulary_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        anki_vocabulary_oracle("anki-vocabulary.el"),
        &name,
        "anki_vocabulary_parity",
        cases,
    );
}
