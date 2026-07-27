use expect_test::expect;

use super::assert_annoying_arrows_mode_parity;

#[test]
fn annoying_arrows_counter_resets_for_first_different_and_untracked_commands() {
    let elisp_form = r##"(let ((annoying-arrows--commands
               '(next-line previous-line))
               (annoying-arrows--current-count 7))
         (mapcar
          (lambda (state)
            (setq this-command (car state)
                  last-command (cadr state))
            (let ((before annoying-arrows--current-count)
                  (result (annoying-arrows--maybe-complain
                           (car state))))
              (list state before result
                    annoying-arrows--current-count)))
          '((next-line previous-line)
            (next-line next-line)
            (forward-word forward-word)
            (previous-line next-line))))"##;
    let expect = expect![
        "OK (((next-line previous-line) 7 0 0) ((next-line next-line) 0 nil 1) ((forward-word forward-word) 1 0 0) ((previous-line next-line) 0 0 0))"
    ];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_complains_only_after_strict_threshold_and_formats_suggestion() {
    let elisp_form = r##"(let ((annoying-arrows--commands '(next-line))
               (annoying-arrows--current-count 0)
               (annoying-arrows-too-far-count 2)
               (this-command 'next-line)
               (last-command 'next-line)
               beeps messages)
         (cl-letf (((symbol-function 'beep)
                    (lambda (&optional arg) (push arg beeps)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages)))
                   ((symbol-function 'random) (lambda (&rest _) 0))
                   ((symbol-function 'annoying-arrows--commands-with-shortcuts)
                    (lambda (_) '(forward-paragraph))))
           (let (states)
             (dotimes (_ 5)
               (push (annoying-arrows--maybe-complain 'next-line) states)
               (push annoying-arrows--current-count states))
             (list (nreverse states)
                   (nreverse beeps)
                   (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK ((nil 1 nil 2 #3=(#("Annoying! How about using forward-paragraph (M-}) instead?" 45 48 (face help-key-binding font-lock-face help-key-binding)) . #1=(#("Annoying! How about using forward-paragraph (M-}) instead?" 45 48 (face help-key-binding font-lock-face help-key-binding)) . #2=(#("Annoying! How about using forward-paragraph (M-}) instead?" 45 48 (face help-key-binding font-lock-face help-key-binding))))) 3 #1# 4 #2# 5) (1 1 1) #3#)"#
    ]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_threshold_zero_complains_on_first_repeated_command() {
    let elisp_form = r##"(let ((annoying-arrows--commands '(right-char))
               (annoying-arrows--current-count 0)
               (annoying-arrows-too-far-count 0)
               (this-command 'right-char)
               (last-command 'right-char)
               calls)
         (cl-letf (((symbol-function 'beep)
                    (lambda (&rest args) (push (cons 'beep args) calls)))
                   ((symbol-function 'message)
                    (lambda (&rest args) (push (cons 'message args) calls)))
                   ((symbol-function 'random) (lambda (&rest _) 0))
                   ((symbol-function 'annoying-arrows--commands-with-shortcuts)
                    (lambda (_) '(right-word))))
           (annoying-arrows--maybe-complain 'right-char)
           (list annoying-arrows--current-count
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (1 ((beep 1) (message "Annoying! How about using %S (%s) instead?" right-word #("C-<right>" 0 9 (font-lock-face help-key-binding face help-key-binding)))))"#
    ]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_maybe_complain_uses_cmd_argument_for_alternatives() {
    let elisp_form = r##"(let ((annoying-arrows--commands '(next-line))
               (annoying-arrows--current-count 10)
               (annoying-arrows-too-far-count 1)
               (this-command 'next-line)
               (last-command 'next-line)
               looked-up)
         (put 'custom-command 'annoying-arrows--alts
              '(custom-alternative))
         (cl-letf (((symbol-function 'beep) #'ignore)
                   ((symbol-function 'message)
                    (lambda (&rest args) args))
                   ((symbol-function 'random) (lambda (&rest _) 0))
                   ((symbol-function 'annoying-arrows--commands-with-shortcuts)
                    (lambda (commands)
                      (setq looked-up commands)
                      commands)))
           (list
            (annoying-arrows--maybe-complain 'custom-command)
            looked-up
            annoying-arrows--current-count)))"##;
    let expect = expect![[
        r#"OK (("Annoying! How about using %S (%s) instead?" custom-alternative #("M-x custom-alternative" 0 22 (font-lock-face help-key-binding face help-key-binding))) (custom-alternative) 11)"#
    ]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_real_previous_line_advice_counts_only_when_mode_enabled() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\ntwo\nthree\nfour\n")
         (goto-char (point-max))
         (let ((annoying-arrows-too-far-count 99)
               (annoying-arrows--current-count 0)
               (this-command 'previous-line)
               (last-command 'previous-line))
           (annoying-arrows-mode -1)
           (previous-line 1)
           (let ((disabled-count annoying-arrows--current-count))
             (annoying-arrows-mode 1)
             (previous-line 1)
             (previous-line 1)
             (list disabled-count
                   annoying-arrows--current-count
                   (line-number-at-pos)
                   annoying-arrows-mode))))"##;
    let expect = expect!["OK (0 2 2 t)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_local_mode_toggles_without_changing_buffer_or_keymap() {
    let elisp_form = r##"(with-temp-buffer
         (insert "content")
         (let ((before-map (current-local-map)))
           (annoying-arrows-mode 1)
           (let ((enabled
                  (list annoying-arrows-mode
                        (current-local-map)
                        (buffer-string))))
             (annoying-arrows-mode -1)
             (list enabled
                   annoying-arrows-mode
                   (eq before-map (current-local-map))
                   (buffer-string)))))"##;
    let expect = expect![[r#"OK ((t nil "content") nil t "content")"#]];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn global_annoying_arrows_mode_enables_eligible_buffers_and_disables_cleanly() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *annoying-a*"))
               (second (generate-new-buffer " *annoying-b*")))
         (unwind-protect
             (progn
               (with-current-buffer first (fundamental-mode))
               (with-current-buffer second (emacs-lisp-mode))
               (global-annoying-arrows-mode 1)
               (let ((enabled
                      (list global-annoying-arrows-mode
                            (buffer-local-value 'annoying-arrows-mode first)
                            (buffer-local-value 'annoying-arrows-mode second))))
                 (global-annoying-arrows-mode -1)
                 (list enabled
                       global-annoying-arrows-mode
                       (buffer-local-value 'annoying-arrows-mode first)
                       (buffer-local-value 'annoying-arrows-mode second))))
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect!["OK ((t t t) nil nil nil)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}

#[test]
fn annoying_arrows_empty_shortcut_candidates_expose_exact_failure_contract() {
    let elisp_form = r##"(let ((annoying-arrows--commands '(next-line))
               (annoying-arrows--current-count 3)
               (annoying-arrows-too-far-count 1)
               (this-command 'next-line)
               (last-command 'next-line))
         (cl-letf (((symbol-function 'beep) #'ignore)
                   ((symbol-function 'annoying-arrows--commands-with-shortcuts)
                    (lambda (_) nil)))
           (condition-case err
               (annoying-arrows--maybe-complain 'next-line)
             (error
              (list (car err) (cdr err)
                    annoying-arrows--current-count)))))"##;
    let expect = expect!["OK (args-out-of-range (0) 4)"];
    assert_annoying_arrows_mode_parity(elisp_form, expect);
}
