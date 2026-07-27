use std::time::Duration;

use crate::{ACT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod font_lock;
mod mode;
mod registry;

const ACT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn act_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACT_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned act-mode source below ./tmp")
        .with_timeout(ACT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed act-mode parity test")
        .into()
}

fn assert_act_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = act_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("act-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_act_mode_parity(elisp_form: &str, expected: Expect) {
    assert_act_mode_source_parity("act-mode.el", elisp_form, expected);
}

pub(crate) fn assert_act_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_act_mode_source_parity("act-mode-autoloads.el", elisp_form, expected);
}
