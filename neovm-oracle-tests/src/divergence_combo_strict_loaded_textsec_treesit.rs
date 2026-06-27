//! Strict combo oracle probes, batch 88: recent Emacs features — textsec
//! (confusable/homoglyph detection) and treesit availability.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q2_textsec_confusable_detection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (condition-case err (textsec-unidata-restrictions ?a) (error (cons 'err (car err))))
      (condition-case err (textsec-string-has-confusables "abc")
        (error (cons 'err (car err))))
      (condition-case err (textsec-string-has-confusables "аbc")
        (error (cons 'err (car err)))))
"##,
        &["international/textsec.el"],
    );
}

#[test]
fn div_q2_treesit_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (treesit-available-p) (error (car err)))
      (fboundp 'treesit-parse-string)
      (fboundp 'treesit-node-type))
"##,
    );
}
