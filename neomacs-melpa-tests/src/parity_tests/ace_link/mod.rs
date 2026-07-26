use std::time::Duration;

use crate::{ACE_LINK_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod actions;
mod autoloads;
mod collectors;
mod dispatch;
mod email;
mod setup;
mod surface;
mod workflows;

const ACE_LINK_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ace_link_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACE_LINK_MELPA_PIN, source_file)
        .expect("prepare pinned ace-link source below ./tmp")
        .with_timeout(ACE_LINK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ace-link parity test")
        .into()
}

pub(crate) fn assert_ace_link_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_link_oracle("ace-link.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-link parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_link_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_link_oracle("ace-link.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ace-link signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ace_link_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ace_link_oracle("ace-link-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ace-link autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ace_link_exact_pin_dependencies_metadata_and_feature_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ace-link
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-summary descriptor)
                (copy-tree
                 (package-desc-extras descriptor))
                (featurep 'ace-link)))"##;
    let expect = expect![[
        r#"OK (ace-link "20241101.1344" ((avy (0 4 0))) "Quickly follow links." ((:maintainers ("Oleh Krehel" . "ohwoeowho@gmail.com")) (:authors ("Oleh Krehel" . "ohwoeowho@gmail.com")) (:keywords "convenience" "links" "avy") (:revdesc . "d9bd4a25a02b") (:commit . "d9bd4a25a02bdfde4ea56247daf3a9ff15632ea4") (:url . "https://github.com/abo-abo/ace-link")) t)"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_required_avy_feature_is_loaded() {
    let elisp_form = r##"(list
         (featurep 'avy)
         (featurep 'ace-link))"##;
    let expect = expect!["OK (t t)"];
    assert_ace_link_parity(elisp_form, expect);
}
