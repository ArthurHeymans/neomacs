use std::time::Duration;

use crate::{ACE_ISEARCH_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod adapters;
mod autoload_absence;
mod autoloads;
mod jumper;
mod modes;
mod options;
mod surface;
mod switching;
mod variables;

const ACE_ISEARCH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_isearch_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_ISEARCH_MELPA_PIN, source_file)
        .expect("prepare pinned ace-isearch source below ./tmp")
        .with_prelude("(provide 'ace-jump-mode)")
        .with_timeout(ACE_ISEARCH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-isearch parity test")
        .into()
}

pub(crate) fn assert_ace_isearch_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_isearch_oracle("ace-isearch.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-isearch parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_isearch_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_isearch_oracle("ace-isearch.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ace-isearch signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_isearch_avy_backend_parity(form: &str, expected: Expect) {
    assert_ace_isearch_with_prelude_parity("(provide 'avy)", form, expected);
}

pub(crate) fn assert_ace_isearch_with_prelude_parity(prelude: &str, form: &str, expected: Expect) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_ISEARCH_MELPA_PIN, "ace-isearch.el")
        .expect("prepare pinned ace-isearch source below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ACE_ISEARCH_TEST_TIMEOUT)
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-isearch prelude parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_isearch_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_ISEARCH_MELPA_PIN, "ace-isearch-autoloads.el")
        .expect("prepare pinned ace-isearch autoloads below ./tmp")
        .with_timeout(ACE_ISEARCH_TEST_TIMEOUT)
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-isearch autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_isearch_autoload_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = CachedMelpaOracle::new(ACE_ISEARCH_MELPA_PIN, "ace-isearch-autoloads.el")
        .expect("prepare pinned ace-isearch autoloads below ./tmp")
        .with_timeout(ACE_ISEARCH_TEST_TIMEOUT)
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-isearch autoload signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_isearch_exact_pin_dependencies_feature_and_group_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-isearch
                      package-alist))))
               (list
                (package-desc-name
                 descriptor)
                (package-version-join
                 (package-desc-version
                  descriptor))
                (package-desc-reqs
                 descriptor)
                (package-desc-summary
                 descriptor)
                (copy-tree
                 (package-desc-extras
                  descriptor))
                (featurep
                 'ace-isearch)
                (get
                 'ace-isearch
                 'group-documentation)
                (assq
                 'ace-isearch
                 (get
                  'convenience
                  'custom-group))))"##;
    let expect = expect![[
        r#"OK (ace-isearch "20220809.1748" ((emacs (24))) "A seamless bridge between isearch, ace-jump-mode, avy, helm-swoop and swiper." ((:revdesc . "a24bfc626100") (:commit . "a24bfc626100f183dbad016bd7723eb12e238534") (:url . "https://github.com/tam17aki/ace-isearch")) t "Group of ace-isearch." (ace-isearch custom-group))"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}
