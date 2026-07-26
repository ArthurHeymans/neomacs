use expect_test::expect;

use super::{assert_dash_parity, assert_dash_signal_parity};

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_thread_first_last_and_placeholder_macros_place_values_exactly() {
    let elisp_form = r##"(list
              (-> 5 1+ (* 2))
              (->> '(1 2 3) (mapcar #'1+) (apply #'+))
              (--> 5 (+ it 1) (* it 2))
              (-as-> 5 value (+ value 1) (* value value))
              (-> 'value)
              (->> 'value)
              (--> 'value)
              (-as-> 'value item (list item)))"##;
    let expect = expect!["OK (12 9 12 36 value value value (value))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_some_threading_short_circuits_nil_and_evaluates_the_source_once() {
    let elisp_form = r##"(list
              (-some-> 5 1+ (* 2))
              (-some-> nil 1+ (* 2))
              (-some->> '(1 2) (mapcar #'1+) (apply #'+))
              (-some->> nil (mapcar #'1+) (apply #'+))
              (-some--> 5 (+ it 1) (* it 2))
              (-some--> nil (1+ it) (* it 2))
              (let ((evaluations 0)
                    (steps 0))
                (list
                 (-some->
                     (progn (setq evaluations (1+ evaluations)) nil)
                   (progn (setq steps (1+ steps)) 1+))
                 evaluations
                 steps))
              (let ((evaluations 0))
                (list
                 (-some->
                     (progn (setq evaluations (1+ evaluations)) 5)
                   1+)
                 evaluations)))"##;
    let expect = expect!["OK (12 nil 5 nil 12 nil (nil 1 0) (6 1))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_doto_variants_return_the_original_object_after_side_effects() {
    let elisp_form = r##"(list
              (let ((value (list 1)))
                (list
                 (-doto value
                   (setcdr '(2 3))
                   (nreverse))
                 value))
              (let ((value (list 1)))
                (list
                 (--doto value
                   (setcdr it '(2 3))
                   (nreverse it))
                 value))
              (-doto 'value)
              (--doto 'value))"##;
    let expect = expect!["OK ((#1=(1) #1#) (#2=(1) #2#) value value)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_let_destructures_lists_vectors_and_rest_patterns() {
    let elisp_form = r##"(list
              (-let (((a b . rest) '(1 2 3 4)))
                (list a b rest))
              (-let* (((a b) '(1 2))
                       ((c d) (list b a)))
                (list a b c d))
              (-let [[a b &rest rest] [1 2 3 4]]
                (list a b rest))
              (-let (((a &optional b c) '(1)))
                (list a b c))
              (-let (((a &as whole b) '(1 2 3)))
                (list a b whole)))"##;
    let expect = expect!["OK ((1 2 (3 4)) (1 2 2 1) (1 2 [3 4]) (1 nil nil) ((1 2 3) 2 1))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_let_destructures_plists_alists_and_hash_tables() {
    let elisp_form = r##"(list
              (-let (((&plist :name name :age age)
                      '(:name ada :age 36)))
                (list name age))
              (-let (((&alist 'name name 'age age)
                      '((name . ada) (age . 36))))
                (list name age))
              (let ((table (make-hash-table :test 'eq)))
                (puthash 'name 'ada table)
                (-let (((&hash 'name name 'missing missing) table))
                  (list name missing)))
              (-let (((&plist :missing missing) nil))
                missing))"##;
    let expect = expect!["OK ((ada 36) (ada 36) (ada nil) nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_lambda_and_setq_apply_destructuring_at_call_and_assignment_time() {
    let elisp_form = r##"(list
              (mapcar
               (-lambda ((key . value)) (list value key))
               '((a . 1) (b . 2)))
              (funcall (-lambda ((a b)) (+ a b)) '(2 3))
              (let (a b)
                (-setq (a b) '(1 2))
                (list a b))
              (let (a rest)
                (-setq (a . rest) '(1 2 3))
                (list a rest)))"##;
    let expect = expect!["OK (((1 a) (2 b)) 5 (1 2) (1 (2 3)))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_if_let_variants_select_branches_and_bind_destructured_values() {
    let elisp_form = r##"(list
              (-if-let (value 3) (* value 2) 'missing)
              (-if-let ((a b) '(1 2)) (+ a b) 'missing)
              (-if-let* ((a 2) (b (+ a 3))) (* a b) 'missing)
              (--if-let 4 (* it 2) 'missing)
              (-if-let (value nil) value 'else)
              (-if-let* ((a 1) (b nil)) (+ a b) 'else)
              (let ((evaluations 0))
                (list
                 (-if-let
                     (value
                      (progn (setq evaluations (1+ evaluations)) 3))
                     value
                   'missing)
                 evaluations)))"##;
    let expect = expect!["OK (6 3 10 8 else else (3 1))"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_when_let_variants_run_only_for_successful_bindings() {
    let elisp_form = r##"(list
              (-when-let (value 3) (* value 2))
              (-when-let* ((a 2) (b (+ a 3))) (* a b))
              (--when-let 4 (* it 2))
              (-when-let (value nil) value)
              (-when-let* ((a 1) (b nil)) (+ a b))
              (--when-let nil it))"##;
    let expect = expect!["OK (6 10 8 nil nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_fontification_modes_register_info_and_unload_cleanly() {
    let elisp_form = r##"(list
              (with-temp-buffer
                (dash-fontify-mode 1)
                (prog1 dash-fontify-mode
                  (dash-fontify-mode -1)))
              (with-temp-buffer
                (dash-enable-font-lock 1)
                (prog1 dash-fontify-mode
                  (dash-enable-font-lock -1)))
              (progn
                (global-dash-fontify-mode 1)
                (prog1 global-dash-fontify-mode
                  (global-dash-fontify-mode -1)))
              (progn (dash-register-info-lookup) t)
              (dash-unload-function))"##;
    let expect = expect!["OK (t nil t t nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_public_configuration_variables_have_the_expected_defaults() {
    let elisp_form = r##"(list
              -compare-fn
              -fixfn-max-iterations
              dash-fontify-mode-lighter
              dash-enable-fontlock)"##;
    let expect = expect!["OK (nil 1000 nil nil)"];

    assert_dash_parity(elisp_form, expect);
}

#[test]
#[ignore = "live MELPA parity corpus: downloads pinned Dash below ./tmp"]
fn dash_short_vector_destructuring_signals_exact_type_data() {
    let elisp_form = r##"(-let ([a b c] [1 2]) (list a b c))"##;
    let expect = expect!["ERR (wrong-type-argument arrayp nil)"];

    assert_dash_signal_parity(elisp_form, expect);
}
