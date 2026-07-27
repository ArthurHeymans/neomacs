use std::time::Duration;

use crate::{AGENIX_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod decrypt;
mod filesystem;
mod mode;
mod process;
mod registry;
mod save;

const AGENIX_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn agenix_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGENIX_MELPA_PIN, source_file)
        .expect("prepare pinned agenix source below ./tmp")
        .with_timeout(AGENIX_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed agenix parity test").into()
}

fn assert_agenix_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agenix_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agenix parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_agenix_parity(elisp_form: &str, expected: Expect) {
    assert_agenix_source_parity("agenix.el", elisp_form, expected);
}

pub(crate) fn assert_agenix_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_agenix_source_parity("agenix-autoloads.el", elisp_form, expected);
}
