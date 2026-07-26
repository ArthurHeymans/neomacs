use expect_test::expect;

use super::{assert_ac_php_core_parity, assert_ac_php_core_signal_parity};

#[test]
fn ac_php_core_index_process_filter_accumulates_errors_and_updates_only_changed_progress() {
    let elisp_form = r##"(let ((ac-php-rebuild-tmp-error-msg
                    nil)
                   (ac-php-phptags-index-progress
                    0)
                   calls)
               (cl-letf
                   (((symbol-function
                      'force-mode-line-update)
                     (lambda (&optional all)
                       (push
                        (list
                         'update all
                         ac-php-phptags-index-progress)
                        calls)
                       'updated)))
                 (list
                  (ac-php-phptags-index-process-filter
                   'process
                   "10%\n10%\nPHPParser: first\nnoise\n75%\nPHPParser: second\n")
                  ac-php-phptags-index-progress
                  ac-php-rebuild-tmp-error-msg
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (nil 75 "\nPHPParser: first\nPHPParser: second" ((update nil 10) (update nil 75)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_rebuild_file_list_wires_process_arguments_mode_sentinel_and_filter() {
    let elisp_form = r##"(let ((ac-php-php-executable
                    "/usr/bin/php")
                   (ac-php-ctags-executable
                    "/package/phpctags")
                   (ac-php-tags-path
                    "/cache")
                   (ac-php-project-root-dir-use-truename
                    t)
                   (ac-php-rebuild-tmp-error-msg
                    'stale)
                   (ac-php-phptags-index-progress
                    99)
                   sentinel
                   filter
                   calls)
               (cl-letf
                   (((symbol-function
                      'start-process)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'start
                         arguments)
                        calls)
                       'fake-process))
                    ((symbol-function
                      'ac-php-mode)
                     (lambda (&optional argument)
                       (push
                        (list
                         'mode argument)
                        calls)
                       argument))
                    ((symbol-function
                      'force-mode-line-update)
                     (lambda (&optional all)
                       (push
                        (list
                         'update all)
                        calls)
                       'updated))
                    ((symbol-function
                      'set-process-sentinel)
                     (lambda
                         (process function)
                       (push
                        (list
                         'sentinel process)
                        calls)
                       (setq
                        sentinel
                        function)))
                    ((symbol-function
                      'set-process-filter)
                     (lambda
                         (process function)
                       (push
                        (list
                         'filter process function)
                        calls)
                       (setq
                        filter
                        function)))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (when
                           (and
                            (stringp
                             (car arguments))
                            (string-prefix-p
                             "ac-php:"
                             (car arguments)))
                         (push
                          (cons
                           'message
                           arguments)
                          calls))
                       'messaged)))
                 (let ((return
                        (ac-php--rebuild-file-list
                         "/project/" "/unused/" t)))
                   (funcall
                    sentinel
                    'fake-process
                    "finished\n")
                   (setq
                    ac-php-rebuild-tmp-error-msg
                    "parse failed")
                   (funcall
                    sentinel
                    'fake-process
                    "finished\n")
                   (funcall
                    sentinel
                    'fake-process
                    "exited abnormally with code 1\n")
                   (list
                    return
                    ac-php-rebuild-tmp-error-msg
                    ac-php-phptags-index-progress
                    filter
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (ac-php-phptags-index-process-filter "parse failed" 0 ac-php-phptags-index-process-filter ((message "ac-php: Rebuild file list...") (start "ac-phptags" "*AC-PHPTAGS*" "/usr/bin/php" "/package/phpctags" "--config-file=/project/.ac-php-conf.json" "--tags_dir=/cache" "--rebuild=yes" "--realpath_flag=yes") (mode t) (update nil) (sentinel fake-process) (filter fake-process ac-php-phptags-index-process-filter) (mode 0) (message "ac-php: The project has been successfully re-indexed") (mode 0) (message "ac-php: An error occurred during to re-index: %s" "parse failed") (mode 0) (message "ac-php: Something went wrong\nac-php: The re-indexing process exited abnormally\nac-php: Please re-check for incorrect syntax and possible PHP errors and try again later")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_remake_tags_guard_allows_idle_or_debug_and_rejects_overlap() {
    let elisp_form = r##"(let ((ac-php-debug-flag
                    nil)
                   (ac-php-gen-tags-flag
                    t)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php--remake-tags-ex)
                     (lambda
                         (root force)
                       (push
                        (list
                         root force)
                        calls)
                       'started)))
                 (let ((overlap
                        (ac-php--remake-tags
                         "/overlap/" nil)))
                   (setq
                    ac-php-debug-flag
                    t)
                   (let ((debug
                          (ac-php--remake-tags
                           "/debug/" t)))
                     (setq
                      ac-php-debug-flag
                      nil
                      ac-php-gen-tags-flag
                      nil)
                     (let ((idle
                            (ac-php--remake-tags
                             "/idle/" nil)))
                       (list
                        overlap
                        debug
                        idle
                        ac-php-gen-tags-flag
                        (nreverse calls)))))))"##;
    let expect = expect![[r#"OK (nil started started t (("/debug/" t) ("/idle/" nil)))"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_remake_tags_ex_validates_tools_project_and_vendor_force_behavior() {
    let elisp_form = r##"(let ((ac-php-ctags-executable
                    "/package/phpctags")
                   (ac-php-php-executable
                    "/usr/bin/php")
                   (ac-php-gen-tags-flag
                    t)
                   (available
                    '("/package/phpctags"
                      "/usr/bin/php"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'f-exists?)
                     (lambda (path)
                       (member path
                               available)))
                    ((symbol-function
                      'ac-php--get-tags-save-dir)
                     (lambda (root)
                       (push
                        (list
                         'save root)
                        calls)
                       "/cache/"))
                    ((symbol-function
                      'ac-php--rebuild-file-list)
                     (lambda
                         (root save force)
                       (push
                        (list
                         'rebuild root save force)
                        calls)
                       'rebuilt))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (when
                           (and
                            (stringp
                             (car arguments))
                            (string-prefix-p
                             "ac-php:"
                             (car arguments)))
                         (push
                          (cons
                           'message
                           arguments)
                          calls))
                       'messaged)))
                 (let ((buffer-file-name
                        "/project/src/file.php"))
                   (let ((valid
                          (ac-php--remake-tags-ex
                           "/project/" nil)))
                     (setq
                      available
                      '("/usr/bin/php")
                      ac-php-gen-tags-flag
                      t)
                     (let ((missing-ctags
                            (ac-php--remake-tags-ex
                             "/project/" nil)))
                       (setq
                        available
                        '("/package/phpctags")
                        ac-php-gen-tags-flag
                        t)
                       (let ((missing-php
                              (ac-php--remake-tags-ex
                               "/project/" nil)))
                         (setq
                          available
                          '("/package/phpctags"
                            "/usr/bin/php")
                          ac-php-gen-tags-flag
                          t
                          buffer-file-name
                          "/project/vendor/pkg/file.php")
                         (let ((vendor
                                (ac-php--remake-tags-ex
                                 "/project/" nil)))
                           (list
                            valid
                            missing-ctags
                            missing-php
                            vendor
                            ac-php-gen-tags-flag
                            (nreverse calls)))))))))"##;
    let expect = expect![[
        r#"OK (rebuilt nil rebuilt rebuilt t ((message "ac-php: Starting to re-index the project located at %s%s" "/project/" "") (save "/project/") (rebuild "/project/" "/cache/" nil) (message "ac-php: Starting to re-index the project located at %s%s" "/project/" "") (message "ac-php: Unable to locate phpctags executable at %s\nac-php: Restarting GNU Emacs might help" "/package/phpctags") (message "ac-php: Starting to re-index the project located at %s%s" "/project/" "") (message "ac-php: Unable to locate PHP executable at %s\nac-php: You need to install PHP CLI and restart GNU Emacs" "/usr/bin/php") (save "/project/") (rebuild "/project/" "/cache/" nil) (message "ac-php: Starting to re-index the project located at %s%s" "/project/" "with a forced rebuilding of all tags") (save "/project/") (rebuild "/project/" "/cache/" t)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_remake_tags_ex_exposes_nil_project_root_path_reduction_signal() {
    let elisp_form = r##"(let ((buffer-file-name
                    "/standalone.php"))
               (ac-php--remake-tags-ex
                nil nil))"##;
    let expect = expect!["ERR (wrong-type-argument stringp nil)"];

    assert_ac_php_core_signal_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_public_remake_commands_forward_root_and_exact_force_flags() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-php--get-project-root-dir)
                     (lambda ()
                       (push
                        '(root)
                        calls)
                       "/project/"))
                    ((symbol-function
                      'ac-php--remake-tags)
                     (lambda
                         (root force)
                       (push
                        (list
                         'remake root force)
                        calls)
                       'remade)))
                 (list
                  (call-interactively
                   'ac-php-remake-tags)
                  (call-interactively
                   'ac-php-remake-tags-all)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (remade remade (#1=(root) (remake "/project/" nil) #1# (remake "/project/" t)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_cscope_dispatch_sets_cache_directory_or_reports_configuration() {
    let elisp_form = r##"(let ((ac-php-use-cscope-flag
                    nil)
                   (config-enabled
                    nil)
                   (cscope-initial-directory
                    "stale")
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php--get-project-root-dir)
                     (lambda ()
                       "/project/"))
                    ((symbol-function
                      'ac-php--get-use-cscope-from-config-file)
                     (lambda (_root)
                       config-enabled))
                    ((symbol-function
                      'ac-php--get-tags-save-dir)
                     (lambda (root)
                       (push
                        (list
                         'save root)
                        calls)
                       "/cache/"))
                    ((symbol-function
                      'cscope-find-egrep-pattern)
                     (lambda (symbol)
                       (push
                        (list
                         'search
                         symbol
                         cscope-initial-directory)
                        calls)
                       'searched))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'message
                         arguments)
                        calls)
                       'messaged)))
                 (let ((disabled
                        (ac-php-cscope-find-egrep-pattern
                         "first")))
                   (setq
                    config-enabled
                    t)
                   (let ((configured
                          (ac-php-cscope-find-egrep-pattern
                           "second")))
                     (setq
                      config-enabled
                      nil
                      ac-php-use-cscope-flag
                      t)
                     (let ((global
                            (ac-php-cscope-find-egrep-pattern
                             "third")))
                       (list
                        disabled
                        configured
                        global
                        cscope-initial-directory
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (messaged searched searched "/cache/" ((message "need config: %s -> use-cscope:true" ".ac-php-conf.json") (save "/project/") (search "second" "/cache/") (save "/project/") (search "third" "/cache/")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_remake_cscope_gates_generation_writes_reversed_file_list_and_runs_exact_command() {
    let elisp_form = r##"(let ((ac-php-cscope
                    nil)
                   (ac-php-use-cscope-flag
                    nil)
                   (config-enabled
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php--get-use-cscope-from-config-file)
                     (lambda (root)
                       (push
                        (list
                         'config root)
                        calls)
                       config-enabled))
                    ((symbol-function
                      'ac-php--get-tags-save-dir)
                     (lambda (root)
                       (push
                        (list
                         'save root)
                        calls)
                       "/cache/"))
                    ((symbol-function
                      'f-write)
                     (lambda
                         (text encoding path)
                       (push
                        (list
                         'write text encoding path)
                        calls)
                       path))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push
                        (list
                         'shell command)
                        calls)
                       "indexed"))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'message
                         arguments)
                        calls)
                       'messaged)))
                 (let ((disabled
                        (ac-php--remake-cscope
                         "/project/"
                         '(("/project/a.php")
                           ("/project/src/b.php")))))
                   (setq
                    ac-php-cscope
                    t)
                   (let ((config-off
                          (ac-php--remake-cscope
                           "/project/"
                           '(("/project/a.php")))))
                     (setq
                      config-enabled
                      t)
                     (let ((config-on
                            (ac-php--remake-cscope
                             "/project/"
                             '(("/project/a.php")
                               ("/project/src/b.php")))))
                       (setq
                        config-enabled
                        nil
                        ac-php-use-cscope-flag
                        t)
                       (let ((flag-on
                              (ac-php--remake-cscope
                               "/other/"
                               '(("/other/z.php")))))
                         (list
                          disabled
                          config-off
                          config-on
                          flag-on
                          (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK (nil nil "indexed" "indexed" ((config "/project/") (config "/project/") (message "rebuild cscope data file ") (save "/project/") (write "/project/src/b.php\n/project/a.php" utf-8 "/cache/cscope.files") (shell " cd /cache/ &&  cscope -bkq -i cscope.files ") (config "/other/") (message "rebuild cscope data file ") (save "/other/") (write "/other/z.php" utf-8 "/cache/cscope.files") (shell " cd /cache/ &&  cscope -bkq -i cscope.files ")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_minor_mode_and_eldoc_setup_manage_buffer_local_state() {
    let elisp_form = r##"(let ((ac-php-gen-tags-flag
                    nil)
                   calls)
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function
                        'eldoc-mode)
                       (lambda (&optional argument)
                         (push
                          (list
                           argument
                           eldoc-documentation-function
                           (local-variable-p
                            'eldoc-documentation-function))
                          calls)
                         'eldoc-enabled)))
                   (let ((enable
                          (ac-php-mode 1)))
                     (let ((after-enable
                            (list
                             enable
                             ac-php-mode
                             ac-php-gen-tags-flag
                             (local-variable-p
                              'ac-php-mode))))
                       (let ((disable
                              (ac-php-mode -1)))
                         (let ((after-disable
                                (list
                                 disable
                                 ac-php-mode
                                 ac-php-gen-tags-flag)))
                           (let ((eldoc
                                  (call-interactively
                                   'ac-php-core-eldoc-setup)))
                             (list
                              after-enable
                              after-disable
                              eldoc
                              (eq
                               eldoc-documentation-function
                               #'ac-php-eldoc-documentation-function)
                              (nreverse calls))))))))))"##;
    let expect = expect![
        "OK ((t t t t) (nil nil nil) eldoc-enabled t ((1 ac-php-eldoc-documentation-function t)))"
    ];

    assert_ac_php_core_parity(elisp_form, expect);
}
