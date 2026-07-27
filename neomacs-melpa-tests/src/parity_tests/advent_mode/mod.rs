use std::time::Duration;

use crate::{ADVENT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod answers;
mod calendar;
mod commands;
mod context;
mod filesystem;
mod http;
mod mode;
mod paths;
mod registry;
mod session;

const ADVENT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn advent_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADVENT_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned advent-mode source below ./tmp")
        .with_timeout(ADVENT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed advent-mode parity test")
        .into()
}

fn assert_advent_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = advent_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("advent-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_advent_mode_parity(elisp_form: &str, expected: Expect) {
    assert_advent_mode_source_parity("advent-mode.el", elisp_form, expected);
}

pub(crate) fn assert_advent_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_advent_mode_source_parity("advent-mode-autoloads.el", elisp_form, expected);
}
