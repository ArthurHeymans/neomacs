//! Oracle parity tests for GNU `subr.el` string conversion helpers.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_prop_gnu_string_to_list_vector_byte_and_property_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el:string-to-list is `(append string nil)` and
    // string-to-vector is `(vconcat string)`.  Multibyte strings produce
    // characters, unibyte strings produce bytes, NUL bytes are preserved, and
    // text properties do not alter the produced numeric elements.
    let form = r#"
(let* ((multi (propertize "éa" 'face 'bold))
       (uni (string-as-unibyte "é"))
       (nul "a\0b"))
  (list
   (string-to-list multi)
   (mapcar (lambda (i) (get-text-property i 'face multi))
           (number-sequence 0 (1- (length multi))))
   (string-to-vector multi)
   (multibyte-string-p uni)
   (string-to-list uni)
   (string-to-vector uni)
   (string-to-list nul)
   (string-to-vector nul)))
"#;
    assert_oracle_parity_with_bootstrap(form);
}
