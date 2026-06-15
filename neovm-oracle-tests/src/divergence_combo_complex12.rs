//! Complex combo batch 12 — font/fontset info, match-data format, char-table
//! parent inheritance, window-point+marker, looking-at/looking-back, silent
//! modifications, max Unicode codepoint, circular hash print, cl-labels mutual
//! recursion, buffer-local lifecycle.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx12_font_info_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((fi (font-info (face-attribute 'default :font))))
      (list (vectorp fi) (> (length fi) 5)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx12_fontset_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (stringp (frame-parameter nil 'font))
      (fboundp 'fontset-info)
      (fboundp 'query-fontset))
"##,
    );
}

#[test]
fn div_cx12_match_data_vector_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (string-match "\\(.\\)\\(.\\)\\(.\\)" "abcdef")
  (let ((md (match-data)))
    (list (vectorp md) (length md)
          (aref md 0) (aref md 1) (aref md 2) (aref md 3))))
"##,
    );
}

#[test]
fn div_cx12_char_table_parent_inherit_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-char-table 'cx12 nil)) (c1 (make-char-table 'cx12 nil)) (c2 (make-char-table 'cx12 nil)))
  (aset p ?a :parent-a)
  (aset p ?b :parent-b)
  (set-char-table-parent c1 p)
  (set-char-table-parent c2 c1)
  (aset c1 ?a :child1-a)
  (list (aref c2 ?a) (aref c2 ?b) (aref c1 ?a) (aref p ?a)
        (char-table-parent c2) (char-table-parent c1)))
"##,
    );
}

#[test]
fn div_cx12_window_point_marker_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *neo-cx12-wp*")))
  (with-current-buffer buf (insert "0123456789"))
  (set-window-buffer (selected-window) buf)
  (set-window-point (selected-window) 5)
  (let ((wp (window-point)))
    (let ((m (point-marker)))
      (save-window-excursion
        (set-window-point (selected-window) 8))
      (prog1 (list wp (window-point) (marker-position m))
        (set-window-buffer (selected-window) (get-buffer-create "*scratch*"))
        (kill-buffer buf))))
"##,
    );
}

#[test]
fn div_cx12_looking_at_looking_back_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello world foo")
  (goto-char 7)
  (list (looking-at "world")
        (looking-back "hello " 1)
        (progn (looking-at "world") (match-end 0))
        (looking-at "foo")))
"##,
    );
}

#[test]
fn div_cx12_with_silent_modifications() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (let (hooks-fired)
    (add-hook 'after-change-functions (lambda (b e l) (push :fired hooks-fired)) nil t)
    (with-silent-modifications
      (insert "X"))
    (list (buffer-modified-p) hooks-fired)))
"##,
    );
}

#[test]
fn div_cx12_max_unicode_codepoint() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((max #x10FFFF))
  (list (char-to-string max)
        (length (char-to-string max))
        (string-bytes (char-to-string max))
        (aref (char-to-string max) 0)
        (characterp max)))
"##,
    );
}

#[test]
fn div_cx12_circular_hash_table_print() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((ht (make-hash-table :test 'eq))
       (print-circle t))
  (puthash 'self ht ht)
  (string-match "#s(hash-table" (prin1-to-string ht)))
"##,
    );
}

#[test]
fn div_cx12_cl_labels_mutual_recursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-labels ((even-p (n) (if (= n 0) t (odd-p (1- n))))
            (odd-p (n) (if (= n 0) nil (even-p (1- n)))))
  (list (even-p 10) (odd-p 7) (even-p 0) (odd-p 0)))
"##,
    );
}

#[test]
fn div_cx12_buffer_local_lifecycle_default_kill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defvar neo-cx12-bl 'global)
  (let ((results nil))
    (with-temp-buffer
      (setq-local neo-cx12-bl :local)
      (push (default-value 'neo-cx12-bl) results)
      (push neo-cx12-bl results)
      (setq-default neo-cx12-bl :new-default)
      (push (default-value 'neo-cx12-bl) results))
    (push (default-value 'neo-cx12-bl) results)
    (nreverse results)))
"##,
    );
}

#[test]
fn div_cx12_cl_coerce_chain_vector_list_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((v [97 98 233])
       (l (cl-coerce v 'list))
       (s (cl-coerce l 'string)))
  (list l s (append s nil)))
"##,
    );
}

#[test]
fn div_cx12_set_match_data_restore_after_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (with-temp-buffer
    (insert "hello world")
    (string-match "world" "the world is here")
    (let ((md (match-data)))
      (set-match-data md)
      (list (match-beginning 0) (match-end 0)
            (match-string 0)
            (save-match-data
              (re-search-forward "hello" nil t)
              (match-string 0))
            (progn (set-match-data md)
                   (match-string 0))))))
"##,
    );
}

#[test]
fn div_cx12_overlay_keymap_lookup_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((m (make-sparse-keymap)))
  (define-key m "x" 'overlay-action)
  (with-temp-buffer
    (insert "hello")
    (let ((ov (make-overlay 2 4)))
      (overlay-put ov 'keymap m)
      (goto-char 3)
      (list (get-char-property (point) 'keymap)
            (local-key-binding "x")))))
"##,
    );
}

#[test]
fn div_cx12_field_property_beginning_of_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "line1\nAAAABBBB\nline3\n")
  (put-text-property 7 11 'field 'field-a)
  (put-text-property 11 15 'field 'field-b)
  (goto-char 9)
  (list (line-beginning-position)
        (line-beginning-position t)
        (field-beginning (point) t)
        (field-end (point) t)))
"##,
    );
}

#[test]
fn div_cx12_string_match_p_vs_string_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-match-p "café" "le café here")
      (progn (string-match-p "café" "le café here") (match-data))
      (string-match-p "[a-zé]+" "héllo"))
"##,
    );
}

#[test]
fn div_cx12_read_without_circle_circular() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((read-circle nil))
  (condition-case e (read-from-string "#1=(a . #1#)") (error (cons 'errored (car e)))))
"##,
    );
}

#[test]
fn div_cx12_unwind_protpect_gc_cons_threshold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((orig gc-cons-threshold))
  (unwind-protect
      (progn (setq gc-cons-threshold most-positive-fixnum)
             (list gc-cons-threshold (> gc-cons-threshold orig)))
    (setq gc-cons-threshold orig)))
"##,
    );
}

#[test]
fn div_cx12_process_coding_system_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx12-pc" :command '("echo" "x"))))
  (prog1 (list (consp (process-coding-system p))
               (car (process-coding-system p))
               (cdr (process-coding-system p)))
    (delete-process p)))
"##,
    );
}

#[test]
fn div_cx12_text_property_sticky_with_rear_nonsticky() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "AAAAABBBBB")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 6 10 'face 'italic)
  (put-text-property 5 6 'rear-nonsticky '(face))
  (goto-char 5)
  (insert "X")
  (list (get-text-property 5 'face)
        (get-text-property 6 'face)))
"##,
    );
}

#[test]
fn div_cx12_format_percent_s_of_overlay_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello")
  (let ((ov (make-overlay 2 4))
        (m (set-marker (make-marker) 3)))
    (list (format "%s" (overlay-start ov))
          (format "%s" (marker-position m))
          (length (format "%s" ov)))))
"##,
    );
}
