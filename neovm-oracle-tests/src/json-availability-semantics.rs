//! Oracle parity tests for GNU `subr.el` JSON availability helper.

use super::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn oracle_prop_gnu_json_available_p_is_native_json_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el:json-available-p is now a simple native-JSON availability
    // marker that returns t.  Keep it paired with the native JSON functions so
    // startup/runtime registration cannot drift independently.
    let form = r#"
(list
 (json-available-p)
 (functionp 'json-available-p)
 (json-serialize [1 2 3])
 (json-parse-string "{\"a\":1}" :object-type 'alist)
 (condition-case err
     (json-available-p nil)
   (error (car err))))
"#;
    assert_oracle_parity(form);
}
