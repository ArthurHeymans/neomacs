use std::time::Duration;

use crate::{ACE_POPUP_MENU_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod dispatch;
mod mode;
mod surface;

const ACE_POPUP_MENU_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_popup_menu_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_POPUP_MENU_MELPA_PIN, source_file)
        .expect("prepare pinned ace-popup-menu source below ./tmp")
        .with_timeout(ACE_POPUP_MENU_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-popup-menu parity test")
        .into()
}

fn assert_ace_popup_menu_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_popup_menu_oracle("ace-popup-menu.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ace-popup-menu parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_ace_popup_menu_autoload_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_popup_menu_oracle("ace-popup-menu-autoloads.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ace-popup-menu autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_popup_menu_exact_pin_dependencies_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-popup-menu
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-popup-menu)))"##;
    let expect = expect![[
        r#"OK (ace-popup-menu "20230606.1445" ((emacs (24 4)) (avy-menu (0 1))) "Replace GUI popup menu with something more efficient." ((:maintainers ("Mark Karpov" . "markkarpov92@gmail.com")) (:authors ("Mark Karpov" . "markkarpov92@gmail.com")) (:keywords "convenience" "popup" "menu") (:revdesc . "a8b970d1b59e") (:commit . "a8b970d1b59efbe7e1e29ed16d71af257a22699f") (:url . "https://github.com/mrkkrp/ace-popup-menu")) t)"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_required_dependency_features_match() {
    let elisp_form = r##"(list
         (featurep 'avy-menu)
         (featurep 'avy)
         (featurep 'ace-popup-menu))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_popup_menu_parity(elisp_form, expect);
}
