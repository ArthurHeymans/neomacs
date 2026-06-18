//! Column/indentation/line-motion parity: current-column with tabs and wide
//! (CJK) chars, move-to-column (+force), indent-to / indent-line-to /
//! current-indentation / back-to-indentation, forward-line return value,
//! beginning/end-of-line with arg + line-beginning/end-position, char-after/
//! before/following/preceding at boundaries, count-lines edges.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn back_to_indentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "   \t  text here")
  (back-to-indentation)
  (list (point) (current-column)))"##,
    );
}

#[test]
fn beginning_end_line_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "line1\nline2\nline3\n")
  (goto-char 1)
  (list (progn (end-of-line 2) (point))
        (progn (beginning-of-line 1) (point))
        (line-beginning-position 3) (line-end-position 2)))"##,
    );
}

#[test]
fn char_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc")
  (list (progn (goto-char 1) (list (char-after) (char-before) (following-char) (preceding-char)))
        (progn (goto-char (point-max)) (list (char-after) (char-before)))))"##,
    );
}

#[test]
fn count_lines_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "a\nb\nc")
  (list (count-lines (point-min) (point-max))
        (progn (insert "\n") (count-lines (point-min) (point-max)))
        (count-lines 1 1)))"##,
    );
}

#[test]
fn current_column_tabs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (setq tab-width 8)
  (insert "a\tb\tc")
  (goto-char (point-max))
  (list (current-column)
        (progn (goto-char 3) (current-column))
        (progn (goto-char 2) (current-column))))"##,
    );
}

#[test]
fn current_column_wide() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "日本x")
  (goto-char (point-max))
  (list (current-column) (progn (goto-char 2) (current-column))
        (progn (goto-char 3) (current-column))))"##,
    );
}

#[test]
fn forward_line_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "a\nb\nc")
  (goto-char (point-min))
  (list (forward-line 2) (line-number-at-pos)
        (forward-line 5) (line-number-at-pos)))"##,
    );
}

#[test]
fn indent_line_to() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "    hello")
  (goto-char (point-min))
  (indent-line-to 8)
  (list (current-indentation) (buffer-string)))"##,
    );
}

#[test]
fn indent_to_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "x")
  (indent-to 10)
  (list (current-column) (buffer-string)))"##,
    );
}

#[test]
fn move_to_column_force() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (setq tab-width 8)
  (insert "ab\tcd")
  (goto-char (point-min))
  (list (move-to-column 5) (current-column)
        (progn (goto-char (point-min)) (move-to-column 20 t)) (current-column)))"##,
    );
}
