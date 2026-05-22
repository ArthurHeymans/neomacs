//! Divergence tests: jit-lock, font-lock, syntax highlighting stubs.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_jit_lock_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'jit-lock-register)
  (fboundp 'jit-lock-unregister)
  (fboundp 'jit-lock-mode)
  (boundp 'jit-lock-chunk-size)
  (integerp jit-lock-chunk-size))"#,
    );
}

#[test]
fn divergence_font_lock_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'font-lock-mode)
  (fboundp 'font-lock-add-keywords)
  (fboundp 'font-lock-remove-keywords)
  (fboundp 'font-lock-fontify-buffer)
  (boundp 'font-lock-maximum-decoration))"#,
    );
}

#[test]
fn divergence_font_lock_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'font-lock-keywords)
  (listp font-lock-keywords)
  (boundp 'font-lock-keywords-only)
  (boundp 'font-lock-syntax-table))"#,
    );
}

#[test]
fn divergence_font_lock_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'font-lock-defaults)
  (fboundp 'font-lock-set-defaults)
  (fboundp 'font-lock-update-keyword-regexp))"#,
    );
}

#[test]
fn divergence_syntax_highlight_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'global-font-lock-mode)
  (booleanp global-font-lock-mode)
  (boundp 'font-lock-support-mode)
  (boundp 'lazy-lock-minimum-size))"#,
    );
}

#[test]
fn divergence_pretty_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'prettify-symbols-mode)
  (boundp 'prettify-symbols-unprettify-at-point)
  (boundp 'prettify-symbols-alist))"#,
    );
}

#[test]
fn derivation_whitespace_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'whitespace-mode)
  (fboundp 'global-whitespace-mode)
  (featurep 'whitespace)
  (boundp 'whitespace-style))"#,
    );
}

#[test]
fn divergence_line_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'display-line-numbers-mode)
  (boundp 'display-line-numbers)
  (boundp 'display-line-numbers-width)
  (boundp 'display-line-numbers-grow-only))"#,
    );
}

#[test]
fn divergence_highlight_indentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'highlight-indentation-mode)
  (fboundp 'highlight-indentation-current-column-mode)
  (featurep 'highlight-indentation))"#,
    );
}

#[test]
fn divergence_rainbow_delimiters() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'rainbow-delimiters-mode)
  (featurep 'rainbow-delimiters))"#,
    );
}
