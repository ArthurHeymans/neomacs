//! Divergence tests: bookmark, register, kmacro edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_bookmark_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'bookmark-set)
  (fboundp 'bookmark-jump)
  (fboundp 'bookmark-bmenu-list)
  (featurep 'bookmark))"#,
    );
}

#[test]
fn divergence_bookmark_save() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'bookmark-save)
  (fboundp 'bookmark-load)
  (boundp 'bookmark-default-file)
  (stringp bookmark-default-file)) "#,
    );
}

#[test]
fn divergence_register_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'point-to-register)
  (fboundp 'jump-to-register)
  (fboundp 'insert-register)
  (fboundp 'view-register)
  (fboundp 'register-alist-defaults)
  (boundp 'register-alist)
  (listp register-alist)) "#,
    );
}

#[test]
fn divergence_register_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'register-describe-oneline)
  (fboundp 'get-register)
  (fboundp 'set-register)) "#,
    );
}

#[test]
fn divergence_abbrev_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'define-abbrev)
  (fboundp 'abbrev-expand)
  (fboundp 'write-abbrev-file)
  (fboundp 'read-abbrev-file)
  (featurep 'abbrev)) "#,
    );
}

#[test]
fn divergence_abbrev_tables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-abbrev-table)
  (fboundp 'clear-abbrev-table)
  (fboundp 'define-abbrev-table)
  (boundp 'global-abbrev-table)
  (listp global-abbrev-table)) "#,
    );
}

#[test]
fn divergence_auto_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'auto-fill-mode)
  (boundp 'auto-fill-function)
  (boundp 'normal-auto-fill-function)
  (boundp 'fill-prefix)
  (stringp fill-prefix)) "#,
    );
}

#[test]
fn divergence_paragraph_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'forward-paragraph)
  (fboundp 'backward-paragraph)
  (fboundp 'mark-paragraph)
  (boundp 'paragraph-start)
  (boundp 'paragraph-separate)) "#,
    );
}

#[test]
fn divergence_page_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'forward-page)
  (fboundp 'backward-page)
  (fboundp 'count-lines-page)
  (boundp 'page-delimiter)
  (stringp page-delimiter)) "#,
    );
}

#[test]
fn divergence_sentence_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'forward-sentence)
  (fboundp 'backward-sentence)
  (boundp 'sentence-end)
  (boundp 'sentence-end-double-space)
  (booleanp sentence-end-double-space)) "#,
    );
}
