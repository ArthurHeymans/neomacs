/// Batch 520: regexp replace with count, string-trim with predicate, string-join deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx520_regexp_replace_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (replace-regexp-in-string "a" "X" "aaa aaa aaa" nil nil nil 1)
      (replace-regexp-in-string "a" "X" "aaa aaa aaa" nil nil nil 2)
      (replace-regexp-in-string "a" "X" "aaa aaa aaa" nil nil nil 5))
"##,
        expect_test::expect![[r#""OK (\"XX XXX XXX\" \"X XXX XXX\" \"XX XXX\")""#]],
    );
}

#[test]
fn div_cx520_regexp_replace_subexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (replace-regexp-in-string "\\(a\\)\\(b\\)" "\\2\\1" "abcabc")
      (replace-regexp-in-string "\\(a\\)\\(b\\)" "\\2\\1" "abcabc" nil nil nil 1))
"##,
        expect_test::expect![[r#""OK (\"bacbac\" \"bcbac\")""#]],
    );
}

#[test]
fn div_cx520_regexp_replace_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (replace-regexp-in-string "." "X" "abc" nil t)
      (replace-regexp-in-string "." "X" "abc" nil nil))
"##,
        expect_test::expect![[r#""OK (\"XXX\" \"XXX\")""#]],
    );
}

#[test]
fn div_cx520_string_trim_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-trim "  hello  ")
      (string-trim "xxhelloxx" "x+" "x+"))
"##,
        expect_test::expect![[r#""OK (\"hello\" \"hello\")""#]],
    );
}

#[test]
fn div_cx520_string_join_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-join '("a" "b" "c") ", ")
      (string-join '("x") ", ")
      (string-join '()))
"##,
        expect_test::expect![[r#""OK (\"a, b, c\" \"x\" \"\")""#]],
    );
}

#[test]
fn div_cx520_string_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-fill "hello world" 5)
      (string-fill "hello world" 20))
"##,
        expect_test::expect![[r#""OK (\"hello\nworld\" \"hello world\")""#]],
    );
}

#[test]
fn div_cx520_string_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-limit "hello" 3) (string-limit "hello" 3 t))
"##,
        expect_test::expect![[r#""OK (\"hel\" \"llo\")""#]],
    );
}

#[test]
fn div_cx520_string_title_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'subr-x)
  (string-titleize "hello world"))
"##,
        expect_test::expect![[r#""ERR (void-function string-titleize)""#]],
    );
}

#[test]
fn div_cx520_string_ellipsis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'subr-x)
  (string-ellipsis "hello world" 5))
"##,
        expect_test::expect![[r#""ERR (void-function string-ellipsis)""#]],
    );
}

#[test]
fn div_cx520_string_pad_left_right() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-pad "hello" 10)
      (string-pad "hello" 10 nil t)
      (string-pad "hello" 3))
"##,
        expect_test::expect![[r#""OK (\"hello     \" \"     hello\" \"hello\")""#]],
    );
}

#[test]
fn div_cx520_string_truncate_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'subr-x)
  (list (string-truncate-left 5 "hello world")
        (string-truncate-left 20 "short")))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument sequencep 5)""#]],
    );
}

#[test]
fn div_cx520_string_search_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-search "world" "hello world")
      (string-search "xyz" "hello world")
      (string-search "o" "hello world"))
"##,
        expect_test::expect![[r#""OK (6 nil 4)""#]],
    );
}

#[test]
fn div_cx520_string_repeat_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (string-repeat "ab" 3) (string-repeat "x" 0) (string-repeat "" 5))
"##,
        expect_test::expect![[r#""ERR (void-function string-repeat)""#]],
    );
}

#[test]
fn div_cx520_format_field_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (format "%5s" "hi") (format "%-5s" "hi") (format "%05d" 42))
"##,
        expect_test::expect![[r#""OK (\"   hi\" \"hi   \" \"00042\")""#]],
    );
}

#[test]
fn div_cx520_format_decimal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (format "%d" 42) (format "%d" -42) (format "%o" 255) (format "%x" 255))
"##,
        expect_test::expect![[r#""OK (\"42\" \"-42\" \"377\" \"ff\")""#]],
    );
}
