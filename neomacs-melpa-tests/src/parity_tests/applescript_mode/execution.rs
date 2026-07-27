use expect_test::expect;

use super::assert_applescript_mode_parity;

#[test]
fn applescript_mode_execute_code_encodes_and_escapes_input_then_decodes_real_boundary_output() {
    let elisp_form = r##"(let (received)
         (cl-letf
             (((symbol-function
                'do-applescript)
               (lambda (code)
                 (setq received code)
                 (as-encode-string
                  "completed 日本語"))))
           (let ((result
                  (as-execute-code
                   "set pathText to \"C:\\\\Temp\"\nreturn \"日本語\"")))
             (list
              result
              (string-to-list received)
              (string-bytes received)
              (as-decode-string
               (replace-regexp-in-string
                "\\\\\\\\"
                "\\\\"
                received
                t
                t))))))"##;
    let expect = expect![[
        r#"OK (#("completed 日本語" 10 13 (charset japanese-jisx0208)) (115 101 116 32 112 97 116 104 84 101 120 116 32 116 111 32 34 67 58 92 92 92 92 84 101 109 112 34 13 114 101 116 117 114 110 32 34 147 250 150 123 140 234 34) 49 "set pathText to \"C:\\\\\\\\Temp\"\nreturn \"ú{ê\"")"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_execute_region_sends_exact_source_appends_output_and_restores_window() {
    let elisp_form = r##"(save-window-excursion
         (let ((source
                (generate-new-buffer
                 "*applescript-source*"))
               (output
                (get-buffer-create
                 as-output-buffer))
               received)
           (unwind-protect
               (progn
                 (with-current-buffer output
                   (erase-buffer)
                   (insert
                    "Previous result:\n"))
                 (switch-to-buffer source)
                 (insert
                  "set ignored to 0\n"
                  "set chosen to 42\n"
                  "return chosen\n")
                 (goto-char
                  (point-min))
                 (forward-line 1)
                 (let ((start
                        (point)))
                   (goto-char
                    (point-max))
                   (let ((end
                          (point))
                         (window
                          (selected-window)))
                     (cl-letf
                         (((symbol-function
                            'as-execute-code)
                           (lambda (code)
                             (setq received code)
                             "42\n")))
                       (as-execute-region
                        start
                        end))
                     (list
                      received
                      (with-current-buffer output
                        (buffer-string))
                      (eq window
                          (selected-window))
                      (eq source
                          (current-buffer))
                      (point)))))
             (kill-buffer source)
             (kill-buffer output))))"##;
    let expect =
        expect![[r#"OK ("set chosen to 42\nreturn chosen\n" "Previous result:\n42\n" t t 49)"#]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_execute_buffer_forwards_exact_bounds_content_and_async_prefix() {
    let elisp_form = r##"(mapcar
         (lambda (async)
           (with-temp-buffer
             (insert
              "on run\n"
              "    return 42\n"
              "end run\n")
             (goto-char 12)
             (let (received)
               (cl-letf
                   (((symbol-function
                      'as-execute-region)
                     (lambda
                         (start end
                          &optional forwarded-async)
                       (setq received
                             (list
                              start
                              end
                              forwarded-async
                              (buffer-substring-no-properties
                               start
                               end)
                              (point))))))
                 (as-execute-buffer
                  async))
               received)))
         '(nil (4)))"##;
    let expect = expect![[
        r#"OK ((1 30 nil "on run\n    return 42\nend run\n" 12) (1 30 (4) "on run\n    return 42\nend run\n" 12))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_execute_string_uses_fresh_work_buffers_and_forwards_async_state() {
    let elisp_form = r##"(let ((as-output-buffer
                "*AppleScript Test Output*")
               calls)
         (unwind-protect
             (progn
               (get-buffer-create
                as-output-buffer)
               (cl-letf
                   (((symbol-function
                      'as-execute-region)
                     (lambda
                         (start end
                          &optional async)
                       (setq calls
                             (append
                              calls
                              (list
                               (list
                                (buffer-name)
                                start
                                end
                                async
                                (buffer-substring-no-properties
                                 start
                                 end))))))))
                 (as-execute-string
                  "return 1")
                 (as-execute-string
                  "display dialog \"two\""
                  '(4)))
               (list
                calls
                 (sort
                  (mapcar
                   #'buffer-name
                  (cl-remove-if-not
                   (lambda (buffer)
                     (string-match-p
                      "AppleScript Test Output"
                      (buffer-name buffer)))
                   (buffer-list)))
                 #'string<)))
           (applescript-test-kill-buffers
            "AppleScript Test Output")))"##;
    let expect = expect![[
        r#"OK ((("*AppleScript Test Output*<2>" 1 9 nil "return 1") ("*AppleScript Test Output*<3>" 1 21 (4) "display dialog \"two\"")) ("*AppleScript Test Output*" "*AppleScript Test Output*<2>" "*AppleScript Test Output*<3>"))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_repeated_region_execution_appends_each_real_result_in_order() {
    let elisp_form = r##"(save-window-excursion
         (let ((source
                (generate-new-buffer
                 "*applescript-repeat-source*"))
               (output
                (get-buffer-create
                 as-output-buffer))
               (results
                '("first\n"
                  "second\n")))
           (unwind-protect
               (progn
                 (with-current-buffer output
                   (erase-buffer))
                 (switch-to-buffer source)
                 (insert
                  "return 1\n"
                  "return 2\n")
                 (cl-letf
                     (((symbol-function
                        'as-execute-code)
                       (lambda (_code)
                         (prog1
                             (car results)
                           (setq results
                                 (cdr results))))))
                   (as-execute-region
                    (point-min)
                    10)
                   (as-execute-region
                    10
                    (point-max)))
                 (list
                  (with-current-buffer output
                    (buffer-string))
                  results
                  (eq source
                      (current-buffer))))
             (kill-buffer source)
             (kill-buffer output))))"##;
    let expect = expect![[r#"OK ("first\nsecond\n" nil t)"#]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_version_commands_emit_exact_user_messages_and_language_query() {
    let elisp_form = r##"(let (messages queries)
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest args)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         args)))
                   (setq messages
                         (append
                          messages
                          (list text)))
                   text)))
              ((symbol-function
                'as-execute-code)
               (lambda (code)
                 (setq queries
                       (append
                        queries
                        (list code)))
                 "2.8")))
           (list
            (as-mode-version)
            (as-language-version)
            messages
            queries)))"##;
    let expect = expect![[
        r#"OK (nil nil ("Using `applescript-mode' version $Revision$" "Using AppleScript version 2.8") ("AppleScript's version"))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_execute_code_propagates_boundary_errors_without_partial_decoding() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'do-applescript)
           (lambda (code)
             (signal
              'file-error
              (list
               "AppleScript execution failed"
               (string-bytes code))))))
         (condition-case error
             (as-execute-code
              "error number -128")
           (error
            (list
             (car error)
             (cadr error)
             (caddr error)))))"##;
    let expect = expect![[r#"OK (file-error "AppleScript execution failed" 17)"#]];

    assert_applescript_mode_parity(elisp_form, expect);
}
