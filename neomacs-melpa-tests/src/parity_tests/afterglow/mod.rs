use std::time::Duration;

use crate::{AFTERGLOW_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod advice;
mod autoloads;
mod mode;
mod overlays;
mod surface;
mod triggers;

const AFTERGLOW_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn afterglow_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AFTERGLOW_MELPA_PIN, source_file)
        .expect("prepare pinned afterglow source below ./tmp")
        .with_timeout(AFTERGLOW_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed afterglow parity test")
        .into()
}

fn assert_afterglow_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = afterglow_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("afterglow parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_afterglow_parity(elisp_form: &str, expected: Expect) {
    assert_afterglow_source_parity("afterglow.el", elisp_form, expected);
}

pub(crate) fn assert_afterglow_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_afterglow_source_parity("afterglow-autoloads.el", elisp_form, expected);
}
