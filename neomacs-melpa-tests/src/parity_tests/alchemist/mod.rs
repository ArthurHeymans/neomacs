use std::time::Duration;

use crate::{ALCHEMIST_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod completion;
mod eval;
mod help_navigation;
mod mix_test;
mod modes;
mod process;
mod project;
mod surface;
mod utils_scope;

const ALCHEMIST_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn alchemist_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALCHEMIST_MELPA_PIN, "alchemist.el")
        .expect("prepare pinned Alchemist source and dependencies below ./tmp")
        .with_prelude(
            r##"(defvar byte-compile-current-file nil
                   "Compatibility declaration for Alchemist's legacy macros.")"##,
        )
        .with_timeout(ALCHEMIST_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed Alchemist parity test")
        .into()
}

pub(crate) fn assert_alchemist_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alchemist_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("Alchemist parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
