use expect_test::expect;

use super::assert_apiwrap_parity;

#[test]
fn apiwrap_plist_conversion_rejects_odd_and_dotted_inputs_with_native_errors() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (condition-case error
               (apiwrap-plist->alist value)
             (error (list (car error) (cdr error)))))
         '((:one)
           (:one 1 :two)
           (:one 1 . dangling)))"##;
    let expect = expect![[
        r#"OK ((error ("bad plist")) (error ("bad plist")) (wrong-type-argument (listp dangling)))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_url_encoder_rejects_non_numeric_non_string_values_natively() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (condition-case error
               (apiwrap--encode-url value)
             (error
              (list (car error)
                    (mapcar
                     (lambda (item)
                       (if (stringp item)
                           (replace-regexp-in-string
                            "0x[0-9a-f]+" "<address>" item)
                         item))
                     (cdr error))))))
         '(nil owner (:path "x") [1 2]))"##;
    let expect = expect![[
        r#"OK ("/" (wrong-type-argument (char-or-string-p owner)) (wrong-type-argument (char-or-string-p (:path "x"))) (wrong-type-argument (char-or-string-p [1 2])))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_non_macro_around_configuration_is_rejected_at_endpoint_definition() {
    let elisp_form = r##"(progn
         (apiwrap-new-backend "Bad" "awbadaround" nil
           :request #'ignore
           :around #'identity)
         (condition-case error
             (eval
              '(defapiget-awbadaround "/items"
                 "Items."
                 "items"))
           (error (list (car error) (cdr error)))))"##;
    let expect = expect![[r#"OK (error (":around must be a macro: identity"))"#]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_invalid_condition_handler_is_rejected_at_endpoint_definition() {
    let elisp_form = r##"(progn
         (apiwrap-new-backend "Bad" "awbadhandler" nil
           :request #'ignore)
         (condition-case error
             (eval
              '(defapiget-awbadhandler "/items"
                 "Items."
                 "items"
                 :condition-case
                 ((not-an-error-condition nil))))
           (error (list (car error) (cdr error)))))"##;
    let expect = expect![[
        r#"OK (error (":condition-case must be a list of error handlers; see the documentation: ((not-an-error-condition nil))"))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_missing_request_primitive_rejects_backend_after_registry_update() {
    let elisp_form = r##"(let ((apiwrap-backends nil))
         (list
          (condition-case error
              (eval
               '(apiwrap-new-backend
                    "Missing" "awnorequest" nil))
            (error (list (car error) (cdr error))))
          apiwrap-backends
          (fboundp 'defapiget-awnorequest)))"##;
    let expect =
        expect![[r#"OK ((wrong-type-argument (consp nil)) (("Missing" . "awnorequest")) nil)"#]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_preprocessor_error_short_circuits_request_primitive() {
    let elisp_form = r##"(progn
         (define-error 'awinvalid-params "Invalid params")
         (defun awpreerror-request (&rest _args) nil)
         (defun awpreerror-params (value) value)
         (apiwrap-new-backend "Errors" "awpreerror" nil
           :request #'awpreerror-request
           :pre-process-params #'awpreerror-params)
         (defapiget-awpreerror "/search" "Search." "search")
         (let (calls)
           (cl-letf (((symbol-function 'awpreerror-params)
                      (lambda (params)
                        (signal 'awinvalid-params (list params))))
                     ((symbol-function 'awpreerror-request)
                      (lambda (&rest args)
                        (push args calls)
                        'unexpected)))
             (list
              (condition-case error
                  (awpreerror-get-search :query "bad")
                (error (list (car error) (cdr error))))
              calls))))"##;
    let expect = expect![[r#"OK ((awinvalid-params ((:query "bad"))) nil)"#]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_link_generator_error_prevents_partial_function_definition() {
    let elisp_form = r##"(progn
         (apiwrap-new-backend "Errors" "awlinkerror" nil
           :request #'ignore
           :link (lambda (properties)
                   (error "bad endpoint: %s"
                          (alist-get 'endpoint properties))))
         (list
          (condition-case error
              (eval
               '(defapiget-awlinkerror "/broken"
                  "Broken."
                  "broken"))
            (error (list (car error) (cdr error))))
          (fboundp 'awlinkerror-get-broken)
          (get 'awlinkerror-get-broken 'apiwrap)))"##;
    let expect = expect![[r#"OK ((error ("bad endpoint: /broken")) nil nil)"#]];
    assert_apiwrap_parity(elisp_form, expect);
}
