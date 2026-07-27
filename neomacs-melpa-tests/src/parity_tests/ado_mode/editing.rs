use expect_test::expect;

use super::assert_ado_mode_parity;

#[test]
fn ado_mode_extension_detection_covers_content_headers_and_filename_precedence() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq buffer-file-name (car case))
             (insert (cadr case))
             (list (ado-find-extension) (current-message))))
         '((nil "{smcl}\n{manlink R regress}\n")
           (nil "{smcl}\nplain output\n")
           (nil "class demo\n")
           (nil "DIALOG demo\n")
           (nil "program define demo\n")
           (nil "pro def demo\n")
           (nil "mata:\n")
           (nil "display 1\n")
           ("/work/forced.do" "program define demo\n")
           ("/work/UPPER.ADO" "display 1\n")
           (nil "*! header\nversion 18\ncapture program drop demo\nprogram demo\n")))"##;
    let expect = expect![[
        r#"OK (("sthlp" nil) ("smcl" nil) ("class" nil) ("dlg" nil) ("ado" nil) ("ado" nil) ("mata" nil) ("do" nil) ("do" nil) ("ado" nil) ("ado" nil))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_filename_inference_covers_program_class_help_and_fallback_formats() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq ado-extension (car case)
                   buffer-file-name (cadr case))
             (insert (caddr case))
             (condition-case error-data
                 (if (member (car case) '("hlp" "sthlp"))
                     (ado-make-help-name)
                   (ado-make-ado-name))
               (error (list 'signal (car error-data) (cdr error-data))))))
         '(("ado" "/work/original.ado" "program define alpha\n")
           ("ado" "/work/original.ado" "pr def _underscored9\n")
           ("ado" "/work/original.ado" "display 1\n")
           ("class" "/work/original.class" "class MyClass\n")
           ("do" "/work/original.do" "display 1\n")
           ("sthlp" "/work/original.sthlp" "{mansection R regress:regress}\n")
           ("sthlp" "/work/original.sthlp" "{manlink R summarize}\n")
           ("sthlp" "/work/original.sthlp" "{cmd:help tabulate oneway}\n")
           ("sthlp" "/work/original.sthlp" "{hi:mean command}\n")
           ("sthlp" "/work/original.sthlp" "unrecognizable\n")))"##;
    let expect = expect![[
        r#"OK ("alpha.ado" "original.ado" "original.ado" "MyClass.class" "original.do" "regress.sthlp" "summarize.sthlp" "tabulate_oneway.sthlp" "mean_command.sthlp" (signal error ("Could not figure out help file name!")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_help_name_start_and_local_program_name_cover_all_supported_layouts() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (text)
            (with-temp-buffer
              (insert text)
              (list (ado-find-help-name-start)
                    (condition-case error-data
                        (ado-find-help-name-start-pre12)
                      (error (list 'signal (car error-data)
                                   (cdr error-data)))))))
          '("{manlink R regress}\n"
            "{cmd:help summarize}\nTitle\n"
            "help for old\n{bf:oldcmd}\nTitle\n"
            "Title\ntext\nSyntax\n{cmd:newcmd}\n"
            "nothing\n"))
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (insert (car case))
              (goto-char (min (cadr case) (point-max)))
              (ado-find-local-name)))
          '(("program define outer\nbody\nprogram inner\nbody\n" 25)
            ("program define outer\nbody\nprogram inner\nbody\n" 45)
            ("display 1\n" 5)
            ("program\n" 8))))"##;
    let expect = expect![[
        r#"OK (((12 nil) (nil 11) (18 (signal search-failed ("Syntax"))) (nil nil) (nil nil)) ("outer" "inner" nil nil))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_timestamp_updates_cover_ado_help_smcl_generic_and_noop_paths() {
    let elisp_form = r##"(let (results)
         (cl-letf (((symbol-function 'ado-nice-current-date)
                    (lambda () "FIXED-DATE")))
           (dolist
               (case
                '(("ado" "*! version 1.2.3 old date\nprogram define x\n")
                  ("class" "*! version 4 old\nclass x\n")
                  ("do" "display 1\n")
                  ("sthlp" "{* Last Updated: old}{...}\n")
                  ("hlp" "{*** *! version 1.2 old}{...}\n")
                  ("sthlp" "{smcl}\n{* old date}{...}\n")
                  ("txt" "*! version 9.1 old\n")
                  ("txt" "Version 2.0 old\n")
                  ("txt" "plain\n")))
             (with-temp-buffer
               (setq ado-extension (car case))
               (insert (cadr case))
               (ado-update-timestamp)
               (push (buffer-string) results))))
         (nreverse results))"##;
    let expect = expect![[
        r#"OK ("*! version 1.2.3 FIXED-DATE\nprogram define x\n" "*! version 4 FIXED-DATE\nclass x\n" "display 1\n" "{* Last Updated: old}{...}\n" "{*** *! version 1.2 FIXED-DATE}{...}\n" "{smcl}\n{* FIXED-DATE}{...}\n" "*! version 9.1 FIXED-DATE\n" "Version 2.0 old\n" "plain\n")"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_skip_header_and_special_comment_scanners_stop_at_exact_boundaries() {
    let elisp_form = r##"(let ((print-circle nil))
         (prin1-to-string
          (list
         (mapcar
          (lambda (text)
            (condition-case nil
                (with-temp-buffer
                  (insert text)
                  (goto-char (point-min))
                  (ado-skip-special-comments)
                  (list (point)
                        (buffer-substring
                         (point) (line-end-position))))
              (error 'errored)))
          '("*! one\n*! two\n\nprogram x\n"
            "\n\ncommand\n"
            "* ordinary\ncommand\n"
            "command\n"))
         (mapcar
          (lambda (text)
            (condition-case nil
                (with-temp-buffer
                  (insert text)
                  (goto-char (point-min))
                  (ado-skip-header-lines)
                  (list (point)
                        (buffer-substring
                         (point) (line-end-position))))
              (error 'errored)))
          '("* heading\n*! metadata\nversion 18\ncapture program drop demo\nprogram demo\n"
            "vers 17\ncapture p drop demo\npro def demo\n"
            "\n\nmata:\n"
            "display 1\n")))))"##;
    let expect = expect![[
        r#"OK "(((16 \"program x\") (3 \"command\") (1 \"* ordinary\") (1 \"command\")) ((60 \"program demo\") (9 \"capture p drop demo\") (3 \"mata:\") (1 \"display 1\")))""#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_command_navigation_handles_cr_continuations_comments_and_semicolon_delimiters() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (needle)
            (with-temp-buffer
              (set-syntax-table ado-mode-syntax-table)
              (insert "display 1 /// keep going\n  + 2 // tail\nsummarize x\n")
              (goto-char (point-min))
              (search-forward needle)
              (let ((continued (ado-beginning-of-command))
                    (start (point)))
                (ado-end-of-command)
                (list needle continued start (point)
                      (buffer-substring-no-properties start (point))))))
          '("display" "+ 2" "summarize"))
         (with-temp-buffer
           (set-syntax-table ado-mode-syntax-table)
           (insert "#delimit ;\ndisplay \";inside\"; summarize x;")
           (goto-char (point-max))
           (search-backward "summarize")
           (let ((semi (ado-delimit-is-semi-p))
                 (continued (ado-beginning-of-command))
                 (start (point)))
             (ado-end-of-command)
             (list semi continued start (point)
                   (buffer-substring-no-properties start (point)))))
         (with-temp-buffer
           (insert "display 1")
           (goto-char (point-max))
           (list (ado-delimit-is-semi-p)
                 (condition-case error-data
                     (progn
                       (cl-letf (((symbol-function 'ado-delimit-is-semi-p)
                                  (lambda () t)))
                         (ado-end-of-command)))
                   (error (list 'signal (car error-data)
                                (cdr error-data)))))))"##;
    let expect = expect![[
        r#"OK ((("display" nil 1 31 "display 1 /// keep going\n  + 2") ("+ 2" t 1 31 "display 1 /// keep going\n  + 2") ("summarize" nil 40 51 "summarize x")) (t nil 31 42 "summarize x") (nil (signal error ("No end of command"))))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_copy_command_returns_or_saves_exact_region() {
    let elisp_form = r##"(with-temp-buffer
         (insert "display 1\nsummarize x\n")
         (goto-char (point-min))
         (let ((kill-ring nil))
           (list
            (ado-copy-command t)
            (progn
              (ado-copy-command nil)
              (car kill-ring))
            (point))))"##;
    let expect = expect![[r#"OK ("display 1" "display 1" 1)"#]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_macro_and_string_editors_cover_region_word_and_empty_point_paths() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "alpha beta")
           (goto-char 3)
           (ado-macify-selection-or-word)
           (list (buffer-string) (point)))
         (with-temp-buffer
           (insert "alpha beta")
           (goto-char (point-min))
           (set-mark (point))
           (search-forward "alpha")
           (setq mark-active t transient-mark-mode t)
           (ado-strmacify-selection-or-word)
           (list (buffer-string) (point)))
         (with-temp-buffer
           (insert " ")
           (goto-char (point-max))
           (ado-macify-selection-or-word)
           (list (buffer-string) (point)))
         (with-temp-buffer
           (insert "value")
           (goto-char (point-min))
           (set-mark (point))
           (goto-char (point-max))
           (setq mark-active t transient-mark-mode t)
           (ado-stringify-selection)
           (list (buffer-string) (point)))
         (with-temp-buffer
           (ado-stringify-selection)
           (list (buffer-string) (point))))"##;
    let expect = expect![[
        r#"OK (("`alpha' beta" 8) ("`\"`alpha'\"' beta" 12) (" `'" 3) ("`\"value\"'" 10) ("`\"\"'" 3))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_split_line_and_continuation_alignment_cover_modern_and_legacy_modes() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq ado-smart-indent-flag nil
                   ado-use-modern-split-flag (nth 0 case)
                   ado-line-up-continuations (nth 1 case)
                   comment-column 12)
             (insert "display alpha beta")
             (goto-char (+ (point-min) 8))
             (ado-split-line)
             (list (buffer-string) (point))))
         '((t nil) (nil nil) (t t) (nil t)))"##;
    let expect = expect![[
        r#"OK (("display  ///\nalpha beta" 14) ("display /*\n*/alpha beta" 14) ("display     /// \nalpha beta" 17) ("display     /*\n*/alpha beta" 18))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_depth_and_indentation_cover_blocks_continuations_and_special_columns() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (text)
            (with-temp-buffer
              (set-syntax-table ado-mode-syntax-table)
              (insert text)
              (goto-char (point-max))
              (ado-find-depth)))
          '("program define x\n"
            "program define x\nif x {\n"
            "program define x\nend\n"
            "display 1 ///\n  + 2"
            "mata:\nfunction x() {\n"))
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (set-syntax-table ado-mode-syntax-table)
              (setq ado-smart-indent-flag t
                    tab-width 3
                    ado-continued-statement-indent-spaces 2
                    ado-delimit-indent-flag t
                    ado-delimit-indent-column 1
                    ado-comment-indent-flag t
                    ado-comment-indent-column 2
                    ado-debugging-indent-flag t
                    ado-debugging-indent-column 4
                    ado-close-under-line-flag t)
              (insert (car case))
              (goto-char (point-max))
              (forward-line -1)
              (list (ado-indent-line) (buffer-string) (point))))
          '(("program define x\nbody\n")
            ("   #delimit ;\n")
            ("   * comment\n")
            ("   pause\n")
            ("display 1 ///\nnext\n"))))"##;
    let expect = expect![[
        r#"OK (((1 nil) (2 nil) (0 nil) (0 t) (2 nil)) ((3 "program define x\n\11body\n" 19) (-2 " #delimit ;\n" 2) (-3 "* comment\n" 1) (1 "\11 pause\n" 3) (2 "display 1 ///\n  next\n" 17)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_clean_buffer_comment_indent_and_nested_comment_helpers_match() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'current-time-string)
                    (lambda () "FIXED-TIME"))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments) messages))))
           (list
            (with-temp-buffer
              (insert "one\n   \n\t\ntwo\n")
              (ado-clean-buffer)
              (buffer-string))
            (mapcar
             (lambda (case)
               (with-temp-buffer
                 (setq comment-column (car case))
                 (insert (cadr case))
                 (goto-char (point-max))
                 (ado-comment-indent)))
             '((40 "code /*") (0 "   /*") (40 "} /*") (40 "#endif /*")))
            (mapcar
             (lambda (text)
               (with-temp-buffer
                 (set-syntax-table ado-mode-syntax-table)
                 (insert text)
                 (goto-char (point-max))
                 (list (ado-line-starts-with-end-comment)
                       (progn
                         (ado-start-of-nested-comment t)
                         (point)))))
             '("/* outer /* inner */ tail */"
               "code */"
               "plain"))
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("one\n\n\ntwo\n" (40 6 3 7) ((nil 1) (nil 6) (nil 6)) ("Ended ado-clean-buffer: FIXED-TIME"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_error_navigation_and_brace_balancing_cover_forward_backward_and_missing_cases() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "command\n{err} bad\nr(198)\n")
           (goto-char (point-min))
           (let ((forward-one (progn (ado-next-error) (point)))
                 (forward-two (progn (ado-next-error) (point)))
                 (backward (progn (ado-prev-error) (point))))
             (list forward-one forward-two backward)))
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (set-syntax-table ado-mode-syntax-table)
              (insert (car case))
              (goto-char (cadr case))
              (setq transient-mark-mode t)
              (let ((result (ado-balance-brace (caddr case))))
                (list result (point) (mark t) (current-message)))))
          '(("foreach x in a b {\n display x\n}\n" 29 t)
            ("display (1 + 2)\n" 12 nil)
            ("plain text\n" 5 nil))))"##;
    let expect = expect![[
        r#"OK ((9 9 9) ((nil 1 32 nil) (nil 9 16 nil) ("Not inside braces" 5 nil nil)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_before_save_extension_show_and_file_write_adapters_cover_rename_and_cancel_paths() {
    let elisp_form = r##"(let ((ado-update-timestamp-flag t)
               (ado-confirm-overwrite-flag t)
               events messages)
         (cl-letf (((symbol-function 'ado-make-ado-name)
                    (lambda () "derived.ado"))
                   ((symbol-function 'ado-find-extension)
                    (lambda () "ado"))
                   ((symbol-function 'ado-update-timestamp)
                    (lambda () (push 'timestamp events)))
                   ((symbol-function 'set-visited-file-name)
                    (lambda (filename &rest _arguments)
                      (setq buffer-file-name
                            (expand-file-name filename default-directory))
                      (push (list 'visited filename) events)))
                   ((symbol-function 'file-exists-p)
                    (lambda (&rest _arguments) t))
                   ((symbol-function 'y-or-n-p)
                    (lambda (&rest _arguments) ado-test-confirm))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages)))
                   ((symbol-function 'save-buffer)
                    (lambda (&rest arguments)
                      (push (cons 'save arguments) events)))
                   ((symbol-function 'write-file)
                    (lambda (&rest arguments)
                      (push (cons 'write arguments) events))))
           (let (accepted canceled starred plain)
             (with-temp-buffer
               (setq buffer-file-name
                     (expand-file-name "old.ado" default-directory)
                     ado-test-confirm t)
               (ado-before-save-file)
               (setq accepted
                     (list ado-extension buffer-file-name)))
             (with-temp-buffer
               (setq buffer-file-name
                     (expand-file-name "old.ado" default-directory)
                     ado-test-confirm nil)
               (setq canceled
                     (condition-case error-data
                         (ado-before-save-file)
                       (error
                        (list 'signal (car error-data)
                              (cdr error-data))))))
             (with-temp-buffer
               (rename-buffer "*temporary*")
               (setq starred (ado-write-file-as-buffer-name)))
             (with-temp-buffer
               (rename-buffer "named<2>")
               (setq plain (ado-write-file-as-buffer-name)))
             (list accepted canceled starred plain
                   (nreverse events) (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (("ado" "[ORACLE-SANDBOX]/derived.ado") (signal error ("Canceled")) #2=((save) . #1=((write "named" t))) #1# (timestamp (visited "derived.ado") timestamp (visited "derived.ado") . #2#) nil)"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_electric_editors_cover_braces_newline_semicolon_and_dispatch_flags() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (case)
            (with-temp-buffer
              (setq ado-auto-newline-flag (nth 0 case)
                    ado-closing-brace-alone-flag (nth 1 case)
                    ado-smart-indent-flag nil
                    last-command-event (nth 2 case))
              (insert (nth 3 case))
              (goto-char (point-max))
              (if (eq (nth 2 case) ?})
                  (ado-electric-closing-brace nil)
                (ado-electric-brace nil))
              (list (buffer-string) (point))))
          '((nil nil ?{ "")
            (t nil ?{ "if x ")
            (nil t ?} "body")
            (t t ?} "body")))
         (with-temp-buffer
           (setq ado-smart-indent-flag nil)
           (insert "line")
           (ado-newline)
           (list (buffer-string) (point)))
         (mapcar
          (lambda (semi)
            (with-temp-buffer
              (setq last-command-event ?\;)
              (cl-letf (((symbol-function 'ado-delimit-is-semi-p)
                         (lambda () semi))
                        ((symbol-function 'ado-electric-brace)
                         (lambda (repeat)
                           (list 'electric repeat))))
                (list (ado-electric-semi 3)
                      (buffer-string)))))
          '(nil t)))"##;
    let expect = expect![[
        r#"OK ((("{" 2) ("if x {\n" 8) ("body\n}" 7) ("body\n}\n" 8)) ("line\n" 6) ((nil ";;;") ((electric 3) "")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_boilerplate_signature_show_and_block_adapters_forward_exactly() {
    let elisp_form = r##"(let ((ado-site-template-dir "/virtual/templates")
               (ado-signature-file "/old/signature")
               messages events)
         (cl-letf (((symbol-function 'insert-file-contents)
                    (lambda (path &rest arguments)
                      (push (list 'insert path arguments) events)
                      (insert (concat "<" path ">"))))
                   ((symbol-function 'ado-insert-file-and-indent)
                    (lambda (path)
                      (push (list 'indented path) events)
                      (insert (concat "[" path "]"))))
                   ((symbol-function 'read-file-name)
                    (lambda (&rest arguments)
                      (push (cons 'read arguments) events)
                      "/chosen/signature"))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages)))
                   ((symbol-function 'ado-find-extension)
                    (lambda () "sthlp"))
                   ((symbol-function 'ado-make-ado-name)
                    (lambda () "derived.sthlp"))
                   ((symbol-function 'ado-find-local-name)
                    (lambda () "local-program"))
                   ((symbol-function 'ado-find-depth)
                    (lambda () '(3 t)))
                   ((symbol-function 'ado-delimit-is-semi-p)
                    (lambda () t))
                   ((symbol-function 'ado-balance-brace)
                    (lambda (&rest arguments)
                      (push (cons 'balance arguments) events)))
                   ((symbol-function 'ado-send-command-to-stata)
                    (lambda (&rest arguments)
                      (push (cons 'send arguments) events))))
           (list
            (with-temp-buffer
              (ado-insert-boilerplate "plain.blp" nil nil)
              (buffer-string))
            (with-temp-buffer
              (ado-insert-boilerplate "/raw.blp" t t)
              (buffer-string))
            (ado-set-ado-signature-file)
            ado-signature-file
            (ado-set-ado-extension)
            ado-extension
            (ado-sho<w-extension)
            (ado-show-ado-name)
            (ado-show-local-name)
            (ado-show-depth)
            (ado-show-delimiter)
            (ado-grab-block)
            (ado-send-block-to-stata)
            (nreverse events)
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK ("[/virtual/templates/plain.blp]" "</raw.blp>" "/chosen/signature" "/chosen/signature" "sthlp" "sthlp" #7=("I think the extension is `sthlp'" . #1=("Suggested name: `derived.sthlp'" . #2=("The local program is `local-program'" . #3=("The depth is 3  with continuation" . #4=("The delimiter is ;"))))) #1# #2# #3# #4# #6=((balance t) (balance t) . #5=((send))) #5# ((indented "/virtual/templates/plain.blp") (insert "/raw.blp" nil) (read "Set ado signature file to: " "/old/") . #6#) #7#)"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_directory_listing_helper_covers_all_self_sub_and_default_modes() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'directory-files)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      '("/root/a" "/root/b"))))
           (list
            (ado-find-ado-dirs "/root" nil)
            (ado-find-ado-dirs "/root" "all")
            (ado-find-ado-dirs "/root" "self")
            (ado-find-ado-dirs "/root" "sub")
            (ado-find-ado-dirs "/root" "invalid")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("/root" "/root/a" "/root/b") ("/root" "/root/a" "/root/b") ("/root") ("/root/a" "/root/b") nil (("/root" t "^[a-z_0-9]$") ("/root" t "^[a-z_0-9]$") ("/root" t "^[a-z_0-9]$")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_continuation_region_and_file_indent_helpers_execute_exact_contracts() {
    let elisp_form = r##"(let (region-calls file-events)
         (list
          (mapcar
           (lambda (case)
             (with-temp-buffer
               (setq ado-use-modern-split-flag (car case)
                     comment-column 12)
               (insert (cadr case))
               (goto-char (point-max))
               (ado-continuation-indent)
               (list (buffer-string) (point))))
           '((t "display 1")
             (t "display 1 ///")
             (nil "display 1")
             (nil "display 1 /*")
             (nil "display 1 /* closed */")))
          (with-temp-buffer
            (insert "first\n\n  second\nthird\n")
            (cl-letf (((symbol-function 'ado-indent-line)
                       (lambda ()
                         (push (list (line-number-at-pos) (point))
                               region-calls))))
              (ado-indent-region 2 (point-max))
              (list (nreverse region-calls)
                    (buffer-string)
                    (point))))
          (with-temp-buffer
            (cl-letf (((symbol-function 'ado-indent-region)
                       (lambda (&rest arguments)
                         (push arguments region-calls))))
              (insert "whole buffer\n")
              (goto-char 4)
              (ado-indent-buffer)
              (list (car region-calls) (point))))
          (with-temp-buffer
            (cl-letf (((symbol-function 'insert-file-contents)
                       (lambda (file &rest arguments)
                         (push (list 'insert file arguments)
                               file-events)
                         (insert "alpha\nbeta\n")
                         (list file 11)))
                      ((symbol-function 'ado-indent-region)
                       (lambda (&rest arguments)
                         (push (cons 'indent arguments)
                               file-events))))
              (insert "prefix:")
              (ado-insert-file-and-indent "./tmp/virtual-template")
              (list (buffer-string) (point)
                    (nreverse file-events))))
          (with-temp-buffer
            (setq-local indent-line-function
                        (lambda () (indent-to 3)))
            (ado-insert-with-lfd "generated")
            (list (buffer-string) (point)))))"##;
    let expect = expect![[
        r#"OK ((("display 1   ///" 16) ("display 1   ///" 16) ("display 1   /*" 15) ("display 1   /*" 15) ("display 1 /* closed */ \11/*" 27)) (((1 1) (3 10) (4 17)) "first\n\n  second\nthird\n" 23) ((1 14) 4) ("prefix:alpha\nbeta\n" 19 ((insert "./tmp/virtual-template" nil) (indent 8 19))) ("generated\n   " 14))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}
