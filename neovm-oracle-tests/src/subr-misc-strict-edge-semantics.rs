//! Strict oracle parity for small GNU `subr.el` helpers.
//!
//! These helpers are pure Lisp in GNU Emacs.  The cases below target exact
//! regex/path handling, dynamic variable dependence, and hash/obarray side
//! effects that are easy to approximate incorrectly.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_subr_misc_package_unmsys_prefix_apropos_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (mapcar #'package--description-file
         '("/tmp/pkg-1.2.3/"
           "/tmp/pkg-1pre2"
           "/tmp/pkg-1.0beta3"
           "/tmp/pkg-1snapshot4"
           "/tmp/.hidden-1.2"
           "/tmp/pkg-nover"
           "/tmp/pkg-1.2.3-extra"
           "/tmp/pkg-20200101"))
 (let ((system-type 'gnu/linux))
   (mapcar #'unmsys--file-name
           '("/c/foo/bar" "/C/foo" "/notdrive/foo" "relative")))
 (let ((system-type 'windows-nt))
   (mapcar #'unmsys--file-name
           '("/c/foo/bar" "/C/foo" "/notdrive/foo" "relative")))
 (let ((definition-prefixes (make-hash-table :test 'equal)))
   (register-definition-prefixes "a.el" '("foo-" "bar-"))
   (register-definition-prefixes "b.el" '("foo-"))
   (list (gethash "foo-" definition-prefixes)
         (gethash "bar-" definition-prefixes)
         (gethash "missing-" definition-prefixes)))
 (let ((obarray (make-vector 17 0)))
   (set (intern "nmo-alpha") 1)
   (intern "nmo-beta")
   (intern "other")
   (list (apropos-internal "\\`nmo-")
         (apropos-internal "\\`nmo-" #'boundp))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
