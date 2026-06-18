/// Batch 485: format-spec deep, replace-regexp-in-string edge, Unicode edge, string edge.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx485_format_spec_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'format-spec)
  (let ((spec (format-spec-make ?a "hello" ?b "world" ?n 42)))
    (list (format-spec "%a-%b" spec)
          (format-spec "%(a%|b%)" (format-spec-make ?a "x" ?b "y"))
          (format-spec "%%a" spec))))
"##,
    );
}

#[test]
fn div_cx485_replace_regexp_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (replace-regexp-in-string "\\(.\\)" "\\1\\1" "abc")
      (replace-regexp-in-string "a" "\\&" "abc")
      (replace-regexp-in-string "a" "\\?" "abc")
      (replace-regexp-in-string "^" "x" "abc"))
"##,
    );
}

#[test]
fn div_cx485_unicode_combining() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-width "a\u0301bc")
      (string-width "cafe\u0301")
      (char-width #x0301)
      (length "a\u0301bc"))
"##,
    );
}

#[test]
fn div_cx485_unicode_smp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-width "\U0001F600")
      (char-width #x1F600)
      (length "\U0001F600")
      (string-bytes "\U0001F600"))
"##,
    );
}

#[test]
fn div_cx485_unicode_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-width "\0\1\2\3")
      (char-width 0)
      (string-width "\t\n\r"))
"##,
    );
}

#[test]
fn div_cx485_string_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (substring "abc" 0 0)
      (substring "abc" 3)
      (substring "abc" -1)
      (concat)
      (string))
"##,
    );
}

#[test]
fn div_cx485_truncate_string_to_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (truncate-string-to-width "hello" 3 nil nil ?.)
      (truncate-string-to-width "hello" 6)
      (truncate-string-to-width "世界" 2)
      (truncate-string-to-width "abc" 5 nil nil t))
"##,
    );
}

#[test]
fn div_cx485_compare_strings_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (compare-strings "abc" 0 3 "abc" 0 3)
      (compare-strings "abc" 0 2 "abd" 0 2)
      (compare-strings "abc" nil nil "ABC" nil nil t))
"##,
    );
}

#[test]
fn div_cx485_assoc_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((al '((a . 1) (b . 2) (c . 3))))
  (list (assoc 'b al)
        (assoc-default 'b al)
        (assoc-default 'b al 'eq)
        (rassoc 1 al)))
"##,
    );
}

#[test]
fn div_cx485_copy_sequence_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (copy-sequence '(1 2 3))
      (copy-sequence [1 2 3])
      (copy-sequence "abc")
      (length (copy-sequence (make-hash-table))))
"##,
    );
}

#[test]
fn div_cx485_cl_subseq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (cl-subseq '(1 2 3 4 5) 1 3)
      (cl-subseq [1 2 3 4] 2)
      (cl-subseq "hello" 1 4))
"##,
    );
}

#[test]
fn div_cx485_map_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(map-vector (lambda (e) (* 2 e)) [1 2 3 4])
"##,
    );
}

#[test]
fn div_cx485_map_car() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(mapcar (lambda (x) (* x 2)) '(1 2 3 4))
"##,
    );
}

#[test]
fn div_cx485_map_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(mapconcat (lambda (x) (format "%d" x)) '(1 2 3) "-")
"##,
    );
}

#[test]
fn div_cx485_map_can() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(mapcan (lambda (x) (list x (* x 2))) '(1 2 3))
"##,
    );
}
