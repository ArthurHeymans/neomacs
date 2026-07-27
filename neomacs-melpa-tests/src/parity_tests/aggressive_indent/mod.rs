use std::time::Duration;

use crate::{AGGRESSIVE_INDENT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod indentation;
mod lifecycle;
mod registry;
mod timers;
mod tracking;

const AGGRESSIVE_INDENT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn aggressive_indent_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGGRESSIVE_INDENT_MELPA_PIN, source_file)
        .expect("prepare pinned aggressive-indent source below ./tmp")
        .with_timeout(AGGRESSIVE_INDENT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aggressive-indent parity test")
        .into()
}

fn assert_aggressive_indent_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aggressive_indent_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aggressive-indent parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aggressive_indent_parity(elisp_form: &str, expected: Expect) {
    assert_aggressive_indent_source_parity("aggressive-indent.el", elisp_form, expected);
}

pub(crate) fn assert_aggressive_indent_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_aggressive_indent_source_parity("aggressive-indent-autoloads.el", elisp_form, expected);
}
