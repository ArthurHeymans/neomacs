use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_user_guards_return_values_report_one_error_and_recover() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (&rest arguments)
                      (push arguments messages)
                      'messaged)))
           (let ((truthy
                  (let ((aggressive-indent-dont-indent-if
                         '((> 5 2))))
                    (aggressive-indent--run-user-hooks)))
                 first-error
                 second-error
                 recovered
                 third-error)
             (setq aggressive-indent--has-errored nil)
             (let ((aggressive-indent-dont-indent-if
                    '((error "broken guard"))))
               (setq first-error
                     (aggressive-indent--run-user-hooks)
                     second-error
                     (aggressive-indent--run-user-hooks)))
             (let ((aggressive-indent-dont-indent-if
                    '((stringp "healthy"))))
               (setq recovered
                     (aggressive-indent--run-user-hooks)))
             (let ((aggressive-indent-dont-indent-if
                    '((error "broken again"))))
               (setq third-error
                     (aggressive-indent--run-user-hooks)))
             (list
              truthy
              first-error
              second-error
              recovered
              third-error
              aggressive-indent--has-errored
              (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (t messaged nil t messaged t (("One of the forms in `aggressive-indent-dont-indent-if' had the following error, I've disabled it until you fix it: %S" (error "broken guard")) ("One of the forms in `aggressive-indent-dont-indent-if' had the following error, I've disabled it until you fix it: %S" (error "broken again"))))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_processes_recent_changes_in_lifo_order_and_uses_mode_strategy() {
    let elisp_form = r##"(let (all-events)
         (dolist (mode '(emacs-lisp-mode fundamental-mode))
           (with-temp-buffer
             (funcall mode)
             (insert "(value)\n")
             (set-buffer-modified-p t)
             (setq aggressive-indent--changed-list
                   (mapcar
                    (lambda (number)
                      (list number (+ number 10)))
                    (number-sequence 1 13)))
             (let (events)
               (cl-letf (((symbol-function
                           'aggressive-indent--softly-indent-defun)
                          (lambda (&rest limits)
                            (push
                             (cons 'defun limits)
                             events)))
                         ((symbol-function
                           'aggressive-indent--softly-indent-region-and-on)
                          (lambda (&rest limits)
                            (push
                             (cons 'region limits)
                             events))))
                 (aggressive-indent--process-changed-list-and-indent)
                 (push
                  (list
                   mode
                   (nreverse events)
                   aggressive-indent--changed-list)
                  all-events)))))
         (nreverse all-events))"##;
    let expect = expect![
        "OK ((emacs-lisp-mode ((defun 1 11) (defun 2 12) (defun 3 13) (defun 4 14) (defun 5 15) (defun 6 16) (defun 7 17) (defun 8 18) (defun 9 19) (defun 10 20) (defun 11 21)) nil) (fundamental-mode ((region 1 11) (region 2 12) (region 3 13) (region 4 14) (region 5 15) (region 6 16) (region 7 17) (region 8 18) (region 9 19) (region 10 20) (region 11 21)) nil))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_processing_respects_protected_commands_regions_and_custom_guards() {
    let elisp_form = r##"(let (results)
         (dolist (scenario
                  '((protected-last undo nil nil)
                    (protected-current nil query-replace nil)
                    (active-region nil nil region)
                    (custom nil nil custom)
                    (allowed nil self-insert-command nil)))
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert "(defun x ()\n(message \"x\"))")
             (set-buffer-modified-p t)
             (setq aggressive-indent--changed-list
                   '((1 10)))
             (let ((last-command (nth 1 scenario))
                   (this-command (nth 2 scenario))
                   (aggressive-indent-dont-indent-if
                    (if (eq (nth 3 scenario) 'custom)
                        '(t)
                      nil))
                   calls)
               (when (eq (nth 3 scenario) 'region)
                 (goto-char (point-max))
                 (set-mark (point-min))
                 (setq mark-active t
                       transient-mark-mode t))
               (cl-letf (((symbol-function
                           'aggressive-indent--softly-indent-defun)
                          (lambda (&rest limits)
                            (push limits calls))))
                 (aggressive-indent--process-changed-list-and-indent)
                 (push
                  (list
                   (car scenario)
                   (nreverse calls)
                   aggressive-indent--changed-list)
                  results)))))
         (nreverse results))"##;
    let expect = expect![
        "OK ((protected-last nil #1=((1 10))) (protected-current nil #1#) (active-region nil #1#) (custom nil #1#) (allowed ((1 10)) nil))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_mode_tracks_real_edits_then_before_save_repairs_the_defun() {
    let elisp_form = r##"(let (scheduled)
         (cl-letf (((symbol-function 'run-with-idle-timer)
                    (lambda (&rest arguments)
                      (push arguments scheduled)
                      'fake-idle-timer)))
           (with-temp-buffer
             (emacs-lisp-mode)
             (aggressive-indent-mode 1)
             (insert
              "(defun workflow (enabled)\n"
              "(when enabled\n"
              "(message \"first\")\n"
              "(message \"second\")))\n")
             (let ((changes-before-save
                    (copy-tree
                     aggressive-indent--changed-list))
                   (hooks-before-save
                    (list
                     (memq
                      #'aggressive-indent--keep-track-of-changes
                      after-change-functions)
                     (memq
                      #'aggressive-indent--process-changed-list-and-indent
                      before-save-hook))))
               (run-hooks 'before-save-hook)
               (list
                changes-before-save
                aggressive-indent--changed-list
                hooks-before-save
                (buffer-string)
                (mapcar
                 (lambda (call)
                   (list
                    (nth 0 call)
                    (nth 1 call)
                    (nth 2 call)
                    (bufferp (nth 3 call))))
                 (nreverse scheduled)))))))"##;
    let expect = expect![[
        r#"OK (((59 80) (41 59) (27 41) (1 27)) nil ((aggressive-indent--keep-track-of-changes t) (aggressive-indent--process-changed-list-and-indent t)) "(defun workflow (enabled)\n  (when enabled\n    (message \"first\")\n    (message \"second\")))\n" ((0.05 t aggressive-indent--indent-if-changed t) (0.05 t aggressive-indent--indent-if-changed t) (0.05 t aggressive-indent--indent-if-changed t) (0.05 t aggressive-indent--indent-if-changed t)))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_non_lisp_workflow_reformats_real_c_edits_before_save() {
    let elisp_form = r##"(cl-letf (((symbol-function 'run-with-idle-timer)
                    (lambda (&rest _arguments)
                      'deterministic-timer)))
         (with-temp-buffer
           (c-mode)
           (aggressive-indent-mode 1)
           (insert
            "int main(void) {\n"
            "if (1) {\n"
            "printf(\"value=%d\\n\", 1);\n"
            "}\n"
            "return 0;\n"
            "}\n")
           (let ((before
                  (buffer-string))
                 (changes
                  (copy-tree
                   aggressive-indent--changed-list)))
             (run-hooks 'before-save-hook)
             (list
              before
              (buffer-string)
              changes
              aggressive-indent--changed-list
              (buffer-modified-p)))))"##;
    let expect = expect![[
        r#"OK ("int main(void) {\nif (1) {\nprintf(\"value=%d\\n\", 1);\n}\nreturn 0;\n}\n" "int main(void) {\n  if (1) {\n    printf(\"value=%d\\n\", 1);\n }\n return 0;\n}\n" ((64 66) (54 64) (52 54) (27 52) (18 27) (1 18)) nil t)"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_change_tracking_cancels_prior_timer_and_schedules_buffer_callback() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (setq aggressive-indent-mode t
               aggressive-indent--idle-timer
               (timer-create)
               aggressive-indent-sit-for-time 0.125)
         (let (cancelled scheduled)
           (cl-letf (((symbol-function 'cancel-timer)
                      (lambda (timer)
                        (push (timerp timer) cancelled)))
                     ((symbol-function 'run-with-idle-timer)
                      (lambda (&rest arguments)
                        (push arguments scheduled)
                        'replacement-timer)))
             (aggressive-indent--keep-track-of-changes
              4 11 2)
             (aggressive-indent--keep-track-of-changes
              8 9)
             (list
              aggressive-indent--changed-list
              aggressive-indent--idle-timer
              (nreverse cancelled)
              (mapcar
               (lambda (call)
                 (list
                  (nth 0 call)
                  (nth 1 call)
                  (nth 2 call)
                  (bufferp (nth 3 call))))
               (nreverse scheduled))))))"##;
    let expect = expect![
        "OK (((8 9) (4 11)) replacement-timer (t) ((0.125 t aggressive-indent--indent-if-changed t) (0.125 t aggressive-indent--indent-if-changed t)))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_clear_change_list_supports_revert_and_is_buffer_local() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *aggressive-first*"))
               (second
                (generate-new-buffer
                 " *aggressive-second*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (setq aggressive-indent--changed-list
                       '((1 2) (3 4))))
               (with-current-buffer second
                 (setq aggressive-indent--changed-list
                       '((9 10))))
               (with-current-buffer first
                 (aggressive-indent--clear-change-list))
               (list
                (with-current-buffer first
                  (list
                   aggressive-indent--changed-list
                   (local-variable-p
                    'aggressive-indent--changed-list)))
                (with-current-buffer second
                  (list
                   aggressive-indent--changed-list
                   (local-variable-p
                    'aggressive-indent--changed-list)))))
           (when (buffer-live-p first)
             (kill-buffer first))
           (when (buffer-live-p second)
             (kill-buffer second))))"##;
    let expect = expect!["OK ((nil t) (((9 10)) t))"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_comment_policy_changes_real_post_edit_processing_at_point() {
    let elisp_form = r##"(let (results)
         (dolist (comments-too '(nil t))
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(defun sample ()\n"
              ";; comment body\n"
              "(message \"value\"))\n")
             (goto-char (point-min))
             (search-forward "comment")
             (set-buffer-modified-p t)
             (setq aggressive-indent--changed-list
                   '((18 25)))
             (let ((aggressive-indent-comments-too
                    comments-too)
                   calls)
               (cl-letf (((symbol-function
                           'aggressive-indent--softly-indent-defun)
                          (lambda (&rest limits)
                            (push limits calls))))
                 (aggressive-indent--process-changed-list-and-indent)
                 (push
                  (list
                   comments-too
                   (nreverse calls)
                   aggressive-indent--changed-list)
                  results)))))
         (nreverse results))"##;
    let expect = expect!["OK ((nil nil ((18 25))) (t ((18 25)) nil))"];
    assert_aggressive_indent_parity(elisp_form, expect);
}
