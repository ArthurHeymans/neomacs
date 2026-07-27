use std::time::Duration;

use crate::{APPARMOR_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod completion;
mod flymake;
mod font_lock;
mod indentation;
mod mode;
mod registry;

const APPARMOR_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn apparmor_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(APPARMOR_MODE_MELPA_PIN, "apparmor-mode.el")
        .expect("prepare pinned apparmor-mode source below ./tmp")
        .with_timeout(APPARMOR_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apparmor-mode parity test")
        .into()
}

pub(crate) fn assert_apparmor_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apparmor_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apparmor-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
