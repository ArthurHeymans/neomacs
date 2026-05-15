//! Oracle parity tests for GNU `url/url-util.el` URL utility semantics.
//!
//! GNU `url-util.el` implements percent encoding/decoding, query parsing, and
//! query construction in Elisp.  These tests pin exact public behavior around
//! newline handling, key normalization, empty values, and per-URI-component
//! allowed character masks.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_url_unhex_string_newlines_plus_and_invalid_escapes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-unhex-string nil)
   (url-unhex-string "a%20b%2Bc")
   (url-unhex-string "line%0Afeed%0Dcarriage")
   (url-unhex-string "line%0Afeed%0Dcarriage" t)
   (url-unhex-string "plus+is+literal")
   (url-unhex-string "%zz%4G%")))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_url_hexify_string_default_utf8_and_allowed_masks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-hexify-string "AZaz09-_.~")
   (url-hexify-string "a b+c&d=e")
   (url-hexify-string "snowman ☃")
   (url-hexify-string "a/b?c=d&e" url-path-allowed-chars)
   (url-hexify-string "a/b?c=d&e" url-query-allowed-chars)
   (url-hexify-string "a/b?c=d&e" url-query-key-value-allowed-chars)
   (url-hexify-string "%already" url--query-key-value-preserved-chars)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_url_parse_query_string_grouping_and_downcase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-parse-query-string "a=1&b=two;c=3")
   (url-parse-query-string "A=1&a=2" t)
   (url-parse-query-string "empty=&missing&repeat=one&repeat=two")
   (url-parse-query-string "plus=a+b&space=a%20b")
   (url-parse-query-string "line=x%0Ay" nil nil)
   (url-parse-query-string "line=x%0Ay" nil t)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_url_build_query_string_empty_values_and_separators() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-build-query-string '((key1 val1)
                             (key2 "two words")
                             (key3 "a&b" "c=d")
                             (key4)
                             (key5 "")))
   (url-build-query-string '((key1 val1)
                             (key2 "two words")
                             (key3 "a&b" "c=d")
                             (key4)
                             (key5 "")) t)
   (url-build-query-string '((key4) (key5 "")) nil t)
   (url-build-query-string '((:keyword value) ("string key" "string value")))
   (url-build-query-string '((percent "%already") (slash "a/b")))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
