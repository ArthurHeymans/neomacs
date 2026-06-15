//! Complex combo batch 16 — secure-hash file FIXED probe (delete+rewrite),
//! with-temp-message deeper, process exit code via call-process return,
//! encode-coding-region consistency across codings, char-fold + case-fold
//! combined, overlay before/after-string + narrowing + point-motion,
//! cl-loop with multiple accumulators + conditionals, timer-list after
//! multiple timers, read-delimited, format-spec with multibyte values,
//! buffer-local face-remap + font-lock precedence, process send-string
//! + process-buffer + narrowing.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx16_secure_hash_file_rewritten() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx16-sh-")))
  (unwind-protect
      (progn
        (write-region "ascii content" nil f nil 0)
        (let ((h1 (secure-hash 'sha256 f)))
          (delete-file f)
          (write-region "café世界" nil f nil 0)
          (let ((h2 (secure-hash 'sha256 f)))
            (list h1 h2 (equal h1 h2)))))
    (ignore-errors (delete-file f))))
"##,
    );
}

#[test]
fn div_cx16_call_process_return_exit_code() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (call-process "sh" nil nil nil "-c" "exit 0")
      (call-process "sh" nil nil nil "-c" "exit 7")
      (call-process "sh" nil nil nil "-c" "exit 42")
      (call-process "sh" nil nil nil "-c" "exit 255"))
"##,
    );
}

#[test]
fn div_cx16_encode_coding_region_all_codings_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "AB"))
  (list (with-temp-buffer (insert s) (encode-coding-region 1 (point-max) 'utf-8) (length (buffer-string)))
        (with-temp-buffer (insert s) (encode-coding-region 1 (point-max) 'latin-1) (length (buffer-string)))
        (with-temp-buffer (insert s) (encode-coding-region 1 (point-max) 'utf-16be) (length (buffer-string)))
        (append (encode-coding-string s 'utf-8) nil)
        (with-temp-buffer (insert s) (encode-coding-region 1 (point-max) 'utf-8) (append (buffer-string) nil))))
"##,
    );
}

#[test]
fn div_cx16_char_fold_case_fold_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (string-match (char-fold-to-regexp ?e) "É")
        (string-match (char-fold-to-regexp ?É) "e")
        (string-match (char-fold-to-regexp ?a) "Á")))
"##,
    );
}

#[test]
fn div_cx16_overlay_before_after_narrow_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789ABCDEF")
  (let ((ov (make-overlay 5 8)))
    (overlay-put ov 'before-string ">>")
    (overlay-put ov 'after-string "<<")
    (overlay-put ov 'face 'bold))
  (narrow-to-region 3 13)
  (goto-char 5)
  (insert "X")
  (list (point-min) (point-max) (buffer-string)
        (length (overlays-in (point-min) (point-max)))
        (get-char-property 4 'face)))
"##,
    );
}

#[test]
fn div_cx16_cl_loop_multi_accumulator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-loop for i from 1 to 10
         if (cl-evenp i) collect i into evens
         else collect i into odds
         sum i into total
         finally (return (list evens odds total)))
"##,
    );
}

#[test]
fn div_cx16_timer_list_after_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t1 (run-with-timer 100 nil (lambda ())))
      (t2 (run-with-timer 200 nil (lambda ())))
      (t3 (run-with-idle-timer 100 nil (lambda ()))))
  (prog1 (list (length timer-list)
               (length timer-idle-list)
               (timerp t1) (timerp t2) (timerp t3))
    (cancel-timer t1) (cancel-timer t2) (cancel-timer t3)))
"##,
    );
}

#[test]
fn div_cx16_format_spec_multibyte_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (format-spec "%a and %b"
                 '((97 . "café") (98 . "世界")))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx16_face_remap_font_lock_precedence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun foo ())")
    (font-lock-fontify-buffer)
    (let ((cookie (face-remap-add-relative 'font-lock-keyword-face :weight 'bold)))
      (prog1 (list (get-text-property 2 'face)
                   (face-attribute 'font-lock-keyword-face :weight))
        (face-remap-remove-relative cookie)))))
"##,
    );
}

