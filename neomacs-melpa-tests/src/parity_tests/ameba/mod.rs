use std::time::Duration;

use crate::{AMEBA_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod autoloads;
mod commands;
mod integration;
mod paths;
mod project;
mod surface;

const AMEBA_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ameba_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMEBA_MELPA_PIN, source_file)
        .expect("prepare pinned Ameba source below ./tmp")
        .with_timeout(AMEBA_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Ameba parity test").into()
}

fn assert_ameba_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ameba_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("Ameba parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ameba_parity(elisp_form: &str, expected: Expect) {
    assert_ameba_source_parity("ameba.el", elisp_form, expected);
}

pub(crate) fn assert_ameba_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ameba_source_parity("ameba-autoloads.el", elisp_form, expected);
}
