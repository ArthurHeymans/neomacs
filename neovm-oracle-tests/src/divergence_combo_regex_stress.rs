//! Divergence tests: complex regex engine stress combinations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_nested_backrefs_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert \"start\\nkey=abc\\nkey=def\\nkey=abc\\nkey=ghi\\nend\")
  (goto-char 1)
  (let ((matches nil))
    (while (re-search-forward \"key=\\\\(\\\\w+\\\\)\" nil t)
      (push (match-string 1) matches))
    (let ((all (nreverse matches)))
      (list all
            (length all)
            (equal all '(\"abc\" \"def\" \"abc\" \"ghi\")))))) ",
        expect_test::expect![[
            r#""start\nkey=abc\nkey=def\nkey=abc\nkey=ghi\nendOK ((\"abc\" \"def\" \"abc\" \"ghi\") 4 t)""#
        ]],
    );
}

#[test]
fn divergence_regex_alternation_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"2024-01-15 14:30:00 user=john action=login ip=10.0.0.1\"))
  (list (string-match \"[0-9]+-[0-9]+-[0-9]+\" text)
        (match-string 0 text)
        (string-match \"user=\\\\([a-z]+\\\\)\" text)
        (match-string 1 text)
        (string-match \"ip=\\\\([0-9.]+\\\\)\" text)
        (match-string 1 text)
        (string-match \"action=\\\\(login\\\\|logout\\\\|error\\\\)\" text)
        (match-string 1 text))) ",
        expect_test::expect![[
            r#""OK (0 \"2024-01-15\" 20 \"john\" 43 \"10.0.0.1\" 30 \"login\")""#
        ]],
    );
}

#[test]
fn divergence_regex_greedy_vs_lazy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((html \"<b>bold1</b> text <b>bold2</b>\"))
  (list (string-match \"<b>\\\\(.*\\\\)</b>\" html)
        (match-string 1 html)
        (string-match \"<b>\\\\([^<]*\\\\)</b>\" html)
        (match-string 1 html)
        (replace-regexp-in-string
         \"<b>\\\\([^<]*\\\\)</b>\" \"[\\\\1]\" html))) ",
        expect_test::expect![[
            r#""OK (0 \"bold1</b> text <b>bold2\" 0 \"bold1\" \"[bold1] text [bold2]\")""#
        ]],
    );
}

#[test]
fn divergence_regex_with_escaped_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"price: $19.99 (tax: $3.80) total: $23.79\"))
  (list (string-match \"\\\\\\\\$\\\\([0-9.]+\\\\)\" text)
        (match-string 1 text)
        (let ((total 0.0))
          (with-temp-buffer
            (insert text)
            (goto-char 1)
            (while (re-search-forward \"\\\\\\\\$\\\\([0-9.]+\\\\)\" nil t)
              (setq total (+ total (string-to-number (match-string 1))))))
          (> total 47.0)
          (< (abs (- total 47.58)) 0.01)))) ",
        expect_test::expect![[r#""OK (nil \"ce: $\" nil)""#]],
    );
}

#[test]
fn divergence_regex_word_constituents() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"foo_bar baz-quux hello.world a1b2c3\"))
  (list (string-match \"\\\\\\\\<foo_bar\\\\\\\\>\" text)
        (string-match \"\\\\\\\\<baz\\\\\\\\>\" text)
        (string-match \"\\\\\\\\<hello\\\\\\\\.world\\\\\\\\>\" text)
        (string-match \"\\\\\\\\<a1b2c3\\\\\\\\>\" text)
        (replace-regexp-in-string
         \"\\\\\\\\<\\\\([a-z0-9_]+\\\\)\\\\\\\\>\" \"[\\\\1]\" text))) ",
        expect_test::expect![[r#""OK (nil nil nil nil \"foo_bar baz-quux hello.world a1b2c3\")""#]],
    );
}

#[test]
fn divergence_regex_syntax_classes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"abc ABC 123 !@# \\t\\n\"))
  (list (string-match \"\\\\ca\" text)
        (string-match \"\\\\cA\" text)
        (string-match \"\\\\cd\" text)
        (string-match \"\\\\cg\" text)
        (string-match \"\\\\cs\" text)
        (string-match \"\\\\c \" text))) ",
        expect_test::expect![[r#""OK (0 nil nil nil nil nil)""#]],
    );
}

#[test]
fn divergence_regex_repeated_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"a1b2c3d4e5\"))
  (list (string-match \"\\\\([a-z]\\\\)+\" text)
        (match-string 0 text)
        (match-string 1 text)
        (string-match \"\\\\([a-z]\\\\([0-9]\\\\)\\\\)+\" text)
        (match-string 0 text)
        (match-string 1 text)
        (match-string 2 text))) ",
        expect_test::expect![[r#""OK (0 \"a\" \"a\" 0 \"a1b2c3d4e5\" \"e5\" \"5\")""#]],
    );
}

#[test]
fn divergence_regex_case_fold_multi_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((case-fold-search t)
        (text \"Hello HELLO hello HeLLo\"))
  (list text
        (replace-regexp-in-string \"hello\" \"world\" text)
        (replace-regexp-in-string \"hello\" 'upcase text)
        (replace-regexp-in-string \"hello\"
          (lambda (m) (concat \"<\" (upcase m) \">\")) text))) ",
        expect_test::expect![[
            r#""OK (\"Hello HELLO hello HeLLo\" \"World WORLD world World\" \"HELLO HELLO HELLO HELLO\" \"<HELLO> <HELLO> <HELLO> <HELLO>\")""#
        ]],
    );
}

#[test]
fn divergence_regex_multiline_dot() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(let ((text \"line1\\nline2\\nline3\"))
  (list (string-match \"line1.*line3\" text)
        (string-match \"line1\" text)
        (string-match \"line3\" text)
        (replace-regexp-in-string \"\\n\" \" | \" text))) ",
        expect_test::expect![[r#""OK (nil 0 12 \"line1 | line2 | line3\")""#]],
    );
}

#[test]
fn divergence_regex_save_match_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert \"AAA match1 BBB match2 CCC\")
  (goto-char 1)
  (re-search-forward \"match\\\\([0-9]\\\\)\")
  (let ((first (match-data t)))
    (save-match-data
      (re-search-forward \"match\\\\([0-9]\\\\)\")
      (let ((second-inner (match-string 1)))
        (set-match-data first)
        (list (match-string 1) second-inner
              (match-beginning 0) (match-end 0)))))) ",
        expect_test::expect![[r#""AAA match1 BBB match2 CCCOK (\"1\" \"2\" 5 11)""#]],
    );
}
