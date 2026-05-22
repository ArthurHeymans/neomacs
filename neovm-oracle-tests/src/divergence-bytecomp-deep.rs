//! Divergence tests: byte-compile, byte-optimize, disassemble deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_byte_compile_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'byte-compile-file)
  (fboundp 'byte-compile-buffer)
  (fboundp 'byte-compile-from-buffer)
  (fboundp 'byte-compile-function-form)
  (featurep 'bytecomp))"#,
    );
}

#[test]
fn divergence_byte_optimize() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'byte-optimize)
  (boundp 'byte-compile-optimize)
  (fboundp 'byte-optimize-form)
  (fboundp 'byte-optimize-body))"#,
    );
}

#[test]
fn divergence_disassemble() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'disassemble)
  (fboundp 'byte-decompile-bytecode)
  (fboundp 'byte-code)
  (fboundp 'fetch-bytecode))"#,
    );
}

#[test]
fn divergence_byte_compile_warn() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'byte-compile-warnings)
  (boundp 'byte-compile-error-on-warn)
  (fboundp 'byte-compile-warn)
  (fboundp 'byte-compile-log-warning))"#,
    );
}

#[test]
fn divergence_byte_compile_macroexpand() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'macroexpand)
  (fboundp 'macroexpand-all)
  (fboundp 'internal-macroexpand-for-bytecomp)
  (fboundp 'byte-compile-macroexpand))"#,
    );
}

#[test]
fn divergence_byte_compile_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'byte-compile-inline-expand)
  (boundp 'byte-compile-inline-max-size)
  (fboundp 'defsubst)
  (fboundp 'define-inline))"#,
    );
}

#[test]
fn divergence_compiled_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'compiled-function-p)
  (compiled-function-p (symbol-function 'car))
  (compiled-function-p (symbol-function 'list))
  (compiled-function-p (lambda (x) x))) "#,
    );
}

#[test]
fn divergence_byte_compile_dynamic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'byte-compile-dynamic)
  (boundp 'byte-compile-dynamic-docstrings)
  (boundp 'byte-compile-delete-errors)
  (fboundp 'byte-compile-dest-file))"#,
    );
}

#[test]
fn divergence_byte_stack_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'internal--byte-code-meter)
  (boundp 'byte-code-meter)
  (listp byte-code-meter)
  (boundp 'byte-metering-enabled)) "#,
    );
}

#[test]
fn divergence_byte_out_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'byte-compile-output)
  (fboundp 'byte-compile-output-docform)
  (fboundp 'byte-compile-keep-pending))"#,
    );
}
