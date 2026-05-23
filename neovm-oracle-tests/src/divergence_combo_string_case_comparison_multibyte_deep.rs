//! Deep combo: string manipulation + case conversion + comparison + multibyte.
//! Tests string operations with case folding, locale, and Unicode.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_upcase_downcase_buffer_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (upcase \"hello world\")\n\
         (downcase \"HELLO WORLD\")\n\
         (upcase-initials \"hello world foo\")\n\
         (capitalize \"hello world foo\")))",
    );
}

#[test]
fn deficiency_string_equal_case_fold() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (string-equal \"Hello\" \"hello\")\n\
         (string-equal-ignore-case \"Hello\" \"hello\")\n\
         (compare-strings \"Hello\" 0 nil \"hello\" 0 nil t)\n\
         (compare-strings \"Hello\" 0 nil \"hello\" 0 nil nil)))",
    );
}

#[test]
fn deficiency_string_collation_lessp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (string-lessp \"abc\" \"abd\")\n\
         (string-lessp \"abc\" \"abc\")\n\
         (string-version-lessp \"file2\" \"file10\")\n\
         (string-version-lessp \"file10\" \"file2\")))",
    );
}

#[test]
fn deficiency_string_pad_truncate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (string-pad \"hi\" 10)\n\
         (string-pad \"hi\" 10 ?-)\n\
         (string-pad \"hello\" 3)\n\
         (truncate-string-to-width \"hello world\" 8)\n\
         (truncate-string-to-width \"hello world\" 8 nil nil t)))",
    );
}

#[test]
fn deficiency_string_replace_in_region_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((s \"foo bar foo baz foo\"))\n\
         (list (string-replace \"foo\" \"FOO\" s)\n\
         (string-replace \"foo\" \"\" s)\n\
         (replace-regexp-in-string \"fo+\" \"X\" s))))",
    );
}

#[test]
fn deficiency_string_split_trim_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (split-string \"  a  b  c  \" \"\\\\s-+\" t)\n\
         (split-string \"a,b,,c\" \",\")\n\
         (split-string \"a,b,,c\" \",\" t)\n\
         (string-trim \"  hello  \")\n\
         (string-trim-left \"xxxhello\" \"x+\")\n\
         (string-trim-right \"helloxxx\" \"x+\")))",
    );
}

#[test]
fn deficiency_string_multibyte_case_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (upcase \"caf\\u00e9\")\n\
         (downcase \"CAF\\u00c9\")\n\
         (capitalize \"hello caf\\u00e9 world\")\n\
         (upcase-initials \"hello caf\\u00e9 world\")))",
    );
}

#[test]
fn deficiency_string_search_from_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((s \"abcabcabc\"))\n\
         (list (string-search \"abc\" s)\n\
         (string-search \"abc\" s 1)\n\
         (string-search \"abc\" s 3)\n\
         (string-search \"abc\" s 6)\n\
         (string-search \"abc\" s 7)\n\
         (string-search \"xyz\" s))))",
    );
}

#[test]
fn deficiency_string_reverse_and_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (reverse \"hello\")\n\
         (string-to-multibyte \"abc\")\n\
         (string-to-unibyte \"abc\")\n\
         (length (string-to-list \"abc\\u00e9\"))\n\
         (apply 'string (string-to-list \"ABC\"))))",
    );
}

#[test]
fn deficiency_string_format_with_all_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (format \"%s %S\" 'symbol '(a b c))\n\
         (format \"%.3f\" 3.14)\n\
         (format \"%b\" 10)\n\
         (format \"%d\" most-positive-fixnum)\n\
         (format \"%%d=%d\" 42)))",
    );
}
