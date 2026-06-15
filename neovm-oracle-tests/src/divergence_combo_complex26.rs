//! Complex combo batch 26 — read malformed input, format %s complex objects,
//! cl-coerce edges, window-resize effect, syntax-pp after mod, hash-table
//! custom test prin1, seq/map extensions, process thread combo.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx26_read_malformed_error_handling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (read-from-string "(") (error (car e)))
      (condition-case e (read-from-string "\"unterminated") (error (car e)))
      (condition-case e (read-from-string "#(") (error (car e)))
      (condition-case e (read-from-string ".") (error (car e))))
"##,
    );
}

#[test]
fn div_cx26_format_s_hash_table_compiled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal)))
  (puthash 'a 1 ht)
  (let ((cf (byte-compile (lambda (x) (* x 2)))))
    (list (format "%s" ht)
          (string-match "hash-table" (format "%s" ht))
          (format "%s" cf)
          (string-match "#<compiled\\|#<closure\\|lambda" (format "%s" cf)))))
"##,
    );
}

#[test]
fn div_cx26_hash_table_custom_test_prin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht-eq (make-hash-table :test 'eq))
      (ht-equal (make-hash-table :test 'equal))
      (ht-uniq (make-hash-table :test 'equal-including-properties)))
  (list (string-match ":test eq" (prin1-to-string ht-eq))
        (string-match ":test equal" (prin1-to-string ht-equal))
        (string-match "equal-including-properties" (prin1-to-string ht-uniq))))
"##,
    );
}

#[test]
fn div_cx26_cl_coerce_edge_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-coerce 65 'char)
      (cl-coerce ?a 'integer)
      (cl-coerce '(1 2 3) 'vector)
      (cl-coerce [1 2 3] 'list)
      (cl-coerce "abc" 'list)
      (cl-coerce '(97 98 99) 'string)
      (condition-case e (cl-coerce "abc" 'integer) (error (car e))))
"##,
    );
}

#[test]
fn div_cx26_window_resize_effect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((orig-h (window-total-height)))
      (window-resize (selected-window) -2)
      (let ((smaller (window-total-height)))
        (window-resize (selected-window) 2)
        (list orig-h smaller (window-total-height)
              (>= (window-total-height) orig-h))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx26_syntax_pp_after_modification() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defun foo () \"str\")")
  (goto-char 15)
  (let ((pp1 (syntax-ppss)))
    (insert "X")
    (let ((pp2 (syntax-ppss)))
      (list (nth 0 pp1) (nth 0 pp2)
            (nth 3 pp1) (nth 3 pp2)
            (nth 8 pp1) (nth 8 pp2)))))
"##,
    );
}

#[test]
fn div_cx26_seq_reduce_max_extensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'seq)
  (list (seq-reduce #'+ '(1 2 3 4) 0)
        (seq-max '(3 1 4 1 5 9))
        (seq-min '(3 1 4 1 5 9))
        (seq-find #'cl-evenp '(1 3 4 5))
        (seq-count #'cl-oddp '(1 2 3 4 5))))
"##,
    );
}

#[test]
fn div_cx26_process_thread_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx26-pt" :command '("echo" "x")))
      (t1 (make-thread (lambda () (sleep-for 0.01)))))
  (accept-process-output p 0.5)
  (let ((p-status (process-status p)))
    (thread-join t1)
    (list p-status (eq (process-status p) 'exit)
          (null (thread-live-p t1))
          (threadp t1))))
"##,
    );
}

#[test]
fn div_cx26_condition_case_no_debug() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((debug-on-error t))
  (list (condition-case-no-debug
            (error "boom")
          (error :caught-no-debug))
        (condition-case
            (error "boom")
          (error :caught-normal))))
"##,
    );
}

#[test]
fn div_cx26_decode_encode_string_no_conversion_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((raw (unibyte-string 0 1 127 128 200 255)))
  (list (decode-coding-string raw 'no-conversion)
        (encode-coding-string (decode-coding-string raw 'no-conversion) 'no-conversion)
        (equal raw (encode-coding-string (decode-coding-string raw 'no-conversion) 'no-conversion))))
"##,
    );
}

