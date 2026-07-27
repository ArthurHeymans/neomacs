use std::time::Duration;

use crate::{ANDROID_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod autoloads;
mod builders;
mod logcat;
mod manifest;
mod processes;
mod project;
mod sdk;
mod surface;

const ANDROID_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn android_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANDROID_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned android-mode source below ./tmp")
        .with_prelude(
            r##"(progn
                   (setq exec-path nil)
                   (setenv "PATH" ""))"##,
        )
        .with_timeout(ANDROID_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed android-mode parity test")
        .into()
}

fn assert_android_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = android_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("android-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_android_mode_parity(elisp_form: &str, expected: Expect) {
    assert_android_mode_source_parity("android-mode.el", elisp_form, expected);
}

pub(crate) fn assert_android_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_android_mode_source_parity("android-mode-autoloads.el", elisp_form, expected);
}
