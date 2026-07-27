use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_parameter_quoting_handles_options_regex_metacharacters_unicode_and_empty_text() {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (list
            text
            (agtags--fix-param text)
            (agtags--quote-text text)))
         '("symbol"
           "-danger"
           "--regexp"
           "a+b[c]"
           "path with space"
           "λ-value"
           ""))"##;
    let expect = expect![[
        r#"OK (("symbol" "symbol" "symbol") ("-danger" "\\-danger" "\\-danger") ("--regexp" "\\--regexp" "\\--regexp") ("a+b[c]" "a+b[c]" "a\\+b\\[c]") ("path with space" "path with space" "path with space") ("λ-value" "λ-value" "λ-value") ("" "" ""))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_project_root_and_active_detection_use_real_deterministic_filesystem() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-project"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (nested
                 (expand-file-name
                  "src/deep"
                  root))
                (tag-file
                 (expand-file-name
                  "GTAGS"
                  root)))
         (unwind-protect
             (progn
               (make-directory nested t)
               (write-region
                "tag database"
                nil tag-file nil 'silent)
               (let ((default-directory
                      (file-name-as-directory
                       nested)))
                 (cl-letf (((symbol-function
                             'project-current)
                            (lambda (&rest _)
                              'project-object))
                           ((symbol-function
                             'project-root)
                            (lambda (project)
                              (list
                               project root)
                              (file-name-as-directory
                               root))))
                   (list
                    (agtags--parse-root)
                    (agtags--is-active root)
                    (agtags--is-active nested)
                    (agtags--is-active "")
                    (progn
                      (delete-file tag-file)
                      (make-directory
                       tag-file)
                      (agtags--is-active
                       root))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[r#"OK ("[ORACLE-SANDBOX]/agtags-project/" t nil nil nil)"#]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_run_global_to_list_executes_real_fake_global_with_directory_and_arguments() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-real-global"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (program
                 (expand-file-name
                  "global"
                  root))
                (log
                 (expand-file-name
                  "global.log"
                  root)))
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "database"
                nil
                (expand-file-name
                 "GTAGS" root)
                nil 'silent)
               (write-region
                "#!/bin/sh\nprintf '%s\\n' \"$PWD\" > \"$AGTAGS_LOG\"\nprintf '<%s>\\n' \"$@\" >> \"$AGTAGS_LOG\"\nprintf 'alpha\\nalphabet\\nbeta\\n'\n"
                nil program nil 'silent)
               (set-file-modes program #o755)
               (let ((exec-path
                      (cons root exec-path))
                     (process-environment
                      (copy-sequence
                       process-environment)))
                 (setenv
                  "PATH"
                  (concat
                   root path-separator
                   (getenv "PATH")))
                 (setenv "AGTAGS_LOG" log)
                 (list
                  (agtags--run-global-to-list
                   '("-c" "al")
                   root)
                  (with-temp-buffer
                    (insert-file-contents log)
                    (buffer-string)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (("alpha" "alphabet" "beta") "[ORACLE-SANDBOX]/agtags-real-global\n<-c>\n<al>\n")"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_run_global_to_list_returns_nil_for_process_failure_or_inactive_database() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-global-failure"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (tag-file
                 (expand-file-name
                  "GTAGS" root))
                calls)
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "database" nil
                tag-file nil 'silent)
               (cl-letf (((symbol-function
                           'process-lines)
                          (lambda (&rest arguments)
                            (push arguments calls)
                            (error
                             "global process failed"))))
                 (let ((failed
                        (agtags--run-global-to-list
                         '("-c" "symbol")
                         root)))
                   (delete-file tag-file)
                   (list
                    failed
                    (agtags--run-global-to-list
                     '("-c" "symbol")
                     root)
                    (nreverse calls)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[r#"OK (nil nil (("global" "-c" "symbol")))"#]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_cached_global_reuses_exact_root_and_argument_key_then_refreshes() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-cache"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (program
                 (expand-file-name
                  "global" root))
                (log
                 (expand-file-name
                  "calls.log" root)))
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "database" nil
                (expand-file-name
                 "GTAGS" root)
                nil 'silent)
               (write-region
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$AGTAGS_LOG\"\nprintf '%s-result\\n' \"$*\"\n"
                nil program nil 'silent)
               (set-file-modes program #o755)
               (let ((exec-path
                      (cons root exec-path))
                     (process-environment
                      (copy-sequence
                       process-environment))
                     (agtags--global-to-list-cache
                      nil))
                 (setenv
                  "PATH"
                  (concat
                   root path-separator
                   (getenv "PATH")))
                 (setenv "AGTAGS_LOG" log)
                 (cl-letf (((symbol-function
                             'agtags--parse-root)
                            (lambda ()
                              (file-name-as-directory
                               root))))
                   (let ((first
                          (agtags--run-cached-global-to-list
                           '("-c" "alpha")))
                         (second
                          (agtags--run-cached-global-to-list
                           '("-c" "alpha")))
                         (third
                          (agtags--run-cached-global-to-list
                           '("-c" "beta"))))
                     (list
                      first second third
                      agtags--global-to-list-cache
                      (with-temp-buffer
                        (insert-file-contents
                         log)
                        (buffer-string)))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (#1=("-c alpha-result") #1# #2=("-c beta-result") ("[ORACLE-SANDBOX]/agtags-cache/$-c$beta" . #2#) "-c alpha\n-c beta\n")"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_run_global_to_mode_builds_grep_and_path_commands_with_real_root_options() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-command-root"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                calls)
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "database" nil
                (expand-file-name
                 "GTAGS" root)
                nil 'silent)
               (cl-letf (((symbol-function
                           'agtags--parse-root)
                          (lambda ()
                            (file-name-as-directory
                             root)))
                         ((symbol-function
                           'compilation-start)
                          (lambda
                              (command mode)
                            (push
                             (list
                              command mode
                              default-directory
                              display-buffer-overriding-action)
                             calls)
                            'started)))
                 (let ((agtags-global-ignore-case
                        t)
                       (agtags-global-treat-text
                        t))
                   (list
                    (agtags--run-global-to-mode
                     "-needle with space"
                     '("-r"))
                    (agtags--run-global-to-mode
                     "*.el"
                     '("-P")
                     "path")
                    (nreverse calls)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (started started (("global --result=grep -i -o -r -needle\\ with\\ space" agtags-grep-mode "[ORACLE-SANDBOX]/agtags-command-root/" #1=((display-buffer-reuse-window display-buffer-same-window) (inhibit-same-window))) ("global --result=path -i -o -P \\*.el" agtags-path-mode "[ORACLE-SANDBOX]/agtags-command-root/" #1#)))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_global_completion_composes_flags_and_exercises_all_completion_protocol_codes() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agtags--run-cached-global-to-list)
                    (lambda (arguments)
                      (push arguments calls)
                      '("alpha"
                        "alphabet"
                        "beta"))))
           (let ((agtags-global-ignore-case t)
                 (agtags-global-treat-text t))
             (list
              (agtags--run-global-completing
               'tags "al" nil nil)
              (agtags--run-global-completing
               'files "a" nil t)
              (condition-case error-data
                  (agtags--run-global-completing
                   'rtags "alpha" nil 'lambda)
                (error
                 (list
                  (car error-data)
                  (cadr error-data))))
              (condition-case error-data
                  (agtags--run-global-completing
                   'rtags "missing" nil 'lambda)
                (error
                 (list
                  (car error-data)
                  (cadr error-data))))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("alpha" ("alpha" "alphabet") (wrong-type-argument obarrayp) (wrong-type-argument obarrayp) (("-c" "-i" "-o" "al") ("-c" "-P" "-i" "-o" "a") ("-c" "-r" "-i" "-o" "alpha") ("-c" "-r" "-i" "-o" "missing")))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_read_helpers_preserve_region_text_properties_symbol_defaults_prompts_and_history() {
    let elisp_form = r##"(let (minibuffer-calls)
         (cl-letf (((symbol-function
                     'read-from-minibuffer)
                    (lambda (&rest arguments)
                      (push arguments
                            minibuffer-calls)
                      (pcase
                          (car arguments)
                        ("Plain: " "typed")
                        ("DWIM (default symbol-name): "
                         "")
                        (_ "fallback")))))
           (let ((agtags--history-list
                  '("older")))
             (list
              (with-temp-buffer
                (insert
                 (propertize
                  "chosen text"
                  'face 'bold))
                (goto-char (point-max))
                (set-mark (point-min))
                (setq mark-active t
                      transient-mark-mode t)
                (let ((value
                       (agtags--read-dwim)))
                  (list
                   value
                   (text-properties-at
                    0 value))))
              (with-temp-buffer
                (insert "symbol-name")
                (goto-char 4)
                (list
                 (agtags--read-dwim)
                 (agtags--read-input-dwim
                  "DWIM")))
              (with-temp-buffer
                (insert "   ")
                (goto-char 2)
                (agtags--read-dwim))
              (agtags--read-input "Plain")
              (nreverse
               minibuffer-calls)))))"##;
    let expect = expect![[
        r#"OK (("chosen text" nil) ("symbol-name" "symbol-name") nil "typed" (("DWIM (default symbol-name): " nil nil nil #1=("older") "symbol-name") ("Plain: " nil nil nil #1#)))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_completing_read_helpers_supply_dynamic_table_flags_defaults_and_history() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agtags--run-cached-global-to-list)
                    (lambda (arguments)
                      (list
                       (concat
                        (car (last arguments))
                        "pha")
                       "alphabet"
                       "beta")))
                   ((symbol-function 'completing-read)
                    (lambda
                        (prompt collection
                         predicate require-match
                         initial history
                         &optional default)
                      (push
                       (list
                        prompt predicate
                        require-match initial
                        history default
                        (funcall
                         collection
                         "al" nil t))
                       calls)
                      (if default
                          ""
                        "selected"))))
           (let ((agtags--history-list
                  '("prior")))
             (list
              (agtags--read-completing
               'files "Open file")
              (with-temp-buffer
                (insert "symbol")
                (goto-char 3)
                (agtags--read-completing-dwim
                 'rtags "Find rtag"))
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("selected" "symbol" (("Open file: " nil nil nil #1=("prior") nil ("alpha" "alphabet")) ("Find rtag (default symbol): " nil nil nil #1# "symbol" ("alpha" "alphabet"))))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}
