use std::time::Duration;

use crate::{ANNOTATION_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ANNOTATION_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn annotation_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANNOTATION_MELPA_PIN, source_file)
        .expect("prepare pinned annotation source below ./tmp")
        .with_timeout(ANNOTATION_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed annotation parity test")
        .into()
}

fn assert_annotation_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = annotation_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("annotation parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_annotation_parity(elisp_form: &str, expected: Expect) {
    assert_annotation_source_parity("annotation.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_annotation_parity` cases (2a).
pub(crate) fn assert_annotation_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        annotation_oracle("annotation.el"),
        &name,
        "annotation_parity",
        cases,
    );
}
