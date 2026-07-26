use std::time::Duration;

use crate::{ABC_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod editing;
mod processes;
mod songs;
mod surface;

const ABC_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn abc_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABC_MODE_MELPA_PIN, "abc-mode.el")
        .expect("prepare pinned abc-mode source below ./tmp")
        .with_timeout(ABC_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abc-mode parity test")
        .into()
}

pub(crate) fn assert_abc_mode_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abc_mode_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("abc-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_abc_mode_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abc_mode_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("abc-mode signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
