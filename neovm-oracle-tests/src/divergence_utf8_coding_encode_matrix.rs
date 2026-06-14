//! UTF-8 / multibyte *coding-system encode matrix* (all GNU coding systems).
//!
//! One focused #[test] per coding system: encode "café世界" (built via char
//! codes to avoid Rust string-escape pitfalls) and compare bytes. Unsupported
//! codings return nil in Neomacs vs a byte string in GNU (Theme 9, encode).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_utf8_csenc_adobe_standard_encoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'adobe-standard-encoding) nil)");
}

#[test]
fn div_utf8_csenc_chinese_big5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'chinese-big5) nil)");
}

#[test]
fn div_utf8_csenc_chinese_big5_hkscs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'chinese-big5-hkscs) nil)");
}

#[test]
fn div_utf8_csenc_chinese_gb18030() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'chinese-gb18030) nil)");
}

#[test]
fn div_utf8_csenc_chinese_gbk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'chinese-gbk) nil)");
}

#[test]
fn div_utf8_csenc_chinese_hz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'chinese-hz) nil)");
}

#[test]
fn div_utf8_csenc_chinese_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'chinese-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_compound_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'compound-text) nil)");
}

#[test]
fn div_utf8_csenc_compound_text_with_extensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'compound-text-with-extensions) nil)");
}

#[test]
fn div_utf8_csenc_cp1125() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp1125) nil)");
}

#[test]
fn div_utf8_csenc_cp437() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp437) nil)");
}

#[test]
fn div_utf8_csenc_cp737() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp737) nil)");
}

#[test]
fn div_utf8_csenc_cp775() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp775) nil)");
}

#[test]
fn div_utf8_csenc_cp850() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp850) nil)");
}

#[test]
fn div_utf8_csenc_cp851() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp851) nil)");
}

#[test]
fn div_utf8_csenc_cp852() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp852) nil)");
}

#[test]
fn div_utf8_csenc_cp855() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp855) nil)");
}

#[test]
fn div_utf8_csenc_cp857() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp857) nil)");
}

#[test]
fn div_utf8_csenc_cp858() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp858) nil)");
}

#[test]
fn div_utf8_csenc_cp860() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp860) nil)");
}

#[test]
fn div_utf8_csenc_cp861() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp861) nil)");
}

#[test]
fn div_utf8_csenc_cp862() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp862) nil)");
}

#[test]
fn div_utf8_csenc_cp863() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp863) nil)");
}

#[test]
fn div_utf8_csenc_cp865() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp865) nil)");
}

#[test]
fn div_utf8_csenc_cp866() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp866) nil)");
}

#[test]
fn div_utf8_csenc_cp869() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp869) nil)");
}

#[test]
fn div_utf8_csenc_cp874() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cp874) nil)");
}

#[test]
fn div_utf8_csenc_ctext_no_compositions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ctext-no-compositions) nil)");
}

#[test]
fn div_utf8_csenc_cyrillic_alternativnyj() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cyrillic-alternativnyj) nil)");
}

#[test]
fn div_utf8_csenc_cyrillic_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cyrillic-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_cyrillic_koi8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'cyrillic-koi8) nil)");
}

#[test]
fn div_utf8_csenc_ebcdic_uk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ebcdic-uk) nil)");
}

#[test]
fn div_utf8_csenc_ebcdic_us() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ebcdic-us) nil)");
}

#[test]
fn div_utf8_csenc_emacs_mule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'emacs-mule) nil)");
}

#[test]
fn div_utf8_csenc_euc_jis_2004() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'euc-jis-2004) nil)");
}

#[test]
fn div_utf8_csenc_euc_tw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'euc-tw) nil)");
}

#[test]
fn div_utf8_csenc_eucjp_ms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'eucjp-ms) nil)");
}

#[test]
fn div_utf8_csenc_georgian_academy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'georgian-academy) nil)");
}

#[test]
fn div_utf8_csenc_georgian_ps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'georgian-ps) nil)");
}

#[test]
fn div_utf8_csenc_greek_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'greek-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_hebrew_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'hebrew-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_hp_roman8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'hp-roman8) nil)");
}

#[test]
fn div_utf8_csenc_ibm038() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm038) nil)");
}

#[test]
fn div_utf8_csenc_ibm1047() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm1047) nil)");
}

#[test]
fn div_utf8_csenc_ibm256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm256) nil)");
}

#[test]
fn div_utf8_csenc_ibm273() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm273) nil)");
}

#[test]
fn div_utf8_csenc_ibm274() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm274) nil)");
}

#[test]
fn div_utf8_csenc_ibm275() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm275) nil)");
}

#[test]
fn div_utf8_csenc_ibm277() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm277) nil)");
}

#[test]
fn div_utf8_csenc_ibm278() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm278) nil)");
}

#[test]
fn div_utf8_csenc_ibm280() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm280) nil)");
}

#[test]
fn div_utf8_csenc_ibm281() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm281) nil)");
}

#[test]
fn div_utf8_csenc_ibm284() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm284) nil)");
}

#[test]
fn div_utf8_csenc_ibm285() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm285) nil)");
}

#[test]
fn div_utf8_csenc_ibm290() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm290) nil)");
}

#[test]
fn div_utf8_csenc_ibm297() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'ibm297) nil)");
}

