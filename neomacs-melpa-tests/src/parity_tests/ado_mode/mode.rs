use expect_test::expect;

use super::assert_ado_mode_parity;

#[test]
fn ado_mode_initialization_sets_local_editor_contract_for_supported_extensions() {
    let elisp_form = r##"(mapcar
         (lambda (filename)
           (with-temp-buffer
             (setq buffer-file-name filename
                   ado-add-sysdir-font-lock nil
                   ado-mode-home "/virtual/ado-mode/"
                   ado-site-template-dir "/virtual/ado-mode/templates/"
                   ado-script-dir "/virtual/ado-mode/scripts/"
                   ado-new-dir "/virtual/new/"
                   ado-personal-dir "/virtual/personal/")
             (insert (cond
                      ((string-suffix-p ".ado" filename)
                       "program define sample\nend\n")
                      ((string-suffix-p ".sthlp" filename)
                       "{smcl}\n{manlink R regress}\n")
                      (t "display 1\n")))
             (ado-mode)
             (list major-mode mode-name ado-extension
                   (eq (current-local-map) ado-mode-map)
                   (eq local-abbrev-table ado-mode-abbrev-table)
                   (eq (syntax-table) ado-mode-syntax-table)
                   (eq indent-line-function #'ado-indent-line)
                   comment-start comment-end comment-column
                   comment-start-skip comment-multi-line
                   parse-sexp-ignore-comments require-final-newline
                   delete-auto-save-files
                   (memq 'ado-before-save-file before-save-hook)
                   font-lock-defaults
                   ado-smart-indent-flag
                   (local-variable-p 'ado-extension)
                   (local-variable-p 'ado-submit-default))))
         '("/virtual/sample.ado"
           "/virtual/sample.do"
           "/virtual/sample.sthlp"))"##;
    let expect = expect![[
        r#"OK ((ado-mode "Ado" "ado" t t t t "//" "" 40 "/\\*+ *" nil t t t (ado-before-save-file) #1=(ado-font-lock-keywords) t t t) (ado-mode "Ado" "do" t t t t "//" "" 40 "/\\*+ *" nil t t t (ado-before-save-file) #1# t t t) (ado-mode "Ado" "sthlp" t t t t "//" "" 40 "/\\*+ *" nil t t t (ado-before-save-file) #1# nil t t))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_return_binding_toggle_and_generic_flag_toggle_match() {
    let elisp_form = r##"(let ((ado-mode-map (make-sparse-keymap))
               (ado-return-also-indents-flag nil))
         (setq ado-test-flag nil)
         (ado-set-return nil)
         (let ((initial
                (list (lookup-key ado-mode-map "\C-m")
                      (lookup-key ado-mode-map "\C-j"))))
           (ado-return-toggle)
           (let ((toggled
                  (list ado-return-also-indents-flag
                        (lookup-key ado-mode-map "\C-m")
                        (lookup-key ado-mode-map "\C-j"))))
             (ado-toggle-flag 'ado-test-flag)
             (let ((first ado-test-flag))
               (ado-toggle-flag 'ado-test-flag)
               (list initial toggled first ado-test-flag)))))"##;
    let expect = expect!["OK ((newline ado-newline) (t ado-newline newline) t nil)"];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_number_changers_cover_direct_prompt_unchanged_and_reindent_branches() {
    let elisp_form = r##"(let ((ado-test-number 3)
               (tab-width 4)
               (ado-continued-statement-indent-spaces 2)
               responses messages reindents)
         (cl-letf (((symbol-function 'read-from-minibuffer)
                    (lambda (&rest _arguments) (pop responses)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments) messages)))
                   ((symbol-function 'y-or-n-p)
                    (lambda (&rest _arguments) t))
                   ((symbol-function 'ado-indent-buffer)
                    (lambda () (push 'indent reindents))))
           (let ((direct (ado-change-number 'ado-test-number 8)))
             (setq responses '("" "8" "11"))
             (let ((empty (ado-change-number 'ado-test-number 'ask))
                   (same (ado-change-number 'ado-test-number nil))
                   (changed (ado-change-number 'ado-test-number 'ask)))
               (ado-tab-width-change 6)
               (ado-continued-statement-indent-spaces-change 5)
               (list direct empty same changed
                     ado-test-number tab-width
                     ado-continued-statement-indent-spaces
                     (nreverse messages)
                     (nreverse reindents))))))"##;
    let expect = expect![[
        r#"OK (t nil nil t 3 6 5 ("Value of `ado-test-number' left unchanged." "Value of `ado-test-number' left unchanged." "Value of `ado-test-number' set to 11.") (indent indent))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_loop_insertion_helpers_emit_exact_stata_syntax_and_point() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (setq ado-smart-indent-flag nil)
           (ado-foreach-loop "item" "")
           (list (buffer-string) (point)))
         (with-temp-buffer
           (setq ado-smart-indent-flag nil)
           (ado-foreach-loop "variable" "varlist")
           (list (buffer-string) (point)))
         (with-temp-buffer
           (setq ado-smart-indent-flag nil)
           (ado-forvalues-loop "index" "1/4")
           (list (buffer-string) (point)))
         (condition-case error-data
             (ado-parse-loop)
           (error (list 'signal (car error-data) (cdr error-data)))))"##;
    let expect = expect![[
        r#"OK (("foreach item in \"\" {\n\n}" 18) ("foreach variable of varlist   {\n\n}" 29) ("forvalues index = 1/4 {\n\n}" 25) (signal error ("This is out of date! Use a foreach loop (\\[ado-foreach-loop]), instead")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_file_creator_wrappers_forward_all_arguments_exactly() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ado-new-generic)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      arguments)))
           (list
            (ado-new-do t "one" "purpose one")
            (ado-new-mata nil "two" "purpose two")
            (ado-new-class t "three" nil)
            (ado-new-ado nil "four" "purpose four")
            (ado-new-program t "five" "purpose five")
            (ado-new-testado nil "six" "purpose six")
            (nreverse calls)
            (eq (symbol-function 'ado-new-program)
                (symbol-function 'ado-new-ado))
            (eq (symbol-function 'ado-save-program)
                (symbol-function 'save-buffer)))))"##;
    let expect = expect![[
        r#"OK (#1=("do-file" "do" t "one" "purpose one") #2=("mata file" "mata" nil "two" "purpose two") #3=("class" "class" t "three" nil) #4=("program" "ado" nil "four" "purpose four") #5=("program" "ado" t "five" "purpose five") #6=("program" "do" nil "six" "purpose six") (#1# #2# #3# #4# #5# #6#) nil nil)"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_insert_helpers_dates_help_options_and_cscript_match() {
    let elisp_form = r##"(let ((ado-date-format "%Y-%m-%d %H:%M")
               (ado-lowercase-date-flag nil)
               (ado-initials-flag t)
               (ado-initials "AB")
               mode-calls)
         (cl-letf (((symbol-function 'format-time-string)
                    (lambda (format-string &rest _arguments)
                      (concat "DATE[" format-string "]")))
                   ((symbol-function 'ado-mode)
                    (lambda () (push (buffer-name) mode-calls))))
           (unwind-protect
               (list
                (ado-nice-current-date)
                (progn
                  (setq ado-lowercase-date-flag t)
                  (ado-nice-current-date))
                (with-temp-buffer
                  (ado-insert-nice-current-date)
                  (buffer-string))
                (with-temp-buffer
                  (ado-help-insert-option-in-body "replace")
                  (list (buffer-string) (point)))
                (progn
                  (setq ado-help-extension "sthlp")
                  (list (ado-toggle-help-extension)
                        ado-help-extension
                        (ado-toggle-help-extension)
                        ado-help-extension))
                (progn
                  (ado-new-cscript "" "verify")
                  (prog1
                      (list (buffer-name) (buffer-string))
                    (kill-buffer (current-buffer))))
                (nreverse mode-calls))
             (when (get-buffer "verify.do")
               (kill-buffer "verify.do")))))"##;
    let expect = expect![[
        r#"OK ("DATE[%Y-%m-%d %H:%M] AB" "date[%y-%m-%d %h:%m] AB" "date[%y-%m-%d %h:%m] AB" ("{p 0 4}{cmd:replace}\n{p_end}\n\n" 28) ("ado-help-extension is now ‘hlp’" "hlp" "ado-help-extension is now ‘sthlp’" "sthlp") ("verify.do" "cscript \"verify\" adofile verify") ("verify.do"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_imenu_statacorp_and_window_adapters_match() {
    let elisp_form = r##"(let ((ado-close-under-line-flag t)
               (ado-lowercase-date-flag nil)
               (ado-date-format "old")
               (ado-initials-flag t)
               (tab-width 3)
               widths messages)
         (cl-letf (((symbol-function 'set-frame-width)
                    (lambda (frame width)
                      (push (list frame width) widths)))
                   ((symbol-function 'selected-frame)
                    (lambda () 'selected-frame))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments) messages))))
           (ado-set-imenu-items)
           (ado-set-window-width 99)
           (ado-statacorp-defaults)
           (list imenu-case-fold-search imenu-generic-expression
                 ado-close-under-line-flag ado-lowercase-date-flag
                 ado-date-format ado-initials-flag tab-width
                 (nreverse widths)
                 (seq-filter
                  (lambda (text)
                    (string-prefix-p
                     "ado-mode options set"
                     text))
                  (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (nil ((nil "^\\s-*pr\\(o\\|og\\|ogr\\|ogra\\|ogram\\)\\(\\s-+\\(de\\|def\\|defi\\|defin\\|define\\)?\\)\\s-+\\([a-zA-Z_][a-zA-Z_0-9]*\\)" 4)) nil t "%d%b%Y" nil 8 ((selected-frame 99) (selected-frame 80)) ("ado-mode options set to StataCorp defaults"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_generic_creator_expands_template_tokens_dates_names_and_save_contract() {
    let elisp_form = r##"(let ((ado-version-command "version 18")
               (ado-fontify-new-flag nil)
               (ado-new-dir nil)
               (ado-personal-dir nil)
               events)
         (cl-letf (((symbol-function 'ado-mode)
                    (lambda () (push 'mode events)))
                   ((symbol-function 'ado-insert-boilerplate)
                    (lambda (&rest arguments)
                      (push (cons 'template arguments) events)
                      (insert "*! version stata!!version\n"
                              "*! purpose: \n"
                              "program define putNameHere\n"
                              "startHere\n")))
                   ((symbol-function 'ado-nice-current-date)
                    (lambda () "FIXED-DATE"))
                   ((symbol-function 'ado-save-program)
                    (lambda (&rest arguments)
                      (push (cons 'save arguments) events)))
                   ((symbol-function 'set-visited-file-name)
                    (lambda (&rest arguments)
                      (push (cons 'visited arguments) events)))
                   ((symbol-function 'file-exists-p)
                    (lambda (&rest _arguments) nil)))
           (unwind-protect
               (progn
                 (ado-new-generic
                  "program" "do" t "sample" "does work" "/template.blp")
                 (list (buffer-name) (buffer-string) (point)
                       (nreverse events)))
             (when (get-buffer "sample.do")
               (kill-buffer "sample.do")))))"##;
    let expect = expect![[
        r#"OK ("sample.do" "*! version version 18FIXED-DATE\n*! purpose: does work\nprogram define sample\n\n" 77 (mode (template "/template.blp" nil t) (visited "sample.do") (save)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_generic_creator_covers_prompts_templates_directories_overwrite_version_and_fontification()
 {
    let elisp_form = r##"(let (results events ado-test-read-responses
              ado-test-confirmations ado-test-file-exists)
         (cl-letf
             (((symbol-function 'read-from-minibuffer)
               (lambda (prompt &rest _arguments)
                 (let ((answer (pop ado-test-read-responses)))
                   (push (list 'read prompt answer) events)
                   answer)))
              ((symbol-function 'y-or-n-p)
               (lambda (prompt)
                 (let ((answer (pop ado-test-confirmations)))
                   (push (list 'confirm prompt answer) events)
                   answer)))
              ((symbol-function 'ado-mode)
               (lambda () (push (list 'mode (buffer-name)) events)))
              ((symbol-function 'ado-insert-boilerplate)
               (lambda (&rest arguments)
                 (push (cons 'template arguments) events)
                 (insert "*! version stata!!version\n"
                         "*! purpose: \n"
                         "program define putNameHere\n"
                         "startHere\n")))
              ((symbol-function 'ado-nice-current-date)
               (lambda () " FIXED-DATE"))
              ((symbol-function 'ado-save-program)
               (lambda (&rest arguments)
                 (push (cons 'save arguments) events)))
              ((symbol-function 'set-visited-file-name)
               (lambda (&rest arguments)
                 (push (cons 'visited arguments) events)))
              ((symbol-function 'file-exists-p)
               (lambda (&rest _arguments) ado-test-file-exists))
              ((symbol-function 'cd)
               (lambda (directory)
                 (push (list 'cd directory) events)))
              ((symbol-function 'ado-reset-version-command)
               (lambda ()
                 (setq ado-version-command "version reset")
                 (push '(reset-version) events)))
              ((symbol-function 'turn-on-font-lock)
               (lambda () (push '(font-lock) events))))
           (dolist
               (case
                '((("program" "do" nil nil nil nil)
                   "prompted.do" "/new/" "/personal/" nil
                   ("prompted" "PROMPT PURPOSE") (t))
                  (("class" "class" nil "classy" "CLASS PURPOSE" nil)
                   "classy.class" nil "/personal/" nil nil (t))
                  (("do-file" "do" nil "runner" "RUN PURPOSE" nil)
                   "runner.do" "/new/" "/personal/" nil nil nil)
                  (("mata file" "mata" nil "declined" "MATA PURPOSE" nil)
                   "declined.mata" "/new/" "/personal/" nil nil (nil))
                  (("ado" "ado" nil "existing" "OLD PURPOSE" nil)
                   "existing.ado" nil nil t nil (nil))
                  (("ado" "ado" t "overwrite" "NEW PURPOSE" nil)
                   "overwrite.ado" nil nil t nil (t))))
             (setq events nil
                   ado-version-command ""
                   ado-fontify-new-flag t
                   ado-new-dir (nth 2 case)
                   ado-personal-dir (nth 3 case)
                   ado-test-file-exists (nth 4 case)
                   ado-test-read-responses (copy-sequence (nth 5 case))
                   ado-test-confirmations (copy-sequence (nth 6 case)))
             (let ((target (nth 1 case)))
               (unwind-protect
                   (progn
                     (apply #'ado-new-generic (car case))
                     (let ((buffer (get-buffer target)))
                       (push
                        (list target
                              (and buffer (buffer-live-p buffer))
                              (and buffer
                                   (with-current-buffer buffer
                                     (buffer-string)))
                              ado-version-command
                              (nreverse events))
                        results)))
                 (when (get-buffer target)
                   (kill-buffer target)))))
           (nreverse results)))"##;
    let expect = expect![[
        r#"OK (("prompted.do" t "*! version version reset FIXED-DATE\n*! purpose: PROMPT PURPOSE\nprogram define prompted\n\n" "version reset" ((read "What is the name of the program? " "prompted") (mode "prompted.do") (template "testado.blp") (read "What does it do? " "PROMPT PURPOSE") (confirm "Put in 'new' directory? " t) (cd "/new") #1=(reset-version) #2=(font-lock) (visited "prompted.do") (save))) ("classy.class" t "*! version version reset FIXED-DATE\n*! purpose: CLASS PURPOSE\nprogram define classy\n\n" "version reset" ((mode "classy.class") (template "class.blp") (confirm "Put in 'personal' directory? " t) (cd "/personal") #1# #2# (save))) ("runner.do" t "*! version version reset FIXED-DATE\n*! purpose: RUN PURPOSE\nprogram define runner\n\n" "version reset" ((mode "runner.do") (template "do.blp") #1# #2# (visited "runner.do") (save))) ("declined.mata" t "*! version version reset FIXED-DATE\n*! purpose: MATA PURPOSE\nprogram define declined\n\n" "version reset" ((mode "declined.mata") (template "mata.blp") (confirm "Put in 'new' directory? " nil) #1# #2# (visited "declined.mata") (save))) ("existing.ado" nil nil "" ((mode "existing.ado") (template "ado.blp") (confirm "File existing.ado already exists! Overwrite?" nil))) ("overwrite.ado" t "*! version version reset FIXED-DATE\n*! purpose: NEW PURPOSE\nprogram define overwrite\n\n" "version reset" ((mode "overwrite.ado") (template "ado.blp") (confirm "File overwrite.ado already exists! Overwrite?" t) #1# #2# (save))))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_program_label_and_help_creators_transform_templates_and_position_point() {
    let elisp_form = r##"(let ((ado-new-dir nil)
               (ado-label-dir nil)
               (ado-help-extension "sthlp")
               (ado-help-author-flag nil)
               events)
         (cl-letf (((symbol-function 'ado-mode)
                    (lambda () (push (list 'mode (buffer-name)) events)))
                   ((symbol-function 'ado-save-program)
                    (lambda (&rest arguments)
                      (push (cons 'save arguments) events)))
                   ((symbol-function 'ado-nice-current-date)
                    (lambda () "FIXED-DATE"))
                   ((symbol-function 'y-or-n-p)
                    (lambda (&rest _arguments) nil))
                   ((symbol-function 'ado-insert-boilerplate)
                    (lambda (file &rest _arguments)
                      (push (list 'template file) events)
                      (let ((start (point)))
                        (cond
                         ((string= file "smallado.blp")
                          (insert "program define \nend\n"))
                         ((string= file "lbl.blp")
                          (insert "label def \n"))
                         ((string= file "help.blp")
                          (insert "{smcl}\n"
                                  "XXX\n"
                                  "version #.#.# old\n"
                                  "{title:Author}\n"
                                  "{pstd}\n"
                                  "author placeholder\n"
                                  "{title:References}\n"
                                  "title of command\n")))
                        (goto-char start)))))
           (let (program-result label-result help-result)
             (with-temp-buffer
               (ado-insert-new-program "helper")
               (setq program-result (list (buffer-string) (point))))
             (unwind-protect
                 (progn
                   (ado-new-label "labels")
                   (setq label-result
                         (list (buffer-name) (buffer-string) (point))))
               (when (get-buffer "labels.lbl")
                 (kill-buffer "labels.lbl")))
             (unwind-protect
                 (progn
                   (ado-new-help "command")
                   (setq help-result
                         (list (buffer-name) (buffer-string) (point))))
               (when (get-buffer "command.sthlp")
                 (kill-buffer "command.sthlp")))
             (list program-result label-result help-result
                   (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (("program define helper\nend\n" 26) ("labels.lbl" "label def labels\n" 18) ("command.sthlp" "{smcl}\ncommand\nversion #.#.# FIXED-DATE}{...}\n{title:References}\ntitle of command\n" 66) ((template "smallado.blp") (template "lbl.blp") (mode "labels.lbl") (template "help.blp") (mode "command.sthlp") (save)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_help_creator_uses_installed_template_and_covers_all_author_sources() {
    let elisp_form = r##"(let* ((descriptor (cadr (assq 'ado-mode package-alist)))
                 (ado-site-template-dir
                  (expand-file-name
                   "templates"
                   (package-desc-dir descriptor)))
                 (original-insert
                  (symbol-function 'insert-file-contents))
                 results events ado-test-read-responses
                 ado-test-confirmations)
         (cl-letf
             (((symbol-function 'ado-mode)
               (lambda () (push (list 'mode (buffer-name)) events)))
              ((symbol-function 'ado-save-program)
               (lambda (&rest arguments)
                 (push (cons 'save arguments) events)))
              ((symbol-function 'ado-nice-current-date)
               (lambda () "FIXED-DATE"))
              ((symbol-function 'y-or-n-p)
               (lambda (prompt)
                 (let ((answer (pop ado-test-confirmations)))
                   (push (list 'confirm prompt answer) events)
                   answer)))
              ((symbol-function 'read-from-minibuffer)
               (lambda (prompt &rest _arguments)
                 (let ((answer (pop ado-test-read-responses)))
                   (push (list 'read prompt answer) events)
                   answer)))
              ((symbol-function 'cd)
               (lambda (directory)
                 (push (list 'cd directory) events)))
              ((symbol-function 'ado-set-ado-signature-file)
               (lambda ()
                 (setq ado-signature-file
                       "./tmp/prompted-signature")
                 (push '(signature-prompt) events)))
              ((symbol-function 'insert-file-contents)
               (lambda (file &rest arguments)
                 (if (and ado-signature-file
                          (string= file ado-signature-file))
                     (let ((text
                            (if (string-suffix-p
                                 "prompted-signature" file)
                                "Prompted Signature"
                              "Direct Signature")))
                       (push (list 'signature-read file) events)
                       (insert text)
                       (list file (length text)))
                   (apply original-insert file arguments)))))
           (dolist
               (case
                '(("direct-author" t "./tmp/direct-signature"
                   nil "" nil (t) "./tmp/new")
                  ("prompted-signature" t nil
                   t "" nil nil nil)
                  ("prompted-claim" t nil
                   nil "" ("Prompted Author") nil nil)
                  ("known-claim" t nil
                   nil "Known Author" nil nil nil)))
             (setq events nil
                   ado-help-extension "sthlp"
                   ado-help-author-flag (nth 1 case)
                   ado-signature-file (nth 2 case)
                   ado-signature-prompt-flag (nth 3 case)
                   ado-claim-name (nth 4 case)
                   ado-test-read-responses
                   (copy-sequence (nth 5 case))
                   ado-test-confirmations
                   (copy-sequence (nth 6 case))
                   ado-new-dir (nth 7 case))
             (let ((target (concat (car case) ".sthlp")))
               (unwind-protect
                   (progn
                     (ado-new-help (car case))
                     (let ((buffer (get-buffer target)))
                       (push
                        (with-current-buffer buffer
                          (let ((author-section
                                 (save-excursion
                                   (goto-char (point-min))
                                   (search-forward "{title:Author}")
                                   (let ((start
                                          (line-beginning-position)))
                                     (search-forward
                                      "{title:References}")
                                     (beginning-of-line)
                                     (buffer-substring-no-properties
                                      start (point))))))
                            (list target
                                  (buffer-size)
                                  (secure-hash
                                   'sha256 (current-buffer))
                                  author-section
                                  (point)
                                  ado-smart-indent-flag
                                  ado-claim-name
                                  (nreverse events))))
                        results)))
                 (when (get-buffer target)
                   (kill-buffer target)))))
           (nreverse results)))"##;
    let expect = expect![[
        r#"OK (("direct-author.sthlp" 3788 "bd0250624b09d9a3ee87b847ea3605d6cd906a0d7daa1c730d82055ed1efd177" "{title:Author}\n\n{pstd}\nDirect Signature\n{p_end}\n\n{marker references}{...}\n" 1012 nil "" ((confirm "Put in 'new' directory? " t) (cd "./tmp/new") (signature-read "./tmp/direct-signature") (mode "direct-author.sthlp") (save))) ("prompted-signature.sthlp" 3860 "37b6bd0a49de40b4f02c14a0a1c067270fab30c284f2eac6373e5adfdc191dba" "{title:Author}\n\n{pstd}\nPrompted Signature\n{p_end}\n\n{marker references}{...}\n" 1067 nil "" ((signature-prompt) (signature-read "./tmp/prompted-signature") (mode "prompted-signature.sthlp") (save))) ("prompted-claim.sthlp" 3801 "e0ce451b292745e56bc3427825c70031ad551ba561f6b7024128eeac2fffd10d" "{title:Author}\n\n{pstd}\nPrompted Author\n{p_end}\n\n{marker references}{...}\n" 1023 nil "Prompted Author" ((read "Whose name(s) should be used as authors? " "Prompted Author") (mode "prompted-claim.sthlp") (save))) ("known-claim.sthlp" 3756 "db343861495e228c03ccc1e281e22f1b2d2df5d11a63dc786d1837bfa91a9e67" "{title:Author}\n\n{pstd}\nKnown Author\n{p_end}\n\n{marker references}{...}\n" 990 nil "Known Author" ((mode "known-claim.sthlp") (save))))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}
