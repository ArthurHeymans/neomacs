//! Complex combo batch 310 — `string` operations ultimate: `string-replace`,
//! `string-trim` variants, `split-string` with TRIM, `string-pad`, `string-limit`,
//! `string-lines`, `string-join` with separators.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx310_string_replace_all_occurrences() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-replace "o" "0" "hello world foo")
      (string-replace "world" "Emacs" "hello world")
      (string-replace "" "X" "abc")
      (string-replace "a" "" "banana")
      (string-replace " " "-" "a b c d e"))
"##,
    )
}

#[test]
fn div_cx310_string_trim_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-trim "   hello   ")
      (string-trim-left "   hello")
      (string-trim-right "hello   ")
      (string-trim "\n\nhello\n\n")
      (string-trim "xxhelloxx" "x+" "x+")
      (string-trim "  hello  " "[ ]+" "[ ]+")
      (string-trim-left "-----hello" "-+")
      (string-trim-right "hello-----" "-+"))
"##,
    )
}

#[test]
fn div_cx310_split_string_with_trim_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (split-string "  hello  world  " "[ \t]+" t)
      (split-string ",a,b,c," "," t)
      (split-string "  hello  world  " "[ \t]+" nil)
      (split-string "alpha,beta,gamma," "," t)
      (split-string "a\nb\nc\n" "\n" t)
      (split-string "" ",")
      (split-string "no delimiters here" ",")
      (split-string "alpha beta gamma"))
"##,
    )
}

#[test]
fn div_cx310_string_pad_and_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (string-pad "hello" 10)
          (string-pad "hello" 10 ?-)
          (string-pad "hello" 3)
          (string-pad "hello" 5)
          (string-pad "hello" 0)
          (when (fboundp 'string-limit) (string-limit "hello world" 5)))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx310_string_lines_and_join() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (when (fboundp 'string-lines) (string-lines "line1\nline2\nline3"))
          (when (fboundp 'string-lines) (length (string-lines "a\nb\nc\nd")))
          (string-join '("alpha" "beta" "gamma") ",")
          (string-join '("alpha" "beta" "gamma") " -> ")
          (string-join '("single"))
          (string-join '()))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx310_string_distance_levenshtein_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-distance "kitten" "sitting")
      (string-distance "flaw" "lawn")
      (string-distance "same" "same")
      (string-distance "" "")
      (string-distance "a" "")
      (string-distance "" "a")
      (string-distance "abc" "xyz")
      (string-distance "café" "cafe"))
"##,
    )
}

#[test]
fn div_cx310_string_version_lessp_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-version-lessp "file2.txt" "file10.txt")
      (string-version-lessp "file10.txt" "file2.txt")
      (string-version-lessp "file1.0" "file1.1")
      (string-version-lessp "file1.10" "file1.2")
      (string-version-lessp "1" "2")
      (string-version-lessp "2" "10")
      (string-version-lessp "v1.0.0" "v1.0.1")
      (string-version-lessp "v1.0.9" "v1.0.10"))
"##,
    )
}

#[test]
fn div_cx310_string_prefix_suffix_predicates_full() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-prefix-p "hello" "hello world")
      (string-prefix-p "world" "hello world")
      (string-prefix-p "HELLO" "hello world" t)
      (string-prefix-p "HELLO" "hello world" nil)
      (string-suffix-p "world" "hello world")
      (string-suffix-p "hello" "hello world")
      (string-suffix-p "WORLD" "hello world" t))
"##,
    )
}

#[test]
fn div_cx310_compare_strings_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (compare-strings "abc" nil nil "abc" nil nil)
      (compare-strings "abc" nil nil "abd" nil nil)
      (compare-strings "abc" nil nil "abcd" nil nil)
      (compare-strings "abc" nil nil "ab" nil nil)
      (compare-strings "abc" nil nil "ABC" nil nil t)
      (compare-strings "abc" nil nil "ABC" nil nil nil)
      (compare-strings "abc" 0 3 "xabc" 1 4)
      (compare-strings "abc" 0 2 "abx" 0 2))
"##,
    )
}

#[test]
fn div_cx310_string_ops_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s1 "  hello world  ")
       (s2 "alpha,beta,gamma")
       (parts (split-string s2 "," t))
       (trimmed (string-trim s1)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "%s | %s | %S" trimmed s2 parts))
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 20)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 28)
      (let ((state (list trimmed s2 parts
                         (string-distance "kitten" "sitting")
                         (string-version-lessp "file2" "file10")
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    )
}
