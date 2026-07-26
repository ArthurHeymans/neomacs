use std::time::Duration;

use crate::{ACE_JUMP_BUFFER_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod advice;
mod autoloads;
mod commands;
mod integrations;
mod macro_configuration;
mod options;
mod selection;
mod surface;
mod variables;

const ACE_JUMP_BUFFER_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_jump_buffer_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_JUMP_BUFFER_MELPA_PIN, source_file)
        .expect("prepare pinned ace-jump-buffer source below ./tmp")
        .with_timeout(ACE_JUMP_BUFFER_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-jump-buffer parity test")
        .into()
}

pub(crate) fn assert_ace_jump_buffer_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_buffer_oracle("ace-jump-buffer.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-jump-buffer parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_buffer_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_buffer_oracle("ace-jump-buffer.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-buffer signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_buffer_with_prelude_parity(
    prelude: &str,
    form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_JUMP_BUFFER_MELPA_PIN, "ace-jump-buffer.el")
        .expect("prepare pinned ace-jump-buffer source below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ACE_JUMP_BUFFER_TEST_TIMEOUT)
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-buffer prelude parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_buffer_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_jump_buffer_oracle("ace-jump-buffer-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-buffer autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_jump_buffer_autoload_with_prelude_parity(
    prelude: &str,
    form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_JUMP_BUFFER_MELPA_PIN, "ace-jump-buffer-autoloads.el")
        .expect("prepare pinned ace-jump-buffer autoloads below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ACE_JUMP_BUFFER_TEST_TIMEOUT)
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-jump-buffer autoload prelude parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_jump_buffer_exact_pin_dependencies_feature_and_group_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-jump-buffer
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-jump-buffer)
                (get
                 'ace-jump-buffer
                 'group-documentation)
                (get
                 'ace-jump-buffer
                 'custom-version)
                (get
                 'ace-jump-buffer
                 'custom-links)
                (assq
                 'ace-jump-buffer
                 (get
                  'convenience
                  'custom-group))))"##;
    let expect = expect![[
        r#"OK (ace-jump-buffer "20171031.1550" ((avy (0 4 0)) (dash (2 4 0))) "Fast buffer switching extension to `avy'." ((:maintainers ("Justin Talbott" . "justin@waymondo.com")) (:authors ("Justin Talbott" . "justin@waymondo.com")) (:revdesc . "ae5be0415c82") (:commit . "ae5be0415c823f7bb66833aa4af2180d4cf99cef") (:url . "https://github.com/waymondo/ace-jump-buffer")) t "Fast buffer switching extension to `avy'." "0.4.0" ((url-link "https://github.com/waymondo/ace-jump-buffer")) (ace-jump-buffer custom-group))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_required_features_are_loaded() {
    let elisp_form = r##"(mapcar
               #'featurep
               '(bs
                 avy
                 recentf
                 dash
                 ace-jump-buffer))"##;
    let expect = expect!["OK (t t t t t)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
