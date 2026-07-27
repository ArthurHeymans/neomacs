use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_mode_before_save_runs_real_single_file_update_and_clears_session_caches() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-auto-update"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (source
                 (expand-file-name
                  "src/main.c" root))
                (program
                 (expand-file-name
                  "global" root))
                (log
                 (expand-file-name
                  "update.log" root)))
         (unwind-protect
             (progn
               (make-directory
                (file-name-directory
                 source)
                t)
               (write-region
                "database" nil
                (expand-file-name
                 "GTAGS" root)
                nil 'silent)
               (write-region
                "int main(void) { return 0; }\n"
                nil source nil 'silent)
               (write-region
                "#!/bin/sh\nprintf '%s\\n' \"$PWD\" > \"$AGTAGS_LOG\"\nprintf '<%s>\\n' \"$@\" >> \"$AGTAGS_LOG\"\nexit 0\n"
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
                 (with-temp-buffer
                   (setq buffer-file-name
                         source
                         agtags--history-list
                         '("old query")
                         agtags--global-to-list-cache
                         '("old-key"
                           "old-value"))
                   (agtags-mode 1)
                   (cl-letf (((symbol-function
                               'agtags--parse-root)
                              (lambda ()
                                (file-name-as-directory
                                 root))))
                     (run-hooks
                      'before-save-hook))
                   (list
                    agtags--history-list
                    agtags--global-to-list-cache
                    (with-temp-buffer
                      (insert-file-contents
                       log)
                      (buffer-string))
                    (memq
                     'agtags--auto-update
                     before-save-hook)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (nil nil "[ORACLE-SANDBOX]/agtags-auto-update\n<-u>\n<--single-update=>\n" (agtags--auto-update t))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_auto_update_skips_disabled_missing_outside_and_inactive_buffers_without_processes() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-auto-skip"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (inside
                 (expand-file-name
                  "inside.c" root))
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
                           'call-process)
                          (lambda (&rest arguments)
                            (push arguments calls)
                            0)))
                 (list
                  (with-temp-buffer
                    (setq agtags-mode nil
                          buffer-file-name
                          inside)
                    (agtags--auto-update))
                  (with-temp-buffer
                    (setq agtags-mode t
                          buffer-file-name
                          nil)
                    (agtags--auto-update))
                  (with-temp-buffer
                    (setq agtags-mode t
                          buffer-file-name
                          "/outside/file.c")
                    (agtags--auto-update))
                  (progn
                    (delete-file
                     (expand-file-name
                      "GTAGS" root))
                    (with-temp-buffer
                      (setq agtags-mode t
                            buffer-file-name
                            inside)
                      (agtags--auto-update)))
                  (nreverse calls))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_update_tags_deletes_stale_databases_runs_real_fake_gtags_and_reports_success() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-create-tags"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (program
                 (expand-file-name
                  "gtags" root))
                (log
                 (expand-file-name
                  "gtags.log" root))
                messages)
         (unwind-protect
             (progn
               (make-directory root t)
               (dolist
                   (file
                    agtags-created-tag-files)
                 (write-region
                  (concat "stale-" file)
                  nil
                  (expand-file-name
                   file root)
                  nil 'silent))
               (write-region
                "#!/bin/sh\nprintf '%s\\n' \"$PWD\" > \"$AGTAGS_LOG\"\nprintf '<%s>\\n' \"$@\" >> \"$AGTAGS_LOG\"\nprintf 'fresh' > GTAGS\nexit 0\n"
                nil program nil 'silent)
               (set-file-modes program #o755)
               (let ((exec-path
                      (cons root exec-path))
                     (process-environment
                      (copy-sequence
                       process-environment))
                     (agtags--history-list
                      '("old"))
                     (agtags--global-to-list-cache
                      '("old" "cache")))
                 (setenv
                  "PATH"
                  (concat
                   root path-separator
                   (getenv "PATH")))
                 (setenv "AGTAGS_LOG" log)
                 (cl-letf (((symbol-function
                             'read-directory-name)
                            (lambda (&rest _)
                              (file-name-as-directory
                               root)))
                           ((symbol-function
                             'message)
                            (lambda (&rest arguments)
                              (when
                                  (string-prefix-p
                                   "Tags create"
                                   (car arguments))
                                (push arguments
                                      messages)))))
                   (list
                    (agtags-update-tags)
                    agtags--history-list
                    agtags--global-to-list-cache
                    (mapcar
                     (lambda (file)
                       (list
                        file
                        (file-exists-p
                         (expand-file-name
                          file root))
                        (and
                         (file-regular-p
                          (expand-file-name
                           file root))
                         (with-temp-buffer
                           (insert-file-contents
                            (expand-file-name
                             file root))
                           (buffer-string)))))
                     agtags-created-tag-files)
                    (with-temp-buffer
                      (insert-file-contents
                       log)
                      (buffer-string))
                    (nreverse messages)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (#1=(("Tags create successed: %s" "[ORACLE-SANDBOX]/agtags-create-tags/")) nil nil (("GPATH" nil nil) ("GTAGS" t "fresh") ("GRTAGS" nil nil)) "[ORACLE-SANDBOX]/agtags-create-tags\n<-i>\n" #1#)"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_update_tags_failure_still_removes_stale_files_and_reports_exact_root() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-create-failure"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                messages)
         (unwind-protect
             (progn
               (make-directory root t)
               (dolist
                   (file
                    agtags-created-tag-files)
                 (write-region
                  "stale" nil
                  (expand-file-name
                   file root)
                  nil 'silent))
               (cl-letf (((symbol-function
                           'read-directory-name)
                          (lambda (&rest _)
                            (file-name-as-directory
                             root)))
                         ((symbol-function
                           'executable-find)
                          (lambda (_program)
                            nil))
                         ((symbol-function
                           'message)
                          (lambda (&rest arguments)
                            (when
                                (string-prefix-p
                                 "Tags create"
                                 (car arguments))
                              (push arguments
                                    messages)))))
                 (list
                  (agtags-update-tags)
                  (mapcar
                   (lambda (file)
                     (file-exists-p
                      (expand-file-name
                       file root)))
                   agtags-created-tag-files)
                  (nreverse messages))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (#1=(("Tags create failed: %s" "[ORACLE-SANDBOX]/agtags-create-failure/")) (nil nil nil) #1#)"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_open_file_resolves_completion_against_project_root_and_visits_real_content() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agtags-open-file"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (relative
                 "src/path with space.c")
                (target
                 (expand-file-name
                  relative root))
                visited)
         (unwind-protect
             (progn
               (make-directory
                (file-name-directory
                 target)
                t)
               (write-region
                "line one\nline two\n"
                nil target nil 'silent)
               (cl-letf (((symbol-function
                           'agtags--read-completing)
                          (lambda
                              (flag prompt)
                            (list flag prompt)
                            relative))
                         ((symbol-function
                           'agtags--parse-root)
                          (lambda ()
                            (file-name-as-directory
                             root))))
                 (setq visited
                       (agtags-open-file))
                 (list
                  (buffer-file-name
                   visited)
                  (with-current-buffer
                      visited
                    (list
                     (buffer-string)
                     (point)
                     (buffer-modified-p))))))
           (when
               (buffer-live-p visited)
             (with-current-buffer visited
               (set-buffer-modified-p
                nil))
             (kill-buffer visited))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/agtags-open-file/src/path with space.c" (#("line one\nline two\n" 0 18 (fontified nil)) 1 nil))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_interactive_search_commands_route_defaults_quotes_patterns_flags_and_empty_input() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agtags--read-input)
                    (lambda (prompt)
                      (pcase prompt
                        ("Find files"
                         "*.el")
                        (_ ""))))
                   ((symbol-function
                     'agtags--read-completing-dwim)
                    (lambda (flag prompt)
                      (pcase flag
                        ('tags "-target")
                        ('rtags "caller")
                        (_
                         (concat prompt)))))
                   ((symbol-function
                     'agtags--read-input-dwim)
                    (lambda (prompt)
                      (pcase prompt
                        ("Search pattern"
                         "-raw.*")
                        ("Search string"
                         "literal+[x]")
                        (_ ""))))
                   ((symbol-function
                     'agtags--run-global-to-mode)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      'started)))
           (let ((results
                  (list
                   (agtags-find-file)
                   (agtags-find-tag)
                   (agtags-find-rtag)
                   (agtags-find-with-pattern)
                   (agtags-find-with-string))))
             (cl-letf (((symbol-function
                         'agtags--read-input)
                        (lambda (_prompt)
                          "")))
               (list
                results
                (agtags-find-file)
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK ((started started started started started) nil (("*.el" ("-P") "path") ("\\-target" nil) ("caller" ("-r")) ("\\-raw.*" ("-g")) ("literal\\+\\[x]" ("-g"))))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_switch_prefers_grep_then_path_then_real_other_buffer() {
    let elisp_form = r##"(let ((origin
                (generate-new-buffer
                 " *agtags-origin*"))
               (fallback
                (generate-new-buffer
                 " *agtags-fallback*"))
               grep-buffer
               path-buffer
               events)
         (unwind-protect
             (cl-letf (((symbol-function
                         'switch-to-buffer)
                        (lambda (buffer
                                 &rest _)
                          (push
                           (buffer-name buffer)
                           events)
                          buffer))
                       ((symbol-function
                         'other-buffer)
                        (lambda (&rest _)
                          fallback)))
               (with-current-buffer origin
                 (let ((none
                        (agtags-switch-dwim)))
                   (setq path-buffer
                         (get-buffer-create
                          "*agtags-path*"))
                   (let ((path
                          (agtags-switch-dwim)))
                     (setq grep-buffer
                           (get-buffer-create
                            "*agtags-grep*"))
                     (list
                      (buffer-name none)
                      (buffer-name path)
                      (buffer-name
                       (agtags-switch-dwim))
                      (nreverse events))))))
           (dolist
               (buffer
                (list origin fallback
                      grep-buffer path-buffer))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (" *agtags-fallback*" "*agtags-path*" "*agtags-grep*" (" *agtags-fallback*" "*agtags-path*" "*agtags-grep*"))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_bind_keys_installs_complete_custom_prefix_command_map() {
    let elisp_form = r##"(let ((agtags-key-prefix
                "C-c C-a"))
         (unwind-protect
             (progn
               (agtags-bind-keys)
               (mapcar
                (lambda (suffix)
                  (list
                   suffix
                   (key-binding
                    (kbd
                     (concat
                      agtags-key-prefix
                      " " suffix)))))
                '("q" "b" "f" "F"
                  "t" "r" "p" "g")))
           (dolist
               (suffix
                '("q" "b" "f" "F"
                  "t" "r" "p" "g"))
             (global-unset-key
              (kbd
               (concat
                agtags-key-prefix
                " " suffix))))))"##;
    let expect = expect![[
        r#"OK (("q" agtags-switch-dwim) ("b" agtags-update-tags) ("f" agtags-open-file) ("F" agtags-find-file) ("t" agtags-find-tag) ("r" agtags-find-rtag) ("p" agtags-find-with-string) ("g" agtags-find-with-pattern))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}
