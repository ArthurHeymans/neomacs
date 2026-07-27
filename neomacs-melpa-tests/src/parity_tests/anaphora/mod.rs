use std::time::Duration;

use crate::{ANAPHORA_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod arithmetic;
mod control_flow;
mod dispatch;
mod registry;

const ANAPHORA_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anaphora_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANAPHORA_MELPA_PIN, source_file)
        .expect("prepare pinned anaphora source below ./tmp")
        .with_timeout(ANAPHORA_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anaphora parity test")
        .into()
}

fn assert_anaphora_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anaphora_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anaphora parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anaphora_parity(elisp_form: &str, expected: Expect) {
    assert_anaphora_source_parity("anaphora.el", elisp_form, expected);
}

pub(crate) fn assert_anaphora_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anaphora_source_parity("anaphora-autoloads.el", elisp_form, expected);
}
