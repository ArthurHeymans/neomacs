use expect_test::expect;

use super::assert_ah_parity;

#[test]
fn ah_keyboard_quit_workflow_runs_before_immediately_and_after_on_post_command() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-c-g-hook
                (list (lambda () (push 'before events))))
               (ah-after-c-g-hook
                (list (lambda () (push 'after events))))
               post-command-hook)
         (list
          (ah--cg-keyboard-quit
           (lambda () (push 'original events) 'quit-result))
          (not
           (null
            (memq #'ah--cg-post-processing post-command-hook)))
          (progn
            (let ((this-command 'keyboard-quit))
              (run-hooks 'post-command-hook))
            (nreverse events))
          post-command-hook))"##;
    let expect = expect!["OK (quit-result t (before original after) nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_isearch_abort_workflow_runs_the_same_delayed_after_hook() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-c-g-hook
                (list (lambda () (push 'before events))))
               (ah-after-c-g-hook
                (list (lambda () (push 'after events))))
               post-command-hook)
         (ah--cg-isearch-abort
          (lambda () (push 'isearch-abort events) 'aborted))
         (let ((this-command 'isearch-abort))
           (run-hooks 'post-command-hook))
         (list (nreverse events) post-command-hook))"##;
    let expect = expect!["OK ((before isearch-abort after) nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_post_processing_ignores_other_commands_but_always_removes_itself() {
    let elisp_form = r##"(let* ((events nil)
               (ah-after-c-g-hook
                (list (lambda () (push 'after events))))
               (post-command-hook '(ah--cg-post-processing))
               (this-command 'self-insert-command))
         (ah--cg-post-processing)
         (list events post-command-hook))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_empty_after_quit_hook_does_not_install_post_command_processing() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-c-g-hook
                (list (lambda () (push 'before events))))
               (ah-after-c-g-hook nil)
               post-command-hook)
         (list
          (ah--cg-keyboard-quit
           (lambda () (push 'original events) 17))
          (nreverse events)
          post-command-hook))"##;
    let expect = expect!["OK (17 (before original) nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_quit_wrapper_preserves_original_error_and_pending_after_hook() {
    let elisp_form = r##"(let* ((events nil)
               (ah-before-c-g-hook
                (list (lambda () (push 'before events))))
               (ah-after-c-g-hook
                (list (lambda () (push 'after events))))
               post-command-hook)
         (list
          (condition-case error-data
              (ah--cg-keyboard-quit
               (lambda () (push 'original events) (error "quit failed")))
            (error (list (car error-data) (cadr error-data))))
          (nreverse events)
          (not
           (null
            (memq #'ah--cg-post-processing post-command-hook)))))"##;
    let expect = expect![[r#"OK ((error "quit failed") (before original) t)"#]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_repeated_quit_wrappers_schedule_only_one_post_command_callback() {
    let elisp_form = r##"(let ((ah-before-c-g-hook nil)
               (ah-after-c-g-hook '(ignore))
               post-command-hook)
         (ah--cg-keyboard-quit #'ignore)
         (ah--cg-isearch-abort #'ignore)
         (list
          (length post-command-hook)
          (eq (car post-command-hook) #'ah--cg-post-processing)))"##;
    let expect = expect!["OK (1 t)"];
    assert_ah_parity(elisp_form, expect);
}
