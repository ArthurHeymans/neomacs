use std::time::Duration;

use crate::{AMSREFTEX_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod integration;
mod lifecycle;
mod parsing;
mod registry;
mod sorting;

const AMSREFTEX_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn amsreftex_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMSREFTEX_MELPA_PIN, source_file)
        .expect("prepare pinned amsreftex source below ./tmp")
        .with_timeout(AMSREFTEX_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed amsreftex parity test")
        .into()
}

fn assert_amsreftex_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = amsreftex_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("amsreftex parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_amsreftex_parity(elisp_form: &str, expected: Expect) {
    assert_amsreftex_source_parity("amsreftex.el", elisp_form, expected);
}

pub(crate) fn assert_amsreftex_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_amsreftex_source_parity("amsreftex-autoloads.el", elisp_form, expected);
}
