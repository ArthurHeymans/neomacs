//! Strong uncovered-features-54 oracle tests — org-macs utilities, org-faces, org-check-external.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-trim
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_trim() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-trim "  hello  ")
        (org-trim "\nhello\n")
        (org-trim "  \n hello \n  "))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-string-width
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_string_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-string-width "hello")
        (org-string-width "hello world")
        (org-string-width ""))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-remove-indentation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_remove_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-remove-indentation "  hello\n  world")
        (org-remove-indentation "hello\nworld")
        (org-remove-indentation "    hello\n      world"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-do-remove-indentation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_do_remove_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "  hello\n  world")
  (org-do-remove-indentation)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-number-sequence
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_number_seq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-number-sequence 1 5)
        (org-number-sequence 1 10 2)
        (org-number-sequence 5 1 -1))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-not-nil
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_not_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-not-nil "test")
        (org-not-nil nil)
        (org-not-nil "")
        (org-not-nil 0))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-not-empty
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_not_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-not-empty "test")
        (org-not-empty "")
        (org-not-empty nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-unescape-string
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_unescape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-unescape-string "hello\\nworld")
        (org-unescape-string "hello\\tworld")
        (org-unescape-string "hello\\\\world"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-replace-escapes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_replace_escapes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (org-replace-escapes "hello\\nworld")
        (org-replace-escapes "hello\\tworld")
        (org-replace-escapes "hello\\\\world"))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-faces level
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_faces_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'org-level-1 :foreground nil t)
        (face-attribute 'org-level-2 :foreground nil t)
        (face-attribute 'org-level-3 :foreground nil t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-faces todo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_faces_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'org-todo :foreground nil t)
        (face-attribute 'org-done :foreground nil t)
        (face-attribute 'org-priority :foreground nil t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-faces table
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_faces_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'org-table :foreground nil t)
        (face-attribute 'org-table-row :foreground nil t)
        (face-attribute 'org-formula :foreground nil t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-faces link
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_faces_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'org-link :foreground nil t)
        (face-attribute 'org-meta-line :foreground nil t)
        (face-attribute 'org-document-info :foreground nil t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-faces block
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_faces_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'org-block :foreground nil t)
        (face-attribute 'org-verbatim :foreground nil t)
        (face-attribute 'org-code :foreground nil t))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-check-external-command
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_check_external() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-check-external-command "ls" "test")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-open-file
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_open_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-open-file "/tmp/test.org")
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-switch-to-buffer-other-window
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_switch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-switch-to-buffer-other-window (current-buffer))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-pop-to-buffer-same-window
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_pop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case nil
    (org-pop-to-buffer-same-window (current-buffer))
  (error nil))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-escape-code-in-region
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello\nworld\n")
  (org-escape-code-in-region (point-min) (point-max))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-unescape-code-in-region
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf54_unescape_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello\n,world\n")
  (org-unescape-code-in-region (point-min) (point-max))
  (buffer-string))"##,
    );
}
