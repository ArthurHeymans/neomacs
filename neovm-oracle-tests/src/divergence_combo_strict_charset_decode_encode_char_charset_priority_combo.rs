//! Strict combo oracle probes, batch 177: charset operations. decode-char /
//! encode-char round-trips for iso-8859-1 / ascii / unicode, char-charset over
//! latin / CJK / combining, charsetp, charset-priority-list, and coding-system-
//! charset-list.
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_charset_decode_encode_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (decode-char 'iso-8859-1 233)
      (encode-char ?é 'iso-8859-1)
      (decode-char 'ascii 65)
      (encode-char ?A 'ascii)
      (decode-char 'unicode 0x00e9)
      (encode-char ?é 'unicode)
      (decode-char 'eight-bit 200)
      (encode-char (decode-char 'iso-8859-1 241) 'iso-8859-1)
      (decode-char 'iso-8859-1 65))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_char_charset_p_charset_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (char-charset ?a)
      (char-charset ?é)
      (char-charset ?日)
      (char-charset ? )
      (char-charset 127)
      (charsetp 'ascii)
      (charsetp 'unicode)
      (charsetp 'iso-8859-1)
      (charsetp 'not-a-charset)
      (charsetp 42)
      (consp (charset-priority-list))
      (memq 'ascii (charset-priority-list))
      (memq 'unicode (charset-priority-list)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_charset_list_dimension_plane() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (charset-dimension 'ascii)
      (charset-dimension 'iso-8859-1)
      (charset-dimension 'unicode)
      (charset-plist 'ascii)
      (charset-plist 'iso-8859-1)
      (consp (charset-list))
      (memq 'ascii (charset-list))
      (memq 'unicode (charset-list))
      (charset-id 'ascii)
      (charset-id 'unicode)
      (list-charset-chars 'ascii))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
