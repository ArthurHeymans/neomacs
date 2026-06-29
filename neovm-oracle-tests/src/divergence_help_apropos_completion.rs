//! Divergence tests: apropos, help, info, man stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_apropos_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'apropos-command)
  (fboundp 'apropos-variable)
  (fboundp 'apropos-documentation)
  (fboundp 'apropos-library))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_help_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'describe-function)
  (fboundp 'describe-variable)
  (fboundp 'describe-key)
  (fboundp 'describe-mode)
  (fboundp 'describe-char))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_info_functions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'info-lookup-symbol)
  (fboundp 'info-display-manual)
  (featurep 'info))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_man_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'man)
  (fboundp 'woman)
  (featurep 'man))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_elisp_index() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'elisp-index-search)
  (fboundp 'emacs-index-search))"#,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn divergence_completion_styles() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (listp completion-styles)
  (member 'basic completion-styles)
  (fboundp 'completion-styles-alist))"#,
        expect_test::expect![[r#""OK (t (basic partial-completion emacs22) nil)""#]],
    );
}

#[test]
fn divergence_completion_categories() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'completion-category-defaults)
  (fboundp 'completion-category-overrides)
  (boundp 'completion-category-defaults))"#,
        expect_test::expect![[r#""OK (nil nil t)""#]],
    );
}

#[test]
fn divergence_minibuffer_completion_auto() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'minibuffer-complete)
  (fboundp 'minibuffer-complete-word)
  (fboundp 'minibuffer-complete-and-exit))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_corfu_company() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'company-mode)
  (fboundp 'corfu-mode)
  (featurep 'company)
  (featurep 'corfu))"#,
        expect_test::expect![[r#""OK (nil nil nil nil)""#]],
    );
}

#[test]
fn divergence_which_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'which-key-mode)
  (featurep 'which-key))"#,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}
