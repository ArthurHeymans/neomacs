use std::time::Duration;

use crate::{AC_EMMET_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod document;
mod setup;
mod surface;

const AC_EMMET_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AC_EMMET_LOOP_COMPATIBILITY_PRELUDE: &str =
    "(require 'cl-lib) (defalias 'loop (symbol-function 'cl-loop))";

fn ac_emmet_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_EMMET_MELPA_PIN, "ac-emmet.el")
        .expect("prepare pinned ac-emmet source below ./tmp")
        .with_prelude(AC_EMMET_LOOP_COMPATIBILITY_PRELUDE)
        .with_timeout(AC_EMMET_TEST_TIMEOUT)
}

fn unshimmed_ac_emmet_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_EMMET_MELPA_PIN, "ac-emmet.el")
        .expect("prepare pinned unshimmed ac-emmet source below ./tmp")
        .with_timeout(AC_EMMET_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-emmet parity test")
        .into()
}

pub(crate) fn assert_ac_emmet_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_emmet_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-emmet parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_unshimmed_ac_emmet_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = unshimmed_ac_emmet_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("unshimmed ac-emmet signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
