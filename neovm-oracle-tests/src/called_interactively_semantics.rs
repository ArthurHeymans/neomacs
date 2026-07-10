//! Oracle parity tests for GNU `called-interactively-p` and `interactive-p`.
//!
//! GNU implements these in `lisp/subr.el` by inspecting the backtrace around
//! `funcall-interactively` and by honoring dynamic `noninteractive` and
//! `executing-kbd-macro` bindings for KIND `interactive`.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn oracle_prop_gnu_called_interactively_batch_contracts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defun neovm--oracle-ci-target ()
    (interactive)
    (list (called-interactively-p)
          (called-interactively-p 'any)
          (called-interactively-p 'interactive)
          (interactive-p)))
  (unwind-protect
      (list
       (neovm--oracle-ci-target)
       (call-interactively 'neovm--oracle-ci-target)
       (funcall-interactively 'neovm--oracle-ci-target)
       (command-execute 'neovm--oracle-ci-target))
    (fmakunbound 'neovm--oracle-ci-target)))
"#;

    let expect = expect_test::expect![
        r#""OK ((nil nil nil nil) (t t nil nil) (t t nil nil) (t t nil nil))""#
    ];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn oracle_prop_gnu_called_interactively_dynamic_gates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defun neovm--oracle-ci-dynamic-target ()
    (interactive)
    (list
     (called-interactively-p 'bad-kind)
     (let ((executing-kbd-macro t))
       (list (called-interactively-p 'any)
             (called-interactively-p 'interactive)))
     (let ((noninteractive nil))
       (called-interactively-p 'interactive))))
  (unwind-protect
      (list
       (call-interactively 'neovm--oracle-ci-dynamic-target)
       (funcall-interactively 'neovm--oracle-ci-dynamic-target)
       (command-execute 'neovm--oracle-ci-dynamic-target))
    (fmakunbound 'neovm--oracle-ci-dynamic-target)))
"#;

    let expect = expect_test::expect![r#""OK ((t (t nil) t) (t (t nil) t) (t (t nil) t))""#];
    crate::common::assert_oracle_parity_expect(form, expect);
}
