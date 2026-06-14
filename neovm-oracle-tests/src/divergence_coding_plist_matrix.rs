//! Per-coding-system *coding-system-plist* matrix (all GNU coding systems).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cod_plist_adobe_standard_encoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'adobe-standard-encoding)");
}

#[test]
fn div_cod_plist_chinese_big5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'chinese-big5)");
}

#[test]
fn div_cod_plist_chinese_big5_hkscs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'chinese-big5-hkscs)");
}

#[test]
fn div_cod_plist_chinese_gb18030() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'chinese-gb18030)");
}

#[test]
fn div_cod_plist_chinese_gbk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'chinese-gbk)");
}

#[test]
fn div_cod_plist_chinese_hz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'chinese-hz)");
}

#[test]
fn div_cod_plist_chinese_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'chinese-iso-8bit)");
}

#[test]
fn div_cod_plist_compound_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'compound-text)");
}

#[test]
fn div_cod_plist_compound_text_with_extensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'compound-text-with-extensions)");
}

#[test]
fn div_cod_plist_cp1125() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp1125)");
}

#[test]
fn div_cod_plist_cp437() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp437)");
}

#[test]
fn div_cod_plist_cp737() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp737)");
}

#[test]
fn div_cod_plist_cp775() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp775)");
}

#[test]
fn div_cod_plist_cp850() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp850)");
}

#[test]
fn div_cod_plist_cp851() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp851)");
}

#[test]
fn div_cod_plist_cp852() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp852)");
}

#[test]
fn div_cod_plist_cp855() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp855)");
}

#[test]
fn div_cod_plist_cp857() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp857)");
}

#[test]
fn div_cod_plist_cp858() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp858)");
}

#[test]
fn div_cod_plist_cp860() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp860)");
}

#[test]
fn div_cod_plist_cp861() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp861)");
}

#[test]
fn div_cod_plist_cp862() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp862)");
}

#[test]
fn div_cod_plist_cp863() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp863)");
}

#[test]
fn div_cod_plist_cp865() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp865)");
}

#[test]
fn div_cod_plist_cp866() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp866)");
}

#[test]
fn div_cod_plist_cp869() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp869)");
}

#[test]
fn div_cod_plist_cp874() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cp874)");
}

#[test]
fn div_cod_plist_ctext_no_compositions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ctext-no-compositions)");
}

#[test]
fn div_cod_plist_cyrillic_alternativnyj() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cyrillic-alternativnyj)");
}

#[test]
fn div_cod_plist_cyrillic_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cyrillic-iso-8bit)");
}

#[test]
fn div_cod_plist_cyrillic_koi8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'cyrillic-koi8)");
}

#[test]
fn div_cod_plist_ebcdic_uk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ebcdic-uk)");
}

#[test]
fn div_cod_plist_ebcdic_us() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ebcdic-us)");
}

#[test]
fn div_cod_plist_emacs_mule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'emacs-mule)");
}

#[test]
fn div_cod_plist_euc_jis_2004() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'euc-jis-2004)");
}

#[test]
fn div_cod_plist_euc_tw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'euc-tw)");
}

#[test]
fn div_cod_plist_eucjp_ms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'eucjp-ms)");
}

#[test]
fn div_cod_plist_georgian_academy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'georgian-academy)");
}

#[test]
fn div_cod_plist_georgian_ps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'georgian-ps)");
}

#[test]
fn div_cod_plist_greek_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'greek-iso-8bit)");
}

#[test]
fn div_cod_plist_hebrew_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'hebrew-iso-8bit)");
}

#[test]
fn div_cod_plist_hp_roman8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'hp-roman8)");
}

#[test]
fn div_cod_plist_ibm038() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm038)");
}

#[test]
fn div_cod_plist_ibm1047() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm1047)");
}

#[test]
fn div_cod_plist_ibm256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm256)");
}

#[test]
fn div_cod_plist_ibm273() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm273)");
}

#[test]
fn div_cod_plist_ibm274() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm274)");
}

#[test]
fn div_cod_plist_ibm275() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm275)");
}

#[test]
fn div_cod_plist_ibm277() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm277)");
}

#[test]
fn div_cod_plist_ibm278() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm278)");
}

#[test]
fn div_cod_plist_ibm280() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm280)");
}

#[test]
fn div_cod_plist_ibm281() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm281)");
}

#[test]
fn div_cod_plist_ibm284() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm284)");
}

#[test]
fn div_cod_plist_ibm285() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm285)");
}

#[test]
fn div_cod_plist_ibm290() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm290)");
}

#[test]
fn div_cod_plist_ibm297() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'ibm297)");
}

#[test]
fn div_cod_plist_in_is13194_devanagari() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'in-is13194-devanagari)");
}

#[test]
fn div_cod_plist_iso_2022_7bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-7bit)");
}

#[test]
fn div_cod_plist_iso_2022_7bit_lock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-7bit-lock)");
}

#[test]
fn div_cod_plist_iso_2022_7bit_lock_ss2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-7bit-lock-ss2)");
}

#[test]
fn div_cod_plist_iso_2022_7bit_ss2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-7bit-ss2)");
}

#[test]
fn div_cod_plist_iso_2022_8bit_ss2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-8bit-ss2)");
}

