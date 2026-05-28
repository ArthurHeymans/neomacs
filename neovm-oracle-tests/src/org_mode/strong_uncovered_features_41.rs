//! Strong uncovered-features-41 oracle tests — org-table, org-timer, org-duration.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-table-create
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-table-create "3x2")
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-create-with-table-width
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_create_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-table-create-with-table-width 3)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-justify
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_justify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-justify)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-align
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_align() {
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
// org-table-insert-row
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_insert_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-insert-row)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-insert-column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_insert_col() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (org-table-insert-column)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-delete-row
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_delete_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-delete-row)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-delete-column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_delete_col() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |")
  (org-table-goto-column 2)
  (org-table-delete-column)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-move-row
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_move_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n| 3 | 4 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-move-row 'down)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-move-column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_move_col() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |")
  (org-table-goto-column 2)
  (org-table-move-column 'right)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-sort-lines
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_sort() {
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
// org-table-transpose-table-at-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_transpose() {
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
// org-table-blank-field
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_blank() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (forward-char 2)
  (org-table-blank-field)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-toggle-formula-debugger
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_debug() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (org-table-toggle-formula-debugger)
  (org-table-toggle-formula-debugger)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-edit-field
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-edit-field nil)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-next-field
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_next() {
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
fn uf41_table_prev() {
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
// org-table-copy-down
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_copy_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |\n|   |   |")
  (goto-char (point-min))
  (forward-line 1)
  (forward-char 2)
  (org-table-copy-down 1)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-wrap-region
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_table_wrap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| long text here | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-wrap-region 10)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timer-start
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_timer_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (condition-case nil
      (org-timer-start)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timer-set-timer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_timer_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (condition-case nil
      (org-timer-set-timer 5)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-duration-to-minutes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_duration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-duration-to-minutes "1:30")
        (org-duration-to-minutes "2h30min")
        (org-duration-to-minutes "1d 2h")
        (org-duration-to-minutes "90min"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-duration-from-minutes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_duration_from() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-duration-from-minutes 90)
        (org-duration-from-minutes 150)
        (org-duration-from-minutes 1500))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-duration-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf41_duration_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-duration-p "1:30")
        (org-duration-p "2h30min")
        (org-duration-p "invalid")
        (org-duration-p "90min"))"##,
    );
}
