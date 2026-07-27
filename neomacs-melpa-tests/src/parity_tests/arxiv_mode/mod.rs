use std::time::Duration;

use crate::{ARXIV_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod bibliography;
mod commands;
mod navigation;
mod query;
mod registry;
mod rendering;

const ARXIV_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn arxiv_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARXIV_MODE_MELPA_PIN, source_file)
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

fn assert_arxiv_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arxiv_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arxiv-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arxiv_mode_parity(elisp_form: &str, expected: Expect) {
    assert_arxiv_mode_source_parity("arxiv-mode.el", elisp_form, expected);
}

pub(crate) fn assert_arxiv_mode_query_parity(elisp_form: &str, expected: Expect) {
    assert_arxiv_mode_source_parity("arxiv-query.el", elisp_form, expected);
}

pub(crate) fn assert_arxiv_mode_vars_parity(elisp_form: &str, expected: Expect) {
    assert_arxiv_mode_source_parity("arxiv-vars.el", elisp_form, expected);
}

pub(crate) fn assert_arxiv_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_arxiv_mode_source_parity("arxiv-mode-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_arxiv_mode_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arxiv_mode_oracle("arxiv-mode.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arxiv-mode signal case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn arxiv_mode_harness_contract_reports_exact_package_and_dependency_identity() {
    let elisp_form = r##"(list
         (featurep 'arxiv-mode)
         (featurep 'arxiv-query)
         (featurep 'arxiv-vars)
         (featurep 'hydra)
         (file-name-nondirectory (locate-library "arxiv-mode"))
         (file-name-nondirectory (locate-library "hydra"))
         (package-installed-p 'arxiv-mode '(20240111 2203)))"##;
    let expect = expect![[r#"OK (t t t t "arxiv-mode.el" "hydra.el" t)"#]];
    assert_arxiv_mode_parity(elisp_form, expect);
}
