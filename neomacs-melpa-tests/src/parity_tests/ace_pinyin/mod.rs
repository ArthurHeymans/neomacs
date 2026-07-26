use std::time::Duration;

use crate::{ACE_PINYIN_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod jumps;
mod modes;
mod regex;
mod surface;
mod words;

const ACE_PINYIN_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_pinyin_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_PINYIN_MELPA_PIN, source_file)
        .expect("prepare pinned ace-pinyin source below ./tmp")
        .with_timeout(ACE_PINYIN_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-pinyin parity test")
        .into()
}

pub(crate) fn assert_ace_pinyin_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_pinyin_oracle("ace-pinyin.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-pinyin parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_pinyin_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_pinyin_oracle("ace-pinyin.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ace-pinyin signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_pinyin_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_pinyin_oracle("ace-pinyin-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-pinyin autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_pinyin_exact_pin_dependencies_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-pinyin
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-pinyin)))"##;
    let expect = expect![[
        r#"OK (ace-pinyin "20210827.355" ((avy (0 2 0)) (pinyinlib (0 1 0))) "Jump to Chinese characters using avy or ace-jump-mode." ((:maintainers ("Junpeng Qiu" . "qjpchmail@gmail.com")) (:authors ("Junpeng Qiu" . "qjpchmail@gmail.com")) (:keywords "extensions") (:revdesc . "47662c0b0577") (:commit . "47662c0b05775ba353464b44c0f1a037c85e746e") (:url . "https://github.com/cute-jumper/ace-pinyin")) t)"#
    ]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_required_and_optional_dependency_features_match() {
    let elisp_form = r##"(list
         (featurep 'avy)
         (featurep 'pinyinlib)
         (featurep 'ace-jump-mode)
         (featurep 'ace-pinyin))"##;
    let expect = expect!["OK (t t nil t)"];
    assert_ace_pinyin_parity(elisp_form, expect);
}
