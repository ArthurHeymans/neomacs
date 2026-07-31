use std::time::Duration;

use crate::{ANNOTATE_DEPTH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ANNOTATE_DEPTH_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn annotate_depth_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANNOTATE_DEPTH_MELPA_PIN, source_file)
        .expect("prepare pinned annotate-depth source below ./tmp")
        .with_timeout(ANNOTATE_DEPTH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed annotate-depth parity test")
        .into()
}

fn assert_annotate_depth_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = annotate_depth_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("annotate-depth parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_annotate_depth_parity(elisp_form: &str, expected: Expect) {
    assert_annotate_depth_source_parity("annotate-depth.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_annotate_depth_parity` cases (2a).
pub(crate) fn assert_annotate_depth_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        annotate_depth_oracle("annotate-depth.el"),
        &name,
        "annotate_depth_parity",
        cases,
    );
}
