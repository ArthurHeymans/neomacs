/// Batch 515: function/macro/alias combined deep characterization.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx515_function_cell_mutation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (make-symbol "cx515-fcm")))
  (fset s (lambda (x) (* x 2)))
  (list (fboundp s) (funcall s 5)))
"##,
        expect_test::expect![[r#""OK (t 10)""#]],
    );
}

#[test]
fn div_cx515_defalias_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((a (make-symbol "cx515-a"))
      (b (make-symbol "cx515-b")))
  (defalias a 'car)
  (defalias b a)
  (eq (indirect-function b) (symbol-function 'car)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx515_function_get_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (make-symbol "cx515-fgp")))
  (defalias s 'cdr)
  (function-put s 'test-attr 'test-val)
  (function-get s 'test-attr))
"##,
        expect_test::expect![[r#""OK test-val""#]],
    );
}

#[test]
fn div_cx515_interactive_specs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f1 (lambda (x) (interactive "p") x))
      (f2 (lambda () (interactive) 42)))
  (list (commandp f1) (commandp f2) (interactive-form f1)))
"##,
        expect_test::expect![[r#""OK (t t (interactive \"p\"))""#]],
    );
}

#[test]
fn div_cx515_command_modes_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (lambda () (interactive))))
  (put 'cx515-cmd 'command-modes '(text-mode))
  (defalias 'cx515-cmd f)
  (command-modes 'cx515-cmd))
"##,
        expect_test::expect![[r#""OK (text-mode)""#]],
    );
}

#[test]
fn div_cx515_subr_arity_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(mapcar #'subr-arity '(car cdr cons + - * / concat list vector))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument subrp car)""#]],
    );
}

#[test]
fn div_cx515_subr_name_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(mapcar #'subr-name '(car cdr cons +))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument subrp car)""#]],
    );
}

#[test]
fn div_cx515_help_function_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (help-function-arglist 'car)
      (help-function-arglist 'concat)
      (help-function-arglist 'if))
"##,
        expect_test::expect![[r#""OK ((arg1) (&rest rest) (arg1 arg2 &rest rest))""#]],
    );
}

#[test]
fn div_cx515_documentation_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((doc (documentation 'car)))
  (and (stringp doc) (> (length doc) 0)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx515_indirect_function_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (let ((a (make-symbol "cx515-ifa"))
          (b (make-symbol "cx515-ifb")))
      (defalias a b)
      (defalias b a)
      (indirect-function a))
  (error (car e)))
"##,
        expect_test::expect![[r#""OK cyclic-function-indirection""#]],
    );
}

#[test]
fn div_cx515_called_interactively() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (lambda ()
             (interactive)
             (called-interactively-p 'any))))
  (list (commandp f)))
"##,
        expect_test::expect![[r#""OK (t)""#]],
    );
}

#[test]
fn div_cx515_function_alias_p_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (make-symbol "cx515-fap")))
  (defalias s 'forward-char)
  (fboundp s))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx515_autoload_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'autoload)
  (list (boundp 'autoload-modified-buffers) (fboundp 'autoload-rubric)))
"##,
        expect_test::expect![[r#""OK (nil t)""#]],
    );
}

#[test]
fn div_cx515_byte_code_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (byte-compile (lambda (x) (* x 3)))))
  (list (byte-code-function-p f) (funcall f 7)))
"##,
        expect_test::expect![[r#""OK (t 21)""#]],
    );
}

#[test]
fn div_cx515_closurep_interp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (lambda (x) (+ x 1))))
  (list (functionp f) (closurep f) (subrp f)))
"##,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}
