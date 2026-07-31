use std::time::Duration;

use crate::{ARXIV_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ARXIV_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arxiv_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARXIV_MODE_MELPA_PIN, "arxiv-mode.el")
        .expect("prepare pinned arxiv-mode source below ./tmp")
        .with_timeout(ARXIV_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arxiv-mode parity test")
        .into()
}

pub(crate) fn assert_arxiv_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arxiv_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arxiv-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_arxiv_mode_parity` cases (2a).
pub(crate) fn assert_arxiv_mode_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arxiv_mode_oracle(),
        &name,
        "arxiv_mode_parity",
        cases,
    );
}
