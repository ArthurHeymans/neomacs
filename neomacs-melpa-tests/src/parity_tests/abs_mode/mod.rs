use std::time::Duration;

use crate::{ABS_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod compilation;
mod diagnostics;
mod navigation;
mod parsing;
mod runtime;
mod surface;

const ABS_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn abs_mode_oracle(source_file_name: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABS_MODE_MELPA_PIN, source_file_name)
        .expect("prepare pinned abs-mode source below ./tmp")
        .with_timeout(ABS_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abs-mode parity test")
        .into()
}

pub(crate) fn assert_abs_mode_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abs_mode_oracle("abs-mode.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("abs-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_abs_mode_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abs_mode_oracle("abs-mode.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("abs-mode signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
