//! Oracle parity tests for GNU `subr.el` readability and function aliases.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_prop_gnu_subr_function_alias_p_and_readablep_contracts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el:function-alias-p walks symbol-function links until a
    // non-symbol definition, and readablep returns either prin1 syntax or nil
    // through print-unreadable-function.
    let form = r#"(unwind-protect
    (progn
      (defalias 'neovm-function-alias-base (lambda (x) x))
      (defalias 'neovm-function-alias-a 'neovm-function-alias-base)
      (defalias 'neovm-function-alias-b 'neovm-function-alias-a)
      (defalias 'neovm-function-alias-subr '+)
      (with-temp-buffer
        (list
         (function-alias-p 'neovm-function-alias-base)
         (function-alias-p 'neovm-function-alias-a)
         (function-alias-p 'neovm-function-alias-b)
         (function-alias-p 'neovm-function-alias-subr)
         (function-alias-p (lambda (x) x))
         (function-alias-p 'neovm-function-alias-missing)
         (mapcar (lambda (x)
                   (let ((r (readablep x)))
                     (list (type-of x) (if r t nil) r)))
                 (list nil
                       t
                       42
                       "str"
                       'sym
                       [a b]
                       (make-symbol "uninterned")
                       (current-buffer)
                       (point-marker)
                       (make-hash-table)))))))
  (mapc #'fmakunbound
        '(neovm-function-alias-base
          neovm-function-alias-a
          neovm-function-alias-b
          neovm-function-alias-subr)))"#;
    assert_oracle_parity_with_bootstrap(form);
}
