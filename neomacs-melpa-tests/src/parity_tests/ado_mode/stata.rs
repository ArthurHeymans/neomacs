use expect_test::expect;

use super::assert_ado_mode_parity;

#[test]
fn ado_mode_send_command_and_help_wrappers_forward_exact_modes_and_flags() {
    let elisp_form = r##"(let ((ado-submit-default "include")
               (ado-comeback-flag t)
               calls)
         (cl-letf (((symbol-function 'ado-command-to-clip)
                    (lambda (&rest arguments)
                      (push (cons 'clip arguments) calls)))
                   ((symbol-function 'ado-send-clip-to-stata)
                    (lambda (&rest arguments)
                      (push (cons 'send arguments) calls)))
                   ((symbol-function 'ado-help-at-point-to-clip)
                    (lambda () (push '(help at-point) calls)))
                   ((symbol-function 'ado-help-command-to-clip)
                    (lambda () (push '(help command) calls))))
           (list
            (ado-send-command-to-stata nil)
            (ado-send-command-to-command t)
            (ado-send-command-to-menu nil)
            (ado-send-command-to-dofile t)
            (ado-send-command-to-include nil)
            (ado-stata-help t)
            (ado-stata-help nil)
            (ado-help-at-point)
            (ado-help-command)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (#11=((send "include" t) (clip "command" t) . #3=((send "command" t) (clip "menu" nil) . #4=((send "menu" t) (clip "dofile" t) . #5=((send "dofile" t) (clip "include" nil) . #6=((send "include" t) #1=(help at-point) . #7=((send "include") #2=(help command) . #8=((send "include") #1# . #9=((send "include") #2# . #10=((send "include")))))))))) #3# #4# #5# #6# #7# #8# #9# #10# ((clip "include" nil) . #11#))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_send_clip_to_stata_covers_all_operating_system_command_builders() {
    let elisp_form = r##"(let ((ado-submit-default "command")
               (ado-comeback-flag nil)
               (ado-temp-dofile "TEMP DO.do")
               (ado-stata-instance 2)
               (ado-stata-version "18")
               (ado-stata-flavor "MP")
               (ado-send-to-all-flag t)
               (ado-strict-match-flag t)
               (invocation-directory "/Applications/Emacs.app/Contents/MacOS/")
               shell-calls process-calls messages)
         (cl-letf (((symbol-function 'ado-send2stata-name)
                    (lambda (name) (concat "/scripts with spaces/" name)))
                   ((symbol-function 'shell-quote-argument)
                    (lambda (value) (format "<%s>" value)))
                   ((symbol-function 'shell-command)
                    (lambda (command)
                      (push command shell-calls)
                      0))
                   ((symbol-function 'call-process-shell-command)
                    (lambda (&rest arguments)
                      (push arguments process-calls)
                      0))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (let ((text (apply #'format format-string arguments)))
                        (push text messages)
                        text))))
           (let (results)
             (dolist (case
                      '((darwin "command" nil)
                        (darwin "include" t)
                        (windows-nt "command" nil)
                        (windows-nt "dofile" t)
                        (gnu/linux "menu" nil)
                        (haiku "command" nil)
                        (haiku "dofile" nil)
                        (windows-nt "menu" t)
                        (gnu/linux "invalid" nil)))
               (setq system-type (nth 0 case))
               (push
                (condition-case error-data
                    (ado-send-clip-to-stata (nth 1 case) (nth 2 case))
                  (error (list 'signal (car error-data) (cdr error-data))))
                results))
             (list (nreverse results)
                   (nreverse shell-calls)
                   (nreverse process-calls)
                   (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (("selection sent to Stata" "selection sent to Stata" nil nil nil nil nil (signal error ("Cannot comeback to Stata after using a menu in MS Windows")) (signal error ("Bad value for `do-this' in ado-send-region-to-stata"))) ("osascript </scripts with spaces/send2stata.scpt> <command>" "osascript </scripts with spaces/send2stata.scpt> <include>" "open \"/Applications/Emacs.app\"" "</scripts with spaces/send2ztata.sh> -d <menu> &") (("</scripts with spaces/send2stata.exe> <command> <> <TEMP DO.do> <2> <18> <MP> <t> <t>" nil 0) ("</scripts with spaces/send2stata.exe> <dofile> <t> <TEMP DO.do> <2> <18> <MP> <t> <t>" nil 0)) ("selection sent to Stata" "selection sent to Stata" "Working via commands not supported yet in haiku, but the command is on the clipboard and you can paste it in the command window by hand" "Working via dofiles not supported yet in haiku"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_send_buffer_and_minibuffer_cover_modified_saved_and_default_paths() {
    let elisp_form = r##"(let* ((ado-submit-default "menu")
                (ado-comeback-flag t)
                (calls nil)
                (clipboard nil)
                (interprogram-cut-function
                 (lambda (text) (push text clipboard))))
         (cl-letf (((symbol-function 'ado-send-command-to-dofile)
                    (lambda (&rest arguments)
                      (push (cons 'dofile arguments) calls)))
                   ((symbol-function 'ado-send-command-to-stata)
                    (lambda (&rest arguments)
                      (push (cons 'default arguments) calls)))
                   ((symbol-function 'ado-send-clip-to-stata)
                    (lambda (&rest arguments)
                      (push (cons 'send arguments) calls)))
                   ((symbol-function 'read-from-minibuffer)
                    (lambda (&rest _arguments) "summarize mpg")))
           (with-temp-buffer
             (setq buffer-file-name "/work/saved file.do")
             (set-buffer-modified-p t)
             (ado-send-buffer-to-stata nil))
           (with-temp-buffer
             (setq buffer-file-name "/work/saved file.do")
             (set-buffer-modified-p nil)
             (ado-send-buffer-to-stata nil))
           (with-temp-buffer
             (ado-send-buffer-to-stata t))
           (ado-input-to-stata)
           (list (nreverse calls) (nreverse clipboard))))"##;
    let expect = expect![[
        r#"OK (((dofile t) (send "command" t) (default t) (send "menu" t)) ("do \"/work/saved file.do\"" "summarize mpg"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_script_path_and_directory_validation_cover_success_and_errors() {
    let elisp_form = r##"(let ((ado-script-dir "/virtual/scripts")
               existing)
         (cl-letf (((symbol-function 'file-exists-p)
                    (lambda (path) (member path existing)))
                   ((symbol-function 'locate-file)
                    (lambda (name directories)
                      (let ((candidate
                             (expand-file-name name (car directories))))
                        (and (member candidate existing) candidate)))))
           (let (results)
             (setq existing '("/virtual/scripts/"
                              "/virtual/scripts/send2stata.scpt"))
             (push (ado-check-a-directory 'ado-script-dir) results)
             (push (ado-send2stata-name "send2stata.scpt") results)
             (setq existing '("/virtual/scripts/"))
             (push
              (condition-case error-data
                  (ado-send2stata-name "missing")
                (error (list 'signal (car error-data) (cdr error-data))))
              results)
             (setq existing nil)
             (push
              (condition-case error-data
                  (ado-check-a-directory 'ado-script-dir)
                (error (list 'signal (car error-data) (cdr error-data))))
              results)
             (setq ado-script-dir nil)
             (push
              (condition-case error-data
                  (ado-check-a-directory 'ado-script-dir)
                (error (list 'signal (car error-data) (cdr error-data))))
              results)
             (nreverse results))))"##;
    let expect = expect![[
        r#"OK ("/virtual/scripts/" "/virtual/scripts/send2stata.scpt" (signal error ("Could not find missing. Did you change ado-script-dir by hand? If you did, try changing its default value back to nil.")) (signal error ("ado-script-dir's value: /virtual/scripts/ does not exist.")) (signal error ("ado-script-dir is nil")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_find_stata_covers_platform_flavor_priority_missing_home_and_unknown_os() {
    let elisp_form = r##"(let (existing)
         (cl-letf (((symbol-function 'file-exists-p)
                    (lambda (path) (member path existing)))
                   ((symbol-function 'file-directory-p)
                    (lambda (path) (member path existing))))
           (mapcar
            (lambda (case)
              (setq system-type (nth 0 case)
                    existing (nth 1 case)
                    ado-stata-home (nth 2 case))
              (condition-case error-data
                  (ado-find-stata (nth 3 case))
                (error (list 'signal (car error-data) (cdr error-data)))))
            '((darwin ("/apps/StataSE.app") nil "/apps")
              (darwin nil nil "/apps")
              (windows-nt ("/apps/StataMP-64.exe") nil "/apps")
              (gnu/linux ("/apps/stata-be") nil "/apps")
              (gnu/linux nil nil "/apps")
              (haiku nil nil "/apps")
              (gnu/linux ("/configured/stata") "/configured" nil)
              (gnu/linux nil nil nil)))))"##;
    let expect = expect![[
        r#"OK ("/apps/StataSE.app/Contents/MacOS/StataSE" (signal error ("Could not find any Stata in /apps")) "/apps/StataMP-64.exe" "/apps/stata-be" (signal error ("Could not find Console Stata (needed for background tasks) in /apps")) (signal wrong-type-argument (sequencep haiku)) "/configured/stata" (signal error ("You need to set ado-stata-home to open files on the adopath")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_stata_directory_reset_and_open_adapters_preserve_order_and_selection() {
    let elisp_form = r##"(let ((ado-stata-home "/stata")
               (ado-open-read-only-flag t)
               resets finds opens)
         (cl-letf (((symbol-function 'ado-get-filename-from-stata)
                    (lambda (command arguments)
                      (push (list command arguments) finds)
                      (cond
                       ((string-prefix-p "c(sysdir_" arguments)
                        (concat "/dirs/" arguments))
                       ((string= arguments "missing.ado") nil)
                       (t (concat "/found/" arguments)))))
                   ((symbol-function 'find-file-read-only)
                    (lambda (path) (push (list 'readonly path) opens)))
                   ((symbol-function 'find-file)
                    (lambda (path) (push (list 'writable path) opens))))
           (ado-reset-adopath)
           (push (list ado-personal-dir ado-plus-dir
                       ado-site-dir ado-oldplace-dir)
                 resets)
           (ado-reset-personal-dir)
           (ado-reset-plus-dir)
           (ado-reset-site-dir)
           (ado-reset-oldplace-dir)
           (ado-reset-sysdir "personal")
           (ado-open-file-on-adopath "alpha")
           (setq ado-open-read-only-flag nil)
           (ado-open-file-on-adopath "beta.sthlp")
           (let ((missing
                  (condition-case error-data
                      (ado-open-file-on-adopath "missing")
                    (error (list 'signal (car error-data)
                                 (cdr error-data))))))
             (setq ado-stata-home nil)
             (list (nreverse resets)
                   (nreverse finds)
                   (nreverse opens)
                   missing
                   (condition-case error-data
                       (ado-open-file-on-adopath "alpha")
                     (error (list 'signal (car error-data)
                                  (cdr error-data))))))))"##;
    let expect = expect![[
        r#"OK ((("/dirs/c(sysdir_personal)" "/dirs/c(sysdir_plus)" "/dirs/c(sysdir_site)" "/dirs/c(sysdir_oldplace)")) (("display" "c(sysdir_personal)") ("display" "c(sysdir_plus)") ("display" "c(sysdir_site)") ("display" "c(sysdir_oldplace)") ("display" "c(sysdir_personal)") ("display" "c(sysdir_plus)") ("display" "c(sysdir_site)") ("display" "c(sysdir_oldplace)") ("display" "c(sysdir_personal)") ("findfile" "alpha.ado") ("findfile" "beta.sthlp") ("findfile" "missing.ado")) ((readonly "/found/alpha.ado") (writable "/found/beta.sthlp")) (signal error ("File missing.ado not found on adopath")) (signal error ("You need to set ado-stata-home to open files on the adopath")))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_stata_result_parser_and_os_command_contracts_match_without_io() {
    let elisp_form = r##"(let (commands reads results)
         (cl-letf
             (((symbol-function 'ado-system-tmp-dir)
               (lambda () "./tmp/oracle-stata/"))
              ((symbol-function 'ado-find-stata)
               (lambda (&optional _where) "/virtual/Stata Console"))
              ((symbol-function 'shell-quote-argument)
               (lambda (value) (format "<%s>" value)))
              ((symbol-function 'shell-command)
               (lambda (command)
                 (push command commands)
                 0))
              ((symbol-function 'insert-file-contents)
               (lambda (&rest arguments)
                 (push arguments reads)
                 (erase-buffer)
                 (insert ado-test-log-content)
                 (list "virtual-log"
                       (length ado-test-log-content)))))
           (dolist
               (case
                '((darwin "display" "c(tmpdir)"
                          "header\nresult-value\n")
                  (windows-nt "version" nil
                              "header\nversion 18\n")
                  (gnu/linux "display" "c(value)"
                             "header\nr(198)\n")
                  (gnu/linux "display" "c(wrapped)"
                             "header\nwrapped result\n")))
             (setq system-type (nth 0 case)
                   ado-test-log-content (nth 3 case))
             (push
              (ado-get-one-result (nth 1 case) (nth 2 case))
              results))
           (setq system-type 'haiku)
           (push
            (condition-case error-data
                (ado-get-one-result "display" "c(value)")
              (error (list 'signal (car error-data)
                           (cdr error-data))))
            results)
           (list (nreverse results)
                 (nreverse commands)
                 (nreverse reads))))"##;
    let expect = expect![[
        r#"OK (("result-value" "version 18" nil "wrapped result" (signal wrong-type-argument (sequencep haiku))) ("cd <./tmp/oracle-stata/> ; </virtual/Stata Console> -q -b -e <display> <c(tmpdir)>" "cd <./tmp/oracle-stata/> & </virtual/Stata Console> /q /e <version>" "cd <./tmp/oracle-stata/> ; </virtual/Stata Console> -q -e <display> <c(value)>" "cd <./tmp/oracle-stata/> ; </virtual/Stata Console> -q -e <display> <c(wrapped)>") (("./tmp/oracle-stata/stata.log" nil nil nil t) ("./tmp/oracle-stata/stata.log" nil nil nil t) ("./tmp/oracle-stata/stata.log" nil nil nil t) ("./tmp/oracle-stata/stata.log" nil nil nil t)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_stata_version_and_filename_adapters_cover_fallback_reset_and_windows_paths() {
    let elisp_form = r##"(let (results)
         (setq system-type 'windows-nt)
         (cl-letf (((symbol-function 'ado-get-one-result)
                    (lambda (&rest _arguments)
                      "C:\\Users\\Ada\\file.ado\nignored")))
           (push (ado-get-filename-from-stata
                  "display" "c(filename)")
                 results))
         (cl-letf (((symbol-function 'ado-get-one-result)
                    (lambda (&rest _arguments) nil)))
           (push (ado-get-stata-version) results))
         (cl-letf (((symbol-function 'ado-get-one-result)
                    (lambda (&rest _arguments) "version 18.0")))
           (ado-reset-version-command)
           (push (list (ado-get-stata-version)
                       ado-version-command)
                 results))
         (nreverse results))"##;
    let expect = expect![[
        r#"OK ("C:\\Users\\Ada\\file.ado\nignored" "version !!??" ("version 18.0" "version 18.0"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_system_directory_and_string_helpers_cover_non_linux_branches_without_disk_io() {
    let elisp_form = r##"(let ((process-environment
                (cons "TEMP=C:\\Users\\Ada\\Temp"
                      process-environment)))
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (&rest _arguments)
                      "/Users/ada/Library/Caches/T/\n")))
           (list
            (progn (setq system-type 'darwin)
                   (ado-system-tmp-dir))
            (progn (setq system-type 'windows-nt)
                   (ado-system-tmp-dir))
            (progn
              (setq system-type 'haiku)
              (condition-case error-data
                  (ado-system-tmp-dir)
                (error (list 'signal (car error-data)
                             (cdr error-data)))))
            (mapcar #'ado-strip-after-newline
                    '("one\ntwo\n" "single" "\nleading" "")))))"##;
    let expect = expect![[
        r#"OK ("/Users/ada/Library/Caches/T/" "C:\\Users\\Ada\\Temp/" (signal error ("System temp dir not found, somehow")) ("one\n" "single" "" ""))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_user_prompt_open_wrappers_show_commands_and_tcc_reset_match() {
    let elisp_form = r##"(let (opens messages shells)
         (cl-letf (((symbol-function 'read-from-minibuffer)
                    (lambda (&rest _arguments) "chosen"))
                   ((symbol-function 'ado-grab-something)
                    (lambda (&rest _arguments) "current"))
                   ((symbol-function 'ado-open-file-on-adopath)
                    (lambda (filename) (push filename opens)))
                   ((symbol-function 'ado-find-stata)
                    (lambda (&rest _arguments) "/virtual/stata"))
                   ((symbol-function 'ado-system-tmp-dir)
                    (lambda () "./tmp/stata/"))
                   ((symbol-function 'ado-get-stata-version)
                    (lambda () "version 18"))
                   ((symbol-function 'shell-command)
                    (lambda (command) (push command shells)))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages))))
           (list
            (ado-ask-filename)
            (ado-open-command)
            (ado-open-any-file)
            (ado-show-stata)
            (ado-show-tmp-dir)
            (ado-show-stata-version)
            (progn
              (when (boundp 'aquamacs-version)
                (makunbound 'aquamacs-version))
              (ado-reset-tcc))
            (progn
              (setq aquamacs-version "test")
              (ado-reset-tcc))
            (nreverse opens)
            (nreverse messages)
            (nreverse shells))))"##;
    let expect = expect![[
        r#"OK ("chosen" #5=("current" . #1=("chosen")) #1# #6=("Found Stata here: /virtual/stata" . #2=("Found tmpdir here: ./tmp/stata/" . #3=("Found Stata version: version 18"))) #2# #3# #7=("tccutil reset All org.gnu.Emacs" . #4=("tccutil reset All org.gnu.Aquamacs")) #4# #5# #6# #7#)"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}
