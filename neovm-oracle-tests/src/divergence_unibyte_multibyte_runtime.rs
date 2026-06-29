//! Unibyte vs multibyte result parity: multibyte-string-p of base64-decode,
//! string-to/as-unibyte, unibyte buffer-string, decode 'binary, encode utf-8,
//! url-unhex, secure-hash binary, concat/make-string/read/format/mapconcat of
//! unibyte parts. (Core handling is correct; cf. json-serialize and binary
//! process-filter divergences which are multibyte where GNU is unibyte.)

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn base64_decode_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((d (base64-decode-string "SGVsbG8=")))
  (list d (multibyte-string-p d) (length d)))"##,
        expect_test::expect![[r#""OK (\"Hello\" nil 5)""#]],
    );
}

#[test]
fn buffer_string_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert 65 66 67)
  (let ((s (buffer-string))) (list s (multibyte-string-p s) (length s))))"##,
        expect_test::expect![[r#""OK (\"ABC\" nil 3)""#]],
    );
}

#[test]
fn concat_unibyte_parts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((c (concat (unibyte-string 200) (unibyte-string 201))))
  (list (multibyte-string-p c) (length c) (append c nil)))"##,
        expect_test::expect![[r#""OK (nil 2 (200 201))""#]],
    );
}

#[test]
fn decode_binary_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((d (decode-coding-string (unibyte-string 72 105) 'binary)))
  (list d (multibyte-string-p d)))"##,
        expect_test::expect![[r#""OK (\"Hi\" t)""#]],
    );
}

#[test]
fn encode_utf8_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((e (encode-coding-string "café" 'utf-8)))
  (list (multibyte-string-p e) (length e) (string-bytes e)))"##,
        expect_test::expect![[r#""OK (nil 5 5)""#]],
    );
}

#[test]
fn format_unibyte_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((r (format "%s" (unibyte-string 200 201))))
  (list (multibyte-string-p r) (length r)))"##,
        expect_test::expect![[r#""OK (nil 2)""#]],
    );
}

#[test]
fn make_string_highbyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (make-string 3 200))
       (su (make-string 3 200 t)))
  (list (multibyte-string-p s) (multibyte-string-p su) (length su)))"##,
        expect_test::expect![[r#""OK (t t 3)""#]],
    );
}

#[test]
fn mapconcat_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((r (mapconcat #'identity (list (unibyte-string 200) (unibyte-string 201)) "")))
  (list (multibyte-string-p r) (length r)))"##,
        expect_test::expect![[r#""OK (nil 2)""#]],
    );
}

#[test]
fn read_unibyte_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((s (car (read-from-string (prin1-to-string (unibyte-string 200 201))))))
  (list (multibyte-string-p s) (length s) (append s nil)))"##,
        expect_test::expect![[r#""OK (nil 2 (200 201))""#]],
    );
}

#[test]
fn secure_hash_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((h (secure-hash 'md5 "x" nil nil t)))
  (list (multibyte-string-p h) (length h)))"##,
        expect_test::expect![[r#""OK (nil 16)""#]],
    );
}

#[test]
fn string_to_unibyte_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((u (string-to-unibyte "abc")) (a (string-as-unibyte "abc")))
  (list (multibyte-string-p u) (multibyte-string-p a)))"##,
        expect_test::expect![[r#""OK (nil nil)""#]],
    );
}

#[test]
fn url_unhex_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(require 'url-util)
(let ((u (url-unhex-string "%48%69")))
  (list u (multibyte-string-p u)))"##,
        expect_test::expect![[r#""OK (\"Hi\" nil)""#]],
    );
}
