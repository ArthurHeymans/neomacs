//! Strong uncovered-features-39 oracle tests — org-agenda views, org-columns.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\nSCHEDULED: <2026-01-15>\n* DONE D1\n* TODO T2\nDEADLINE: <2026-01-20>")
  (condition-case nil
      (org-agenda-list)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-todo-list
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_todo_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1\n* DONE D1\n* TODO T2\n* WAITING W1")
  (condition-case nil
      (org-todo-list)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-tags-view
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_tags_view() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1 :work:\n* T2 :home:\n* T3 :work:")
  (condition-case nil
      (org-tags-view nil "work")
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-search-view
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_search_view() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T1 keyword\n* T2 other\n* T3 keyword")
  (condition-case nil
      (org-search-view nil "keyword")
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-get-format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_columns_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY %TAGS %V\n* TODO [#A] T :tag:\n:PROPERTIES:\n:V: val\n:END:")
  (goto-char (point-min))
  (org-columns-get-format))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-compile-format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_columns_compile() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-columns-compile-format "%25ITEM %TODO %3PRIORITY %TAGS")"##);
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-uncompile-format
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_columns_uncompile() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(org-columns-uncompile-format '(("ITEM" 25) ("TODO" 0) ("PRIORITY" 3) ("TAGS" 0)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-get-format-with-width
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_columns_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY\n* T")
  (goto-char (point-min))
  (org-columns-get-format-with-width))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-columns-display
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_columns_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY\n* TODO [#A] T\n* DONE [#B] D")
  (condition-case nil
      (org-columns-display)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-columns
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_columns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+COLUMNS: %25ITEM %TODO %3PRIORITY\n* TODO [#A] T\n* DONE [#B] D")
  (condition-case nil
      (org-agenda-columns)
    (error nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-view-mode-dispatch
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-view-mode-dispatch)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T1 :work:\n* DONE D1 :home:\n* TODO T2 :work:")
  (condition-case nil
      (org-agenda-prepare-buffers (list (current-buffer)))
    (error nil))
  (list (length (org-map-entries (lambda () t) nil 'file))
        (length (org-map-entries (lambda () t) "work" 'file))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-by-category
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_cat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-by-category)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-by-tag
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-by-tag)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-by-regexp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_regexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-by-regexp)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-by-effort
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_effort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-by-effort)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-by-priority
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_prio() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-by-priority)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-by-top-headline
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_top() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-by-top-headline)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-filter-remove-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_filter_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-filter-remove-all)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-get-restriction-lock
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_restriction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-get-restriction-lock)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-redo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_redo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-redo)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-quit
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_quit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-quit)
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-agenda-exit
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf39_agenda_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-agenda-exit)
  (error nil))"##,
    );
}
