/// Batch 458: module/load/d-load/symbol/obarray edge probes.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx458_symbol_name_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (symbol-name 'forward-char)
      (symbol-function 'forward-char)
      (symbol-plist 'forward-char)
      (symbol-value 'emacs-major-version))"##,
        expect_test::expect![[r#""OK (\"forward-char\" #<subr forward-char> nil 31)""#]],
    );
}

#[test]
fn div_cx458_symbol_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (make-symbol "neo-cx458-s")))
  (put s 'prop1 'val1)
  (put s 'prop2 'val2)
  (list (symbol-plist s) (get s 'prop1) (get s 'prop2)))"##,
        expect_test::expect![[r#""OK ((prop1 val1 prop2 val2) val1 val2)""#]],
    );
}

#[test]
fn div_cx458_obarray_intern_unintern() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (intern "test-symbol" obs)
  (list (internp (intern "test-symbol" obs))
        (unintern "test-symbol" obs)
        (internp (intern "test-symbol" obs))))"##,
        expect_test::expect![[r#""ERR (void-function internp)""#]],
    );
}

#[test]
fn div_cx458_obarray_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (obarray-default)))
  (list (vectorp obs) (> (length obs) 0)))"##,
        expect_test::expect![[r#""ERR (void-function obarray-default)""#]],
    );
}

#[test]
fn div_cx458_mapatoms_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((count 0))
  (mapatoms (lambda (s) (when (string-prefix-p "forward-" (symbol-name s)) (setq count (1+ count)))))
  count)"##,
        expect_test::expect![[r#""OK 26""#]],
    );
}

#[test]
fn div_cx458_obarray_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (obarray-size (obarray-default))
  (error (car e)))"##,
        expect_test::expect![[r#""OK void-function""#]],
    );
}

#[test]
fn div_cx458_load_no_autoload() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (make-temp-file "neo-cx458-load-" nil ".el" "(setq neo-cx458-loaded t)")))
  (load f nil t t)
  neo-cx458-loaded)"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx458_autoload_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn
  (defvar neo-cx458-autoloaded nil)
  (autoload 'neo-cx458-autofn "neo-cx458-fake" "test autoload" t)
  (list (autoloadp (symbol-function 'neo-cx458-autofn))
        (condition-case e (autoload-do-load (symbol-function 'neo-cx458-autofn)) (error (car e)))))"##,
        expect_test::expect![[r#""OK (t file-missing)""#]],
    );
}

#[test]
fn div_cx458_byte_code_object() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (byte-compile (lambda (x) (+ x 1)))))
  (list (byte-code-function-p f)
        (condition-case e (funcall f 5) (error (car e)))))"##,
        expect_test::expect![[r#""OK (t 6)""#]],
    );
}

#[test]
fn div_cx458_compiled_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f1 (lambda (x) (* x 2)))
      (f2 (byte-compile (lambda (x) (* x 2)))))
  (list (compiled-function-p f1)
        (compiled-function-p f2)))"##,
        expect_test::expect![[r#""OK (nil t)""#]],
    );
}

#[test]
fn div_cx458_indirect_function_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((a (make-symbol "neo-cx458-a"))
      (b (make-symbol "neo-cx458-b")))
  (defalias a 'forward-char)
  (defalias b a)
  (list (fboundp b)
        (eq (indirect-function b) (symbol-function 'forward-char))))"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx458_defalias_fset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (make-symbol "neo-cx458-fset")))
  (fset s (lambda (x) (* x 3)))
  (list (fboundp s) (funcall s 7)))"##,
        expect_test::expect![[r#""OK (t 21)""#]],
    );
}

#[test]
fn div_cx458_function_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((a (lambda (x) (* x 2)))
      (b (lambda (x) (* x 2))))
  (list (function-equal a a) (function-equal a b)))"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx458_byte_code_pure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (byte-compile (lambda () 42))))
  (list (byte-code-function-p f)
        (funcall f)))"##,
        expect_test::expect![[r#""OK (t 42)""#]],
    );
}

#[test]
fn div_cx458_apply_partially_varargs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((f (apply-partially #'+ 10 20)))
  (funcall f 30))"##,
        expect_test::expect![[r#""OK 60""#]],
    );
}
