use std::time::Duration;

use crate::{ANKI_EDITOR_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ANKI_EDITOR_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anki_editor_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANKI_EDITOR_MELPA_PIN, source_file)
        .expect("prepare pinned anki-editor source below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq exec-path nil)
                   (setenv "PATH" ""))"##,
        )
        .with_timeout(ANKI_EDITOR_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anki-editor parity test")
        .into()
}

fn assert_anki_editor_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anki_editor_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anki-editor parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_anki_editor_parity(elisp_form: &str, expected: Expect) {
    assert_anki_editor_source_parity("anki-editor.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_anki_editor_parity` cases (2a).
pub(crate) fn assert_anki_editor_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        anki_editor_oracle("anki-editor.el"),
        &name,
        "anki_editor_parity",
        cases,
    );
}
