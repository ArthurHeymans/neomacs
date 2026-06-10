//! Divergence tests: motion, goto, forward/backward, end-of deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_line_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Line1\nLine2\nLine3\nLine4\n")
  (goto-char (point-min))
  (forward-line 2)
  (list (point) (buffer-substring (line-beginning-position) (line-end-position)))
  (forward-line -1)
  (list (point) (buffer-substring (line-beginning-position) (line-end-position)))) "#,
    );
}

#[test]
fn divergence_beginning_end_of_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World")
  (goto-char 5)
  (list (point)
        (progn (end-of-line) (point))
        (progn (beginning-of-line) (point)))) "#,
    );
}

#[test]
fn divergence_buffer_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World")
  (list (point-min) (point-max)
        (buffer-size)
        (= (point-min) 1)
        (= (point-max) (1+ (buffer-size))))) "#,
    );
}

#[test]
fn divergence_word_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "hello world foo bar")
  (goto-char 1)
  (forward-word 1)
  (let ((p1 (point)))
    (forward-word 1)
    (let ((p2 (point)))
      (backward-word 1)
      (list p1 p2 (point))))) "#,
    );
}

#[test]
fn divergence_char_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Hello World")
  (goto-char 1)
  (forward-char 5)
  (let ((p1 (point)))
    (backward-char 3)
    (list p1 (point)
          (char-after (point))
          (char-before (point))))) "#,
    );
}

#[test]
fn divergence_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "Line1\nLine2\nLine3\nLine4\n")
  (goto-char 1)
  (list (line-number-at-pos)
        (progn (forward-line 2) (line-number-at-pos))
        (progn (forward-line 1) (line-number-at-pos))
        (count-lines (point-min) (point-max)))) "#,
    );
}

#[test]
fn divergence_skip_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "   hello   world   ")
  (goto-char 1)
  (skip-chars-forward " ")
  (let ((p1 (point)))
    (skip-chars-forward "a-z")
    (let ((p2 (point)))
      (skip-chars-forward " ")
      (list p1 p2 (point))))) "#,
    );
}

#[test]
fn divergence_skip_chars_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "   hello   world   ")
  (goto-char (point-max))
  (skip-chars-backward " ")
  (let ((p1 (point)))
    (skip-chars-backward "a-z")
    (list p1 (point)))) "#,
    );
}

#[test]
fn divergence_search_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "foo bar foo bar foo")
  (goto-char 1)
  (list (search-forward "bar" nil t)
        (search-forward "bar" nil t)
        (search-forward "bar" nil t)
        (search-forward "bar" nil t))) "#,
    );
}

#[test]
fn divergence_count_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "a\nb\nc\nd\ne\n")
  (list (count-lines (point-min) (point-max))
        (= (count-lines (point-min) (point-max)) 5)
        (fboundp 'what-line))) "#,
    );
}
