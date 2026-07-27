use expect_test::expect;

use super::assert_ag_parity;

#[test]
fn ag_search_builds_real_default_literal_grouped_command_and_result_name() {
    let elisp_form = r##"(let* ((directory
                 (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                (ag-executable "ag")
                (ag-arguments
                 '("--smart-case" "--stats"))
                (ag-group-matches t)
                (ag-context-lines nil)
                (ag-ignore-list nil)
                (current-prefix-arg nil)
                captured)
         (cl-letf (((symbol-function 'compilation-start)
                    (lambda (command mode name-function)
                      (setq captured
                            (list
                             command
                             mode
                             (funcall name-function "Ag")
                             default-directory))
                      'started)))
           (list
            (ag/search "needle with space" directory)
            captured
            ag-arguments)))"##;
    let expect = expect![[
        r#"OK (started ("ag --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --smart-case --stats -- needle\\ with\\ space ." ag-mode "*ag search text:needle with space dir:[ORACLE-SANDBOX]*" "[ORACLE-SANDBOX]/") ("--smart-case" "--stats"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_search_composes_regexp_file_type_file_regex_files_ignore_and_context_options() {
    let elisp_form = r##"(let* ((directory
                 (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                (ag-executable "/opt/Silver Searcher/ag")
                (ag-arguments '("--hidden" "--"))
                (ag-group-matches nil)
                (ag-context-lines 7)
                (ag-ignore-list
                 '("target" "*.min.js" "space dir"))
                (current-prefix-arg 3)
                captured)
         (cl-letf (((symbol-function 'compilation-start)
                    (lambda (command mode name-function)
                      (setq captured
                            (list
                             command
                             mode
                             (funcall name-function "Ag")
                             default-directory))
                      'compiled)))
           (list
            (ag/search
             "n(e+)"
             directory
             :regexp t
             :file-regex "\\.el$"
             :file-type "lisp"
             :files '("src" "test suite"))
            captured
            ag-arguments
            ag-ignore-list)))"##;
    let expect = expect![[
        r#"OK (compiled ("/opt/Silver\\ Searcher/ag --ignore target --ignore \\*.min.js --ignore space\\ dir --context\\=3 --lisp --file-search-regex \\\\.el\\$ --nogroup --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --hidden -- n\\(e\\+\\) src test\\ suite" ag-mode "*ag search regexp:n(e+) dir:[ORACLE-SANDBOX]*" "[ORACLE-SANDBOX]/") ("--hidden" "--") ("target" "*.min.js" "space dir"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_search_negative_prefix_opens_edit_prompt_at_expected_command_position() {
    let elisp_form = r##"(let* ((directory
                 (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                (ag-executable "ag")
                (ag-arguments '("--stats"))
                (ag-group-matches t)
                (ag-context-lines 99)
                (ag-ignore-list nil)
                (current-prefix-arg -4)
                prompts
                compiled)
         (cl-letf (((symbol-function 'read-from-minibuffer)
                    (lambda (&rest arguments)
                      (push arguments prompts)
                      "edited ag command"))
                   ((symbol-function 'compilation-start)
                    (lambda (command mode name-function)
                      (setq compiled
                            (list
                             command
                             mode
                             (funcall name-function "Ag")))
                      'compiled)))
           (list
            (ag/search "needle" directory)
            (nreverse prompts)
            compiled)))"##;
    let expect = expect![[
        r#"OK (compiled (("ag command: " ("ag --context\\=4 --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --stats  -- needle ." . 114))) ("edited ag command" ag-mode "*ag search text:needle dir:[ORACLE-SANDBOX]*"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_search_windows_and_cygwin_paths_add_vimgrep_without_losing_other_options() {
    let elisp_form = r##"(let ((directory
                (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               commands)
         (cl-letf (((symbol-function 'compilation-start)
                    (lambda (command _mode _name-function)
                      (push command commands)
                      'compiled))
                   ((symbol-function
                     'w32-shell-dos-semantics)
                    (lambda () nil)))
           (dolist (platform '(windows-nt cygwin gnu/linux))
             (let ((system-type platform)
                   (ag-executable "ag")
                   (ag-arguments nil)
                   (ag-group-matches t)
                   (ag-context-lines nil)
                   (ag-ignore-list nil)
                   (current-prefix-arg nil))
               (ag/search "x" directory)))
           (nreverse commands)))"##;
    let expect = expect![[
        r#"OK ("ag --vimgrep --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 -- x ." "ag --vimgrep --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 -- x ." "ag --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 -- x .")"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_search_rejects_missing_directory_before_starting_compilation() {
    let elisp_form = r##"(let* ((directory
                 (expand-file-name
                  "definitely-missing/search-root"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (ag-executable "ag")
                (ag-arguments nil)
                (ag-group-matches t)
                (ag-context-lines nil)
                (ag-ignore-list nil)
                (current-prefix-arg nil)
                compilation-called)
         (cl-letf (((symbol-function 'compilation-start)
                    (lambda (&rest _arguments)
                      (setq compilation-called t)
                      'unexpected)))
           (list
            (condition-case error-data
                (ag/search "needle" directory)
              (error
               (list
                (car error-data)
                (cadr error-data))))
            compilation-called
            (file-exists-p directory))))"##;
    let expect = expect![[
        r#"OK ((error "No such directory [ORACLE-SANDBOX]/definitely-missing/search-root/") nil nil)"#
    ]];
    assert_ag_parity(elisp_form, expect);
}
