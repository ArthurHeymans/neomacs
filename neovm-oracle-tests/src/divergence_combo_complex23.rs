//! Complex combo batch 23 — extend known roots (encode-region, process signals,
//! front-nonsticky, Cyrillic case-fold) + new edges (split-string TRIM,
//! string-trim regex, assoc-delete-all, format-spec-make, seq-partition).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx23_encode_region_utf16_vs_string_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café"))
  (list (append (encode-coding-string s 'utf-16be) nil)
        (with-temp-buffer
          (insert s)
          (encode-coding-region (point-min) (point-max) 'utf-16be)
          (append (buffer-string) nil))))
"##,
    );
}

#[test]
fn div_cx23_process_exit_via_sigterm() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx23-st" :command '("sleep" "30"))))
  (accept-process-output p 0.1)
  (signal-process p 15)
  (accept-process-output p 1)
  (list (process-status p) (process-exit-status p)))
"##,
    );
}

#[test]
fn div_cx23_front_nonsticky_text_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "AAAAABBBBB")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 5 6 'front-nonsticky '(face))
  (goto-char 5)
  (insert "X")
  (list (get-text-property 4 'face)
        (get-text-property 5 'face)
        (get-text-property 6 'face)))
"##,
    );
}

#[test]
fn div_cx23_cyrillic_case_fold_replace_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (replace-regexp-in-string "[фг]" "X" "ФГфг test"))
"##,
    );
}

#[test]
fn div_cx23_set_multibyte_3_raw_bytes_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 128 129 65 66 200))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (point-max)
        (char-after 1) (char-after 2) (char-after 3) (char-after 4) (char-after 5)))
"##,
    );
}

#[test]
fn div_cx23_split_string_trim_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (split-string "  a, b ,c  " "," t)
      (split-string "x..y..z" "\\.+" nil t)
      (split-string "  trim me  " "\\s-+" nil t))
"##,
    );
}

#[test]
fn div_cx23_string_trim_with_regex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-trim "xxxhello worldxxx" "x+" "x+")
      (string-trim-left "000abc" "0+")
      (string-trim-right "abc999" "[0-9]+"))
"##,
    );
}

#[test]
fn div_cx23_assoc_delete_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((al '(("a" . 1) ("b" . 2) ("a" . 3) ("c" . 4) ("b" . 5))))
  (list (assoc-delete-all "a" al)
        (assq-delete-all 'b '((a . 1) (b . 2) (a . 3)))))
"##,
    );
}

#[test]
fn div_cx23_format_spec_make_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((spec (format-spec-make ?a "alpha" ?b "beta")))
      (list (format-spec "%a-%b" spec)
            (format-spec "%b-%a" spec)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx23_seq_partition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'seq)
  (list (seq-partition '(1 2 3 4 5 6 7) 3)
        (seq-partition [a b c d e] 2)
        (seq-partition "abcdefg" 2)))
"##,
    );
}

#[test]
fn div_cx23_map_pairs_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((al '((a . 1) (b . 2))))
      (sort (map-pairs (lambda (k v) (format "%s=%s" k v)) al)
            #'string<))
  (void-function (list :not-available))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx23_encode_region_big5_vs_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "中文"))
  (list (append (encode-coding-string s 'big5) nil)
        (with-temp-buffer
          (insert s)
          (encode-coding-region (point-min) (point-max) 'big5)
          (append (buffer-string) nil))))
"##,
    );
}

#[test]
fn div_cx23_process_exit_sigint() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx23-si" :command '("sleep" "30"))))
  (accept-process-output p 0.1)
  (signal-process p 2)
  (accept-process-output p 1)
  (list (process-status p) (process-exit-status p)))
"##,
    );
}

#[test]
fn div_cx23_error_predicate_across_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (aref "ab" t) (wrong-type-argument (car e)) (error :other))
      (condition-case e (aset "ab" t 65) (wrong-type-argument (car e)) (error :other))
      (condition-case e (substring "ab" t) (wrong-type-argument (car e)) (error :other)))
"##,
    );
}

#[test]
fn div_cx23_decode_encode_latin1_roundtrip_with_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s "café")
       (enc (encode-coding-string s 'latin-1))
       (dec (decode-coding-string enc 'latin-1)))
  (list (append enc nil) (append dec nil)
        (equal s dec)
        (mapcar #'char-charset (append dec nil))))
"##,
    );
}

#[test]
fn div_cx23_char_fold_exclude_include() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((char-fold-exclude '("ñ")))
      (list (string-match (char-fold-to-regexp ?n) "cañón")
            (string-match (char-fold-to-regexp ?n) "canon")))
  (void-variable (list :not-available))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx23_window_resize_no_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((orig-h (window-total-height)))
      (window-resize (selected-window) -1)
      (let ((new-h (window-total-height)))
        (window-resize (selected-window) 1)
        (list orig-h new-h (window-total-height))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx23_cl_struct_slot_value_by_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (cl-defstruct neo-cx23-point x y)
  (let ((p (make-neo-cx23-point :x 10 :y 20)))
    (list (cl-struct-slot-value 'neo-cx23-point 'x p)
          (cl-struct-slot-value 'neo-cx23-point 'y p)
          (cl-struct-slot-offset 'neo-cx23-point 'x))))
"##,
    );
}

#[test]
fn div_cx23_undo_limit_boundary_behavior() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (let ((undo-limit 10) (undo-strong-limit 20))
    (insert "0123456789")
    (undo-boundary)
    (insert "ABCDEFGHIJ")
    (let ((entries (length buffer-undo-list)))
      (undo)
      (list entries (buffer-string) (> (length buffer-undo-list) 0)))))
"##,
    );
}

#[test]
fn div_cx23_process_send_string_then_receive_hash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (received)
  (let ((p (make-process :name "neo-cx23-sh" :command '("cat")
                         :buffer nil :connection-type 'pipe
                         :filter (lambda (proc str) (push str received))))
        (data "hash this content for round-trip\n"))
    (process-send-string p data)
    (process-send-eof p)
    (accept-process-output p 1)
    (secure-hash 'md5 (apply #'concat (nreverse received)))))
"##,
    );
}
