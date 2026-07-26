use expect_test::expect;

use super::assert_ac_php_parity;

#[test]
fn ac_php_document_formats_members_with_cleaned_help_and_all_properties() {
    let elisp_form = r##"(mapcar
               (lambda (tag-type)
                 (let ((item
                        (propertize
                         "member"
                         'ac-php-help
                         "[#summary#]"
                         'ac-php-tag-type
                         tag-type
                         'ac-php-return-type
                         "Result"
                         'ac-php-access
                         "protected"
                         'ac-php-from
                         "\\Acme\\Base")))
                   (let ((document
                          (ac-php-document
                           item)))
                     (list
                      tag-type
                      (substring-no-properties
                       document)
                      (text-properties-at
                       0 document)))))
               '("p" "m" "d"))"##;
    let expect = expect![[
        r#"OK (("p" "member\n\11[  type]:Result\n\11[access]:protected\n\11[  from]:\\Acme\\Base" (ac-php-from "\\Acme\\Base" ac-php-access "protected" ac-php-return-type "Result" ac-php-tag-type "p" ac-php-help "[#summary#]")) ("m" "member\n\11[  type]:Result\n\11[access]:protected\n\11[  from]:\\Acme\\Base" (ac-php-from "\\Acme\\Base" ac-php-access "protected" ac-php-return-type "Result" ac-php-tag-type "m" ac-php-help "[#summary#]")) ("d" "member\n\11[  type]:Result\n\11[access]:protected\n\11[  from]:\\Acme\\Base" (ac-php-from "\\Acme\\Base" ac-php-access "protected" ac-php-return-type "Result" ac-php-tag-type "d" ac-php-help "[#summary#]")))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_document_formats_function_return_and_plain_item_branches() {
    let elisp_form = r##"(let ((function-item
                    (propertize
                     "calculate("
                     'ac-php-help
                     "[#$value#]"
                     'ac-php-tag-type
                     "f"
                     'ac-php-return-type
                     "Result"))
                   (return-item
                    (propertize
                     "constant"
                     'ac-php-help
                     "<#ignored#>"
                     'ac-php-tag-type
                     "c"
                     'ac-php-return-type
                     "int"))
                   (plain-item
                    (propertize
                     "plain"
                     'ac-php-help
                     "<#ignored#>"
                     'ac-php-tag-type
                     "c")))
               (mapcar
                (lambda (item)
                  (let ((document
                         (ac-php-document
                          item)))
                    (let ((property-position
                           (if
                               (text-properties-at
                                0 document)
                               0
                             (next-property-change
                              0 document))))
                      (list
                       (substring-no-properties
                        document)
                       property-position
                       (text-properties-at
                        property-position
                        document)))))
                (list
                 function-item
                 return-item
                 plain-item)))"##;
    let expect = expect![[
        r#"OK (("Result calculate($value ) " 7 (ac-php-help "[#$value#]" ac-php-tag-type "f" ac-php-return-type "Result")) ("int constant " 4 (ac-php-return-type "int" ac-php-tag-type "c" ac-php-help "<#ignored#>")) ("plain" 0 (ac-php-help "<#ignored#>" ac-php-tag-type "c")))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_document_formats_functions_without_return_type_or_help() {
    let elisp_form = r##"(let ((without-return
                    (propertize
                     "call("
                     'ac-php-help
                     "[#$argument#]"
                     'ac-php-tag-type
                     "f"))
                   (without-help
                    (propertize
                     "empty("
                     'ac-php-tag-type
                     "f")))
               (mapcar
                (lambda (item)
                  (substring-no-properties
                   (ac-php-document
                    item)))
                (list
                 without-return
                 without-help)))"##;
    let expect = expect![[r#"OK ("call($argument )" "empty()")"#]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_document_and_template_document_handle_nonstrings_and_missing_help() {
    let elisp_form = r##"(let ((plain
                    "plain")
                   (with-help
                    (propertize
                     "template"
                     'ac-php-help
                     "[#clean#]")))
               (list
                (ac-php-document nil)
                (ac-php-document 'symbol)
                (ac-php-document plain)
                (ac-php-template-document nil)
                (ac-php-template-document 'symbol)
                (ac-php-template-document plain)
                (ac-php-template-document
                 with-help)))"##;
    let expect = expect![[r#"OK (nil nil "plain" nil nil nil "clean ")"#]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_template_candidate_and_prefix_return_live_state_by_identity() {
    let elisp_form = r##"(let* ((candidates
                     (list
                      "first"
                      "second"))
                    (ac-php-template-candidates
                     candidates)
                    (ac-php-template-start-point
                     (copy-marker
                      17)))
               (let ((returned-candidates
                      (ac-php-template-candidate))
                     (returned-prefix
                      (ac-php-template-prefix)))
                 (list
                  returned-candidates
                  (eq
                   returned-candidates
                   candidates)
                  returned-prefix
                  (eq
                   returned-prefix
                   ac-php-template-start-point))))"##;
    let expect = expect![[r#"OK (("first" "second") t (:marker 1 "*scratch*") t)"#]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_candidate_adapter_copies_prefix_then_delegates_once() {
    let elisp_form = r##"(let ((ac-prefix
                    "needle")
                   (ac-php-prefix-str
                    "stale")
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-candidate)
                     (lambda ()
                       (push
                        ac-php-prefix-str
                        calls)
                       '("first"
                         "second"))))
                 (list
                  (ac-php-candidate-ac)
                  ac-php-prefix-str
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (("first" "second") "needle" ("needle"))"#]];

    assert_ac_php_parity(elisp_form, expect);
}
