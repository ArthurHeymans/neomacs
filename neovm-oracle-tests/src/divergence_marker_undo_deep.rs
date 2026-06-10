//! Divergence tests: deep marker and undo interaction edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_marker_insertion_type_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefgh")
  (let ((m-front (set-marker (make-marker) 4))
        (m-back (set-marker (make-marker) 4)))
    (set-marker-insertion-type m-front t)
    (set-marker-insertion-type m-back nil)
    (goto-char 4)
    (insert "XY")
    (list (marker-position m-front)
          (marker-position m-back)
          (marker-insertion-type m-front)
          (marker-insertion-type m-back))))"#,
    );
}

#[test]
fn divergence_marker_at_point_min() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abc")
  (let ((m (set-marker (make-marker) 1)))
    (goto-char 1)
    (insert "X")
    (list (marker-position m) (buffer-string))))"#,
    );
}

#[test]
fn divergence_marker_at_point_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abc")
  (let ((m (set-marker (make-marker) 4)))
    (goto-char 4)
    (insert "X")
    (list (marker-position m) (buffer-string))))"#,
    );
}

#[test]
fn divergence_copy_marker_preserves_insertion_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abc")
  (let* ((m (set-marker (make-marker) 2))
         (_ (set-marker-insertion-type m t))
         (c (copy-marker m)))
    (list (marker-position c)
          (marker-insertion-type c)
          (eq (marker-buffer m) (marker-buffer c)))))"#,
    );
}

#[test]
fn divergence_undo_after_multiple_edits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (goto-char 3)
  (insert "X")
  (goto-char 7)
  (insert "Y")
  (undo)
  (undo)
  (buffer-string))"#,
    );
}

#[test]
fn divergence_undo_yank() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (kill-region 3 7)
  (goto-char 3)
  (yank)
  (undo)
  (buffer-string))"#,
    );
}

#[test]
fn divergence_undo_after_replace_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "foo bar foo baz foo")
  (goto-char 1)
  (while (re-search-forward "foo" nil t)
    (replace-match "quux"))
  (undo)
  (buffer-string))"#,
    );
}

#[test]
fn divergence_marker_after_kill_yank() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (let ((m (set-marker (make-marker) 5)))
    (kill-region 3 7)
    (goto-char 3)
    (yank)
    (list (marker-position m) (buffer-string))))"#,
    );
}

#[test]
fn divergence_buffer_undo_list_disabled() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (buffer-enable-undo)
  (insert "abc")
  (setq buffer-undo-list t)
  (insert "def")
  (list buffer-undo-list (buffer-string)))"#,
    );
}

#[test]
fn divergence_undo_boundary_amalgamation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdef")
  (undo-boundary)
  (goto-char 3)
  (insert "X")
  (undo-boundary)
  (goto-char 3)
  (insert "Y")
  (undo)
  (buffer-string))"#,
    );
}
