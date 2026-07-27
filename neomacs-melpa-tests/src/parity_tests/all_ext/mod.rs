use std::time::Duration;

use crate::{ALL_EXT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod conversion;
mod cursors;
mod integration;
mod navigation;
mod surface;

const ALL_EXT_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ALL_DEPENDENCY: (&str, &str, &str, &str) = (
    "all",
    "1.0",
    "https://raw.githubusercontent.com/conao3/all.el/d3f5a18962170c69cbff9dbbabcf07acfa2763f8/all.el",
    "4a46ebedd5e64488428976510cec57d7b0b6ab378b8c60e9b08feee2df4e7d38",
);

fn all_ext_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new_with_melpa_url_dependency(
        ALL_EXT_MELPA_PIN,
        "all-ext.el",
        ALL_DEPENDENCY,
    )
    .expect("prepare pinned all-ext source and dependencies below ./tmp")
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
