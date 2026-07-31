use std::time::Duration;

use crate::{ANKI_CONNECT_MELPA_PIN, CachedMelpaOracle, S_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ANKI_CONNECT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn anki_connect_oracle(
    source_file: &str,
    include_undeclared_s_dependency: bool,
) -> CachedMelpaOracle {
    let oracle = CachedMelpaOracle::new(ANKI_CONNECT_MELPA_PIN, source_file)
        .expect("prepare pinned anki-connect source below ./tmp");
    let oracle = if include_undeclared_s_dependency {
        oracle
            .with_melpa_dependency(S_MELPA_PIN)
            .expect("prepare anki-connect's undeclared s dependency below ./tmp")
            .with_prelude("(require 's)")
    } else {
        oracle
    };
    oracle.with_timeout(ANKI_CONNECT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anki-connect parity test")
        .into()
}

fn assert_anki_connect_source_parity(
    source_file: &str,
    include_undeclared_s_dependency: bool,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = anki_connect_oracle(source_file, include_undeclared_s_dependency)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anki-connect parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anki_connect_parity(elisp_form: &str, expected: Expect) {
    assert_anki_connect_source_parity("anki-connect.el", true, elisp_form, expected);
}

pub(crate) fn assert_anki_connect_missing_dependency_signal(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anki_connect_oracle("anki-connect.el", false)
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("anki-connect missing-dependency parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}





/// Multi-probe batch for `assert_anki_connect_parity` cases (2a).
pub(crate) fn assert_anki_connect_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        anki_connect_oracle("anki-connect.el", true),
        &name,
        "anki_connect_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_anki_connect_missing_dependency_signal` cases (2a).
pub(crate) fn assert_anki_connect_missing_dependency_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        anki_connect_oracle("anki-connect.el", false),
        &name,
        "anki_connect_missing_dependency_signal",
        cases,
    );
}
