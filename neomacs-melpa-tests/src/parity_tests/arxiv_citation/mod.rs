use std::time::Duration;

use crate::{ARXIV_CITATION_MELPA_PIN, CachedMelpaOracle, DASH_MELPA_PIN, S_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const ARXIV_CITATION_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arxiv_citation_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARXIV_CITATION_MELPA_PIN, "arxiv-citation.el")
        .expect("prepare pinned arxiv-citation source below ./tmp")
        .with_melpa_dependency(DASH_MELPA_PIN)
        .expect("prepare pinned Dash dependency below ./tmp")
        .with_melpa_dependency(S_MELPA_PIN)
        .expect("prepare pinned s dependency below ./tmp")
        .with_timeout(ARXIV_CITATION_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arxiv-citation parity test")
        .into()
}

pub(crate) fn assert_arxiv_citation_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arxiv_citation_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arxiv-citation parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_arxiv_citation_parity` cases (2a).
pub(crate) fn assert_arxiv_citation_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arxiv_citation_oracle(),
        &name,
        "arxiv_citation_parity",
        cases,
    );
}
