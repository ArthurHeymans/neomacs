//! Complex combo batch 154 — `byte-compile-log` / `byte-compile-dest-file`
//! / `byte-recompile-directory` / `autoload-compute-prefixes` /
//! `read-feature-id`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx154_byte_compile_dest_file_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (list (fboundp 'byte-compile-dest-file)
          (fboundp 'byte-compile-dest-file-function)
          (boundp 'byte-compile-dest-file-function)
          (boundp 'byte-compile-verbose)
          (boundp 'byte-compile-depth t))
  (error (list :errored (car e))))
"##,
        expect_test::expect![[r#""OK (:errored wrong-number-of-arguments)""#]],
    );
}

#[test]
fn div_cx154_byte_compile_dest_file_extension() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let* ((el (expand-file-name "foo.el" temporary-file-directory))
           (elc (byte-compile-dest-file el)))
      (list el elc
            (stringp elc)
            (string-suffix-p ".elc" elc)))
  (error (list :errored (car e))))
"##,
        expect_test::expect![[
            r#""OK (\"/tmp/nix-shell.XcUf3d/foo.el\" \"/tmp/nix-shell.XcUf3d/foo.elc\" t t)""#
        ]],
    );
}

#[test]
fn div_cx154_byte_recompile_directory_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (fboundp 'byte-recompile-directory)
      (boundp 'byte-compile-ignore-files)
      (boundp 'byte-compile-error-on-warn))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn div_cx154_autoload_computed_prefixes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (boundp 'autoload-compute-prefixes)
      (fboundp 'make-autoload)
      (fboundp 'autoloadp)
      (fboundp 'update-file-autoloads))
"##,
        expect_test::expect![[r#""OK (nil nil t nil)""#]],
    );
}

#[test]
fn div_cx154_make_autoload_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((form '(defun neo-cx154-fn (x) "doc" (+ x 1))))
      (let ((autoload-form (make-autoload form "neo/cx154/file")))
        (list autoload-form
              (autoloadp autoload-form)
              (car autoload-form)
              (cadr autoload-form))))
  (error (list :errored (car e))))
"##,
        expect_test::expect![[r#""OK (:errored void-function)""#]],
    );
}

#[test]
fn div_cx154_loaddefs_generate_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (progn
      (require 'loaddefs-gen)
      (list (fboundp 'loaddefs-generate)
            (boundp 'loaddefs-generate-batch)))
  (error (list :errored (car e))))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx154_generated_autoload_file_var() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (boundp 'generated-autoload-file)
      (stringp generated-autoload-file)
      (boundp 'autoload-file-name))
"##,
        expect_test::expect![[r#""OK (t nil nil)""#]],
    );
}

#[test]
fn div_cx154_byte_compile_dynamic_binding_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (boundp 'byte-compile-dynamic)
      (boundp 'byte-compile-dynamic-bound-vars)
      (boundp 'byte-compile-bound-variables)
      (boundp 'byte-compile-free-assignments))
"##,
        expect_test::expect![[r#""OK (t nil t nil)""#]],
    );
}

#[test]
fn div_cx154_compiled_function_arities() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let* ((lex (let ((lexical-binding t)) (lambda (a b &optional c &rest d) (list a b c d))))
       (bc (byte-compile lex)))
  (list (func-arity lex)
        (func-arity bc)
        (aref bc 0)
        (aref bc 1)))
"##,
        expect_test::expect![[
            r#""OK ((2 . many) (2 . many) 898 \"\u{3}\u{3}\u{3}\u{3}F\\207\")""#
        ]],
    );
}

#[test]
fn div_cx154_byte_compile_lambdas_with_closures() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((lexical-binding t))
  (let* ((counter 0)
         (inc (lambda () (cl-incf counter)))
         (bc-inc (byte-compile inc)))
    (list (funcall inc)
          (funcall inc)
          (funcall bc-inc)
          counter)))
"##,
        expect_test::expect![[r#""OK (1 2 1 2)""#]],
    );
}

#[test]
fn div_cx154_disassemble_byte_code_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let* ((lex (lambda (x) (* x x)))
           (bc (byte-compile lex)))
      (let ((disassembled (disassemble bc)))
        (list (consp disassembled)
              (stringp (car disassembled))
              (consp (cadr disassembled)))))
  (error (list :errored (car e))))
"##,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

#[test]
fn div_cx154_byte_compile_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let* ((lex (let ((lexical-binding t)) (lambda (x) (* x x))))
       (bc (byte-compile lex)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "Byte-compile mega: %S" (funcall bc 7)))
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (let ((state (list (byte-code-function-p bc)
                         (func-arity bc)
                         (funcall bc 5)
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
        expect_test::expect![[r#""ERR (args-out-of-range 1 1)""#]],
    );
}
