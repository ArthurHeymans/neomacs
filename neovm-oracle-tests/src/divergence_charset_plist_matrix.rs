//! Per-charset *charset-plist* matrix (all GNU charsets).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cs_plist_adobe_standard_encoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'adobe-standard-encoding)");
}

#[test]
fn div_cs_plist_alternativnyj() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'alternativnyj)");
}

#[test]
fn div_cs_plist_arabic_1_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'arabic-1-column)");
}

#[test]
fn div_cs_plist_arabic_2_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'arabic-2-column)");
}

#[test]
fn div_cs_plist_arabic_digit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'arabic-digit)");
}

#[test]
fn div_cs_plist_arabic_iso8859_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'arabic-iso8859-6)");
}

#[test]
fn div_cs_plist_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ascii)");
}

#[test]
fn div_cs_plist_assamese_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'assamese-cdac)");
}

#[test]
fn div_cs_plist_bengali_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'bengali-akruti)");
}

#[test]
fn div_cs_plist_bengali_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'bengali-cdac)");
}

#[test]
fn div_cs_plist_big5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'big5)");
}

#[test]
fn div_cs_plist_big5_hkscs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'big5-hkscs)");
}

#[test]
fn div_cs_plist_chinese_big5_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-big5-1)");
}

#[test]
fn div_cs_plist_chinese_big5_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-big5-2)");
}

#[test]
fn div_cs_plist_chinese_cns11643_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-1)");
}

#[test]
fn div_cs_plist_chinese_cns11643_15() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-15)");
}

#[test]
fn div_cs_plist_chinese_cns11643_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-2)");
}

#[test]
fn div_cs_plist_chinese_cns11643_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-3)");
}

#[test]
fn div_cs_plist_chinese_cns11643_4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-4)");
}

#[test]
fn div_cs_plist_chinese_cns11643_5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-5)");
}

#[test]
fn div_cs_plist_chinese_cns11643_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-6)");
}

#[test]
fn div_cs_plist_chinese_cns11643_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-cns11643-7)");
}

#[test]
fn div_cs_plist_chinese_gb2312() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-gb2312)");
}

#[test]
fn div_cs_plist_chinese_gbk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-gbk)");
}

#[test]
fn div_cs_plist_chinese_sisheng() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'chinese-sisheng)");
}

#[test]
fn div_cs_plist_control_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'control-1)");
}

#[test]
fn div_cs_plist_cp00858() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp00858)");
}

#[test]
fn div_cs_plist_cp038() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp038)");
}

#[test]
fn div_cs_plist_cp1047() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1047)");
}

#[test]
fn div_cs_plist_cp1125() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1125)");
}

#[test]
fn div_cs_plist_cp1250() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1250)");
}

#[test]
fn div_cs_plist_cp1251() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1251)");
}

#[test]
fn div_cs_plist_cp1252() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1252)");
}

#[test]
fn div_cs_plist_cp1253() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1253)");
}

#[test]
fn div_cs_plist_cp1254() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1254)");
}

#[test]
fn div_cs_plist_cp1255() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1255)");
}

#[test]
fn div_cs_plist_cp1256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1256)");
}

#[test]
fn div_cs_plist_cp1257() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1257)");
}

#[test]
fn div_cs_plist_cp1258() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp1258)");
}

#[test]
fn div_cs_plist_cp154() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp154)");
}

#[test]
fn div_cs_plist_cp437() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp437)");
}

#[test]
fn div_cs_plist_cp720() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp720)");
}

#[test]
fn div_cs_plist_cp737() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp737)");
}

#[test]
fn div_cs_plist_cp775() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp775)");
}

#[test]
fn div_cs_plist_cp850() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp850)");
}

#[test]
fn div_cs_plist_cp851() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp851)");
}

#[test]
fn div_cs_plist_cp852() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp852)");
}

#[test]
fn div_cs_plist_cp855() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp855)");
}

#[test]
fn div_cs_plist_cp857() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp857)");
}

#[test]
fn div_cs_plist_cp858() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp858)");
}

#[test]
fn div_cs_plist_cp860() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp860)");
}

#[test]
fn div_cs_plist_cp861() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp861)");
}

