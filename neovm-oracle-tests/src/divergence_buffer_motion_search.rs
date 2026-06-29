//! Divergence tests: misc buffer operations, goto-char, search edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_goto_char_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "Hello")
  (goto-char 1)
  (list (point))
  (goto-char 10)
  (list (point))
  (goto-char -1)
  (list (point)))"#,
        expect_test::expect![[r#""HelloOK (1)""#]],
    );
}

#[test]
fn divergence_forward_char_backward_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "ABCDEFGH")
  (goto-char 3)
  (forward-char 2)
  (list (point))
  (backward-char 1)
  (list (point))
  (forward-char -3)
  (list (point)))"#,
        expect_test::expect![[r#""ABCDEFGHOK (1)""#]],
    );
}

#[test]
fn divergence_forward_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "line1\nline2\nline3\nline4\nline5")
  (goto-char 1)
  (forward-line 2)
  (list (point) (line-number-at-pos))
  (forward-line -1)
  (list (point) (line-number-at-pos))
  (end-of-line)
  (list (point))
  (beginning-of-line)
  (list (point)))"#,
        expect_test::expect![[r#""line1\nline2\nline3\nline4\nline5OK (7)""#]],
    );
}

#[test]
fn divergence_search_forward_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "abcabcabc")
  (goto-char 1)
  (search-forward "abc")
  (list (point) (match-beginning 0) (match-end 0))
  (search-forward "abc")
  (list (point))
  (search-backward "abc")
  (list (point) (match-beginning 0) (match-end 0)))"#,
        expect_test::expect![[r#""abcabcabcOK (4 4 7)""#]],
    );
}

#[test]
fn divergence_search_no_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "hello world")
  (goto-char 1)
  (list (search-forward "world" nil t)
        (search-forward "xyz" nil t)
        (search-backward "hello" nil t)))"#,
        expect_test::expect![[r#""hello worldOK (12 nil 1)""#]],
    );
}

#[test]
fn divergence_re_search_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "foo123bar456baz")
  (goto-char 1)
  (re-search-forward "[0-9]+")
  (list (match-string 0) (point))
  (re-search-forward "[0-9]+")
  (list (match-string 0) (point)))"#,
        expect_test::expect![[r#""foo123bar456bazOK (\"456\" 13)""#]],
    );
}

#[test]
fn divergence_skip_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "   \t\n  hello")
  (goto-char 1)
  (skip-chars-forward " \t\n")
  (list (point))
  (skip-chars-backward "a-z")
  (list (point)))"#,
        expect_test::expect![[r#""   \t\n  helloOK (8)""#]],
    );
}

#[test]
fn divergence_thing_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "hello world 42")
  (goto-char 1)
  (list (word-at-point)
        (progn (forward-word 1) (word-at-point))
        (progn (goto-char 14) (thing-at-point 'number))))"#,
        expect_test::expect![[r#""hello world 42ERR (void-function word-at-point)""#]],
    );
}

#[test]
fn divergence_bounds_of_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "hello world 42")
  (goto-char 1)
  (list (bounds-of-thing-at-point 'word)
        (car (bounds-of-thing-at-point 'word))
        (cdr (bounds-of-thing-at-point 'word))))"#,
        expect_test::expect![[r#""hello world 42OK ((1 . 6) 1 6)""#]],
    );
}

#[test]
fn divergence_forward_word_backward_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "one two three four")
  (goto-char 1)
  (forward-word 3)
  (list (point) (word-at-point))
  (backward-word 1)
  (list (point) (word-at-point)))"#,
        expect_test::expect![[r#""one two three fourOK (9 \"three\")""#]],
    );
}
