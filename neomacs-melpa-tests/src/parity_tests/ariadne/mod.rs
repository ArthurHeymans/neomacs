use std::time::Duration;

use crate::{ARIADNE_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod dispatch;
mod framing;
mod navigation;
mod process;
mod registry;

const ARIADNE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn ariadne_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARIADNE_MELPA_PIN, source_file)
        .expect("prepare pinned Ariadne source below ./tmp")
        .with_timeout(ARIADNE_TEST_TIMEOUT)
}

fn ariadne_oracle_with_legacy_cl(source_file: &str) -> CachedMelpaOracle {
    ariadne_oracle(source_file).with_prelude("(require 'cl)")
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed Ariadne parity test")
        .into()
}

fn assert_ariadne_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ariadne_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("Ariadne parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ariadne_parity(elisp_form: &str, expected: Expect) {
    assert_ariadne_source_parity("ariadne.el", elisp_form, expected);
}

pub(crate) fn assert_ariadne_with_legacy_cl_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ariadne_oracle_with_legacy_cl("ariadne.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("Ariadne legacy-cl parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ariadne_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ariadne_oracle("ariadne.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("Ariadne signal case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ariadne_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ariadne_source_parity("ariadne-autoloads.el", elisp_form, expected);
}

#[test]
fn ariadne_harness_contract_reports_exact_package_and_dependency_identity() {
    let elisp_form = r##"(list
         (featurep 'ariadne)
         (featurep 'bert)
         (file-name-nondirectory (locate-library "ariadne"))
         (file-name-nondirectory (locate-library "bert"))
         (package-installed-p 'ariadne '(20131117 1711))
         (package-installed-p 'bert '(20131117 1014)))"##;
    let expect = expect![[r#"OK (t t "ariadne.el" "bert.el" t t)"#]];
    assert_ariadne_parity(elisp_form, expect);
}
