use std::time::Duration;

use crate::{ARDUINO_MODE_FLYCHECK_MELPA_PIN, ARDUINO_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod ede_makefile;
mod ede_preferences;
mod ede_projects;
mod editing;
mod flycheck;
mod org_babel;
mod processes;
mod surface;

const ARDUINO_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arduino_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    let oracle = CachedMelpaOracle::new(ARDUINO_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned arduino-mode source below ./tmp")
        .with_timeout(ARDUINO_MODE_TEST_TIMEOUT);
    if source_file == "flycheck-arduino.el" {
        oracle
            .with_melpa_dependency(ARDUINO_MODE_FLYCHECK_MELPA_PIN)
            .expect("prepare pinned Flycheck dependency below ./tmp")
    } else {
        oracle
    }
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arduino-mode parity test")
        .into()
}

fn assert_arduino_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arduino_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arduino-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_arduino_source_signal_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arduino_mode_oracle(source_file)
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("arduino-mode signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arduino_mode_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("arduino-mode.el", elisp_form, expected);
}

pub(crate) fn assert_arduino_init_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("arduino-mode-init.el", elisp_form, expected);
}

pub(crate) fn assert_ede_arduino_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("ede-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_ede_arduino_signal_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_signal_parity("ede-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_flycheck_arduino_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("flycheck-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_ob_arduino_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("ob-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_ob_arduino_signal_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_signal_parity("ob-arduino.el", elisp_form, expected);
}
