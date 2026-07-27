use std::time::Duration;

use crate::{AG_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod dired;
mod mode;
mod pure;
mod registry;
mod search;

const AG_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ag_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AG_MELPA_PIN, source_file)
        .expect("prepare pinned ag source below ./tmp")
        .with_timeout(AG_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ag parity test").into()
}

fn assert_ag_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ag_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ag parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ag_parity(elisp_form: &str, expected: Expect) {
    assert_ag_source_parity("ag.el", elisp_form, expected);
}

pub(crate) fn assert_ag_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ag_source_parity("ag-autoloads.el", elisp_form, expected);
}
