//! Divergence tests: jit-lock, font-lock, syntax highlighting stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_jit_lock_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'jit-lock-register)
  (fboundp 'jit-lock-unregister)
  (fboundp 'jit-lock-mode)
  (boundp 'jit-lock-chunk-size)
  (integerp jit-lock-chunk-size))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_font_lock_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'font-lock-mode)
  (fboundp 'font-lock-add-keywords)
  (fboundp 'font-lock-remove-keywords)
  (fboundp 'font-lock-fontify-buffer)
  (boundp 'font-lock-maximum-decoration))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_font_lock_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'font-lock-keywords)
  (listp font-lock-keywords)
  (boundp 'font-lock-keywords-only)
  (boundp 'font-lock-syntax-table))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_font_lock_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'font-lock-defaults)
  (fboundp 'font-lock-set-defaults)
  (fboundp 'font-lock-update-keyword-regexp))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_syntax_highlight_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'global-font-lock-mode)
  (booleanp global-font-lock-mode)
  (boundp 'font-lock-support-mode)
  (boundp 'lazy-lock-minimum-size))"#,
        expect_test::expect![[r#""OK (t t t nil)""#]],
    );
}

#[test]
fn divergence_pretty_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'prettify-symbols-mode)
  (boundp 'prettify-symbols-unprettify-at-point)
  (boundp 'prettify-symbols-alist))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn derivation_whitespace_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'whitespace-mode)
  (fboundp 'global-whitespace-mode)
  (featurep 'whitespace)
  (boundp 'whitespace-style))"#,
        expect_test::expect![[r#""OK (t t nil nil)""#]],
    );
}

#[test]
fn divergence_line_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'display-line-numbers-mode)
  (boundp 'display-line-numbers)
  (boundp 'display-line-numbers-width)
  (boundp 'display-line-numbers-grow-only))"#,
        expect_test::expect![[r#""OK (t t t nil)""#]],
    );
}

#[test]
fn divergence_highlight_indentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'highlight-indentation-mode)
  (fboundp 'highlight-indentation-current-column-mode)
  (featurep 'highlight-indentation))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

#[test]
fn divergence_rainbow_delimiters() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'rainbow-delimiters-mode)
  (featurep 'rainbow-delimiters))"#,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}
