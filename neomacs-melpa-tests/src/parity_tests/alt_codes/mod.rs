use std::time::Duration;

use crate::{ALT_CODES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod data;
mod hooks;
mod modes;
mod registry;

const ALT_CODES_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn alt_codes_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALT_CODES_MELPA_PIN, source_file)
        .expect("prepare pinned alt-codes source below ./tmp")
        // The package constructs a 383-arm `pcase` dynamically.  Loading the
        // interpreted source, as this differential harness intentionally
        // does, needs more than GNU Emacs's default evaluation depth.
        .with_prelude("(setq max-lisp-eval-depth 10000)")
        .with_timeout(ALT_CODES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alt-codes parity test")
        .into()
}

fn assert_alt_codes_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alt_codes_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alt-codes parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alt_codes_parity(elisp_form: &str, expected: Expect) {
    assert_alt_codes_source_parity("alt-codes.el", elisp_form, expected);
}

pub(crate) fn assert_alt_codes_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alt_codes_source_parity("alt-codes-autoloads.el", elisp_form, expected);
}
