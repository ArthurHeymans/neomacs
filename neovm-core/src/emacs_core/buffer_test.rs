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

/// GNU's `general_insert_function` (src/editfns.c:1307-1345) converts and
/// inserts one argument at a time, so `wrong_type_argument` for a later
/// argument leaves every preceding argument already in the buffer.  Neomacs
/// used to validate the whole argument vector before touching the buffer, so a
/// package that passed a valid prefix plus a bad value inserted nothing.
///
/// Verified expectations come from running the same forms under GNU Emacs:
///   (insert "ab" '(1) "c")  => buffer "ab",    point 3
///   (insert "pick " '("x") "\n") => buffer "pick ", point 6, modified t
#[test]
fn insert_keeps_the_arguments_it_already_inserted_before_signalling_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();

    assert_eq!(
        format_eval_result(&eval.eval_str(
            r#"(progn
                  (erase-buffer)
                  (list (condition-case e (insert "ab" '(1) "c") (error e))
                        (buffer-string)
                        (point)))"#
        )),
        r#"OK ((wrong-type-argument char-or-string-p (1)) "ab" 3)"#,
    );

    assert_eq!(
        format_eval_result(&eval.eval_str(
            r#"(progn
                  (erase-buffer)
                  (list (condition-case e (insert "pick " '("x") "\n") (error e))
                        (buffer-string)
                        (point)
                        (buffer-modified-p)))"#
        )),
        r#"OK ((wrong-type-argument char-or-string-p ("x")) "pick " 6 t)"#,
    );
}

/// GNU signals the same way for every member of the `general_insert_function`
/// family, and each variant still inserts its valid prefix first.
#[test]
fn insert_variants_all_insert_their_valid_prefix_before_signalling_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();

    for name in [
        "insert",
        "insert-before-markers",
        "insert-and-inherit",
        "insert-before-markers-and-inherit",
    ] {
        assert_eq!(
            format_eval_result(&eval.eval_str(&format!(
                r#"(progn
                      (erase-buffer)
                      (list (condition-case e ({name} "ab" 67 '(1)) (error e))
                            (buffer-string)))"#
            ))),
            r#"OK ((wrong-type-argument char-or-string-p (1)) "abC")"#,
            "{name} must insert its valid prefix before signalling"
        );
    }
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
