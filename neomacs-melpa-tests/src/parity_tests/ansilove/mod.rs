use std::time::Duration;

use crate::{ANSILOVE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod conversion;
mod filesystem;
mod mode;
mod registry;
mod workflow;

const ANSILOVE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn ansilove_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANSILOVE_MELPA_PIN, source_file)
        .expect("prepare pinned ansilove source below ./tmp")
        .with_timeout(ANSILOVE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ansilove parity test")
        .into()
}

fn assert_ansilove_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ansilove_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ansilove parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ansilove_parity(elisp_form: &str, expected: Expect) {
    assert_ansilove_source_parity("ansilove.el", elisp_form, expected);
}

pub(crate) fn assert_ansilove_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ansilove_oracle("ansilove.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ansilove signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ansilove_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ansilove_source_parity("ansilove-autoloads.el", elisp_form, expected);
}
