use expect_test::expect;

use super::{assert_aas_parity, assert_aas_signal_parity};

#[test]
fn aas_string_expansion_runs_conditions_and_hooks_at_exact_points_with_transient_state() {
    let elisp_form = r##"(with-temp-buffer
               (insert "prefix=>")
               (let (events)
                 (setq-local
                  aas-global-condition-hook
                  (list
                   #'aas--key-is-fully-typed?
                   (lambda ()
                     (push
                      (list
                       'global
                       (point)
                       aas-transient-snippet-key
                       aas-transient-snippet-expansion)
                      events)
                     'global-ok))
                  aas-pre-snippet-expand-hook
                  (list
                   (lambda ()
                     (push
                      (list
                       'pre
                       (buffer-string)
                       (point)
                       aas-transient-snippet-key
                       aas-transient-snippet-expansion
                       aas-transient-snippet-condition-result)
                      events)))
                  aas-post-snippet-expand-hook
                  (list
                   (lambda ()
                     (push
                      (list
                       'post
                       (buffer-string)
                       (point)
                       aas-transient-snippet-key
                       aas-transient-snippet-expansion
                       aas-transient-snippet-condition-result)
                      events))))
                 (let ((result
                        (aas-expand-snippet-maybe
                         "=>" "→"
                         (lambda ()
                           (push
                            (list 'condition (point))
                            events)
                           'condition-ok))))
                   (list
                    result
                    (buffer-string)
                    (point)
                    (nreverse events)
                    aas-transient-snippet-key
                    aas-transient-snippet-expansion
                    aas-transient-snippet-condition-result))))"##;
    let expect = expect![[
        r#"OK (t "prefix→" 8 ((global 7 "=>" "→") (condition 7) (pre "prefix" 7 "=>" "→" condition-ok) (post "prefix→" 8 "=>" "→" condition-ok)) nil nil nil)"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_global_condition_short_circuits_specific_condition_and_all_expansion_hooks() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abc")
               (let (events)
                 (setq-local
                  aas-global-condition-hook
                  (list
                   (lambda ()
                     (push '(global-first) events)
                     nil)
                   (lambda ()
                     (push '(global-unexpected) events)
                     t))
                  aas-pre-snippet-expand-hook
                  (list
                   (lambda ()
                     (push '(pre-unexpected) events)))
                  aas-post-snippet-expand-hook
                  (list
                   (lambda ()
                     (push '(post-unexpected) events))))
                 (list
                  (aas-expand-snippet-maybe
                   "abc" "expanded"
                   (lambda ()
                     (push '(specific-unexpected) events)
                     t))
                  (buffer-string)
                  (point)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (nil "abc" 4 ((global-first)))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_specific_condition_runs_before_the_key_and_nil_restores_point_without_expanding() {
    let elisp_form = r##"(with-temp-buffer
               (insert "xxabc")
               (let (events)
                 (setq-local
                  aas-global-condition-hook
                  (list #'aas--key-is-fully-typed?)
                  aas-pre-snippet-expand-hook
                  (list
                   (lambda ()
                     (push '(pre-unexpected) events))))
                 (list
                  (aas-expand-snippet-maybe
                   "abc" "expanded"
                   (lambda ()
                     (push
                      (list 'condition (point))
                      events)
                     nil))
                  (buffer-string)
                  (point)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (nil "xxabc" 6 ((condition 3)))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_function_expansion_is_called_interactively_after_key_deletion() {
    let elisp_form = r##"(with-temp-buffer
               (insert "call!")
               (let (events)
                 (setq-local
                  aas-global-condition-hook
                  (list #'aas--key-is-fully-typed?))
                 (cl-letf
                     (((symbol-function
                        'neomacs-aas-interactive-expansion)
                       (lambda (prefix)
                         (interactive "p")
                         (push
                          (list
                           'command
                           prefix
                           (buffer-string)
                           (point)
                           (called-interactively-p 'interactive))
                          events)
                         (insert "[done]")
                         'command-result)))
                   (list
                    (aas-expand-snippet-maybe
                     "!" #'neomacs-aas-interactive-expansion)
                    (buffer-string)
                    (point)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (t "call[done]" 11 ((command 1 "call" 5 nil)))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_yas_and_tempel_expansions_forward_their_payloads_with_distinct_call_shapes() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'yas-expand-snippet)
                     (lambda (&rest arguments)
                       (push (cons 'yas arguments) events)
                       'yas-result))
                    ((symbol-function 'tempel-insert)
                     (lambda (snippet)
                       (push (list 'tempel snippet) events)
                       'tempel-result)))
                 (list
                  (with-temp-buffer
                    (insert "yas")
                    (setq-local
                     aas-global-condition-hook
                     (list #'aas--key-is-fully-typed?))
                    (list
                     (aas-expand-snippet-maybe
                      "yas"
                      '(yas "${1:body}" 2 4))
                     (buffer-string)))
                  (with-temp-buffer
                    (insert "tmp")
                    (setq-local
                     aas-global-condition-hook
                     (list #'aas--key-is-fully-typed?))
                    (list
                     (aas-expand-snippet-maybe
                      "tmp"
                      '(tempel "body" p "tail"))
                     (buffer-string)))
                  (nreverse events))))"##;
    let expect =
        expect![[r#"OK ((t "") (t "") ((yas "${1:body}" 2 4) (tempel ("body" p "tail"))))"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_invalid_expansion_deletes_key_and_runs_pre_hook_before_signaling_without_post_hook() {
    let elisp_form = r##"(with-temp-buffer
               (insert "prefixbad")
               (let (events signal)
                 (setq-local
                  aas-global-condition-hook
                  (list #'aas--key-is-fully-typed?)
                  aas-pre-snippet-expand-hook
                  (list
                   (lambda ()
                     (push
                      (list 'pre (buffer-string) (point))
                      events)))
                  aas-post-snippet-expand-hook
                  (list
                   (lambda ()
                     (push '(post-unexpected) events))))
                 (condition-case error
                     (aas-expand-snippet-maybe
                      "bad" '(unsupported payload))
                   (error
                    (setq signal error)))
                 (list
                  signal
                  (buffer-string)
                  (point)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ((error "Invalid AAS expansion form: (unsupported payload)") "prefix" 7 ((pre "prefix" 7)))"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_expansion_with_a_key_longer_than_the_available_prefix_signals_beginning_of_buffer() {
    let elisp_form = r##"(with-temp-buffer
               (insert "x")
               (aas-expand-snippet-maybe "longer" "value"))"##;
    let expect = expect!["ERR (beginning-of-buffer)"];

    assert_aas_signal_parity(elisp_form, expect);
}
