use std::time::Duration;

use crate::{
    ALERT_MELPA_PIN, ALERT_TERMUX_MELPA_PIN, CachedMelpaOracle, EmacsRuntime,
    prepare_cached_locked_melpa_package,
};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod registry;
mod workflow;

const ALERT_TERMUX_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn alert_termux_oracle(source_file: &str) -> CachedMelpaOracle {
    let oracle = CachedMelpaOracle::new(ALERT_TERMUX_MELPA_PIN, source_file)
        .expect("prepare pinned alert-termux source below ./tmp")
        .with_timeout(ALERT_TERMUX_TEST_TIMEOUT);
    if source_file == "alert-termux.el" {
        let alert_directory =
            prepare_cached_locked_melpa_package(&EmacsRuntime::gnu_emacs(), ALERT_MELPA_PIN)
                .expect("prepare exact alert dependency below ./tmp");
        let alert_source = alert_directory.join("alert.el");
        oracle.with_prelude(format!(
            "(load {:?} nil t t)",
            alert_source.to_string_lossy()
        ))
    } else {
        oracle
    }
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alert-termux parity test")
        .into()
}

fn assert_alert_termux_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alert_termux_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alert-termux parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alert_termux_parity(elisp_form: &str, expected: Expect) {
    assert_alert_termux_source_parity("alert-termux.el", elisp_form, expected);
}

pub(crate) fn assert_alert_termux_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alert_termux_source_parity("alert-termux-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_alert_termux_autoload_parity` cases (2a).
pub(crate) fn assert_alert_termux_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        alert_termux_oracle("alert-termux-autoloads.el"),
        &name,
        "alert_termux_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_alert_termux_parity` cases (2a).
pub(crate) fn assert_alert_termux_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        alert_termux_oracle("alert-termux.el"),
        &name,
        "alert_termux_parity",
        cases,
    );
}
