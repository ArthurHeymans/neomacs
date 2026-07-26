use expect_test::expect;

use super::{assert_ac_dcd_parity, assert_ac_dcd_signal_parity};

#[test]
fn ac_dcd_stop_server_interrupts_the_named_process() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'interrupt-process)
                     (lambda
                         (process
                          &optional current-group)
                       (push
                        (list
                         process
                         current-group)
                        calls)
                       'interrupted)))
                 (list
                  (ac-dcd-stop-server)
                  (nreverse calls)
                  (interactive-form
                   #'ac-dcd-stop-server))))"##;
    let expect = expect![[r#"OK (interrupted (("dcd-server" nil)) (interactive nil))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_start_server_passes_port_flags_and_hidden_buffer_to_start_process() {
    let elisp_form = r##"(let ((ac-dcd-server-executable
                    "/opt/dcd-server")
                   (ac-dcd-server-port 8123)
                   (ac-dcd-flags
                    '("-I/one" "-I/two"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'start-process)
                     (lambda
                         (name buffer program
                          &rest args)
                       (push
                        (list
                         name
                         (buffer-name buffer)
                         program
                         args
                         (eq
                          buffer
                          (current-buffer)))
                        calls)
                       'fake-process)))
                 (unwind-protect
                     (list
                      (ac-dcd-start-server)
                      (nreverse calls)
                      (buffer-live-p
                       (get-buffer
                        " *dcd-server*")))
                   (when
                       (get-buffer
                        " *dcd-server*")
                     (kill-buffer
                      " *dcd-server*")))))"##;
    let expect = expect![[
        r#"OK (fake-process (("dcd-server" " *dcd-server*" "/opt/dcd-server" ("-p" "8123" "-I/one" "-I/two") t)) t)"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_maybe_start_server_obeys_emacs_process_and_pidof_guards() {
    let elisp_form = r##"(let ((ac-dcd-server-executable
                    "server")
                   (ac-dcd-server-port 9166)
                   (ac-dcd-flags nil)
                   process
                   (pid "0")
                   events)
               (cl-letf
                   (((symbol-function
                      'get-process)
                     (lambda (_)
                       process))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push
                        (list 'shell command)
                        events)
                       pid))
                    ((symbol-function
                      'start-process)
                     (lambda (&rest args)
                       (push
                        (cons 'start args)
                        events)
                       'started)))
                 (unwind-protect
                     (list
                      (ac-dcd-maybe-start-server)
                      (progn
                        (setq process
                              'existing)
                        (ac-dcd-maybe-start-server))
                      (progn
                        (setq process nil
                              pid "42\n")
                        (ac-dcd-maybe-start-server))
                      (nreverse events))
                   (when
                       (get-buffer
                        " *dcd-server*")
                     (kill-buffer
                      " *dcd-server*")))))"##;
    let expect = expect![[
        r#"OK (started nil nil ((shell "pidof dcd-server") (start "dcd-server" (:buffer nil) "server" "-p" "9166") (shell "pidof dcd-server")))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_init_server_interrupts_waits_restarts_and_clears_version_cache() {
    let elisp_form = r##"(let ((ac-dcd-version 0.15)
                   (ac-dcd-delay-after-kill-process
                    375)
                   (ac-dcd-server-executable
                    "server")
                   (ac-dcd-server-port 9999)
                   (ac-dcd-flags '("-I/src"))
                   events)
               (cl-letf
                   (((symbol-function
                      'get-process)
                     (lambda (_)
                       'existing))
                    ((symbol-function
                      'interrupt-process)
                     (lambda (process)
                       (push
                        (list 'interrupt process)
                        events)))
                    ((symbol-function
                      'sleep-for)
                     (lambda
                         (seconds milliseconds)
                       (push
                        (list
                         'sleep
                         seconds
                         milliseconds)
                        events)))
                    ((symbol-function
                      'start-process)
                     (lambda (&rest args)
                       (push
                        (cons 'start args)
                        events)
                       'new-process)))
                 (unwind-protect
                     (list
                      (ac-dcd-init-server)
                      ac-dcd-version
                      (nreverse events))
                   (when
                       (get-buffer
                        " *dcd-server*")
                     (kill-buffer
                      " *dcd-server*")))))"##;
    let expect = expect![[
        r#"OK (nil nil ((interrupt "dcd-server") (sleep 0 375) (start "dcd-server" (:buffer nil) "server" "-p" "9999" "-I/src")))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_call_process_forwards_region_tcp_and_arguments_and_reuses_output_buffer() {
    let elisp_form = r##"(let ((ac-dcd-executable
                    "/opt/dcd-client")
                   calls)
               (cl-letf
                   (((symbol-function
                      'call-process-region)
                     (lambda
                         (start end program
                          delete destination
                          display &rest args)
                       (push
                        (list
                         start end program delete
                         (buffer-name destination)
                         display args
                         (buffer-string))
                        calls)
                       (with-current-buffer
                           destination
                         (insert "response"))
                       0)))
                 (unwind-protect
                     (list
                      (with-temp-buffer
                        (insert "source")
                        (ac-dcd-call-process
                         '("-c" "7")))
                      (with-current-buffer
                          ac-dcd-output-buffer-name
                        (buffer-string))
                      (with-temp-buffer
                        (insert "other")
                        (ac-dcd-call-process
                         '("--version")))
                      (with-current-buffer
                          ac-dcd-output-buffer-name
                        (buffer-string))
                      (nreverse calls))
                   (when
                       (get-buffer
                        ac-dcd-output-buffer-name)
                     (kill-buffer
                      ac-dcd-output-buffer-name)))))"##;
    let expect = expect![[
        r#"OK (nil "response" nil "response" ((1 7 "/opt/dcd-client" nil "*dcd-output*" nil ("--tcp" "-c" "7") "source") (1 6 "/opt/dcd-client" nil "*dcd-output*" nil ("--tcp" "--version") "other")))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_call_process_reports_failures_with_deterministic_command_and_error_buffer() {
    let elisp_form = r##"(let ((ac-dcd-executable
                    "dcd-client")
                   displays)
               (cl-letf
                   (((symbol-function
                      'call-process-region)
                     (lambda
                         (_start _end _program
                          _delete destination
                          _display &rest _args)
                       (with-current-buffer
                           destination
                         (insert
                          "/fixture.d: ParseError: unexpected token\n"))
                       17))
                    ((symbol-function
                      'current-time-string)
                     (lambda ()
                       "Thu Jan 01 00:00:00 1970"))
                    ((symbol-function
                      'display-buffer)
                     (lambda (buffer &rest _)
                       (push
                        (buffer-name buffer)
                        displays)
                       'window)))
                 (unwind-protect
                     (list
                      (with-temp-buffer
                        (ac-dcd-call-process
                         '("-c" "12")))
                      (with-current-buffer
                          ac-dcd-error-buffer-name
                        (list
                         (buffer-string)
                         (point)))
                      displays)
                   (dolist
                       (name
                        (list
                         ac-dcd-output-buffer-name
                         ac-dcd-error-buffer-name))
                     (when
                         (get-buffer name)
                       (kill-buffer name))))))"##;
    let expect = expect![[
        r#"OK (window ("Thu Jan 01 00:00:00 1970\n\"dcd-client --tcp -c 12\" failed.\nError type is: ParseError : unexpected token\n" 1) ("*dcd-error*"))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_call_process_with_no_executable_messages_without_spawning() {
    let elisp_form = r##"(let ((ac-dcd-executable nil)
                   messages
                   calls)
               (cl-letf
                   (((symbol-function
                      'message)
                     (lambda
                         (format-string
                          &rest args)
                       (push
                        (apply
                         #'format
                         format-string args)
                        messages)))
                    ((symbol-function
                      'call-process-region)
                     (lambda (&rest args)
                       (push args calls)
                       0)))
                 (unwind-protect
                     (list
                      (with-temp-buffer
                        (ac-dcd-call-process
                         '("--version")))
                      messages
                      calls
                      (with-current-buffer
                          ac-dcd-output-buffer-name
                        (buffer-string)))
                   (when
                       (get-buffer
                        ac-dcd-output-buffer-name)
                     (kill-buffer
                      ac-dcd-output-buffer-name)))))"##;
    let expect =
        expect![[r#"OK (nil ("ac-dcd error: could not find dcd-client executable") nil "")"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_get_ddoc_saves_source_builds_exact_request_and_returns_document() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (source
                     (expand-file-name
                      "ddoc/source.d"
                      root))
                    (ac-dcd-executable
                     "dcd-client")
                    (ac-dcd-server-port
                     9333)
                    calls)
               (make-directory
                (file-name-directory source)
                t)
               (with-temp-file source
                 (insert "void main() {}"))
               (let ((buffer
                      (find-file-noselect source)))
                 (unwind-protect
                     (with-current-buffer buffer
                       (goto-char
                        (point-max))
                       (cl-letf
                           (((symbol-function
                              'call-process-region)
                             (lambda
                                 (start end
                                  program delete
                                  destination
                                  display
                                  &rest args)
                               (push
                                (list
                                 start end program
                                 delete
                                 (buffer-name
                                  destination)
                                 display args)
                                calls)
                               (with-current-buffer
                                   destination
                                 (insert
                                  "Fixture docs\\nline"))
                               0)))
                         (list
                          (ac-dcd-get-ddoc)
                          (nreverse calls)
                          (file-relative-name
                           buffer-file-name
                           root))))
                   (kill-buffer buffer)
                   (when
                       (get-buffer
                        ac-dcd-document-buffer-name)
                     (kill-buffer
                      ac-dcd-document-buffer-name)))))"##;
    let expect = expect![[
        r#"OK ("Fixture docs\\nline" ((1 1 "dcd-client" nil "*dcd-document*" nil ("--tcp" "-c" "15" "-p" "9333" "-d" "[ORACLE-SANDBOX]/ddoc/source.d"))) "ddoc/source.d")"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_get_ddoc_signals_for_each_upstream_empty_document_sentinel() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (source
                     (expand-file-name
                      "ddoc-empty/source.d"
                      root))
                    (ac-dcd-executable
                     "dcd-client")
                    (payload ""))
               (make-directory
                (file-name-directory source)
                t)
               (with-temp-file source
                 (insert "void main() {}"))
               (let ((buffer
                      (find-file-noselect source)))
                 (unwind-protect
                     (with-current-buffer buffer
                       (cl-letf
                           (((symbol-function
                              'call-process-region)
                             (lambda
                                 (_start _end
                                  _program _delete
                                  destination
                                  _display
                                  &rest _args)
                               (with-current-buffer
                                   destination
                                 (insert payload))
                               0)))
                         (ac-dcd-get-ddoc)))
                   (kill-buffer buffer)
                   (when
                       (get-buffer
                        ac-dcd-document-buffer-name)
                     (kill-buffer
                      ac-dcd-document-buffer-name)))))"##;
    let expect = expect![[r#"ERR (error "No document for the symbol at point!")"#]];

    assert_ac_dcd_signal_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_goto_marker_ring_pushes_pops_and_reports_empty_or_deleted_buffers() {
    let elisp_form = r##"(let ((ac-dcd-goto-definition-marker-ring
                    (make-ring 2))
                   (first
                    (get-buffer-create
                     " *ac-dcd-first*"))
                   (second
                    (get-buffer-create
                     " *ac-dcd-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (insert "abcdef")
                       (goto-char 4)
                       (ac-dcd-goto-def-push-marker))
                     (with-current-buffer second
                       (insert "uvwxyz")
                       (goto-char 2)
                       (ac-dcd-goto-def-push-marker))
                     (switch-to-buffer first)
                     (let ((first-pop
                            (progn
                              (ac-dcd-goto-def-pop-marker)
                              (list
                               (buffer-name)
                               (point)
                               (ring-length
                                ac-dcd-goto-definition-marker-ring))))
                           second-pop
                           empty-error)
                       (setq second-pop
                             (progn
                               (ac-dcd-goto-def-pop-marker)
                               (list
                                (buffer-name)
                                (point)
                                (ring-length
                                 ac-dcd-goto-definition-marker-ring))))
                       (setq empty-error
                             (condition-case
                                 error-data
                                 (ac-dcd-goto-def-pop-marker)
                               (error
                                (cons
                                 :error
                                 error-data))))
                       (list
                        first-pop
                        second-pop
                        empty-error)))
                 (dolist
                     (buffer
                      (list first second))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((" *ac-dcd-second*" 2 1) (" *ac-dcd-first*" 4 0) (:error error "Marker ring is empty. Can’t pop."))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_setup_enables_completion_imports_sources_keys_and_optional_yasnippet() {
    let elisp_form = r##"(let ((map
                    (make-sparse-keymap))
                   (ac-sources nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (arg)
                       (push
                        (list 'auto arg)
                        events)))
                    ((symbol-function
                      'hack-dir-local-variables-non-file-buffer)
                     (lambda ()
                       (push 'dir-locals events)))
                    ((symbol-function
                      'yas-minor-mode-on)
                     (lambda ()
                       (push 'yas events)))
                    ((symbol-function
                      'ac-dcd-maybe-start-server)
                     (lambda ()
                       (push 'server events)))
                    ((symbol-function
                      'ac-dcd-add-imports)
                     (lambda ()
                       (push 'imports events))))
                 (cl-progv
                     '(d-mode-map)
                     (list map)
                   (list
                    (ac-dcd-setup)
                    (nreverse events)
                    ac-sources
                    (mapcar
                     (lambda (key)
                       (lookup-key
                        d-mode-map
                        (kbd key)))
                     '("C-c ?" "C-c ." "C-c ," "C-c s"))))))"##;
    let expect = expect![
        "OK (nil ((auto t) dir-locals server imports) (ac-source-dcd) (ac-dcd-show-ddoc-with-buffer ac-dcd-goto-definition ac-dcd-goto-def-pop-marker ac-dcd-search-symbol))"
    ];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_show_ddoc_runs_fetch_reformat_and_display_in_order() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-dcd-get-ddoc)
                     (lambda ()
                       (push 'fetch events)
                       "docs"))
                    ((symbol-function
                      'ac-dcd-reformat-document)
                     (lambda ()
                       (push 'reformat events)))
                    ((symbol-function
                      'display-buffer)
                     (lambda (buffer &rest _)
                       (push
                        (list
                         'display
                         (buffer-name buffer))
                        events)
                       'shown)))
                 (unwind-protect
                     (list
                      (ac-dcd-show-ddoc-with-buffer)
                      (nreverse events))
                   (when
                       (get-buffer
                        ac-dcd-document-buffer-name)
                     (kill-buffer
                      ac-dcd-document-buffer-name)))))"##;
    let expect = expect![[r#"OK (shown (fetch reformat (display "*dcd-document*")))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_symbol_declaration_request_uses_file_cursor_port_and_output_buffer() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (source
                     (expand-file-name
                      "declaration/source.d"
                      root))
                    (ac-dcd-server-port
                     9555)
                    calls)
               (make-directory
                (file-name-directory source)
                t)
               (with-temp-file source
                 (insert "void main() {}"))
               (let ((buffer
                      (find-file-noselect source)))
                 (unwind-protect
                     (with-current-buffer buffer
                       (goto-char 6)
                       (cl-letf
                           (((symbol-function
                              'ac-dcd-call-process)
                             (lambda (args)
                               (push args calls)
                               (with-current-buffer
                                   (get-buffer-create
                                    ac-dcd-output-buffer-name)
                                 (insert
                                  "stdin\t4\n")))))
                         (list
                          (ac-dcd-call-process-for-symbol-declaration)
                          (nreverse calls)
                          (with-current-buffer
                              ac-dcd-output-buffer-name
                            (buffer-string)))))
                   (kill-buffer buffer)
                   (when
                       (get-buffer
                        ac-dcd-output-buffer-name)
                     (kill-buffer
                      ac-dcd-output-buffer-name)))))"##;
    let expect = expect![[
        r#"OK ("stdin\0114\n" (("-c" "6" "-p" "9555" "-l" "[ORACLE-SANDBOX]/declaration/source.d")) "stdin\0114\n")"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_goto_definition_handles_not_found_stdin_and_external_locations() {
    let elisp_form = r##"(let ((source
                    (get-buffer-create
                     " *ac-dcd-goto-source*"))
                   (target
                    (get-buffer-create
                     " *ac-dcd-goto-target*"))
                   (ac-dcd-goto-definition-marker-ring
                    (make-ring 8))
                   location
                   events)
               (unwind-protect
                   (progn
                     (with-current-buffer source
                       (insert "abcdef")
                       (goto-char 5))
                     (with-current-buffer target
                       (insert "uvwxyz"))
                     (switch-to-buffer source)
                     (cl-letf
                         (((symbol-function
                            'save-buffer)
                           (lambda ()
                             (push 'save events)))
                          ((symbol-function
                            'ac-dcd-call-process-for-symbol-declaration)
                           (lambda ()
                             (push 'query events)))
                          ((symbol-function
                            'ac-dcd-parse-output-for-get-symbol-declaration)
                           (lambda ()
                             location))
                          ((symbol-function
                            'message)
                           (lambda
                               (format-string
                                &rest args)
                             (push
                              (cons
                               'message
                               (apply
                                #'format
                                format-string
                                args))
                              events)))
                          ((symbol-function
                            'find-file)
                           (lambda (file)
                             (push
                              (list 'find file)
                              events)
                             (switch-to-buffer
                              target))))
                       (let ((not-found
                              (progn
                                (setq location
                                      '(nil . nil))
                                (ac-dcd-goto-definition)
                                (list
                                 (buffer-name)
                                 (point)
                                 (ring-length
                                  ac-dcd-goto-definition-marker-ring))))
                             stdin-location
                             external-location)
                         (setq location
                               '("stdin" . "2")
                               stdin-location
                               (progn
                                 (ac-dcd-goto-definition)
                                 (list
                                  (buffer-name)
                                  (point)
                                  (ring-length
                                   ac-dcd-goto-definition-marker-ring))))
                         (switch-to-buffer source)
                         (goto-char 6)
                         (setq location
                               '("/fixture/other.d"
                                 . "3")
                               external-location
                               (progn
                                 (ac-dcd-goto-definition)
                                 (list
                                  (buffer-name)
                                  (point)
                                  (ring-length
                                   ac-dcd-goto-definition-marker-ring))))
                         (list
                          not-found
                          stdin-location
                          external-location
                          (nreverse events)))))
                 (dolist
                     (buffer
                      (list source target))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((" *ac-dcd-goto-source*" 5 0) (" *ac-dcd-goto-source*" 3 1) (" *ac-dcd-goto-target*" 4 2) (save query (message . "Not found") save query save query (find "/fixture/other.d")))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_visit_file_in_line_opens_position_and_installs_return_binding() {
    let elisp_form = r##"(let ((results
                    (get-buffer-create
                     ac-dcd-search-symbol-buffer-name))
                   (target
                    (get-buffer-create
                     " *ac-dcd-visit-target*"))
                   opened)
               (unwind-protect
                   (progn
                     (with-current-buffer results
                       (insert
                        "/fixture/module.d 5\n")
                       (goto-char
                        (point-min)))
                     (switch-to-buffer results)
                     (cl-letf
                         (((symbol-function
                            'find-file)
                           (lambda (file)
                             (setq opened file)
                             (switch-to-buffer
                              target))))
                       (ac-dcd-visit-file-in-line)
                       (let ((binding
                              (local-key-binding
                               (kbd
                                "C-c <left>"))))
                         (list
                          opened
                          (buffer-name)
                          (point)
                          (functionp binding)
                          (commandp binding)))))
                 (dolist
                     (buffer
                      (list results target))
                   (when
                       (buffer-live-p buffer)
                     (kill-buffer buffer)))))"##;
    let expect = expect![[r#"OK ("/fixture/module.d" " *ac-dcd-visit-target*" 1 t t)"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_search_symbol_uses_word_or_prompt_and_configures_multi_result_buffer() {
    let elisp_form = r##"(let ((ac-dcd-executable
                    "dcd-client")
                   calls
                   displays
                   prompts)
               (cl-letf
                   (((symbol-function
                      'call-process-region)
                     (lambda
                         (start end program
                          delete destination
                          display &rest args)
                       (push
                        (list
                         start end program delete
                         (buffer-name destination)
                         display args)
                        calls)
                       (with-current-buffer
                           destination
                         (insert
                          "/one.d 1\n/two.d 2\n"))
                       0))
                    ((symbol-function
                      'display-buffer)
                     (lambda (buffer &rest _)
                       (push
                        (buffer-name buffer)
                        displays)
                       'shown))
                    ((symbol-function
                      'read-from-minibuffer)
                     (lambda (prompt &rest _)
                       (push prompt prompts)
                       "prompted")))
                 (unwind-protect
                     (list
                      (with-temp-buffer
                        (insert "needle")
                        (goto-char 3)
                        (ac-dcd-search-symbol))
                      (with-temp-buffer
                        (ac-dcd-search-symbol))
                      (nreverse calls)
                      (nreverse displays)
                      (nreverse prompts)
                      (with-current-buffer
                          ac-dcd-search-symbol-buffer-name
                        (list
                         (buffer-string)
                         (point)
                         (local-key-binding "q")
                         (local-key-binding
                          (kbd "RET")))))
                   (when
                       (get-buffer
                        ac-dcd-search-symbol-buffer-name)
                     (kill-buffer
                      ac-dcd-search-symbol-buffer-name)))))"##;
    let expect = expect![[
        r#"OK (ac-dcd-visit-file-in-line ac-dcd-visit-file-in-line ((1 1 "dcd-client" nil "*dcd-search-symbol*" nil ("--tcp" "--search" "needle")) (1 1 "dcd-client" nil "*dcd-search-symbol*" nil ("--tcp" "--search" "prompted"))) ("*dcd-search-symbol*" "*dcd-search-symbol*") ("Enter symbol: ") ("/one.d 1\n/two.d 2" 1 delete-window ac-dcd-visit-file-in-line))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}
