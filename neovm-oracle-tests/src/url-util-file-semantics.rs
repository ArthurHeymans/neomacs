//! Oracle parity tests for additional GNU `url/url-util.el` helper semantics.
//!
//! These cover URL argument parsing, entity escaping, normalization, URL-style
//! filename splitting, extension handling, and display truncation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_url_parse_args_case_quotes_and_missing_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-parse-args "TEXT/html; charset=\"utf-8\"; boundary='abc def'; flag")
   (url-parse-args "TEXT/html; charset=\"utf-8\"; boundary='abc def'; flag" t)
   (url-parse-args "a=1;b=two words; c = \"three four\" ; bare")
   (url-parse-args "broken=\"unterminated tail; next=ok")))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_url_entities_and_normalize_url() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-insert-entities-in-string "a&b<c>d\"e")
   (url-insert-entities-in-string "plain")
   (url-normalize-url "HTTP://Example.COM:80/a/b?x=1#frag")
   (url-normalize-url "http://example.com:81/a/b#frag")
   (url-normalize-url "mailto:USER@example.com#keep")
   (url-normalize-url "www.example.com/path#keep")))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_url_file_directory_nondirectory_and_extension() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-file-directory nil)
   (url-file-nondirectory nil)
   (url-file-directory "/a/b/c.txt?query=1")
   (url-file-nondirectory "/a/b/c.txt?query=1")
   (url-file-directory "/a%2Fb%2Fc.txt")
   (url-file-nondirectory "/a%2Fb%2Fc.txt")
   (url-file-directory "noslash")
   (url-file-nondirectory "noslash")
   (url-file-extension "/a/b/c.tar.gz")
   (url-file-extension "/a/b/c.tar.gz" t)
   (url-file-extension "/a/b/noext")
   (url-file-extension "/a/b/noext" t)
   (url-basepath "/root/file.el")))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_url_truncate_url_for_viewing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'url-util)
  (list
   (url-truncate-url-for-viewing "http://example.com/short" 80)
   (url-truncate-url-for-viewing "http://example.com/path/to/file?query=long" 35)
   (url-truncate-url-for-viewing "http://example.com/a/b/c/d/e/file.txt" 30)
   (url-truncate-url-for-viewing "http://example.com/a/b/c/d/e/file.txt" 18)))
"#;

    assert_oracle_parity(form);
}
