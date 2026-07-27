use expect_test::expect;

use super::assert_ag_parity;

#[test]
fn ag_dired_align_size_column_pads_realistic_short_sizes_and_leaves_other_lines() {
    let elisp_form = r##"(mapcar
         (lambda (line)
           (with-temp-buffer
             (insert line)
             (goto-char (point-min))
             (ag/dired-align-size-column)
             (list
              line
              (buffer-string)
              (point))))
         '("  -rw-r--r-- 1 user group 7 file.el\n"
           "  -rw-r--r-- 1 user group 123456789012 file.el\n"
           "-rw-r--r-- 1 user group 7 unindented.el\n"
           "  short\n"
           "  drwxr-xr-x 2 user group 42 directory\n"))"##;
    let expect = expect![[
        r#"OK (("  -rw-r--r-- 1 user group 7 file.el\n" "  -rw-r--r-- 1 user group           7 file.el\n" 37) ("  -rw-r--r-- 1 user group 123456789012 file.el\n" "  -rw-r--r-- 1 user group 123456789012 file.el\n" 40) ("-rw-r--r-- 1 user group 7 unindented.el\n" "-rw-r--r-- 1 user group 7 unindented.el\n" 1) ("  short\n" "  short\n" 3) ("  drwxr-xr-x 2 user group 42 directory\n" "  drwxr-xr-x 2 user group          42 directory\n" 36))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_dired_filter_formats_incremental_listing_removes_root_and_advances_marker() {
    let elisp_form = r##"(let* ((buffer
                 (generate-new-buffer
                  " *ag-dired-filter-parity*"))
                (marker (make-marker))
                (root
                 (file-name-as-directory
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                property-calls
                deleted)
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (setq default-directory root)
                 (insert "  header\n")
                 (set-marker marker (point-max) buffer))
               (cl-letf (((symbol-function 'process-buffer)
                          (lambda (_process) buffer))
                         ((symbol-function 'process-mark)
                          (lambda (_process) marker))
                         ((symbol-function
                           'dired-insert-set-properties)
                          (lambda (start end)
                            (push
                             (list start end)
                             property-calls)))
                         ((symbol-function 'delete-process)
                          (lambda (process)
                            (setq deleted process))))
                 (ag/dired-filter
                  'fake-process
                  (format
                   "-rw-r--r-- 1 user group 7 %salpha.el\n-rw-r--r-- 1 user group 123 %ssub/beta.el\npartial"
                   root root))
                 (with-current-buffer buffer
                   (list
                    (buffer-string)
                    (marker-position marker)
                    (nreverse property-calls)
                    deleted))))
           (set-marker marker nil)
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ("  header\n  -rw-r--r-- 1 user group           7 alpha.el\n  -rw-r--r-- 1 user group         123 sub/beta.el\n  partial" 107 (((:marker nil nil) 107)) nil)"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_dired_filter_deletes_process_when_its_destination_buffer_is_dead() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *ag-dead-filter-parity*"))
               deleted)
         (kill-buffer buffer)
         (cl-letf (((symbol-function 'process-buffer)
                    (lambda (_process) buffer))
                   ((symbol-function 'delete-process)
                    (lambda (process)
                      (setq deleted process)
                      'deleted)))
           (list
            (ag/dired-filter 'dead-process "ignored")
            deleted
            (buffer-live-p buffer))))"##;
    let expect = expect!["OK (deleted dead-process nil)"];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_dired_sentinel_updates_real_buffer_status_hook_message_and_process_lifecycle() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *ag-dired-sentinel-parity*"))
               events)
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (insert "  listing\n")
                 (add-hook
                  'dired-after-readin-hook
                  (lambda ()
                    (push
                     (list
                      'hook
                      (buffer-name)
                      (point-max))
                     events))
                  nil t))
               (cl-letf (((symbol-function 'process-buffer)
                          (lambda (_process) buffer))
                         ((symbol-function 'process-status)
                          (lambda (_process) 'exit))
                         ((symbol-function 'delete-process)
                          (lambda (process)
                            (push
                             (list 'delete process)
                             events)))
                         ((symbol-function
                           'force-mode-line-update)
                          (lambda (&optional all)
                            (push
                             (list 'force all)
                             events)))
                         ((symbol-function 'current-time-string)
                          (lambda ()
                            "Mon Jan  2 03:04:05 2006"))
                         ((symbol-function 'message)
                          (lambda (format-string
                                   &rest arguments)
                            (push
                             (cons
                              'message
                              (cons
                               format-string
                               arguments))
                             events))))
                 (let ((result
                        (ag/dired-sentinel
                         'fake-process
                         "finished\n")))
                   (with-current-buffer buffer
                     (list
                      result
                      (buffer-string)
                      mode-line-process
                      (nreverse events))))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (#1=((message "%s finished." (:buffer nil))) "  listing\n\n  ag finished at Mon Jan  2 03:04:05\n" ":exit" ((delete fake-process) (force nil) (hook " *ag-dired-sentinel-parity*" 49) . #1#))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_kill_process_only_deletes_running_find_dired_filter_and_swallows_delete_errors() {
    let elisp_form = r##"(progn
         (defvar ag-parity-status nil)
         (defvar ag-parity-filter nil)
         (defvar ag-parity-delete-errors nil)
         (let (events)
         (cl-letf (((symbol-function 'get-buffer-process)
                    (lambda (_buffer) 'fake-process))
                   ((symbol-function 'process-status)
                    (lambda (_process) ag-parity-status))
                   ((symbol-function 'process-filter)
                    (lambda (_process) ag-parity-filter))
                   ((symbol-function 'delete-process)
                    (lambda (process)
                      (push
                       (list 'delete process)
                       events)
                      (if ag-parity-delete-errors
                          (error "cannot delete")
                        'deleted))))
           (mapcar
            (lambda (case)
              (pcase-let ((`(,status ,filter-name ,errors) case))
                (let ((ag-parity-status status)
                      (ag-parity-filter
                       (if
                        (eq filter-name 'find-dired)
                        #'find-dired-filter
                        #'ignore))
                      (ag-parity-delete-errors errors))
                  (list
                   case
                   (ag/kill-process)
                   (copy-sequence events)))))
            '((run find-dired nil)
              (stop find-dired nil)
              (run ignore nil)
              (run find-dired t))))))"##;
    let expect = expect![
        "OK (((run find-dired nil) deleted (#1=(delete fake-process))) ((stop find-dired nil) nil (#1#)) ((run ignore nil) nil (#1#)) ((run find-dired t) nil ((delete fake-process) #1#)))"
    ];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_dired_regexp_builds_real_command_mode_map_process_and_revert_contract() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "ag dired root"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (marker (make-marker))
                events
                result-buffer)
         (make-directory root t)
         (cl-letf (((symbol-function 'dired-mode)
                    (lambda (directory)
                      (push
                       (list 'dired-mode directory)
                       events)
                      (setq major-mode 'dired-mode
                            mode-name "Dired"
                            dired-directory directory)
                      (use-local-map
                       (make-sparse-keymap))))
                   ((symbol-function
                     'dired-simple-subdir-alist)
                    (lambda ()
                      (push 'subdir-alist events)
                      (setq dired-subdir-alist
                            (list
                             (cons
                              default-directory
                              (point-min-marker))))))
                   ((symbol-function 'switch-to-buffer)
                    (lambda (buffer)
                      (push
                       (list 'switch
                             (buffer-name buffer))
                       events)
                      buffer))
                   ((symbol-function 'shell-command)
                    (lambda (command output-buffer)
                      (push
                       (list
                        'shell
                        command
                        (buffer-name output-buffer))
                       events)
                      (with-current-buffer output-buffer
                        (insert
                         "-rw-r--r-- 1 user group 7 sample.el\n"))
                      0))
                   ((symbol-function 'get-buffer-process)
                    (lambda (buffer)
                      (setq result-buffer buffer)
                      buffer))
                   ((symbol-function 'set-process-filter)
                    (lambda (process function)
                      (push
                       (list
                        'filter
                        process
                        function)
                       events)))
                   ((symbol-function 'set-process-sentinel)
                    (lambda (process function)
                      (push
                       (list
                        'sentinel
                        process
                        function)
                       events)))
                   ((symbol-function 'process-mark)
                    (lambda (_process)
                      (unless (marker-buffer marker)
                        (set-marker marker 1 result-buffer))
                      marker)))
           (unwind-protect
               (progn
                 (let* ((ag-executable "/opt/ag")
                        (ag-dired-arguments
                         '("--nocolor" "-S"))
                        (ag-reuse-buffers nil)
                        (call-result
                         (condition-case error-data
                             (progn
                               (ag-dired-regexp
                                root
                                "src/.*\\.el")
                               'completed)
                           (error
                            (list
                             'error
                             error-data)))))
                   (if (buffer-live-p result-buffer)
                       (with-current-buffer result-buffer
                         (list
                          call-result
                          (buffer-name)
                          default-directory
                          major-mode
                          dired-sort-inhibit
                          (functionp
                           revert-buffer-function)
                          (help-function-arglist
                           revert-buffer-function t)
                          (lookup-key
                           (current-local-map)
                           "\C-c\C-k")
                          mode-line-process
                          (marker-position marker)
                          (buffer-substring-no-properties
                           (point-min)
                           (point-max))
                          (nreverse events)))
                     (list
                      call-result
                      (nreverse events)))))
             (set-marker marker nil)
             (when (buffer-live-p result-buffer)
               (cl-letf (((symbol-function
                           'get-buffer-process)
                          (lambda (_buffer) nil)))
                 (kill-buffer result-buffer)))
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (completed "*ag dired pattern:src/.*\\.el dir:[ORACLE-SANDBOX]/ag dired root/*" "[ORACLE-SANDBOX]/ag dired root/" dired-mode t t (ignore-auto noconfirm) ag/kill-process (":%s") 1 "-rw-r--r-- 1 user group 7 sample.el\n  [ORACLE-SANDBOX]/ag dired root/:\n  /opt/ag --nocolor -S -g 'src/.*\\.el' [ORACLE-SANDBOX]/ag\\ dired\\ root/ | grep -v '^$' | sed s/\\'/\\\\\\\\\\'/g | xargs -I '{}' ls -al '{}' &\n" ((switch "*ag dired pattern:src/.*\\.el dir:[ORACLE-SANDBOX]/ag dired root/*") (shell "/opt/ag --nocolor -S -g 'src/.*\\.el' [ORACLE-SANDBOX]/ag\\ dired\\ root/ | grep -v '^$' | sed s/\\'/\\\\\\\\\\'/g | xargs -I '{}' ls -al '{}' &" "*ag dired pattern:src/.*\\.el dir:[ORACLE-SANDBOX]/ag dired root/*") (dired-mode "[ORACLE-SANDBOX]/ag dired root/") subdir-alist (filter (:buffer nil) ag/dired-filter) (sentinel (:buffer nil) ag/dired-sentinel)))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_dired_public_wrappers_escape_literal_and_resolve_project_root() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ag-dired-regexp)
                    (lambda (directory regexp)
                      (push
                       (list directory regexp)
                       calls)
                      'dired))
                   ((symbol-function 'ag/project-root)
                    (lambda (directory)
                      (push
                       (list 'project-root directory)
                       calls)
                      "/project/root/")))
           (let ((default-directory "/work/current/"))
             (list
              (ag-dired "/chosen/" "a.*(b)")
              (ag-project-dired "file+.el")
              (ag-project-dired-regexp "src/.*")
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (dired dired dired (("/chosen/" "a\\.\\*\\(b\\)") (project-root "/work/current/") ("/project/root/" "file\\+\\.el") (project-root "/work/current/") ("/project/root/" "src/.*")))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}
