use std::time::Duration;

use crate::{ANGULAR_SNIPPETS_MELPA_PIN, CachedMelpaOracle, YASNIPPET_MELPA_PIN};
use expect_test::Expect;

mod docs;
mod html_snippets;
mod javascript_snippets;
mod markup;
mod registry;

const ANGULAR_SNIPPETS_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn angular_snippets_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANGULAR_SNIPPETS_MELPA_PIN, source_file)
        .expect("prepare pinned angular-snippets source below ./tmp")
        .with_melpa_dependency(YASNIPPET_MELPA_PIN)
        .expect("prepare pinned Yasnippet dependency below ./tmp")
        .with_timeout(ANGULAR_SNIPPETS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed angular-snippets parity test")
        .into()
}

pub(crate) fn assert_angular_snippets_parity(elisp_form: &str, expected: Expect) {
    assert_angular_snippets_source_parity("angular-snippets.el", elisp_form, expected);
}

pub(crate) fn assert_angular_snippets_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_angular_snippets_source_parity("angular-snippets-autoloads.el", elisp_form, expected);
}

fn assert_angular_snippets_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = angular_snippets_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("angular-snippets parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
