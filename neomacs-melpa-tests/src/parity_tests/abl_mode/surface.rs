use expect_test::expect;

use super::assert_abl_mode_parity;

#[test]
fn abl_mode_public_surface_and_command_classification_match_the_pin() {
    let elisp_form = r##"(list
               (featurep 'abl-mode)
               (mapcar
                #'fboundp
                '(abl-mode
                  abl-mode-hook
                  abl-find-base-dir
                  abl-capitalized?
                  abl-mode-set-config
                  parse-abl-options
                  abl-mode-local-options
                  abl-git-branch
                  abl-get-project-name
                  abl-make-ve-name
                  abl-mode-shell-name-for-branch
                  abl-shell-busy
                  abl-mode-exec-command
                  abl-ve-name-or-create
                  abl-class-and-indent
                  abl-function-and-indent
                  abl-mode-get-test-entity
                  abl-mode-run-test
                  abl-mode-run-test-at-point
                  abl-mode-rerun-last-test
                  abl-mode-format-file))
               (mapcar
                #'commandp
                '(abl-mode
                  abl-mode-hook
                  abl-mode-run-test
                  abl-mode-run-test-at-point
                  abl-mode-rerun-last-test
                  abl-mode-format-file)))"##;
    let expect = expect!["OK (t (t t t t t t t t t t t t t t t t t t t t t) (t nil nil t t t))"];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_defaults_buffer_local_contracts_and_hash_tables_match_the_pin() {
    let elisp_form = r##"(list
               (mapcar
                #'symbol-value
                '(abl-mode
                  abl-mode-ve-activate-command
                  abl-mode-ve-create-command
                  abl-mode-test-command
                  abl-mode-branch-shell-prefix
                  abl-mode-check-and-activate-ve
                  abl-mode-ve-base-dir
                  abl-mode-install-command
                  abl-mode-test-file-regexp
                  abl-mode-format-command
                  abl-file-class-separator
                  abl-class-method-separator
                  abl-use-test-file-path
                  abl-ve-name
                  abl-mode-shell-name
                  abl-package-base
                  abl-mode-branch
                  abl-mode-project-name
                  abl-mode-shell-child-cmd
                  abl-mode-identifier-re))
               (mapcar
                #'local-variable-if-set-p
                '(abl-mode
                  abl-mode-ve-activate-command
                  abl-mode-ve-create-command
                  abl-mode-test-command
                  abl-mode-branch-shell-prefix
                  abl-mode-check-and-activate-ve
                  abl-mode-ve-base-dir
                  abl-mode-install-command
                  abl-mode-test-file-regexp
                  abl-mode-format-command
                  abl-file-class-separator
                  abl-class-method-separator
                  abl-use-test-file-path
                  abl-mode-use-file-module
                  abl-ve-name
                  abl-mode-shell-name
                  abl-package-base
                  abl-mode-branch
                  abl-mode-project-name))
               (list
                (hash-table-test abl-mode-replacement-vems)
                (hash-table-count abl-mode-replacement-vems)
                (hash-table-test abl-mode-last-shell-points)
                (hash-table-count abl-mode-last-shell-points)
                (hash-table-test abl-mode-last-tests-run)
                (hash-table-count abl-mode-last-tests-run)))"##;
    let expect = expect![[
        r#"OK ((nil "workon %s" "mkvirtualenv %s" "python -m unittest %s" "ABL-SHELL:" t "~/.virtualenvs" "python setup.py develop" ".*_tests.py" "black %1$s && isort --profile black %1$s" "::" "::" t "" "ABL-SHELL" "" "master" "web" "ps --ppid %d  h | wc -l" "[^a-zA-Z0-9_.]") (t t t t t t t t t t t t nil t t t t t t) (equal 0 equal 0 equal 0))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_custom_metadata_keymap_and_minor_mode_registrations_match_the_pin() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (get variable 'custom-group)
                   (get variable 'custom-type)))
                '(abl-mode-ve-activate-command
                  abl-mode-ve-create-command
                  abl-mode-test-command
                  abl-mode-branch-shell-prefix
                  abl-mode-check-and-activate-ve
                  abl-mode-ve-base-dir
                  abl-mode-install-command
                  abl-mode-test-file-regexp
                  abl-mode-format-command
                  abl-file-class-separator
                  abl-class-method-separator
                  abl-use-test-file-path
                  abl-ve-name
                  abl-mode-shell-name))
               (mapcar
                (lambda (key)
                  (lookup-key abl-mode-keymap (kbd key)))
                '("C-c t" "C-c u" "C-c f" "C-c s" "C-c o" "C-c m"))
               (assq 'abl-mode minor-mode-alist)
               (let ((entry
                      (assq 'abl-mode minor-mode-map-alist)))
                 (list
                  (car entry)
                  (eq (cdr entry) abl-mode-keymap))))"##;
    let expect = expect![[
        r#"OK (((abl-mode-ve-activate-command nil nil) (abl-mode-ve-create-command nil nil) (abl-mode-test-command nil nil) (abl-mode-branch-shell-prefix nil nil) (abl-mode-check-and-activate-ve nil nil) (abl-mode-ve-base-dir nil nil) (abl-mode-install-command nil nil) (abl-mode-test-file-regexp nil nil) (abl-mode-format-command nil nil) (abl-file-class-separator nil nil) (abl-class-method-separator nil nil) (abl-use-test-file-path nil nil) (abl-ve-name nil nil) (abl-mode-shell-name nil nil)) (abl-mode-run-test-at-point abl-mode-rerun-last-test abl-mode-format-file nil nil nil) (abl-mode " abl-mode") (abl-mode t))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_disables_itself_with_exact_message_when_no_project_base_exists() {
    let elisp_form = r##"(let ((abl-mode nil)
                    events)
               (with-temp-buffer
                 (setq buffer-file-name "/workspace/code.py")
                 (cl-letf
                     (((symbol-function 'abl-find-base-dir)
                       (lambda (path)
                         (push (list 'find-base path) events)
                         nil))
                      ((symbol-function 'message)
                       (lambda (text &rest arguments)
                         (let ((rendered
                                (apply #'format text arguments)))
                           (push (list 'message rendered) events)
                           rendered))))
                   (list
                    (abl-mode 1)
                    abl-mode
                    (local-variable-p 'abl-mode)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil nil t ((find-base "/workspace/code.py") (message "Could not find project base. Please make sure there is a setup.py or requirements.txt in a higher directory.")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_successfully_initializes_project_state_in_exact_order() {
    let elisp_form = r##"(let ((abl-mode nil)
                    events)
               (with-temp-buffer
                 (setq buffer-file-name "/workspace/project/tests.py")
                 (cl-letf
                     (((symbol-function 'abl-find-base-dir)
                       (lambda (path)
                         (push (list 'find-base path) events)
                         "/workspace/project/"))
                      ((symbol-function 'abl-git-branch)
                       (lambda (base)
                         (push (list 'branch base) events)
                         "feature/x"))
                      ((symbol-function 'abl-get-project-name)
                       (lambda (base)
                         (push (list 'project base) events)
                         "project"))
                      ((symbol-function
                        'abl-mode-shell-name-for-branch)
                       (lambda (project branch)
                         (push
                          (list 'shell-name project branch)
                          events)
                         "SHELL"))
                      ((symbol-function 'abl-make-ve-name)
                       (lambda (&rest arguments)
                         (push (cons 've-name arguments) events)
                         "VENV"))
                      ((symbol-function 'abl-mode-local-options)
                       (lambda (base)
                         (push (list 'options base) events)
                         'options-result)))
                   (list
                    (abl-mode 1)
                    abl-mode
                    abl-package-base
                    abl-mode-branch
                    abl-mode-project-name
                    abl-mode-shell-name
                    abl-ve-name
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (options-result t "/workspace/project/" "feature/x" "project" "SHELL" "VENV" ((find-base "/workspace/project/tests.py") (branch "/workspace/project/") (project "/workspace/project/") (shell-name "project" "feature/x") (ve-name) (options "/workspace/project/")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_prefix_and_hook_semantics_cover_toggle_enable_and_disable() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'abl-find-base-dir)
                     (lambda (&rest _)
                       "/project/"))
                    ((symbol-function 'abl-git-branch)
                     (lambda (&rest _) nil))
                    ((symbol-function 'abl-get-project-name)
                     (lambda (&rest _) "project"))
                    ((symbol-function
                      'abl-mode-shell-name-for-branch)
                     (lambda (&rest _) "shell"))
                    ((symbol-function 'abl-make-ve-name)
                     (lambda (&rest _) "venv"))
                    ((symbol-function 'abl-mode-local-options)
                     (lambda (&rest _)
                       (push 'options events))))
                 (with-temp-buffer
                   (setq buffer-file-name "/project/x.py")
                   (list
                    (progn
                      (abl-mode)
                      abl-mode)
                    (progn
                      (abl-mode)
                      abl-mode)
                    (progn
                      (abl-mode -1)
                      abl-mode)
                    (progn
                      (abl-mode '(4))
                      abl-mode)
                    (progn
                      (abl-mode-hook)
                      abl-mode)
                    (nreverse events)))))"##;
    let expect = expect!["OK (t nil nil t nil (options options))"];

    assert_abl_mode_parity(elisp_form, expect);
}
