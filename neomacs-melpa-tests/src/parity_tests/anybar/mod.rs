use std::time::Duration;

use crate::{ANYBAR_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod images;
mod interactive;
mod network;
mod surface;

const ANYBAR_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn anybar_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANYBAR_MELPA_PIN, source_file)
        .expect("prepare pinned anybar source below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq exec-path nil)
                   (setenv "PATH" ""))"##,
        )
        .with_timeout(ANYBAR_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed anybar parity test").into()
}

fn assert_anybar_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anybar_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anybar parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anybar_parity(elisp_form: &str, expected: Expect) {
    assert_anybar_source_parity("anybar.el", elisp_form, expected);
}

pub(crate) fn assert_anybar_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anybar_source_parity("anybar-autoloads.el", elisp_form, expected);
}
