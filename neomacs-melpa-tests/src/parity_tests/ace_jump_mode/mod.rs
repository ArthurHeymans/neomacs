use std::time::Duration;

use crate::{ACE_JUMP_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod advice;
mod autoloads;
mod candidates;
mod commands;
mod data;
mod execution;
mod marks;
mod overlays;
mod scopes;
mod surface;
mod trees;
mod variables;

const ACE_JUMP_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_jump_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_JUMP_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned ace-jump-mode source below ./tmp")
        .with_timeout(ACE_JUMP_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-jump-mode parity test")
        .into()
}

pub(crate) fn assert_ace_jump_mode_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_mode_oracle("ace-jump-mode.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-jump-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_mode_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_mode_oracle("ace-jump-mode.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-mode signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_mode_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_mode_oracle("ace-jump-mode-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-mode autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_jump_mode_exact_pin_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-jump-mode
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-jump-mode)))"##;
    let expect = expect![[
        r#"OK (ace-jump-mode "20140616.815" nil "A quick cursor location minor mode for emacs." ((:maintainers ("winterTTr" . "winterTTr@gmail.com")) (:authors ("winterTTr" . "winterTTr@gmail.com")) (:keywords "motion" "location" "cursor") (:revdesc . "8351e2df4fbb") (:commit . "8351e2df4fbbeb2a4003f2fb39f46d33803f3dac") (:url . "https://github.com/winterTTr/ace-jump-mode/")) t)"#
    ]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_required_cl_feature_and_minor_mode_registration_match() {
    let elisp_form = r##"(list
               (featurep 'cl)
               (featurep 'ace-jump-mode)
               (assq 'ace-jump-mode minor-mode-alist))"##;
    let expect = expect!["OK (t t (ace-jump-mode ace-jump-mode))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
