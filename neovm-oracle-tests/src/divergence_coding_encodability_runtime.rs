//! Coding encodability/query parity: coding-system-change-eol/text-conversion,
//! charset-list/mnemonic, find-coding-systems-for-charsets, terminal/keyboard
//! coding, decode-coding-region; plus four encodability divergences
//! (unencodable-char-position, find-coding-systems-string, check-coding-//! systems-region, define-coding-system-alias round-trip).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn coding_change_eol() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (coding-system-change-eol-conversion 'utf-8 'dos)
        (coding-system-change-eol-conversion 'utf-8 'unix)
        (coding-system-change-eol-conversion 'utf-8 'mac)
        (coding-system-change-eol-conversion 'utf-8-dos 'unix))"##,
    );
}

#[test]
fn coding_change_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (coding-system-change-text-conversion 'utf-8-dos 'latin-1)
        (coding-system-base (coding-system-change-text-conversion 'undecided-unix 'utf-8)))"##,
    );
}

#[test]
fn coding_charset_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (memq 'ascii (coding-system-charset-list 'utf-8))
        (coding-system-charset-list 'iso-8859-1)
        (coding-system-mnemonic 'utf-8))"##,
    );
}

#[test]
fn decode_coding_region_dest() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (encode-coding-string "héllo" 'utf-8))
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (list (buffer-string) (buffer-size)))"##,
    );
}

#[test]
fn find_cs_for_charsets() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (listp (find-coding-systems-for-charsets '(ascii)))
        (booleanp (and (memq 'iso-8859-1 (find-coding-systems-for-charsets '(latin-iso8859-1))) t)))"##,
    );
}

#[test]
fn terminal_keyboard_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (coding-system-p (or (keyboard-coding-system) 'undecided))
        (coding-system-p (or (terminal-coding-system) 'undecided)))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: unencodable-char-position always returns nil (claims every char is encodable); GNU returns the position of the first char a coding system cannot encode (e.g. 4 for a non-ASCII char under us-ascii/iso-8859-1)."]
fn divergence_unencodable_char_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc日本def")
  (list (unencodable-char-position (point-min) (point-max) 'us-ascii)
        (unencodable-char-position (point-min) (point-max) 'utf-8)
        (unencodable-char-position (point-min) (point-max) 'iso-8859-1)))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: find-coding-systems-string returns only the utf-8/raw family; GNU returns the full set of capable coding systems, so CJK/iso-2022 systems (iso-2022-jp, chinese-gbk, ...) are missing."]
fn divergence_find_coding_systems_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (> (length (find-coding-systems-string "héllo 日本語")) 10)
      (and (memq 'iso-2022-jp (find-coding-systems-string "日本語")) t)
      (and (memq 'chinese-gbk (find-coding-systems-string "中文")) t))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: check-coding-systems-region reports no unencodable positions even when a coding system cannot encode the text (us-ascii vs cafe-with-accent); GNU returns (us-ascii POS). Related to unencodable-char-position."]
fn divergence_check_coding_systems_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "café")
  (let ((r (check-coding-systems-region (point-min) (point-max) '(us-ascii utf-8))))
    (list (assq 'us-ascii r) (assq 'utf-8 r))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: a coding-system alias from define-coding-system-alias is recognized (coding-system-p/-base) but encode/decode through the alias name does not round-trip in neomacs, while GNU's alias works."]
fn divergence_define_coding_system_alias_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(define-coding-system-alias 'neo-utf8-alias-xyz 'utf-8)
(list (coding-system-p 'neo-utf8-alias-xyz)
      (coding-system-base 'neo-utf8-alias-xyz)
      (let ((s "tëst")) (string= s (decode-coding-string (encode-coding-string s 'neo-utf8-alias-xyz) 'neo-utf8-alias-xyz))))"##,
    );
}
