use expect_test::expect;

use super::{assert_ac_helm_parity, assert_ac_helm_signal_parity};

#[test]
fn ac_helm_action_replaces_prefix_calls_candidate_action_and_clears_candidates() {
    let elisp_form = r##"(with-temp-buffer
               (insert "before αβ")
               (goto-char
                (point-max))
               (let ((candidate
                      (propertize
                       "replacement"
                       'action
                       (lambda ()
                         (list
                          'action-result
                          (buffer-string)
                          (point)))))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'helm-attr)
                       (lambda (attribute)
                         (push
                          (list
                           'attr attribute)
                          calls)
                         (pcase attribute
                           ('ac-prefix "αβ"))))
                      ((symbol-function
                        'helm-attrset)
                       (lambda (attribute value)
                         (push
                          (list
                           'attrset
                           attribute
                           value)
                          calls)
                         'cleared)))
                   (list
                    (helm-auto-complete-action
                     candidate)
                    (buffer-string)
                    (point)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((action-result #("before replacement" 7 18 (action #1=#[nil ((list 'action-result (buffer-string) (point))) (t)])) 19) #("before replacement" 7 18 (action #1#)) 19 ((attr ac-prefix) (attrset ac-candidates nil)))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_action_without_callback_inserts_plain_text_and_still_clears_candidates() {
    let elisp_form = r##"(with-temp-buffer
               (insert "prefix")
               (goto-char
                (point-max))
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'helm-attr)
                       (lambda (_attribute)
                         "prefix"))
                      ((symbol-function
                        'helm-attrset)
                       (lambda (attribute value)
                         (push
                          (list attribute value)
                          calls)
                         'cleared)))
                   (list
                    (helm-auto-complete-action
                     "done")
                    (buffer-string)
                    (point)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK (nil "done" 5 ((ac-candidates nil)))"#]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_action_reads_only_the_action_property_at_candidate_start() {
    let elisp_form = r##"(with-temp-buffer
               (insert "x")
               (goto-char
                (point-max))
               (let ((candidate
                      (concat
                       "a"
                       (propertize
                        "b"
                        'action
                        (lambda ()
                          'unexpected))))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'helm-attr)
                       (lambda (_attribute)
                         "x"))
                      ((symbol-function
                        'helm-attrset)
                       (lambda (&rest arguments)
                         (push arguments calls))))
                   (list
                    (helm-auto-complete-action
                     candidate)
                    (buffer-string)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil #("ab" 1 2 (action #[nil ('unexpected) (t)])) ((ac-candidates nil)))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_action_signal_skips_candidate_cache_cleanup_after_insertion() {
    let elisp_form = r##"(with-temp-buffer
               (insert "old")
               (goto-char
                (point-max))
               (let ((candidate
                      (propertize
                       "new"
                       'action
                       (lambda ()
                         (error
                          "fixture action failure")))))
                 (cl-letf
                     (((symbol-function
                        'helm-attr)
                       (lambda (_attribute)
                         "old"))
                      ((symbol-function
                        'helm-attrset)
                       (lambda (&rest arguments)
                         (signal
                          'error
                          (list
                           "cleanup unexpectedly ran"
                           arguments)))))
                   (helm-auto-complete-action
                    candidate))))"##;
    let expect = expect![[r#"ERR (error "fixture action failure")"#]];

    assert_ac_helm_signal_parity(elisp_form, expect);
}
