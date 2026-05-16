//! Oracle parity tests for GNU buffer-local toplevel value semantics.
//!
//! GNU implements these in `src/eval.c`: `buffer-local-toplevel-value` reads
//! the buffer-local binding outside any `let` binding and signals
//! `void-variable` if the target buffer has no local value.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_buffer_local_toplevel_value_read_set_and_missing_local_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((b1 (get-buffer-create " *bltv-oracle-1*"))
      (b2 (get-buffer-create " *bltv-oracle-2*")))
  (makunbound 'neomacs--oracle-bltv)
  (list
   (condition-case err
       (buffer-local-toplevel-value 'neomacs--oracle-bltv b1)
     (error (cons (car err) (cdr err))))
   (set-buffer-local-toplevel-value 'neomacs--oracle-bltv 11 b1)
   (buffer-local-toplevel-value 'neomacs--oracle-bltv b1)
   (local-variable-p 'neomacs--oracle-bltv b1)
   (local-variable-p 'neomacs--oracle-bltv b2)
   (condition-case err
       (buffer-local-toplevel-value 'neomacs--oracle-bltv b2)
     (error (cons (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
