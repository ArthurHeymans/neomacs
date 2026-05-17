//! Oracle parity for regex submatches and macroexpand via binary.
//! GNU src/search.c, src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm_via_binary};

// --- string-match with submatch groups ---

#[test]
fn oracle_string_match_with_groups_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(
        r#"(progn
  (string-match "\\([a-z]+\\) \\([0-9]+\\)" "hello 42")
  (list (match-string 0 "hello 42")
        (match-string 1 "hello 42")
        (match-string 2 "hello 42")))"#,
    );
    assert_ok_eq("(\"hello 42\" \"hello\" \"42\")", &o, &n);
}

// --- macroexpand on defmacro ---

#[test]
fn oracle_macroexpand_defmacro_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(
        r#"(progn
  (defmacro nvm--double (x) (list '* x 2))
  (macroexpand '(nvm--double 5)))"#,
    );
    assert_ok_eq("(* 5 2)", &o, &n);
}

// --- macroexpand on core macros ---

#[test]
fn oracle_macroexpand_when_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(macroexpand '(when t 42))"#);
    // when expands to (if t (progn 42))
    assert_ok_eq("(if t (progn 42))", &o, &n);
}

#[test]
fn oracle_macroexpand_unless_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(r#"(macroexpand '(unless nil 42))"#);
    // unless expands to (if nil nil (progn 42)) — but check real behavior
    assert_ok_eq("(if nil nil 42)", &o, &n);
}

// --- replace-match with backreference ---

#[test]
fn oracle_replace_match_backref_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_via_binary(
        r#"(progn
  (set-buffer (get-buffer-create "*rmb*"))
  (erase-buffer)
  (insert "hello world")
  (goto-char 1)
  (re-search-forward "\\([a-z]+\\) \\([a-z]+\\)" nil t)
  (replace-match "\\2 \\1")
  (buffer-string))"#,
    );
    assert_ok_eq("\"world hello\"", &o, &n);
}
