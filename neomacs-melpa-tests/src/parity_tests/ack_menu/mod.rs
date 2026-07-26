use std::time::Duration;

use crate::{ACK_MENU_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod arguments;
mod autoloads;
mod menu;
mod mode;
mod navigation;
mod process;
mod reading;
mod roots;
mod sgr;
mod surface;
mod types;

const ACK_MENU_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ack_menu_oracle(source_file: &str) -> CachedMelpaOracle {
    ack_menu_oracle_with_prelude(
        source_file,
        r##"(fset 'executable-find
                    (lambda (command)
                      command
                      nil))"##,
    )
}

fn ack_menu_oracle_with_prelude(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACK_MENU_MELPA_PIN, source_file)
        .expect("prepare pinned ack-menu source below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ACK_MENU_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ack-menu parity test")
        .into()
}

pub(crate) fn assert_ack_menu_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ack_menu_oracle("ack-menu.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ack-menu parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ack_menu_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ack_menu_oracle("ack-menu.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ack-menu signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ack_menu_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = ack_menu_oracle_with_prelude("ack-menu.el", prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ack-menu pre-load parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ack_menu_autoload_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ack_menu_oracle("ack-menu-autoloads.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ack-menu autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ack_menu_exact_pin_dependencies_metadata_and_features_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ack-menu
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (mapcar
                 #'featurep
                 '(ack-menu
                   mag-menu
                   compile
                   cl
                   ansi-color
                   thingatpt))))"##;
    let expect = expect![[
        r#"OK (ack-menu "20150504.2022" ((mag-menu (0 1 0))) "A menu-based front-end for ack." ((:keywords "tools" "matching" "convenience") (:revdesc . "f77be93a4697") (:commit . "f77be93a4697926ecf3195a355eb69580f695f4d") (:url . "https://github.com/chumpage/ack-menu")) (t t t t t t))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}
