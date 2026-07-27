use std::time::Duration;

use crate::{ANAKONDO_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod autoloads;
mod cache;
mod completion;
mod external;
mod java;
mod lifecycle;
mod surface;

const ANAKONDO_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn anakondo_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANAKONDO_MELPA_PIN, source_file)
        .expect("prepare pinned anakondo source below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq exec-path nil)
                   (setenv "PATH" ""))"##,
        )
        .with_timeout(ANAKONDO_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anakondo parity test")
        .into()
}

fn assert_anakondo_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anakondo_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anakondo parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anakondo_parity(elisp_form: &str, expected: Expect) {
    assert_anakondo_source_parity("anakondo.el", elisp_form, expected);
}

pub(crate) fn assert_anakondo_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anakondo_source_parity("anakondo-autoloads.el", elisp_form, expected);
}
