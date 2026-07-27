use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn stack_trace_preparation_links_existing_java_source_with_exact_navigation_properties() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (file-name-as-directory
                             (expand-file-name
                              "stack-project"
                              sandbox)))
                           (source
                            (expand-file-name
                             "src/com/example/MainActivity.java"
                             root))
                           (message
                            "boom at com.example.MainActivity.run(MainActivity.java:42)")
                           (missing
                            "boom at com.missing.Other.run(Other.java:9)"))
                      (make-directory
                       (file-name-directory source)
                       t)
                      (with-temp-file source
                        (insert "class MainActivity {}\n"))
                      (cl-letf
                          (((symbol-function 'android-root)
                            (lambda () root)))
                        (let ((linked
                               (android-logcat-prepare-msg
                                message))
                              (unlinked
                               (android-logcat-prepare-msg
                                missing))
                              (plain
                               (android-logcat-prepare-msg
                                "plain log message")))
                          (list
                           (substring-no-properties
                            linked)
                           (mapcar
                            (lambda (property)
                              (get-text-property
                               1 property linked))
                            '(face
                              mouse-face
                              filename
                              linenr
                              follow-link))
                           (list
                            unlinked
                            (text-properties-at
                             1 unlinked))
                           (list
                            plain
                            (text-properties-at
                             1 plain))))))"##;
    let expect = expect![[
        r#"OK ("boom at com.example.MainActivity.run(MainActivity.java:42)" (underline highlight "com/example/MainActivity.java" 42 t) ("boom at com.missing.Other.run(Other.java:9)" nil) ("plain log message" nil))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn streaming_logcat_filter_handles_crlf_partial_chunks_levels_filtering_and_source_links() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (root
                            (file-name-as-directory
                             (expand-file-name
                              "stream-project"
                              sandbox)))
                           (source
                            (expand-file-name
                             "src/com/example/MainActivity.java"
                             root))
                           (android-logcat-buffer
                            "*android-stream-log*")
                           (android-mode-log-filter-regexp
                            "Keep")
                           (android-logcat-pending-output
                            ""))
                      (make-directory
                       (file-name-directory source)
                       t)
                      (with-temp-file source
                        (insert "class MainActivity {}\n"))
                      (when
                          (get-buffer
                           android-logcat-buffer)
                        (kill-buffer
                         android-logcat-buffer))
                      (get-buffer-create
                       android-logcat-buffer)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'android-root)
                                (lambda () root)))
                            (with-current-buffer
                                android-logcat-buffer
                              (setq tab-stop-list
                                    '(2 24))
                              (goto-char (point-max)))
                            (android-logcat-process-filter
                             nil
                             "I/KeepTag( 123): first at com.example.MainActivity.run(MainActivity.java:7)\r\nD/DropTag( 9): hidden\nW/KeepTag( 77): partial")
                            (let ((after-first
                                   (with-current-buffer
                                       android-logcat-buffer
                                     (list
                                      (buffer-string)
                                      android-logcat-pending-output
                                      (point)
                                      (point-max)))))
                              (android-logcat-process-filter
                               nil
                               " message\nunstructured Keep line\n")
                              (with-current-buffer
                                  android-logcat-buffer
                                (goto-char (point-min))
                                (search-forward
                                 "first at")
                                (let ((linked-position
                                       (point)))
                                  (list
                                   after-first
                                   (buffer-string)
                                   android-logcat-pending-output
                                   (point)
                                   (point-max)
                                   (get-text-property
                                    linked-position
                                    'filename)
                                   (get-text-property
                                    linked-position
                                    'linenr)
                                   (get-text-property
                                    (point-min)
                                    'font-lock-face)
                                   (save-excursion
                                     (goto-char
                                      (point-min))
                                     (forward-line 2)
                                     (get-text-property
                                      (point)
                                      'font-lock-face)))))))
                        (kill-buffer
                         android-logcat-buffer)))"##;
    let expect = expect![[
        r#"OK ((#("I KeepTag(123)\11\11first at com.example.MainActivity.run(MainActivity.java:7)\n" 0 2 (font-lock-face android-mode-info-face) 2 9 (font-lock-face font-lock-function-name-face) 9 16 (font-lock-face font-lock-constant-face) 16 74 (face underline mouse-face highlight filename #1=#("com/example/MainActivity.java" 0 3 (font-lock-face android-mode-info-face) 4 11 (font-lock-face android-mode-info-face) 12 29 (font-lock-face android-mode-info-face)) linenr 7 follow-link t font-lock-face android-mode-info-face)) "W/KeepTag( 77): partial" 76 76) #("I KeepTag(123)\11\11first at com.example.MainActivity.run(MainActivity.java:7)\nW KeepTag(77)\11\11partial message\nunstructured Keep line\n" 0 2 (font-lock-face android-mode-info-face) 2 9 (font-lock-face font-lock-function-name-face) 9 16 (font-lock-face font-lock-constant-face) 16 74 (face underline mouse-face highlight filename #1# linenr 7 follow-link t font-lock-face android-mode-info-face) 75 77 (font-lock-face android-mode-warning-face) 77 84 (font-lock-face font-lock-function-name-face) 84 90 (font-lock-face font-lock-constant-face) 90 105 (font-lock-face android-mode-warning-face) 106 128 (font-lock-face font-lock-warning-face)) "" 25 130 #("com/example/MainActivity.java" 0 3 (font-lock-face android-mode-info-face) 4 11 (font-lock-face android-mode-info-face) 12 29 (font-lock-face android-mode-info-face)) 7 android-mode-info-face font-lock-warning-face)"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn logcat_filter_messages_clear_and_erase_work_in_read_only_live_buffer() {
    let elisp_form = r##"(let ((android-logcat-buffer
                           "*android-filter-log*")
                          (android-mode-log-filter-regexp
                           "initial"))
                      (when
                          (get-buffer
                           android-logcat-buffer)
                        (kill-buffer
                         android-logcat-buffer))
                      (with-current-buffer
                          (get-buffer-create
                           android-logcat-buffer)
                        (insert "existing line\n")
                        (setq buffer-read-only t))
                      (unwind-protect
                          (progn
                            (android-logcat-set-filter
                             "Error|Warning")
                            (let ((changed
                                   (with-current-buffer
                                       android-logcat-buffer
                                     (list
                                      (buffer-string)
                                      buffer-read-only
                                      (get-text-property
                                       (- (point-max) 2)
                                       'font-lock-face))))
                                  (changed-filter
                                   android-mode-log-filter-regexp))
                              (android-logcat-clear-filter)
                              (let ((cleared
                                     (with-current-buffer
                                         android-logcat-buffer
                                       (buffer-string)))
                                    (cleared-filter
                                     android-mode-log-filter-regexp))
                                (android-logcat-erase-buffer)
                                (list
                                 changed
                                 changed-filter
                                 cleared
                                 cleared-filter
                                 (with-current-buffer
                                     android-logcat-buffer
                                   (list
                                    (buffer-string)
                                    buffer-read-only))))))
                        (kill-buffer
                         android-logcat-buffer)))"##;
    let expect = expect![[
        r#"OK ((#("existing line\n\n\n*** Filter is changed to 'Error|Warning' ***\n\n" 14 62 (font-lock-face android-mode-info-face)) t android-mode-info-face) "Error|Warning" #("existing line\n\n\n*** Filter is changed to 'Error|Warning' ***\n\n\n\n*** Filter is cleared ***\n\n" 14 62 (font-lock-face android-mode-info-face) 62 91 (font-lock-face android-mode-info-face)) "" ("" t))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}