#[test]
fn div_cs_plist_cp862() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp862)");
}

#[test]
fn div_cs_plist_cp863() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp863)");
}

#[test]
fn div_cs_plist_cp864() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp864)");
}

#[test]
fn div_cs_plist_cp865() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp865)");
}

#[test]
fn div_cs_plist_cp866() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp866)");
}

#[test]
fn div_cs_plist_cp866u() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp866u)");
}

#[test]
fn div_cs_plist_cp869() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp869)");
}

#[test]
fn div_cs_plist_cp874() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp874)");
}

#[test]
fn div_cs_plist_cp932() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp932)");
}

#[test]
fn div_cs_plist_cp932_2_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp932-2-byte)");
}

#[test]
fn div_cs_plist_cp936() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp936)");
}

#[test]
fn div_cs_plist_cp949() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp949)");
}

#[test]
fn div_cs_plist_cp949_2_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cp949-2-byte)");
}

#[test]
fn div_cs_plist_cyrillic_iso8859_5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'cyrillic-iso8859-5)");
}

#[test]
fn div_cs_plist_devanagari_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'devanagari-akruti)");
}

#[test]
fn div_cs_plist_devanagari_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'devanagari-cdac)");
}

#[test]
fn div_cs_plist_ebcdic_int() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ebcdic-int)");
}

#[test]
fn div_cs_plist_ebcdic_uk() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ebcdic-uk)");
}

#[test]
fn div_cs_plist_ebcdic_us() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ebcdic-us)");
}

#[test]
fn div_cs_plist_eight_bit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'eight-bit)");
}

#[test]
fn div_cs_plist_eight_bit_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'eight-bit-control)");
}

#[test]
fn div_cs_plist_eight_bit_graphic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'eight-bit-graphic)");
}

#[test]
fn div_cs_plist_emacs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'emacs)");
}

#[test]
fn div_cs_plist_ethiopic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ethiopic)");
}

#[test]
fn div_cs_plist_gb18030() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gb18030)");
}

#[test]
fn div_cs_plist_gb18030_2_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gb18030-2-byte)");
}

#[test]
fn div_cs_plist_gb18030_4_byte_bmp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gb18030-4-byte-bmp)");
}

#[test]
fn div_cs_plist_gb18030_4_byte_ext_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gb18030-4-byte-ext-1)");
}

#[test]
fn div_cs_plist_gb18030_4_byte_ext_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gb18030-4-byte-ext-2)");
}

#[test]
fn div_cs_plist_gb18030_4_byte_smp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gb18030-4-byte-smp)");
}

#[test]
fn div_cs_plist_georgian_academy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'georgian-academy)");
}

#[test]
fn div_cs_plist_georgian_ps() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'georgian-ps)");
}

#[test]
fn div_cs_plist_greek_iso8859_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'greek-iso8859-7)");
}

#[test]
fn div_cs_plist_gujarati_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gujarati-akruti)");
}

#[test]
fn div_cs_plist_gujarati_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'gujarati-cdac)");
}

#[test]
fn div_cs_plist_hebrew_iso8859_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'hebrew-iso8859-8)");
}

#[test]
fn div_cs_plist_hp_roman8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'hp-roman8)");
}

#[test]
fn div_cs_plist_ibm038() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm038)");
}

#[test]
fn div_cs_plist_ibm1047() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm1047)");
}

#[test]
fn div_cs_plist_ibm256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm256)");
}

#[test]
fn div_cs_plist_ibm273() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm273)");
}

#[test]
fn div_cs_plist_ibm274() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm274)");
}

#[test]
fn div_cs_plist_ibm275() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm275)");
}

#[test]
fn div_cs_plist_ibm277() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm277)");
}

#[test]
fn div_cs_plist_ibm278() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm278)");
}

#[test]
fn div_cs_plist_ibm280() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm280)");
}

#[test]
fn div_cs_plist_ibm281() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm281)");
}

#[test]
fn div_cs_plist_ibm284() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm284)");
}

#[test]
fn div_cs_plist_ibm285() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm285)");
}

#[test]
fn div_cs_plist_ibm290() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm290)");
}

#[test]
fn div_cs_plist_ibm297() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm297)");
}

