//! Strict combo oracle probes, batch 312: URL encoding. url-hexify-string,
//! url-unhex-string, url-build-query-string, url-encode-url, and
//! url-parse-query-string.
//! Uses assert_oracle_parity_expect format.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_url_hexify_unhex_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'url-util)
(list (url-hexify-string "hello world & foo=bar")
      (url-hexify-string "café 日本語")
      (url-unhex-string "hello%20world%26foo%3Dbar")
      (url-unhex-string "caf%C3%A9%20%E6%97%A5%E6%9C%AC%E8%AA%9E")
      (url-unhex-string (url-hexify-string "test & roundtrip!")))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_url_build_parse_query_string_encode_url() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'url-util)
(list (url-build-query-string '(("a" "1") ("b" "x y")))
      (url-build-query-string '(("k" "v") ("special" "!@#")))
      (url-parse-query-string "a=1&b=2&c=3")
      (url-parse-query-string "name=John+Doe&city=New+York")
      (url-encode-url "http://example.com/path with spaces?a=b c"))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_url_domain_file_url_components() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(require 'url-util)
(list (url-type (url-generic-parse-url "https://host.example.com:8080/path?q=1"))
      (url-host (url-generic-parse-url "https://host.example.com:8080/path"))
      (url-filename (url-generic-parse-url "https://h.com/a/b/c.txt"))
      (url-port (url-generic-parse-url "https://h.com:8443/"))
      (url-encode-url "file:///tmp/a b.txt"))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
