use expect_test::expect;

use super::assert_ack_menu_parity;

/// The package's headline story: from a source file, `M-x ack-menu' opens the
/// option menu seeded with the word at point and the buffer's directory, and
/// `r' runs the real ack with those options.  This pins the rendered menu, the
/// exact argument vector and working directory ack was started with, the
/// grouped results buffer with the file/line/match text properties the SGR
/// parser produced, the mode and read-only state of that buffer, the match
/// count message, the option state kept for the next menu, and that the menu
/// buffer is gone afterwards.
#[test]
fn ack_menu_assembles_the_argv_from_the_buffer_and_renders_the_matches() {
    let elisp_form = r##"(progn
  (ack-test-setup)
  (ack-test-restore-ansi-color-constants)
  (ack-test-open "src/main.el" "handler")
  (global-set-key (kbd "C-c a") 'ack-menu)
  (let ((mark (ack-test-message-mark)))
    (execute-kbd-macro (kbd "C-c a"))
    (let ((menu (list (ack-test-menu-text) (ack-test-menu-state))))
      (execute-kbd-macro (kbd "r"))
      (list :menu menu
            :finished (ack-test-wait)
            :invocations (ack-test-invocations)
            :text (ack-test-results-text)
            :segments (ack-test-results-segments)
            :state (ack-test-results-state)
            :options (ack-test-options)
            :menu-buffer (get-buffer "*mag-menu*")
            :messages (ack-test-messages-since mark)))))"##;
    let expect = expect![[
        r#"OK (:menu ("Switches\n -c: Current project dir (-c)\n -bd: Buffer dir (-bd)\n -bp: Buffer project root dir (-bp)\n -a: All files (--all)\n -i: Ignore case (--ignore-case)\n -n: No recurse (--no-recurse)\n -fm: Only print file names matched (--files-with-matches)\n -fs: Only print file names searched (-f)\n -w: Match whole word (--word-regexp)\n -q: Literal search, no regex (--literal)\nArgs\n -m: Match (--match=) handler\n -d: Directory (--directory=) [ORACLE-SANDBOX]/project/src/\n -B: Num context lines before (--before-context=)\n -A: Num context lines after (--after-context=)\n -C: Num context lines around (--context=)\n\nActions\n r: Run\n" (("--ignore-case") (("--directory=" . "./src/") ("--match=" . "handler")))) :finished t :invocations (("argv" "--color" "--nopager" "--ignore-case" "--match=handler") ("cwd" "src")) :text "main.el\n2:(defun handler (request)\n3:  (message \"handler ready\"))\n\nnotes with space.txt\n1:the handler notes\n\n" :segments (("main.el" ack-file "main.el" nil nil) ("\n" nil nil nil nil) ("2" ack-line nil "2" nil) (":(defun " nil nil nil nil) ("handler" ack-match nil nil t) (" (request)\n" nil nil nil nil) ("3" ack-line nil "3" nil) (":  (message \"" nil nil nil nil) ("handler" ack-match nil nil t) (" ready\"))\n\n" nil nil nil nil) ("notes with space.txt" ack-file "notes with space.txt" nil nil) ("\n" nil nil nil nil) ("1" ack-line nil "1" nil) (":the " nil nil nil nil) ("handler" ack-match nil nil t) (" notes\n\n" nil nil nil nil)) :state (:mode ack-mode :read-only t :directory "src/" :next-error-function ack-next-error-function :size 109) :options (("--ignore-case") ("--directory" . "./src/") ("--match" . "handler")) :menu-buffer nil :messages ("Invalid face reference: widget-field [2 times]" "Type a prefix key to toggle it. Run ’actions’ with their prefixes. ’?’ for more help." "Ack finished with 3 matches"))"#
    ]];

    assert_ack_menu_parity(elisp_form, expect);
}

