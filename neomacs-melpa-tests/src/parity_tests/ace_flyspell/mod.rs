use std::time::Duration;

use crate::{ACE_FLYSPELL_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod candidates;
mod handlers;
mod surface;
mod workflows;

const ACE_FLYSPELL_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_flyspell_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_FLYSPELL_MELPA_PIN, source_file)
        .expect("prepare pinned ace-flyspell source below ./tmp")
        .with_timeout(ACE_FLYSPELL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-flyspell parity test")
        .into()
}

pub(crate) fn assert_ace_flyspell_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_flyspell_oracle("ace-flyspell.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-flyspell parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_flyspell_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_flyspell_oracle("ace-flyspell.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-flyspell signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_flyspell_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_flyspell_oracle("ace-flyspell-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("ace-flyspell autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_flyspell_exact_pin_dependencies_feature_and_group_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-flyspell
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
                 'ace-flyspell)
                (get
                 'ace-flyspell
                 'group-documentation)
                (assq
                 'ace-flyspell
                 (get
                  'flyspell
                  'custom-group))))"##;
    let expect = expect![[
        r#"OK (ace-flyspell "20170309.509" ((avy (0 4 0))) "Jump to and correct spelling errors using `ace-jump-mode' and flyspell." ((:maintainers ("Junpeng Qiu" . "qjpchmail@gmail.com")) (:authors ("Junpeng Qiu" . "qjpchmail@gmail.com")) (:keywords "extensions") (:revdesc . "538d4f8508d3") (:commit . "538d4f8508d305262ba0228dfe7c819fb65b53c9") (:url . "https://github.com/cute-jumper/ace-flyspell")) t "Jump to and correct spelling errors using `avy' and flyspell" (ace-flyspell custom-group))"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}
