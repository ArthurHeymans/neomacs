use expect_test::expect;

use super::assert_ah_parity;

#[test]
fn ah_programmatic_cursor_wrappers_forward_every_argument_without_running_user_hooks() {
    let elisp_form = r##"(let ((ah-before-move-cursor-hook
                (list (lambda () (push 'before events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push 'after events))))
               events calls)
         (list
          (ah--cur-next-line
           (lambda (&rest args) (push (cons 'next args) calls) 'next-result)
           2 t)
          (ah--cur-previous-line
           (lambda (&rest args) (push (cons 'previous args) calls) 'previous-result)
           nil nil)
          (ah--cur-forward-char
           (lambda (&rest args) (push (cons 'forward args) calls) 'forward-result)
           3)
          (ah--cur-backward-char
           (lambda (&rest args) (push (cons 'backward args) calls) 'backward-result)
           nil)
          (ah--cur-syntax-subword-forward
           (lambda (&rest args) (push (cons 'subword-forward args) calls) 'sf-result)
           4)
          (ah--cur-syntax-subword-backward
           (lambda (&rest args) (push (cons 'subword-backward args) calls) 'sb-result)
           5)
          (ah--cur-move-beginning-of-line
           (lambda (&rest args) (push (cons 'bol args) calls) 'bol-result)
           6)
          (ah--cur-move-end-of-line
           (lambda (&rest args) (push (cons 'eol args) calls) 'eol-result)
           7)
          (ah--cur-beginning-of-buffer
           (lambda (&rest args) (push (cons 'bob args) calls) 'bob-result)
           8)
          (ah--cur-end-of-buffer
           (lambda (&rest args) (push (cons 'eob args) calls) 'eob-result)
           nil)
          (nreverse calls)
          events))"##;
    let expect = expect![
        "OK (nil nil nil nil nil nil nil nil nil nil ((next 2 t) (previous nil nil) (forward 3) (backward nil) (subword-forward 4) (subword-backward 5) (bol 6) (eol 7) (bob 8) (eob nil)) nil)"
    ];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_horizontal_cursor_workflow_reports_before_and_after_positions() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcd")
         (goto-char 2)
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push (list 'before (point)) events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push (list 'after (point)) events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (call-interactively #'forward-char)
                 (call-interactively #'backward-char)
                 (list (point) (nreverse events)))
             (ah-mode -1))))"##;
    let expect = expect!["OK (2 ((before 2) (after 3) (before 3) (after 2)))"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_vertical_cursor_workflow_crosses_lines_and_preserves_column() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\nlonger\nx\n")
         (goto-char 2)
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push (list 'before (line-number-at-pos) (current-column)) events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push (list 'after (line-number-at-pos) (current-column)) events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (call-interactively #'next-line)
                 (call-interactively #'next-line)
                 (call-interactively #'previous-line)
                 (list (line-number-at-pos) (current-column) (nreverse events)))
             (ah-mode -1))))"##;
    let expect = expect![
        "OK (2 1 ((before 1 1) (after 2 1) (before 2 1) (after 3 1) (before 3 1) (after 2 1)))"
    ];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_line_boundary_workflow_observes_each_completed_move() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha\nbeta gamma\nomega")
         (goto-char 9)
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push (list 'before (point)) events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push (list 'after (point)) events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (call-interactively #'move-beginning-of-line)
                 (call-interactively #'move-end-of-line)
                 (list (point) (buffer-substring (line-beginning-position)
                                                 (line-end-position))
                       (nreverse events)))
             (ah-mode -1))))"##;
    let expect = expect![[r#"OK (17 "beta gamma" ((before 9) (after 7) (before 7) (after 17)))"#]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_buffer_boundary_workflow_handles_interactive_prefixes() {
    let elisp_form = r##"(with-temp-buffer
         (dotimes (index 20)
           (insert (format "line-%02d\n" index)))
         (goto-char 40)
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push (list 'before (point)) events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push (list 'after (point)) events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (let ((current-prefix-arg '(4)))
                   (call-interactively #'beginning-of-buffer))
                 (let ((after-beginning (point)))
                   (let ((current-prefix-arg '(4)))
                     (call-interactively #'end-of-buffer))
                   (list after-beginning (point) (nreverse events))))
             (ah-mode -1))))"##;
    let expect = expect!["OK (1 161 ((before 40) (after 1) (before 1) (after 161)))"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_real_syntax_subword_workflow_navigates_camel_case_components() {
    let elisp_form = r##"(with-temp-buffer
         (require 'subword)
         (emacs-lisp-mode)
         (insert "alphaBetaGamma")
         (goto-char (point-min))
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push (list 'before (point)) events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push (list 'after (point)) events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (cl-letf (((symbol-function 'called-interactively-p)
                            (lambda (&rest _) t)))
                   (ah--cur-syntax-subword-forward
                    #'subword-forward nil)
                   (ah--cur-syntax-subword-forward
                    #'subword-forward nil)
                   (ah--cur-syntax-subword-backward
                    #'subword-backward nil))
                 (list (point) (nreverse events)))
             (ah-mode -1))))"##;
    let expect =
        expect!["OK (6 ((before 1) (after 6) (before 6) (after 10) (before 10) (after 6)))"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_programmatic_real_cursor_calls_remain_silent_while_mode_is_enabled() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abc\ndef")
         (goto-char 1)
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push 'before events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push 'after events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (forward-char 2)
                 (next-line 1)
                 (move-end-of-line 1)
                 (list (point) events))
             (ah-mode -1))))"##;
    let expect = expect!["OK (8 nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_after_cursor_hook_is_skipped_when_the_wrapped_motion_signals() {
    let elisp_form = r##"(let ((ah-before-move-cursor-hook
                (list (lambda () (push 'before events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push 'after events))))
               events)
         (cl-letf (((symbol-function 'called-interactively-p)
                    (lambda (&rest _) t)))
           (list
            (condition-case error-data
                (ah--cur-forward-char
                 (lambda (&rest _) (error "motion failed"))
                 2)
              (error (list (car error-data) (cadr error-data))))
            (nreverse events))))"##;
    let expect = expect!["OK ((void-variable events) nil)"];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_before_cursor_hook_can_transform_the_buffer_before_real_motion() {
    let elisp_form = r##"(with-temp-buffer
         (insert "bc")
         (goto-char 1)
         (let* ((observed nil)
                (ah-before-move-cursor-hook
                (list (lambda () (insert "a"))))
               (ah-after-move-cursor-hook
                (list (lambda () (setq observed
                                       (list (buffer-string) (point)))))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (call-interactively #'forward-char)
                 (list (buffer-string) (point) observed))
             (ah-mode -1))))"##;
    let expect = expect![[r#"OK ("abc" 3 ("abc" 3))"#]];
    assert_ah_parity(elisp_form, expect);
}

#[test]
fn ah_multiple_cursor_hook_functions_follow_standard_hook_order() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abc")
         (goto-char 1)
         (let* ((events nil)
                (ah-before-move-cursor-hook
                (list (lambda () (push 'before-a events))
                      (lambda () (push 'before-b events))))
               (ah-after-move-cursor-hook
                (list (lambda () (push 'after-a events))
                      (lambda () (push 'after-b events)))))
           (unwind-protect
               (progn
                 (ah-mode 1)
                 (call-interactively #'forward-char)
                 (nreverse events))
             (ah-mode -1))))"##;
    let expect = expect!["OK (before-a before-b after-a after-b)"];
    assert_ah_parity(elisp_form, expect);
}
