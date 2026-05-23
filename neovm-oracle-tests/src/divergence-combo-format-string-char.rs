//! Divergence tests: format + string manipulation + char property combos.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_format_complex_specs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (format "%d + %d = %d" 1 2 3)
        (format "%05d" 42)
        (format "%-10s|" "hello")
        (format "%10s|" "hello")
        (format "%.3f" 3.14159)
        (format "%x" 255)
        (format "%o" 8)
        (format "%c" 65)
        (format "%s has %d items" "list" 5)
        (format "%%100"))) "#,
    );
}

#[test]
fn divergence_string_props_after_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (let* ((s1 "hello world")
         (s2 (format "%s %d" s1 42))
         (s3 (concat s1 " " (number-to-string 99))))
    (list (string= s2 "hello world 42")
          (string= s3 "hello world 99")
          (length s1)
          (= (length s1) 11)
          (length s2)
          (= (length s2) 14)
          (length s3)
          (= (length s3) 14)))) "#,
    );
}

#[test]
fn divergence_string_multibyte_concat_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (let ((a "abc")
        (b "\xc3\xa9\xc3\xa0"))
    (let ((c (concat a b))
          (d (concat b a)))
      (list (length a)
            (= (length a) 3)
            (length b)
            (= (length b) 2)
            (length c)
            (= (length c) 5)
            (length d)
            (= (length d) 5)
            (substring c 3 5)
            (string= (substring c 3 5) b))))) "#,
    );
}

#[test]
fn divergence_replace_regex_in_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (replace-regexp-in-string "[0-9]+" "NUM" "abc123def456")
        (string= (replace-regexp-in-string "[0-9]+" "NUM" "abc123def456")
                 "abcNUMdefNUM")
        (replace-regexp-in-string "[a-z]+" "\\&-X" "hello world")
        (string= (replace-regexp-in-string "[a-z]+" "\\&-X" "hello world")
                 "hello-X world-X")
        (replace-regexp-in-string "\\([a-z]+\\)" "(\\1)" "abc def")
        (string= (replace-regexp-in-string "\\([a-z]+\\)" "(\\1)" "abc def")
                 "(abc) (def)")
        (replace-regexp-in-string "x" "y" "no match")
        (string= (replace-regexp-in-string "x" "y" "no match")
                 "no match"))) "#,
    );
}

#[test]
fn divergence_string_search_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (string-match "world" "hello world")
        (= (string-match "world" "hello world") 6)
        (string-match "xyz" "hello world")
        (null (string-match "xyz" "hello world"))
        (string-match "world" "hello world" 6)
        (= (string-match "world" "hello world" 6) 6)
        (string-match "world" "hello world" 7)
        (null (string-match "world" "hello world" 7))
        (match-string 0 "hello world")
        (string= (match-string 0 "hello world") "world"))) "#,
    );
}

#[test]
fn divergence_split_string_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (split-string "  a  b  c  " " +" t)
        (equal (split-string "  a  b  c  " " +" t) '("a" "b" "c"))
        (split-string "a,b,,c" ",")
        (equal (split-string "a,b,,c" ",") '("a" "b" "" "c"))
        (split-string "a,b,,c" "," t)
        (equal (split-string "a,b,,c" "," t) '("a" "b" "c"))
        (split-string "" ",")
        (equal (split-string "" ",") '("")))) "#,
    );
}

#[test]
fn divergence_string_case_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (upcase "hello World")
        (string= (upcase "hello World") "HELLO WORLD")
        (downcase "HELLO World")
        (string= (downcase "HELLO World") "hello world")
        (capitalize "hello world")
        (string= (capitalize "hello world") "Hello World")
        (upcase-initials "hello world foo")
        (string= (upcase-initials "hello world foo") "Hello World Foo"))) "#,
    );
}

#[test]
fn divergence_string_pad_trim() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (string-pad "hello" 10)
        (string= (string-pad "hello" 10) "hello     ")
        (string-pad "hello" 3)
        (string= (string-pad "hello" 3) "hello")
        (string-trim "  hello  ")
        (string= (string-trim "  hello  ") "hello")
        (string-trim-left "  hello  ")
        (string= (string-trim-left "  hello  ") "hello  ")
        (string-trim-right "  hello  ")
        (string= (string-trim-right "  hello  ") "  hello"))) "#,
    );
}

#[test]
fn divergence_string_reverse_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (list (string-reverse "abc")
        (string= (string-reverse "abc") "cba")
        (length (string-reverse "\xc3\xa9\xc3\xa0"))
        (= (length (string-reverse "\xc3\xa9\xc3\xa0")) 2)
        (string-reverse "hello")
        (string= (string-reverse "hello") "olleh")
        (string-reverse "")
        (string= (string-reverse "") ""))) "#,
    );
}

#[test]
fn divergence_string_bytes_vs_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (let ((s "\xc3\xa9\xc3\xa0\xc3\xb9"))
    (list (length s)
          (= (length s) 3)
          (string-bytes s)
          (>= (string-bytes s) 3)
          (string-equal s s)
          (string= s s)
          (= (string-bytes "abc") 3)
          (= (length "abc") 3)
          (= (string-bytes s) (* (length s) 2))))) "#,
    );
}