#[test]
fn div_utf8_csenc_in_is13194_devanagari() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'in-is13194-devanagari) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_7bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-7bit) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_7bit_lock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-7bit-lock) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_7bit_lock_ss2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-7bit-lock-ss2) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_7bit_ss2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-7bit-ss2) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_8bit_ss2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-8bit-ss2) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_cn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-cn) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_cn_ext() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-cn-ext) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_jp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-jp) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_jp_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-jp-2) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_jp_2004() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-jp-2004) nil)");
}

#[test]
fn div_utf8_csenc_iso_2022_kr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-2022-kr) nil)");
}

#[test]
fn div_utf8_csenc_iso_8859_11() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-8859-11) nil)");
}

#[test]
fn div_utf8_csenc_iso_8859_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-8859-6) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-1) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_10() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-10) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-2) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-3) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-4) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-5) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-6) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-7) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-8) nil)");
}

#[test]
fn div_utf8_csenc_iso_latin_9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'iso-latin-9) nil)");
}

#[test]
fn div_utf8_csenc_japanese_cp932() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'japanese-cp932) nil)");
}

#[test]
fn div_utf8_csenc_japanese_iso_7bit_1978_irv() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'japanese-iso-7bit-1978-irv) nil)");
}

#[test]
fn div_utf8_csenc_japanese_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'japanese-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_japanese_shift_jis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'japanese-shift-jis) nil)");
}

#[test]
fn div_utf8_csenc_japanese_shift_jis_2004() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'japanese-shift-jis-2004) nil)");
}

#[test]
fn div_utf8_csenc_koi8_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'koi8-t) nil)");
}

#[test]
fn div_utf8_csenc_koi8_u() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'koi8-u) nil)");
}

#[test]
fn div_utf8_csenc_korean_cp949() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'korean-cp949) nil)");
}

#[test]
fn div_utf8_csenc_korean_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'korean-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_lao() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'lao) nil)");
}

#[test]
fn div_utf8_csenc_mac_roman() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'mac-roman) nil)");
}

#[test]
fn div_utf8_csenc_mik() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'mik) nil)");
}

#[test]
fn div_utf8_csenc_next() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'next) nil)");
}

#[test]
fn div_utf8_csenc_no_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'no-conversion) nil)");
}

#[test]
fn div_utf8_csenc_no_conversion_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'no-conversion-multibyte) nil)");
}

#[test]
fn div_utf8_csenc_prefer_utf_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'prefer-utf-8) nil)");
}

#[test]
fn div_utf8_csenc_pt154() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'pt154) nil)");
}

#[test]
fn div_utf8_csenc_raw_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'raw-text) nil)");
}

#[test]
fn div_utf8_csenc_thai_tis620() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'thai-tis620) nil)");
}

#[test]
fn div_utf8_csenc_tibetan_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'tibetan-iso-8bit) nil)");
}

#[test]
fn div_utf8_csenc_undecided() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'undecided) nil)");
}

#[test]
fn div_utf8_csenc_us_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'us-ascii) nil)");
}

#[test]
fn div_utf8_csenc_utf_16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-16) nil)");
}

#[test]
fn div_utf8_csenc_utf_16be() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-16be) nil)");
}

#[test]
fn div_utf8_csenc_utf_16be_with_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-16be-with-signature) nil)");
}

#[test]
fn div_utf8_csenc_utf_16le() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-16le) nil)");
}

#[test]
fn div_utf8_csenc_utf_16le_with_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-16le-with-signature) nil)");
}

#[test]
fn div_utf8_csenc_utf_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-7) nil)");
}

#[test]
fn div_utf8_csenc_utf_7_imap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-7-imap) nil)");
}

#[test]
fn div_utf8_csenc_utf_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-8) nil)");
}

#[test]
fn div_utf8_csenc_utf_8_auto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-8-auto) nil)");
}

#[test]
fn div_utf8_csenc_utf_8_emacs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-8-emacs) nil)");
}

#[test]
fn div_utf8_csenc_utf_8_with_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'utf-8-with-signature) nil)");
}

#[test]
fn div_utf8_csenc_vietnamese_viqr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'vietnamese-viqr) nil)");
}

#[test]
fn div_utf8_csenc_vietnamese_viscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'vietnamese-viscii) nil)");
}

#[test]
fn div_utf8_csenc_vietnamese_vscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'vietnamese-vscii) nil)");
}

#[test]
fn div_utf8_csenc_windows_1250() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1250) nil)");
}

#[test]
fn div_utf8_csenc_windows_1251() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1251) nil)");
}

#[test]
fn div_utf8_csenc_windows_1252() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1252) nil)");
}

#[test]
fn div_utf8_csenc_windows_1253() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1253) nil)");
}

#[test]
fn div_utf8_csenc_windows_1254() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1254) nil)");
}

#[test]
fn div_utf8_csenc_windows_1255() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1255) nil)");
}

#[test]
fn div_utf8_csenc_windows_1256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1256) nil)");
}

#[test]
fn div_utf8_csenc_windows_1257() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1257) nil)");
}

#[test]
fn div_utf8_csenc_windows_1258() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(append (encode-coding-string (string 99 97 102 233 19990 30028) 'windows-1258) nil)");
}
