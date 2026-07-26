use super::{assert_ack_menu_parity, assert_ack_menu_signal_parity};
use expect_test::expect;

#[test]
fn ack_menu_group_definition_matches_every_action_switch_and_argument() {
    let elisp_form = r##"(copy-tree
         ack-menu-group)"##;
    let expect = expect![[
        r#"OK (ack (man-page nil) (actions ("r" "Run" ack-menu-action)) (switches ("-c" "Current project dir" "-c" ack-menu-current-project-switch) ("-bd" "Buffer dir" "-bd" ack-menu-buffer-dir-switch) ("-bp" "Buffer project root dir" "-bp" ack-menu-buffer-project-dir-switch) ("-a" "All files" "--all") ("-i" "Ignore case" "--ignore-case") ("-n" "No recurse" "--no-recurse") ("-fm" "Only print file names matched" "--files-with-matches" ack-menu-only-print-files-switch) ("-fs" "Only print file names searched" "-f" ack-menu-only-print-files-switch) ("-w" "Match whole word" "--word-regexp") ("-q" "Literal search, no regex" "--literal")) (arguments ("-m" "Match" "--match=" mag-menu-read-generic ack-menu-match-history) ("-d" "Directory" "--directory=" mag-menu-read-directory-name) ("-B" "Num context lines before" "--before-context=" mag-menu-read-generic) ("-A" "Num context lines after" "--after-context=" mag-menu-read-generic) ("-C" "Num context lines around" "--context=" mag-menu-read-generic)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_buffer_major_mode_and_current_word_preserve_exact_values() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (setq major-mode
                 'fixture-major-mode)
           (ack-buffer-major-mode
            (current-buffer)))
         (with-temp-buffer
           (insert
            "alpha beta")
           (goto-char 3)
           (list
            (ack-get-current-word
             "fallback")
            (text-properties-at
             0
             (ack-get-current-word
              "fallback"))))
         (with-temp-buffer
           (insert
            "   ")
           (goto-char 2)
           (ack-get-current-word
            "fallback")))"##;
    let expect = expect![[r#"OK (fixture-major-mode ("alpha" nil) "fallback")"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_directory_switches_set_exact_options() {
    let elisp_form = r##"(let ((ack-menu-current-state
                (list
                 (current-buffer)
                 nil))
               (ack-current-project-directory
                "/fixture/current/")
               calls)
         (cl-letf
             (((symbol-function
                'mag-menu-set-option)
               (lambda (&rest arguments)
                 (push arguments calls)
                 (cons
                  'set
                  arguments)))
              ((symbol-function
                'ack-buffer-dir)
               (lambda (buffer)
                 (push
                  (list
                   'buffer-dir
                   (buffer-name buffer))
                  calls)
                 "/fixture/buffer/"))
              ((symbol-function
                'ack-guess-project-root)
               (lambda ()
                 "/fixture/project/")))
           (list
            (ack-menu-current-project-switch
             "-c"
             '(("--all")))
            (ack-menu-buffer-dir-switch
             "-bd"
             '(("--all")))
            (ack-menu-buffer-project-dir-switch
             "-bp"
             '(("--all")))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((set . #1=((("--all")) "--directory" "/fixture/current/")) (set . #2=((("--all")) "--directory" "/fixture/buffer/")) (set . #3=((("--all")) "--directory" "/fixture/project/")) (#1# (buffer-dir "*scratch*") #2# #3#))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_current_project_switch_signals_when_directory_is_unset() {
    let elisp_form = r##"(let ((ack-current-project-directory
                nil))
         (ack-menu-current-project-switch
          "-c"
          nil))"##;
    let expect = expect![[r#"ERR (error "ack-current-project-directory isn’t set")"#]];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

#[test]
fn ack_menu_buffer_project_switch_signals_when_root_cannot_be_guessed() {
    let elisp_form = r##"(let ((ack-menu-current-state
                (list
                 (current-buffer)
                 nil)))
         (cl-letf
             (((symbol-function
                'ack-guess-project-root)
               (lambda ()
                 nil)))
           (ack-menu-buffer-project-dir-switch
            "-bp"
            nil)))"##;
    let expect = expect![[r#"ERR (error "Failed to guess project root for buffer ’*scratch*’")"#]];
    assert_ack_menu_signal_parity(elisp_form, expect);
}

#[test]
fn ack_menu_only_print_files_switch_enforces_mutual_exclusion_and_toggle_semantics() {
    let elisp_form = r##"(list
         (ack-menu-only-print-files-switch
          "--files-with-matches"
          '(("--ignore-case")))
         (ack-menu-only-print-files-switch
          "-f"
          '(("--files-with-matches")
            ("-f")
            ("--match" . "x")))
         (ack-menu-only-print-files-switch
          "--files-with-matches"
          '(("--files-with-matches")
            ("--ignore-case"))))"##;
    let expect = expect![[
        r#"OK ((("--ignore-case") ("--files-with-matches")) (("--files-with-matches") ("--match" . "x")) (("--ignore-case")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_command_populates_defaults_invokes_menu_and_optionally_prompts_match() {
    let elisp_form = r##"(let ((ack-menu-options
                '(("--ignore-case")))
               (ack-pushy-match-prompt
                nil)
               calls)
         (cl-letf
             (((symbol-function
                'ack-check-version)
               (lambda ()
                 (push
                  'check-version
                  calls)))
              ((symbol-function
                'ack-buffer-dir)
               (lambda (buffer)
                 (push
                  (list
                   'buffer-dir
                   (buffer-name buffer))
                  calls)
                 "/fixture/buffer/"))
              ((symbol-function
                'ack-get-current-word)
               (lambda (default)
                 (push
                  (list
                   'word
                   default)
                  calls)
                 (or default
                     "current")))
              ((symbol-function
                'mag-menu-set-option)
               (lambda (options name value)
                 (push
                  (list
                   'set
                   name
                   value)
                  calls)
                 (cons
                  (cons name value)
                  options)))
              ((symbol-function
                'mag-menu)
               (lambda (group options)
                 (push
                  (list
                   'menu
                   (car group)
                   options)
                  calls)
                 'shown))
              ((symbol-function
                'mag-menu-add-argument)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'add-argument
                   arguments)
                  calls))))
           (let ((first
                  (ack-menu)))
             (setq ack-pushy-match-prompt
                   t
                   ack-menu-options
                   '(("--directory"
                      .
                      "/preset/")
                     ("--match"
                      .
                      "preset")))
             (let ((second
                    (ack-menu)))
               (list
                first
                second
                (mapcar
                 (lambda (value)
                   (if (bufferp value)
                       (buffer-name value)
                     value))
                 ack-menu-current-state)
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (nil #1=((add-argument (ack (man-page nil) (actions ("r" "Run" ack-menu-action)) (switches ("-c" "Current project dir" "-c" ack-menu-current-project-switch) ("-bd" "Buffer dir" "-bd" ack-menu-buffer-dir-switch) ("-bp" "Buffer project root dir" "-bp" ack-menu-buffer-project-dir-switch) ("-a" "All files" "--all") ("-i" "Ignore case" "--ignore-case") ("-n" "No recurse" "--no-recurse") ("-fm" "Only print file names matched" "--files-with-matches" ack-menu-only-print-files-switch) ("-fs" "Only print file names searched" "-f" ack-menu-only-print-files-switch) ("-w" "Match whole word" "--word-regexp") ("-q" "Literal search, no regex" "--literal")) (arguments ("-m" "Match" "--match=" mag-menu-read-generic ack-menu-match-history) ("-d" "Directory" "--directory=" mag-menu-read-directory-name) ("-B" "Num context lines before" "--before-context=" mag-menu-read-generic) ("-A" "Num context lines after" "--after-context=" mag-menu-read-generic) ("-C" "Num context lines around" "--context=" mag-menu-read-generic))) "--match=" mag-menu-read-generic ack-menu-match-history)) ("*scratch*" "current") (check-version (word nil) (buffer-dir "*scratch*") (set "--directory" "/fixture/buffer/") (word "search") (set "--match" "search") (menu ack (("--match" . "search") ("--directory" . "/fixture/buffer/") ("--ignore-case"))) check-version (word nil) (menu ack (("--directory" . "/preset/") ("--match" . "preset"))) . #1#))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_action_persists_options_processes_arguments_and_runs_ack() {
    let elisp_form = r##"(let ((ack-menu-options
                nil)
               calls)
         (cl-letf
             (((symbol-function
                'ack-process-args)
               (lambda (options)
                 (push
                  (list
                   'process
                   (copy-tree options))
                  calls)
                 '("/fixture/"
                   ("--color"
                    "--match=x"))))
              ((symbol-function
                'ack-run-impl)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'run
                   arguments)
                  calls)
                 'started)))
           (list
            (ack-menu-action
             '(("--match" . "x")
               ("--ignore-case")))
            ack-menu-options
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (started (("--match" . "x") ("--ignore-case")) ((process (("--match" . "x") ("--ignore-case"))) (run "/fixture/" "--color" "--match=x")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}
