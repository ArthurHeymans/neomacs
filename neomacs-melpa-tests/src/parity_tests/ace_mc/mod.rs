use std::time::Duration;

use crate::{ACE_MC_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod commands;
mod hooks;
mod lifecycle;
mod surface;
mod workflows;

const ACE_MC_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_mc_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_MC_MELPA_PIN, source_file)
        .expect("prepare pinned ace-mc source below ./tmp")
        .with_timeout(ACE_MC_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ace-mc parity test").into()
}

pub(crate) fn assert_ace_mc_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_mc_oracle("ace-mc.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-mc parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_mc_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_mc_oracle("ace-mc.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ace-mc signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_mc_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_mc_oracle("ace-mc-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-mc autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_mc_exact_pin_dependencies_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-mc
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-mc)))"##;
    let expect = expect![[
        r#"OK (ace-mc "20190206.749" ((ace-jump-mode (1 0)) (multiple-cursors (1 0)) (dash (2 10 0))) "Add multiple cursors quickly using ace jump." ((:maintainers ("Josh Moller-Mara" . "jmm@cns.nyu.edu")) (:authors ("Josh Moller-Mara" . "jmm@cns.nyu.edu")) (:keywords "motion" "location" "cursor") (:revdesc . "6877880efd99") (:commit . "6877880efd99e177e4e9116a364576def3da391b") (:url . "https://github.com/mm--/ace-mc")) t)"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_required_dependency_features_are_loaded() {
    let elisp_form = r##"(list
         (featurep 'ace-jump-mode)
         (featurep 'multiple-cursors-core)
         (featurep 'dash)
         (featurep 'ace-mc))"##;
    let expect = expect!["OK (t t t t)"];
    assert_ace_mc_parity(elisp_form, expect);
}
