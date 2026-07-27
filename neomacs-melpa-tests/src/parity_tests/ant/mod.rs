use std::time::Duration;

use crate::{ANT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod registry;
mod roots;
mod tasks;

const ANT_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn ant_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANT_MELPA_PIN, source_file)
        .expect("prepare pinned ant source below ./tmp")
        .with_timeout(ANT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ant parity test").into()
}

fn assert_ant_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ant_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ant parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ant_parity(elisp_form: &str, expected: Expect) {
    assert_ant_source_parity("ant.el", elisp_form, expected);
}

pub(crate) fn assert_ant_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ant_source_parity("ant-autoloads.el", elisp_form, expected);
}
