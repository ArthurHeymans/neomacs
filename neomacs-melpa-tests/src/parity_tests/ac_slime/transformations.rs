use expect_test::expect;

use super::{assert_ac_slime_parity, assert_ac_slime_signal_parity};

#[test]
fn ac_slime_case_correcting_completions_downcase_lookup_and_mutate_matched_collection_strings() {
    let elisp_form = r##"(let* ((collection
                     '("foo"
                       "foobar"
                       "fOo-baz"
                       "other"))
                    (before
                     (mapcar
                      #'copy-sequence
                      collection))
                    (result
                     (ac-source-slime-case-correcting-completions
                      "FO"
                      collection)))
               (list
                result
                before
                collection
                (mapcar
                 (lambda (candidate)
                   (text-properties-at
                    0
                    candidate))
                 result)))"##;
    let expect = expect![[
        r#"OK (("FOo" "FOobar") ("foo" "foobar" "fOo-baz" "other") ("FOo" "FOobar" "fOo-baz" "other") (nil nil))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_case_correcting_completions_cover_empty_no_match_and_exact_names() {
    let elisp_form = r##"(list
              (ac-source-slime-case-correcting-completions
               ""
               '("Alpha"
                 "beta"))
              (ac-source-slime-case-correcting-completions
               "ZZ"
               '("alpha"
                 "beta"))
              (ac-source-slime-case-correcting-completions
               "ALPHA"
               '("alpha"
                 "alphabet")))"##;
    let expect = expect![[r#"OK (("Alpha" "beta") nil ("ALPHA" "ALPHAbet"))"#]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_case_correcting_completions_surface_unicode_replacement_signal() {
    let elisp_form = r##"(ac-source-slime-case-correcting-completions
              "Å"
              '("åland"
                "älg"
                "other"))"##;
    let expect =
        expect![[r#"ERR (error "Attempt to store non-ASCII char into multibyte string")"#]];

    assert_ac_slime_signal_parity(elisp_form, expect);
}

#[test]
fn ac_slime_case_correcting_completions_preserve_name_properties_on_replaced_prefix() {
    let elisp_form = r##"(let* ((name
                     (propertize
                      "FO"
                      'source
                      'name))
                    (result
                     (ac-source-slime-case-correcting-completions
                      name
                      '("foobar"))))
               (list
                result
                (mapcar
                 (lambda (index)
                   (text-properties-at
                    index
                    (car
                     result)))
                 '(0 1 2 3 4 5))
                name
                (text-properties-at
                 0
                 name)))"##;
    let expect = expect![[
        r#"OK (("FOobar") (nil nil nil nil nil nil) #("FO" 0 2 (source name)) (source name))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_case_correcting_completions_propagate_invalid_collection_signals() {
    let elisp_form = r##"(ac-source-slime-case-correcting-completions
              "FO"
              17)"##;
    let expect = expect!["ERR (invalid-function 17)"];

    assert_ac_slime_signal_parity(elisp_form, expect);
}

#[test]
fn ac_slime_case_correcting_completions_reject_non_string_name_before_collection_lookup() {
    let elisp_form = r##"(ac-source-slime-case-correcting-completions
              'not-a-string
              '("alpha"
                "beta"))"##;
    let expect = expect!["ERR (wrong-type-argument char-or-string-p not-a-string)"];

    assert_ac_slime_signal_parity(elisp_form, expect);
}

#[test]
fn ac_slime_documentation_strips_properties_and_forwards_exact_swank_request() {
    let elisp_form = r##"(let ((symbol-name
                    (propertize
                     "pkg:symbol"
                     'source
                     t))
                   calls
                   (response
                    (list
                     'documentation
                     "body")))
               (cl-letf
                   (((symbol-function
                      'slime-eval)
                     (lambda (form)
                       (push
                        (list
                         form
                         (text-properties-at
                          0
                          (cadr
                           form)))
                        calls)
                       response)))
                 (let ((result
                        (ac-slime-documentation
                         symbol-name)))
                   (list
                    result
                    (eq
                     result
                     response)
                    (nreverse
                     calls)
                    symbol-name
                    (text-properties-at
                     0
                     symbol-name)))))"##;
    let expect = expect![[
        r#"OK ((documentation "body") t (((swank:documentation-symbol "pkg:symbol") nil)) #("pkg:symbol" 0 10 (source t)) (source t))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_documentation_propagates_slime_eval_signals_with_exact_request() {
    let elisp_form = r##"(cl-letf
              (((symbol-function
                 'slime-eval)
                (lambda (form)
                  (signal
                   'error
                   (list
                    form)))))
              (ac-slime-documentation
               "fixture"))"##;
    let expect = expect![[r#"ERR (error (swank:documentation-symbol "fixture"))"#]];

    assert_ac_slime_signal_parity(elisp_form, expect);
}

#[test]
fn ac_slime_documentation_rejects_non_strings_before_contacting_slime() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'slime-eval)
                     (lambda (form)
                       (push
                        form
                        calls)
                       'unexpected)))
                 (ac-slime-documentation
                  'not-a-string)))"##;
    let expect = expect!["ERR (wrong-type-argument stringp not-a-string)"];

    assert_ac_slime_signal_parity(elisp_form, expect);
}

#[test]
fn ac_slime_init_clears_the_current_dynamic_documentation_slot_and_returns_nil() {
    let elisp_form = r##"(let ((ac-slime-current-doc
                    (list
                     'old
                     "documentation")))
               (list
                (ac-slime-init)
                ac-slime-current-doc))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ac_slime_parity(elisp_form, expect);
}
