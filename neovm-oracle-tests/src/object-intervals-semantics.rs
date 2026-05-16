//! Oracle parity tests for GNU `object-intervals` semantics.
//!
//! GNU implements `Fobject_intervals` in `src/fns.c`: strings and buffers
//! return a copied interval list, including explicit nil-property runs around
//! non-empty property intervals; plain objects without intervals return nil.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_object_intervals_string_and_buffer_interval_shape() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (copy-sequence "abcdef"))
      (b (get-buffer-create " *object-intervals-oracle*")))
  (put-text-property 1 3 'face 'bold s)
  (put-text-property 3 6 'help-echo "tail" s)
  (with-current-buffer b
    (erase-buffer)
    (insert "abcd")
    (put-text-property 2 4 'face 'italic))
  (list
   (object-intervals "plain")
   (object-intervals s)
   (object-intervals b)
   (condition-case err
       (object-intervals 42)
     (error (cons (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