#[test]
fn div_cx26_overlay_after_string_then_buffer_substring_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((ov (make-overlay 2 4)))
    (overlay-put ov 'after-string (propertize "XY" 'face 'bold 'mouse-face 'highlight)))
  (let* ((ov (car (overlays-at 2)))
         (as (overlay-get ov 'after-string))
         (sub (buffer-substring 2 5)))
    (list (text-properties-at 0 as)
          (text-properties-at 1 as)
          sub
          (text-properties-at 0 sub))))
"##,
    );
}

#[test]
fn div_cx26_cl_setf_on_aref_vector_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((v (vector 1 2 3 4 5)))
  (setf (aref v 2) 99)
  (cl-rotatef (aref v 0) (aref v 4))
  (cl-shiftf (aref v 1) (aref v 3) 0)
  v)
"##,
    );
}

#[test]
fn div_cx26_char_table_default_value_access() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'cx26 :default-val)))
  (list (char-table-range ct ?a)
        (char-table-range ct ?z)
        (char-table-range ct t)
        (progn (set-char-table-default-slot ct :new-default)
               (char-table-range ct ?a))))
"##,
    );
}

#[test]
fn div_cx26_buffer_undo_list_format_overlay_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "hello")
  (let ((before (copy-sequence buffer-undo-list)))
    (let ((ov (make-overlay 1 3)))
      (overlay-put ov 'face 'bold))
    (undo-boundary)
    (delete-region 1 2)
    (list (length buffer-undo-list)
          (> (length buffer-undo-list) (length before))
          (consp (car buffer-undo-list)))))
"##,
    );
}

#[test]
fn div_cx26_format_escape_combined_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((print-escape-newlines t)
      (print-escape-nonascii t)
      (print-escape-multibyte t)
      (print-circle t))
  (list (prin1-to-string "café\n\t世界")
        (length (prin1-to-string "café\n\t世界"))
        (let ((x (list 1))) (setcdr x x) (prin1-to-string x))))
"##,
    );
}

#[test]
fn div_cx26_process_environment_set_then_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((orig-env (copy-sequence process-environment)))
  (setenv "NEO_CX26_ENV" "test-value")
  (let ((direct (getenv "NEO_CX26_ENV")))
    (setq process-environment orig-env)
    (list direct (getenv "NEO_CX26_ENV"))))
"##,
    );
}

#[test]
fn div_cx26_marker_buffer_after_kill_buffer_marker_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf1 (get-buffer-create " *neo-cx26-m1*"))
      (buf2 (get-buffer-create " *neo-cx26-m2*")))
  (let ((m1 (set-marker (make-marker) 3 buf1))
        (m2 (set-marker (make-marker) 5 buf2)))
    (kill-buffer buf1)
    (prog1 (list (marker-buffer m1) (marker-position m1)
                 (marker-buffer m2) (marker-position m2))
      (kill-buffer buf2))))
"##,
    );
}

#[test]
fn div_cx26_string_bytes_of_format_c_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s1 (format "%c%c%c" ?a ?é #x3042)))
  (list s1 (length s1) (string-bytes s1)
        (append s1 nil)
        (multibyte-string-p s1)))
"##,
    );
}

#[test]
fn div_cx26_overlay_evaporate_after_delete_undo_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "0123456789")
  (let ((ov (make-overlay 3 6)))
    (overlay-put ov 'evaporate t)
    (overlay-put ov 'face 'bold)
    (undo-boundary)
    (delete-region 3 6)
    (let ((after (list (overlayp ov) (overlay-start ov))))
      (undo)
      (list after (overlayp ov) (overlay-start ov) (overlay-end ov)
            (get-char-property 3 'face)))))
"##,
    );
}

#[test]
fn div_cx26_coding_system_mime_charset_broad() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cs) (coding-system-get cs :mime-charset))
        '(utf-8 utf-16 utf-16be utf-16le latin-1 iso-8859-15
          iso-8859-7 big5 gb2312 shift_jis euc-jp emacs-mule))
"##,
    );
}
