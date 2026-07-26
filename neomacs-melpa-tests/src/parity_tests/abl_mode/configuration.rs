use expect_test::expect;

use super::assert_abl_mode_parity;

#[test]
fn abl_find_base_dir_checks_markers_in_priority_order_and_short_circuits() {
    let elisp_form = r##"(let ((answers
                    '(nil "/requirements-root/" "/pyproject-root/"))
                   events)
               (cl-letf
                   (((symbol-function 'locate-dominating-file)
                     (lambda (path marker)
                       (push (list path marker) events)
                       (pop answers))))
                 (list
                  (abl-find-base-dir "/one/code.py")
                  (nreverse events)
                  (progn
                    (setq events nil
                          answers
                          '("/setup-root/"
                            "/unused/"
                            "/unused/"))
                    (list
                     (abl-find-base-dir "/two/code.py")
                     (nreverse events)))
                  (progn
                    (setq events nil
                          answers '(nil nil "/pyproject-root/"))
                    (list
                     (abl-find-base-dir "/three/code.py")
                     (nreverse events))))))"##;
    let expect = expect![[
        r#"OK ("/requirements-root/" (("/one/code.py" "setup.py") ("/one/code.py" "requirements.txt")) ("/setup-root/" (("/two/code.py" "setup.py"))) ("/pyproject-root/" (("/three/code.py" "setup.py") ("/three/code.py" "requirements.txt") ("/three/code.py" "pyproject.toml"))))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_capitalized_matches_upstream_ascii_unicode_digit_and_empty_boundaries() {
    let elisp_form = r##"(mapcar
               (lambda (value)
                 (condition-case error
                     (list 'value value (abl-capitalized? value))
                   (error
                    (list 'signal error))))
               '("Hello" "hello" "Äpfel" "äpfel" "1thing" ""))"##;
    let expect = expect![[
        r#"OK ((value "Hello" t) (value "hello" nil) (value "Äpfel" t) (value "äpfel" nil) (value "1thing" t) (signal (args-out-of-range "" 0 1)))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_set_config_reads_evaluates_and_assigns_in_the_current_buffer() {
    let elisp_form = r##"(let ((abl-ve-name "before")
                    (abl-mode-check-and-activate-ve t))
               (with-temp-buffer
                 (list
                  (abl-mode-set-config
                   "abl-ve-name"
                   "\"branch env\"")
                  abl-ve-name
                  (local-variable-p 'abl-ve-name)
                  (abl-mode-set-config
                   "abl-mode-check-and-activate-ve"
                   "nil")
                  abl-mode-check-and-activate-ve
                  (local-variable-p
                   'abl-mode-check-and-activate-ve))))"##;
    let expect = expect![[r#"OK ("branch env" "branch env" t nil nil t)"#]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn parse_abl_options_preserves_line_and_value_spacing_contract() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'insert-file-contents)
                     (lambda (path)
                       (push (list 'read path) events)
                       (insert
                        "abl-ve-name \"VENAME\"\n"
                        "abl-mode-check-and-activate-ve nil\n"
                        "abl-mode-test-command \"pytest -x %s\"\n")
                       '(path 0)))
                    ((symbol-function 'abl-mode-set-config)
                     (lambda (name value)
                       (push
                        (list 'set name value)
                        events)
                       value)))
                 (list
                  (parse-abl-options "/project/.abl")
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil ((read "/project/.abl") (set "abl-ve-name" "\"VENAME\"") (set "abl-mode-check-and-activate-ve" "nil") (set "abl-mode-test-command" "\"pytest -x %s\"")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_local_options_checks_exact_path_and_parses_only_existing_file() {
    let elisp_form = r##"(let ((answers '(t nil))
                    events)
               (cl-letf
                   (((symbol-function 'file-exists-p)
                     (lambda (path)
                       (push (list 'exists path) events)
                       (pop answers)))
                    ((symbol-function 'parse-abl-options)
                     (lambda (path)
                       (push (list 'parse path) events)
                       'parsed)))
                 (list
                  (abl-mode-local-options "/project")
                  (abl-mode-local-options "/other/")
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (parsed nil ((exists "/project/.abl") (parse "/project/.abl") (exists "/other/.abl")))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_git_branch_builds_exact_command_trims_output_and_recognizes_fatal_text() {
    let elisp_form = r##"(let ((outputs
                    '("  feature/topic \n"
                      "fatal: not a git repository (or any parent)\n"
                      "\n"))
                   events)
               (cl-letf
                   (((symbol-function 'shell-command-to-string)
                     (lambda (command)
                       (push command events)
                       (pop outputs))))
                 (list
                  (abl-git-branch "/work/project/")
                  (abl-git-branch "/work/not repo")
                  (abl-git-branch "/work/detached")
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("feature/topic" nil "" ("cd /work/project/ && git branch --show-current" "cd /work/not repo && git branch --show-current" "cd /work/detached && git branch --show-current"))"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_project_virtualenv_and_shell_names_cover_branches_replacements_and_nil() {
    let elisp_form = r##"(let ((abl-mode-branch "feature/x")
                    (abl-mode-project-name "project")
                    (abl-mode-shell-name "shell-key")
                    (abl-mode-branch-shell-prefix "ABL:"))
               (clrhash abl-mode-replacement-vems)
               (cl-letf
                   (((symbol-function 'abl-find-base-dir)
                     (lambda (_path)
                       "/work/project/")))
                 (list
                  (abl-get-project-name "/work/project/file.py")
                  (abl-make-ve-name)
                  (abl-make-ve-name nil "override")
                  (abl-make-ve-name "release" "override")
                  (progn
                    (puthash
                     "shell-key"
                     "replacement"
                     abl-mode-replacement-vems)
                    (abl-make-ve-name "ignored" "ignored"))
                  (abl-mode-shell-name-for-branch
                   "project"
                   "feature/x"))))"##;
    let expect = expect![[
        r#"OK ("project" "project_feature-x" "override_feature-x" "override_release" "replacement" "ABL:project_feature/x")"#
    ]];

    assert_abl_mode_parity(elisp_form, expect);
}
