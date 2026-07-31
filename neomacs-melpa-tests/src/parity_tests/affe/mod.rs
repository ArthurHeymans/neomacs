use std::time::Duration;

use crate::{AFFE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod async_frontend;
mod autoloads;
mod backend_producer;
mod backend_protocol;
mod backend_search;
mod commands;
mod surface;
mod transport;
mod workflows;

const AFFE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn affe_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AFFE_MELPA_PIN, source_file)
        .expect("prepare pinned affe source and Consult dependency below ./tmp")
        .with_prelude("(require 'cl-lib)")
        .with_timeout(AFFE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed affe parity test").into()
}

fn assert_affe_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = affe_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("affe parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_affe_parity(elisp_form: &str, expected: Expect) {
    assert_affe_source_parity("affe.el", elisp_form, expected);
}

pub(crate) fn assert_affe_backend_parity(elisp_form: &str, expected: Expect) {
    assert_affe_source_parity("affe-backend.el", elisp_form, expected);
}

pub(crate) fn assert_affe_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_affe_source_parity("affe-autoloads.el", elisp_form, expected);
}







/// Multi-probe batch for `assert_affe_autoload_parity` cases (2a).
pub(crate) fn assert_affe_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        affe_oracle("affe-autoloads.el"),
        &name,
        "affe_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_affe_backend_parity` cases (2a).
pub(crate) fn assert_affe_backend_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        affe_oracle("affe-backend.el"),
        &name,
        "affe_backend_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_affe_parity` cases (2a).
pub(crate) fn assert_affe_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        affe_oracle("affe.el"),
        &name,
        "affe_parity",
        cases,
    );
}
