use std::time::Duration;

use crate::{ACTIVITY_WATCH_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod heartbeat;
mod lifecycle;
mod project;
mod registry;

const ACTIVITY_WATCH_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn activity_watch_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACTIVITY_WATCH_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned activity-watch-mode source below ./tmp")
        .with_timeout(ACTIVITY_WATCH_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed activity-watch-mode parity test")
        .into()
}

fn assert_activity_watch_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = activity_watch_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("activity-watch-mode parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_activity_watch_mode_parity(elisp_form: &str, expected: Expect) {
    assert_activity_watch_mode_source_parity("activity-watch-mode.el", elisp_form, expected);
}

pub(crate) fn assert_activity_watch_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_activity_watch_mode_source_parity(
        "activity-watch-mode-autoloads.el",
        elisp_form,
        expected,
    );
}