#[test]
fn div_cs_plist_ibm850() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm850)");
}

#[test]
fn div_cs_plist_ibm866() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ibm866)");
}

#[test]
fn div_cs_plist_indian_1_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'indian-1-column)");
}

#[test]
fn div_cs_plist_indian_2_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'indian-2-column)");
}

#[test]
fn div_cs_plist_indian_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'indian-glyph)");
}

#[test]
fn div_cs_plist_indian_is13194() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'indian-is13194)");
}

#[test]
fn div_cs_plist_ipa() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ipa)");
}

#[test]
fn div_cs_plist_iso_8859_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-1)");
}

#[test]
fn div_cs_plist_iso_8859_10() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-10)");
}

#[test]
fn div_cs_plist_iso_8859_11() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-11)");
}

#[test]
fn div_cs_plist_iso_8859_13() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-13)");
}

#[test]
fn div_cs_plist_iso_8859_14() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-14)");
}

#[test]
fn div_cs_plist_iso_8859_15() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-15)");
}

#[test]
fn div_cs_plist_iso_8859_16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-16)");
}

#[test]
fn div_cs_plist_iso_8859_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-2)");
}

#[test]
fn div_cs_plist_iso_8859_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-3)");
}

#[test]
fn div_cs_plist_iso_8859_4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-4)");
}

#[test]
fn div_cs_plist_iso_8859_5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-5)");
}

#[test]
fn div_cs_plist_iso_8859_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-6)");
}

#[test]
fn div_cs_plist_iso_8859_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-7)");
}

#[test]
fn div_cs_plist_iso_8859_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-8)");
}

#[test]
fn div_cs_plist_iso_8859_9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'iso-8859-9)");
}

#[test]
fn div_cs_plist_japanese_jisx0208() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0208)");
}

#[test]
fn div_cs_plist_japanese_jisx0208_1978() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0208-1978)");
}

#[test]
fn div_cs_plist_japanese_jisx0212() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0212)");
}

#[test]
fn div_cs_plist_japanese_jisx0213_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0213-1)");
}

#[test]
fn div_cs_plist_japanese_jisx0213_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0213-2)");
}

#[test]
fn div_cs_plist_japanese_jisx0213_a() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0213-a)");
}

#[test]
fn div_cs_plist_japanese_jisx0213_2004_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'japanese-jisx0213.2004-1)");
}

#[test]
fn div_cs_plist_jisx0201() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'jisx0201)");
}

#[test]
fn div_cs_plist_kannada_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'kannada-akruti)");
}

#[test]
fn div_cs_plist_kannada_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'kannada-cdac)");
}

#[test]
fn div_cs_plist_katakana_jisx0201() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'katakana-jisx0201)");
}

#[test]
fn div_cs_plist_katakana_sjis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'katakana-sjis)");
}

#[test]
fn div_cs_plist_koi8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'koi8)");
}

#[test]
fn div_cs_plist_koi8_r() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'koi8-r)");
}

#[test]
fn div_cs_plist_koi8_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'koi8-t)");
}

#[test]
fn div_cs_plist_koi8_u() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'koi8-u)");
}

#[test]
fn div_cs_plist_korean_ksc5601() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'korean-ksc5601)");
}

#[test]
fn div_cs_plist_lao() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'lao)");
}

#[test]
fn div_cs_plist_latin_iso8859_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-1)");
}

#[test]
fn div_cs_plist_latin_iso8859_10() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-10)");
}

#[test]
fn div_cs_plist_latin_iso8859_13() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-13)");
}

#[test]
fn div_cs_plist_latin_iso8859_14() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-14)");
}

#[test]
fn div_cs_plist_latin_iso8859_15() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-15)");
}

#[test]
fn div_cs_plist_latin_iso8859_16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-16)");
}

#[test]
fn div_cs_plist_latin_iso8859_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-2)");
}

#[test]
fn div_cs_plist_latin_iso8859_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-3)");
}

#[test]
fn div_cs_plist_latin_iso8859_4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-4)");
}

#[test]
fn div_cs_plist_latin_iso8859_9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-iso8859-9)");
}

#[test]
fn div_cs_plist_latin_jisx0201() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'latin-jisx0201)");
}

