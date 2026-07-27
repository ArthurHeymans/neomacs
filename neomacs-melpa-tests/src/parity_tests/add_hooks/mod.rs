use std::time::Duration;

use crate::{ADD_HOOKS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod helpers;
mod pair;
mod pairs;
mod registry;

const ADD_HOOKS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn add_hooks_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADD_HOOKS_MELPA_PIN, source_file)
        .expect("prepare pinned add-hooks source below ./tmp")
        .with_timeout(ADD_HOOKS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed add-hooks parity test")
        .into()
}

fn assert_add_hooks_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = add_hooks_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("add-hooks parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_add_hooks_parity(elisp_form: &str, expected: Expect) {
    assert_add_hooks_source_parity("add-hooks.el", elisp_form, expected);
}

pub(crate) fn assert_add_hooks_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_add_hooks_source_parity("add-hooks-autoloads.el", elisp_form, expected);
}
