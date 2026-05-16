//! Oracle parity tests for GNU `subr.el` `field-at-pos`.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_prop_gnu_field_at_pos_boundary_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el:field-at-pos reads the field at `field-beginning`; when that
    // raw field is `boundary`, it returns the field before `field-end`.
    let form = r#"
(with-temp-buffer
  (insert "aaXbb")
  (put-text-property 1 3 'field 'left)
  (put-text-property 3 4 'field 'boundary)
  (put-text-property 4 6 'field 'right)
  (list
   (mapcar #'field-at-pos (number-sequence 1 5))
   (mapcar (lambda (p)
             (list p
                   (field-beginning p)
                   (field-end p)
                   (get-char-property p 'field)))
           (number-sequence 1 5))))
"#;
    assert_oracle_parity_with_bootstrap(form);
}
