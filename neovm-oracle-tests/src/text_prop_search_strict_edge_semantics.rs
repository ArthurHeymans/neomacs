//! Oracle parity for text-property search + similar areas.
//! GNU src/textprop.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_next_property_change_finds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*npc2*")) (erase-buffer) (insert "abcdef") (put-text-property 2 5 'x 'y) (next-property-change 1))"#,
        expect_test::expect![[r#""OK 2""#]],
    );
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_next_property_change_none() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*npcn*")) (erase-buffer) (insert "abcdef") (list (next-property-change 1) (next-property-change 1 nil t)))"#,
        expect_test::expect![[r#""OK (nil 7)""#]],
    );
    assert_ok_eq("(nil 7)", &o, &n);
}

#[test]
fn oracle_text_property_any_none() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*tpa2*")) (erase-buffer) (insert "abcdef") (text-property-any 1 4 'x nil))"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_text_property_any_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*tpaf*")) (erase-buffer) (insert "abcdef") (put-text-property 2 4 'x 'y) (text-property-any 1 5 'x nil))"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_text_property_any_finds_explicit_value_after_nil_gap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*tpafy*")) (erase-buffer) (insert "abcdef") (put-text-property 2 4 'x 'y) (text-property-any 1 5 'x 'y))"#,
        expect_test::expect![[r#""OK 2""#]],
    );
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_text_property_not_all_respects_nil_gap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*tpna*")) (erase-buffer) (insert "abcdef") (put-text-property 2 4 'x 'y) (list (text-property-not-all 1 5 'x nil) (text-property-not-all 1 5 'x 'y)))"#,
        expect_test::expect![[r#""OK (2 1)""#]],
    );
    assert_ok_eq("(2 1)", &o, &n);
}

#[test]
fn oracle_get_char_property_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*gcp*")) (erase-buffer) (insert "abcdef") (get-char-property 3 'x))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_get_char_property_with_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*gcp2*")) (erase-buffer) (insert "abcdef") (put-text-property 2 4 'x 'found) (get-char-property 3 'x))"#,
        expect_test::expect![[r#""OK found""#]],
    );
    assert_ok_eq("found", &o, &n);
}

#[test]
fn oracle_put_text_property_multiple_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*mtp2*")) (erase-buffer) (insert "abcdef") (put-text-property 1 4 'a 1) (put-text-property 1 4 'b 2) (list (get-text-property 2 'a) (get-text-property 2 'b)))"#,
        expect_test::expect![[r#""OK (1 2)""#]],
    );
    assert_ok_eq("(1 2)", &o, &n);
}

#[test]
fn oracle_previous_char_property_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*ppc*")) (erase-buffer) (insert "abcdef") (put-text-property 2 5 'x 'y) (goto-char 6) (previous-property-change (point)))"#,
        expect_test::expect![[r#""OK 5""#]],
    );
    assert_ok_eq("5", &o, &n);
}
