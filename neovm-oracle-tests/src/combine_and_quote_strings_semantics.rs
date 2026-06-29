//! Oracle parity tests for GNU string quoting helpers.
//!
//! GNU implements `combine-and-quote-strings` and
//! `split-string-and-unquote` in `lisp/subr.el` using `mapconcat`,
//! `replace-regexp-in-string`, `split-string`, and `read-from-string`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_combine_and_quote_basic_space_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases
       '(("alpha" "beta" "gamma")
         ("alpha beta" "gamma")
         ("alpha" "" "gamma")
         ("a\"b" "c\\d" "e f"))))
  (mapcar
   (lambda (strings)
     (let ((combined (combine-and-quote-strings strings)))
       (list strings combined (split-string-and-unquote combined))))
   cases))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK (((\"alpha\" \"beta\" \"gamma\") \"alpha beta gamma\" (\"alpha\" \"beta\" \"gamma\")) ((\"alpha beta\" \"gamma\") \"\\\"alpha beta\\\" gamma\" (\"alpha beta\" \"gamma\")) ((\"alpha\" \"\" \"gamma\") \"alpha  gamma\" (\"alpha\" \"gamma\")) ((\"a\\\"b\" \"c\\\\d\" \"e f\") \"\\\"a\\\\\\\"b\\\" \\\"c\\\\\\\\d\\\" \\\"e f\\\"\" (\"a\\\"b\" \"c\\\\d\" \"e f\")))""#
        ]],
    );
}

#[test]
fn oracle_combine_and_quote_custom_literal_separators() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases
       '((("a,b" "c" "d,e") ",")
         (("a::b" "c" "d::e") "::")
         (("a|b" "c d" "e\"f") "|"))))
  (mapcar
   (lambda (case)
     (let* ((strings (car case))
            (sep (cadr case))
            (combined (combine-and-quote-strings strings sep)))
       (list strings sep combined (split-string-and-unquote combined (regexp-quote sep)))))
   cases))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK (((\"a,b\" \"c\" \"d,e\") \",\" \"\\\"a,b\\\",c,\\\"d,e\\\"\" (\"a,b\" \"c\" \"d,e\")) ((\"a::b\" \"c\" \"d::e\") \"::\" \"\\\"a::b\\\"::c::\\\"d::e\\\"\" (\"a::b\" \"c\" \"d::e\")) ((\"a|b\" \"c d\" \"e\\\"f\") \"|\" \"\\\"a|b\\\"|c d|\\\"e\\\\\\\"f\\\"\" (\"a|b\" \"c d\" \"e\\\"f\")))""#
        ]],
    );
}

#[test]
fn oracle_split_string_and_unquote_without_quotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (split-string-and-unquote "alpha beta  gamma")
 (split-string-and-unquote "alpha,beta,,gamma" ",")
 (split-string-and-unquote ",alpha,beta," ",")
 (split-string-and-unquote "alpha::beta::::gamma" "::+"))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK ((\"alpha\" \"beta\" \"gamma\") (\"alpha\" \"beta\" \"gamma\") (\"alpha\" \"beta\") (\"alpha\" \"beta\" \"gamma\"))""#
        ]],
    );
}

#[test]
fn oracle_split_string_and_unquote_quoted_segments() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (split-string-and-unquote "alpha \"beta gamma\" delta")
 (split-string-and-unquote "\"alpha beta\" \"gamma\\\"delta\"")
 (split-string-and-unquote "pre,\"a,b\",post" ",")
 (split-string-and-unquote "\"\" middle \"\"" "\\s-+"))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK ((\"alpha\" \"beta gamma\" \"delta\") (\"alpha beta\" \"gamma\\\"delta\") (\"pre\" \"a,b\" \"post\") (\"\" \"middle\" \"\"))""#
        ]],
    );
}

#[test]
fn oracle_split_string_and_unquote_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((cases
       (list
        (lambda () (split-string-and-unquote "\"unterminated"))
        (lambda () (split-string-and-unquote "alpha \"bad\\q\" beta"))
        (lambda () (combine-and-quote-strings '("a" 1 "b"))))))
  (mapcar
   (lambda (fn)
     (condition-case err
         (funcall fn)
       (error (list (car err) (cadr err)))))
   cases))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK ((end-of-file nil) (\"alpha\" \"badq\" \"beta\") (wrong-type-argument stringp))""#
        ]],
    );
}

#[test]
fn oracle_combine_and_quote_preserves_string_properties_observably() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((a (copy-sequence "alpha beta"))
      (b (copy-sequence "gamma")))
  (put-text-property 0 5 'face 'bold a)
  (put-text-property 0 5 'face 'italic b)
  (let ((combined (combine-and-quote-strings (list a b))))
    (list combined
          (substring-no-properties combined)
          (text-properties-at 1 combined)
          (text-properties-at 14 combined))))
"#;

    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK (#(\"\\\"alpha beta\\\" gamma\" 1 6 (face bold) 13 18 (face italic)) \"\\\"alpha beta\\\" gamma\" (face bold) (face italic))""#
        ]],
    );
}
