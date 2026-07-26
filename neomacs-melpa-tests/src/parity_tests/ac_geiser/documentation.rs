use expect_test::expect;

use super::{assert_ac_geiser_parity, assert_ac_geiser_signal_parity};

#[test]
fn ac_geiser_documentation_before_geiser_components_load_signals_void_function() {
    let elisp_form = r##"(ac-geiser-documentation
               "fixture")"##;
    let expect = expect!["ERR (void-function geiser-doc--get-docstring)"];

    assert_ac_geiser_signal_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_documentation_helpers_exist_after_current_geiser_components_load() {
    let elisp_form = r##"(progn
               (require 'geiser-doc)
               (require 'geiser-autodoc)
               (mapcar
                #'functionp
                '(geiser-doc--get-docstring
                  geiser-autodoc--str*)))"##;
    let expect = expect!["OK (t t)"];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_documentation_strips_properties_uses_fresh_symbol_and_forwards_nil_module() {
    let elisp_form = r##"(let ((candidate
                    (propertize
                     "neomacs-ac-geiser-fixture"
                     'summary "candidate"
                     'fixture '(nested value)))
                   calls)
               (cl-letf
                   (((symbol-function
                      'geiser-doc--get-docstring)
                     (lambda (symbol module)
                       (push
                        (list
                         'doc
                         (symbol-name symbol)
                         (intern-soft
                          (symbol-name symbol))
                         module)
                        calls)
                       '(("signature"
                          . ("fixture" "arg"))
                         ("docstring"
                          . "Fixture docs"))))
                    ((symbol-function
                      'geiser-autodoc--str*)
                     (lambda (signature)
                       (push
                        (list
                         'render
                         signature)
                        calls)
                       "fixture(arg)")))
                 (list
                  (ac-geiser-documentation
                   candidate)
                  (nreverse calls)
                  candidate
                  (text-properties-at
                   0 candidate))))"##;
    let expect = expect![[
        r#"OK ("fixture(arg)\n----\nFixture docs" ((doc "neomacs-ac-geiser-fixture" nil nil) (render ("fixture" "arg"))) #("neomacs-ac-geiser-fixture" 0 25 (summary "candidate" fixture (nested value))) (summary "candidate" fixture (nested value)))"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_documentation_uses_first_duplicate_fields_and_empty_doc_fallback() {
    let elisp_form = r##"(let (rendered)
               (cl-letf
                   (((symbol-function
                      'geiser-doc--get-docstring)
                     (lambda (symbol _module)
                       (pcase
                           (symbol-name symbol)
                         ("duplicate"
                          '(("signature" . first)
                            ("signature" . second)
                            ("docstring" . "first docs")
                            ("docstring" . "second docs")))
                         ("missing-doc"
                          '(("signature" . only)))
                         ("nil-data" nil))))
                    ((symbol-function
                      'geiser-autodoc--str*)
                     (lambda (signature)
                       (push signature rendered)
                       (format
                        "<%S>"
                        signature))))
                 (list
                  (mapcar
                   #'ac-geiser-documentation
                   '("duplicate"
                     "missing-doc"
                     "nil-data"))
                  (nreverse rendered))))"##;
    let expect = expect![[
        r#"OK (("<first>\n----\nfirst docs" "<only>\n----\n" "<nil>\n----\n") (first only nil))"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_documentation_preserves_multiline_unicode_and_text_properties_from_helpers() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'geiser-doc--get-docstring)
                 (lambda (_symbol _module)
                   (list
                    (cons
                     "signature"
                     'fixture)
                    (cons
                     "docstring"
                     (propertize
                      "λ first\n第二 line"
                      'face 'bold)))))
                ((symbol-function
                  'geiser-autodoc--str*)
                 (lambda (_signature)
                   (propertize
                    "(λ x)"
                    'face 'italic))))
               (let ((result
                      (ac-geiser-documentation
                       "unicode")))
                 (list
                  result
                  (substring-no-properties
                   result)
                  (text-properties-at
                   0 result)
                  (text-properties-at
                   9 result))))"##;
    let expect = expect![[
        r#"OK (#("(λ x)\n----\nλ first\n第二 line" 0 5 (face italic) 11 26 (face bold)) "(λ x)\n----\nλ first\n第二 line" (face italic) nil)"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_documentation_propagates_lookup_signals_before_rendering() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'geiser-doc--get-docstring)
                 (lambda (&rest arguments)
                   (signal
                    'error
                    (list
                     "fixture documentation failure"
                     arguments))))
                ((symbol-function
                  'geiser-autodoc--str*)
                 (lambda (_signature)
                   "unexpected")))
               (ac-geiser-documentation
                "fixture"))"##;
    let expect = expect![[r#"ERR (error "fixture documentation failure" (fixture nil))"#]];

    assert_ac_geiser_signal_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_documentation_rejects_non_string_candidate_values() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'geiser-doc--get-docstring)
                 (lambda (&rest _arguments)
                   nil))
                ((symbol-function
                  'geiser-autodoc--str*)
                 (lambda (_signature)
                   "")))
               (ac-geiser-documentation
                'not-a-string))"##;
    let expect = expect!["ERR (wrong-type-argument stringp not-a-string)"];

    assert_ac_geiser_signal_parity(elisp_form, expect);
}
