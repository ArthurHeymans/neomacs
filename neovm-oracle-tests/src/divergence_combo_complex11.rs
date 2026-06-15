//! Complex combo batch 11 — extend encode-coding-region no-op root across more
//! codings, process stderr/exit deeper, set-buffer-multibyte with undo+overlays,
//! coding-system-plist :safe-charsets, string-make-unibyte data loss, read-from-
//! string position tracking, cl-subst nested, hash-table rehash, narrow+save-
//! excursion+marker, overlay evaporate with undo, char-table-decode/encode-char.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx11_encode_region_latin1_vs_string_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café"))
  (list (append (encode-coding-string s 'latin-1) nil)
        (with-temp-buffer
          (insert s)
          (encode-coding-region (point-min) (point-max) 'latin-1)
          (append (buffer-string) nil))))
"##,
    );
}

#[test]
fn div_cx11_process_stderr_exit_code_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((err-buf (generate-new-buffer " *neo-cx11-err*")))
    (let ((code (call-process "sh" nil (list t err-buf) nil "-c" "echo stderr-msg 1>&2; exit 5")))
      (prog1 (list code (buffer-string)
                   (with-current-buffer err-buf (buffer-string)))
        (kill-buffer err-buf)))))
"##,
    );
}

#[test]
fn div_cx11_coding_system_plist_safe_charsets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-get 'utf-8 :safe-charsets)
      (coding-system-get 'latin-1 :safe-charsets)
      (coding-system-get 'utf-8 :ascii-compatible-p))
"##,
    );
}

#[test]
fn div_cx11_string_make_unibyte_data_loss() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((orig "café世界")
       (u (string-make-unibyte orig)))
  (list (append orig nil) (append u nil) (length u) (string-bytes u)))
"##,
    );
}

#[test]
fn div_cx11_read_from_string_position_tracking() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((input "(a b) (c d) (e f)")
       (r1 (read-from-string input))
       (r2 (read-from-string input (cdr r1)))
       (r3 (read-from-string input (cdr r2))))
  (list (car r1) (cdr r1) (car r2) (cdr r2) (car r3) (cdr r3)))
"##,
    );
}

#[test]
fn div_cx11_cl_subst_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-subst 'new 'old '(a old (b old) ((old . c) . old)))
"##,
    );
}

#[test]
fn div_cx11_hash_table_rehash_after_resize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal :size 2 :rehash-size 2.0)))
  (dotimes (i 100) (puthash (number-to-string i) (* i i) ht))
  (list (hash-table-count ht)
        (gethash "50" ht)
        (gethash "99" ht)
        (> (hash-table-size ht) 10)))
"##,
    );
}

#[test]
fn div_cx11_narrow_save_excursion_marker_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789ABCDEF")
  (let ((m (set-marker (make-marker) 8)))
    (narrow-to-region 3 13)
    (save-excursion
      (save-restriction
        (widen)
        (goto-char 15)
        (insert "X")))
    (list (point) (point-min) (point-max) (marker-position m))))
"##,
    );
}

#[test]
fn div_cx11_overlay_evaporate_undo_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "abcdef")
  (let ((ov (make-overlay 2 4)))
    (overlay-put ov 'evaporate t)
    (overlay-put ov 'face 'bold)
    (undo-boundary)
    (delete-region 2 4)
    (let ((alive-before (overlay-start ov)))
      (undo)
      (list alive-before (overlay-start ov) (overlay-end ov)))))
"##,
    );
}

#[test]
fn div_cx11_encode_char_decode_char_non_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((c (make-char 'japanese-jisx0208 36 34)))
  (list (encode-char c 'japanese-jisx0208)
        (decode-char 'japanese-jisx0208 (encode-char c 'japanese-jisx0208))
        (char-charset c)))
"##,
    );
}

#[test]
fn div_cx11_set_buffer_multibyte_undo_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "café")
  (let ((ov (make-overlay 2 3)))
    (overlay-put ov 'face 'bold)
    (undo-boundary)
    (set-buffer-multibyte nil)
    (let ((before (list (length (buffer-string)) (overlay-start ov))))
      (set-buffer-multibyte t)
      (list before (length (buffer-string)) (overlay-start ov) (buffer-string)))))
"##,
    );
}

#[test]
fn div_cx11_cl_map_into_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((v [1 2 3 4]))
  (cl-map-into v #'* v v)
  v)
"##,
    );
}

#[test]
fn div_cx11_decode_coding_string_multibyte_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((d (decode-coding-string (unibyte-string 195 169) 'utf-8)))
  (list (multibyte-string-p d) (unibyte-string-p d)
        (append d nil) (length d) (string-bytes d)))
"##,
    );
}

#[test]
fn div_cx11_string_lessp_collate_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((words '("apple" "éclair" "zebra" "café" "世界")))
  (sort (copy-sequence words) #'string-lessp))
"##,
    );
}

#[test]
fn div_cx11_process_kill_buffer_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx11-pkb*")))
  (with-current-buffer buf (insert "preexisting"))
  (let ((p (make-process :name "neo-cx11-pkb" :command '("echo" "process-output")
                         :buffer buf)))
    (accept-process-output p 1))
  (prog1 (with-current-buffer buf (buffer-string))
    (kill-buffer buf)))
"##,
    );
}

#[test]
fn div_cx11_char_table_range_cons_range_intersect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'cx11 nil)))
  (set-char-table-range ct '(?a . ?z) :lower)
  (set-char-table-range ct '(?m . ?s) :mid)
  (list (char-table-range ct ?a)
        (char-table-range ct ?n)
        (char-table-range ct ?z)))
"##,
    );
}

#[test]
fn div_cx11_undo_after_delete_text_property_sticky() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "AAAABBBBCCCC")
  (put-text-property 1 4 'face 'bold)
  (put-text-property 5 8 'face 'italic)
  (undo-boundary)
  (delete-region 3 9)
  (let ((after (list (buffer-string) (text-properties-at 1))))
    (undo)
    (list after (buffer-string) (text-properties-at 1) (text-properties-at 5))))
"##,
    );
}

#[test]
fn div_cx11_format_mode_line_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (list (format-mode-line mode-name)
        (format-mode-line "%m")))
"##,
    );
}

#[test]
fn div_cx11_encode_coding_region_big5_non_cjk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "中"))
  (list (append (encode-coding-string s 'big5) nil)
        (with-temp-buffer
          (insert s)
          (encode-coding-region (point-min) (point-max) 'big5)
          (append (buffer-string) nil))))
"##,
    );
}

#[test]
fn div_cx11_process_filter_accumulate_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx11-fa*")))
  (with-current-buffer buf (erase-buffer))
  (let ((p (make-process :name "neo-cx11-fa" :command '("printf" "%d %d %d" 1 2 3)
                         :buffer nil
                         :filter (lambda (proc msg)
                                   (with-current-buffer buf (insert msg))))))
    (accept-process-output p 1)
    (accept-process-output p 0.5))
  (prog1 (with-current-buffer buf (buffer-string))
    (kill-buffer buf)))
"##,
    );
}

#[test]
fn div_cx11_coding_system_mutually_compatible_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-p 'utf-8-auto)
      (coding-system-plist 'utf-8-auto)
      (coding-system-type 'utf-8-auto))
"##,
    );
}
