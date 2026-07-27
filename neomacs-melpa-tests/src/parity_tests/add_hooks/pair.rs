use expect_test::expect;

use super::assert_add_hooks_parity;

#[test]
fn add_hooks_pair_calls_add_hook_for_exact_normalized_cartesian_product_and_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'add-hook)
               (lambda (&rest arguments)
                 (push
                  arguments
                  calls)
                 'fixture-result)))
           (list
            (add-hooks-pair
             '(alpha beta-hook)
             '(first second))
            (nreverse
             calls))))"##;
    let expect = expect![
        "OK (nil ((alpha-hook first) (alpha-hook second) (beta-hook first) (beta-hook second)))"
    ];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_pair_real_hook_state_preserves_prepend_order_existing_values_and_deduplicates() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          '(existing)
          beta-hook
          nil)
         (list
          (add-hooks-pair
           '(alpha beta)
           '(first second first))
          (copy-sequence
           alpha-hook)
          (copy-sequence
           beta-hook)
          (progn
            (add-hooks-pair
             'alpha-hook
             'existing)
            (copy-sequence
             alpha-hook))
          (progn
            (add-hooks-pair
             '(alpha alpha-hook)
             'third)
            (copy-sequence
             alpha-hook))))"##;
    let expect = expect![
        "OK (nil (second first existing) (second first) (second first existing) (third second first existing))"
    ];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_pair_lambda_and_named_functions_execute_on_every_target_hook() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          nil
          beta-hook
          nil)
         (let (events)
         (cl-labels
             ((named
               ()
               (push
                'named
                events)))
           (let ((anonymous
                  (lambda ()
                    (push
                     'anonymous
                     events))))
             (add-hooks-pair
              '(alpha beta)
              (list
               #'named
               anonymous))
             (run-hooks
              'alpha-hook)
             (let ((after-alpha
                    (reverse
                     events)))
               (setq
                events
                nil)
               (run-hooks
                'beta-hook)
               (list
                after-alpha
                (reverse
                 events)
                (length
                 alpha-hook)
                (length
                 beta-hook)))))))"##;
    let expect = expect!["OK ((anonymous named) (anonymous named) 2 2)"];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_pair_nil_axes_are_noops_invalid_hooks_signal_and_arbitrary_functions_are_stored() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          nil)
         (list
          (add-hooks-pair
           nil
           'first)
          alpha-hook
          (add-hooks-pair
           'alpha
           nil)
          alpha-hook
          (mapcar
           (lambda (arguments)
             (condition-case error-data
                 (list
                  'ok
                  (apply
                   #'add-hooks-pair
                   arguments))
               (error
                (list
                 'error
                 (car
                  error-data)
                 (error-message-string
                  error-data)
                 (cdr
                 error-data)))))
           '(("not-a-symbol" first)
             (17 first)
             (alpha 17)))
          alpha-hook))"##;
    let expect = expect![[
        r#"OK (nil nil nil nil ((error wrong-type-argument "Wrong type argument: symbolp, \"not-a-symbol\"" (symbolp "not-a-symbol")) (error wrong-type-argument "Wrong type argument: symbolp, 17" (symbolp 17)) (ok nil)) (17))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_pair_accepts_function_form_as_one_function_instead_of_destructuring_it() {
    let elisp_form = r##"(progn
         (setq
          alpha-hook
          nil)
         (let (events)
         (add-hooks-pair
          'alpha
          (lambda ()
            (push
             'ran
             events)))
         (let ((stored
                alpha-hook))
           (run-hooks
            'alpha-hook)
           (list
            (length
             stored)
            (functionp
             (car
              stored))
            events))))"##;
    let expect = expect!["OK (1 t (ran))"];
    assert_add_hooks_parity(elisp_form, expect);
}
