use std::time::Duration;

use crate::{ALL_EXT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod conversion;
mod cursors;
mod integration;
mod navigation;
mod surface;

const ALL_EXT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn all_ext_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALL_EXT_MELPA_PIN, "all-ext.el")
        .expect("prepare pinned all-ext source and immutable dependencies below ./tmp")
        .with_timeout(ALL_EXT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed all-ext parity test")
        .into()
}

pub(crate) fn assert_all_ext_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = all_ext_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("all-ext parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_all_ext_signal(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = all_ext_oracle()
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("all-ext signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
