use std::time::Duration;

use crate::{ANNALIST_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ANNALIST_TEST_TIMEOUT: Duration = Duration::from_secs(180);
fn annalist_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANNALIST_MELPA_PIN, source_file)
        .expect("prepare pinned annalist source below ./tmp")
        .with_timeout(ANNALIST_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed annalist parity test")
        .into()
}

fn assert_annalist_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = annalist_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("annalist parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_annalist_parity(elisp_form: &str, expected: Expect) {
    assert_annalist_source_parity("annalist.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_annalist_parity` cases (2a).
pub(crate) fn assert_annalist_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        annalist_oracle("annalist.el"),
        &name,
        "annalist_parity",
        cases,
    );
}
