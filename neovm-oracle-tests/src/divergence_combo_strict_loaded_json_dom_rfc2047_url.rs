//! Strict combo oracle probes, batch 36: complex encoding/parsing loaded
//! libraries via assert_oracle_parity_with_load — json.el (encode/read),
//! dom.el (dom-by-tag/attr/strings), rfc2047.el (MIME header decode/encode),
//! and url.el (url-generic-parse-url).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_h3_json_encode_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (json-encode '((a . 1) (b . "str")))
      (json-encode '((arr . [1 2 3]) (b . t) (c . :null)))
      (json-read-from-string "{\"a\":1,\"b\":\"x\"}")
      (json-read-from-string "[1,2,3]")
      (json-encode '((unicode . "café日本"))))
"##,
        &["emacs-lisp/json.el"],
    );
}

#[test]
fn div_h3_dom_traversal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((d '(html nil
              (head nil (title nil "Hi"))
              (body nil (p ((class . "x")) "text") (p nil "more")))))
  (list (dom-tag d)
        (length (dom-by-tag d 'p))
        (dom-attr (car (dom-by-tag d 'p)) 'class)
        (dom-strings d)
        (length (dom-children d))))
"##,
        &["dom.el"],
    );
}

#[test]
fn div_h3_rfc2047_decode_and_ascii_encode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (rfc2047-decode-string "=?utf-8?B?aGVsbG8=?=")
      (rfc2047-decode-string "=?utf-8?Q?h=C3=A9llo?=")
      (rfc2047-decode-string "plain text only")
      (rfc2047-encode-string "hello"))
"##,
        &["mail/rfc2047.el"],
    );
}

#[test]
fn div_h3_rfc2047_encode_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK "=?utf-8?Q?h=C3=A9llo?="
    // Neomacs:   ERR (error "Invalid data for rfc2047 encoding: héllo")
    // rfc2047-encode-string fails on a multibyte string in Neomacs (an
    // underlying primitive that rfc2047.el relies on diverges); ASCII
    // encode and all decode forms agree. Running GNU's rfc2047.el under
    // assert_oracle_parity_with_load.
    assert_oracle_parity_with_load(
        r##"
(rfc2047-encode-string "héllo")
"##,
        &["mail/rfc2047.el"],
    );
}

#[test]
fn div_h3_url_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(let ((u (url-generic-parse-url "https://user:pw@host:8080/path?q=1#frag")))
  (list (aref u 0)
        (aref u 1)
        (aref u 3)
        (aref u 4)
        (url-host u)))
"##,
        &["url/url.el"],
    );
}

#[test]
fn div_h3_json_array_objects_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (json-read-from-string "[{\"x\":[1,2,{\"y\":3}]},4]")
      (json-encode '((nested . ((deep . ((deeper . 42))))))))
"##,
        &["emacs-lisp/json.el"],
    );
}
