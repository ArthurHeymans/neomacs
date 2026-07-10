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
    let expect = expect_test::expect![[r#""ERR (wrong-number-of-arguments (1 . 1) 2)""#]];
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
    let expect = expect_test::expect![[
        r#""OK ((if t (progn (progn (setq x (cons 1 x)) (car-safe (prog1 x (setq x (cdr x))))))) (let* ((i 0) (--cl-var-- nil)) (while (< i 3) (setq --cl-var-- (cons i --cl-var--)) (setq i (+ i 1))) (nreverse --cl-var--)) t t)""#
    ]];
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
    let expect = expect_test::expect![[
        r#""OK ((let* ((v x)) ((car v) . #[257 \"\\302\\301\u{2}\\300#\\207\" [(v) #[385 \"\\300\\301\u{2}\u{4}C\\\"B\\207\" [setcar append] 6 (\"/home/exec/Projects/github.com/eval-exec/neomacs/lisp/emacs-lisp/gv.elc\" . 10031)] apply] 5 (\"/home/exec/Projects/github.com/eval-exec/neomacs/lisp/emacs-lisp/gv.elc\" . 546)])) t (let* ((v x)) (setcar v (+ (car v) 1))) (if (memql 1 lst) (with-no-warnings lst) (setq lst (cons 1 lst))))""#
    ]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
