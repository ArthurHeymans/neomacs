//! Strong uncovered-features-56 oracle tests — org-table formulas, org-table-complex.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-table-formula-to-user
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_formula_user() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-table-formula-to-user "$1+$2")
        (org-table-formula-to-user "@1$1+@2$2")
        (org-table-formula-to-user "remote(name,$1)"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-formula-to-internal
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_formula_internal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-table-formula-to-internal "$1+$2")
        (org-table-formula-to-internal "@1$1+@2$2")
        (org-table-formula-to-internal "remote(name,$1)"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-eval-formula
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_eval_formula() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 |   |\n| 3 | 4 |   |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-eval-formula "$3=$1+$2")
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get-range
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |")
  (goto-char (point-min))
  (list (org-table-get-range "1" "2")
        (org-table-get-range "2" "3")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (list (org-table-get "1" "2")
        (org-table-get "2" "3")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-put
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (org-table-put "1" "2" "X")
  (list (org-table-get "1" "2") (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-get-elem
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_elem() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (list (org-table-get-elem 1 1)
        (org-table-get-elem 1 2)
        (org-table-get-elem 2 1)
        (org-table-get-elem 2 2)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-current-line
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-current-line))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-current-column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_col() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-current-column))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-analyze
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_analyze() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|---+---|\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (let ((a (org-table-analyze)))
    (list (nth 0 a) (nth 1 a))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-maybe-eval-formula
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 |   |\n| 3 | 4 |   |\n#+TBLFM: $3=$1+$2")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-maybe-eval-formula)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-iterate
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_iter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 |   |\n| 2 |   |\n#+TBLFM: $2=$1*2")
  (org-table-iterate)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-table-create "3x2")
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-align
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_align() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (org-table-align)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-transpose-table-at-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_transpose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (org-table-transpose-table-at-point)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-sort-lines
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| name | val |\n|---+---|\n| c | 3 |\n| a | 1 |\n| b | 2 |")
  (org-table-sort-lines nil ?a)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-to-lisp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_lisp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|---+---|\n| 1 | 2 |\n| 3 | 4 |")
  (org-table-to-lisp))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-next-field
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_next() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |")
  (goto-char (point-min))
  (forward-line 1)
  (let ((r '()))
    (push (org-table-current-column) r)
    (org-table-next-field)
    (push (org-table-current-column) r)
    (org-table-next-field)
    (push (org-table-current-column) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-previous-field
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_table_prev() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-next-field)
  (let ((r '()))
    (push (org-table-current-column) r)
    (org-table-previous-field)
    (push (org-table-current-column) r)
    (nreverse r)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-map table/table-row/table-cell
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf56_map_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|---+---|\n| 1 | 2 |\n| 3 | 4 |")
  (list (length (org-element-map (org-element-parse-buffer) 'table 'identity))
        (length (org-element-map (org-element-parse-buffer) 'table-row 'identity))
        (length (org-element-map (org-element-parse-buffer) 'table-cell 'identity))))"##,
    );
}
