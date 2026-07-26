use expect_test::expect;

use super::assert_abl_mode_parity;

#[test]
fn abl_shell_busy_handles_missing_buffer_idle_and_busy_child_counts() {
    let elisp_form = r##"(let ((buffers '(nil buffer buffer))
                    (outputs '("0\n" " 2\n"))
                    (abl-mode-shell-name "default-shell")
                    (abl-mode-shell-child-cmd "children %d")
                    events)
               (cl-letf
                   (((symbol-function 'get-buffer)
                     (lambda (name)
                       (push (list 'buffer name) events)
                       (pop buffers)))
                    ((symbol-function 'get-buffer-process)
                     (lambda (buffer)
                       (push (list 'process buffer) events)
                       'process))
                    ((symbol-function 'process-id)
                     (lambda (process)
                       (push (list 'pid process) events)
                       42))
                    ((symbol-function 'shell-command-to-string)
                     (lambda (command)
                       (push (list 'shell command) events)
                       (pop outputs))))
                 (list
                  (abl-shell-busy)
                  (abl-shell-busy "named")
                  (abl-shell-busy)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil nil t ((buffer "default-shell") (buffer "named") (process buffer) (pid process) (shell "children 42") (buffer "default-shell") (process buffer) (pid process) (shell "children 42")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_ve_name_or_create_covers_disabled_existing_create_and_replacement_paths() {
    let elisp_form = r##"(let ((abl-mode-shell-name "shell")
                    (abl-mode-ve-base-dir "/venvs")
                    answers
                    prompts)
               (clrhash abl-mode-replacement-vems)
               (cl-letf
                   (((symbol-function 'file-exists-p)
                     (lambda (path)
                       (member
                        path
                        '("/venvs/existing"
                          "/venvs/replacement"))))
                    ((symbol-function 'read-from-minibuffer)
                     (lambda (prompt)
                       (push prompt prompts)
                       (pop answers))))
                 (list
                  (let ((abl-mode-check-and-activate-ve nil))
                    (abl-ve-name-or-create "ignored"))
                  (let ((abl-mode-check-and-activate-ve t))
                    (abl-ve-name-or-create "existing"))
                  abl-ve-name
                  (gethash "shell" abl-mode-replacement-vems)
                  (progn
                    (setq answers '("y"))
                    (let ((abl-mode-check-and-activate-ve t))
                      (abl-ve-name-or-create "new")))
                  (progn
                    (setq answers '("replacement"))
                    (let ((abl-mode-check-and-activate-ve t))
                      (abl-ve-name-or-create "missing")))
                  abl-ve-name
                  (gethash "shell" abl-mode-replacement-vems)
                  (nreverse prompts))))"##;
    let expect = expect![[
        r#"OK ((nil) ("existing") "existing" "existing" ("new" . t) ("replacement") "replacement" "replacement" ("No virtualenv new; y to create it, or name of existing to use instead: " "No virtualenv missing; y to create it, or name of existing to use instead: "))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_exec_command_builds_create_activate_and_plain_command_chains() {
    let elisp_form = r##"(let ((abl-package-base "/project/")
                    (abl-mode-shell-name "ABL-SHELL")
                    (abl-mode-ve-create-command "create %s")
                    (abl-mode-ve-activate-command "activate %s")
                    (abl-mode-install-command "install")
                    (ve-answers
                     '(("new" . t)
                       ("existing")
                       (nil)))
                    events)
               (clrhash abl-mode-last-shell-points)
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function 'abl-ve-name-or-create)
                       (lambda (&rest _)
                         (pop ve-answers)))
                      ((symbol-function 'get-buffer)
                       (lambda (name)
                         (push (list 'get-buffer name) events)
                         nil))
                      ((symbol-function 'shell)
                       (lambda (name)
                         (push (list 'shell name) events)
                         'shell-result))
                      ((symbol-function 'sleep-for)
                       (lambda (seconds)
                         (push (list 'sleep seconds) events)))
                      ((symbol-function 'selected-window)
                       (lambda ()
                         'code-window))
                      ((symbol-function 'select-window)
                       (lambda (window)
                         (push (list 'select window) events)
                         'selected))
                      ((symbol-function 'comint-send-input)
                       (lambda ()
                         (push
                          (list 'send (buffer-string))
                          events)
                         'sent)))
                   (list
                    (abl-mode-exec-command "pytest one")
                    (prog1
                        (buffer-string)
                      (erase-buffer))
                    (abl-mode-exec-command "pytest two")
                    (prog1
                        (buffer-string)
                      (erase-buffer))
                    (abl-mode-exec-command "pytest three")
                    (buffer-string)
                    (gethash
                     "ABL-SHELL"
                     abl-mode-last-shell-points)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (selected "cd /project/ && create new && activate new && install && pytest one" selected "cd /project/ && activate existing && pytest two" selected "cd /project/ && pytest three" 1 ((get-buffer "ABL-SHELL") (shell "ABL-SHELL") (sleep 2) (send "cd /project/ && create new && activate new && install && pytest one") (select code-window) (get-buffer "ABL-SHELL") (shell "ABL-SHELL") (sleep 2) (send "cd /project/ && activate existing && pytest two") (select code-window) (get-buffer "ABL-SHELL") (shell "ABL-SHELL") (sleep 2) (send "cd /project/ && pytest three") (select code-window)))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_exec_command_reuses_visible_window_before_hidden_buffer() {
    let elisp_form = r##"(let* ((abl-package-base "/project")
                     (abl-mode-shell-name "abl-visible-shell")
                     (window (selected-window))
                     (original-buffer (window-buffer window))
                     (shell-buffer
                      (generate-new-buffer abl-mode-shell-name))
                     events)
                (unwind-protect
                    (progn
                      (set-window-buffer window shell-buffer)
                      (with-current-buffer shell-buffer
                        (cl-letf
                            (((symbol-function 'abl-ve-name-or-create)
                              (lambda (&rest _)
                                '("ve")))
                             ((symbol-function 'switch-to-buffer)
                              (lambda (buffer)
                                (push (list 'switch buffer) events)))
                             ((symbol-function 'shell)
                              (lambda (name)
                                (push (list 'shell name) events)))
                             ((symbol-function 'sleep-for)
                              (lambda (seconds)
                                (push (list 'sleep seconds) events)))
                             ((symbol-function 'comint-send-input)
                              (lambda ()
                                (push
                                 (list 'send (buffer-string))
                                 events))))
                          (let ((result
                                 (abl-mode-exec-command "command")))
                            (list
                             (eq result window)
                             (buffer-string)
                             (nreverse events))))))
                  (set-window-buffer window original-buffer)
                  (kill-buffer shell-buffer)))"##;
    let expect = expect![[
        r#"OK (t "cd /project && workon ve && command" ((send "cd /project && workon ve && command")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_run_test_busy_and_idle_paths_message_execute_and_record_exactly() {
    let elisp_form = r##"(let ((abl-mode-test-command "pytest %s")
                    (abl-mode-shell-name "shell")
                    (busy-answers '(t nil))
                    events)
               (clrhash abl-mode-last-tests-run)
               (cl-letf
                   (((symbol-function 'abl-shell-busy)
                     (lambda (&rest _)
                       (pop busy-answers)))
                    ((symbol-function 'message)
                     (lambda (text &rest arguments)
                       (let ((rendered
                              (apply #'format text arguments)))
                         (push (list 'message rendered) events)
                         rendered)))
                    ((symbol-function 'abl-mode-exec-command)
                     (lambda (command)
                       (push (list 'exec command) events)
                       'executed)))
                 (list
                  (abl-mode-run-test "tests.py::One")
                  (gethash "shell" abl-mode-last-tests-run)
                  (abl-mode-run-test "tests.py::Two" "ignored-branch")
                  (gethash "shell" abl-mode-last-tests-run)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("The shell is busy; please end the process before running a test" nil "tests.py::Two" "tests.py::Two" ((message "The shell is busy; please end the process before running a test") (message "Running test(s) tests.py::Two on shell") (exec "pytest tests.py::Two")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_run_at_point_rerun_and_format_commands_forward_exact_values() {
    let elisp_form = r##"(let ((abl-mode-shell-name "shell")
                    (abl-mode-format-command "fmt %s")
                    events)
               (clrhash abl-mode-last-tests-run)
               (with-temp-buffer
                 (setq buffer-file-name "/project/file name.py")
                 (cl-letf
                     (((symbol-function 'abl-mode-get-test-entity)
                       (lambda ()
                         (push '(entity) events)
                         "tests.py::Case"))
                      ((symbol-function 'abl-mode-run-test)
                       (lambda (test)
                         (push (list 'run test) events)
                         'run-result))
                      ((symbol-function 'abl-mode-exec-command)
                       (lambda (command)
                         (push (list 'exec command) events)
                         'exec-result))
                      ((symbol-function 'message)
                       (lambda (text &rest arguments)
                         (let ((rendered
                                (apply #'format text arguments)))
                           (push (list 'message rendered) events)
                           rendered))))
                   (list
                    (abl-mode-run-test-at-point)
                    (abl-mode-rerun-last-test)
                    (progn
                      (puthash
                       "shell"
                       "last-test"
                       abl-mode-last-tests-run)
                      (abl-mode-rerun-last-test))
                    (abl-mode-format-file nil)
                    (abl-mode-format-file '(4))
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (run-result "You haven't run any tests yet." run-result exec-result exec-result ((entity) (run "tests.py::Case") (message "You haven't run any tests yet.") (run "last-test") (exec "fmt /project/file name.py") (exec "fmt .")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}
