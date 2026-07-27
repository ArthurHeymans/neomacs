use std::time::Duration;

use crate::{ANYINS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod insertion;
mod mode;
mod positions;
mod registry;
mod upstream_features;

const ANYINS_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anyins_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANYINS_MELPA_PIN, source_file)
        .expect("prepare pinned anyins source below ./tmp")
        .with_timeout(ANYINS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed anyins parity test").into()
}

fn assert_anyins_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anyins_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anyins parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anyins_parity(elisp_form: &str, expected: Expect) {
    assert_anyins_source_parity("anyins.el", elisp_form, expected);
}

pub(crate) fn assert_anyins_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anyins_oracle("anyins.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anyins signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anyins_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anyins_source_parity("anyins-autoloads.el", elisp_form, expected);
}
