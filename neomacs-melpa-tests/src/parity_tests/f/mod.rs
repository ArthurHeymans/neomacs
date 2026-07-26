use std::time::Duration;

use crate::{CachedMelpaOracle, F_MELPA_PIN};
use expect_test::Expect;

mod filesystem;
mod io;
mod paths;

const F_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn f_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(F_MELPA_PIN, "f.el")
        .expect("prepare pinned f source and dependencies below ./tmp")
        .with_timeout(F_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed f parity test").into()
}

pub(crate) fn assert_f_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = f_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("f parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_f_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = f_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("f signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
