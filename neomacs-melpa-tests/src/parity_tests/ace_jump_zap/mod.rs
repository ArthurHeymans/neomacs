use std::time::Duration;

use crate::{ACE_JUMP_ZAP_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod commands;
mod filters;
mod state;
mod surface;
mod workflows;

const ACE_JUMP_ZAP_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_jump_zap_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_JUMP_ZAP_MELPA_PIN, source_file)
        .expect("prepare pinned ace-jump-zap source below ./tmp")
        .with_timeout(ACE_JUMP_ZAP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-jump-zap parity test")
        .into()
}

pub(crate) fn assert_ace_jump_zap_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_zap_oracle("ace-jump-zap.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-jump-zap parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_zap_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_zap_oracle("ace-jump-zap.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-zap signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_zap_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_zap_oracle("ace-jump-zap-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-zap autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_zap_autoload_with_prelude_parity(
    prelude: &str,
    form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_JUMP_ZAP_MELPA_PIN, "ace-jump-zap-autoloads.el")
        .expect("prepare pinned ace-jump-zap autoloads below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ACE_JUMP_ZAP_TEST_TIMEOUT)
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-zap autoload prelude parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_jump_zap_exact_pin_dependencies_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-jump-zap
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-jump-zap)))"##;
    let expect = expect![[
        r#"OK (ace-jump-zap "20170717.1849" ((ace-jump-mode (1 0)) (dash (2 10 0))) "Character zapping, `ace-jump-mode` style." ((:maintainers ("justin talbott" . "justin@waymondo.com")) (:authors ("justin talbott" . "justin@waymondo.com")) (:keywords "convenience" "tools" "extensions") (:revdesc . "52b5d4c6c73b") (:commit . "52b5d4c6c73bd0fc833a0dcb4e803a5287d8cae8") (:url . "https://github.com/waymondo/ace-jump-zap")) t)"#
    ]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_required_features_are_loaded() {
    let elisp_form = r##"(mapcar
         #'featurep
         '(ace-jump-mode
           dash
           ace-jump-zap))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}
