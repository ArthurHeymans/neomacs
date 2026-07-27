use expect_test::expect;

use super::assert_age_parity;

#[test]
fn age_start_builds_armored_output_command_environment_and_raw_processes() {
    let elisp_form = r##"(let ((context
                (cl-letf (((symbol-function
                            'age-find-configuration)
                           (lambda (_protocol)
                             '((program . "/opt/bin/rage")))))
                  (age-make-context 'Age t)))
               pipe-spec
               process-spec
               process-buffer
               error-buffer)
         (setf (age-context-output-file context) "/vault/output.age")
         (unwind-protect
             (cl-letf (((symbol-function 'make-pipe-process)
                        (lambda (&rest arguments)
                          (setq pipe-spec arguments)
                          'stderr-process))
                       ((symbol-function 'make-process)
                        (lambda (&rest arguments)
                          (setq process-spec
                                (append
                                 arguments
                                 (list
                                  :inside-emacs
                                  (replace-regexp-in-string
                                   "[0-9.]+,age\\'"
                                   "<VERSION>,age"
                                   (car process-environment))
                                  :file-modes
                                  (default-file-modes))))
                          (setq process-buffer
                                (plist-get arguments :buffer))
                          'main-process)))
               (age--start
                context
                '("--encrypt" "-r" "age1recipient" "--" "plain.txt"))
               (setq error-buffer
                     (age-context-error-buffer context))
               (list
                (age-context-process context)
                (cl-loop for (key value) on pipe-spec by #'cddr
                         collect
                         (list key
                               (cond
                                ((bufferp value) (buffer-name value))
                                ((functionp value) t)
                                (t value))))
                (cl-loop for (key value) on process-spec by #'cddr
                         collect
                         (list key
                               (cond
                                ((bufferp value) (buffer-name value))
                                ((functionp value) t)
                                (t value))))
                (with-current-buffer process-buffer
                  (list enable-multibyte-characters
                        age-read-point
                        age-process-filter-running
                        (eq age-context context)))
                (with-current-buffer error-buffer
                  (eq age-context context))))
           (when (buffer-live-p process-buffer)
             (kill-buffer process-buffer))
           (when (buffer-live-p error-buffer)
             (kill-buffer error-buffer))))"##;
    let expect = expect![[
        r#"OK (main-process ((:name "age-error") (:buffer "*age-error") (:sentinel t) (:filter t) (:noquery t)) ((:name "age") (:buffer " *age*") (:command ("/opt/bin/rage" "--armor" "--output" "/vault/output.age" "--encrypt" "-r" "age1recipient" "--" "plain.txt")) (:connection-type pipe) (:coding raw-text) (:filter t) (:stderr stderr-process) (:noquery t) (:inside-emacs "INSIDE_EMACS=<VERSION>,age") (:file-modes 448)) (nil 1 nil t) t)"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_stderr_filter_records_error_failure_status_and_debug_transcript() {
    let elisp_form = r##"(let* ((context
                  (cl-letf (((symbol-function
                              'age-find-configuration)
                             (lambda (_protocol)
                               '((program . "age")))))
                    (age-make-context)))
                 (buffer (generate-new-buffer " *age-filter-test*"))
                 (process
                  (make-pipe-process
                   :name "age-filter-test"
                   :buffer buffer
                   :noquery t))
                 (age-debug t)
                 (age-debug-buffer
                  (generate-new-buffer " *age-debug-test*")))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (setq-local age-context context)
                 (setq-local age-process-filter-running nil))
               (age--process-stderr-filter
                process
                "age: error: no identity matched\n")
               (let ((informational-result
                      (condition-case error-data
                          (age--process-stderr-filter
                           process
                           "informational line\n")
                        (error
                         (list
                          (car error-data)
                          (cadr error-data)
                          (nth 2 error-data)
                          (nth 3 error-data))))))
               (list
                (age-context-result-for context 'error)
                (age-context-result-for context 'age-failed)
                (with-current-buffer age-debug-buffer
                  (buffer-string))
                (with-current-buffer buffer
                  age-process-filter-running)
                informational-result)))
           (when (process-live-p process)
             (delete-process process))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))
           (when (buffer-live-p age-debug-buffer)
             (kill-buffer age-debug-buffer))))"##;
    let expect = expect![[
        r#"OK (((age-error "no identity matched")) t "age: error: no identity matched\ninformational line\n" nil (args-out-of-range "informational line\n" 12 31))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_stdout_filter_is_silent_normally_and_messages_exact_debug_payload() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages))))
           (let ((age-debug nil))
             (age--process-stdout-filter nil "first\n"))
           (let ((age-debug t))
             (age--process-stdout-filter nil "cipher bytes\n"))
           (nreverse messages)))"##;
    let expect = expect![[r#"OK ("debug: age stdout: cipher bytes\n")"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_wait_for_completion_pumps_process_reverses_errors_and_captures_stderr() {
    let elisp_form = r##"(let* ((context
                  (cl-letf (((symbol-function
                              'age-find-configuration)
                             (lambda (_protocol)
                               '((program . "age")))))
                    (age-make-context)))
                 (error-buffer
                  (generate-new-buffer " *age-wait-error*"))
                 (statuses '(run run exit))
                 events)
         (with-current-buffer error-buffer
           (insert "age: error: failed\n"))
         (setf (age-context-process context) 'fake-process
               (age-context-error-buffer context) error-buffer
               (age-context-result context)
               '((error (age-error "second")
                        (age-error "first"))))
         (unwind-protect
             (cl-letf (((symbol-function 'process-status)
                        (lambda (process)
                          (push (list 'status process) events)
                          (prog1 (car statuses)
                            (setq statuses (cdr statuses)))))
                       ((symbol-function 'accept-process-output)
                        (lambda (process seconds)
                          (push (list 'accept process seconds) events)
                          t))
                       ((symbol-function 'sleep-for)
                        (lambda (seconds)
                          (push (list 'sleep seconds) events))))
               (age-wait-for-completion context)
               (list
                (age-context-result-for context 'error)
                (age-context-error-output context)
                (nreverse events)))
           (when (buffer-live-p error-buffer)
             (kill-buffer error-buffer))))"##;
    let expect = expect![[
        r#"OK (((age-error "first") (age-error "second")) "age: error: failed\n" ((status fake-process) (accept fake-process 1) (status fake-process) (accept fake-process 1) (status fake-process) (sleep 0.1)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_cancel_records_quit_in_process_buffer_and_stops_running_process() {
    let elisp_form = r##"(let* ((context
                  (cl-letf (((symbol-function
                              'age-find-configuration)
                             (lambda (_protocol)
                               '((program . "age")))))
                    (age-make-context)))
                 (buffer
                  (generate-new-buffer " *age-cancel-test*"))
                 (process
                  (make-pipe-process
                   :name "age-cancel-test"
                   :buffer buffer
                   :noquery t))
                 deleted)
         (setf (age-context-process context) process)
         (with-current-buffer buffer
           (setq-local age-context context)
           (age-context-set-result-for
            context 'error
            '((age-error "existing"))))
         (unwind-protect
             (cl-letf (((symbol-function 'process-status)
                        (lambda (_process) 'run))
                       ((symbol-function 'delete-process)
                        (lambda (passed-process)
                          (setq deleted
                                (eq passed-process process)))))
               (age-cancel context)
               (list
                (age-context-result-for context 'error)
                deleted
                (buffer-live-p buffer)))
           (when (process-live-p process)
             (delete-process process))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[r#"OK (((quit) (age-error "existing")) t t)"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_start_rejects_reusing_context_with_running_process_before_allocating_buffers() {
    let elisp_form = r##"(let ((context
                (cl-letf (((symbol-function
                            'age-find-configuration)
                           (lambda (_protocol)
                             '((program . "rage")))))
                  (age-make-context)))
               allocated)
         (setf (age-context-process context) 'running-process)
         (cl-letf (((symbol-function 'process-status)
                    (lambda (_process) 'run))
                   ((symbol-function 'generate-new-buffer)
                    (lambda (&rest _arguments)
                      (setq allocated t)
                      (error "unexpected allocation"))))
           (list
            (condition-case error-data
                (age--start context '("--decrypt"))
              (error
               (list (car error-data)
                     (cadr error-data))))
            allocated
            (age-context-process context))))"##;
    let expect =
        expect![[r#"OK ((error "rage is already running in this context") nil running-process)"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_status_passphrase_prompts_sends_encoded_line_and_records_quit() {
    let elisp_form = r##"(let* ((context
                  (cl-letf (((symbol-function
                              'age-find-configuration)
                             (lambda (_protocol)
                               '((program . "rage")))))
                    (age-make-context)))
                 callback-events
                 process-events)
         (setf (age-context-process context) 'fake-process
               (age-context-passphrase-callback context)
               (cons
                (lambda (passed-context handback)
                  (push (list (eq passed-context context)
                              handback)
                        callback-events)
                  (copy-sequence "sëcret"))
                '("vault.age")))
         (cl-letf (((symbol-function 'process-send-string)
                    (lambda (process string)
                      (push (list 'send process
                                  (string-to-list string))
                            process-events)))
                   ((symbol-function 'delete-process)
                    (lambda (process)
                      (push (list 'delete process)
                            process-events))))
           (let ((age-passphrase-coding-system 'utf-8))
             (age--status-GET_PASSPHRASE
              context
              "passphrase.enter"))
           (age--status-GET_PASSPHRASE context "unrelated")
           (setf (age-context-passphrase-callback context)
                 (list
                  (lambda (&rest _arguments)
                    (signal 'quit nil))))
           (age--status-GET_PASSPHRASE
            context
            "passphrase.retry")
           (list
            (nreverse callback-events)
            (nreverse process-events)
            (age-context-result-for context 'error))))"##;
    let expect = expect![[
        r#"OK (((t ("vault.age"))) ((send fake-process (115 195 171 99 114 101 116 10)) (delete fake-process)) ((quit)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_start_decrypt_composes_identity_files_values_lists_and_passphrase_mode() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               calls)
         (let ((identity-file
                (expand-file-name "identities.txt" root))
               (cipher-file
                (expand-file-name "vault.age" root)))
           (write-region "AGE-SECRET-KEY-1..." nil identity-file
                         nil 'quiet)
           (write-region "cipher" nil cipher-file nil 'quiet)
           (cl-letf (((symbol-function 'age--start)
                      (lambda (context arguments)
                        (push (list
                               (age-context-operation context)
                               arguments)
                              calls))))
             (dolist
                 (case
                  `((,identity-file nil)
                    ((,identity-file "/missing/id") nil)
                    (nil t)))
               (pcase-let ((`(,identity ,passphrase) case))
                 (let ((age-default-identity identity)
                       (age-always-use-default-keys t)
                       (context
                        (cl-letf
                            (((symbol-function
                               'age-find-configuration)
                              (lambda (_protocol)
                                '((program . "age")))))
                          (age-make-context))))
                   (setf (age-context-passphrase context)
                         passphrase)
                   (age-start-decrypt
                    context
                    (age-make-data-from-file cipher-file)))))
             (list
              (nreverse calls)
              (condition-case error-data
                  (let ((context
                         (cl-letf
                             (((symbol-function
                                'age-find-configuration)
                               (lambda (_protocol)
                                 '((program . "age")))))
                           (age-make-context))))
                    (age-start-decrypt
                     context
                     (age-make-data-from-string "cipher")))
                (error
                 (list (car error-data)
                       (cadr error-data))))))))"##;
    let expect = expect![[
        r#"OK (((decrypt ("--decrypt" "-i" "[ORACLE-SANDBOX]/identities.txt" "--" "[ORACLE-SANDBOX]/vault.age")) (decrypt ("--decrypt" "-i" "[ORACLE-SANDBOX]/identities.txt" "--" "[ORACLE-SANDBOX]/vault.age")) (decrypt ("--decrypt" "--" "[ORACLE-SANDBOX]/vault.age"))) (error "Not a file"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_start_encrypt_composes_file_and_literal_recipients_and_streams_string_data() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               calls sends eof-events)
         (let ((recipient-file
                (expand-file-name "recipients.txt" root))
               (plain-file
                (expand-file-name "plain.txt" root)))
           (write-region "ssh-ed25519 AAAA..." nil recipient-file
                         nil 'quiet)
           (write-region "plain" nil plain-file nil 'quiet)
           (cl-letf (((symbol-function 'age--start)
                      (lambda (context arguments)
                        (push arguments calls)
                        (setf (age-context-process context)
                              'fake-process)))
                     ((symbol-function 'process-status)
                      (lambda (_process) 'run))
                     ((symbol-function 'process-send-string)
                      (lambda (process string)
                        (push (list process string) sends)))
                     ((symbol-function 'process-send-eof)
                      (lambda (process)
                        (push process eof-events))))
             (let ((age-default-recipient
                    "age1defaultrecipient")
                   (context
                    (cl-letf
                        (((symbol-function
                           'age-find-configuration)
                          (lambda (_protocol)
                            '((program . "age")))))
                      (age-make-context))))
               (age-start-encrypt
                context
                (age-make-data-from-file plain-file)
                (list recipient-file
                      "age1literalrecipient")))
             (let ((age-default-recipient nil)
                   (context
                    (cl-letf
                        (((symbol-function
                           'age-find-configuration)
                          (lambda (_protocol)
                            '((program . "age")))))
                      (age-make-context))))
               (age-start-encrypt
                context
                (age-make-data-from-string "streamed plaintext")
                nil))
             (list
              (nreverse calls)
              (nreverse sends)
              (nreverse eof-events)))))"##;
    let expect = expect![[
        r#"OK ((("--encrypt" "-R" "[ORACLE-SANDBOX]/recipients.txt" "-r" "age1literalrecipient" "--" "[ORACLE-SANDBOX]/plain.txt") ("--encrypt" "-p")) ((fake-process "streamed plaintext")) (fake-process))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_decrypt_and_encrypt_sync_workflows_manage_outputs_cleanup_and_error_signals() {
    let elisp_form = r##"(let ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               events
               counter)
         (cl-letf (((symbol-function 'make-temp-file)
                    (lambda (prefix &rest _arguments)
                      (setq counter (1+ (or counter 0)))
                      (expand-file-name
                       (format "%s-%d" prefix counter)
                       root)))
                   ((symbol-function 'age-start-decrypt)
                    (lambda (context cipher)
                      (push (list 'decrypt
                                  (age-data-file cipher))
                            events)
                      (write-region
                       "decrypted payload"
                       nil
                       (age-context-output-file context)
                       nil 'quiet)))
                   ((symbol-function 'age-start-encrypt)
                    (lambda (context plain recipients)
                      (push (list 'encrypt
                                  (and (age-data-file plain)
                                       (with-temp-buffer
                                         (insert-file-contents-literally
                                          (age-data-file plain))
                                         (buffer-string)))
                                  recipients)
                            events)
                      (write-region
                       "encrypted payload"
                       nil
                       (age-context-output-file context)
                       nil 'quiet)))
                   ((symbol-function 'age-wait-for-completion)
                    (lambda (_context)
                      (push 'wait events)))
                   ((symbol-function 'age-reset)
                    (lambda (context)
                      (push (list 'reset
                                  (age-context-operation context))
                            events))))
           (let ((context
                  (cl-letf
                      (((symbol-function 'age-find-configuration)
                        (lambda (_protocol)
                          '((program . "age")))))
                    (age-make-context))))
             (list
              (age-decrypt-string context "cipher bytes")
              (age-encrypt-string
               context
               "plain bytes"
               '("age1recipient"))
              (nreverse events)
              (directory-files root nil
                               "\\`age-\\(input\\|output\\)")))))"##;
    let expect = expect![[
        r#"OK ("decrypted payload" "encrypted payload" ((decrypt "[ORACLE-SANDBOX]/age-input-1") wait (reset nil) (encrypt "plain bytes" ("age1recipient")) wait (reset nil)) nil)"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_file_crypto_wrappers_support_returned_and_explicit_outputs_with_cleanup() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                (plain-input
                 (expand-file-name "plain-input.txt" root))
                (cipher-input
                 (expand-file-name "cipher-input.age" root))
                (plain-output
                 (expand-file-name "plain-output.txt" root))
                (cipher-output
                 (expand-file-name "cipher-output.age" root))
                events
                counter)
         (write-region "plain source" nil plain-input nil 'quiet)
         (write-region "cipher source" nil cipher-input nil 'quiet)
         (cl-letf (((symbol-function 'make-temp-file)
                    (lambda (prefix &rest _arguments)
                      (setq counter (1+ (or counter 0)))
                      (expand-file-name
                       (format "%s-%d" prefix counter)
                       root)))
                   ((symbol-function 'age-start-decrypt)
                    (lambda (context cipher)
                      (push (list 'decrypt
                                  (age-data-file cipher)
                                  (age-context-output-file context))
                            events)
                      (write-region
                       "decrypted file payload"
                       nil
                       (age-context-output-file context)
                       nil 'quiet)))
                   ((symbol-function 'age-start-encrypt)
                    (lambda (context plain recipients)
                      (push (list 'encrypt
                                  (age-data-file plain)
                                  recipients
                                  (age-context-output-file context))
                            events)
                      (write-region
                       "encrypted file payload"
                       nil
                       (age-context-output-file context)
                       nil 'quiet)))
                   ((symbol-function 'age-wait-for-completion)
                    (lambda (_context)
                      (push 'wait events)))
                   ((symbol-function 'age-reset)
                    (lambda (_context)
                      (push 'reset events))))
           (let ((decrypt-context
                  (cl-letf
                      (((symbol-function 'age-find-configuration)
                        (lambda (_protocol)
                          '((program . "age")))))
                    (age-make-context)))
                 (encrypt-context
                  (cl-letf
                      (((symbol-function 'age-find-configuration)
                        (lambda (_protocol)
                          '((program . "age")))))
                    (age-make-context))))
             (let ((returned-plain
                    (age-decrypt-file
                     decrypt-context cipher-input nil))
                   (explicit-plain
                    (age-decrypt-file
                     decrypt-context cipher-input plain-output))
                   (returned-cipher
                    (age-encrypt-file
                     encrypt-context plain-input
                     '("age1alice") nil))
                   (explicit-cipher
                    (age-encrypt-file
                     encrypt-context plain-input
                     '("age1bob") cipher-output)))
               (list
                returned-plain
                explicit-plain
                returned-cipher
                explicit-cipher
                (with-temp-buffer
                  (insert-file-contents-literally plain-output)
                  (buffer-string))
                (with-temp-buffer
                  (insert-file-contents-literally cipher-output)
                  (buffer-string))
                (directory-files root nil
                                 "\\`age-output")
                (nreverse events))))))"##;
    let expect = expect![[
        r#"OK ("decrypted file payload" nil "encrypted file payload" nil "decrypted file payload" "encrypted file payload" nil ((decrypt "[ORACLE-SANDBOX]/cipher-input.age" "[ORACLE-SANDBOX]/age-output-1") wait reset (decrypt "[ORACLE-SANDBOX]/cipher-input.age" "[ORACLE-SANDBOX]/plain-output.txt") wait reset (encrypt "[ORACLE-SANDBOX]/plain-input.txt" ("age1alice") "[ORACLE-SANDBOX]/age-output-2") wait reset (encrypt "[ORACLE-SANDBOX]/plain-input.txt" ("age1bob") "[ORACLE-SANDBOX]/cipher-output.age") wait reset))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_error_checks_distinguish_failed_decrypt_and_encrypt_result_shapes() {
    let elisp_form = r##"(let ((context
                (cl-letf (((symbol-function
                            'age-find-configuration)
                           (lambda (_protocol)
                             '((program . "age")))))
                  (age-make-context))))
         (age-context-set-result-for
          context 'error
          '((age-error "bad identity")
            (age-error "bad recipient")))
         (let ((without-flag
                (age--check-error-for-decrypt context)))
           (age-context-set-result-for
            context 'age-failed t)
           (list
            without-flag
            (condition-case error-data
                (age--check-error-for-decrypt context)
              (error
               (list (car error-data)
                     (cdr error-data))))
            (age--wrong-password-p context)
            (progn
              (setf (age-context-error-output context)
                    "age: error: incorrect passphrase\n")
              (age--wrong-password-p context)))))"##;
    let expect = expect![[
        r#"OK (nil (age-error ("Age failed with error" "bad identity; bad recipient")) nil "incorrect passphrase")"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_read_delete_and_reset_manage_real_output_and_process_buffers() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                 (output
                  (expand-file-name "result.bin" root))
                 (process-buffer
                  (generate-new-buffer " *age-reset-process*"))
                 (error-buffer
                  (generate-new-buffer " *age-reset-error*"))
                 (process
                  (make-pipe-process
                   :name "age-reset-process"
                   :buffer process-buffer
                   :noquery t))
                 (context
                  (cl-letf (((symbol-function
                              'age-find-configuration)
                             (lambda (_protocol)
                               '((program . "age")))))
                    (age-make-context))))
         (write-region
          (concat "payload" (string 0 255))
          nil output nil 'quiet)
         (setf (age-context-output-file context) output
               (age-context-process context) process
               (age-context-error-buffer context) error-buffer
               (age-context-edit-callback context) 'callback)
         (let ((read
                (string-to-list (age-read-output context))))
           (age-delete-output-file context)
           (age-reset context)
           (list
            read
            (file-exists-p output)
            (buffer-live-p process-buffer)
            (buffer-live-p error-buffer)
            (age-context-process context)
            (age-context-edit-callback context))))"##;
    let expect = expect!["OK ((112 97 121 108 111 97 100 0 195 191) nil nil nil nil nil)"];
    assert_age_parity(elisp_form, expect);
}
