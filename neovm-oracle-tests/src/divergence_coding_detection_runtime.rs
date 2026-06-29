//! Coding-system detection parity: detect-coding-string/region on ascii,
//! BOM-prefixed utf-16 decode, alias/eol queries; plus two detection gaps
//! (utf-8 region, latin-1) where neomacs returns 'undecided.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn coding_priority_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (coding-system-p 'iso-latin-1) (coding-system-p 'chinese-gbk)
        (coding-system-aliases 'utf-8) (coding-system-eol-type 'undecided))"##,
        expect_test::expect![[
            r#""OK (t t (utf-8 mule-utf-8 cp65001) [undecided-unix undecided-dos undecided-mac])""#
        ]],
    );
}

#[test]
fn decode_with_bom_utf16() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (decode-coding-string (string-to-unibyte (concat (unibyte-string 255 254) (encode-coding-string "Hi" 'utf-16le))) 'utf-16)
        (decode-coding-string (string-to-unibyte (concat (unibyte-string 254 255) (encode-coding-string "Hi" 'utf-16be))) 'utf-16))"##,
        expect_test::expect![[r#""OK (\"Hi\" \"Hi\")""#]],
    );
}

#[test]
fn detect_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(detect-coding-string "plain ascii text" t)"##,
        expect_test::expect![[r#""OK undecided""#]],
    );
}

#[test]
#[ignore = "DIVERGENCE: detect-coding-region returns 'undecided for UTF-8 multibyte content where GNU detects 'utf-8 (same root cause as detect-coding-string)."]
fn divergence_detect_coding_region_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (encode-coding-string "日本語" 'utf-8))
  (list (detect-coding-region (point-min) (point-max) t)))"##,
        expect_test::expect![[r#""OK (utf-8)""#]],
    );
}

#[test]
#[ignore = "DIVERGENCE: detect-coding-string returns 'undecided for Latin-1 high-byte content where GNU detects 'iso-latin-1."]
fn divergence_detect_coding_latin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (detect-coding-string (unibyte-string 72 233 108 108 111) t)
        (coding-system-p (detect-coding-string (unibyte-string 65 66 67) t)))"##,
        expect_test::expect![[r#""OK (iso-latin-1 t)""#]],
    );
}
