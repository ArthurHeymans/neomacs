use expect_test::expect;

use super::assert_add_hooks_parity;

#[test]
fn add_hooks_delegates_each_pair_car_and_cdr_in_source_order_without_rewriting() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'add-hooks-pair)
               (lambda (hooks functions)
                 (push
                  (list
                   hooks
                   functions)
                  calls)
                 'pair-result)))
           (list
            (add-hooks
             '((alpha . first)
               ((beta gamma) . (second third))
               (delta
                (lambda ()
                  value))))
            (nreverse
             calls))))"##;
    let expect = expect![
        "OK (nil ((alpha first) ((beta gamma) (second third)) (delta ((lambda nil value)))))"
    ];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_real_mixed_pairs_build_expected_hook_state_order_and_deduplication() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          nil
          beta-hook
          '(existing)
          gamma-hook
          nil)
         (list
          (add-hooks
           '((alpha . first)
             ((alpha beta) . (second third))
             (gamma . first)
             (beta . existing)
             (alpha . first)))
          alpha-hook
          beta-hook
          gamma-hook))"##;
    let expect = expect!["OK (nil (third second first) (third second existing) (first))"];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_lambda_pair_syntax_executes_and_preserves_function_identity() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          nil)
         (let (events)
         (let ((function
                (lambda ()
                  (push
                   'ran
                   events))))
           (add-hooks
            `((alpha
               ,function)))
           (let ((stored
                  (car
                   alpha-hook)))
             (run-hooks
              'alpha-hook)
             (list
              (eq
               stored
               function)
              (functionp
               stored)
              events)))))"##;
    let expect = expect!["OK (t t (ran))"];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_empty_and_nil_pairs_are_noops_while_atomic_pairs_signal_exactly() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          nil)
         (list
          (add-hooks
           nil)
          (add-hooks
           '(nil))
          alpha-hook
          (mapcar
           (lambda (pairs)
             (condition-case error-data
                 (list
                  'ok
                  (add-hooks
                   pairs))
               (error
                (list
                 'error
                 (car
                  error-data)
                 (error-message-string
                  error-data)
                 (cdr
                  error-data)))))
           '((alpha)
             (17)
             ("pair")))))"##;
    let expect = expect![[
        r#"OK (nil nil nil ((error wrong-type-argument "Wrong type argument: listp, alpha" (listp alpha)) (error wrong-type-argument "Wrong type argument: listp, 17" (listp 17)) (error wrong-type-argument "Wrong type argument: listp, \"pair\"" (listp "pair"))))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}
