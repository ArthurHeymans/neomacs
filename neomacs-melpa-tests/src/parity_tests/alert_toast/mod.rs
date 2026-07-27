use std::time::Duration;

use crate::{ALERT_TOAST_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod notify;
mod paths;
mod process;
mod registry;
mod xml;

const ALERT_TOAST_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const DETERMINISTIC_LINUX_PRELUDE: &str = r##"
(require 'cl-lib)
(fset 'shell-command-to-string
      (lambda (_command) "6.8.0-generic\n"))
"##;

fn alert_toast_oracle(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALERT_TOAST_MELPA_PIN, source_file)
        .expect("prepare pinned alert-toast source below ./tmp")
        .with_prelude(format!("{DETERMINISTIC_LINUX_PRELUDE}\n{prelude}"))
        .with_timeout(ALERT_TOAST_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alert-toast parity test")
        .into()
}

fn assert_alert_toast_source_parity(
    source_file: &str,
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = alert_toast_oracle(source_file, prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alert-toast parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alert_toast_parity(elisp_form: &str, expected: Expect) {
    assert_alert_toast_source_parity("alert-toast.el", "", elisp_form, expected);
}

pub(crate) fn assert_alert_toast_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    assert_alert_toast_source_parity("alert-toast.el", prelude, elisp_form, expected);
}

pub(crate) fn assert_alert_toast_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alert_toast_source_parity("alert-toast-autoloads.el", "", elisp_form, expected);
}
