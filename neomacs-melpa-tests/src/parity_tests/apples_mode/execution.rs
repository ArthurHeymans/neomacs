use expect_test::expect;

use super::{assert_apples_mode_parity, assert_apples_mode_signal_parity};

#[test]
fn applescript_process_uses_file_arguments_or_inline_script_flags_and_delivers_callback_output() {
    let elisp_form = r##"(let* ((script-file
                          (expand-file-name
                           "process-input.applescript"
                           temporary-file-directory))
                         (calls nil)
                         (sentinels nil)
                         (callbacks nil))
                    (with-temp-file script-file
                      (insert "return 41\n"))
                    (cl-letf (((symbol-function 'get-buffer-process)
                               (lambda (_buffer) nil))
                              ((symbol-function 'start-process)
                               (lambda (name buffer program &rest args)
                                 (push (list name program args) calls)
                                 (with-current-buffer buffer
                                   (erase-buffer)
                                   (insert
                                    (if (member "-e" args)
                                        "inline-result\n"
                                      "file-result\n")))
                                 (list :fake-process (length calls))))
                              ((symbol-function 'set-process-sentinel)
                               (lambda (process sentinel)
                                 (push (cons process sentinel) sentinels)))
                              ((symbol-function 'process-exit-status)
                               (lambda (_process) 0)))
                      (apples-do-applescript
                       script-file
                       (lambda (result status source)
                         (push
                          (list result status
                                (if (file-exists-p source)
                                    (file-name-nondirectory source)
                                  source))
                          callbacks)))
                      (funcall (cdar sentinels) (caar sentinels) "finished\n")
                      (setq sentinels nil)
                      (apples-do-applescript
                       "return \"Hello\""
                       (lambda (result status source)
                         (push (list result status source) callbacks)))
                      (funcall (cdar sentinels) (caar sentinels) "finished\n")
                      (list (nreverse calls) (nreverse callbacks))))"##;
    let expect = expect![[
        r#"OK ((("apples-do-applescript" "osascript" ("[ORACLE-TMPDIR]/process-input.applescript")) ("apples-do-applescript" "osascript" ("-ss" "-e" "return \"Hello\""))) (("file-result" 0 "process-input.applescript") ("inline-result" 0 "return \"Hello\"")))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn run_commands_select_file_buffer_region_and_minibuffer_payloads_with_precise_run_info() {
    let elisp_form = r##"(let* ((script-file
                          (expand-file-name
                           "run-command.applescript"
                           temporary-file-directory))
                         (calls nil))
                    (with-temp-file script-file
                      (insert "return 9\n"))
                    (cl-letf (((symbol-function 'apples-do-applescript)
                               (lambda (payload &optional _callback)
                                 (push
                                  (list
                                   (if (and (stringp payload)
                                            (file-exists-p payload))
                                       (list :file
                                             (file-name-nondirectory payload))
                                     (list :script payload))
                                   (car (apples-plist-get :run-info))
                                   (eq (cdr (apples-plist-get :run-info))
                                       (current-buffer)))
                                  calls))))
                      (with-temp-buffer
                        (insert
                         "set firstValue to 1\n"
                         "set secondValue to 2\n"
                         "return firstValue + secondValue\n")
                        (apples-run-file script-file)
                        (apples-run-buffer)
                        (apples-run-region 21 41)
                        (apples-run-minibuf "return 99"))
                      (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (((:file "run-command.applescript") nil nil) ((:script "set firstValue to 1\nset secondValue to 2\nreturn firstValue + secondValue\n") 1 t) ((:script "set secondValue to 2") 21 t) ((:script "return 99") nil nil))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn compile_creates_requested_parent_and_output_then_invokes_osacompile_with_exact_arguments() {
    let elisp_form = r##"(let* ((root
                          (expand-file-name
                           "apples-compile-contract"
                           temporary-file-directory))
                         (source (expand-file-name "source.applescript" root))
                         (output (expand-file-name "nested/output.scpt" root))
                         (apples-compile-create-file-flag t)
                         process-call sentinel messages)
                    (make-directory root t)
                    (with-temp-file source (insert "return 42\n"))
                    (cl-letf (((symbol-function 'start-process)
                               (lambda (name buffer program &rest args)
                                 (setq process-call
                                       (list name
                                             (buffer-name buffer)
                                             program
                                             (mapcar #'file-name-nondirectory args)))
                                 'fake-process))
                              ((symbol-function 'set-process-sentinel)
                               (lambda (_process function)
                                 (setq sentinel function)))
                              ((symbol-function 'process-exit-status)
                               (lambda (_process) 0))
                              ((symbol-function 'message)
                               (lambda (format-string &rest args)
                                 (let ((text (apply #'format format-string args)))
                                   (when (string-match "\\`Compiling" text)
                                     (push text messages))
                                   text))))
                      (apples-compile source output)
                      (funcall sentinel 'fake-process "finished\n")
                      (list
                       process-call
                       (file-exists-p output)
                       (file-directory-p (file-name-directory output))
                       (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (("apples-compile" " *apples-compile*" "osacompile" ("-o" "output.scpt" "source.applescript")) t t ("Compiling..." "Compiling...done"))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn decompile_process_passes_complete_script_to_callback_and_consumes_its_output_buffer() {
    let elisp_form = r##"(let* ((input
                          (expand-file-name
                           "compiled-script.scpt"
                           temporary-file-directory))
                         process-call sentinel callback-value)
                    (with-temp-file input (insert "compiled"))
                    (let ((apples-decompile-callback
                           (lambda (script filename)
                             (setq callback-value
                                   (list script
                                         (file-name-nondirectory filename))))))
                      (cl-letf (((symbol-function 'start-process)
                                 (lambda (name buffer program &rest args)
                                   (setq process-call
                                         (list name program
                                               (mapcar #'file-name-nondirectory args)))
                                   (with-current-buffer buffer
                                     (erase-buffer)
                                     (insert "set answer to 42\nreturn answer\n"))
                                   'fake-process))
                                ((symbol-function 'set-process-sentinel)
                                 (lambda (_process function)
                                   (setq sentinel function)))
                                ((symbol-function 'process-exit-status)
                                 (lambda (_process) 0))
                                ((symbol-function 'message)
                                 (lambda (&rest _args) "Decompiling...")))
                        (apples-decompile input)
                        (funcall sentinel 'fake-process "finished\n")
                        (list process-call callback-value
                              (with-current-buffer " *apples-decompile*"
                                (buffer-string))))))"##;
    let expect = expect![[
        r#"OK (("apples-decompile" "osadecompile" ("compiled-script.scpt")) ("set answer to 42\nreturn answer" "compiled-script.scpt") "")"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn decompile_handler_overwrites_files_inserts_at_point_and_copies_to_kill_ring() {
    let elisp_form = r##"(let* ((file
                          (expand-file-name
                           "decompile-target.applescript"
                           temporary-file-directory))
                         overwrite inserted copied)
                    (with-temp-file file (insert "old source\n"))
                    (let ((apples-decompile-query ?o))
                      (apples-handle-decompile "new source\nreturn 1\n" file))
                    (setq overwrite
                          (with-temp-buffer
                            (insert-file-contents file)
                            (buffer-string)))
                    (with-temp-buffer
                      (insert "prefix|suffix")
                      (goto-char 8)
                      (let ((apples-decompile-query ?i))
                        (apples-handle-decompile "INSERTED" file))
                      (setq inserted (buffer-string)))
                    (let ((kill-ring nil)
                          (apples-decompile-query ?c))
                      (apples-handle-decompile "copied script" file)
                      (setq copied (current-kill 0 t)))
                    (list overwrite inserted copied))"##;
    let expect =
        expect![[r#"OK ("new source\nreturn 1\n" "prefix|INSERTEDsuffix" "copied script")"#]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn send_to_editor_builds_a_complete_escaped_script_from_the_active_region() {
    let elisp_form = r##"(with-temp-buffer
                (insert
                 "set pathValue to \"C:\\\\Users\\\\Ada\"\n"
                 "display dialog \"Hello\"\n"
                 "return pathValue\n")
                (goto-char (point-min))
                (forward-line 1)
                (push-mark (point-max) t t)
                (setq mark-active t
                      transient-mark-mode t
                      apples-tmp-send
                      (expand-file-name
                       "editor-send.applescript"
                       temporary-file-directory))
                (let (sent)
                  (cl-letf (((symbol-function 'do-applescript)
                             (lambda (script) (setq sent script))))
                    (apples-send-to-applescript-editor)
                    sent)))"##;
    let expect = expect![[
        r#"OK "tell application \"AppleScript Editor\"\n    activate\n    open \"[ORACLE-TMPDIR]/editor-send.applescript\"\n    tell document \"editor-send.applescript\"\n        set contents to \"display dialog \\\"Hello\\\"\nreturn pathValue\n\"\n        execute\n    end tell\nend tell""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn dictionary_command_emits_the_exact_accessibility_keystroke_script() {
    let elisp_form = r##"(let (sent)
                (cl-letf (((symbol-function 'do-applescript)
                           (lambda (script) (setq sent script))))
                  (apples-open-dict-index)
                  sent))"##;
    let expect = expect![[
        r#"OK "tell application \"AppleScript Editor\" to activate\ntell application \"System Events\"\n    tell process \"AppleScript Editor\"\n        key code 31 using {shift down, command down}\n    end tell\nend tell""#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn run_buffer_surfaces_the_exact_missing_buffer_error_before_process_dispatch() {
    let elisp_form = r##"(apples-run-buffer " *missing-apples-buffer*")"##;
    let expect = expect![[r#"ERR (error "No buffer named  *missing-apples-buffer*")"#]];
    assert_apples_mode_signal_parity(elisp_form, expect);
}
