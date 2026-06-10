//! Oracle parity tests for `read-char`.
//!
//! GNU implements `read-char` in `src/keyboard.c` — reads one character
//! from the minibuffer or input stream.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, eval_oracle_and_neovm};

#[test]
fn oracle_read_char_wrong_number_of_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(read-char nil nil nil nil)");
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");
}

#[test]
fn oracle_read_char_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(read-char '(bad))");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
