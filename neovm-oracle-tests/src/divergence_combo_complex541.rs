/// Batch 541: string-ao, string-as-unibyte, string-as-multibyte, encode-coding.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx541_string_as_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "abc"))
  (string-as-unibyte s))
"##,
    );
}

#[test]
fn div_cx541_string_as_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s (string-as-unibyte "abc")))
  (string-as-multibyte s))
"##,
    );
}

#[test]
fn div_cx541_string_to_unibyte_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "abc"))
  (string-to-unibyte s))
"##,
    );
}

#[test]
fn div_cx541_string_to_multibyte_round() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s (string-to-unibyte "abc")))
  (string-to-multibyte s))
"##,
    );
}

#[test]
fn div_cx541_encode_coding_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (encode-coding-string "abc" 'utf-8)
      (string-bytes (encode-coding-string "abc" 'utf-8)))
"##,
    );
}

#[test]
fn div_cx541_decode_coding_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((enc (encode-coding-string "hello" 'utf-8)))
  (decode-coding-string enc 'utf-8))
"##,
    );
}

#[test]
fn div_cx541_encode_coding_region_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc")
  (encode-coding-region (point-min) (point-max) 'utf-8)
  (buffer-size))
"##,
    );
}

#[test]
fn div_cx541_decode_coding_region_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc")
  (encode-coding-region (point-min) (point-max) 'utf-8)
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (buffer-string))
"##,
    );
}

#[test]
fn div_cx541_multibyte_string_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (multibyte-string-p "abc")
      (multibyte-string-p (string-to-multibyte "abc"))
      (multibyte-string-p (string-as-multibyte "abc")))
"##,
    );
}

#[test]
fn div_cx541_string_make_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "abc"))
  (string-multibyte-p (string-make-multibyte s)))
"##,
    );
}

#[test]
fn div_cx541_string_unibyte_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "abc"))
  (list (string-unibyte-p s)
        (string-unibyte-p (string-to-unibyte s))))
"##,
    );
}

#[test]
fn div_cx541_unibyte_to_multibyte_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (unibyte-char-to-multibyte 65)
      (unibyte-char-to-multibyte 200))
"##,
    );
}

#[test]
fn div_cx541_multibyte_to_unibyte_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (multibyte-char-to-unibyte 200)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx541_string_bytes_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "cafe世界"))
  (list (string-bytes s) (length s)))
"##,
    );
}

#[test]
fn div_cx541_string_width_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "cafe世界"))
  (list (string-width s) (length s)))
"##,
    );
}