#[test]
fn div_cs_plist_mac_roman() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'mac-roman)");
}

#[test]
fn div_cs_plist_malayalam_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'malayalam-akruti)");
}

#[test]
fn div_cs_plist_malayalam_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'malayalam-cdac)");
}

#[test]
fn div_cs_plist_mik() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'mik)");
}

#[test]
fn div_cs_plist_mule_lao() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'mule-lao)");
}

#[test]
fn div_cs_plist_mule_unicode_0100_24ff() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'mule-unicode-0100-24ff)");
}

#[test]
fn div_cs_plist_mule_unicode_2500_33ff() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'mule-unicode-2500-33ff)");
}

#[test]
fn div_cs_plist_mule_unicode_e000_ffff() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'mule-unicode-e000-ffff)");
}

#[test]
fn div_cs_plist_next() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'next)");
}

#[test]
fn div_cs_plist_oriya_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'oriya-akruti)");
}

#[test]
fn div_cs_plist_oriya_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'oriya-cdac)");
}

#[test]
fn div_cs_plist_pt154() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'pt154)");
}

#[test]
fn div_cs_plist_ptcp154() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ptcp154)");
}

#[test]
fn div_cs_plist_punjabi_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'punjabi-akruti)");
}

#[test]
fn div_cs_plist_punjabi_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'punjabi-cdac)");
}

#[test]
fn div_cs_plist_ruscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ruscii)");
}

#[test]
fn div_cs_plist_sanskrit_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'sanskrit-cdac)");
}

#[test]
fn div_cs_plist_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'symbol)");
}

#[test]
fn div_cs_plist_tamil_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'tamil-akruti)");
}

#[test]
fn div_cs_plist_tamil_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'tamil-cdac)");
}

#[test]
fn div_cs_plist_tcvn_5712() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'tcvn-5712)");
}

#[test]
fn div_cs_plist_telugu_akruti() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'telugu-akruti)");
}

#[test]
fn div_cs_plist_telugu_cdac() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'telugu-cdac)");
}

#[test]
fn div_cs_plist_thai_iso8859_11() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'thai-iso8859-11)");
}

#[test]
fn div_cs_plist_thai_tis620() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'thai-tis620)");
}

#[test]
fn div_cs_plist_tibetan() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'tibetan)");
}

#[test]
fn div_cs_plist_tibetan_1_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'tibetan-1-column)");
}

#[test]
fn div_cs_plist_tis620_2533() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'tis620-2533)");
}

#[test]
fn div_cs_plist_ucs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'ucs)");
}

#[test]
fn div_cs_plist_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'unicode)");
}

#[test]
fn div_cs_plist_unicode_bmp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'unicode-bmp)");
}

#[test]
fn div_cs_plist_unicode_sip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'unicode-sip)");
}

#[test]
fn div_cs_plist_unicode_smp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'unicode-smp)");
}

#[test]
fn div_cs_plist_unicode_ssp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'unicode-ssp)");
}

#[test]
fn div_cs_plist_vietnamese_viscii_lower() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'vietnamese-viscii-lower)");
}

#[test]
fn div_cs_plist_vietnamese_viscii_upper() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'vietnamese-viscii-upper)");
}

#[test]
fn div_cs_plist_viscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'viscii)");
}

#[test]
fn div_cs_plist_vscii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'vscii)");
}

#[test]
fn div_cs_plist_vscii_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'vscii-2)");
}

#[test]
fn div_cs_plist_windows_1250() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1250)");
}

#[test]
fn div_cs_plist_windows_1251() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1251)");
}

#[test]
fn div_cs_plist_windows_1252() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1252)");
}

#[test]
fn div_cs_plist_windows_1253() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1253)");
}

#[test]
fn div_cs_plist_windows_1254() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1254)");
}

#[test]
fn div_cs_plist_windows_1255() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1255)");
}

#[test]
fn div_cs_plist_windows_1256() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1256)");
}

#[test]
fn div_cs_plist_windows_1257() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1257)");
}

#[test]
fn div_cs_plist_windows_1258() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-1258)");
}

#[test]
fn div_cs_plist_windows_936() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(charset-plist 'windows-936)");
}
