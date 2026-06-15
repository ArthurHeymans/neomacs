//! Complex combo batch 25 — print representations of more objects, read
//! byte-code literal, process+timer+sentinel mega, char-table-decode more
//! charsets, deep text-property search, cl-defmethod :static, prin1 #s :type,
//! encoding error data variants.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx25_prin1_keymap_category_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((km (make-sparse-keymap))
      (ct (make-category-table)))
  (list (keymapp km) (category-table-p ct)
        (string-match "keymap" (prin1-to-string km))
        (string-match "category-table\\|#^" (prin1-to-string ct))))
"##,
    );
}

#[test]
fn div_cx25_read_byte_code_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((bc (car (read-from-string "#[0 \"\\300\\207\" [42] 1]"))))
      (list (byte-code-function-p bc)
            (compiled-function-p bc)
            (condition-case e2 (funcall bc) (error (car e2)))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx25_process_timer_sentinel_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (timer-fired sentinel-fired)
  (run-with-timer 0 nil (lambda () (setq timer-fired :timer)))
  (let ((p (make-process :name "neo-cx25-mega" :command '("echo" "done")
                         :sentinel (lambda (proc event) (setq sentinel-fired event)))))
    (accept-process-output p 2))
  (list timer-fired
        (if sentinel-fired (string-match "finished" sentinel-fired) nil)))
"##,
    );
}

