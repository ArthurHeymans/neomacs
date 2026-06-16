//! Complex combo batch 32 — extend char-syntax-in-unibyte, reader edge,
//! coding auto-detect, process signal exit code, remaining interaction edges.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx32_char_syntax_high_codepoint_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (mapcar #'char-syntax (list ?a ?A ?1 ?\s ?\n ?\( ?\; #x3042 #x4e2d #x1f600)))
"##,
    );
}

#[test]
fn div_cx32_char_category_high_codepoint_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (mapcar #'char-category (list ?a ?A ?1 #x3042 #x4e2d)))
"##,
    );
}

#[test]
fn div_cx32_reader_backquote_nested_splice() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((x 1) (y 2) (lst '(a b c)))
  (list (eval (car (read-from-string "`(,x (,@lst) ,y)")) t)
        (eval (car (read-from-string "`(,x ,@(mapcar #'1+ '(1 2 3)))")) t)))
"##,
    );
}

#[test]
fn div_cx32_coding_system_for_read_undecided_auto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx32-ua-")))
  (let ((coding-system-for-write 'utf-8-with-signature))
    (write-region "café世界" nil f nil 'silent))
  (prog1 (with-temp-buffer
           (let ((coding-system-for-read 'undecided))
             (insert-file-contents f))
           (list (buffer-string) (buffer-file-coding-system)))
    (ignore-errors (delete-file f))))
"##,
    );
}

#[test]
fn div_cx32_process_signal_exit_code_sigterm() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx32-term" :command '("sleep" "30"))))
  (accept-process-output p 0.1)
  (signal-process p 15)
  (accept-process-output p 1)
  (list (process-status p) (process-exit-status p)))
"##,
    );
}

#[test]
fn div_cx32_encode_coding_region_no_op_extension() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café"))
  (with-temp-buffer
    (insert s)
    (encode-coding-region (point-min) (point-max) 'utf-8)
    (list (length (buffer-string))
          (append (buffer-string) nil))))
"##,
    );
}

#[test]
fn div_cx32_display_property_move_to_column_exact() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 1 3 'display (make-string 8 88))
  (move-to-column 5)
  (current-column))
"##,
    );
}

#[test]
fn div_cx32_set_buffer_multibyte_overlay_position_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "AAAAABBBBBCCCCC")
  (let ((ov (make-overlay 4 8)) (m (set-marker (make-marker) 6)))
    (overlay-put ov 'face 'bold)
    (set-buffer-multibyte nil)
    (let ((nil-state (list (overlay-start ov) (overlay-end ov) (marker-position m))))
      (set-buffer-multibyte t)
      (list nil-state (overlay-start ov) (overlay-end ov) (marker-position m)))))
"##,
    );
}

#[test]
fn div_cx32_cl_loop_for_hash_being_keys_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal)))
  (dotimes (i 10) (puthash (number-to-string i) (* i i) ht))
  (sort (cl-loop for k being the hash-keys of ht
                 when (cl-evenp (string-to-number k))
                 collect k)
        #'string<))
"##,
    );
}

#[test]
fn div_cx32_coding_system_decode_string_no_conversion_vs_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((raw (unibyte-string 99 97 102 195 169 226 130 172)))
  (list (decode-coding-string raw 'no-conversion)
        (decode-coding-string raw 'utf-8)
        (length (decode-coding-string raw 'no-conversion))
        (length (decode-coding-string raw 'utf-8))))
"##,
    );
}

#[test]
fn div_cx32_undo_after_insert_with_text_property_sticky_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "AAAAABBBBB")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 6 10 'face 'italic)
  (undo-boundary)
  (goto-char 5)
  (insert "X")
  (let ((after (list (get-text-property 4 'face)
                     (get-text-property 5 'face)
                     (get-text-property 6 'face))))
    (undo)
    (list after
          (get-text-property 4 'face)
          (get-text-property 5 'face))))
"##,
    );
}

#[test]
fn div_cx32_process_buffer_with_invisible_text_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx32-it*")))
  (with-current-buffer buf
    (insert "VIS1 ")
    (put-text-property 1 5 'invisible t))
  (let ((p (make-process :name "neo-cx32-it" :command '("echo" "output")
                         :buffer buf)))
    (accept-process-output p 1))
  (prog1 (with-current-buffer buf (buffer-string))
    (kill-buffer buf)))
"##,
    );
}

#[test]
fn div_cx32_window_parameter_persistent_after_buffer_switch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)) (buf1 (get-buffer-create " *neo-cx32-b1*")))
  (set-window-buffer w buf1)
  (set-window-parameter w 'neo-cx32-wp :val)
  (let ((p1 (window-parameter w 'neo-cx32-wp)))
    (set-window-buffer w (get-buffer-create "*scratch*"))
    (prog1 (list p1 (window-parameter w 'neo-cx32-wp))
      (kill-buffer buf1))))
"##,
    );
}

#[test]
fn div_cx32_string_bytes_concat_unibyte_multibyte_mix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((u (unibyte-string 200 201))
       (m "café")
       (cat (concat u m)))
  (list (multibyte-string-p u) (multibyte-string-p m)
        (multibyte-string-p cat)
        (length cat) (string-bytes cat)
        (append cat nil)))
"##,
    );
}

#[test]
fn div_cx32_format_spec_with_multibyte_in_spec_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((spec (format-spec-make ?a "café" ?b "世界")))
      (format-spec "key-a: %a, key-b: %b" spec))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx32_overlay_priority_face_precedence_five_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789")
  (let ((o1 (make-overlay 1 10)) (o2 (make-overlay 1 10))
        (o3 (make-overlay 1 10)) (o4 (make-overlay 1 10))
        (o5 (make-overlay 1 10)))
    (overlay-put o1 'face 'bold)
    (overlay-put o2 'face 'italic)
    (overlay-put o3 'face 'underline)
    (overlay-put o4 'face 'shadow)
    (overlay-put o5 'face 'highlight)
    (overlay-put o1 'priority 1) (overlay-put o2 'priority 2)
    (overlay-put o3 'priority 3) (overlay-put o4 'priority 4)
    (overlay-put o5 'priority 5)
    (get-char-property 5 'face)))
"##,
    );
}

#[test]
fn div_cx32_coding_system_get_charset_list_broad() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-get 'utf-8 :charset-list)
      (coding-system-get 'iso-8859-1 :charset-list)
      (coding-system-get 'iso-8859-7 :charset-list)
      (coding-system-get 'big5 :charset-list))
"##,
    );
}

#[test]
fn div_cx32_char_syntax_ascii_in_multibyte_then_unibyte_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc")
  (let ((mb (mapcar #'char-syntax '(?a ?b ?c ?\( ?\; ?\s))))
    (set-buffer-multibyte nil)
    (let ((ub (mapcar #'char-syntax '(?a ?b ?c ?\( ?\; ?\s))))
      (set-buffer-multibyte t)
      (list mb ub (equal mb ub)))))
"##,
    );
}

#[test]
fn div_cx32_print_circle_shared_cons_read_back_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((x (cons 1 nil))
       (print-circle t)
       (_ (setcdr x x))
       (p (prin1-to-string x))
       (back (car (read-from-string p))))
  (list (eq back (cdr back))
        (car back)))
"##,
    );
}

#[test]
fn div_cx32_buffer_hash_after_propertize_then_depropertize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello world")
  (let ((h1 (buffer-hash)))
    (put-text-property 1 5 'face 'bold)
    (let ((h2 (buffer-hash)))
      (set-text-properties 1 (point-max) nil)
      (list h1 h2 (equal h1 (buffer-hash))))))
"##,
    );
}
