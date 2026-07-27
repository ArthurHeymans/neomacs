use std::time::Duration;

use crate::{AMREAD_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod autoloads;
mod lifecycle;
mod surface;
mod text;
mod voice;

const AMREAD_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn amread_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMREAD_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned amread-mode source and dependencies below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq exec-path nil)
                   (setenv "PATH" ""))"##,
        )
        .with_timeout(AMREAD_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed amread-mode parity test")
        .into()
}

fn assert_amread_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = amread_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("amread-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_amread_mode_parity(elisp_form: &str, expected: Expect) {
    assert_amread_mode_source_parity("amread-mode.el", elisp_form, expected);
}

pub(crate) fn assert_amread_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_amread_mode_source_parity("amread-mode-autoloads.el", elisp_form, expected);
}
