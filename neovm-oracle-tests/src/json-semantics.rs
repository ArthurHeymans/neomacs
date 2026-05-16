//! Oracle parity tests for GNU JSON primitive option semantics.
//!
//! GNU implements `json-serialize` and `json-parse-string` in `src/json.c`.
//! Its `json_parse_args` walks keyword pairs from right to left so duplicate
//! keyword values that appear first take precedence.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_json_duplicate_keyword_options_use_first_value_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (json-serialize :a :null-object :a :null-object :b)
   (error (list (car err) (cdr err))))
 (condition-case err
     (json-serialize :b :null-object :a :null-object :b)
   (error (list (car err) (cdr err))))
 (condition-case err
     (json-parse-string
      "[null,false]"
      :array-type 'list
      :null-object 'first-null
      :null-object 'second-null
      :false-object 'first-false
      :false-object 'second-false)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_json_serialize_default_null_and_false_sentinels_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (json-serialize nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (json-serialize :null)
   (error (list (car err) (cdr err))))
 (condition-case err
     (json-serialize :false)
   (error (list (car err) (cdr err))))
 (condition-case err
     (json-serialize :json-false)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
