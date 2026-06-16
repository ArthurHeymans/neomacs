//! Complex combo batch 296 — `byte-compile` deep: compile lambdas with
//! closures, `disassemble` output, `func-arity` of compiled vs
//! interpreted, `compiled-function-p` predicates, `byte-compile-warnings`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx296_byte_compile_lambda_with_closure_capture() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lexical-binding t))
  (let ((counter 0))
    (let* ((inc (lambda () (cl-incf counter)))
           (bc-inc (byte-compile inc)))
      (list (funcall inc)
            (funcall inc)
            (funcall bc-inc)
            (byte-code-function-p bc-inc)
            counter))))
"##,
    )
}

#[test]
fn div_cx296_byte_compile_with_rest_and_optional() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lex (lambda (a &optional b &rest c) (list a b c)))
       (bc (byte-compile lex)))
  (list (func-arity lex)
        (func-arity bc)
        (funcall bc 1)
        (funcall bc 1 2)
        (funcall bc 1 2 3 4 5)))
"##,
    )
}

#[test]
fn div_cx296_disassemble_compiled_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lex (lambda (x) (* x x)))
       (bc (byte-compile lex)))
  (condition-case e
      (let ((disassembled (disassemble bc)))
        (list (consp disassembled)
              (stringp (car disassembled))))
    (error (list :err (car e)))))
"##,
    )
}

#[test]
fn div_cx296_compiled_function_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lex (lambda (x) x))
       (bc (byte-compile lex)))
  (list (compiled-function-p bc)
        (compiled-function-p lex)
        (byte-code-function-p bc)
        (byte-code-function-p lex)
        (closurep bc)
        (closurep lex)))
"##,
    )
}

#[test]
fn div_cx296_byte_compile_constant_folding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lex (lambda () (+ 1 2 3)))
       (bc (byte-compile lex)))
  (list (byte-code-function-p bc)
        (funcall bc)))
"##,
    )
}

#[test]
fn div_cx296_byte_compile_warning_categories_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((categories '(free-vars unresolved callargs redefine obsolete
                    noruntime cl-functions interactive make-local)))
  (mapcar (lambda (cat)
            (list cat (memq cat byte-compile-warnings)))
          categories))
"##,
    )
}

#[test]
fn div_cx296_byte_compile_dest_file_function_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((el (expand-file-name "neo-cx296-test.el" temporary-file-directory))
       (elc (byte-compile-dest-file el)))
  (list (stringp elc)
        (string-suffix-p ".elc" elc)))
"##,
    )
}

#[test]
fn div_cx296_native_comp_availability_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'native-compile)
          (fboundp 'native-compile-async)
          (boundp 'native-comp-jit-compilation)
          (boundp 'native-comp-driver-options))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx296_byte_optimize_with_mapcar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lex (lambda (lst) (mapcar (lambda (x) (* x 2)) lst)))
       (bc (byte-compile lex)))
  (list (byte-code-function-p bc)
        (funcall bc '(1 2 3 4 5))))
"##,
    )
}

#[test]
fn div_cx296_byte_compile_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lex (let ((lexical-binding t)) (lambda (x) (* x x))))
       (bc (byte-compile lex)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "Byte-compile mega: %d" (funcall bc 7)))
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((state (list (byte-code-function-p bc)
                         (func-arity bc)
                         (funcall bc 5)
                         (compiled-function-p bc)
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    )
}
