//! Oracle parity for looking-at, pos-bol, pos-eol, line-end-position edges.
//! GNU src/search.c, src/editfns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_looking_at_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*la3*")) (erase-buffer) (insert "hello") (goto-char 1) (looking-at "hel"))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_looking_at_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*la4*")) (erase-buffer) (insert "hello") (goto-char 1) (looking-at "xyz"))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_looking_at_with_regex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*la5*")) (erase-buffer) (insert "hello123") (goto-char 1) (looking-at "[a-z]+"))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_pos_bol_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*pbm*")) (erase-buffer) (insert "abc\ndef\nghi") (goto-char 7) (pos-bol))"#,
        expect_test::expect![[r#""OK 5""#]],
    );
    assert_ok_eq("5", &o, &n);
}

#[test]
fn oracle_pos_eol_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*pem*")) (erase-buffer) (insert "abc\ndef\nghi") (goto-char 6) (pos-eol))"#,
        expect_test::expect![[r#""OK 8""#]],
    );
    assert_ok_eq("8", &o, &n);
}

#[test]
fn oracle_line_end_position_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*lepm*")) (erase-buffer) (insert "abc\ndef\nghi") (goto-char 1) (line-end-position))"#,
        expect_test::expect![[r#""OK 4""#]],
    );
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_bolp_after_hard_newline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*bhn*")) (erase-buffer) (insert "abc\ndef") (goto-char 5) (bolp))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}
