//! Oracle parity tests for `get-file-buffer`.
//!
//! GNU implements `get-file-buffer` in `src/buffer.c` via `Fget_file_buffer`,
//! which expands the filename and scans live buffers for a matching `buffer-file-name`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_get_file_buffer_nil_for_nonexistent_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(get-file-buffer \"/nonexistent/path/to/nowhere.txt\")",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_get_file_buffer_requires_string_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(get-file-buffer 42)",
        expect_test::expect![[r#""ERR (wrong-type-argument stringp 42)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_get_file_buffer_wrong_number_of_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(get-file-buffer)",
        expect_test::expect![[r#""ERR (wrong-number-of-arguments get-file-buffer 0)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");

    let (oracle2, neovm2) = crate::common::eval_oracle_and_neovm_expect(
        "(get-file-buffer \"a\" \"b\")",
        expect_test::expect![[r#""ERR (wrong-number-of-arguments get-file-buffer 2)""#]],
    );
    assert_err_kind(&oracle2, &neovm2, "wrong-number-of-arguments");
}
