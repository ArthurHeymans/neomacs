//! Divergence tests: char-tables, category tables, syntax tables deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_make_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ct (make-char-table 'syntax-table nil)))
  (list (char-table-p ct)
        (char-table-type ct)
        (aref ct ?a)))"#,
    );
}

#[test]
fn divergence_char_table_set_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ct (make-char-table 'syntax-table nil)))
  (aset ct ?a 'word)
  (aset ct ?A 'word)
  (aset ct ?0 'digit)
  (list (aref ct ?a)
        (aref ct ?A)
        (aref ct ?0)
        (aref ct ?z)
        (aref ct ? )))"#,
    );
}

#[test]
fn divergence_char_table_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ct (make-char-table 'syntax-table nil)))
  (set-char-table-range ct '(?A . ?Z) 'word)
  (list (aref ct ?A)
        (aref ct ?M)
        (aref ct ?Z)
        (aref ct ?a)
        (char-table-range ct '(?A . ?Z))))"#,
    );
}

#[test]
fn divergence_syntax_table_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((st (standard-syntax-table)))
  (list (char-syntax ?a)
        (char-syntax ? )
        (char-syntax ?()
        (char-syntax ?))
        (char-syntax ?\")
        (char-syntax ?\\)))"#,
    );
}

#[test]
fn divergence_syntax_class_codes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((st (copy-syntax-table (standard-syntax-table))))
  (modify-syntax-entry ?$ "_" st)
  (with-syntax-table st
    (list (char-syntax ?$)
          (char-syntax ?a))))"#,
    );
}

#[test]
fn divergence_modify_syntax_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (modify-syntax-entry ?@ "w" (standard-syntax-table))
  (list (char-syntax ?@)
        (string (char-syntax ?@))))"#,
    );
}

#[test]
fn divergence_category_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ct (standard-category-table)))
  (list (char-table-p ct)
        (category-table-p ct)
        (aref ct ?a)
        (category-set-mnemonics (aref ct ?a))))"#,
    );
}

#[test]
fn divergence_syntax_forward_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "hello world foo")
  (goto-char 1)
  (forward-word 1)
  (list (point))
  (forward-word 1)
  (list (point))
  (forward-word -1)
  (list (point)))"#,
    );
}

#[test]
fn divergence_parse_partial_sexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "(foo (bar \"baz\") quux)")
  (list
    (nth 0 (parse-partial-sexp 1 20))
    (nth 3 (parse-partial-sexp 1 20))))"#,
    );
}
