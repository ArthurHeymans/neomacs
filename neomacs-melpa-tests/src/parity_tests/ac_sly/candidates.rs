use expect_test::expect;

use super::{assert_ac_sly_parity, assert_ac_sly_signal_parity};

#[test]
fn ac_sly_fuzzy_candidates_short_circuit_every_completion_input_when_disconnected() {
    let elisp_form = r##"(let (calls)
               (makunbound
                'ac-prefix)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       nil))
                    ((symbol-function
                      'sly-fuzzy-completions)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'unexpected)))
                 (list
                  (ac-source-sly-fuzzy-candidates)
                  calls
                  (boundp
                   'ac-prefix))))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fuzzy_candidates_strip_prefix_properties_bind_limit_and_attach_exact_summaries() {
    let elisp_form = r##"(let* ((alpha
                     "Alpha")
                    (beta
                     "Beta")
                    (gamma
                     (propertize
                      "Gamma"
                      'source
                      'candidate))
                    (rows
                     (list
                      (list
                       alpha
                       9
                       "A!")
                      (list
                       beta
                       nil
                       nil)
                      (list
                       gamma
                       'middle
                       "G!")))
                    (rows-before
                     (mapcar
                      (lambda (row)
                        (cons
                         (copy-sequence
                          (car
                           row))
                         (copy-tree
                          (cdr
                           row))))
                      rows))
                    (ac-prefix
                     (propertize
                      "MiXeD"
                      'source
                      'prefix))
                    (ac-sly-show-flags
                     t)
                    (sly-fuzzy-completion-limit
                     777)
                    calls)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       'connected))
                    ((symbol-function
                      'sly-fuzzy-completions)
                     (lambda (prefix)
                       (push
                        (list
                         prefix
                         (text-properties-at
                          0
                          prefix)
                         (symbol-value
                          'sly-fuzzy-completion-limit))
                        calls)
                       (list
                        rows
                        "ignored-common"))))
                 (let ((result
                        (ac-source-sly-fuzzy-candidates)))
                   (list
                    result
                    (mapcar
                     (lambda (candidate)
                       (list
                        (substring-no-properties
                         candidate)
                        (text-properties-at
                         0
                         candidate)))
                     result)
                    (cl-mapcar
                     (lambda (candidate row)
                       (eq
                        candidate
                        (car
                         row)))
                     result
                     rows)
                    (equal
                     rows
                     rows-before)
                    rows
                    (nreverse
                     calls)
                    sly-fuzzy-completion-limit
                    (text-properties-at
                     0
                     ac-prefix)))))"##;
    let expect = expect![[
        r#"OK ((#("Alpha" 0 5 (summary "A!")) #("Beta" 0 4 (summary nil)) #("Gamma" 0 5 (summary "G!" source candidate))) (("Alpha" (summary "A!")) ("Beta" (summary nil)) ("Gamma" (summary "G!" source candidate))) (nil nil nil) t (("Alpha" 9 "A!") ("Beta" nil nil) (#("Gamma" 0 5 (source candidate)) middle "G!")) (("MiXeD" nil 50)) 777 (source prefix))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fuzzy_candidates_without_flags_preserve_candidate_identity_and_properties() {
    let elisp_form = r##"(let* ((first
                     (propertize
                      "One"
                      'source
                      1))
                    (second
                     (propertize
                      "Two"
                      'source
                      2))
                    (rows
                     (list
                      (list
                       first
                       'score
                       "1")
                      (list
                       second
                       'score
                       "2")))
                    (ac-prefix
                     "prefix")
                    (ac-sly-show-flags
                     nil))
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-fuzzy-completions)
                     (lambda (_prefix)
                       (list
                        rows))))
                 (let ((result
                        (ac-source-sly-fuzzy-candidates)))
                   (list
                    result
                    (eq
                     (car result)
                     first)
                    (eq
                     (cadr result)
                     second)
                    (mapcar
                     (lambda (candidate)
                       (text-properties-at
                        0
                        candidate))
                     result)))))"##;
    let expect = expect![[
        r#"OK ((#("One" 0 3 (source 1)) #("Two" 0 3 (source 2))) t t ((source 1) (source 2)))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fuzzy_candidates_preserve_empty_completion_results() {
    let elisp_form = r##"(let ((ac-prefix
                    "prefix")
                   (ac-sly-show-flags
                    t)
                   calls)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-fuzzy-completions)
                     (lambda (prefix)
                       (push
                        prefix
                        calls)
                       '(nil "common"))))
                 (list
                  (ac-source-sly-fuzzy-candidates)
                  (nreverse
                   calls))))"##;
    let expect = expect![[r#"OK (nil ("prefix"))"#]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fuzzy_candidates_propagate_completion_signals_with_the_dynamic_limit() {
    let elisp_form = r##"(let ((ac-prefix
                    "prefix"))
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-fuzzy-completions)
                     (lambda (prefix)
                       (signal
                        'error
                        (list
                         prefix
                         sly-fuzzy-completion-limit)))))
                 (ac-source-sly-fuzzy-candidates)))"##;
    let expect = expect![[r#"ERR (error "prefix" 50)"#]];

    assert_ac_sly_signal_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fuzzy_candidates_reject_non_string_prefix_before_calling_backend() {
    let elisp_form = r##"(let ((ac-prefix
                    'not-a-string)
                   calls)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-fuzzy-completions)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'unexpected)))
                 (ac-source-sly-fuzzy-candidates)))"##;
    let expect = expect!["ERR (wrong-type-argument stringp not-a-string)"];

    assert_ac_sly_signal_parity(elisp_form, expect);
}

