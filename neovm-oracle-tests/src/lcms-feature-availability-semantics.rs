//! Oracle parity tests for GNU LCMS feature-gated primitives.
//!
//! GNU implements these in `src/lcms.c` under `#ifdef HAVE_LCMS2`; when the
//! local GNU binary is built without LCMS2, the symbols are not fbound.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_lcms_primitives_follow_gnu_build_feature_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (fboundp 'lcms2-available-p)
 (fboundp 'lcms-temp->white-point)
 (condition-case err
     (lcms-temp->white-point 6500)
   (error (cons (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
