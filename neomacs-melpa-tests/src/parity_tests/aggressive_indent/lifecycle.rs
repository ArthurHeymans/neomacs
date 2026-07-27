use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_local_mode_installs_and_removes_complete_buffer_lifecycle() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (let ((before-electric electric-indent-mode))
           (aggressive-indent-mode 1)
           (let ((enabled
                  (list
                   aggressive-indent-mode
                   (key-binding (kbd "C-c C-q"))
                   (car
                    (cdr
                     (assq
                      'backspace
                      aggressive-indent-mode-map)))
                   (memq
                    #'aggressive-indent--keep-track-of-changes
                    after-change-functions)
                   (memq
                    #'aggressive-indent--clear-change-list
                    after-revert-hook)
                   (memq
                    #'aggressive-indent--process-changed-list-and-indent
                    before-save-hook)
                   (memq
                    #'aggressive-indent--maybe-cancel-timer
                    kill-buffer-hook)
                   electric-indent-mode
                   (local-variable-p
                    'electric-indent-mode))))
             (aggressive-indent-mode -1)
             (list
              before-electric
              enabled
              (list
               aggressive-indent-mode
               (memq
                #'aggressive-indent--keep-track-of-changes
                after-change-functions)
               (memq
                #'aggressive-indent--clear-change-list
                after-revert-hook)
               (memq
                #'aggressive-indent--process-changed-list-and-indent
                before-save-hook)
               (memq
                #'aggressive-indent--maybe-cancel-timer
                kill-buffer-hook))))))"##;
    let expect = expect![
        "OK (t (t aggressive-indent-indent-defun menu-item (aggressive-indent--keep-track-of-changes t) (aggressive-indent--clear-change-list t) (aggressive-indent--process-changed-list-and-indent t) (aggressive-indent--maybe-cancel-timer t) t nil) (nil nil nil nil nil))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_mode_obeys_boolean_and_derived_mode_electric_preferences() {
    let elisp_form = r##"(progn
         (define-derived-mode
           aggressive-indent-parity-child-mode
           emacs-lisp-mode
           "AggChild")
         (mapcar
          (lambda (preference)
            (with-temp-buffer
              (aggressive-indent-parity-child-mode)
              (let ((aggressive-indent-dont-electric-modes
                     preference))
                (aggressive-indent-mode 1)
                (list
                 preference
                 aggressive-indent-mode
                 electric-indent-mode
                 (local-variable-p
                  'electric-indent-mode)))))
          '(nil
            t
            (emacs-lisp-mode)
            (fundamental-mode))))"##;
    let expect = expect![
        "OK ((nil t t nil) (t t nil t) ((emacs-lisp-mode) t nil t) ((fundamental-mode) t t nil))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_global_context_rejects_excluded_readonly_and_unindentable_buffers() {
    let elisp_form = r##"(progn
         (define-derived-mode
           aggressive-indent-parity-excluded-mode
           emacs-lisp-mode
           "AggExcluded")
         (let ((global-aggressive-indent-mode t)
               (aggressive-indent-excluded-modes
                '(aggressive-indent-parity-excluded-mode)))
           (mapcar
            (lambda (scenario)
              (with-temp-buffer
                (pcase scenario
                  ('normal
                   (emacs-lisp-mode))
                  ('excluded
                   (aggressive-indent-parity-excluded-mode))
                  ('text
                   (text-mode))
                  ('fundamental
                   (fundamental-mode))
                  ('indent-relative
                   (emacs-lisp-mode)
                   (setq indent-line-function
                         #'indent-relative))
                  ('readonly
                   (emacs-lisp-mode)
                   (setq buffer-read-only t)))
                (aggressive-indent-mode 1)
                (list
                 scenario
                 major-mode
                 aggressive-indent-mode
                 (memq
                  #'aggressive-indent--keep-track-of-changes
                  after-change-functions))))
            '(normal excluded text fundamental
              indent-relative readonly))))"##;
    let expect = expect![
        "OK ((normal emacs-lisp-mode t (aggressive-indent--keep-track-of-changes t)) (excluded aggressive-indent-parity-excluded-mode nil nil) (text text-mode nil nil) (fundamental fundamental-mode nil nil) (indent-relative emacs-lisp-mode nil nil) (readonly emacs-lisp-mode nil nil))"
    ];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_global_mode_enables_only_eligible_real_buffers_and_alias_disables() {
    let elisp_form = r##"(let ((code
                (generate-new-buffer
                 " *aggressive-code*"))
               (excluded
                (generate-new-buffer
                 " *aggressive-text*"))
               (plain
                (generate-new-buffer
                 " *aggressive-plain*")))
         (unwind-protect
             (progn
               (with-current-buffer code
                 (emacs-lisp-mode))
               (with-current-buffer excluded
                 (text-mode))
               (with-current-buffer plain
                 (fundamental-mode))
               (global-aggressive-indent-mode 1)
               (let ((enabled
                      (list
                       global-aggressive-indent-mode
                       (with-current-buffer code
                         aggressive-indent-mode)
                       (with-current-buffer excluded
                         aggressive-indent-mode)
                       (with-current-buffer plain
                         aggressive-indent-mode))))
                 (aggressive-indent-global-mode -1)
                 (list
                  enabled
                  global-aggressive-indent-mode
                  (with-current-buffer code
                    aggressive-indent-mode)
                  (with-current-buffer excluded
                    aggressive-indent-mode)
                  (eq
                   (indirect-function
                    'aggressive-indent-global-mode)
                   (indirect-function
                    'global-aggressive-indent-mode)))))
           (global-aggressive-indent-mode -1)
           (dolist (buffer
                    (list code excluded plain))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect!["OK ((t t nil nil) nil nil nil t)"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_backspace_filter_selects_join_only_for_indentable_leading_space() {
    let elisp_form = r##"(let* ((binding
                  (cdr
                   (assq
                    'backspace
                    aggressive-indent-mode-map)))
                 (filter
                  (plist-get
                   (nthcdr 3 binding)
                   :filter)))
         (mapcar
          (lambda (scenario)
            (with-temp-buffer
              (emacs-lisp-mode)
              (insert
               (pcase scenario
                 ('leading "   value")
                 ('content "value")
                 ('protected "   value")
                 ('custom "   value")))
              (goto-char
               (if (eq scenario 'content)
                   3
                 3))
              (let ((last-command
                     (and
                      (eq scenario 'protected)
                      'undo))
                    (aggressive-indent-dont-indent-if
                     (and
                      (eq scenario 'custom)
                      '(t))))
                (list
                 scenario
                 (funcall filter)
                 (point)
                 (buffer-string)))))
          '(leading content protected custom)))"##;
    let expect = expect![[
        r#"OK ((leading delete-indentation 3 "   value") (content nil 3 "value") (protected nil 3 "   value") (custom nil 3 "   value"))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_disable_cancels_real_timer_and_removes_stale_post_command_hook() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (aggressive-indent-mode 1)
         (add-hook
          'post-command-hook
          #'aggressive-indent--softly-indent-defun
          nil t)
         (setq aggressive-indent--idle-timer
               (timer-create))
         (let (cancelled)
           (cl-letf (((symbol-function 'cancel-timer)
                      (lambda (timer)
                        (push (timerp timer) cancelled))))
             (aggressive-indent-mode -1)
             (list
              aggressive-indent-mode
              aggressive-indent--idle-timer
              (nreverse cancelled)
              (memq
               #'aggressive-indent--softly-indent-defun
               post-command-hook)
              (memq
               #'aggressive-indent--keep-track-of-changes
               after-change-functions)))))"##;
    let expect = expect!["OK (nil nil (t) nil nil)"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_local_electric_uses_real_local_mode_without_changing_other_buffer() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *aggressive-electric-first*"))
               (second
                (generate-new-buffer
                 " *aggressive-electric-second*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (emacs-lisp-mode)
                 (aggressive-indent--local-electric nil))
               (with-current-buffer second
                 (emacs-lisp-mode)
                 (aggressive-indent--local-electric t))
               (list
                (with-current-buffer first
                  (list
                   electric-indent-mode
                   (local-variable-p
                    'electric-indent-mode)))
                (with-current-buffer second
                  (list
                   electric-indent-mode
                   (local-variable-p
                    'electric-indent-mode)))))
           (when (buffer-live-p first)
             (kill-buffer first))
           (when (buffer-live-p second)
             (kill-buffer second))))"##;
    let expect = expect!["OK ((nil t) (t nil))"];
    assert_aggressive_indent_parity(elisp_form, expect);
}

#[test]
fn aggressive_indent_bug_report_composes_version_message_and_exact_browser_target() {
    let elisp_form = r##"(let (messages urls)
         (cl-letf (((symbol-function 'message)
                    (lambda (&rest arguments)
                      (push arguments messages)
                      "shown"))
                   ((symbol-function 'browse-url)
                    (lambda (&rest arguments)
                      (push arguments urls)
                      'opened)))
           (let ((emacs-version
                  "30.1-parity-runtime"))
             (list
              (aggressive-indent-bug-report)
              (nreverse messages)
              (nreverse urls)))))"##;
    let expect = expect![[
        r#"OK (opened (("Your `aggressive-indent-version' is: %s, and your emacs version is: %s.\nPlease include this in your report!" nil "30.1-parity-runtime")) (("https://github.com/Malabarba/aggressive-indent-mode/issues/new")))"#
    ]];
    assert_aggressive_indent_parity(elisp_form, expect);
}