/// Two menu switches that change what is searched: `-bp' points ack at the
/// project root `ack-guess-project-root' found (the directory holding `.git'),
/// so the documentation files join the search, and the default
/// `--ignore-case' makes ack match the capitalised "Handler" in
/// docs/CHANGELOG.  Re-opening the menu shows the options the previous run
/// stored, and toggling `-i' off drops that match again.
#[test]
fn the_project_root_switch_widens_the_search_and_ignore_case_toggles_back_off() {
    let elisp_form = r##"(progn
  (ack-test-setup)
  (ack-test-restore-ansi-color-constants)
  (ack-test-open "src/main.el" "handler")
  (global-set-key (kbd "C-c a") 'ack-menu)
  (execute-kbd-macro (kbd "C-c a"))
  (execute-kbd-macro (kbd "- b p"))
  (let ((menu (ack-test-menu-state)))
    (execute-kbd-macro (kbd "r"))
    (ack-test-wait)
    (let ((wide (list :invocations (ack-test-invocations)
                      :text (ack-test-results-text)
                      :options (ack-test-options))))
      (execute-kbd-macro (kbd "C-c a"))
      (let ((reopened (ack-test-menu-state)))
        (execute-kbd-macro (kbd "- i"))
        (let ((toggled (ack-test-menu-state)))
          (execute-kbd-macro (kbd "r"))
          (ack-test-wait)
          (list :menu-after-bp menu
                :wide wide
                :reopened reopened
                :toggled toggled
                :invocations (last (ack-test-invocations) 2)
                :text (ack-test-results-text)
                :options (ack-test-options)))))))"##;
    let expect = expect![[
        r#"OK (:menu-after-bp (("--ignore-case") (("--directory=" . "./") ("--match=" . "handler"))) :wide (:invocations (("argv" "--color" "--nopager" "--ignore-case" "--match=handler") ("cwd" ".")) :text "docs/CHANGELOG\n1:Handler rewritten\n\ndocs/readme.md\n2:The café handler serves naïve clients.\n\nsrc/main.el\n2:(defun handler (request)\n3:  (message \"handler ready\"))\n\nsrc/notes with space.txt\n1:the handler notes\n\n" :options (("--ignore-case") ("--directory" . "./") ("--match" . "handler"))) :reopened (("--ignore-case") (("--directory=" . "./") ("--match=" . "handler"))) :toggled (nil (("--directory=" . "./") ("--match=" . "handler"))) :invocations (("argv" "--color" "--nopager" "--match=handler") ("cwd" ".")) :text "docs/readme.md\n2:The café handler serves naïve clients.\n\nsrc/main.el\n2:(defun handler (request)\n3:  (message \"handler ready\"))\n\nsrc/notes with space.txt\n1:the handler notes\n\n" :options (("--directory" . "./") ("--match" . "handler")))"#
    ]];

    assert_ack_menu_parity(elisp_form, expect);
}

/// Navigating the results the way the mode's own bindings do: `n' and `p'
/// walk matches, `M-n' walks files, `RET' visits the match under point, and
/// `next-error' walks them from anywhere.  Every landing is pinned with its
/// buffer, point, line and column -- including the match in the Unicode
/// document, whose column proves the offset is counted in characters and not
/// in bytes, and the file whose name contains a space.
#[test]
fn navigating_the_results_visits_the_exact_file_line_and_column() {
    let elisp_form = r##"(progn
  (ack-test-setup)
  (ack-test-restore-ansi-color-constants)
  (ack-test-open "src/main.el" "handler")
  (global-set-key (kbd "C-c a") 'ack-menu)
  (execute-kbd-macro (kbd "C-c a"))
  (execute-kbd-macro (kbd "- b p"))
  (execute-kbd-macro (kbd "r"))
  (ack-test-wait)
  (let (observed)
    (set-window-buffer (selected-window) (get-buffer "*ack*"))
    (set-buffer "*ack*")
    (goto-char (point-min))
    (execute-kbd-macro (kbd "n"))
    (push (list :n (point)) observed)
    (execute-kbd-macro (kbd "n"))
    (push (list :n-n (point)) observed)
    (execute-kbd-macro (kbd "p"))
    (push (list :p (point)) observed)
    (execute-kbd-macro (kbd "M-n"))
    (push (list :next-file (point)) observed)
    (execute-kbd-macro (kbd "n"))
    (push (list :match-after-file (point)) observed)
    (execute-kbd-macro (kbd "RET"))
    (push (list :visited (ack-test-window-state)) observed)
    (set-window-buffer (selected-window) (get-buffer "*ack*"))
    (set-buffer "*ack*")
    (goto-char (point-min))
    (next-error 1 t)
    (push (list :first (ack-test-window-state)) observed)
    (next-error 1)
    (push (list :second (ack-test-window-state)) observed)
    (next-error 1)
    (push (list :third (ack-test-window-state)) observed)
    (next-error 1)
    (push (list :fourth (ack-test-window-state)) observed)
    (push (list :error-pos (with-current-buffer "*ack*" ack-error-pos)
                :last-buffer (buffer-name next-error-last-buffer))
          observed)
    (nreverse observed)))"##;
    let expect = expect![[
        r#"OK ((:n 18) (:n-n 63) (:p 18) (:next-file 37) (:match-after-file 63) (:visited ("readme.md" 19 2 9 "The café handler serves naïve clients.")) (:first ("CHANGELOG" 1 1 0 "Handler rewritten")) (:second ("readme.md" 19 2 9 "The café handler serves naïve clients.")) (:third ("main.el" 37 2 7 "(defun handler (request)")) (:fourth ("main.el" 67 3 12 "  (message \"handler ready\"))")) (:error-pos 147 :last-buffer "*ack*"))"#
    ]];

    assert_ack_menu_parity(elisp_form, expect);
}

