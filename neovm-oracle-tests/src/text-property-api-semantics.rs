//! Oracle parity tests for GNU text property API edge semantics.
//!
//! GNU implements these primitives in `src/textprop.c`.  These tests focus on
//! return values, empty ranges, range validation, property-change limits, and
//! buffer/string indexing differences.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_text_property_mutator_return_values_and_empty_ranges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (copy-sequence "abcd")))
  (list
   (add-text-properties 0 2 '(a 1) s)
   (add-text-properties 0 2 '(a 1) s)
   (put-text-property 0 2 'b 2 s)
   (set-text-properties 2 2 '(c 3) s)
   (remove-text-properties 0 2 '(missing nil) s)
   (remove-text-properties 0 2 '(a nil) s)
   s
   (text-properties-at 0 s)
   (text-properties-at 2 s)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_text_properties_at_end_and_range_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (propertize "abc" 'face 'bold)))
  (list
   (text-properties-at 0 s)
   (text-properties-at 3 s)
   (get-text-property 3 'face s)
   (condition-case err
       (text-properties-at 4 s)
     (error (list (car err) (cdr err))))
   (condition-case err
       (text-properties-at -1 s)
     (error (list (car err) (cdr err))))
   (condition-case err
       (put-text-property 2 1 'a 1 s)
     (error (list (car err) (cdr err))))
   (condition-case err
       (add-text-properties 0 1 '(a) s)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_next_previous_property_change_limit_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((s (concat (propertize "ab" 'face 'bold)
                 (propertize "cd" 'face 'italic)
                 "ef")))
  (list
   (next-property-change 0 s)
   (next-property-change 0 s 1)
   (next-property-change 0 s 2)
   (next-property-change 2 s t)
   (next-property-change 4 s)
   (next-property-change 4 s 5)
   (previous-property-change 6 s)
   (previous-property-change 6 s 5)
   (previous-property-change 4 s t)
   (previous-property-change 2 s 1)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_buffer_text_property_positions_are_one_based() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "abcd")
  (add-text-properties 1 3 '(face bold) (current-buffer))
  (list
   (get-text-property 1 'face)
   (get-text-property 3 'face)
   (text-properties-at 1)
   (text-properties-at 5)
   (next-property-change 1 nil)
   (previous-property-change 5 nil)
   (condition-case err
       (text-properties-at 0)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
