use crate::emacs_core::format_eval_result;
use crate::emacs_core::value::Value;

/// GNU `Fget_truename_buffer` (src/buffer.c:524-539) returns the live buffer
/// whose `buffer-file-truename` is `string-equal` to FILENAME.  It used to be
/// a stub returning nil here, which silently disabled the supersession check
/// in `lock_file` (filelock.c:603).
#[test]
fn get_truename_buffer_finds_the_visiting_buffer_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .set_buffer_file_truename(current, Value::string("/work/note.txt"))
        .expect("set buffer-file-truename");

    assert_eq!(
        format_eval_result(&eval.eval_str(
            r#"(eq (get-truename-buffer "/work/note.txt")
                                                 (current-buffer))"#
        )),
        "OK t",
    );
    assert_eq!(
        format_eval_result(&eval.eval_str(r#"(get-truename-buffer "/work/other.txt")"#)),
        "OK nil",
        "GNU compares truenames literally, with no expansion or fallback"
    );
}

/// GNU never expands or canonicalizes either side: `find-file` has already
/// stored the truename, so a relative or unexpanded FILENAME does not match.
#[test]
fn get_truename_buffer_does_not_expand_its_argument_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let current = eval.buffers.current_buffer_id().expect("current buffer");
    eval.buffers
        .set_buffer_file_truename(current, Value::string("/work/note.txt"))
        .expect("set buffer-file-truename");

    assert_eq!(
        format_eval_result(&eval.eval_str(r#"(get-truename-buffer "note.txt")"#)),
        "OK nil",
    );
}
