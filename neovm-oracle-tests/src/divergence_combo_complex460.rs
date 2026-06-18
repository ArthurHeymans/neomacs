/// Batch 460: input-method, quail, charset, category, case-table deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx460_input_method_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (current-input-method)
      (input-method-name)
      (input-method-after-insert-chunk-hook))"##,
    );
}

#[test]
fn div_cx460_quail_define_package() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'quail)
  (list (fboundp 'quail-define-package)
        (fboundp 'quail-define-rules)))"##,
    );
}

#[test]
fn div_cx460_charset_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc")
  (list (charset-after 1) (charset-after 2)))"##,
    );
}

#[test]
fn div_cx460_category_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (copy-category-table)))
  (define-category ?x "test" ct)
  (modify-category-entry ?a ?x ct)
  (list (char-category-set ?a ct)
        (char-category-set ?b ct)))"##,
    );
}

#[test]
fn div_cx460_case_table_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (copy-case-table)))
  (list (case-table-p ct)
        (char-table-p ct)))"##,
    );
}

#[test]
fn div_cx460_case_table_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (copy-case-table)))
  (list (aref ct ?a) (aref ct ?A)))"##,
    );
}

#[test]
fn div_cx460_char_table_prototype() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (make-char-table 'syntax-table ?w)))
  (list (char-table-prototype ct)
        (aref ct 0)))"##,
    );
}

#[test]
fn div_cx460_char_table_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((parent (make-char-table 'syntax-table ?w))
      (child (make-char-table 'syntax-table ?x)))
  (set-char-table-parent child parent)
  (list (char-table-parent child)
        (aref child ?a)))"##,
    );
}

#[test]
fn div_cx460_syntax_table_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((st (make-syntax-table (syntax-table))))
  (list (char-table-p st)
        (syntax-table-p st)))"##,
    );
}

#[test]
fn div_cx460_string_to_syntax_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(list (string-to-syntax "!" ) (string-to-syntax "!b"))"##);
}

#[test]
fn div_cx460_syntax_class_to_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (syntax-class-to-char 0)
  (error (car e)))"##,
    );
}

#[test]
fn div_cx460_unibyte_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (unibyte-string 65 66 67)
      (unibyte-string 200 201)
      (length (unibyte-string 128 129)))"##,
    );
}

#[test]
fn div_cx460_multibyte_string_p_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s1 "abc")
      (s2 "cafe")
      (s3 (unibyte-string 65 66)))
  (list (multibyte-string-p s1)
        (multibyte-string-p s2)
        (multibyte-string-p s3)))"##,
    );
}

#[test]
fn div_cx460_string_bytes_vs_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "cafe世界"))
  (list (string-bytes s) (length s)))"##,
    );
}

#[test]
fn div_cx460_warehouse_string_make() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-string 5 ?a)
      (string ?h ?e ?l ?l ?o)
      (concat (string ?w ?o) (string ?r ?l ?d)))"##,
    );
}
