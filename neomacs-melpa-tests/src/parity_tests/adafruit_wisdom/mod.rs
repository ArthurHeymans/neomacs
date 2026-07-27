use std::time::Duration;

use crate::{ADAFRUIT_WISDOM_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod cache;
mod command;
mod initialization;
mod registry;
mod selection;

const ADAFRUIT_WISDOM_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn adafruit_wisdom_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADAFRUIT_WISDOM_MELPA_PIN, source_file)
        .expect("prepare pinned adafruit-wisdom source below ./tmp")
        .with_timeout(ADAFRUIT_WISDOM_TEST_TIMEOUT)
}

fn adafruit_wisdom_no_littering_oracle() -> CachedMelpaOracle {
    adafruit_wisdom_oracle("adafruit-wisdom.el").with_prelude(
        r##"(progn
               (defun no-littering-expand-var-file-name (filename)
                 (expand-file-name
                  filename
                  (expand-file-name
                   "var/no-littering/"
                   user-emacs-directory)))
               (provide
                'no-littering))"##,
    )
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed adafruit-wisdom parity test")
        .into()
}

fn assert_adafruit_wisdom_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = adafruit_wisdom_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("adafruit-wisdom parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_adafruit_wisdom_parity(elisp_form: &str, expected: Expect) {
    assert_adafruit_wisdom_source_parity("adafruit-wisdom.el", elisp_form, expected);
}

pub(crate) fn assert_adafruit_wisdom_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_adafruit_wisdom_source_parity("adafruit-wisdom-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_adafruit_wisdom_no_littering_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = adafruit_wisdom_no_littering_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("adafruit-wisdom no-littering parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