#[test]
fn div_cod_plist_iso_2022_cn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-cn)");
}

#[test]
fn div_cod_plist_iso_2022_cn_ext() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-cn-ext)");
}

#[test]
fn div_cod_plist_iso_2022_jp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-jp)");
}

#[test]
fn div_cod_plist_iso_2022_jp_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-jp-2)");
}

#[test]
fn div_cod_plist_iso_2022_jp_2004() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-jp-2004)");
}

#[test]
fn div_cod_plist_iso_2022_kr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-2022-kr)");
}

#[test]
fn div_cod_plist_iso_8859_11() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-8859-11)");
}

#[test]
fn div_cod_plist_iso_8859_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-8859-6)");
}

#[test]
fn div_cod_plist_iso_latin_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-1)");
}

#[test]
fn div_cod_plist_iso_latin_10() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-10)");
}

#[test]
fn div_cod_plist_iso_latin_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-2)");
}

#[test]
fn div_cod_plist_iso_latin_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-3)");
}

#[test]
fn div_cod_plist_iso_latin_4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-4)");
}

#[test]
fn div_cod_plist_iso_latin_5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-5)");
}

#[test]
fn div_cod_plist_iso_latin_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-6)");
}

#[test]
fn div_cod_plist_iso_latin_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-7)");
}

#[test]
fn div_cod_plist_iso_latin_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-8)");
}

#[test]
fn div_cod_plist_iso_latin_9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'iso-latin-9)");
}

#[test]
fn div_cod_plist_japanese_cp932() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'japanese-cp932)");
}

#[test]
fn div_cod_plist_japanese_iso_7bit_1978_irv() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'japanese-iso-7bit-1978-irv)");
}

#[test]
fn div_cod_plist_japanese_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'japanese-iso-8bit)");
}

#[test]
fn div_cod_plist_japanese_shift_jis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'japanese-shift-jis)");
}

#[test]
fn div_cod_plist_japanese_shift_jis_2004() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'japanese-shift-jis-2004)");
}

#[test]
fn div_cod_plist_koi8_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'koi8-t)");
}

#[test]
fn div_cod_plist_koi8_u() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'koi8-u)");
}

#[test]
fn div_cod_plist_korean_cp949() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'korean-cp949)");
}

#[test]
fn div_cod_plist_korean_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'korean-iso-8bit)");
}

#[test]
fn div_cod_plist_lao() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'lao)");
}

#[test]
fn div_cod_plist_mac_roman() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'mac-roman)");
}

#[test]
fn div_cod_plist_mik() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'mik)");
}

#[test]
fn div_cod_plist_next() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'next)");
}

#[test]
fn div_cod_plist_no_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'no-conversion)");
}

#[test]
fn div_cod_plist_no_conversion_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'no-conversion-multibyte)");
}

#[test]
fn div_cod_plist_prefer_utf_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'prefer-utf-8)");
}

#[test]
fn div_cod_plist_pt154() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'pt154)");
}

#[test]
fn div_cod_plist_raw_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'raw-text)");
}

#[test]
fn div_cod_plist_thai_tis620() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'thai-tis620)");
}

#[test]
fn div_cod_plist_tibetan_iso_8bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'tibetan-iso-8bit)");
}

#[test]
fn div_cod_plist_undecided() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'undecided)");
}

#[test]
fn div_cod_plist_us_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'us-ascii)");
}

#[test]
fn div_cod_plist_utf_16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-16)");
}

#[test]
fn div_cod_plist_utf_16be() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-16be)");
}

#[test]
fn div_cod_plist_utf_16be_with_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-16be-with-signature)");
}

#[test]
fn div_cod_plist_utf_16le() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-16le)");
}

#[test]
fn div_cod_plist_utf_16le_with_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-16le-with-signature)");
}

#[test]
fn div_cod_plist_utf_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-7)");
}

#[test]
fn div_cod_plist_utf_7_imap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-7-imap)");
}

#[test]
fn div_cod_plist_utf_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-8)");
}

#[test]
fn div_cod_plist_utf_8_auto() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-8-auto)");
}

#[test]
fn div_cod_plist_utf_8_emacs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-8-emacs)");
}

#[test]
fn div_cod_plist_utf_8_with_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'utf-8-with-signature)");
}

#[test]
fn div_cod_plist_vietnamese_viqr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'vietnamese-viqr)");
}

#[test]
fn div_cod_plist_vietnamese_viscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'vietnamese-viscii)");
}

#[test]
fn div_cod_plist_vietnamese_vscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'vietnamese-vscii)");
}

#[test]
fn div_cod_plist_windows_1250() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1250)");
}

#[test]
fn div_cod_plist_windows_1251() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1251)");
}

#[test]
fn div_cod_plist_windows_1252() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1252)");
}

#[test]
fn div_cod_plist_windows_1253() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1253)");
}

#[test]
fn div_cod_plist_windows_1254() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1254)");
}

#[test]
fn div_cod_plist_windows_1255() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1255)");
}

#[test]
fn div_cod_plist_windows_1256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1256)");
}

#[test]
fn div_cod_plist_windows_1257() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1257)");
}

#[test]
fn div_cod_plist_windows_1258() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(coding-system-plist 'windows-1258)");
}
