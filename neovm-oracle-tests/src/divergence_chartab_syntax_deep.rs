//! Divergence tests: char-tables, category tables, syntax tables deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_make_char_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((ct (make-char-table 'syntax-table nil)))
  (list (char-table-p ct)
        (char-table-type ct)
        (aref ct ?a)))"#,
        expect_test::expect![[r#""ERR (void-function char-table-type)""#]],
    );
}

#[test]
fn divergence_char_table_set_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((ct (make-char-table 'syntax-table nil)))
  (aset ct ?a 'word)
  (aset ct ?A 'word)
  (aset ct ?0 'digit)
  (list (aref ct ?a)
        (aref ct ?A)
        (aref ct ?0)
        (aref ct ?z)
        (aref ct ? )))"#,
        expect_test::expect![[r#""OK (word word digit nil nil)""#]],
    );
}

#[test]
fn divergence_char_table_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((ct (make-char-table 'syntax-table nil)))
  (set-char-table-range ct '(?A . ?Z) 'word)
  (list (aref ct ?A)
        (aref ct ?M)
        (aref ct ?Z)
        (aref ct ?a)
        (char-table-range ct '(?A . ?Z))))"#,
        expect_test::expect![[r#""OK (word word word nil word)""#]],
    );
}

#[test]
fn divergence_syntax_table_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((st (standard-syntax-table)))
  (list (char-syntax ?a)
        (char-syntax ? )
        (char-syntax ?()
        (char-syntax ?))
        (char-syntax ?\")
        (char-syntax ?\\)))"#,
        expect_test::expect![[r#""OK (119 32 40 41 34 92)""#]],
    );
}

#[test]
fn divergence_syntax_class_codes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((st (copy-syntax-table (standard-syntax-table))))
  (modify-syntax-entry ?$ "_" st)
  (with-syntax-table st
    (list (char-syntax ?$)
          (char-syntax ?a))))"#,
        expect_test::expect![[r#""OK (95 119)""#]],
    );
}

#[test]
fn divergence_modify_syntax_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (modify-syntax-entry ?@ "w" (standard-syntax-table))
  (list (char-syntax ?@)
        (string (char-syntax ?@))))"#,
        expect_test::expect![[r#""OK (119 \"w\")""#]],
    );
}

#[test]
fn divergence_category_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((ct (standard-category-table)))
  (list (char-table-p ct)
        (category-table-p ct)
        (aref ct ?a)
        (category-set-mnemonics (aref ct ?a))))"#,
        expect_test::expect![[
            r#""OK (t t #&128\"\\0\\0\\0\\0\\0@\\0\\0\\0\u{10}\\0\\0\u{2}\u{10}\u{4}\\0\" \".Lalr\")""#
        ]],
    );
}

#[test]
fn divergence_syntax_forward_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "hello world foo")
  (goto-char 1)
  (forward-word 1)
  (list (point))
  (forward-word 1)
  (list (point))
  (forward-word -1)
  (list (point)))"#,
        expect_test::expect![[r#""hello world fooOK (7)""#]],
    );
}

#[test]
fn divergence_parse_partial_sexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "(foo (bar \"baz\") quux)")
  (list
    (nth 0 (parse-partial-sexp 1 20))
    (nth 3 (parse-partial-sexp 1 20))))"#,
        expect_test::expect![[r#""(foo (bar \"baz\") quux)OK (1 nil)""#]],
    );
}
