//! Complex combo batch 19 — write-region VISIT deep, process-buffer-kill
//! confirmation, cl-letf generalized places, print truncation extremes,
//! hash-table weakness, prin1 eight-bit strings, sort edge comparators,
//! window-config register round-trip.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx19_write_region_visit_variants_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx19-v-"))
      (results nil))
  (dolist (visit '(nil t 0 'silent))
    (write-region "café" nil f nil visit)
    (push (secure-hash 'md5 f) results))
  (prog1 (nreverse results) (ignore-errors (delete-file f)))
"##,
    );
}

#[test]
fn div_cx19_process_buffer_kill_query_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx19-pq*")))
  (let ((p (make-process :name "neo-cx19-pq" :command '("sleep" "10")
                         :buffer buf)))
    (accept-process-output p 0.1)
    (set-process-query-on-exit-flag p nil)
    (condition-case e
        (progn (kill-buffer buf) :killed-silently)
      (error (cons :errored (car e))))))
"##,
    );
}

#[test]
fn div_cx19_cl_letf_generalized_places() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((lst (list 1 2 3))
       (vec [10 20 30])
       (ht (make-hash-table)))
  (puthash 'key 'orig ht)
  (cl-letf (((car lst) 99)
            ((aref vec 1) 88)
            ((gethash 'key ht) 77))
    (list lst vec (gethash 'key ht)))
  (list lst vec (gethash 'key ht)))
"##,
    );
}

#[test]
fn div_cx19_print_length_zero_print_level_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((print-length 0) (print-level 0))
  (list (prin1-to-string '(1 2 3))
        (prin1-to-string '((1) (2) (3)))
        (prin1-to-string '())))
"##,
    );
}

#[test]
fn div_cx19_hash_table_weakness_behavior() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :weakness 'key :test 'eq)))
  (let ((obj (cons 1 2)))
    (puthash obj :val ht)
    (let ((count-before (hash-table-count ht)))
      (setq obj nil)
      (garbage-collect)
      (list count-before (hash-table-count ht)))))
"##,
    );
}

#[test]
fn div_cx19_prin1_string_with_eight_bit_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((raw (decode-coding-string (unibyte-string 200 201 65) 'utf-8))
       (p (prin1-to-string raw)))
  (list p (length p) (equal (car (read-from-string p)) raw)))
"##,
    );
}

#[test]
fn div_cx19_sort_edge_comparators() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (sort (copy-sequence '(3 1 4 1 5 9 2 6)) (lambda (a b) (< a b)))
      (sort (copy-sequence "cba") #'string<)
      (sort (copy-sequence '((3 . :c) (1 . :a) (2 . :b)))
            (lambda (x y) (< (car x) (car y))))
      (sort (copy-sequence '("bbb" "aaa" "ccc")) #'string<))
"##,
    );
}

#[test]
fn div_cx19_window_config_register_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((cfg (current-window-configuration)))
      (set-register ?w cfg)
      (prog1 (list (window-configuration-p (get-register ?w))
                   (eq cfg (get-register ?w)))
        (set-register ?w nil)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx19_cl_lexical_closure_over_dolist_var() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lexical-binding t))
  (mapcar #'funcall
          (let (acc)
            (dolist (x '(:a :b :c) (nreverse acc))
              (push (lambda () x) acc)))))
"##,
    );
}

#[test]
fn div_cx19_overlay_evaporate_undo_text_prop_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "0123456789")
  (put-text-property 1 5 'face 'bold)
  (let ((ov (make-overlay 3 7))) (overlay-put ov 'face 'italic) (overlay-put ov 'evaporate t))
  (undo-boundary)
  (delete-region 2 8)
  (let ((after (list (buffer-string) (length (overlays-at 1)) (text-properties-at 1))))
    (undo)
    (list after (buffer-string) (length (overlays-at 3)) (text-properties-at 1))))
"##,
    );
}

#[test]
fn div_cx19_coding_system_get_flags_designation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-get 'utf-8 :flags)
      (coding-system-get 'utf-8 :designation)
      (coding-system-get 'emacs-mule :flags))
"##,
    );
}

#[test]
fn div_cx19_unwind_protect_no_body_only_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (ran)
  (condition-case e
      (unwind-protect
          (error "boom")
        (setq ran :cleanup))
    (error (cons (car e) ran))))
"##,
    );
}

#[test]
fn div_cx19_process_stderr_capture_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((err (generate-new-buffer " *neo-cx19-err*")))
    (call-process "sh" nil (list t err) nil "-c" "echo café-err 1>&2; echo ok-out")
    (prog1 (list (buffer-string)
                 (with-current-buffer err (buffer-string)))
      (kill-buffer err))))
"##,
    );
}

#[test]
fn div_cx19_char_table_subtype_and_purpose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'syntax-table nil)))
  (list (char-table-p ct)
        (char-table-subtype ct)
        (eq (char-table-subtype ct) 'syntax-table)))
"##,
    );
}

#[test]
fn div_cx19_format_escape_backslash_in_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%s" "café\\nworld")
      (format "%S" "café\n\t")
      (length (format "%S" "café\n")))
"##,
    );
}

#[test]
fn div_cx19_set_process_filter_then_get_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((f (lambda (proc msg) nil))
       (p (make-process :name "neo-cx19-fg" :command '("true"))))
  (set-process-filter p f)
  (prog1 (eq (process-filter p) f)
    (delete-process p)))
"##,
    );
}

#[test]
fn div_cx19_decode_encode_string_unicode_supplementary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s (string #x1f600 #x10000 #x3042))
       (enc (encode-coding-string s 'utf-8))
       (dec (decode-coding-string enc 'utf-8)))
  (list (length s) (string-bytes s)
        (length enc) (length dec)
        (equal s dec)
        (append dec nil)))
"##,
    );
}

#[test]
fn div_cx19_read_print_circle_nested_shared_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((v (vector 1 2 3))
       (s (list v v (vector v v)))
       (print-circle t))
  (string-match "#1=" (prin1-to-string s)))
"##,
    );
}

#[test]
fn div_cx19_cl_coerce_vector_to_list_back_to_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((v [1 2 3])
       (l (cl-coerce v 'list))
       (back (cl-coerce l 'vector)))
  (list l back (equalp v back)))
"##,
    );
}

#[test]
fn div_cx19_buffer_modification_hooks_inhibit_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (log)
  (with-temp-buffer
    (add-hook 'before-change-functions (lambda (b e) (push :before log)) nil t)
    (add-hook 'after-change-functions (lambda (b e l) (push :after log)) nil t)
    (insert "a")
    (let ((with-hooks (length log)))
      (setq log nil)
      (let ((inhibit-modification-hooks t))
        (insert "b")
        (delete-region 1 2)
        (goto-char 1)
        (insert "c"))
      (list with-hooks (length log) (buffer-string)))))
"##,
    );
}
