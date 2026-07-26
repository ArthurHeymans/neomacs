use std::time::Duration;

use crate::{AC_RACER_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod candidates;
mod lifecycle;
mod surface;

const AC_RACER_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_racer_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_RACER_MELPA_PIN, source_file)
        .expect("prepare pinned ac-racer source below ./tmp")
        .with_timeout(AC_RACER_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-racer parity test")
        .into()
}

pub(crate) fn assert_ac_racer_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_racer_oracle("ac-racer.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-racer parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_racer_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_racer_oracle("ac-racer.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-racer signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_racer_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_racer_oracle("ac-racer-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-racer autoload parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn ac_racer_exact_pin_dependencies_features_group_and_source_contract_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-racer
                      package-alist))))
               (list
                (package-desc-name
                 descriptor)
                (package-version-join
                 (package-desc-version
                  descriptor))
                (package-desc-reqs
                 descriptor)
                (mapcar
                 #'featurep
                 '(ac-racer
                   auto-complete
                   racer
                   cl-lib))
                (get
                 'ac-racer
                 'group-documentation)
                (assq
                 'ac-racer
                 (get
                  'auto-complete
                  'custom-group))
                (file-name-nondirectory
                 ac-racer--tempfile)
                (equal
                 ac-racer--tempfile
                 (concat
                  temporary-file-directory
                  "ac-racer-complete"))
                ac-source-racer))"##;
    let expect = expect![[
        r#"OK (ac-racer "20170114.809" ((emacs (24 3)) (auto-complete (1 5 0)) (racer (0 0 2))) (t t t t) "auto-complete source of racer" (ac-racer custom-group) "ac-racer-complete" t ((prefix . ac-racer--prefix) (candidates . ac-racer--candidates) (requires . -1)))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}
