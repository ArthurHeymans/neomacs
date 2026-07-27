use std::time::Duration;

use crate::{ASN1_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod abbrev_outline;
mod indentation;
mod registry;
mod syntax_font_lock;
mod tokens;

const ASN1_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn asn1_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASN1_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned asn1-mode source below ./tmp")
        .with_timeout(ASN1_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed asn1-mode parity test")
        .into()
}

fn assert_asn1_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = asn1_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("asn1-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_asn1_mode_parity(elisp_form: &str, expected: Expect) {
    assert_asn1_mode_source_parity("asn1-mode.el", elisp_form, expected);
}

pub(crate) fn assert_asn1_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_asn1_mode_source_parity("asn1-mode-autoloads.el", elisp_form, expected);
}

#[test]
fn asn1_mode_harness_contract_reports_exact_package_and_dependency_identity() {
    let elisp_form = r##"(let ((desc (cadr (assq 'asn1-mode package-alist)))
              (s-desc (cadr (assq 's package-alist))))
          (list
           (featurep 'asn1-mode)
           (featurep 's)
           (package-version-join (package-desc-version desc))
           (package-version-join (package-desc-version s-desc))
           (file-name-nondirectory (locate-library "asn1-mode"))
           (package-installed-p 'asn1-mode '(20170729 226))))"##;
    let expect = expect![[r#"OK (t t "20170729.226" "20220902.1511" "asn1-mode.el" t)"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}
