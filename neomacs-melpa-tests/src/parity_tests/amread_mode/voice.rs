use expect_test::expect;

use super::assert_amread_mode_parity;

#[test]
fn language_selector_accepts_explicit_value_or_prompts_with_complete_choices() {
    let elisp_form = r##"(let ((amread-voice-reader-language
                           'chinese)
                          prompts)
                      (cl-letf
                          (((symbol-function 'completing-read)
                            (lambda
                                (prompt collection
                                 &rest arguments)
                              (push
                               (list
                                prompt collection
                                arguments)
                               prompts)
                              "english")))
                        (list
                         (amread--voice-reader-set-language
                          'japanese)
                         amread-voice-reader-language
                         (amread--voice-reader-set-language)
                         amread-voice-reader-language
                         (nreverse prompts))))"##;
    let expect = expect![[
        r#"OK (japanese japanese english english (("[amread] Select language: " ("chinese" "english") nil)))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn voice_reader_gate_skips_disabled_empty_text_and_dispatches_by_operating_system() {
    let elisp_form = r##"(let ((amread-voice-reader-enabled t)
                          (amread--voice-reader-proc-finished
                           'not-started)
                          events)
                      (cl-letf
                          (((symbol-function
                             'amread--voice-reader-read-text-with-say)
                            (lambda (text &rest arguments)
                              (push
                               (list 'say text arguments)
                               events)
                              'said))
                           ((symbol-function
                             'amread--voice-reader-read-text-with-tts)
                            (lambda (text)
                              (push
                               (list 'tts text)
                               events)
                              'spoken)))
                        (let ((system-type 'darwin))
                          (list
                           (amread--voice-reader-read-text nil)
                           (amread--voice-reader-read-text "")
                           (amread--voice-reader-read-text
                            "Hello reader")
                           amread--voice-reader-proc-finished))
                        (setq
                         amread--voice-reader-proc-finished
                         'not-started)
                        (let ((system-type 'gnu/linux))
                          (amread--voice-reader-read-text
                           "中文段落"))
                        (let ((amread-voice-reader-enabled
                               nil))
                          (amread--voice-reader-read-text
                           "disabled"))
                        (list
                         amread--voice-reader-proc-finished
                         (nreverse events))))"##;
    let expect = expect![[r#"OK (running ((say "Hello reader" nil) (tts "中文段落")))"#]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn python_command_helpers_execute_workspace_local_interpreter_with_multiline_code_and_file() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (bin
                           (file-name-as-directory
                            (expand-file-name
                             "python-bin" sandbox)))
                          (python
                           (expand-file-name
                            "python3" bin))
                          (code-log
                           (expand-file-name
                            "python-code-arg" sandbox))
                          (file-log
                           (expand-file-name
                            "python-file-arg" sandbox))
                          (code-file
                           (expand-file-name
                            "reader.py" sandbox))
                          (exec-path (list bin))
                          (process-environment
                           (copy-sequence
                            process-environment)))
                      (make-directory bin t)
                      (with-temp-file python
                        (insert
                         "#!/bin/sh\n"
                         "if [ \"$1\" = \"-c\" ]; then\n"
                         "  printf '%s' \"$2\" > \"$NEOMACS_TEST_SANDBOX_ROOT/python-code-arg\"\n"
                         "  printf 'CODE:%s\\n' \"$2\"\n"
                         "else\n"
                         "  printf '%s' \"$1\" > \"$NEOMACS_TEST_SANDBOX_ROOT/python-file-arg\"\n"
                         "  printf 'FILE:%s\\n' \"$1\"\n"
                         "fi\n"))
                      (set-file-modes python #o755)
                      (with-temp-file code-file
                        (insert "print('reader file')\n"))
                      (setenv "PATH" bin)
                      (let ((code-output
                             (amread--voice-reader-run-python-code-to-string
                              "value = \"a b\""
                              "print(value)"
                              "print(\"quoted\")"))
                            (file-output
                             (amread--voice-reader-run-python-file-to-string
                              code-file)))
                        (list
                         code-output
                         file-output
                         (with-temp-buffer
                           (insert-file-contents code-log)
                           (buffer-string))
                         (with-temp-buffer
                           (insert-file-contents file-log)
                           (file-relative-name
                            (buffer-string)
                            sandbox)))))"##;
    let expect = expect![[
        r#"OK ("CODE:value = \"a b\"\nprint(value)\nprint(\"quoted\")\n" "FILE:[ORACLE-SANDBOX]/reader.py\n" "value = \"a b\"\nprint(value)\nprint(\"quoted\")" "reader.py")"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn python_repl_helper_starts_once_sends_every_line_in_order_and_returns_last_result() {
    let elisp_form = r##"(let (events)
                      (cl-letf
                          (((symbol-function 'executable-find)
                            (lambda (program)
                              (push
                               (list 'find program)
                               events)
                              "/workspace/python3"))
                           ((symbol-function 'run-python)
                            (lambda (&rest arguments)
                              (push
                               (list 'run arguments)
                               events)
                              'python-process))
                           ((symbol-function
                             'python-shell-send-string-no-output)
                            (lambda (line)
                              (push
                               (list 'send line)
                               events)
                              (concat "reply:" line))))
                        (list
                         (amread--voice-reader-run-python-code-in-repl
                          "import sys"
                          "value = 42"
                          "value")
                         (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("reply:value" ((find "python3") (run nil) (send "import sys") (send "value = 42") (send "value")))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn tts_engine_initializes_once_and_speaks_multiple_practical_texts_through_same_repl() {
    let elisp_form = r##"(let ((amread--voice-reader-engine-initialized
                           nil)
                          events)
                      (cl-letf
                          (((symbol-function
                             'amread--voice-reader-run-python-code-in-repl)
                            (lambda (&rest lines)
                              (push lines events)
                              (if
                                  (member
                                   "True" lines)
                                  "True"
                                "spoken"))))
                        (list
                         (amread--voice-reader-read-text-with-tts
                          "First chapter")
                         amread--voice-reader-engine-initialized
                         (amread--voice-reader-read-text-with-tts
                          "第二章")
                         amread--voice-reader-engine-initialized
                         (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("spoken" "True" "spoken" "True" (("import pyttsx3" "engine = pyttsx3.init()" "True") ("engine.say(\"First chapter\")" "engine.runAndWait()") ("engine.say(\"第二章\")" "engine.runAndWait()")))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn macos_say_reader_selects_voice_resets_options_and_completes_async_process() {
    let elisp_form = r##"(let ((amread-voice-reader-command
                           "/workspace/bin/say")
                          (amread--voice-reader-proc-finished
                           'running)
                          prompts processes)
                      (cl-letf
                          (((symbol-function 'completing-read)
                            (lambda
                                (prompt collection
                                 &rest arguments)
                              (push
                               (list
                                prompt collection
                                arguments)
                               prompts)
                              "Otoya"))
                           ((symbol-function 'make-process)
                            (lambda (&rest properties)
                              (let ((sentinel
                                     (plist-get
                                      properties
                                      :sentinel)))
                                (push
                                 (list
                                  (plist-get
                                   properties :name)
                                  (plist-get
                                   properties :command)
                                  (plist-get
                                   properties :buffer)
                                  (plist-get
                                   properties :stderr))
                                 processes)
                                (funcall
                                 sentinel
                                 'fake-process
                                 "finished\n")
                                'fake-process))))
                        (list
                         (with-temp-buffer
                           (setq-local
                            amread-voice-reader-command-options
                            '("--stale=1"))
                           (list
                            (amread--voice-reader-read-text-with-say
                             "Hello world"
                             'english
                             "Samantha")
                            amread--voice-reader-voice
                            amread-voice-reader-command-options))
                         (with-temp-buffer
                           (list
                            (amread--voice-reader-read-text-with-say
                             "こんにちは"
                             'japanese)
                            amread--voice-reader-voice
                            amread-voice-reader-command-options))
                         amread--voice-reader-proc-finished
                         (nreverse prompts)
                         (nreverse processes))))"##;
    let expect = expect![[
        r#"OK ((fake-process "Samantha" ("--voice=Samantha" . #1=("--rate=200"))) (fake-process "Otoya" ("--voice=Otoya" . #1#)) finished (("[amread] Select voice: " ("Kyoko" "Otoya") nil)) (("amread-voice-reader" ("/workspace/bin/say" "--voice=Samantha" "--rate=200" "Hello world") " *amread-voice-reader*" " *amread-voice-reader*") ("amread-voice-reader" ("/workspace/bin/say" "--voice=Otoya" "--rate=200" "こんにちは") " *amread-voice-reader*" " *amread-voice-reader*")))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn language_detector_exercises_dynamic_english_chinese_fallback_and_missing_word_paths() {
    let elisp_form = r##"(let (responses calls)
                      (cl-letf
                          (((symbol-function
                             'pyim-probe-dynamic-english)
                            (lambda ()
                              (push '(probe) calls)
                              (pop responses)))
                           ((symbol-function 'word-at-point)
                            (lambda ()
                              (push '(word-at-point)
                                    calls)
                              nil)))
                        (list
                         (let ((responses '(nil)))
                           (amread--voice-reader-detect-language
                            "测试"))
                         (let ((responses '(t t)))
                           (amread--voice-reader-detect-language
                            "English"))
                         (let ((responses '(t nil)))
                           (amread--voice-reader-detect-language
                            "中文"))
                         (let ((responses '(t nil)))
                           (amread--voice-reader-detect-language
                            "Latin"))
                         (let ((responses '(t nil)))
                           (amread--voice-reader-detect-language))
                         (nreverse calls))))"##;
    let expect = expect![
        "OK (chinese chinese chinese chinese nil (#1=(probe) #1# #1# #1# (word-at-point)))"
    ];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn language_switch_uses_explicit_interactive_configured_and_detected_priority_order() {
    let elisp_form = r##"(let ((amread-voice-reader-language
                           'chinese)
                          events)
                      (cl-letf
                          (((symbol-function
                             'called-interactively-p)
                            (lambda (kind)
                              (push
                               (list 'interactive kind)
                               events)
                              t))
                           ((symbol-function
                             'amread--voice-reader-set-language)
                            (lambda (&optional language)
                              (push
                               (list 'select language)
                               events)
                              (setq
                               amread-voice-reader-language
                               'japanese)))
                           ((symbol-function
                             'amread--voice-reader-detect-language)
                            (lambda (&optional string)
                              (push
                               (list 'detect string)
                               events)
                              'english)))
                        (list
                         (amread-voice-reader-switch-language-voice
                          'korean)
                         (amread-voice-reader-switch-language-voice)
                         (let ((amread-voice-reader-language
                                nil))
                           (cl-letf
                               (((symbol-function
                                  'called-interactively-p)
                                 (lambda (_)
                                   nil)))
                             (amread-voice-reader-switch-language-voice)))
                         amread-voice-reader-language
                         (nreverse events))))"##;
    let expect = expect![
        "OK (korean japanese english japanese ((interactive interactive) (select nil) (detect nil)))"
    ];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn buffer_reader_walks_all_lines_and_obeys_voice_process_backpressure_states() {
    let elisp_form = r##"(let (events)
                      (cl-letf
                          (((symbol-function
                             'amread--voice-reader-read-text)
                            (lambda (text)
                              (push text events)
                              (setq
                               amread--voice-reader-proc-finished
                               'running)
                              'spoken)))
                        (list
                         (with-temp-buffer
                           (insert
                            "one\n"
                            "two\n"
                            "three\n")
                           (let ((amread--voice-reader-proc-finished
                                  'not-started)
                                 events)
                             (amread-voice-reader-read-buffer)
                             (list
                              (nreverse events)
                              amread--voice-reader-proc-finished
                              (point))))
                         (with-temp-buffer
                           (insert
                            "first\n"
                            "second\n")
                           (let ((amread--voice-reader-proc-finished
                                  'finished)
                                 events)
                             (amread-voice-reader-read-buffer)
                             (list
                              (nreverse events)
                              amread--voice-reader-proc-finished
                              (point))))
                         (with-temp-buffer
                           (insert
                            "skip\n"
                            "speak\n"
                            "blocked\n")
                           (let ((amread--voice-reader-proc-finished
                                  nil)
                                 events)
                             (amread-voice-reader-read-buffer)
                             (list
                              (nreverse events)
                              amread--voice-reader-proc-finished
                              (point)))))))"##;
    let expect = expect!["OK ((nil running 15) (nil running 14) (nil running 20))"];
    assert_amread_mode_parity(elisp_form, expect);
}
