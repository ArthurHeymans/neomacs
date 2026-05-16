//! Oracle parity tests for GNU DBus inhibitor lock primitives.
//!
//! GNU implements these in `src/dbusbind.c`: the inhibitor-lock registry starts
//! empty, and argument type checks run before DBus side effects.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_dbus_inhibitor_lock_argument_checks_and_initial_registry() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (dbus-registered-inhibitor-locks)
 (condition-case err
     (dbus-close-inhibitor-lock "not-a-lock")
   (error (cons (car err) (cdr err))))
 (condition-case err
     (dbus-make-inhibitor-lock 1 "why")
   (error (cons (car err) (cdr err))))
 (condition-case err
     (dbus-make-inhibitor-lock "sleep" 2)
   (error (cons (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
