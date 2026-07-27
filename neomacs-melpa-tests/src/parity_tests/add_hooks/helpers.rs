use expect_test::expect;

use super::assert_add_hooks_parity;

#[test]
fn add_hooks_listify_covers_nil_identity_scalars_sequences_symbols_and_functions() {
    let elisp_form = r##"(let* ((proper
                  (list
                   'a
                   'b))
                 (lambda-value
                  (lambda ()
                    'ran))
                 (dotted
                  '(a . b)))
         (list
          (add-hooks-listify
           nil)
          (let ((result
                 (add-hooks-listify
                  proper)))
            (list
             (eq
              proper
              result)
             result))
          (add-hooks-listify
           dotted)
          (add-hooks-listify
           'plain-symbol)
          (add-hooks-listify
           #'car)
          (add-hooks-listify
           lambda-value)
          (add-hooks-listify
           42)
          (add-hooks-listify
           "text")
          (add-hooks-listify
           [a b])
          (add-hooks-listify
           :keyword)))"##;
    let expect = expect![[
        r#"OK (nil (t (a b)) (a . b) (plain-symbol) (car) (#[nil ('ran) (t)]) (42) ("text") ([a b]) (:keyword))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_listify_distinguishes_function_forms_from_ordinary_nested_lists() {
    let elisp_form = r##"(let* ((lambda-form
                  (list
                   'lambda
                   nil
                   (list
                    'quote
                    'value)))
                 (ordinary
                  (list
                   'not-a-function
                   'a
                   'b))
                 (nested
                  (list
                   lambda-form))
                 (lambda-result
                  (add-hooks-listify
                   lambda-form))
                 (ordinary-result
                  (add-hooks-listify
                   ordinary))
                 (nested-result
                  (add-hooks-listify
                   nested)))
         (list
          (functionp
           lambda-form)
          (length
           lambda-result)
          (eq
           (car
            lambda-result)
           lambda-form)
          (functionp
           ordinary)
          (eq
           ordinary-result
           ordinary)
          (functionp
           nested)
          (eq
           nested-result
           nested)
          (functionp
           (car
            nested-result))))"##;
    let expect = expect!["OK (t 1 t nil t nil t t)"];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_normalize_hook_covers_suffix_case_keywords_nil_and_uninterned_symbols() {
    let elisp_form = r##"(let ((uninterned
                (make-symbol
                 "private")))
         (list
          (add-hooks-normalize-hook
           'alpha)
          (add-hooks-normalize-hook
           'alpha-hook)
          (let ((case-fold-search
                 t))
            (add-hooks-normalize-hook
             'ALPHA-HOOK))
          (let ((case-fold-search
                 nil))
            (add-hooks-normalize-hook
             'ALPHA-HOOK))
          (add-hooks-normalize-hook
           nil)
          (add-hooks-normalize-hook
           :keyword)
          (let ((result
                 (add-hooks-normalize-hook
                  uninterned)))
            (list
             (symbol-name
              result)
             (intern-soft
              (symbol-name
               result))
             (eq
              result
              uninterned)))))"##;
    let expect = expect![[
        r#"OK (alpha-hook alpha-hook ALPHA-HOOK ALPHA-HOOK-hook nil-hook :keyword-hook ("private-hook" private-hook nil))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_normalize_hook_returns_every_non_symbol_object_by_identity() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (let ((result
                  (add-hooks-normalize-hook
                   value)))
             (list
              (eq
               value
               result)
              result)))
         (list
          "alpha"
          "alpha-hook"
          '(alpha)
          [alpha]
          17
          (lambda ()
            'alpha)))"##;
    let expect = expect![[
        r#"OK ((t "alpha") (t "alpha-hook") (t (alpha)) (t [alpha]) (t 17) (t #[nil ('alpha) (t)]))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}
