/// Batch 504: BOM/coding-system characterization — all utf-8 variants.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx504_bom_utf8_with_sig() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-temp-file "cx504-sig-")))
  (let ((coding-system-for-write 'utf-8-with-signature))
    (write-region "x" nil f nil 0))
  (prog1 (with-temp-buffer
           (set-buffer-multibyte nil)
           (insert-file-contents f)
           (let ((b (buffer-string)))
             (list (string-bytes b) (aref b 0) (aref b 1) (aref b 2))))
    (delete-file f)))
"##,
    );
}

#[test]
fn div_cx504_bom_utf8_sig_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-p 'utf-8-sig)
      (find-coding-system 'utf-8-sig))
"##,
    );
}

#[test]
fn div_cx504_detect_utf8_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (detect-coding-string "hello")
      (detect-coding-string "cafe")
      (detect-coding-string (string #xef #xbb #xbf 65)))
"##,
    );
}

#[test]
fn div_cx504_detect_utf16_bom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (detect-coding-string (string #xff #xfe 65 0))
      (detect-coding-string (string #xfe #xff 0 65)))
"##,
    );
}

#[test]
fn div_cx504_coding_system_aliases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-aliases 'utf-8)
      (coding-system-aliases 'latin-1))
"##,
    );
}

#[test]
fn div_cx504_coding_system_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-type 'utf-8)
      (coding-system-type 'latin-1)
      (coding-system-type 'raw-text))
"##,
    );
}

#[test]
fn div_cx504_coding_system_mnemonic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-mnemonic 'utf-8)
      (coding-system-mnemonic 'latin-1))
"##,
    );
}

#[test]
fn div_cx504_coding_system_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-base 'utf-8-unix)
      (coding-system-base 'utf-8-dos)
      (coding-system-base 'utf-8-mac))
"##,
    );
}

#[test]
fn div_cx504_coding_system_eol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-eol-type 'utf-8-unix)
      (coding-system-eol-type 'utf-8-dos)
      (coding-system-eol-type 'utf-8-mac))
"##,
    );
}

#[test]
fn div_cx504_coding_system_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (coding-system-category 'utf-8)
      (coding-system-category 'latin-1)
      (condition-case e (coding-system-category 'nonexistent) (error (car e))))
"##,
    );
}

#[test]
fn div_cx504_coding_system_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (plist-get (coding-system-plist 'utf-8) :category)
      (plist-get (coding-system-plist 'utf-8) :ascii-compatible-p))
"##,
    );
}

#[test]
fn div_cx504_encode_coding_string_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (encode-coding-string "hello" 'utf-8)
      (encode-coding-string "cafe" 'utf-8)
      (string-bytes (encode-coding-string "cafe" 'utf-8)))
"##,
    );
}

#[test]
fn div_cx504_decode_coding_string_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((enc (encode-coding-string "hello" 'utf-8)))
  (list (decode-coding-string enc 'utf-8)
        (string= (decode-coding-string enc 'utf-8) "hello")))
"##,
    );
}

#[test]
fn div_cx504_prefer_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (prefer-coding-system 'utf-8)
      (prefer-coding-system 'latin-1))
"##,
    );
}

#[test]
fn div_cx504_set_terminal_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (set-terminal-coding-system 'utf-8)
  (error (car e)))
"##,
    );
}
