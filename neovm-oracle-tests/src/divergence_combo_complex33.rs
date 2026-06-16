//! Complex combo batch 33 — char-width/syntax in unibyte, sentinel-collision
//! range post-fix, remaining process/coding/timer combos.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx33_char_width_high_codepoint_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (mapcar #'char-width (list ?a ?A ?1 #x3042 #x4e2d #x1f600)))
"##,
    );
}

#[test]
fn div_cx33_format_c_sentinel_range_post_fix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (aref (format "%c" #xe080) 0)
      (aref (format "%c" #xe0a0) 0)
      (aref (format "%c" #xe0ff) 0)
      (aref (format "%c" #xe300) 0)
      (aref (format "%c" #xe3ff) 0))
"##,
    );
}

#[test]
fn div_cx33_syntax_after_in_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert "ab")
  (list (syntax-after 1) (syntax-after 2)))
"##,
    );
}

#[test]
fn div_cx33_string_make_unibyte_data_loss_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (let ((u (string-make-unibyte s)))
            (list (length u) (append u nil))))
        (list "abc" "café" "世界" "😀"))
"##,
    );
}

#[test]
fn div_cx33_process_kill_query_off_then_kill_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx33-qo*")))
  (let ((p (make-process :name "neo-cx33-qo" :command '("sleep" "10")
                         :buffer buf)))
    (accept-process-output p 0.1)
    (set-process-query-on-exit-flag p nil)
    (kill-buffer buf)
    (list (buffer-live-p buf) (process-live-p p) (process-status p))))
"##,
    );
}

#[test]
fn div_cx33_coding_system_for_write_doesnt_propagate_to_subprocess() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coding-system-for-write 'utf-8-dos))
  (with-temp-buffer
    (call-process "printf" nil t nil "café\n")
    (list (buffer-string) (string-bytes (buffer-string)))))
"##,
    );
}

#[test]
fn div_cx33_timer_cancel_all_after_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((timers (list (run-with-timer 100 nil (lambda ()))
                     (run-with-timer 200 nil (lambda ()))
                     (run-with-idle-timer 100 nil (lambda ())))))
  (let ((active (length timer-list))
        (idle (length timer-idle-list)))
    (mapc #'cancel-timer timers)
    (list active idle (length timer-list) (length timer-idle-list))))
"##,
    );
}

#[test]
fn div_cx33_overlay_before_string_with_display_and_face_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (let ((ov (make-overlay 3 6)))
    (overlay-put ov 'before-string (propertize ">>" 'face 'bold 'display "XX"))
    (overlay-put ov 'after-string (propertize "<<" 'face 'italic)))
  (list (get-char-property 2 'face)
        (get-char-property 3 'face)
        (overlay-get (car (overlays-at 3)) 'before-string)))
"##,
    );
}

#[test]
fn div_cx33_cl_defstruct_with_reader_writer_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct (neo-cx33-box (:conc-name neo-cx33-box-)
                              (:reader neo-cx33-read-box))
    (val 0) name)
  (let ((b (make-neo-cx33-box :val 42 :name "test")))
    (list (neo-cx33-box-val b)
          (neo-cx33-box-name b)
          (neo-cx33-read-box b))))
"##,
    );
}

#[test]
fn div_cx33_decode_coding_string_then_set_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((decoded (decode-coding-string (unibyte-string 99 97 102 195 169) 'utf-8))
       (proped (propertize decoded 'face 'bold)))
  (list decoded (text-properties-at 0 proped) (length proped)))
"##,
    );
}

#[test]
fn div_cx33_undo_after_delete_then_insert_text_prop_overlay_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "0123456789")
  (put-text-property 1 5 'face 'bold)
  (let ((ov (make-overlay 3 7)) (m (set-marker (make-marker) 6)))
    (overlay-put ov 'face 'italic)
    (undo-boundary)
    (delete-region 2 5)
    (undo-boundary)
    (goto-char 3) (insert "XYZ")
    (let ((state (list (buffer-string) (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1))))
      (undo) (undo)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (text-properties-at 1)))))
"##,
    );
}

#[test]
fn div_cx33_window_text_height_and_body_after_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((orig (window-body-height)))
  (condition-case e
      (progn
        (split-window nil nil 'below)
        (let ((after (window-body-height)))
          (delete-other-windows)
          (list orig after (>= orig after))))
    (error (list orig :errored))))
"##,
    );
}

#[test]
fn div_cx33_process_output_with_explicit_coding_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((coding-system-for-read 'utf-8-unix))
    (call-process "printf" nil t nil "café世界\n"))
  (list (buffer-string) (count-lines 1 (point-max))
        (secure-hash 'sha256 (buffer-string))))
"##,
    );
}

#[test]
fn div_cx33_set_match_data_vector_then_search_again() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (string-match "\\(.\\)\\(.\\)" "xy")
  (set-match-data [0 2 0 1 1 2])
  (let ((md1 (list (match-string 1) (match-string 2))))
    (string-match "z" "xyz")
    (list md1 (match-string 0) (match-beginning 0))))
"##,
    );
}

#[test]
fn div_cx33_coding_system_priority_list_contains_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((prio (coding-system-priority-list)))
  (list (memq 'utf-8 prio)
        (memq 'utf-8-auto prio)
        (memq 'emacs-mule prio)))
"##,
    );
}

#[test]
fn div_cx33_char_category_in_multibyte_for_cjk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (char-category ?\x4e2d)
      (char-category ?\x3042)
      (char-category ?\xac00))
"##,
    );
}

#[test]
fn div_cx33_print_escape_nonascii_with_eight_bit_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((print-escape-nonascii t))
  (list (prin1-to-string (string-make-multibyte (unibyte-string 200 201 65)))
        (length (prin1-to-string (string-make-multibyte (unibyte-string 200))))))
"##,
    );
}

#[test]
fn div_cx33_overlay_evaporate_delete_undo_text_prop_all_restored() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "0123456789")
  (put-text-property 1 4 'face 'bold)
  (let ((ov (make-overlay 2 5)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (undo-boundary)
    (delete-region 2 5)
    (let ((evaporated (list (overlayp ov) (text-properties-at 1))))
      (undo)
      (list evaporated (overlayp ov) (overlay-start ov)
            (text-properties-at 1) (text-properties-at 2)))))
"##,
    );
}

#[test]
fn div_cx33_format_c_with_codepoint_then_concat_then_string_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((c1 (format "%c" #x3042))
       (c2 (format "%c" #x1f600))
       (cat (concat c1 c2)))
  (list (length c1) (string-bytes c1)
        (length c2) (string-bytes c2)
        (length cat) (string-bytes cat)
        (append cat nil)))
"##,
    );
}

#[test]
fn div_cx33_buffer_local_then_let_shadow_then_setq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defvar neo-cx33-var :global)
  (with-temp-buffer
    (setq-local neo-cx33-var :local)
    (list neo-cx33-var
          (let ((neo-cx33-var :shadowed)) neo-cx33-var)
          (let ((neo-cx33-var :shadowed)) (setq neo-cx33-var :set-in-shadow) neo-cx33-var)
          neo-cx33-var
          (default-value 'neo-cx33-var))))
"##,
    );
}
