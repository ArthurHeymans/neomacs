//! Complex combo batch 440 — 15 final probes: face-attribute relative,
//! buffer-local overlay marker (the 4 passers from 439 retested plus
//! new variants), face+font+frame pass combo, ex-439 passers:
//! posn+display+invisible, face-attribute+font+frame,
//! buffer-local+overlay+marker, plus new edge combos.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// Re-test passers from 439 with small variations.
#[test]
fn div_cx440_face_font_frame_pass() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'bold :weight nil 'default)
      (face-attribute 'italic :slant nil 'default)
      (face-font 'default))"##,
    );
}

/// buffer-local + overlay + marker: state tracking (passed before).
#[test]
fn div_cx440_buffer_local_overlay_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abcdef")
  (let ((o (make-overlay 2 4))) (overlay-put o 'face 'bold))
  (let ((m (set-marker (make-marker) 3))
        (v (make-local-variable 'neo-cx440-v)))
    (setq neo-cx440-v 'val)
    (list (marker-position m)
          (length (overlays-in 1 10)))))"##,
    );
}

/// posn-at-point + display + visible text (passed before).
#[test]
fn div_cx440_posn_display_visible() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello world")
  (put-text-property 3 4 'display "XX")
  (condition-case e (posn-at-point 3) (error (car e))))"##,
    );
}

/// bidi-string-mark-left-to-right with multibyte.
#[test]
fn div_cx440_bidi_string_mark_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'bidi-string)
  (string-mark-left-to-right "abcالعربية123"))"##,
    );
}

/// char-bytes / char-width with edge Unicode ranges.
#[test]
fn div_cx440_char_bytes_width_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (char-bytes ?a) (char-bytes ?é) (char-bytes ?世) (char-bytes #x1F600)
      (char-width ?a) (char-width ?é) (char-width ?世) (char-width #x1F600))"##,
    );
}

/// assoc + assq + rassoc + rassq with symbol/string keys.
#[test]
fn div_cx440_assoc_assq_rassoc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((al '((a . 1) (b . 2) (c . 3)))
      (sl '(("a" . 1) ("b" . 2))))
  (list (assq 'a al) (assoc "a" sl) (rassq 1 al) (rassoc 1 sl)))"##,
    );
}

/// length+ + safe-length + proper-list-p on various types.
#[test]
fn div_cx440_length_safe_proper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (length '(a b c))
      (safe-length '(a b c))
      (safe-length '(a . b))
      (proper-list-p '(a b c))
      (proper-list-p '(a . b)))"##,
    );
}

/// delete + delq + remove + remq with eq and equal.
#[test]
fn div_cx440_delete_delq_remove_remq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((l '(a b c b a)))
  (list (delq 'b (copy-sequence l))
        (delete 'b (copy-sequence l))
        (remq 'b (copy-sequence l))
        (remove 'b (copy-sequence l))))"##,
    );
}

/// keymap lookup with multiple inheritance chain.
#[test]
fn div_cx440_keymap_inherit_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((k1 (make-sparse-keymap))
      (k2 (make-sparse-keymap))
      (k3 (make-sparse-keymap)))
  (define-key k1 "a" 'fn1)
  (define-key k2 "b" 'fn2)
  (define-key k3 "c" 'fn3)
  (set-keymap-parent k2 k1)
  (set-keymap-parent k3 k2)
  (list (key-binding "a" nil nil k3)
        (key-binding "b" nil nil k3)
        (key-binding "c" nil nil k3)))"##,
    );
}

/// syntax-table: copy + modify + set.
#[test]
fn div_cx440_syntax_table_copy_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((st (copy-syntax-table (syntax-table))))
  (modify-syntax-entry ?_ "w" st)
  (with-temp-buffer
    (set-syntax-table st)
    (insert "foo_bar baz")
    (goto-char 1)
    (forward-word)
    (point)))"##,
    );
}

/// memory-info / memory-use-counts basic.
#[test]
fn div_cx440_memory_info_counts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (listp (memory-info))
      (listp (memory-use-counts))
      (listp (memory-limit)))"##,
    );
}

/// window-state-get with parameters and buffer.
#[test]
fn div_cx440_window_state_with_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "state")
  (let ((state (window-state-get (selected-window) t)))
    (list (listp state)
          (> (length state) 0))))"##,
    );
}

/// prin1 with print-escape-control-characters.
#[test]
fn div_cx440_prin1_escape_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((print-escape-control-characters t))
  (prin1-to-string "hello\n\tworld\r\0"))"##,
    );
}

/// format with %S on vectors and records.
#[test]
fn div_cx440_format_S_vectors_records() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (format "%S" [1 2 3])
      (format "%S" (record 'test 1 2))
      (format "%S" [:key :value]))"##,
    );
}

/// abbrev-expansion / abbrev-symbol with mixed case.
#[test]
fn div_cx440_abbrev_mixed_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((tab (make-abbrev-table)))
  (define-abbrev tab "teh" "the")
  (define-abbrev tab "dont" "don't" nil 1)
  (list (abbrev-expansion "teh" tab)
        (abbrev-expansion "DONT" tab)))"##,
    );
}