/// The two "only print file names" switches are mutually exclusive, and
/// choosing either one drops the `--match' argument from the command line
/// entirely: `-fs' asks ack for the files it would search, so the results
/// buffer lists the tree, no text carries a match property and the run
/// reports zero matches.
#[test]
fn only_printing_file_names_replaces_the_match_argument() {
    let elisp_form = r##"(progn
  (ack-test-setup)
  (ack-test-restore-ansi-color-constants)
  (ack-test-open "src/main.el" "handler")
  (global-set-key (kbd "C-c a") 'ack-menu)
  (let ((mark (ack-test-message-mark)))
    (execute-kbd-macro (kbd "C-c a"))
    (execute-kbd-macro (kbd "- b p"))
    (execute-kbd-macro (kbd "- f m"))
    (let ((files-with-matches (ack-test-menu-state)))
      (execute-kbd-macro (kbd "- f s"))
      (let ((files-searched (ack-test-menu-state)))
        (execute-kbd-macro (kbd "r"))
        (ack-test-wait)
        (list :files-with-matches files-with-matches
              :files-searched files-searched
              :invocations (last (ack-test-invocations) 2)
              :text (ack-test-results-text)
              :segments (ack-test-results-segments)
              :options (ack-test-options)
              :messages (ack-test-messages-since mark))))))"##;
    let expect = expect![[
        r#"OK (:files-with-matches (("--ignore-case" "--files-with-matches") (("--directory=" . "./") ("--match=" . "handler"))) :files-searched (("--ignore-case" "-f") (("--directory=" . "./") ("--match=" . "handler"))) :invocations (("argv" "--color" "--nopager" "--ignore-case" "-f") ("cwd" ".")) :text "docs/CHANGELOG\ndocs/readme.md\nsrc/main.el\nsrc/notes with space.txt\n" :segments (("docs/CHANGELOG\ndocs/readme.md\nsrc/main.el\nsrc/notes with space.txt\n" nil nil nil nil)) :options (("--ignore-case") ("-f") ("--directory" . "./") ("--match" . "handler")) :messages ("Invalid face reference: widget-field [2 times]" "Type a prefix key to toggle it. Run ’actions’ with their prefixes. ’?’ for more help." "Invalid face reference: widget-field [6 times]" "Ack finished with 0 matches"))"#
    ]];

    assert_ack_menu_parity(elisp_form, expect);
}

