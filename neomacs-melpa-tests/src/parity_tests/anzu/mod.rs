use std::time::Duration;

use crate::{ANZU_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod modes;
mod registry;
mod replace;
mod search;

const ANZU_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anzu_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANZU_MELPA_PIN, source_file)
        .expect("prepare pinned anzu source below ./tmp")
        .with_timeout(ANZU_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed anzu parity test").into()
}

fn assert_anzu_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anzu_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anzu parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anzu_parity(elisp_form: &str, expected: Expect) {
    assert_anzu_source_parity("anzu.el", elisp_form, expected);
}

pub(crate) fn assert_anzu_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anzu_source_parity("anzu-autoloads.el", elisp_form, expected);
}
