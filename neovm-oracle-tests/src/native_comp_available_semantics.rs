//! Oracle parity tests for GNU native compilation availability.
//!
//! GNU implements `native-comp-available-p` in `src/comp.c`: it returns non-nil
//! only when native compilation support is built in and gccjit can be loaded.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_native_comp_available_matches_gnu_build_capability() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(r#"(native-comp-available-p)"#, expect);
}
