use std::time::Duration;

use crate::{ACE_JUMP_HELM_LINE_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod actions;
mod autoloads;
mod collection;
mod dispatch;
mod execution;
mod idle;
mod macro_hook;
mod preview;
mod surface;
mod variables;

const ACE_JUMP_HELM_LINE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_jump_helm_line_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_JUMP_HELM_LINE_MELPA_PIN, source_file)
        .expect("prepare pinned ace-jump-helm-line source below ./tmp")
        .with_timeout(ACE_JUMP_HELM_LINE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-jump-helm-line parity test")
        .into()
}

pub(crate) fn assert_ace_jump_helm_line_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_helm_line_oracle("ace-jump-helm-line.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-jump-helm-line parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_helm_line_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_helm_line_oracle("ace-jump-helm-line.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-helm-line signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_helm_line_with_prelude_parity(
    prelude: &str,
    form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_JUMP_HELM_LINE_MELPA_PIN, "ace-jump-helm-line.el")
        .expect("prepare pinned ace-jump-helm-line source below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ACE_JUMP_HELM_LINE_TEST_TIMEOUT)
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-helm-line prelude parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_helm_line_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_helm_line_oracle("ace-jump-helm-line-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-helm-line autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_jump_helm_line_exact_pin_dependencies_feature_and_summary_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-jump-helm-line
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-jump-helm-line)))"##;
    let expect = expect![[
        r#"OK (ace-jump-helm-line "20160918.1836" ((avy (0 4 0)) (helm (1 6 3))) "Ace-jump to a candidate in helm window." ((:maintainers ("Junpeng Qiu" . "qjpchmail@gmail.com")) (:authors ("Junpeng Qiu" . "qjpchmail@gmail.com")) (:keywords "extensions") (:revdesc . "1483055255df") (:commit . "1483055255df3f8ae349f7520f05b1e43ea3ed37") (:url . "https://github.com/cute-jumper/ace-jump-helm-line")) t)"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_required_features_are_loaded() {
    let elisp_form = r##"(mapcar
               #'featurep
               '(avy helm linum ace-jump-helm-line))"##;
    let expect = expect!["OK (t t t t)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
