//! Strong uncovered-features-28 oracle tests — org-timer, org-learn, org-macros.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-timer-start/timer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_timer_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\n:END:")
  (goto-char (point-min))
  (condition-case nil
      (org-timer-start)
    (error nil))
  (org-timer-item)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timer-set-timer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_timer_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (condition-case nil
      (org-timer-set-timer 5)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timer-pause-or-continue
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_timer_pause() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (condition-case nil
      (progn (org-timer-start) (org-timer-pause-or-continue))
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-timer-stop
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_timer_stop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-min))
  (condition-case nil
      (progn (org-timer-start) (org-timer-stop))
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-replace-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello $1!\n{{{greeting(World)}}} and {{{greeting(Elisp)}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-accumulate-arguments
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_macro_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-macro-accumulate-arguments "{{{macro(a,b,c)}}}" 0)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro-expand-macro
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_macro_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello $1!\n{{{greeting(World)}}}")
  (let ((org-macro-templates (org-macro--collect-macros)))
    (org-macro-expand-macro "{{{greeting(World)}}}" org-macro-templates)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-macro--collect-macros
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_macro_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: a 1\n#+MACRO: b 2\n{{{a}}} {{{b}}}")
  (org-macro--collect-macros))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-learn
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_learn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15 +1d>")
  (goto-char (point-min))
  (condition-case nil
      (org-learn nil 5)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-learn-get-entries
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_learn_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15 +1d>\n* U\nSCHEDULED: <2026-01-20 +1w>")
  (condition-case nil
      (org-learn-get-entries)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-duration-to-minutes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_duration() {
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
fn uf28_duration_from() {
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
fn uf28_duration_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-duration-p "1:30")
        (org-duration-p "2h30min")
        (org-duration-p "invalid")
        (org-duration-p "90min"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-active-p
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_cache_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-active-p))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-flush
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_cache_flush() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-flush (point-min))
  (let ((s (org-element-cache-status)))
    (list (plist-get s :size))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-element-cache-sync
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_cache_sync() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H\nBody")
  (org-element-cache-sync)
  (let ((s (org-element-cache-status)))
    (list (plist-get s :size))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-blank-field
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_blank() {
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
// org-table-insert-row/column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n| 1 | 2 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-insert-row)
  (org-table-insert-column)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-delete-row/column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-delete-row)
  (org-table-goto-column 2)
  (org-table-delete-column)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-move-row/column
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |")
  (goto-char (point-min))
  (forward-line 1)
  (org-table-move-row-down)
  (org-table-goto-column 2)
  (org-table-move-column-right)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-sort-lines
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| name | val |\n|---+---|\n| c | 3 |\n| a | 1 |\n| b | 2 |")
  (goto-char (point-min))
  (org-table-sort-lines nil ?a)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-table-transpose-table-at-point
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_transpose() {
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
// org-table-toggle-formula-debugger
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf28_table_debug() {
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
fn uf28_table_edit() {
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
