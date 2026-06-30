//! Strict combo oracle probes, batch 144: window prev/next buffers after
//! bury+kill, marker tracking through complex replace-match sequence,
//! with-temp-message + message-log-max, and indirect buffer undo
//! independence.
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_window_prev_next_bury_kill_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let ((a (get-buffer-create " *probe-pnb-a*"))
      (b (get-buffer-create " *probe-pnb-b*"))
      (c (get-buffer-create " *probe-pnb-c*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer a)
        (switch-to-buffer b)
        (switch-to-buffer c)
        (let ((prev1 (mapcar #'buffer-name (window-prev-buffers))))
          (bury-buffer b)
          (let ((prev2 (mapcar #'buffer-name (window-prev-buffers))))
            (kill-buffer b)
            (let ((prev3 (mapcar (lambda (e) (buffer-name (car e)))
                                  (window-prev-buffers))))
              (list prev1 prev2 prev3
                    (mapcar #'buffer-name (window-next-buffers)))))))
    (mapc (lambda (x) (when (buffer-live-p x) (kill-buffer x))) (list a c))
    (delete-other-windows)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_marker_through_complex_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-temp-buffer
  (insert "The QUICK brown FOX jumps LAZY dog")
  (let ((m1 (set-marker (make-marker) 5))
        (m2 (set-marker (make-marker) 20))
        (m3 (set-marker (make-marker) (point-max))))
    (goto-char 1)
    (while (re-search-forward "[A-Z]+" nil t)
      (replace-match (downcase (match-string 0)))
      (undo-boundary))
    (list (buffer-string)
          (marker-position m1)
          (marker-position m2)
          (marker-position m3)
          (marker-buffer m1))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_with_temp_message_message_log() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-current-buffer (get-buffer-create "*Messages*")
  (let ((inhibit-read-only t))
    (erase-buffer)))
(let ((saved-log-max message-log-max))
  (with-temp-message "temp-probe-msg"
    (list (current-message)
          (with-current-buffer "*Messages*" (buffer-string))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_indirect_buffer_undo_independence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let* ((base (generate-new-buffer " *probe-ibu-base*"))
       (ind (make-indirect-buffer base " *probe-ibu-ind*")))
  (unwind-protect
      (progn
        (with-current-buffer base
          (buffer-enable-undo)
          (insert "base-text")
          (undo-boundary))
        (with-current-buffer ind
          (buffer-enable-undo)
          (goto-char 5)
          (insert "IND")
          (undo-boundary)
          (list (buffer-string)
                (progn (undo) (buffer-string))
                (eq (buffer-base-buffer ind) base)))
        (with-current-buffer base
          (list (buffer-string)
                (consp buffer-undo-list))))
    (kill-buffer ind)
    (kill-buffer base)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_char_table_extra_slot_range_and_parent_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(let* ((parent (make-char-table 'syntax-table 'parent-val))
       (child (make-char-table 'syntax-table 'child-val)))
  (aset parent ?a 'parent-a)
  (aset parent ?z 'parent-z)
  (set-char-table-parent child parent)
  (aset child ?a 'child-a)
  (set-char-table-extra-slot child 0 'extra-0)
  (list (char-table-range child ?a)
        (char-table-range child ?z)
        (char-table-range child ?m)
        (char-table-range parent ?a)
        (char-table-extra-slot child 0)
        (char-table-extra-slot parent 0)
        (eq (char-table-parent child) parent)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
