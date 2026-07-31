use std::time::Duration;

use crate::{AGDA_EDITOR_TACTICS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod indentation;
mod mode;
mod parsing;
mod registry;
mod rendering;
mod workflows;

const AGDA_EDITOR_TACTICS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn agda_editor_tactics_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGDA_EDITOR_TACTICS_MELPA_PIN, source_file)
        .expect("prepare pinned agda-editor-tactics source below ./tmp")
        .with_timeout(AGDA_EDITOR_TACTICS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed agda-editor-tactics parity test")
        .into()
}

fn assert_agda_editor_tactics_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agda_editor_tactics_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("agda-editor-tactics parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_agda_editor_tactics_parity(elisp_form: &str, expected: Expect) {
    assert_agda_editor_tactics_source_parity("agda-editor-tactics.el", elisp_form, expected);
}

pub(crate) fn assert_agda_editor_tactics_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_agda_editor_tactics_source_parity(
        "agda-editor-tactics-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_agda_editor_tactics_autoload_parity` cases (2a).
pub(crate) fn assert_agda_editor_tactics_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        agda_editor_tactics_oracle("agda-editor-tactics-autoloads.el"),
        &name,
        "agda_editor_tactics_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_agda_editor_tactics_parity` cases (2a).
pub(crate) fn assert_agda_editor_tactics_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        agda_editor_tactics_oracle("agda-editor-tactics.el"),
        &name,
        "agda_editor_tactics_parity",
        cases,
    );
}
