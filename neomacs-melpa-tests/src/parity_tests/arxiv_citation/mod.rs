use std::time::Duration;

use crate::{ARXIV_CITATION_MELPA_PIN, CachedMelpaOracle, DASH_MELPA_PIN, S_MELPA_PIN};
use expect_test::Expect;

mod citation;
mod dependencies;
mod download;
mod editing;
mod parsing;
mod surface;

const ARXIV_CITATION_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arxiv_citation_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARXIV_CITATION_MELPA_PIN, source_file)
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

fn assert_arxiv_citation_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arxiv_citation_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arxiv-citation parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arxiv_citation_parity(elisp_form: &str, expected: Expect) {
    assert_arxiv_citation_source_parity("arxiv-citation.el", elisp_form, expected);
}

pub(crate) fn assert_arxiv_citation_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_arxiv_citation_source_parity("arxiv-citation-autoloads.el", elisp_form, expected);
}
