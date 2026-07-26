use expect_test::expect;

use super::assert_ac_clang_parity;

#[test]
fn clang_server_session_activation_deactivation_and_update_commands_are_stateful_and_idempotent() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name
                     "fixture.cpp")
               (let ((clang-server--session-name
                      nil)
                     (clang-server--process
                      'fake-process)
                     (clang-server-session-establishing-buffers
                      nil)
                     events)
                 (cl-letf
                     (((symbol-function
                        'clang-server--send-create-session-command)
                       (lambda ()
                         (push 'create events)))
                      ((symbol-function
                        'clang-server--send-delete-session-command)
                       (lambda ()
                         (push 'delete events)))
                      ((symbol-function
                        'clang-server--send-reparse-command)
                       (lambda ()
                         (push 'reparse events)))
                      ((symbol-function
                        'clang-server--send-cflags-command)
                       (lambda ()
                         (push 'cflags events)))
                      ((symbol-function
                        'process-live-p)
                       (lambda (_) t)))
                   (list
                    (clang-server-activate-session)
                    (clang-server-activate-session)
                    clang-server--session-name
                    (mapcar
                     #'buffer-name
                     clang-server-session-establishing-buffers)
                    (clang-server-reparse-buffer)
                    (clang-server-update-cflags)
                    (clang-server-deactivate-session)
                    (clang-server-deactivate-session)
                    clang-server--session-name
                    clang-server-session-establishing-buffers
                    (clang-server-reparse-buffer)
                    (clang-server-update-cflags)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (t nil "fixture.cpp" (" *temp*") #2=(reparse . #1=(cflags delete)) #1# t nil nil nil nil nil (create . #2#))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_interactive_cflag_and_prefix_header_setters_preserve_exact_parsing_rules() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'read-string)
                     (lambda (&rest arguments)
                       (push
                        (cons 'read-string arguments)
                        events)
                       "-Wall  -DNAME=値"))
                    ((symbol-function
                      'read-shell-command)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'read-shell-command arguments)
                        events)
                       "flags-command"))
                    ((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push
                        (list 'shell command)
                        events)
                       "-Ione\n-Itwo  -DTHREE=3\n"))
                    ((symbol-function
                      'clang-server-update-cflags)
                     (lambda ()
                       (push
                        (list
                         'update
                         clang-server-cflags)
                        events)
                       'updated)))
                 (with-temp-buffer
                   (setq buffer-file-name
                         "/project/src/file.cpp")
                   (let ((clang-server-cflags nil)
                         (clang-server-prefix-header
                          "old.pch"))
                     (list
                      (clang-server-set-cflags)
                      clang-server-cflags
                      (clang-server-set-cflags-from-shell-command)
                      clang-server-cflags
                      (progn
                        (clang-server-set-prefix-header
                         " \t")
                        clang-server-prefix-header)
                      (progn
                        (clang-server-set-prefix-header
                         "next.pch")
                        clang-server-prefix-header)
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (updated #1=("-Wall" "-DNAME=値") updated #2=("-Ione" "-Itwo" "-DTHREE=3") nil "next.pch" ((read-string "New cflags: ") (update #1#) (read-shell-command "Shell command: " nil nil "../../../../../../../../../project/src/file.cpp") (shell "flags-command") (update #2#)))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_temporary_pch_cleanup_deletes_only_matching_files_below_the_bound_directory() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (temporary-file-directory
                     (expand-file-name
                      "pch-cleanup/"
                      root))
                    (matching
                     (expand-file-name
                      "preamble-one.pch"
                      temporary-file-directory))
                    (matching-two
                     (expand-file-name
                      "preamble-two.pch"
                      temporary-file-directory))
                    (other
                     (expand-file-name
                      "other.pch"
                      temporary-file-directory))
                    (near
                     (expand-file-name
                      "preamble-three.txt"
                      temporary-file-directory)))
               (make-directory
                temporary-file-directory t)
               (dolist
                   (file
                    (list
                     matching matching-two other near))
                 (with-temp-file file
                   (insert "fixture")))
               (list
                (clang-server--clean-tmp-pch)
                (mapcar
                 #'file-exists-p
                 (list
                  matching matching-two other near))))"##;
    let expect = expect!["OK (nil (nil nil t t))"];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_required_version_check_parses_and_compares_all_semantic_components() {
    let elisp_form = r##"(let ((clang-server--executable
                    "/fake/clang-server")
                   (clang-server--require-version
                    '(2 3 4))
                   (responses
                    '("noise"
                      "server version 1.99.99"
                      "server version 2.2.99"
                      "server version 2.3.3"
                      "server version 2.3.4"
                      "server version 2.4.0"
                      "server version 3.0.0"))
                   commands)
               (cl-letf
                   (((symbol-function
                      'shell-command-to-string)
                     (lambda (command)
                       (push command commands)
                       (pop responses))))
                 (list
                  (clang-server--check-require-version-p)
                  (clang-server--check-require-version-p)
                  (clang-server--check-require-version-p)
                  (clang-server--check-require-version-p)
                  (clang-server--check-require-version-p)
                  (clang-server--check-require-version-p)
                  (clang-server--check-require-version-p)
                  responses
                  (nreverse commands))))"##;
    let expect = expect![[
        r#"OK (nil nil nil nil t t t nil ("/fake/clang-server --version" "/fake/clang-server --version" "/fake/clang-server --version" "/fake/clang-server --version" "/fake/clang-server --version" "/fake/clang-server --version" "/fake/clang-server --version"))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_launch_configures_fake_process_packet_codecs_and_clang_parameters_once() {
    let elisp_form = r##"(let ((clang-server--executable
                    "/fake/clang-server")
                   (clang-server--process nil)
                   (clang-server--status
                    'before)
                   (clang-server--transaction-hash
                    (let ((table
                           (make-hash-table
                            :test #'eq)))
                      (puthash 1 'old table)
                      table))
                   (clang-server-input-data-type
                    'json)
                   (clang-server-output-data-type
                    's-expression)
                   (clang-server-stdin-buffer-size
                    2)
                   (clang-server-stdout-buffer-size
                    3)
                   events)
               (cl-letf
                   (((symbol-function
                      'start-process)
                     (lambda (&rest arguments)
                       (push
                        (cons 'start arguments)
                        events)
                       'fake-process))
                    ((symbol-function
                      'set-process-coding-system)
                     (lambda (&rest arguments)
                       (push
                        (cons 'coding arguments)
                        events)))
                    ((symbol-function
                      'set-process-filter)
                     (lambda (&rest arguments)
                       (push
                        (cons 'filter arguments)
                        events)))
                    ((symbol-function
                      'set-process-query-on-exit-flag)
                     (lambda (&rest arguments)
                       (push
                        (cons 'query arguments)
                        events)))
                    ((symbol-function
                      'clang-server--send-clang-parameters-command)
                     (lambda ()
                       (push 'parameters events))))
                 (list
                  (clang-server-launch)
                  (clang-server-launch)
                  clang-server--process
                  clang-server--status
                  (clang-server--count-transaction)
                  clang-server--packet-encoder
                  clang-server--packet-decoder
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (t nil fake-process idle 0 clang-server--encode-json-packet clang-server--decode-s-expression-packet ((start "Clang-Server" "*Clang-Server*" "/fake/clang-server" "--stdin-buffer-size" "2" "--stdout-buffer-size" "3" "--input-data" "json" "--output-data" "s-expression") (coding fake-process utf-8 binary) (filter fake-process clang-server--process-filter) (query fake-process nil) parameters))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_shutdown_and_parameter_update_follow_process_presence_and_live_state() {
    let elisp_form = r##"(let ((clang-server--process
                    'fake-process)
                   (clang-server--status 'idle)
                   events)
               (get-buffer-create
                clang-server--process-buffer-name)
               (cl-letf
                   (((symbol-function
                      'process-live-p)
                     (lambda (_)
                       (push 'live events)
                       t))
                    ((symbol-function
                      'clang-server--send-shutdown-command)
                     (lambda ()
                       (push 'send-shutdown events)))
                    ((symbol-function
                      'clang-server--send-clang-parameters-command)
                     (lambda ()
                       (push 'parameters events)
                       'sent)))
                 (let ((updated
                        (clang-server-update-clang-parameters))
                       (shutdown
                        (clang-server-shutdown)))
                   (list
                    updated shutdown
                    clang-server--process
                    clang-server--status
                    (get-buffer
                     clang-server--process-buffer-name)
                    (clang-server-update-clang-parameters)
                    (nreverse events)))))"##;
    let expect = expect!["OK (t t nil shutdown nil nil (parameters live send-shutdown))"];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_initialize_covers_missing_old_and_preset_executable_paths() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'display-warning)
                     (lambda (type message &rest rest)
                       (push
                        (list
                         'warning type message rest)
                        events)))
                    ((symbol-function
                      'clang-server-launch)
                     (lambda ()
                       (push
                        (list
                         'launch
                         clang-server--executable)
                        events)
                       'launched)))
                 (let ((clang-server--executable
                        nil))
                   (cl-letf
                       (((symbol-function
                          'executable-find)
                         (lambda (name)
                           (push
                            (list 'find-missing name)
                            events)
                           nil)))
                     (clang-server-initialize)))
                 (let ((clang-server--executable
                        nil))
                   (cl-letf
                       (((symbol-function
                          'executable-find)
                         (lambda (name)
                           (push
                            (list 'find-old name)
                            events)
                           "/fake/old"))
                        ((symbol-function
                          'clang-server--check-require-version-p)
                         (lambda ()
                           (push 'version-old events)
                           nil)))
                     (clang-server-initialize)))
                 (let ((clang-server--executable
                        "/configured/server"))
                   (clang-server-initialize))
                 (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((find-missing "clang-server") (warning clang-server "clang-server binary not found." nil) (find-old "clang-server") version-old (warning clang-server "clang-server binary is old. please replace new binary. require version is (2 0 0) over." nil) (launch "/configured/server"))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_reset_and_reboot_visit_only_live_buffers_and_restore_saved_sessions() {
    let elisp_form = r##"(let ((first
                    (generate-new-buffer
                     " *clang-reset-first*"))
                   (second
                    (generate-new-buffer
                     " *clang-reset-second*"))
                   (dead
                    (generate-new-buffer
                     " *clang-reset-dead*"))
                   events)
               (kill-buffer dead)
               (unwind-protect
                   (let ((clang-server--process
                          'fake-process)
                         (clang-server-session-establishing-buffers
                          (list first dead second)))
                     (cl-letf
                         (((symbol-function
                            'clang-server-deactivate-session)
                           (lambda ()
                             (push
                              (list
                               'deactivate
                               (buffer-name))
                              events)
                             t))
                          ((symbol-function
                            'clang-server-activate-session)
                           (lambda ()
                             (push
                              (list
                               'activate
                               (buffer-name))
                              events)
                             t))
                          ((symbol-function
                            'process-live-p)
                           (lambda (_) t))
                          ((symbol-function
                            'clang-server--send-reset-command)
                           (lambda ()
                             (push 'send-reset events)))
                          ((symbol-function
                            'clang-server-shutdown)
                           (lambda ()
                             (push 'shutdown events)
                             t))
                          ((symbol-function
                            'clang-server-launch)
                           (lambda ()
                             (push 'launch events)
                             t))
                          ((symbol-function
                            'message)
                           (lambda
                             (format-string &rest args)
                             (push
                              (apply
                               #'format
                               format-string args)
                              events))))
                       (list
                        (clang-server-reset)
                        clang-server-session-establishing-buffers
                        (progn
                          (setq
                           clang-server-session-establishing-buffers
                           (list first dead second))
                          (clang-server-reboot))
                        (nreverse events))))
                 (when (buffer-live-p first)
                   (kill-buffer first))
                 (when (buffer-live-p second)
                   (kill-buffer second))))"##;
    let expect = expect![[
        r#"OK (t nil t ((deactivate " *clang-reset-first*") (deactivate " *clang-reset-second*") send-reset (deactivate " *clang-reset-first*") (deactivate " *clang-reset-second*") send-reset shutdown launch (activate " *clang-reset-first*") (activate " *clang-reset-second*") "clang-server : reboot success."))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_jump_receiver_stack_back_and_dispatch_commands_preserve_locations_and_boundaries() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name
                     "/project/source.cpp")
               (insert "one\ntwo\n")
               (goto-char 6)
               (let ((ac-clang--jump-stack nil)
                     events)
                 (cl-letf
                     (((symbol-function
                        'ac-clang--jump)
                       (lambda (location)
                         (push
                          (list 'jump location)
                          events)))
                      ((symbol-function
                        'ac-clang-mode--on)
                       (lambda ()
                         (push 'mode-on events)))
                      ((symbol-function
                        'clang-server-request-transaction)
                       (lambda
                         (sender receiver arguments)
                         (push
                          (list
                           'request sender receiver
                           arguments)
                          events)
                         'requested)))
                   (ac-clang--receive-jump
                    '(:Results
                      (:Path "/project/source.cpp"
                       :Line 2 :Column 2))
                    nil)
                   (ac-clang--receive-jump
                    '(:Results
                      (:Path "/project/target.hpp"
                       :Line 9 :Column 4))
                    nil)
                   (let ((stack-before
                          ac-clang--jump-stack))
                     (list
                      stack-before
                      (ac-clang-jump-back)
                      ac-clang--jump-stack
                      (mapcar
                       #'funcall
                       '(ac-clang-jump-inclusion
                         ac-clang-jump-definition
                         ac-clang-jump-declaration
                         ac-clang-jump-smart))
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK ((#1=("/project/source.cpp" 2 1)) #2=((jump #1#) mode-on (request clang-server-send-inclusion-command ac-clang--receive-jump nil) mode-on (request clang-server-send-definition-command ac-clang--receive-jump nil) mode-on (request clang-server-send-declaration-command ac-clang--receive-jump nil) mode-on (request clang-server-send-smart-jump-command ac-clang--receive-jump nil)) nil (requested requested requested requested) ((jump ("/project/target.hpp" 9 3)) . #2#))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_activation_deactivation_lazy_mode_and_snippet_flags_manage_complete_local_state() {
    let elisp_form = r##"(with-temp-buffer
               (let ((ac-sources
                      '(original source))
                     (ac-clang--ac-sources-backup
                      nil)
                     (ac-clang--snippet-expanding-p
                      nil)
                     events)
                 (cl-letf
                     (((symbol-function
                        'clang-server-activate-session)
                       (lambda ()
                         (push 'activate-session events)
                         t))
                      ((symbol-function
                        'clang-server-deactivate-session)
                       (lambda ()
                         (push 'deactivate-session events)
                         t))
                      ((symbol-function
                        'ac-clang-mode)
                       (lambda (&optional argument)
                         (push
                          (list 'mode argument)
                          events)
                         'mode-result)))
                   (let ((activated
                          (ac-clang-activate)))
                     (ac-clang--enter-snippet-expand)
                     (let ((during
                            ac-clang--snippet-expanding-p))
                       (ac-clang--leave-snippet-expand)
                       (let ((deactivated
                              (ac-clang-deactivate)))
                         (set-buffer-modified-p nil)
                         (let ((lazy
                                (ac-clang-activate-after-modify))
                               (first-change
                                first-change-hook))
                           (set-buffer-modified-p t)
                           (let ((immediate
                                  (ac-clang-activate-after-modify)))
                             (list
                              activated deactivated
                              during
                              ac-clang--snippet-expanding-p
                              ac-sources
                              ac-clang--ac-sources-backup
                              first-change lazy
                              immediate
                              before-revert-hook
                              kill-buffer-hook
                              yas-before-expand-snippet-hook
                              yas-after-exit-snippet-hook
                              (nreverse events))))))))))"##;
    let expect = expect![
        "OK (t t t nil (original source) nil #1=(ac-clang-mode t) #1# mode-result nil (yas--on-buffer-kill uniquify-kill-buffer-function vc-kill-buffer-hook) nil nil (activate-session deactivate-session (mode nil)))"
    ];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_initialize_and_finalize_configure_quick_help_keys_hooks_and_server_boundaries() {
    let elisp_form = r##"(let ((ac-clang-quick-help-prefer-pos-tip-p
                    t)
                   (ac-quick-help-prefer-pos-tip
                    nil)
                   (clang-server-session-establishing-buffers-finalize-hooks
                    nil)
                   (kill-emacs-hook nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'clang-server-initialize)
                     (lambda ()
                       (push 'server-initialize events)
                       t))
                    ((symbol-function
                      'clang-server-finalize)
                     (lambda ()
                       (push 'server-finalize events)
                       t))
                    ((symbol-function
                      'ad-disable-advice)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'disable-advice arguments)
                        events)
                       t)))
                 (let ((initialized
                        (ac-clang-initialize)))
                   (let ((after-initialize
                          (list
                           ac-quick-help-prefer-pos-tip
                           (lookup-key
                            ac-mode-map
                            (kbd "M-."))
                           (lookup-key
                            ac-mode-map
                            (kbd "M-,"))
                           clang-server-session-establishing-buffers-finalize-hooks
                           kill-emacs-hook)))
                     (let ((finalized
                            (ac-clang-finalize)))
                       (list
                        initialized
                        after-initialize
                        finalized
                        (lookup-key
                         ac-mode-map
                         (kbd "M-."))
                        (lookup-key
                         ac-mode-map
                         (kbd "M-,"))
                        (nreverse events)))))))"##;
    let expect = expect![
        "OK (t (t ac-clang-jump-smart ac-clang-jump-back (ac-clang-mode--off) (ac-clang-finalize)) t nil nil (server-initialize server-finalize (disable-advice flymake-on-timer-event around ac-clang--flymake-suspend-advice)))"
    ];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_minor_mode_and_on_off_helpers_call_activation_boundaries_for_each_transition() {
    let elisp_form = r##"(with-temp-buffer
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'ac-clang-activate)
                       (lambda ()
                         (push 'activate events)
                         'activated))
                      ((symbol-function
                        'ac-clang-deactivate)
                       (lambda ()
                         (push 'deactivate events)
                         'deactivated)))
                   (list
                    (ac-clang-mode--on)
                    ac-clang-mode
                    (ac-clang-mode--off)
                    ac-clang-mode
                    (ac-clang-mode 1)
                    ac-clang-mode
                    (ac-clang-mode 0)
                    ac-clang-mode
                    (nreverse events)))))"##;
    let expect = expect!["OK (t t nil nil t t nil nil (activate deactivate activate deactivate))"];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_jump_moves_to_exact_line_column_then_inherits_mode_and_cflags_from_source_buffer() {
    let elisp_form = r##"(let* ((source
                     (generate-new-buffer
                      " *clang-jump-source*"))
                    (target
                     (generate-new-buffer
                      " *clang-jump-target*"))
                    (source-path
                     "/project/source.cpp")
                    (target-path
                     "/project/target.hpp")
                    (ac-clang--jump-stack
                     (list
                      (list source-path 1 0)))
                    events)
               (unwind-protect
                   (progn
                     (with-current-buffer source
                       (setq buffer-file-name
                             source-path)
                       (setq major-mode
                             'neomacs-clang-source-mode)
                       (setq clang-server-cflags
                             '("-Iproject" "-DVALUE=1")))
                     (with-current-buffer target
                       (setq buffer-file-name
                             target-path)
                       (insert
                        "line one\n"
                        "line two\n"
                        "line three\n")
                       (setq major-mode
                             'fundamental-mode)
                       (setq clang-server-cflags
                             nil))
                     (cl-letf
                         (((symbol-function
                            'find-file)
                           (lambda (path)
                             (push
                              (list 'find path)
                              events)
                             (switch-to-buffer
                              target)))
                          ((symbol-function
                            'neomacs-clang-source-mode)
                           (lambda ()
                             (push 'source-mode events)
                             (setq major-mode
                                   'neomacs-clang-source-mode))))
                       (with-current-buffer target
                         (ac-clang--jump
                          (list target-path 2 4))
                         (list
                          (buffer-name)
                          (line-number-at-pos)
                          (current-column)
                          major-mode
                          clang-server-cflags
                          (nreverse events)))))
                 (when (buffer-live-p source)
                   (kill-buffer source))
                 (when (buffer-live-p target)
                   (kill-buffer target))))"##;
    let expect = expect![[
        r#"OK (" *clang-jump-target*" 2 4 neomacs-clang-source-mode ("-Iproject" "-DVALUE=1") ((find "/project/target.hpp") source-mode))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_finalize_runs_buffer_then_global_hooks_shuts_down_cleans_and_resets_executable() {
    let elisp_form = r##"(let ((first
                    (generate-new-buffer
                     " *clang-finalize-first*"))
                   (second
                    (generate-new-buffer
                     " *clang-finalize-second*"))
                   (clang-server--executable
                    "/fake/server")
                   (clang-server-tmp-pch-automatic-cleanup-p
                    t)
                   (clang-server-finalize-hooks
                    '(neomacs-clang-global-finalize))
                   events)
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq-local
                        clang-server-session-establishing-buffers-finalize-hooks
                        '(neomacs-clang-buffer-finalize)))
                     (with-current-buffer second
                       (setq-local
                        clang-server-session-establishing-buffers-finalize-hooks
                        '(neomacs-clang-buffer-finalize)))
                     (let ((clang-server-session-establishing-buffers
                            (list first second)))
                       (cl-letf
                           (((symbol-function
                              'neomacs-clang-buffer-finalize)
                             (lambda ()
                               (push
                                (list
                                 'buffer
                                 (buffer-name))
                                events)))
                            ((symbol-function
                              'neomacs-clang-global-finalize)
                             (lambda ()
                               (push 'global events)))
                            ((symbol-function
                              'clang-server-shutdown)
                             (lambda ()
                               (push 'shutdown events)
                               t))
                            ((symbol-function
                              'clang-server--clean-tmp-pch)
                             (lambda ()
                               (push 'cleanup events))))
                         (list
                          (clang-server-finalize)
                          clang-server--executable
                          (nreverse events)))))
                 (when (buffer-live-p first)
                   (kill-buffer first))
                 (when (buffer-live-p second)
                   (kill-buffer second))))"##;
    let expect = expect![[
        r#"OK (t nil ((buffer " *clang-finalize-first*") (buffer " *clang-finalize-second*") global shutdown cleanup))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}
