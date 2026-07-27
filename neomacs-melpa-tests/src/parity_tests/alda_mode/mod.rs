use std::time::Duration;

use crate::{ALDA_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod editing;
mod history;
mod playback;
mod process;
mod registry;

const ALDA_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn alda_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALDA_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned alda-mode source below ./tmp")
        .with_timeout(ALDA_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alda-mode parity test")
        .into()
}

fn assert_alda_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alda_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alda-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alda_mode_parity(elisp_form: &str, expected: Expect) {
    assert_alda_mode_source_parity("alda-mode.el", elisp_form, expected);
}

pub(crate) fn assert_alda_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alda_mode_source_parity("alda-mode-autoloads.el", elisp_form, expected);
}
