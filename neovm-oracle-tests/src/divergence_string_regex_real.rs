//! Divergence tests: real string/regex behavioral differences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_re_search_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert \"2024-01-15 and 2025-12-31\")
  (goto-char 1)
  (re-search-forward \"\\\\([0-9]+\\\\)-\\\\([0-9]+\\\\)-\\\\([0-9]+\\\\)\")
  (let ((m1 (list (match-string 1) (match-string 2) (match-string 3))))
    (re-search-forward \"\\\\([0-9]+\\\\)-\\\\([0-9]+\\\\)-\\\\([0-9]+\\\\)\")
    (list m1
          (list (match-string 1) (match-string 2) (match-string 3))
          (match-beginning 0)
          (match-end 0)))) ",
        expect_test::expect![[
            r#""2024-01-15 and 2025-12-31OK ((\"2024\" \"01\" \"15\") (\"2025\" \"12\" \"31\") 16 26)""#
        ]],
    );
}

#[test]
fn divergence_string_multibyte_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((s \"Hello \\u4e16\\u754c\"))
  (list (length s)
        (string-bytes s)
        (substring s 0 5)
        (substring s 5)
        (aref s 5)
        (aref s 6)
        (= (aref s 5) #x4e16)
        (= (aref s 6) #x754c))) ",
        expect_test::expect![[r#""OK (8 12 \"Hello\" \" 世界\" 32 19990 nil nil)""#]],
    );
}

#[test]
fn divergence_regex_anchored() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert \"abc\\ndef\\nghi\")
  (goto-char 1)
  (let ((matches nil))
    (while (re-search-forward \"^\\\\([a-z]+\\\\)\" nil t)
      (push (match-string 1) matches))
    (nreverse matches))) ",
        expect_test::expect![[r#""abc\ndef\nghiOK (\"abc\" \"def\" \"ghi\")""#]],
    );
}

#[test]
fn divergence_case_fold_replacement() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((case-fold-search t))
  (list (string-match \"HELLO\" \"hello world\")
        (string-match \"hello\" \"HELLO WORLD\")
        (replace-regexp-in-string \"foo\" \"bar\" \"Foo BAR FOO\"))) ",
        expect_test::expect![[r#""OK (0 0 \"Bar BAR BAR\")""#]],
    );
}

#[test]
fn divergence_string_split_join_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let* ((parts '(\"foo\" \"bar\" \"baz\"))
        (joined (string-join parts \", \"))
        (split (split-string joined \", \")))
  (list joined
        split
        (equal parts split)
        (length split))) ",
        expect_test::expect![[r#""OK (\"foo, bar, baz\" (\"foo\" \"bar\" \"baz\") t 3)""#]],
    );
}

#[test]
fn divergence_unicode_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((s \"caf\\u00e9\"))
  (list (length s)
        (string-bytes s)
        (upcase s)
        (downcase s)
        (string= (downcase (upcase s)) s)
        (string< \"a\" \"b\")
        (string< \"a\" \"\\u00e9\"))) ",
        expect_test::expect![[r#""OK (4 5 \"CAFÉ\" \"café\" t t t)""#]],
    );
}

#[test]
fn divergence_regex_word_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"foo-bar baz\"))
  (list (string-match \"\\\\bfoo\\\\b\" text)
        (string-match \"\\\\bbar\\\\b\" text)
        (string-match \"\\\\bbaz\\\\b\" text)
        (string-match \"\\\\bfoo-bar\\\\b\" text))) ",
        expect_test::expect![[r#""OK (0 4 8 0)""#]],
    );
}

#[test]
fn divergence_string_replace_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (replace-regexp-in-string \"o\" \"0\" \"foo boo moo\")
  (replace-regexp-in-string \"[0-9]+\" \"NUM\" \"abc 123 def 456 ghi\")
  (replace-regexp-in-string \"^\" \"PREFIX: \" \"hello\")
  (replace-regexp-in-string \"$\" \" SUFFIX\" \"hello\")) ",
        expect_test::expect![[
            r#""OK (\"f00 b00 m00\" \"abc NUM def NUM ghi\" \"PREFIX: hello\" \"hello SUFFIX\")""#
        ]],
    );
}

#[test]
fn divergence_string_format_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let* ((obj '(1 \"two\" three (nested)))
        (str (format \"%S\" obj)))
  (list str
        (equal (read-from-string str)
               (cons obj (length str)))
        (format \"%.2f\" 3.14159)
        (format \"%04d\" 7)
        (format \"%-10s|\" \"hi\"))) ",
        expect_test::expect![[
            r#""OK (\"(1 \\\"two\\\" three (nested))\" t \"3.14\" \"0007\" \"hi        |\")""#
        ]],
    );
}

#[test]
fn divergence_rx_composition_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((re (rx bos (group (one-or-more (any \"a-z\")))
              \"-\" (group (one-or-more digit)) eos)))
  (list re
        (string-match re \"hello-42\")
        (match-string 1 \"hello-42\")
        (match-string 2 \"hello-42\")
        (string-match re \"Hello-42\"))) ",
        expect_test::expect![[
            r#""OK (\"\\\\`\\\\([a-z]+\\\\)-\\\\([[:digit:]]+\\\\)\\\\'\" 0 \"hello\" \"42\" 0)""#
        ]],
    );
}
