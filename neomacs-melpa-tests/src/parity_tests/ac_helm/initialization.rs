use expect_test::expect;

use super::assert_ac_helm_parity;

#[test]
fn ac_helm_init_sets_attributes_width_aborts_and_skips_exit_for_many_candidates() {
    let elisp_form = r##"(let ((ac-candidates
                    (list
                     (propertize
                      "alpha"
                      'fixture 'candidate)
                     "alphabet"))
                   (ac-prefix
                    (propertize
                     "al"
                     'fixture 'prefix))
                   calls)
               (cl-letf
                   (((symbol-function
                      'helm-attrset)
                     (lambda (attribute value)
                       (push
                        (list
                         'attrset
                         attribute
                         value)
                        calls)
                       value))
                    ((symbol-function
                      'popup-preferred-width)
                     (lambda (candidates)
                       (push
                        (list
                         'width candidates)
                        calls)
                       17))
                    ((symbol-function
                      'helm-exit-minibuffer)
                     (lambda ()
                       (push '(exit) calls)
                       'exit))
                    ((symbol-function
                      'ac-abort)
                     (lambda ()
                       (push '(abort) calls)
                       'abort-result)))
                 (list
                  (helm-auto-complete-init)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (abort-result ((attrset ac-candidates #1=(#("alpha" 0 5 (fixture candidate)) "alphabet")) (width #1#) (attrset menu-width 17) (attrset ac-prefix #("al" 0 2 (fixture prefix))) (abort)))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_init_exits_before_abort_for_zero_and_one_candidate() {
    let elisp_form = r##"(mapcar
               (lambda (candidates)
                 (let ((ac-candidates candidates)
                       (ac-prefix "prefix")
                       calls)
                   (cl-letf
                       (((symbol-function
                          'helm-attrset)
                         (lambda (attribute value)
                           (push
                            (list
                             'attrset
                             attribute
                             value)
                            calls)))
                        ((symbol-function
                          'popup-preferred-width)
                         (lambda (actual)
                           (push
                            (list 'width actual)
                            calls)
                           9))
                        ((symbol-function
                          'helm-exit-minibuffer)
                         (lambda ()
                           (push '(exit) calls)))
                        ((symbol-function
                          'ac-abort)
                         (lambda ()
                           (push '(abort) calls)
                           'aborted)))
                     (list
                      candidates
                      (helm-auto-complete-init)
                      (nreverse calls)))))
               '(nil ("only")))"##;
    let expect = expect![[
        r#"OK ((nil aborted ((attrset ac-candidates nil) (width nil) (attrset menu-width 9) (attrset ac-prefix "prefix") #2=(exit) #3=(abort))) (#1=("only") aborted ((attrset ac-candidates #1#) (width #1#) (attrset menu-width 9) (attrset ac-prefix "prefix") #2# #3#)))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_init_passes_candidate_and_prefix_objects_without_copying() {
    let elisp_form = r##"(let ((ac-candidates
                    (list "one" "two"))
                   (ac-prefix
                    (copy-sequence "prefix"))
                   candidate-value
                   prefix-value)
               (cl-letf
                   (((symbol-function
                      'helm-attrset)
                     (lambda (attribute value)
                       (pcase attribute
                         ('ac-candidates
                          (setq candidate-value
                                value))
                         ('ac-prefix
                          (setq prefix-value
                                value)))
                       value))
                    ((symbol-function
                      'popup-preferred-width)
                     (lambda (_candidates)
                       5))
                    ((symbol-function
                      'ac-abort)
                     (lambda () nil)))
                 (helm-auto-complete-init)
                 (list
                  (eq
                   candidate-value
                   ac-candidates)
                  (eq
                   prefix-value
                   ac-prefix))))"##;
    let expect = expect!["OK (t t)"];

    assert_ac_helm_parity(elisp_form, expect);
}
