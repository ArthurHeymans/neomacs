use std::time::Duration;

use crate::{AGTAGS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod modes;
mod process;
mod registry;
mod xref;

const AGTAGS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn agtags_oracle(source_file: &str) -> CachedMelpaOracle {
    let oracle = CachedMelpaOracle::new(AGTAGS_MELPA_PIN, source_file)
        .expect("prepare pinned agtags source below ./tmp")
        .with_timeout(AGTAGS_TEST_TIMEOUT);
    if source_file == "agtags-autoloads.el" {
        oracle.with_prelude("(require 'grep) (require 'compile)")
    } else {
        oracle
    }
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed agtags parity test").into()
}

fn assert_agtags_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = agtags_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("agtags parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_agtags_parity(elisp_form: &str, expected: Expect) {
    assert_agtags_source_parity("agtags.el", elisp_form, expected);
}

pub(crate) fn assert_agtags_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_agtags_source_parity("agtags-autoloads.el", elisp_form, expected);
}
