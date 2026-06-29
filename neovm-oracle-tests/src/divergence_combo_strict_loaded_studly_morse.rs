//! Strict combo oracle probes, batch 82: studlify-region (StUdLyCaPs) and
//! morse-region/unmorse-region (Morse code conversion). Deterministic string
//! transformations.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_p6_studlify_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"/home/exec/Projects/github.com/eval-exec/neomacs-main/lisp/studly.el\")""#
    ]];
    crate::common::assert_oracle_parity_with_load_expect(
        r##"
(with-temp-buffer
  (insert "Hello World Foo Bar")
  (studlify-region (point-min) (point-max))
  (buffer-string))
"##,
        &["studly.el"],
        expect,
    );
}

#[test]
fn div_p6_morse_and_unmorse_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"/home/exec/Projects/github.com/eval-exec/neomacs-main/lisp/morse.el\")""#
    ]];
    crate::common::assert_oracle_parity_with_load_expect(
        r##"
(list (with-temp-buffer
        (insert "HELLO")
        (morse-region (point-min) (point-max))
        (buffer-string))
      (with-temp-buffer
        (insert ".... . .-.. .-.. ---")
        (unmorse-region (point-min) (point-max))
        (buffer-string)))
"##,
        &["morse.el"],
        expect,
    );
}
