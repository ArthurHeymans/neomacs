//! Complex combo batch 174 — `string` / `char-table` interactions with
//! syntax-table lookups, `char-syntax` matrix, category set operations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx174_char_syntax_matrix_per_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-syntax c)))
        '(?a ?A ?0 ?9 ?_ ?-
          ?\( ?\) ?\[ ?\] ?\{ ?\}
          ?\" ?\' ?\` ?\; ?, ?.
          ?\\ ?? ?! ?# ?$ ?% ?& ?* ?+ ?< ?> ?@ ?/ ?| ?~ ?^))
"##,
    );
}

#[test]
fn div_cx174_string_to_syntax_class_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (mapcar (lambda (s) (list s (string-to-syntax s)))
            '("w" "_" "." "(" ")" "\"" ";" "'" "\\" "/" "<" ">" "@" "!"))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx174_syntax_class_to_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (syntax-class-to-char (string-to-syntax "w"))
          (syntax-class-to-char (string-to-syntax "_"))
          (syntax-class-to-char (string-to-syntax "."))
          (syntax-class-to-char (string-to-syntax "\""))
          (syntax-class-to-char (string-to-syntax "("))
          (syntax-class-to-char (string-to-syntax ")"))
          (syntax-class-to-char (string-to-syntax ";")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx174_category_set_manipulation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((ct (make-category-table)))
      (define-category ?a "cat-a" ct)
      (define-category ?b "cat-b" ct)
      (define-category ?c "cat-c" ct)
      (modify-category-entry ?x ?a ct)
      (modify-category-entry ?x ?b ct)
      (modify-category-entry ?y ?c ct)
      (let ((x-set (char-category-set ?x ct))
            (y-set (char-category-set ?y ct)))
        (list (category-set-mnemonics x-set)
              (category-set-mnemonics y-set)
              (category-docstring ?a ct)
              (category-docstring ?c ct))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx174_char_table_range_query_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'neo-cx174-test :default)))
  (set-char-table-range ct nil :all-default)
  (set-char-table-range ct '(?a . ?z) :lowercase)
  (set-char-table-range ct '(?A . ?Z) :uppercase)
  (set-char-table-range ct '(?0 . ?9) :digit)
  (set-char-table-range ct ?_ :underscore)
  (set-char-table-range ct '(?\( . ?\)) :paren-family)
  (list (char-table-range ct nil)
        (char-table-range ct ?a)
        (char-table-range ct ?A)
        (char-table-range ct ?5)
        (char-table-range ct ?_)
        (char-table-range ct ?\()
        (char-table-range ct ?!)
        (aref ct ?a)))
"##,
    );
}

#[test]
fn div_cx174_with_syntax_table_local_scope() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((before-at (char-syntax ?@))
      (before-dash (char-syntax ?-)))
  (with-syntax-table (make-syntax-table)
    (modify-syntax-entry ?@ "w")
    (modify-syntax-entry ?- "_")
    (list (char-syntax ?@)
          (char-syntax ?-)
          before-at
          before-dash)))
"##,
    );
}

#[test]
fn div_cx174_char_table_parent_inheritance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((parent (make-char-table 'neo-cx174-parent :parent-default))
       (child (make-char-table 'neo-cx174-child :child-default)))
  (aset parent ?a :in-parent)
  (aset parent ?b :in-parent)
  (set-char-table-parent child parent)
  (aset child ?a :in-child)
  (list (aref child ?a)
        (aref child ?b)
        (aref child ?z)
        (aref parent ?a)
        (char-table-p child)
        (eq (char-table-parent child) parent)))
"##,
    );
}

#[test]
fn div_cx174_map_char_table_collect_counts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (make-char-table 'neo-cx174-map nil)))
  (set-char-table-range ct '(?a . ?e) :vowel-or-low)
  (set-char-table-range ct '(?A . ?E) :vowel-or-up)
  (set-char-table-range ct ?x :special)
  (let (counts)
    (map-char-table
     (lambda (key val)
       (when val
         (let ((entry (assq val counts)))
           (if entry (setcdr entry (1+ (cdr entry)))
             (push (cons val 1) counts)))))
     ct)
    (sort counts (lambda (a b)
                   (string< (symbol-name (car a)) (symbol-name (car b)))))))
"##,
    );
}

#[test]
fn div_cx174_char_table_extra_slots() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((ct (make-char-table 'neo-cx174-extra nil 4)))
      (set-char-table-extra-slot ct 0 :slot-0)
      (set-char-table-extra-slot ct 1 :slot-1)
      (set-char-table-extra-slot ct 2 99)
      (set-char-table-extra-slot ct 3 '("list" "of" "data"))
      (list (char-table-extra-slot ct 0)
            (char-table-extra-slot ct 1)
            (char-table-extra-slot ct 2)
            (char-table-extra-slot ct 3)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx174_syntax_table_per_buffer_switch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf-a (get-buffer-create " *neo-cx174-a*"))
      (buf-b (get-buffer-create " *neo-cx174-b*")))
  (with-current-buffer buf-a
    (set-syntax-table (make-syntax-table))
    (modify-syntax-entry ?@ "w"))
  (with-current-buffer buf-b
    (set-syntax-table (make-syntax-table))
    (modify-syntax-entry ?@ "."))
  (let ((at-a (with-current-buffer buf-a (char-syntax ?@)))
        (at-b (with-current-buffer buf-b (char-syntax ?@))))
    (kill-buffer buf-a)
    (kill-buffer buf-b)
    (list at-a at-b)))
"##,
    );
}

#[test]
fn div_cx174_syntax_char_table_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (set-syntax-table (make-syntax-table))
  (modify-syntax-entry ?_ "w")
  (modify-syntax-entry ?- "w")
  (insert "var_name-1 (call_arg x) end_token")
  (put-text-property 1 5 'face 'bold)
  (let ((m (set-marker (make-marker) 12))
        (ov (make-overlay 4 24)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 30)
    (goto-char 1)
    (forward-word 2)
    (forward-comment 1)
    (let ((state (list (point)
                       (char-syntax (char-after))
                       (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (nth 0 (syntax-ppss (point)))
                       (nth 3 (syntax-ppss (point)))
                       (text-properties-at 1))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1))))))
"##,
    );
}
