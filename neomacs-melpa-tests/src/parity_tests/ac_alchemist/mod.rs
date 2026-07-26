use std::time::Duration;

use crate::{AC_ALCHEMIST_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod candidates;
mod integration;
mod requests;

const AC_ALCHEMIST_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_alchemist_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_ALCHEMIST_MELPA_PIN, "ac-alchemist.el")
        .expect("prepare pinned ac-alchemist source below ./tmp")
        .with_prelude(
            r##"(defvar byte-compile-current-file nil
                   "Compatibility declaration for Alchemist's legacy macros.")"##,
        )
        .with_timeout(AC_ALCHEMIST_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-alchemist parity test")
        .into()
}

pub(crate) fn assert_ac_alchemist_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_alchemist_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-alchemist parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_alchemist_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_alchemist_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ac-alchemist signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
