//! Complex combo batch 114 — `concat` / `mapconcat` / `split-string` /
//! `string-join` edge cases, with separator variations, strings as input,
//! and property preservation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx114_concat_various_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (concat "alpha" "beta" "gamma")
      (concat "x" " " "y" " " "z")
      (concat)
      (concat "pre: " (number-to-string 42) " :post")
      (apply #'concat '("a" "b" "c"))
      (apply #'concat "start" '("a" "b" "c"))
      (mapconcat #'identity '("a" "b" "c") "-")
      (mapconcat (lambda (n) (number-to-string n)) '(1 2 3) ",")
      (mapconcat #'number-to-string [10 20 30] "/"))
"##,
    );
}

#[test]
fn div_cx114_split_string_with_various_separators() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (split-string "a,b,c,d" ",")
      (split-string "a, b, c, d" ", ?")
      (split-string "a  b   c" "[ \t]+")
      (split-string "a-b-c-d" "-")
      (split-string "a:b:c" ":" t)
      (split-string "" ",")
      (split-string "single")
      (split-string "a,b,c," ",")
      (split-string ",a,b" ","))
"##,
    );
}

#[test]
fn div_cx114_string_join_with_separator_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-join '("a" "b" "c") ",")
      (string-join '("a" "b" "c") " ")
      (string-join '("a" "b" "c") "")
      (string-join '("single"))
      (string-join '())
      (string-join '("a" "b" "c") " -> "))
"##,
    );
}

#[test]
fn div_cx114_string_trim_variants() {
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
    );
}

#[test]
fn div_cx114_string_pad_with_spaces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%20s" "hi")
      (format "%-20s|" "hi")
      (format "%10s|%10s" "a" "b")
      (string-pad "hello" 10)
      (string-pad "hello" 10 ?-)
      (string-pad "hello" 3)
      (string-pad "hello" 5))
"##,
    );
}

#[test]
fn div_cx114_string_replace_with_subgroups() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (replace-regexp-in-string "[0-9]+" "#" "abc 123 def 456 ghi 789")
      (replace-regexp-in-string "\\(\\w+\\) \\(\\w+\\)" "\\2 \\1" "alpha beta")
      (replace-regexp-in-string " +" "_" "a  b   c    d")
      (replace-regexp-in-string "[aeiou]" "*" "alphabet" t)
      (replace-regexp-in-string "\\b\\w+\\b" (lambda (m) (upcase m)) "alpha beta")))
"##,
    );
}

#[test]
fn div_cx114_subst_char_in_string_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (subst-char-in-string ?a ?X "banana")
      (subst-char-in-string ?a ?X "BANANA")
      (subst-char-in-string ?- ?_ "snake-case-var")
      (subst-char-in-string ?\s ?- "with spaces")))
"##,
    );
}

#[test]
fn div_cx114_string_to_multibyte_and_back_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café 世界"))
  (list s
        (multibyte-string-p s)
        (string-bytes s)
        (string-make-unibyte s)
        (multibyte-string-p (string-make-unibyte s))
        (string-make-multibyte (string-make-unibyte s))
        (equal s (string-make-multibyte (string-make-unibyte s)))))
"##,
    );
}

#[test]
fn div_cx114_format_with_special_specifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%s" "hello")
      (format "%S" "hello")
      (format "%S" '(1 "two" 3))
      (format "%S" [1 2 3])
      (format "%S" 'symbol)
      (format "%S" ?A)
      (format "%c" 65)
      (format "%d" 42)
      (format "%x" 255)
      (format "%o" 64)
      (format "%b" 10)
      (format "%e" 3.14)
      (format "%f" 3.14)
      (format "%g" 0.0001)
      (format "%%")
      (format "%5d" 42)
      (format "%-5d|" 42)
      (format "%05d" 42)
      (format "%+d" 42))
"##,
    );
}

#[test]
fn div_cx114_string_with_text_properties_concat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s1 (propertize "alpha" 'face 'bold))
       (s2 "beta")
       (s3 (propertize "gamma" 'face 'italic))
       (combined (concat s1 "-" s2 "-" s3)))
  (list combined
        (text-properties-at 0 combined)
        (text-properties-at 5 combined)
        (text-properties-at 6 combined)
        (text-properties-at 11 combined)
        (text-properties-at 12 combined)))
"##,
    );
}

#[test]
fn div_cx114_compare_strings_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (compare-strings "abc" 0 3 "abc" 0 3)
      (compare-strings "abc" 0 3 "abd" 0 3)
      (compare-strings "abc" 0 3 "abcd" 0 4)
      (compare-strings "abc" 0 3 "ab" 0 2)
      (compare-strings "abc" 0 3 "ABC" 0 3 t)
      (compare-strings "abc" 0 3 "ABC" 0 3 nil))
"##,
    );
}

#[test]
fn div_cx114_concat_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((parts (mapcar (lambda (n) (format "part-%d" n)) (number-sequence 1 5)))
       (joined (string-join parts "\n")))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert joined)
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 20)
      (let ((state (list (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}
