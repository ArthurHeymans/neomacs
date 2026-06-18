/// Batch 469: ring, parse-time, rfc2822, ewoc, tq, mailcap, cookie, base64 deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx469_ring_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ring)
  (let ((r (make-ring 5)))
    (dotimes (i 5) (ring-insert r (* i 10)))
    (list (ring-length r) (ring-ref r 0) (ring-ref r 4))))"##,
    );
}

#[test]
fn div_cx469_ring_insert_extend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ring)
  (let ((r (make-ring 3)))
    (dotimes (i 5) (ring-insert r i))
    (ring-remove r 0)
    (ring-insert-at-beginning r 99)
    (list (ring-length r) (ring-member r 99) (ring-elements r))))"##,
    );
}

#[test]
fn div_cx469_parse_time_rfc2822() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'parse-time)
  (parse-time-string "Mon, 16 Jun 2024 14:30:00 +0000")
)"##,
    );
}

#[test]
fn div_cx469_base64_encode_decode_url() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (base64-encode-string "hello")
      (base64-decode-string (base64-encode-string "hello"))
      (base64url-encode-string "hello")
      (base64url-decode-string (base64url-encode-string "hello")))
"##,
    );
}

#[test]
fn div_cx469_ewoc_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ewoc)
  (with-temp-buffer
    (let ((ewoc (ewoc-create 'identity "header" "footer")))
      (ewoc-enter-first ewoc "item1")
      (ewoc-enter-last ewoc "item2")
      (ewoc-location (ewoc-nth ewoc 0)))))
"##,
    );
}

#[test]
fn div_cx469_tq_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'tq)
  (list (fboundp 'tq-create) (fboundp 'tq-enqueue) (fboundp 'tq-close)))
"##,
    );
}

#[test]
fn div_cx469_mailcap_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'mailcap)
  (list (boundp 'mailcap-mime-data) (fboundp 'mailcap-parse-mailcaps)))
"##,
    );
}

#[test]
fn div_cx469_smtpmail_auth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'smtpmail)
  (list (boundp 'smtpmail-auth-credentials)
        (fboundp 'smtpmail-send-it)))
"##,
    );
}

#[test]
fn div_cx469_url_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'url-parse) (require 'url-util)
  (let ((url (url-generic-parse-url "https://user:pass@host:8080/path?q=1#frag")))
    (list (url-type url) (url-host url) (url-port url)
          (url-user url) (url-password url) (url-filename url))))"##,
    );
}

#[test]
fn div_cx469_url_hexify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'url-util)
  (list (url-hexify-string "hello world")
        (url-unhex-string "hello%20world")
        (url-hexify-string "a\000b")))"##,
    );
}

#[test]
fn div_cx469_netrc_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'netrc)
  (list (fboundp 'netrc-parse) (fboundp 'netrc-machine)))
"##,
    );
}

#[test]
fn div_cx469_rfc2109_cookie() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cookie)
  (list (fboundp 'cookie) (fboundp 'cookie-handle-cookie-line)))
"##,
    );
}

#[test]
fn div_cx469_sort_fkeys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'sort)
  (with-temp-buffer
    (insert "b 2\na 1\nc 3\n")
    (sort-fields 1 (point-min) (point-max))
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx469_sort_numeric() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'sort)
  (with-temp-buffer
    (insert "b 2\na 10\nc 1\n")
    (sort-numeric-fields 2 (point-min) (point-max))
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx469_subr_x_when_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subr-x)
  (list (when-let ((a 1)) (+ a 2))
        (if-let ((a 1)) (+ a 2) 'nope)
        (and-let* ((a 1) (b (+ a 2))) (* a b))))
"##,
    );
}
