use std::time::Duration;

use crate::{ACE_WINDOW_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod commands;
mod dispatch;
mod display_mode;
mod filtering;
mod overlays;
mod posframe;
mod selection;
mod surface;

const ACE_WINDOW_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_window_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_WINDOW_MELPA_PIN, source_file)
        .expect("prepare pinned ace-window source below ./tmp")
        .with_timeout(ACE_WINDOW_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-window parity test")
        .into()
}

fn assert_ace_window_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_window_oracle("ace-window.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ace-window parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_ace_window_autoload_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_window_oracle("ace-window-autoloads.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ace-window autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_ace_window_posframe_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_window_oracle("ace-window-posframe.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ace-window posframe parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_window_exact_pin_dependencies_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-window
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-window)))"##;
    let expect = expect![[
        r#"OK (ace-window "20220911.358" ((avy (0 5 0))) "Quickly switch windows." ((:maintainers ("Oleh Krehel" . "ohwoeowho@gmail.com")) (:authors ("Oleh Krehel" . "ohwoeowho@gmail.com")) (:keywords "window" "location") (:revdesc . "77115afc1b0b") (:commit . "77115afc1b0b9f633084cf7479c767988106c196") (:url . "https://github.com/abo-abo/ace-window")) t)"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_required_dependency_and_registered_minor_mode_match() {
    let elisp_form = r##"(list
         (featurep 'avy)
         (featurep 'ace-window)
         (assq 'ace-window-mode
               minor-mode-alist))"##;
    let expect = expect!["OK (t t (ace-window-mode ace-window-mode))"];
    assert_ace_window_parity(elisp_form, expect);
}
