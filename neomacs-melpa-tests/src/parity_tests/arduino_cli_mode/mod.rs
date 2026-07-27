use std::time::Duration;

use crate::{ARDUINO_CLI_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod boards;
mod commands;
mod flags;
mod libraries;
mod monitor;
mod registry;
mod upstream_workflows;

const ARDUINO_CLI_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arduino_cli_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARDUINO_CLI_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned arduino-cli-mode source below ./tmp")
        .with_timeout(ARDUINO_CLI_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arduino-cli-mode parity test")
        .into()
}

fn assert_arduino_cli_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arduino_cli_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arduino-cli-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arduino_cli_mode_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_cli_mode_source_parity("arduino-cli-mode.el", elisp_form, expected);
}

pub(crate) fn assert_arduino_cli_mode_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arduino_cli_mode_oracle("arduino-cli-mode.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arduino-cli-mode signal case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arduino_cli_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_cli_mode_source_parity("arduino-cli-mode-autoloads.el", elisp_form, expected);
}

#[test]
fn arduino_cli_mode_harness_contract_reports_exact_package_identity() {
    let elisp_form = r##"(list
         (featurep 'arduino-cli-mode)
         (file-name-nondirectory (locate-library "arduino-cli-mode"))
         (package-installed-p 'arduino-cli-mode '(20260628 2219)))"##;
    let expect = expect![[r#"OK (t "arduino-cli-mode.el" t)"#]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
