use expect_test::expect;

use super::assert_ac_clang_parity;

#[test]
fn clang_server_launch_language_and_cflag_builders_cover_every_optional_branch() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (pch
                     (expand-file-name
                      "headers/prefix.pch"
                      root)))
               (list
                (let ((clang-server-stdin-buffer-size
                       nil)
                      (clang-server-stdout-buffer-size
                       nil)
                      (clang-server-input-data-type
                       nil)
                      (clang-server-output-data-type
                       nil)
                      (clang-server-logfile nil))
                  (clang-server--build-launch-options))
                (let ((clang-server-stdin-buffer-size
                       2)
                      (clang-server-stdout-buffer-size
                       5)
                      (clang-server-input-data-type
                       'json)
                      (clang-server-output-data-type
                       's-expression)
                      (clang-server-logfile
                       "server.log"))
                  (clang-server--build-launch-options))
                (mapcar
                 (lambda (entry)
                   (with-temp-buffer
                     (setq major-mode
                           (car entry))
                     (setq buffer-file-name
                           (cdr entry))
                     (clang-server--build-language-option)))
                 '((c-mode . "fixture.c")
                   (c++-mode . "fixture.cpp")
                   (objc-mode . "fixture.m")
                   (objc-mode . "fixture.mm")
                   (fundamental-mode
                    . "fixture.txt")))
                (with-temp-buffer
                  (let ((clang-server-language-option-function
                         (lambda () "cuda")))
                    (clang-server--build-language-option)))
                (with-temp-buffer
                  (let ((major-mode 'c-mode)
                        (clang-server-language-option-function
                         (lambda () nil)))
                    (clang-server--build-language-option)))
                (with-temp-buffer
                  (let ((major-mode 'c++-mode)
                        (clang-server-cflags
                         '("-Wall" "-DNAME=値"))
                        (clang-server-prefix-header
                         pch))
                    (clang-server--build-complete-cflags)))
                (with-temp-buffer
                  (let ((major-mode 'c-mode)
                        (clang-server-cflags nil)
                        (clang-server-prefix-header
                         'not-a-string))
                    (clang-server--build-complete-cflags)))))"##;
    let expect = expect![[
        r#"OK (nil ("--stdin-buffer-size" "2" "--stdout-buffer-size" "5" "--input-data" "json" "--output-data" "s-expression" "--logfile" "server.log") ("c" "c++" "objective-c" "objective-c++" "c++") "cuda" "c" ("-cc1" "-fsyntax-only" "-x" "c++" "-Wall" "-DNAME=値" "-include-pch" "[ORACLE-SANDBOX]/headers/prefix.pch") ("-cc1" "-fsyntax-only" "-x" "c"))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_source_utilities_count_encoded_columns_bytes_and_widen_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
               (set-buffer-file-coding-system
                'utf-8-dos)
               (insert "α\r\nβeta\n")
               (goto-char 5)
               (narrow-to-region 4 7)
               (let ((point-before (point))
                     (restriction
                      (list
                       (point-min)
                       (point-max))))
                 (list
                  (clang-server--column-number-at-pos
                   4)
                  (clang-server--column-number-at-pos
                   6)
                  (clang-server--get-buffer-bytes)
                  (clang-server--get-source-code)
                  (multibyte-string-p
                   (clang-server--get-source-code))
                  (point)
                  point-before
                  restriction
                  (list
                   (point-min)
                   (point-max)))))"##;
    let expect = expect![[r#"OK (1 4 8 "����\15\n����eta\n" nil 5 5 (4 7) (4 7))"#]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_profiler_macros_register_mark_and_append_only_when_enabled() {
    let elisp_form = r##"(let ((clang-server-debug-profiler-p
                    nil)
                   (clang-server--debug-profiler-hash
                    (make-hash-table
                     :test #'eq))
                   (times '(1.0 2.0 3.0 4.0))
                   profile)
               (cl-letf
                   (((symbol-function
                      'float-time)
                     (lambda ()
                       (pop times))))
                 (clang-server--mark-and-register-profiler
                  7 :disabled)
                 (clang-server--mark-profiler
                  profile :disabled)
                 (setq
                  clang-server-debug-profiler-p
                  t)
                 (clang-server--mark-and-register-profiler
                  7 :registered)
                 (clang-server--mark-profiler
                  profile :local)
                 (clang-server--append-profiler
                  7 profile)
                 (list
                  (gethash
                   7
                   clang-server--debug-profiler-hash)
                  profile
                  times)))"##;
    let expect = expect!["OK ((:registered 1.0 . #1=(:local 2.0)) #1# (3.0 4.0))"];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_transaction_registry_and_under_limit_request_preserve_exact_sender_receiver_args() {
    let elisp_form = r##"(let ((clang-server--transaction-id
                    4)
                   (clang-server--transaction-limit
                    3)
                   (clang-server--transaction-hash
                    (make-hash-table
                     :test #'eq))
                   events)
               (cl-letf
                   (((symbol-function
                      'neomacs-clang-sender)
                     (lambda (arguments)
                       (push
                        (list 'sender arguments)
                        events)
                       (setq
                        clang-server--transaction-id
                        (1+
                         clang-server--transaction-id))
                       'sent))
                    ((symbol-function
                      'neomacs-clang-receiver)
                     (lambda (&rest arguments)
                       (push
                        (cons 'receiver arguments)
                        events))))
                 (let ((request
                        (clang-server-request-transaction
                         #'neomacs-clang-sender
                         #'neomacs-clang-receiver
                         '(:fixture 1))))
                   (list
                    request
                    (clang-server--count-transaction)
                    (clang-server--query-transaction
                     4)
                    (clang-server--unregister-transaction
                     4)
                    (clang-server--unregister-transaction
                     4)
                    (clang-server--count-transaction)
                    (nreverse events)))))"##;
    let expect = expect![
        "OK (sent 1 #1=(:receiver neomacs-clang-receiver :sender neomacs-clang-sender :args #2=(:fixture 1)) #1# nil 0 ((sender #2#)))"
    ];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_transaction_limit_covers_disabled_recovery_clear_and_reboot_paths() {
    let elisp_form = r##"(let ((clang-server--transaction-limit
                    1)
                   (clang-server--transaction-hash
                    (make-hash-table
                     :test #'eq))
                   events)
               (puthash 0 'busy
                        clang-server--transaction-hash)
               (cl-letf
                   (((symbol-function
                      'message)
                     (lambda (format-string &rest args)
                       (push
                        (apply
                         #'format format-string args)
                        events)))
                    ((symbol-function
                      'sleep-for)
                     (lambda (seconds)
                       (push
                        (list 'sleep seconds)
                        events)))
                    ((symbol-function
                      'clang-server-get-server-specification)
                     (lambda ()
                       (push 'specification events)))
                    ((symbol-function
                      'clang-server-reboot)
                     (lambda ()
                       (push 'reboot events))))
                 (let ((clang-server-automatic-recovery-p
                        nil))
                   (clang-server-request-transaction
                    #'ignore nil nil))
                 (let ((clang-server-automatic-recovery-p
                        t))
                   (clang-server-request-transaction
                    #'ignore nil nil))
                 (puthash 1 'busy
                          clang-server--transaction-hash)
                 (cl-letf
                     (((symbol-function
                        'clang-server-get-server-specification)
                       (lambda ()
                         (push 'specification-busy
                               events)
                         (puthash
                          2 'new-request
                          clang-server--transaction-hash))))
                   (let ((clang-server-automatic-recovery-p
                          t))
                     (clang-server-request-transaction
                      #'ignore nil nil)))
                 (list
                  (clang-server--count-transaction)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (1 ("clang-server : The number of requests of the transaction reached the limit." "clang-server : The number of requests of the transaction reached the limit." specification (sleep 0.1) "clang-server : clear transaction requests." "clang-server : The number of requests of the transaction reached the limit." specification-busy (sleep 0.1) reboot))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_packet_encoders_and_decoders_round_trip_sexp_json_and_plain_text() {
    let elisp_form = r##"(let* ((data
                      '(:Name "値"
                        :Count 2
                        :Items ["a" "β"]
                        :Enabled t))
                    (sexp
                     (clang-server--encode-s-expression-packet
                      data))
                    (json
                     (clang-server--encode-json-packet
                      data)))
               (list
                (clang-server--encode-plane-text-packet
                 "plain")
                (clang-server--decode-plane-text-packet
                 "plain")
                sexp
                (clang-server--decode-s-expression-packet
                 sexp)
                json
                (clang-server--decode-json-packet
                 json)))"##;
    let expect = expect![[
        r#"OK ("plain" "plain" "(:Name \"値\" :Count 2 :Items [\"a\" \"β\"] :Enabled t)" (:Name "値" :Count 2 :Items ["a" "β"] :Enabled t) "{\"Name\":\"値\",\"Count\":2,\"Items\":[\"a\",\"β\"],\"Enabled\":true}" (:Name "値" :Count 2 :Items ["a" "β"] :Enabled t))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_command_context_and_packet_framing_increment_ids_and_use_character_length() {
    let elisp_form = r##"(let ((clang-server--transaction-id
                    9)
                   (clang-server--process
                    'fake-process)
                   (clang-server-debug-profiler-p
                    nil)
                   sent)
               (cl-letf
                   (((symbol-function
                      'process-send-string)
                     (lambda (_process packet)
                       (push packet sent)
                       'sent)))
                 (let ((first
                        (clang-server--create-command-context
                         '(:CommandName "ONE"))))
                   (setq
                    clang-server-debug-profiler-p
                    t)
                   (let ((second
                          (clang-server--create-command-context
                           '(:CommandName "TWO"))))
                     (let ((clang-server--packet-encoder
                            (lambda (_)
                              "αβ")))
                       (list
                        first
                        second
                        clang-server--transaction-id
                        (clang-server--send-command-packet
                         second)
                        (nreverse sent)))))))"##;
    let expect = expect![[
        r#"OK ((:RequestId 9 :CommandName "ONE") (:RequestId 10 :CommandName "TWO" :IsProfile t) 11 sent ("PacketSize:2\nαβ"))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_simple_command_senders_emit_complete_exact_property_lists() {
    let elisp_form = r##"(let ((clang-server-translation-unit-flags
                    "TU")
                   (clang-server-complete-at-flags
                    "CC")
                   (clang-server-complete-results-limit
                    17)
                   (clang-server--session-name
                    "session")
                   (clang-server--transaction-id
                    0)
                   (clang-server--process
                    'fake-process)
                   (clang-server--packet-encoder
                    (lambda (context)
                      (format "%S" context)))
                   (clang-server-debug-log-buffer-p
                    nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'process-send-string)
                     (lambda (_process packet)
                       (push packet events)
                       'sent))
                    ((symbol-function
                      'process-status)
                     (lambda (_)
                       'run)))
                 (mapc
                  #'funcall
                  '(clang-server--send-specification-command
                    clang-server--send-clang-parameters-command
                    clang-server--send-delete-session-command
                    clang-server--send-reset-command
                    clang-server--send-shutdown-command
                    clang-server-send-suspend-command
                    clang-server-send-resume-command))
                 (nreverse events)))"##;
    let expect = expect![[
        r#"OK ("PacketSize:69\n(:RequestId 0 :CommandType \"Server\" :CommandName \"GET_SPECIFICATION\")" "PacketSize:146\n(:RequestId 1 :CommandType \"Server\" :CommandName \"SET_CLANG_PARAMETERS\" :TranslationUnitFlags \"TU\" :CompleteAtFlags \"CC\" :CompleteResultsLimit 17)" "PacketSize:89\n(:RequestId 2 :CommandType \"Server\" :CommandName \"DELETE_SESSION\" :SessionName \"session\")" "PacketSize:57\n(:RequestId 3 :CommandType \"Server\" :CommandName \"RESET\")" "PacketSize:60\n(:RequestId 4 :CommandType \"Server\" :CommandName \"SHUTDOWN\")" "PacketSize:83\n(:RequestId 5 :CommandType \"Session\" :CommandName \"SUSPEND\" :SessionName \"session\")" "PacketSize:82\n(:RequestId 6 :CommandType \"Session\" :CommandName \"RESUME\" :SessionName \"session\")")"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_source_command_senders_emit_session_flags_positions_and_full_source() {
    let elisp_form = r##"(with-temp-buffer
               (insert "αx\nthird")
               (setq buffer-file-name
                     "fixture.cpp")
               (goto-char 3)
               (let ((clang-server--session-name
                     "session")
                     (clang-server-cflags
                      '("-Wall"))
                     (clang-server-prefix-header
                      nil)
                     (clang-server--transaction-id
                      0)
                     (clang-server--process
                      'fake-process)
                     (clang-server--packet-encoder
                      (lambda (context)
                        (format "%S" context)))
                     (clang-server-debug-log-buffer-p
                      nil)
                     events)
                 (cl-letf
                     (((symbol-function
                        'process-send-string)
                       (lambda (_process packet)
                         (push packet events)
                         'sent)))
                   (clang-server--send-create-session-command)
                   (clang-server--send-cflags-command)
                   (clang-server--send-reparse-command)
                   (clang-server-send-completion-command
                    '(:start-point 2))
                   (clang-server-send-diagnostics-command)
                   (clang-server-send-inclusion-command)
                   (clang-server-send-definition-command)
                   (clang-server-send-declaration-command)
                   (clang-server-send-smart-jump-command)
                   (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("PacketSize:172\n(:RequestId 0 :CommandType \"Server\" :CommandName \"CREATE_SESSION\" :SessionName \"session\" :CFLAGS (\"-cc1\" \"-fsyntax-only\" \"-x\" \"c++\" \"-Wall\") :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:169\n(:RequestId 1 :CommandType \"Session\" :CommandName \"SET_CFLAGS\" :SessionName \"session\" :CFLAGS (\"-cc1\" \"-fsyntax-only\" \"-x\" \"c++\" \"-Wall\") :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:114\n(:RequestId 2 :CommandType \"Session\" :CommandName \"REPARSE\" :SessionName \"session\" :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:135\n(:RequestId 3 :CommandType \"Session\" :CommandName \"COMPLETION\" :SessionName \"session\" :Line 1 :Column 3 :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:118\n(:RequestId 4 :CommandType \"Session\" :CommandName \"SYNTAXCHECK\" :SessionName \"session\" :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:134\n(:RequestId 5 :CommandType \"Session\" :CommandName \"INCLUSION\" :SessionName \"session\" :Line 1 :Column 4 :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:135\n(:RequestId 6 :CommandType \"Session\" :CommandName \"DEFINITION\" :SessionName \"session\" :Line 1 :Column 4 :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:136\n(:RequestId 7 :CommandType \"Session\" :CommandName \"DECLARATION\" :SessionName \"session\" :Line 1 :Column 4 :SourceCode \"\\316\\261x\\nthird\")" "PacketSize:134\n(:RequestId 8 :CommandType \"Session\" :CommandName \"SMARTJUMP\" :SessionName \"session\" :Line 1 :Column 4 :SourceCode \"\\316\\261x\\nthird\")")"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn clang_server_process_filter_accumulates_fragments_dispatches_transaction_and_clears_buffer() {
    let elisp_form = r##"(let* ((receive-buffer
                      (generate-new-buffer
                       " *clang-server-receive*"))
                    (receive-marker
                     (with-current-buffer
                         receive-buffer
                       (copy-marker
                        (point-min))))
                    (clang-server--transaction-hash
                     (make-hash-table
                      :test #'eq))
                    (clang-server--packet-decoder
                     #'clang-server--decode-s-expression-packet)
                    (clang-server--status 'idle)
                    events)
               (unwind-protect
                   (progn
                     (puthash
                      7
                      '(:receiver
                        neomacs-clang-receiver
                        :args (:fixture 1))
                      clang-server--transaction-hash)
                     (cl-letf
                         (((symbol-function
                            'process-buffer)
                           (lambda (_)
                             receive-buffer))
                          ((symbol-function
                            'process-mark)
                           (lambda (_)
                             receive-marker))
                          ((symbol-function
                            'neomacs-clang-receiver)
                           (lambda (data arguments)
                             (push
                              (list
                               'receiver data arguments)
                              events)))
                          ((symbol-function
                            'message)
                           (lambda
                             (format-string &rest args)
                             (push
                              (apply
                               #'format
                               format-string args)
                              events))))
                       (clang-server--process-filter
                        'fake
                        "(:RequestId 7 :Results ")
                       (let ((mid-state
                              (list
                               clang-server--status
                               (with-current-buffer
                                   receive-buffer
                                 (buffer-string))
                               (clang-server--count-transaction))))
                         (clang-server--process-filter
                          'fake
                          "ok)$")
                         (list
                          mid-state
                          clang-server--status
                          clang-server--command-result-data
                          (clang-server--count-transaction)
                          (with-current-buffer
                              receive-buffer
                            (buffer-string))
                          (nreverse events)))))
                 (kill-buffer receive-buffer)))"##;
    let expect = expect![[
        r#"OK ((receive "(:RequestId 7 :Results " 1) idle #1=(:RequestId 7 :Results ok) 0 "" ((receiver #1# (:fixture 1))))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}