#[test]
fn div_cx16_process_send_buffer_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "before-middle-after")
  (narrow-to-region 7 13)
  (let ((p (make-process :name "neo-cx16-ps" :command '("cat")
                         :buffer nil :connection-type 'pipe)))
    (process-send-region p (point-min) (point-max))
    (process-send-eof p)
    (accept-process-output p 1))
  (widen)
  (buffer-string))
"##,
    );
}

#[test]
fn div_cx16_decode_coding_region_then_aset_grow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 97 98 195 169))
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (aset (buffer-string) 2 ?X)
  (list (buffer-string) (length (buffer-string)) (string-bytes (buffer-string))))
"##,
    );
}

#[test]
fn div_cx16_cl_lexical_closure_mutual_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lexical-binding t))
  (let ((get-set nil))
    (let ((val 0))
      (setq get-set (cons (lambda () val)
                          (lambda (new) (setq val new)))))
    (list (funcall (car get-set))
          (funcall (cdr get-set) 42)
          (funcall (car get-set)))))
"##,
    );
}

#[test]
fn div_cx16_overlay_priority_mouse_face_precedence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ABCDEFGHIJ")
  (let ((o1 (make-overlay 2 6)) (o2 (make-overlay 4 8)))
    (overlay-put o1 'face 'bold)
    (overlay-put o2 'mouse-face 'highlight)
    (overlay-put o1 'mouse-face 'secondary)
    (overlay-put o1 'priority 1)
    (overlay-put o2 'priority 5)
    (list (get-char-property 3 'face)
          (get-char-property 3 'mouse-face)
          (get-char-property 5 'face)
          (get-char-property 5 'mouse-face))))
"##,
    );
}

#[test]
fn div_cx16_read_from_string_multiple_forms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((input "(a) (b) (c) \"str\" 42 [vec]")
       (pos 0)
       (forms nil))
  (while (< pos (length input))
    (let ((r (read-from-string input pos)))
      (push (car r) forms)
      (setq pos (cdr r))))
  (nreverse forms))
"##,
    );
}

#[test]
fn div_cx16_undo_text_prop_change_does_not_affect_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "hello world")
  (let ((m (set-marker (make-marker) 7)))
    (undo-boundary)
    (put-text-property 1 5 'face 'bold)
    (undo-boundary)
    (delete-region 6 11)
    (let ((after-delete (marker-position m)))
      (undo)
      (list after-delete (marker-position m)
            (text-properties-at 1) (buffer-string)))))
"##,
    );
}

#[test]
fn div_cx16_process_kill_then_status() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx16-pk" :command '("sleep" "30"))))
  (accept-process-output p 0.1)
  (delete-process p)
  (accept-process-output p 0.1)
  (list (process-status p) (process-live-p p)))
"##,
    );
}

#[test]
fn div_cx16_coding_system_mime_charset_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-get 'utf-8 :mime-charset)
      (coding-system-get 'latin-1 :mime-charset)
      (coding-system-get 'iso-8859-7 :mime-charset)
      (coding-system-get 'big5 :mime-charset)
      (coding-system-get 'shift_jis :mime-charset))
"##,
    );
}

#[test]
fn div_cx16_string_lessp_with_raw_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a (string-make-multibyte (unibyte-string 200)))
      (b (string-make-multibyte (unibyte-string 201))))
  (list (string-lessp a b)
        (string-lessp b a)
        (string-lessp a a)))
"##,
    );
}

#[test]
fn div_cx16_buffer_file_name_coding_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((dir (make-temp-file "neo-cx16-bfn-" t))
       (f (expand-file-name "café.txt" dir)))
  (condition-case e
      (progn
        (write-region "content" nil f nil 0)
        (list (file-exists-p f)
              (directory-files dir nil "^[^.]")))
    (error (cons 'errored (car e))))
  (ignore-errors (delete-directory dir t)))
"##,
    );
}

#[test]
fn div_cx16_multiple_coding_systems_decode_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((raw (unibyte-string 99 97 102 233)))
  (list (decode-coding-string raw 'latin-1)
        (decode-coding-string raw 'utf-8)
        (decode-coding-string raw 'no-conversion)
        (append (decode-coding-string raw 'latin-1) nil)
        (append (decode-coding-string raw 'no-conversion) nil)))
"##,
    );
}
