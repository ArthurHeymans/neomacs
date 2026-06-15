//! Complex combo batch 8 — encoding/process edge continues, plus buffer/undo/
//! text-property deep stacks, coding detection, process-tty, sentinel data,
//! set-buffer-multibyte with narrowing+markers+undo, text-property merge
//! across insert/delete boundaries, print-read of propertized overlay strings.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx8_detect_coding_string_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (detect-coding-string (unibyte-string 65 66 67))
      (detect-coding-string (unibyte-string 239 187 191 97))
      (detect-coding-string (unibyte-string 254 255 0 65))
      (detect-coding-string (unibyte-string 255)))
"##,
    );
}

#[test]
fn div_cx8_coding_system_priority_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (length (coding-system-priority-list))
      (memq 'utf-8 (coding-system-priority-list)))
"##,
    );
}

#[test]
fn div_cx8_set_buffer_multibyte_narrow_marker_undo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "AAAAABBBBBCCCCC")
  (let ((m (set-marker (make-marker) 7)))
    (narrow-to-region 3 13)
    (undo-boundary)
    (set-buffer-multibyte nil)
    (set-buffer-multibyte t)
    (list (point-min) (point-max) (marker-position m) (buffer-string))))
"##,
    );
}

#[test]
fn div_cx8_text_property_merge_across_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "AAAAABBBBB")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 6 10 'face 'italic)
  (goto-char 5)
  (insert "X")
  (list (get-text-property 4 'face)
        (get-text-property 5 'face)
        (get-text-property 6 'face)
        (get-text-property 7 'face)))
"##,
    );
}

#[test]
fn div_cx8_text_property_sticky_delete_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "AAAAABBBBBCCCCC")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 6 10 'face 'italic)
  (put-text-property 11 15 'face 'underline)
  (delete-region 5 11)
  (list (length (buffer-string))
        (get-text-property 1 'face)
        (get-text-property 4 'face)
        (get-text-property 5 'face)))
"##,
    );
}

#[test]
fn div_cx8_process_sentinel_event_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (events)
  (let ((p (make-process :name "neo-cx8-sl" :command '("sh" "-c" "exit 3")
                         :sentinel (lambda (proc event) (push event events)))))
    (accept-process-output p 2))
  (if events (car events) :no-event))
"##,
    );
}

#[test]
fn div_cx8_process_inherit_coding_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx8-ic" :command '("echo" "x"))))
  (prog1 (list (process-inherit-coding-system-flag p)
               (progn (set-process-coding-system p 'latin-1-unix 'latin-1-unix)
                      (process-coding-system p)))
    (delete-process p)))
"##,
    );
}

#[test]
fn div_cx8_decode_coding_region_then_position_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 97 98 195 169 99))
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (list (buffer-string) (length (buffer-string))
        (position-bytes 1) (position-bytes 4) (position-bytes 5)))
"##,
    );
}

#[test]
fn div_cx8_encode_coding_region_then_string_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café€")
  (encode-coding-region (point-min) (point-max) 'utf-8)
  (list (length (buffer-string))
        (string-bytes (buffer-string))
        (append (buffer-string) nil)))
"##,
    );
}

#[test]
fn div_cx8_undo_redo_text_properties_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "hello world")
  (undo-boundary)
  (put-text-property 1 5 'face 'bold)
  (let ((ov (make-overlay 7 10))) (overlay-put ov 'face 'italic))
  (undo-boundary)
  (delete-region 6 11)
  (let ((after-delete (buffer-string)))
    (undo)
    (undo)
    (list after-delete
          (buffer-string)
          (text-properties-at 1)
          (length (overlays-at 7)))))
"##,
    );
}

#[test]
fn div_cx8_overlay_after_string_with_multibyte_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café世界")
  (let ((ov (make-overlay 3 5)))
    (overlay-put ov 'after-string (propertize "X世界Y" 'face 'bold)))
  (goto-char 3)
  (insert "Z")
  (list (overlay-start ov) (overlay-end ov)
        (buffer-substring-no-properties 1 (point-max))
        (overlay-get ov 'after-string)))
"##,
    );
}

#[test]
fn div_cx8_cl_getf_putf_remf_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((pl (list :a 1 :b 2 :c 3)))
  (cl-getf pl :b)
  (list pl (cl-getf pl :d :default)))
"##,
    );
}

#[test]
fn div_cx8_condition_case_nested_unwind_throw_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (log)
  (catch 'escape
    (unwind-protect
        (condition-case e
            (signal 'arith-error "boom")
          (error
           (push :handler log)
           (throw 'escape :escaped)))
      (push :cleanup log)))
  (reverse log))
"##,
    );
}

#[test]
fn div_cx8_buffer_format_then_print_circle_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s (propertize "café" 'face 'bold 'mouse-face 'highlight))
       (p (let ((print-circle t)) (prin1-to-string (list s s))))
  (list (string-match "#1=" p) p))
"##,
    );
}

#[test]
fn div_cx8_mapconcat_multibyte_with_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (mapconcat #'char-to-string "café世界" "-")
      (mapconcat (lambda (c) (format "%d" c)) "abé" "+")
      (length (mapconcat #'identity '("α" "β" "γ") ", ")))
"##,
    );
}

#[test]
fn div_cx8_write_region_coding_preserve_eol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx8-eol-")))
  (let ((coding-system-for-write 'utf-8-dos))
    (write-region "line1\nline2\n" nil f nil 0))
  (prog1 (with-temp-buffer
           (let ((coding-system-for-read 'utf-8-dos))
             (insert-file-contents f))
           (list (buffer-string) (string-bytes (buffer-string))))
    (ignore-errors (delete-file f))))
"##,
    );
}

#[test]
fn div_cx8_string_make_unibyte_multibyte_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((orig "café")
       (u (string-make-unibyte orig))
       (back (string-make-multibyte u)))
  (list (unibyte-string-p u) (multibyte-string-p back)
        (append u nil) (append back nil) (equal orig back)))
"##,
    );
}

#[test]
fn div_cx8_print_quoted_lambda_let_backquote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((print-quoted t))
  (list (prin1-to-string '(lambda (x) (* x 2)))
        (prin1-to-string '(quote x))
        (prin1-to-string '\`(a b ,(+ 1 2)))))
"##,
    );
}

#[test]
fn div_cx8_process_plist_default_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx8-pd" :command '("echo" "x"))))
  (accept-process-output p 1)
  (prog1 (list (process-get p :nonexistent)
               (process-get p 'nonexistent-sym))
    (delete-process p)))
"##,
    );
}

#[test]
fn div_cx8_set_buffer_multibyte_undo_corruption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abcdef")
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201))
  (let ((before (buffer-string)))
    (set-buffer-multibyte t)
    (let ((after (buffer-string)))
      (undo)
      (list (length before) (length after) (buffer-string)))))
"##,
    );
}

#[test]
fn div_cx8_coding_system_type_category_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-type 'utf-8)
      (coding-system-type 'latin-1)
      (coding-system-type 'emacs-mule)
      (coding-system-get 'utf-8 :mime-charset)
      (coding-system-get 'latin-1 :mime-charset))
"##,
    );
}
