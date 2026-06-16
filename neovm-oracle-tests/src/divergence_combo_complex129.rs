//! Complex combo batch 129 — `print` / `write-region` / `pp` with circular
//! references, char escaping variations, `print-escape-multibyte`,
//! and read-circle interaction.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx129_print_circle_with_shared_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((shared (list 1 2 3)))
  (let ((data (list shared shared (copy-sequence shared))))
    (list (let ((print-circle t)) (prin1-to-string data))
          (let ((print-circle nil)) (condition-case e (prin1-to-string data)
                                       (error (car e)))))))
"##,
    );
}

#[test]
fn div_cx129_print_circle_with_circular_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((circular (list 1 2 3)))
  (setcdr (cddr circular) circular)
  (list (let ((print-circle t)) (prin1-to-string circular))
        (let ((print-circle nil)) (condition-case e (prin1-to-string circular)
                                     (error (car e))))))
"##,
    );
}

#[test]
fn div_cx129_print_escape_multibyte_with_eight_bit_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s (decode-coding-string (unibyte-string #xff #xfe) 'utf-8-unix t)))
  (list (prin1-to-string s)
        (let ((print-escape-multibyte t)) (prin1-to-string s))
        (let ((print-escape-nonascii t)) (prin1-to-string s))))
"##,
    );
}

#[test]
fn div_cx129_print_length_truncates_lists() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((long (number-sequence 1 50)))
  (list (let ((print-length 5)) (prin1-to-string long))
        (let ((print-length 0)) (prin1-to-string long))
        (let ((print-length nil)) (prin1-to-string long))))
"##,
    );
}

#[test]
fn div_cx129_print_level_truncates_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((deep (list (list (list (list (list :deep)))))))
  (list (let ((print-level 0)) (prin1-to-string deep))
        (let ((print-level 1)) (prin1-to-string deep))
        (let ((print-level 2)) (prin1-to-string deep))
        (let ((print-level nil)) (prin1-to-string deep))))
"##,
    );
}

#[test]
fn div_cx129_print_gensym_for_uninterned_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((gs (gensym)))
  (list (symbol-name gs)
        (let ((print-gensym t)) (prin1-to-string gs))
        (let ((print-gensym nil)) (prin1-to-string gs))))
"##,
    );
}

#[test]
fn div_cx129_pp_indentation_preserves_nested_structure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((data '(:config
              (:option-a "value"
               :option-b (:nested-a 1
                          :nested-b 2))
              :option-c (1 2 3))))
  (let ((pp-str (pp-to-string data)))
    (list (> (length pp-str) 0)
          (> (length (split-string pp-str "\n")) 3)
          (car (split-string pp-str "\n")))))
"##,
    );
}

#[test]
fn div_cx129_read_circle_interactive_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((shared (list 1 2 3))
       (data (list shared shared))
       (printed (let ((print-circle t)) (prin1-to-string data)))
       (read-with (let ((read-circle t)) (read-from-string printed)))
       (read-without (let ((read-circle nil)) (condition-case e
                                                   (read-from-string printed)
                                                 (error (car e))))))
  (list printed
        (car read-with)
        read-without))
"##,
    );
}

#[test]
fn div_cx129_write_region_append_to_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((path (make-temp-file "neo-cx129-app")))
  (delete-file path)
  (with-temp-buffer (insert "A") (write-region (point-min) (point-max) path nil 'silent))
  (with-temp-buffer (insert "B") (write-region (point-min) (point-max) path 'append 'silent))
  (with-temp-buffer (insert "C") (write-region (point-min) (point-max) path 'append 'silent))
  (let ((content (with-temp-buffer
                   (insert-file-contents path)
                   (buffer-string))))
    (delete-file path)
    content))
"##,
    );
}

#[test]
fn div_cx129_prin1_to_string_with_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s (propertize "hello" 'face 'bold 'cat :greek)))
  (list (prin1-to-string s)
        (princ-to-string s)
        (length (prin1-to-string s))))
"##,
    );
}

#[test]
fn div_cx129_print_to_buffer_temp_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx129-print*")))
  (with-current-buffer buf
    (erase-buffer))
  (print '(1 2 3) buf)
  (print "string" buf)
  (print 'symbol buf)
  (let ((content (with-current-buffer buf (buffer-string))))
    (kill-buffer buf)
    content))
"##,
    );
}

#[test]
fn div_cx129_print_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((shared (list 1 2 3)))
  (let ((data (list shared shared (list :a :b))))
    (let ((printed (let ((print-circle t)) (prin1-to-string data))))
      (with-temp-buffer
        (buffer-enable-undo)
        (insert printed)
        (put-text-property 1 6 'face 'bold)
        (let ((m (set-marker (make-marker) 8))
              (ov (make-overlay 4 14)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 18)
          (let ((state (list printed
                             (buffer-string)
                             (marker-position m)
                             (overlay-start ov) (overlay-end ov)
                             (text-properties-at 1))))
            (undo)
            (widen)
            (list state (buffer-string) (marker-position m)
                  (overlay-start ov) (overlay-end ov)
                  (text-properties-at 1)))))))
"##,
    );
}
