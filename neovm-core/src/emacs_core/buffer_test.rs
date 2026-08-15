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

/// GNU's `buffer-enable-undo` (src/buffer.c:1845-1847) resets the list ONLY
/// when undo is actually off:
///
/// ```c
///   if (EQ (BVAR (XBUFFER (real_buffer), undo_list), Qt))
///     bset_undo_list (XBUFFER (real_buffer), Qnil);
/// ```
///
/// Ours reset unconditionally, which destroyed an existing history.  The
/// damage is worst through an indirect buffer, because an indirect buffer
/// shares its base's undo list (`make_indirect_buffer`, src/buffer.c:894, plus
/// the per-switch resync in `set_buffer_internal_2`, src/buffer.c:2352-2367):
/// enabling undo in the indirect buffer wiped the BASE buffer's history.
///
/// The expected values were taken by running this exact form under GNU Emacs
/// 31.0.90 `-Q --batch`.  Each step is snapshotted with `prin1-to-string` at
/// capture time on purpose -- `record_insert` coalesces by mutating the head
/// cons in place (src/undo.c:109), so collecting the live lists and printing
/// them at the end reports the FINAL state for every earlier step.
#[test]
fn buffer_enable_undo_keeps_an_existing_list_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();

    assert_eq!(
        format_eval_result(&eval.eval_str(
            r#"(let* ((base (get-buffer-create "b2")) s1 s2 s3 s4)
                 (set-buffer base) (buffer-enable-undo) (insert "hello")
                 (setq s1 (prin1-to-string buffer-undo-list))
                 (let ((ind (make-indirect-buffer base "i2")))
                   (setq s2 (prin1-to-string buffer-undo-list))
                   (set-buffer ind) (buffer-enable-undo)
                   (set-buffer base) (setq s3 (prin1-to-string buffer-undo-list))
                   (set-buffer ind) (insert "Y")
                   (set-buffer base) (setq s4 (prin1-to-string buffer-undo-list)))
                 (list s1 s2 s3 s4))"#
        )),
        r#"OK ("((1 . 6) (t . 0))" "((1 . 6) (t . 0))" "((1 . 6) (t . 0))" "((1 . 7) (t . 0))")"#,
    );
}

/// The same clobber without any indirection: `buffer-enable-undo` on a buffer
/// that already has a history is a no-op in GNU.
#[test]
fn buffer_enable_undo_is_a_noop_when_undo_is_already_on_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();

    assert_eq!(
        format_eval_result(&eval.eval_str(
            r#"(let ((before nil))
                 (set-buffer (get-buffer-create "b4"))
                 (setq buffer-undo-list nil)
                 (insert "abc")
                 (setq before (prin1-to-string buffer-undo-list))
                 (buffer-enable-undo)
                 (list before (prin1-to-string buffer-undo-list)))"#
        )),
        r#"OK ("((1 . 4) (t . 0))" "((1 . 4) (t . 0))")"#,
    );
}

/// `buffer-enable-undo` still has to turn undo back ON when it is off, and it
/// does so for the whole shared chain: the indirect buffer and its base read
/// one list, so disabling undo through the indirect buffer leaves the BASE at
/// `t` and re-enabling through it clears the base back to nil.  Both halves
/// confirmed under GNU 31.0.90.
#[test]
fn buffer_enable_undo_still_clears_a_disabled_list_through_an_indirect_buffer() {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();

    assert_eq!(
        format_eval_result(&eval.eval_str(
            r#"(let* ((base (get-buffer-create "b5")) disabled)
                 (set-buffer base) (buffer-enable-undo) (insert "hello")
                 (let ((ind (make-indirect-buffer base "i5")))
                   (set-buffer ind) (buffer-disable-undo)
                   (set-buffer base) (setq disabled (prin1-to-string buffer-undo-list))
                   (set-buffer ind) (buffer-enable-undo)
                   (set-buffer base)
                   (list disabled (prin1-to-string buffer-undo-list))))"#
        )),
        r#"OK ("t" "nil")"#,
    );
}
