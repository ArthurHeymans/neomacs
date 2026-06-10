//! Oracle parity tests for GNU `purecopy` runtime semantics.
//!
//! GNU defines `purecopy` as an obsolete function alias to `identity` in
//! `lisp/subr.el`, so runtime calls must return the same object unchanged.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_purecopy_is_identity_alias_at_runtime() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((cons-cell (list (list 'a)))
       (vec (vector (list 'b)))
       (str (copy-sequence "abc"))
       (sym (make-symbol "purecopy-oracle")))
  (list
   (fboundp 'purecopy)
   (eq (purecopy cons-cell) cons-cell)
   (eq (purecopy vec) vec)
   (eq (purecopy str) str)
   (eq (purecopy sym) sym)
   (purecopy 42)
   (condition-case err
       (purecopy)
     (error (list (car err) (cdr err))))
   (condition-case err
       (purecopy cons-cell vec)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity(form);
}