/// `g' in the results buffer re-runs the last search from the same directory
/// with the same arguments, without going back through the menu.  The second
/// invocation has to be argument-for-argument identical to the first, and the
/// rendered results have to come out the same.
#[test]
fn ack_again_repeats_the_last_search_from_the_same_directory() {
    let elisp_form = r##"(progn
  (ack-test-setup)
  (ack-test-restore-ansi-color-constants)
  (ack-test-open "src/notes with space.txt" "handler")
  (global-set-key (kbd "C-c a") 'ack-menu)
  (execute-kbd-macro (kbd "C-c a"))
  (execute-kbd-macro (kbd "r"))
  (ack-test-wait)
  (let ((first (list :invocations (ack-test-invocations)
                     :text (ack-test-results-text))))
    (set-window-buffer (selected-window) (get-buffer "*ack*"))
    (set-buffer "*ack*")
    (let ((rerun-args (mapcar #'ack-test-relative ack-buffer--rerun-args)))
      (execute-kbd-macro (kbd "g"))
      (ack-test-wait)
      (list :first first
            :rerun-args rerun-args
            :invocations (ack-test-invocations)
            :text (ack-test-results-text)
            :state (ack-test-results-state)))))"##;
    let expect = expect![[
        r#"OK (:first (:invocations (("argv" "--color" "--nopager" "--ignore-case" "--match=handler") ("cwd" "src")) :text "main.el\n2:(defun handler (request)\n3:  (message \"handler ready\"))\n\nnotes with space.txt\n1:the handler notes\n\n") :rerun-args ("./src/" "--color" "--nopager" "--ignore-case" "--match=handler") :invocations (("argv" "--color" "--nopager" "--ignore-case" "--match=handler") ("cwd" "src") ("argv" "--color" "--nopager" "--ignore-case" "--match=handler") ("cwd" "src")) :text "main.el\n2:(defun handler (request)\n3:  (message \"handler ready\"))\n\nnotes with space.txt\n1:the handler notes\n\n" :state (:mode ack-mode :read-only t :directory "src/" :next-error-function ack-next-error-function :size 109))"#
    ]];

    assert_ack_menu_parity(elisp_form, expect);
}

/// The two failure paths.  A search that matches nothing leaves an empty
/// buffer, so the sentinel kills it and only the message remains.  And
/// `ack-arguments' -- documented as a list of strings -- is fed straight into
/// the alist the menu builds, so the documented value signals
/// `wrong-type-argument'; wrapped the way the code actually expects, the bad
/// option reaches ack, which fails with a non-zero exit and leaves its error
/// output in the results buffer.
#[test]
fn no_matches_kills_the_results_buffer_and_a_failing_ack_reports_its_error() {
    let elisp_form = r##"(progn
  (ack-test-setup)
  (ack-test-restore-ansi-color-constants)
  (ack-test-open "src/main.el" "handler")
  (let ((mark (ack-test-message-mark)))
    (ack-run-impl ack-test-root "--color" "--nopager" "--match=nothing-matches-this")
    (ack-test-wait)
    (let ((empty (list :buffer (get-buffer "*ack*")
                       :invocations (last (ack-test-invocations) 2)
                       :messages (ack-test-messages-since mark))))
      (list
       :no-matches empty
       :documented-type
       (let ((ack-arguments '("--bogus-option")))
         (condition-case error
             (ack-menu-action '(("--directory" . "./") ("--match" . "handler")))
           (error error)))
       :alist-type
       (let ((ack-arguments '(("--bogus-option"))))
         (ack-menu-action (list (cons "--directory" ack-test-root)
                                (cons "--match" "handler")))
         (ack-test-wait)
         (list :invocations (last (ack-test-invocations) 2)
               :text (ack-test-results-text)
               :segments (ack-test-results-segments)
               :state (ack-test-results-state)))))))"##;
    let expect = expect![[
        r#"OK (:no-matches (:buffer nil :invocations (("argv" "--color" "--nopager" "--match=nothing-matches-this") ("cwd" ".")) :messages ("Ack finished with 0 matches")) :documented-type (wrong-type-argument listp "--bogus-option") :alist-type (:invocations (("argv" "--color" "--nopager" "--bogus-option" "--match=handler") ("cwd" ".")) :text "ack: Unknown option: --bogus-option\n" :segments (("ack: Unknown option: --bogus-option\n" nil nil nil nil)) :state (:mode ack-mode :read-only t :directory "./" :next-error-function ack-next-error-function :size 36)))"#
    ]];

    assert_ack_menu_parity(elisp_form, expect);
}
