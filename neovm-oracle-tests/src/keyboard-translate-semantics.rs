//! Oracle parity tests for GNU `subr.el` `keyboard-translate`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm_with_bootstrap};

#[test]
fn oracle_keyboard_translate_creates_table_and_sets_character_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (let ((keyboard-translate-table nil))
   (list (keyboard-translate ?a ?b)
         (char-table-p keyboard-translate-table)
         (char-table-subtype keyboard-translate-table)
         (aref keyboard-translate-table ?a)
         (aref keyboard-translate-table ?b)
         (char-table-range keyboard-translate-table ?a)
         (char-table-range keyboard-translate-table ?b)))
 (let ((keyboard-translate-table 'stale))
   (list (keyboard-translate ?x ?y)
         (char-table-p keyboard-translate-table)
         (char-table-subtype keyboard-translate-table)
         (aref keyboard-translate-table ?x)
         (condition-case e
             (keyboard-translate "x" ?z)
           (error (list (car e) (cadr e)))))))"#;
    let (oracle, neovm) = eval_oracle_and_neovm_with_bootstrap(form);
    assert_ok_eq(
        "((98 t keyboard-translate-table 98 nil 98 nil) (121 t keyboard-translate-table 121 (wrong-type-argument fixnump)))",
        &oracle,
        &neovm,
    );
}
