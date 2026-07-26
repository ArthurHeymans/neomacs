use std::time::Duration;

use crate::{AC_HASKELL_PROCESS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod availability;
mod candidates;
mod documentation;
mod popup;
mod setup;
mod surface;

const AC_HASKELL_PROCESS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_haskell_process_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_HASKELL_PROCESS_MELPA_PIN, "ac-haskell-process.el")
        .expect("prepare pinned ac-haskell-process source below ./tmp")
        .with_timeout(AC_HASKELL_PROCESS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-haskell-process parity test")
        .into()
}

pub(crate) fn assert_ac_haskell_process_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_haskell_process_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-haskell-process parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_haskell_process_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_haskell_process_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ac-haskell-process signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
