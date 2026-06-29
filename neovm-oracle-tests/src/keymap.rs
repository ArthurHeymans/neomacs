//! Oracle parity tests for keymap primitives.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_keymap_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(keymapp (make-keymap))",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(keymapp (make-sparse-keymap))",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_prop_keymap_copy_and_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(let* ((m (make-sparse-keymap))) (define-key m [24] 'foo) (let ((c (copy-keymap m))) (list (lookup-key c [24]) (keymapp c))))";
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK (foo t)""#]],
    );
    assert_ok_eq("(foo t)", &oracle, &neovm);
}

#[test]
fn oracle_prop_keymap_parent_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = "(let* ((p (make-sparse-keymap)) (c (make-sparse-keymap))) (set-keymap-parent c p) (eq (keymap-parent c) p))";
    let (oracle, neovm) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_prop_keymap_wrong_type_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle_copy, neovm_copy) = crate::common::eval_oracle_and_neovm_expect(
        "(copy-keymap 1)",
        expect_test::expect![[r#""ERR (wrong-type-argument keymapp 1)""#]],
    );
    assert_err_kind(&oracle_copy, &neovm_copy, "wrong-type-argument");

    let (oracle_parent, neovm_parent) = crate::common::eval_oracle_and_neovm_expect(
        "(set-keymap-parent 1 2)",
        expect_test::expect![[r#""ERR (wrong-type-argument keymapp 1)""#]],
    );
    assert_err_kind(&oracle_parent, &neovm_parent, "wrong-type-argument");
}
