use std::time::Duration;

use crate::{ACADEMIC_PHRASES_MELPA_PIN, CachedMelpaOracle};
use expect_test::{Expect, expect};

mod autoloads;
mod callables;
mod data;
mod hash_tables;
mod insertion;
mod lookup;
mod sections;
mod surface;

const ACADEMIC_PHRASES_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn academic_phrases_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACADEMIC_PHRASES_MELPA_PIN, source_file)
        .expect("prepare pinned academic-phrases source below ./tmp")
        .with_timeout(ACADEMIC_PHRASES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed academic-phrases parity test")
        .into()
}

pub(crate) fn assert_academic_phrases_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = academic_phrases_oracle("academic-phrases.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("academic-phrases parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_academic_phrases_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = academic_phrases_oracle("academic-phrases.el")
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("academic-phrases signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_academic_phrases_autoload_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = academic_phrases_oracle("academic-phrases-autoloads.el")
        .run_value(&name, form)
        .unwrap_or_else(|error| {
            panic!("academic-phrases autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

#[test]
fn academic_phrases_exact_pin_dependencies_feature_and_data_binding_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'academic-phrases
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
                (mapcar
                 #'featurep
                 '(academic-phrases
                   cl-lib
                   dash
                   ht
                   s))
                (boundp
                 'academic-phrases--all-phrases)
                (hash-table-p
                 academic-phrases--all-phrases)
                (hash-table-count
                 academic-phrases--all-phrases)))"##;
    let expect = expect![[
        r#"OK (academic-phrases "20180723.1021" ((dash (2 12 0)) (s (1 12 0)) (ht (2 0)) (emacs (24))) "Bypass that mental block when writing your papers." ((:maintainers ("Nasser Alshammari" . "designernasser@gmail.com")) (:authors ("Nasser Alshammari" . "designernasser@gmail.com")) (:keywords "academic" "convenience" "papers" "writing" "wp") (:revdesc . "25d9cf67feac") (:commit . "25d9cf67feac6359cb213f061735e2679c84187f") (:url . "https://github.com/nashamri/academic-phrases")) (t t t t t) t t 57)"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}
