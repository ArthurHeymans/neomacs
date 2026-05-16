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

#[test]
fn oracle_object_intervals_preserves_adjacent_equal_property_runs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU compares text properties by effective interval values, but
    // `object-intervals` still exposes the concrete interval run shape.
    // This follows src/fns.c:Fequal_including_properties/internal_equal and
    // src/textprop.c interval mutation behavior.
    let form = r#"
(let ((split (copy-sequence "xy"))
      (merged (copy-sequence "xy")))
  (put-text-property 0 1 'face 'bold split)
  (put-text-property 1 2 'face 'bold split)
  (put-text-property 0 2 'face 'bold merged)
  (list
   (object-intervals split)
   (object-intervals merged)
   (equal split merged)
   (equal-including-properties split merged)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