#[test]
fn ac_sly_simple_candidates_short_circuit_every_completion_input_when_disconnected() {
    let elisp_form = r##"(let (calls)
               (makunbound
                'ac-prefix)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       nil))
                    ((symbol-function
                      'sly-simple-completions)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'unexpected)))
                 (list
                  (ac-source-sly-simple-candidates)
                  calls
                  (boundp
                   'ac-prefix))))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_simple_candidates_strip_properties_and_select_nested_or_flat_response_shapes() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-simple-completions)
                     (lambda (prefix)
                       (push
                        (list
                         prefix
                         (text-properties-at
                          0
                          prefix))
                        calls)
                       (cond
                        ((equal
                          prefix
                          "nested")
                         '(("one"
                            "two")
                           "common"))
                        ((equal
                          prefix
                          "flat")
                         '("one"
                           "two"))
                        ((equal
                          prefix
                          "empty")
                         nil)
                        (t
                         'unexpected)))))
                 (list
                  (let ((ac-prefix
                         (propertize
                          "nested"
                          'source
                          t)))
                    (ac-source-sly-simple-candidates))
                  (let ((ac-prefix
                         (propertize
                          "flat"
                          'source
                          t)))
                    (ac-source-sly-simple-candidates))
                  (let ((ac-prefix
                         (propertize
                          "empty"
                          'source
                          t)))
                    (ac-source-sly-simple-candidates))
                  (nreverse
                   calls))))"##;
    let expect =
        expect![[r#"OK (("one" "two") "one" nil (("nested" nil) ("flat" nil) ("empty" nil)))"#]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_simple_candidates_return_the_selected_response_object_by_identity() {
    let elisp_form = r##"(let* ((nested-result
                     (list
                      "nested-a"
                      "nested-b"))
                    (nested-response
                     (list
                      nested-result
                      "common"))
                    (flat-response
                     (list
                      "flat-a"
                      "flat-b")))
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-simple-completions)
                     (lambda (prefix)
                       (if
                           (equal
                            prefix
                            "nested")
                           nested-response
                         flat-response))))
                 (list
                  (let ((ac-prefix
                         "nested"))
                    (eq
                     (ac-source-sly-simple-candidates)
                     nested-result))
                  (let ((ac-prefix
                         "flat"))
                    (let ((result
                           (ac-source-sly-simple-candidates)))
                      (list
                       (eq
                        result
                        (car
                         flat-response))
                       result)))
                  nested-response
                  flat-response)))"##;
    let expect =
        expect![[r#"OK (t (t "flat-a") (("nested-a" "nested-b") "common") ("flat-a" "flat-b"))"#]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_simple_candidates_propagate_completion_signals_with_exact_prefix() {
    let elisp_form = r##"(let ((ac-prefix
                    (propertize
                     "prefix"
                     'source
                     t)))
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-simple-completions)
                     (lambda (prefix)
                       (signal
                        'error
                        (list
                         prefix
                         (text-properties-at
                          0
                          prefix))))))
                 (ac-source-sly-simple-candidates)))"##;
    let expect = expect![[r#"ERR (error "prefix" nil)"#]];

    assert_ac_sly_signal_parity(elisp_form, expect);
}

#[test]
fn ac_sly_simple_candidates_reject_non_string_prefix_before_calling_backend() {
    let elisp_form = r##"(let ((ac-prefix
                    'not-a-string)
                   calls)
               (cl-letf
                   (((symbol-function
                      'sly-connected-p)
                     (lambda ()
                       t))
                    ((symbol-function
                      'sly-simple-completions)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'unexpected)))
                 (ac-source-sly-simple-candidates)))"##;
    let expect = expect!["ERR (wrong-type-argument stringp not-a-string)"];

    assert_ac_sly_signal_parity(elisp_form, expect);
}
