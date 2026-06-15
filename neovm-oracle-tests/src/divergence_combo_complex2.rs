//! Complex combo divergence probes batch 2 — deeper interaction edges.
//!
//! decode/encode-coding-region + multibyte buffer, sort + text-property
//! preservation, translate-region + undo, kmacro-exec, with-output-to-string,
//! char-width table + column, match-data save/restore, hash-table test funcs,
//! save-restriction stacking, set-text-properties on strings, read #N= labels.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx2_decode_coding_region_multibyte_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 99 97 102 195 169 226 130 172))
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (list (buffer-string) (length (buffer-string)) (string-bytes (buffer-string))))
"##,
    );
}

#[test]
fn div_cx2_encode_coding_region_latin1_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café€")
  (encode-coding-region (point-min) (point-max) 'latin-1)
  (list (length (buffer-string)) (append (buffer-string) nil)))
"##,
    );
}

#[test]
fn div_cx2_sort_buffer_text_property_preservation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "banana\napple\ncherry\n")
  (put-text-property 1 7 'face 'bold)
  (let ((beg (point-min)) (end (point-max)))
    (sort-lines nil beg end))
  (list (buffer-string)
        (text-properties-at 1)
        (text-properties-at 8)))
"##,
    );
}

#[test]
fn div_cx2_translate_region_undo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'translation-table)))
  (aset ct ?a ?X)
  (aset ct ?b ?Y)
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "abcabc")
    (translate-region (point-min) (point-max) ct)
    (let ((after (buffer-string)))
      (undo)
      (list after (buffer-string)))))
"##,
    );
}

#[test]
fn div_cx2_with_output_to_string_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-output-to-string
  (princ "café")
  (princ "世界")
  (princ (format " %d" 42)))
"##,
    );
}

#[test]
fn div_cx2_kmacro_exec_buffer_mod() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn (require 'kmacro)
      (let ((km (kmacro (kbd "a b c RET"))))
        (with-temp-buffer
          (kmacro-call-macro km)
          (buffer-string))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx2_char_width_table_modify_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "aéb")
  (let ((orig (char-width ?é)))
    (set-char-table-range (char-width-table) ?é 3)
    (prog1 (list orig (char-width ?é) (current-column) (string-width "aéb"))
      (set-char-table-range (char-width-table) ?é orig))))
"##,
    );
}

#[test]
fn div_cx2_match_data_save_restore_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (string-match "\\(a\\)\\(b\\)" "xab")
  (let ((saved (match-data)))
    (string-match "x" "xyz")
    (set-match-data saved)
    (list (match-string 1) (match-string 2) (match-beginning 0) (match-end 0))))
"##,
    );
}

#[test]
fn div_cx2_hash_table_test_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((heq (make-hash-table :test 'eq))
      (heql (make-hash-table :test 'eql))
      (hequal (make-hash-table :test 'equal)))
  (puthash "key" 1 heq)
  (puthash "key" 2 heql)
  (puthash "key" 3 hequal)
  (list (gethash "key" heq 'missing)
        (gethash "key" heql 'missing)
        (gethash "key" hequal 'missing)))
"##,
    );
}

#[test]
fn div_cx2_save_restriction_stack() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "0123456789ABCDE")
  (narrow-to-region 1 10)
  (save-restriction
    (widen)
    (narrow-to-region 5 15)
    (list (point-min) (point-max)))
  (list (point-min) (point-max)))
"##,
    );
}

#[test]
fn div_cx2_set_text_props_on_string_substring() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s (copy-sequence "abcdef"))
       (_ (put-text-property 1 4 'face 'bold s))
       (sub (substring s 2 5)))
  (list (text-properties-at 0 sub)
        (text-properties-at 1 sub)
        (text-properties-at 2 sub)))
"##,
    );
}

#[test]
fn div_cx2_read_circular_labels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((x (car (read-from-string "#1=(a b . #1#)")))
       (tail (cdr (cdr x))))
  (eq tail x))
"##,
    );
}

#[test]
fn div_cx2_cl_coerce_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-coerce "aé中" 'list)
      (length (cl-coerce "aé中" 'list))
      (cl-coerce (cl-coerce "aé中" 'list) 'string))
"##,
    );
}

#[test]
fn div_cx2_default_value_let_setq_lifecycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defvar neo-lc nil)
  (setq-default neo-lc :defaulted)
  (list (default-value 'neo-lc)
        (let ((neo-lc :letbound)) neo-lc)
        (default-value 'neo-lc)
        (let ((neo-lc :letbound)) (setq neo-lc :setq) neo-lc)
        (default-value 'neo-lc)))
"##,
    );
}

#[test]
fn div_cx2_marker_multibyte_toggle_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café世界test")
  (let ((m (set-marker (make-marker) 5)))
    (narrow-to-region 3 8)
    (goto-char 4)
    (insert "X")
    (list (marker-position m) (point-min) (point-max))))
"##,
    );
}

#[test]
fn div_cx2_syntax_text_property_forward_sexp_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "word1 word2 [bracket]")
  (put-text-property 13 14 'syntax-table (string-to-syntax "."))
  (goto-char 1)
  (let ((parse-sexp-lookup-properties t))
    (list (progn (forward-sexp 2) (point))
          (progn (forward-sexp 1) (point)))))
"##,
    );
}

#[test]
fn div_cx2_add_remove_text_props_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefgh")
  (add-text-properties 2 6 '(face bold mouse-face highlight))
  (remove-text-properties 3 5 '(mouse-face))
  (list (text-properties-at 2) (text-properties-at 3) (text-properties-at 5)
        (next-single-property-change 1 'mouse-face)))
"##,
    );
}

#[test]
fn div_cx2_format_spec_dynamic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (format-spec "%a is %b" '((97 . "alpha") (98 . "beta")))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx2_process_send_receive_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (got)
  (let ((p (make-process :name "neo-rt" :command '("cat")
                         :connection-type 'pipe :buffer nil
                         :filter (lambda (proc str) (push str got))
                         :sentinel (lambda (proc ev) nil))))
    (process-send-string p "roundtrip\n")
    (process-send-eof p)
    (accept-process-output p 1))
  (apply #'concat (nreverse got)))
"##,
    );
}

#[test]
fn div_cx2_print_circle_shared_vector_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((v (vector 1 2 3))
       (print-circle t))
  (prin1-to-string (list v v v)))
"##,
    );
}
