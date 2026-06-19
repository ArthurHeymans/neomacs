/// Batch 517: elisp internals bootstrap, load history, byte-code, native-comp.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx517_bootstrap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'bootstrap-version) (featurep 'bootstrap))
"##,
    );
}

#[test]
fn div_cx517_load_history_paths() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lh load-history))
  (list (listp lh) (> (length lh) 0) (consp (car lh))))
"##,
    );
}

#[test]
fn div_cx517_load_suffixes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list load-file-rep-suffixes load-suffixes)
"##,
    );
}

#[test]
fn div_cx517_source_etc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'source-directory) (stringp source-directory))
"##,
    );
}

#[test]
fn div_cx517_byte_compiler() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (fboundp 'byte-compile) (fboundp 'byte-optimize-form))
"##,
    );
}

#[test]
fn div_cx517_byte_code_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (byte-compile (lambda (x) (* x 2)))))
  (list (type-of f) (byte-code-function-p f)))
"##,
    );
}

#[test]
fn div_cx517_compiled_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (fboundp 'compiled-function-p)
      (fboundp 'interpreted-function-p))
"##,
    );
}

#[test]
fn div_cx517_native_comp_avail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (native-comp-available-p)
      (fboundp 'native-comp-unit-file))
"##,
    );
}

#[test]
fn div_cx517_pure_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'pure-space-used) (numberp pure-space-used))
"##,
    );
}

#[test]
fn div_cx517_garbage_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((g (garbage-collect)))
  (list (listp g) (> (length g) 0)))
"##,
    );
}

#[test]
fn div_cx517_memory_report() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'memory-full))
"##,
    );
}

#[test]
fn div_cx517_emacs_lisp_native() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (fboundp 'emacs-lisp-native-compile)
      (fboundp 'emacs-lisp-compilation-mode))
"##,
    );
}

#[test]
fn div_cx517_disassemble_bc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (byte-compile (lambda () 42))))
  (with-temp-buffer
    (disassemble f (current-buffer))
    (> (buffer-size) 0)))
"##,
    );
}

#[test]
fn div_cx517_byte_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(fboundp 'message)
"##,
    );
}

#[test]
fn div_cx517_message_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'format-spec)
  (list (fboundp 'format-spec) (fboundp 'format-spec-make)))
"##,
    );
}
