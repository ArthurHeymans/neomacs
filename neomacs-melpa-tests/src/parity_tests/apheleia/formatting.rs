use expect_test::expect;

use super::assert_apheleia_parity;

#[test]
fn apheleia_real_process_formatter_uppercases_text_and_preserves_semantic_point() {
    let elisp_form = r##"(with-temp-buffer
         (rename-buffer
          "apheleia-uppercase.txt"
          t)
         (insert
          "first line\n"
          "mixed case target\n"
          "last line\n")
         (goto-char
          (point-min))
         (search-forward
          "target")
         (let ((apheleia-formatters
                '((upper
                   . ("tr"
                      "[:lower:]"
                      "[:upper:]")))))
           (let ((callback
                  (apheleia-test-format-buffer
                   'upper)))
             (list
              (buffer-string)
              (point)
              (line-number-at-pos)
              (current-column)
              callback
              (buffer-modified-p)))))"##;
    let expect =
        expect![[r#"OK ("FIRST LINE\nMIXED CASE TARGET\nLAST LINE\n" 29 2 17 (:error nil) t)"#]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_ports_upstream_word_replacement_workflow_and_keeps_point_on_the_same_word() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "The quick brown fox jumped over the lazy dog.")
         (goto-char
          (point-min))
         (search-forward
          "brown")
         (backward-char 3)
         (let ((apheleia-formatters
                '((study
                   . ("sed"
                      "-e"
                      "s/quick/slow/"
                      "-e"
                      "s/lazy/studious/")))))
           (list
            (apheleia-test-format-buffer
             'study)
            (buffer-string)
            (point)
            (current-word)
            (current-column))))"##;
    let expect = expect![[
        r#"OK ((:error nil) "The slow brown fox jumped over the studious dog." 12 "brown" 11)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_ports_upstream_line_reordering_workflow_without_moving_point_from_line_two() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "line one\n"
          "line two with cursor\n"
          "line three\n"
          "line four moves first\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (search-forward
          "cursor")
         (let ((apheleia-formatters
                '((move-fourth
                   . ("awk"
                      "{ lines[NR] = $0 } END { print lines[4]; for (i = 1; i <= 3; i++) print lines[i] }")))))
           (list
            (apheleia-test-format-buffer
             'move-fourth)
            (buffer-string)
            (line-number-at-pos)
            (current-column)
            (buffer-substring-no-properties
             (line-beginning-position)
             (line-end-position)))))"##;
    let expect = expect![[
        r#"OK ((:error nil) "line four moves first\nline one\nline two with cursor\nline three\n" 3 20 "line two with cursor")"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_ports_upstream_whitespace_insertion_alignment_case_at_an_expression() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha\n"
          "a=calculate(value)\n"
          "omega\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (search-forward
          "calculate")
         (let ((apheleia-formatters
                '((space-expression
                   . ("sed"
                      "s/^a=/    a = /")))))
           (list
            (apheleia-test-format-buffer
             'space-expression)
            (buffer-string)
            (line-number-at-pos)
            (current-column)
            (current-word))))"##;
    let expect = expect![[
        r#"OK ((:error nil) "alpha\n    a = calculate(value)\nomega\n" 2 17 "calculate")"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_chains_two_real_processes_in_order_and_emits_one_hook_event_per_formatter() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha beta\n"
          "beta gamma\n")
         (let ((apheleia-formatters
                '((uppercase
                   . ("tr"
                      "[:lower:]"
                      "[:upper:]"))
                  (rename
                   . ("sed"
                      "s/BETA/DELTA/g"))))
               (apheleia-formatter-exited-hook
                '((lambda (&rest properties)
                    (setq apheleia-test-hook-events
                          (append
                           apheleia-test-hook-events
                           (list
                            (list
                             (plist-get
                              properties
                              :formatter)
                             (plist-get
                              properties
                              :error)
                             (and
                              (plist-get
                               properties
                               :log)
                              t)))))))))
           (list
            (apheleia-test-format-buffer
             '(uppercase rename))
            (buffer-string)
            apheleia-test-hook-events)))"##;
    let expect = expect![[
        r#"OK ((:error nil) "ALPHA DELTA\nDELTA GAMMA\n" ((uppercase nil nil) (rename nil nil)))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_input_output_and_inplace_placeholders_drive_real_file_based_formatters() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (with-temp-buffer
             (rename-buffer
              (format
               "apheleia-%s.demo"
               (car spec))
              t)
             (insert
              "mixed Case\n"
              "second Line\n")
             (let ((apheleia-formatters
                    (list
                     (cons
                      (car spec)
                      (cadr spec)))))
               (list
                (car spec)
                (apheleia-test-format-buffer
                 (car spec))
                (buffer-string)))))
         '((input-file
            ("sh"
             "-c"
             "tr '[:lower:]' '[:upper:]' < \"$1\""
             "formatter"
             input))
           (output-file
            ("sh"
             "-c"
             "tr '[:lower:]' '[:upper:]' > \"$1\""
             "formatter"
             output))
           (inplace-file
            ("sh"
             "-c"
             "tr '[:lower:]' '[:upper:]' < \"$1\" > \"$1.next\" && mv \"$1.next\" \"$1\""
             "formatter"
             inplace))))"##;
    let expect = expect![[
        r#"OK ((input-file (:error nil) "MIXED CASE\nSECOND LINE\n") (output-file (:error nil) "MIXED CASE\nSECOND LINE\n") (inplace-file (:error nil) "MIXED CASE\nSECOND LINE\n"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_lisp_formatter_receives_real_context_and_can_transform_chained_scratch_text() {
    let elisp_form = r##"(progn
         (cl-defun apheleia-test-lisp-formatter
             (&key buffer scratch formatter
                   remote async callback
                   &allow-other-keys)
           (setq apheleia-test-hook-events
                 (list
                  (buffer-name buffer)
                  (buffer-name scratch)
                  formatter
                  remote
                  async
                  (with-current-buffer
                      scratch
                    (buffer-string))))
           (with-current-buffer scratch
             (goto-char
              (point-min))
             (while
                 (search-forward
                  "ALPHA"
                  nil
                  t)
               (replace-match
                "OMEGA"
                t
                t)))
           (funcall callback))
         (with-temp-buffer
           (rename-buffer
            "apheleia-lisp-original"
            t)
           (insert
            "alpha beta\n")
           (let ((apheleia-formatters
                  '((upper
                     . ("tr"
                        "[:lower:]"
                        "[:upper:]"))
                    (lisp-transform
                     . apheleia-test-lisp-formatter))))
             (list
              (apheleia-test-format-buffer
               '(upper lisp-transform))
              (buffer-string)
              apheleia-test-hook-events))))"##;
    let expect = expect![[
        r#"OK ((:error nil) "OMEGA BETA\n" ("apheleia-lisp-original" " *apheleia-apheleia-test-lisp-formatter-scratch*" lisp-transform nil t "ALPHA BETA\n"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_builtin_lisp_formatter_reindents_a_practical_function_without_losing_point() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(defun example (items)\n"
          "(mapcar (lambda (item)\n"
          "(when item\n"
          "(list :value item)))\n"
          "items))\n")
         (goto-char
          (point-min))
         (search-forward
          ":value")
         (let ((apheleia-formatters
                '((lisp-indent
                   . apheleia-indent-lisp-buffer))))
           (list
            (apheleia-test-format-buffer
             'lisp-indent)
            (buffer-string)
            (line-number-at-pos)
            (current-column)
            (current-word))))"##;
    let expect = expect![[
        r#"OK ((:error nil) "(defun example (items)\n  (mapcar (lambda (item)\n\11    (when item\n\11      (list :value item)))\n\11  items))\n" 4 26 ":value")"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_mode_formats_and_resaves_a_real_file_after_save() {
    let elisp_form = r##"(let* ((root
                  (make-temp-file
                   "apheleia-save-"
                   t))
                 (path
                  (expand-file-name
                   "project/source.txt"
                   root))
                 (buffer nil))
         (unwind-protect
             (progn
               (make-directory
                (file-name-directory path)
                t)
               (with-temp-file path
                 (insert
                  "first line\n"
                  "mixed case\n"))
               (setq buffer
                     (find-file-noselect path))
               (with-current-buffer buffer
                 (goto-char
                  (point-max))
                 (insert
                  "saved addition\n")
                 (save-buffer)
                 (let ((apheleia-formatters
                        '((upper
                           . ("tr"
                              "[:lower:]"
                              "[:upper:]"))))
                       (apheleia-formatter
                        'upper)
                       (apheleia-post-format-hook
                        '((lambda ()
                            (setq apheleia-test-hook-events
                                  (list
                                   (buffer-string)
                                   (point)
                                   (buffer-modified-p)))))))
                   (setq apheleia-test-hook-events
                         :not-called)
                   (apheleia-mode 1)
                   (run-hooks
                    'after-save-hook)
                   (let ((attempts 0))
                     (while
                         (and
                          (eq apheleia-test-hook-events
                              :not-called)
                          (< attempts 1000))
                       (setq attempts
                             (1+ attempts))
                       (accept-process-output
                        nil
                        0.01)))
                   (when
                       (eq apheleia-test-hook-events
                           :not-called)
                     (error
                      "post-format hook did not run"))
                   (list
                    apheleia-test-hook-events
                    (apheleia-test-read-file
                     path)
                    (buffer-string)
                    (buffer-modified-p)))))
           (when
               (buffer-live-p buffer)
             (with-current-buffer buffer
               (set-buffer-modified-p nil))
             (kill-buffer buffer))
           (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("FIRST LINE\nMIXED CASE\nSAVED ADDITION\n" 1 nil) "FIRST LINE\nMIXED CASE\nSAVED ADDITION\n" "FIRST LINE\nMIXED CASE\nSAVED ADDITION\n" nil)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_aborts_delayed_formatting_when_the_user_edits_the_buffer_in_flight() {
    let elisp_form = r##"(let ((formatted
                (generate-new-buffer
                 " *apheleia-delayed-output*"))
               deferred)
         (unwind-protect
             (with-temp-buffer
               (insert
                "original text\n")
               (let ((apheleia-formatters
                      '((delayed
                         . ("ignored")))))
                 (cl-letf
                     (((symbol-function
                        'apheleia--run-formatters)
                       (lambda
                           (formatters buffer remote callback
                            &optional stdin)
                         (ignore
                          formatters buffer remote stdin)
                         (setq deferred callback))))
                   (setq apheleia-test-callback-result
                         :not-called)
                   (apheleia-format-buffer
                    'delayed
                    nil
                    :callback
                    (lambda (&rest properties)
                      (setq apheleia-test-callback-result
                            properties)))
                   (goto-char
                    (point-max))
                   (insert
                    "user edit\n")
                   (with-current-buffer formatted
                     (insert
                      "FORMATTED TEXT\n"))
                   (funcall
                    deferred
                    nil
                    formatted)
                   (list
                    (apheleia-test-await-callback)
                    (buffer-string)
                    (buffer-modified-p)))))
           (kill-buffer formatted)))"##;
    let expect = expect![[
        r#"OK ((:error (error . "Contents have changed")) "original text\nuser edit\n" t)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_surfaces_unknown_and_missing_formatters_without_modifying_content() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert
            "untouched\n")
           (let ((apheleia-formatters nil))
             (condition-case error
                 (apheleia-test-format-buffer
                  'undefined)
               (error
                (list
                 (car error)
                 (cadr error)
                 (buffer-string))))))
         (with-temp-buffer
           (insert
            "also untouched\n")
           (let ((apheleia-formatters
                  '((missing
                     . ("apheleia-executable-that-does-not-exist"
                        "--format")))))
             (list
              (apheleia-test-format-buffer
               'missing)
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((user-error "No such formatter defined in ‘apheleia-formatters’: undefined" "untouched\n") ((:error (error . "Could not find executable for formatter missing, skipping")) "also untouched\n"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_context_evaluates_arguments_and_tracks_temporary_file_contracts() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (with-temp-buffer
             (rename-buffer
              "context.demo"
              t)
             (insert
              "input text\n")
             (let* ((ctx
                     (apheleia--formatter-context
                      (car spec)
                      (cadr spec)
                      nil))
                    (input
                     (apheleia-formatter--input-fname
                      ctx))
                    (output
                     (apheleia-formatter--output-fname
                      ctx))
                    (result
                     (list
                      (car spec)
                      (apheleia-formatter--arg1
                       ctx)
                      (mapcar
                       (lambda (arg)
                         (cond
                          ((and
                            input
                            (equal arg input))
                           :input-path)
                          ((and
                            output
                            (equal arg output))
                           :output-path)
                          (t arg)))
                       (apheleia-formatter--argv
                        ctx))
                      (and input
                           (file-exists-p input))
                      (and input
                           (file-name-extension
                            input
                            t))
                      (and output
                           (equal input output))
                      (and output
                           (file-exists-p output))
                      (and
                       (apheleia-formatter--stdin
                        ctx)
                       t))))
               (dolist (path
                        (delete-dups
                         (delq nil
                               (list input output))))
                 (when
                     (file-exists-p path)
                   (delete-file path)))
               result)))
         '((evaluated
            ("printf"
             "%s:%s"
             (upcase "word")
             (list "left" "right")))
           (input
            ("cat" input))
           (output
            ("sh" "-c" "true" output))
           (inplace
            ("formatter" inplace))))"##;
    let expect = expect![[
        r#"OK ((evaluated "printf" ("%s:%s" "WORD" "left" "right") nil nil nil nil t) (input "cat" (:input-path) t ".demo" nil nil nil) (output "sh" ("-c" "true" :output-path) nil nil nil t t) (inplace "formatter" (:input-path) t ".demo" t t nil))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}
