//! Strict combo oracle probes, batch 309: cl-prettyprint + pp + macroexpansion
//! surface. cl-prettyprint to buffer, pp-to-string, macroexpand-all, and
//! cl-source-context.
//! Uses assert_oracle_parity_expect format.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_cl_prettyprint_to_buffer_pp_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'cl-extra)
(list (with-temp-buffer
        (cl-prettyprint '(a (b c) (d (e f))) (current-buffer))
        (buffer-string))
      (condition-case err
          (pp-to-string '(1 (2 3) 4))
        (error 'pp-unavailable))
      (with-temp-buffer
        (cl-prettyprint '(lambda (x) (* x 2)) (current-buffer))
        (buffer-string)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_macroexpand_all_source_context() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'cl-extra)
(require 'macroexp)
(list (macroexpand-all '(when t (progn (push 1 x) (pop x))))
      (macroexpand-all '(cl-loop for i below 3 collect i))
      (consp (macroexp-macroexpand '(when t 'x) nil))
      (eq (macroexpand-all t) t))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_cl_setf_get_macroexpand_setf_place() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'cl-extra)
(require 'gv)
(list (gv-get '(car x) #'cons)
      (consp (macroexpand '(setf (car x) 5)))
      (macroexpand-all '(cl-incf (car x)))
      (macroexpand-all '(cl-pushnew 1 lst)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