#[test]
fn div_cx25_char_table_decode_char_many_charsets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((c1 (decode-char 'japanese-jisx0208 35 80))
      (c2 (decode-char 'ascii 65))
      (c3 (decode-char 'unicode #x3042)))
  (list c1 c2 c3
        (char-charset c1) (char-charset c2) (char-charset c3)
        (encode-char c1 'japanese-jisx0208)))
"##,
    );
}

#[test]
fn div_cx25_deep_text_property_search_narrow_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789ABCDEFGHIJKLMNOP")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 6 10 'face 'italic)
  (put-text-property 11 15 'mouse-face 'highlight)
  (let ((ov (make-overlay 8 12))) (overlay-put ov 'face 'underline))
  (narrow-to-region 3 18)
  (goto-char (point-min))
  (let (changes)
    (while (setq pos (next-single-property-change (point) 'face))
      (push pos changes)
      (goto-char pos))
    (list (nreverse changes)
          (text-property-any (point-min) (point-max) 'mouse-face 'highlight)
          (get-char-property 5 'face)
          (get-char-property 7 'face))))
"##,
    );
}

#[test]
fn div_cx25_cl_defmethod_static_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defclass neo-cx25-cls () ((val :initarg :val)))
  (let (log)
    (cl-defgeneric neo-cx25-fn (obj))
    (cl-defmethod neo-cx25-fn :static ((class neo-cx25-cls))
      (push :static log) :static-result)
    (cl-defmethod neo-cx25-fn ((obj neo-cx25-cls))
      (push :instance log) (oref obj val))
    (list (neo-cx25-fn 'neo-cx25-cls)
          (neo-cx25-fn (neo-cx25-cls :val 42))
          (nreverse log))))
"##,
    );
}

#[test]
fn div_cx25_prin1_struct_with_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct (neo-cx25-typed (:type vector) :named) field1 field2)
  (let* ((s (make-neo-cx25-typed :field1 "café" :field2 42))
         (p (prin1-to-string s))
         (back (car (read-from-string p))))
    (list (vectorp s) (aref back 0) (aref back 1) (aref back 2)
          (neo-cx25-typed-p back))))
"##,
    );
}

#[test]
fn div_cx25_encode_error_data_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (encode-coding-string "€" 'iso-8859-1) (error (cons (car e) (cdr e))))
      (condition-case e (encode-coding-string "中" 'iso-8859-7) (error (cons (car e) (cdr e))))
      (condition-case e (encode-coding-string "😀" 'iso-8859-9) (error (cons (car e) (cdr e)))))
"##,
    );
}

#[test]
fn div_cx25_coding_system_for_write_let_propagate_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx25-cw-")))
  (let ((coding-system-for-write 'utf-8-with-signature))
    (write-region "café" nil f nil 'silent))
  (prog1 (with-temp-buffer
           (insert-file-contents f)
           (list (buffer-string)
                 (buffer-file-coding-system)
                 (string-bytes (buffer-string))))
    (ignore-errors (delete-file f))))
"##,
    );
}

#[test]
fn div_cx25_overlay_priority_invisible_char_property_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789ABCDEF")
  (put-text-property 1 5 'face 'bold)
  (let ((o1 (make-overlay 3 7)) (o2 (make-overlay 5 10)))
    (overlay-put o1 'face 'italic)
    (overlay-put o2 'face 'underline)
    (overlay-put o1 'invisible t)
    (overlay-put o1 'priority 1)
    (overlay-put o2 'priority 5))
  (list (get-char-property 1 'face)
        (get-char-property 4 'face)
        (get-char-property 4 'invisible)
        (get-char-property 6 'face)
        (get-char-property 8 'face)
        (get-char-property 11 'face)))
"##,
    );
}

#[test]
fn div_cx25_process_kill_buffer_with_query_off() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx25-qo*")))
  (let ((p (make-process :name "neo-cx25-qo" :command '("sleep" "10")
                         :buffer buf)))
    (accept-process-output p 0.1)
    (set-process-query-on-exit-flag p nil)
    (kill-buffer buf)
    (list (buffer-live-p buf) (process-live-p p) (process-status p))))
"##,
    );
}

#[test]
fn div_cx25_decode_encode_region_latin9_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café€"))
  (list (equal s (decode-coding-string (encode-coding-string s 'iso-8859-15) 'iso-8859-15))
        (append (encode-coding-string s 'iso-8859-15) nil)))
"##,
    );
}

#[test]
fn div_cx25_undo_after_set_text_properties_on_narrowed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "0123456789ABCDEF")
  (narrow-to-region 3 13)
  (undo-boundary)
  (set-text-properties (point-min) 5 '(face bold))
  (let ((p1 (text-properties-at 1)))
    (undo)
    (list p1 (text-properties-at 1) (buffer-string))))
"##,
    );
}

#[test]
fn div_cx25_cl_lexical_dynamic_boundary_with_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lexical-binding t))
  (macrolet ((double (x) (list '+ x x)))
    (let ((f (lambda (n) (double n))))
      (funcall f 21))))
"##,
    );
}

#[test]
fn div_cx25_process_stderr_capture_and_hash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((err (generate-new-buffer " *neo-cx25-err*")))
    (call-process "sh" nil (list t err) nil "-c" "echo std-out; echo err-msg 1>&2")
    (prog1 (list (secure-hash 'md5 (buffer-string))
                 (with-current-buffer err (secure-hash 'md5 (buffer-string))))
      (kill-buffer err))))
"##,
    );
}

#[test]
fn div_cx25_string_match_p_multibyte_no_match_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (set-match-data [0 0 0 0])
  (string-match-p "café" "le café ici")
  (match-data))
"##,
    );
}

#[test]
fn div_cx25_char_width_table_default_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar #'char-width
        (list ?a ?A ?1 ?\s ?\t
              ?é ?\x100 ?\x3042 ?\x4e2d ?\xac00
              ?\x1f600 ?\x300 ?\x200b ?\x2502))
"##,
    );
}

#[test]
fn div_cx25_set_buffer_multibyte_undo_redo_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "café世界")
  (let ((orig (buffer-string)))
    (set-buffer-multibyte nil)
    (let ((uni (buffer-string)))
      (set-buffer-multibyte t)
      (let ((back (buffer-string)))
        (list (length orig) (length uni) (length back) back)))))
"##,
    );
}

#[test]
fn div_cx25_window_start_end_point_after_scroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (dotimes (i 20) (insert (format "line %d\n" i)))
  (goto-char 1)
  (list (window-start)
        (window-end)
        (progn (forward-line 5) (point))
        (window-start)))
"##,
    );
}

#[test]
fn div_cx25_overlay_evaporate_replace_match_grow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "AAAA1234BBBB")
  (let ((ov (make-overlay 5 9)))
    (overlay-put ov 'evaporate t)
    (overlay-put ov 'face 'bold))
  (goto-char 5)
  (re-search-forward "1234")
  (replace-match "XYZ")
  (list (buffer-string)
        (length (overlays-in 1 20))
        (get-char-property 4 'face)))
"##,
    );
}
