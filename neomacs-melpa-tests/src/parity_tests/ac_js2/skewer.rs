use expect_test::expect;

use super::assert_ac_js2_parity;

#[test]
fn ac_js2_skewer_completion_and_document_candidates_preserve_exact_entries() {
    let elisp_form = r##"(let ((ac-js2-skewer-candidates
                    '((alpha
                       . "function alpha(x, y) { return x; }")
                      (beta
                       . "plain documentation")
                      (alpha
                       . "later duplicate"))))
               (list
                (ac-js2-skewer-completion-candidates)
                (ac-js2-skewer-document-candidates
                 "alpha")
                (ac-js2-skewer-document-candidates
                 "beta")
                (ac-js2-skewer-document-candidates
                 "ALPHA")
                (ac-js2-skewer-document-candidates
                 "missing")))"##;
    let expect = expect![[
        r#"OK (("alpha" "beta" "alpha") "function alpha(x, y)" "plain documentation" nil nil)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_skewer_result_callback_replaces_candidates_only_for_success_with_value() {
    let elisp_form = r##"(let ((ac-js2-skewer-candidates
                    '(original))
                   calls)
               (cl-letf
                   (((symbol-function
                      'skewer-success-p)
                     (lambda (result)
                       (push result calls)
                       (eq
                        (cdr
                         (assq 'status result))
                        'success))))
                 (let ((success-return
                        (ac-js2-skewer-result-callback
                         '((status . success)
                           (value
                            . [(alpha . "doc")
                               (beta . nil)])))))
                   (let ((after-success
                          ac-js2-skewer-candidates)
                         (failure-return
                          (ac-js2-skewer-result-callback
                           '((status . failure)
                             (value
                              . [(wrong . "value")])))))
                     (let ((missing-return
                            (ac-js2-skewer-result-callback
                             '((status . success)))))
                       (list
                        success-return
                        (eq
                         success-return
                         after-success)
                        after-success
                        failure-return
                        missing-return
                        ac-js2-skewer-candidates
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (#1=(#2=(alpha . "doc") #3=(beta)) t #1# nil nil #1# (((status . success) (value . [#2# #3#])) ((status . failure) (value . [(wrong . "value")])) ((status . success))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_get_object_properties_forwards_name_and_live_prototype_setting() {
    let elisp_form = r##"(let (calls
                   (ac-js2-add-prototype-completions
                    'fixture-prototypes))
               (cl-letf
                   (((symbol-function
                      'ac-js2-skewer-eval-wrapper)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       'fixture-result)))
                 (list
                  (ac-js2-get-object-properties
                   "object.property")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (fixture-result (("object.property" ((prototypes . fixture-prototypes)))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_skewer_eval_wrapper_evaluates_safe_input_and_processes_result() {
    let elisp_form = r##"(let ((skewer-clients
                    '(fixture-client))
                   (ac-js2-evaluate-calls
                    nil)
                   (ac-js2-skewer-candidates
                    '(stale))
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-js2-has-function-calls)
                     (lambda (string)
                       (push
                        (list 'calls-p string)
                        events)
                       nil))
                    ((symbol-function
                      'skewer-eval-synchronously)
                     (lambda (&rest arguments)
                       (push
                        (cons 'eval arguments)
                        events)
                       '((status . success)
                         (value
                          . [(first . "one")
                             (second . "two")]))))
                    ((symbol-function
                      'skewer-success-p)
                     (lambda (result)
                       (push
                        (list 'success-p result)
                        events)
                       t)))
                 (list
                  (ac-js2-skewer-eval-wrapper
                   "safe.property"
                   '((fixture . exact)))
                  ac-js2-skewer-candidates
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (#1=(#2=(first . "one") #3=(second . "two")) #1# ((calls-p "safe.property") (eval "safe.property" :type "complete" :extra ((fixture . exact))) (success-p ((status . success) (value . [#2# #3#])))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_skewer_eval_wrapper_blocks_calls_or_clears_a_disconnected_queue() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-js2-has-function-calls)
                     (lambda (string)
                       (push
                        (list 'calls-p string)
                        events)
                       t))
                    ((symbol-function
                      'skewer-eval-synchronously)
                     (lambda (&rest arguments)
                       (push
                        (cons 'unexpected-eval
                              arguments)
                        events)
                       (error
                        "evaluation must be blocked"))))
                 (let ((skewer-clients
                        '(fixture-client))
                       (skewer-queue
                        '(queued-request))
                       (ac-js2-evaluate-calls
                        nil)
                       (ac-js2-skewer-candidates
                        '(stale)))
                   (let ((blocked-return
                          (ac-js2-skewer-eval-wrapper
                           "danger()")))
                     (let ((blocked-state
                            (list
                             blocked-return
                             ac-js2-skewer-candidates
                             skewer-queue)))
                       (let ((skewer-clients
                              nil)
                             (skewer-queue
                              '(queued-request))
                             (ac-js2-skewer-candidates
                              '(stale)))
                         (list
                          blocked-state
                          (ac-js2-skewer-eval-wrapper
                           "disconnected()")
                          ac-js2-skewer-candidates
                          skewer-queue
                          (nreverse events))))))))"##;
    let expect = expect![[r#"OK ((nil nil (queued-request)) nil nil nil ((calls-p "danger()")))"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_skewer_eval_wrapper_allows_calls_without_call_detection_when_opted_in() {
    let elisp_form = r##"(let ((skewer-clients
                    '(fixture-client))
                   (ac-js2-evaluate-calls
                    t)
                   (ac-js2-skewer-candidates
                    '(stale))
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-js2-has-function-calls)
                     (lambda (_string)
                       (push '(unexpected-detection)
                             events)
                       (error
                        "call detection must be bypassed")))
                    ((symbol-function
                      'skewer-eval-synchronously)
                     (lambda (&rest arguments)
                       (push
                        (cons 'eval arguments)
                        events)
                       '((status . success)
                         (value
                          . [(allowed
                              . "function allowed()")]))))
                    ((symbol-function
                      'skewer-success-p)
                     (lambda (result)
                       (push
                        (list 'success-p result)
                        events)
                       t)))
                 (list
                  (ac-js2-skewer-eval-wrapper
                   "dangerous()")
                  ac-js2-skewer-candidates
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (#1=(#2=(allowed . "function allowed()")) #1# ((eval "dangerous()" :type "complete" :extra nil) (success-p ((status . success) (value . [#2#])))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_on_skewer_load_injects_addon_and_evaluates_each_external_library() {
    let elisp_form = r##"(let ((ac-js2-data-root
                    "/fixture/package/")
                   (ac-js2-evaluate-calls
                    t)
                   (ac-js2-external-libraries
                    '("/fixture/lib/one.js"
                      "/fixture/lib/two.js"))
                   events)
               (cl-letf
                   (((symbol-function
                      'insert-file-contents)
                     (lambda (filename
                              &rest _arguments)
                       (push
                        (list 'insert filename)
                        events)
                       (insert
                        (concat
                         "contents:"
                         (file-name-nondirectory
                          filename)))
                       '(fixture)))
                    ((symbol-function
                      'skewer-eval)
                     (lambda (&rest arguments)
                       (push
                        (cons 'eval arguments)
                        events)
                       (length events))))
                 (list
                  (ac-js2-on-skewer-load)
                  (buffer-string)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ((3 5) "contents:skewer-addon.js" ((insert "/fixture/package/skewer-addon.js") (insert "/fixture/lib/one.js") (eval "contents:one.js" nil :type "complete") (insert "/fixture/lib/two.js") (eval "contents:two.js" nil :type "complete")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_on_skewer_load_skips_external_libraries_when_evaluation_is_disabled() {
    let elisp_form = r##"(let ((ac-js2-data-root
                    "/fixture/package/")
                   (ac-js2-evaluate-calls
                    nil)
                   (ac-js2-external-libraries
                    '("/fixture/lib/one.js"
                      "/fixture/lib/two.js"))
                   events)
               (cl-letf
                   (((symbol-function
                      'insert-file-contents)
                     (lambda (filename
                              &rest _arguments)
                       (push
                        (list 'insert filename)
                        events)
                       (insert
                        (concat
                         "contents:"
                         (file-name-nondirectory
                          filename)))
                       '(fixture)))
                    ((symbol-function
                      'skewer-eval)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'unexpected-eval
                         arguments)
                        events)
                       (error
                        "external libraries must not be evaluated"))))
                 (list
                  (ac-js2-on-skewer-load)
                  (buffer-string)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil "contents:skewer-addon.js" ((insert "/fixture/package/skewer-addon.js")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}
