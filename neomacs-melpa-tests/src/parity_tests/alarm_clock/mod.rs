use std::time::Duration;

use crate::{ALARM_CLOCK_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod alarms;
mod listing;
mod notifications;
mod persistence;
mod registry;

const ALARM_CLOCK_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn alarm_clock_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALARM_CLOCK_MELPA_PIN, source_file)
        .expect("prepare pinned alarm-clock source below ./tmp")
        .with_timeout(ALARM_CLOCK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alarm-clock parity test")
        .into()
}

fn assert_alarm_clock_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alarm_clock_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alarm-clock parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alarm_clock_parity(elisp_form: &str, expected: Expect) {
    assert_alarm_clock_source_parity("alarm-clock.el", elisp_form, expected);
}

pub(crate) fn assert_alarm_clock_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alarm_clock_source_parity("alarm-clock-autoloads.el", elisp_form, expected);
}
