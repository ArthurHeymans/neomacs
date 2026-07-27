use expect_test::expect;

use super::assert_anaphora_parity;

#[test]
fn anaphora_addition_ports_all_upstream_arity_and_previous_value_cases() {
    let elisp_form = r##"(list
         (a+)
         (a+ 2)
         (a+ 2 3 4)
         (a+ 2 3 4 it)
         (a+ 2 3 4 it 2)
         (a+ 1.5 2 3.25 it)
         (condition-case error
             (a+ it)
           (error
            (list
             (car error)
             (cadr error)))))"##;
    let expect = expect!["OK (0 2 9 13 15 10.0 (void-variable it))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_subtraction_ports_all_upstream_arity_and_previous_value_cases() {
    let elisp_form = r##"(list
         (a-)
         (a- 2)
         (a- 20 3 4)
         (a- 20 3 4 it)
         (a- 20 3 4 it 2)
         (a- 100.0 12.5 7.5 it)
         (condition-case error
             (a- it)
           (error
            (list
             (car error)
             (cadr error)))))"##;
    let expect = expect!["OK (0 -2 13 9 7 72.5 (void-variable it))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_multiplication_ports_all_upstream_arity_and_previous_value_cases() {
    let elisp_form = r##"(list
         (a*)
         (a* 2)
         (a* 2 3 4)
         (a* 2 3 4 it)
         (a* 2 3 4 it 2)
         (a* 1.5 2 3 it)
         (condition-case error
             (a* it)
           (error
            (list
             (car error)
             (cadr error)))))"##;
    let expect = expect!["OK (1 2 24 96 192 27.0 (void-variable it))"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_division_ports_all_upstream_arities_previous_values_and_errors() {
    let elisp_form = r##"(list
         (a/ 200 5)
         (a/ 200 5 2)
         (a/ 200 5 2 it)
         (a/ 200 5 2 it 5)
         (a/ 81.0 3 3 it)
         (condition-case error
             (eval '(a/))
           (error
            (list
             (car error)
             (cdr error))))
         (condition-case error
             (eval '(a/ 200))
           (error
            (list
             (car error)
             (cdr error))))
         (condition-case error
             (a/ 200 it)
           (error
            (list
             (car error)
             (cadr error))))
         (condition-case error
             (a/ 10 0)
           (error
            (list
             (car error)
             (cdr error)))))"##;
    let expect = expect![[
        r#"OK (40 20 10 2 3.0 (wrong-number-of-arguments (#1=#[(dividend divisor &rest divisors) ((cond ((null divisors) (list '/ dividend divisor)) (t (list 'let (list (list 'it divisor)) (list '/ dividend (list '* 'it (cons 'anaphoric-* divisors))))))) (t) nil "Like `/', but the result of evaluating the previous divisor is bound to `it'.\n\nThe variable `it' is available within all expressions after the\nfirst divisor.\n\nDIVIDEND, DIVISOR, and DIVISORS are otherwise as documented for `/'."] 0)) (wrong-number-of-arguments (#1# 1)) (void-variable it) (arith-error nil))"#
    ]];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_arithmetic_evaluates_operands_once_left_to_right_with_rebinding() {
    let elisp_form = r##"(let (events)
         (cl-labels
             ((record
               (label value)
               (push label events)
               value))
           (list
            (a+
             (record :first 2)
             (record :second
                     (+ it 3))
             (record :third
                     (* it 2)))
            (nreverse events)
            (progn
              (setq events nil)
              (a*
               (record :one 2)
               (record :two
                       (1+ it))
               (record :three
                       (+ it 2))))
            (nreverse events)
            (progn
              (setq events nil)
              (a/
               (record :dividend 240)
               (record :divisor 4)
               (record :next
                       (+ it 1))
               (record :last
                       (1- it))))
            (nreverse events))))"##;
    let expect = expect![
        "OK (17 (:first :second :third) 30 (:one :two :three) 3 (:divisor :dividend :next :last))"
    ];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_arithmetic_handles_real_buffer_markers_like_builtin_operators() {
    let elisp_form = r##"(with-temp-buffer
         (insert "0123456789")
         (let ((left
                (copy-marker 3))
               (right
                (copy-marker 8)))
           (unwind-protect
               (list
                (a+ left 4)
                (a- right 3)
                (a-
                 (marker-position right)
                 (marker-position left))
                (marker-position left)
                (marker-position right))
             (set-marker left nil)
             (set-marker right nil))))"##;
    let expect = expect!["OK (7 5 5 3 8)"];
    assert_anaphora_parity(elisp_form, expect);
}

#[test]
fn anaphora_long_and_short_arithmetic_names_match_on_complex_expressions() {
    let elisp_form = r##"(list
         (equal
          (a+ 2 3 it 5)
          (anaphoric-+ 2 3 it 5))
         (equal
          (a- 50 4 it 2)
          (anaphoric-- 50 4 it 2))
         (equal
          (a* 2 3 it 5)
          (anaphoric-* 2 3 it 5))
         (equal
          (a/ 720 6 3 it 2)
          (anaphoric-/ 720 6 3 it 2))
         (list
          (anaphoric-+)
          (anaphoric-- 9)
          (anaphoric-*)
          (anaphoric-/ 12 3)))"##;
    let expect = expect!["OK (t t t t (0 -9 1 4))"];
    assert_anaphora_parity(elisp_form, expect);
}
