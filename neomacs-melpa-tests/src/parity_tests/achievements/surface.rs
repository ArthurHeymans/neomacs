use super::assert_achievements_parity;
use expect_test::expect;

#[test]
fn achievements_callable_surface_arglists_interactivity_docs_and_sources_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist
             symbol
             t)
            (commandp symbol)
            (let ((interactive
                   (interactive-form
                    symbol)))
              (if
                  (and
                   (consp interactive)
                   (let ((spec
                          (cadr
                           interactive)))
                     (or
                      (byte-code-function-p
                       spec)
                      (eq
                       (car-safe spec)
                       'byte-code))))
                  '(interactive
                    compiled)
                interactive))
            (and
             (documentation symbol))
            (file-name-nondirectory
             (symbol-file
              symbol
              'defun))))
         '(achievements-save-achievements
           achievements-load-achievements
           achievements-init
           achievements-variable-was-set
           achievements-num-times-commands-were-run
           achievements-command-was-run
           achievements-earned-message
           achievements-update-score
           achievements-earned-p
           achievements-get-achievements-by-name
           achievements-tabulated-list-entries
           achievements-disable
           achievements-list-mode
           achievements-list-achievements
           achievements-setup-post-command-hook
           achievements-post-command-function
           achievements-mode
           input-in-a-row))"##;
    let expect = expect![[
        r#"OK ((achievements-save-achievements nil t (interactive nil) "Saves achievements to a super secret file." "achievements-functions.el") (achievements-load-achievements nil t (interactive nil) "Load achievements from a super secret file.\nThis overwrites ‘achievements-list’." "achievements-functions.el") (achievements-init nil nil nil "Initialize achievements package." "achievements-functions.el") (achievements-variable-was-set (var) nil nil "If VAR is a cons, return non-nil if (car VAR) is equal to (cdr VAR).\nIf VAR is a symbol, return non-nil if VAR has been set in\ncustomize or .emacs (not yet implemented)." "achievements-functions.el") (achievements-num-times-commands-were-run (command-list) nil nil "Return the number of times any one of the commands was run.\nThis uses ‘keyfreq’, or ‘command-frequency’, or ‘command-history’\ndepending on what is installed." "achievements-functions.el") (achievements-command-was-run (command) nil nil "Return non-nil if COMMAND has been run.\nIt can be a single command form or list of command forms.\nEach form has one of the forms\n\n COMMAND -- must be run once\n (CMD1 CMD2 ...) -- all must be run\n ((CMD1 CMD2 ...)) -- any can be run\n (COMMAND . COUNT) -- must be run at least COUNT times\n ((CMD1 CMD2 ...) . COUNT) -- all must be run COUNT times\n (((CMD1 CMD2 ...)) . COUNT) -- any must be run COUNT times" "achievements-functions.el") (achievements-earned-message (achievement) nil nil "Display the message when an achievement is earned." "achievements-functions.el") (achievements-update-score nil nil nil "Recalculate whether each achievement has been earned." "achievements-functions.el") (achievements-earned-p (achievement) nil nil "Returns non-nil if the achievement is earned." "achievements-functions.el") (achievements-get-achievements-by-name (name) nil nil "Return the achievement identified by NAME." "achievements-functions.el") (achievements-tabulated-list-entries nil nil nil "Turn ‘achievements-list’ into a list for ‘tabulated-list-entries’." "achievements-functions.el") (achievements-disable nil t (interactive nil) "Disable the current achievement.\nThis expects to be called from ‘achievements-list-mode’." "achievements-functions.el") (achievements-list-mode nil t (interactive nil) "Mode for display the list of achievements.\n\nIn addition to any hooks its parent mode ‘tabulated-list-mode’ might\nhave run, this mode runs the hook ‘achievements-list-mode-hook’, as\nthe final or penultimate step during initialization.\n\n" "achievements-functions.el") (achievements-list-achievements nil t (interactive nil) "Display all achievements including whether they have been achieved." "achievements-functions.el") (achievements-setup-post-command-hook nil nil nil "Add the appropriate achievements for the post-command-hook." "achievements-functions.el") (achievements-post-command-function nil nil nil "Check achievements on ‘post-command-hook’." "achievements-functions.el") (achievements-mode (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Turns on automatic earning of achievements when idle.\n\nThis is a global minor mode.  If called interactively, toggle the\n‘Achievements mode’ mode.  If the prefix argument is positive, enable\nthe mode, and if it is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate ‘(default-value 'achievements-mode)’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled." "achievements-functions.el") (input-in-a-row (input val) nil nil nil "basic-achievements.el"))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_macro_surface_and_expanders_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (macrop symbol)
            (help-function-arglist
             symbol
             t)
            (and
             (documentation symbol)
             t)
            (file-name-nondirectory
             (symbol-file
              symbol
              'defun))))
         '(defachievement
           defcommand-achievements
           defvalue-achievements))"##;
    let expect = expect![[
        r#"OK ((defachievement t (name &rest body) nil "achievements-functions.el") (defcommand-achievements t (format-str body &rest arguments) nil "achievements-functions.el") (defvalue-achievements t (var format-str body &rest arguments) nil "achievements-functions.el"))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_struct_surface_defaults_predicate_accessors_and_copy_match() {
    let elisp_form = r##"(let* ((achievement
                  (make-achievement
                   "Fixture"
                   "Description"))
                 (copy
                  (copy-emacs-achievement
                   achievement)))
         (setf
          (emacs-achievement-description
           copy)
          "Copy")
         (list
          (mapcar
           (lambda (symbol)
             (list
              symbol
              (fboundp symbol)
              (help-function-arglist
               symbol
               t)
              (file-name-nondirectory
               (symbol-file
                symbol
                'defun))))
           '(emacs-achievement-p
             make-achievement
             copy-emacs-achievement
             emacs-achievement-name
             emacs-achievement-description
             emacs-achievement-predicate
             emacs-achievement-transient
             emacs-achievement-post-command
             emacs-achievement-points
             emacs-achievement-min-score
             emacs-achievement-unlocks))
          (emacs-achievement-p
           achievement)
          (emacs-achievement-p
           '(not an achievement))
          (list
           (emacs-achievement-name
            achievement)
           (emacs-achievement-description
            achievement)
           (functionp
            (emacs-achievement-predicate
             achievement))
           (emacs-achievement-transient
            achievement)
           (emacs-achievement-post-command
            achievement)
           (emacs-achievement-points
            achievement)
           (emacs-achievement-min-score
            achievement)
           (emacs-achievement-unlocks
            achievement))
          (list
           (emacs-achievement-description
            achievement)
           (emacs-achievement-description
            copy)
           (eq achievement copy))))"##;
    let expect = expect![[
        r#"OK (((emacs-achievement-p t #1=(x) "achievements-functions.el") (make-achievement t (name description &rest --cl-rest--) "achievements-functions.el") (copy-emacs-achievement t (arg) "achievements-functions.el") (emacs-achievement-name t #1# "achievements-functions.el") (emacs-achievement-description t #1# "achievements-functions.el") (emacs-achievement-predicate t #1# "achievements-functions.el") (emacs-achievement-transient t #1# "achievements-functions.el") (emacs-achievement-post-command t #1# "achievements-functions.el") (emacs-achievement-points t #1# "achievements-functions.el") (emacs-achievement-min-score t #1# "achievements-functions.el") (emacs-achievement-unlocks t #1# "achievements-functions.el")) t nil ("Fixture" "Description" t nil nil 5 0 nil) ("Description" "Copy" nil))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_variables_custom_metadata_hooks_and_source_ownership_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((standard
                  (get symbol
                       'standard-value)))
             (list
              symbol
              (cond
               ((eq symbol
                    'achievements-file)
                (file-name-nondirectory
                 (symbol-value
                  symbol)))
               ((eq symbol
                    'achievements-list-mode-map)
                (let ((map
                       (symbol-value
                        symbol)))
                  (list
                   (copy-tree map)
                   (keymap-parent
                    map))))
               ((eq symbol
                    'achievements-list-mode-syntax-table)
                (let ((table
                       (symbol-value
                        symbol)))
                  (list
                   (type-of table)
                   (char-table-subtype
                    table)
                   (eq
                    table
                    tabulated-list-mode-syntax-table)
                   (equal
                    table
                    tabulated-list-mode-syntax-table)
                   (and
                    (char-table-parent
                     table)
                    (type-of
                     (char-table-parent
                      table))))))
               ((eq symbol
                    'achievements-list-mode-abbrev-table)
                (let ((table
                       (symbol-value
                        symbol)))
                  (list
                   (type-of table)
                   (abbrev-table-p
                    table)
                   (let (symbols)
                     (mapatoms
                      (lambda (entry)
                        (push
                         (symbol-name entry)
                         symbols))
                      table)
                     (sort symbols
                           #'string<)))))
               ((eq symbol
                    'achievements-list)
                (length
                 (symbol-value
                  symbol)))
               ((eq symbol
                    'achievements--arrow-key-replacement-commands)
                (copy-sequence
                 (symbol-value
                  symbol)))
               (t
                (symbol-value
                 symbol)))
              (default-boundp symbol)
              (special-variable-p
               symbol)
              (list
               (and standard t)
               (and
                standard
                (eval
                 (car standard)
                 t)))
              (get symbol
                   'custom-type)
              (get symbol
                   'custom-group)
              (documentation-property
               symbol
               'variable-documentation
               t)
              (let ((file
                     (symbol-file
                      symbol
                      'defvar)))
                (and
                 file
                 (file-name-nondirectory
                  file))))))
         '(achievements-file
           achievements-list
           achievements-post-command-list
           achievements-score
           achievements-total
           achievements-debug
           achievements-list-mode-map
           achievements-list-mode-hook
           achievements-list-mode-syntax-table
           achievements-list-mode-abbrev-table
           achievements-timer
           achievements-display-when-earned
           achievements-idle-time
           achievements-mode
           achievements-mode-hook
           achievements--arrow-keys-needing-replacements
           achievements--arrow-key-replacement-commands))"##;
    let expect = expect![[
        r#"OK ((achievements-file ".achievements" t t (nil nil) nil nil "File to store the achievements in." "achievements-functions.el") (achievements-list 101 t t (nil nil) nil nil "List of all possible achievements." "achievements-functions.el") (achievements-post-command-list nil t t (nil nil) nil nil "List of achievements that need to be checked on `post-command-hook'." "achievements-functions.el") (achievements-score 0 t t (nil nil) nil nil "Score of all earned achievements." "achievements-functions.el") (achievements-total 0 t t (nil nil) nil nil "Highest possible score of all unlocked achievements." "achievements-functions.el") (achievements-debug nil t t (t nil) boolean nil "If non-nil, print debug messages regarding achievements activity." "achievements-functions.el") (achievements-list-mode-map ((keymap) nil) t t (nil nil) nil nil "Local keymap for `achievements-list-mode' buffers." "achievements-functions.el") (achievements-list-mode-hook nil t t (nil nil) nil nil "Hook run after entering `achievements-list-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "achievements-functions.el") (achievements-list-mode-syntax-table (char-table syntax-table nil t char-table) t t (nil nil) nil nil "Syntax table for `achievements-list-mode'." "achievements-functions.el") (achievements-list-mode-abbrev-table (obarray t ("")) t t (nil nil) nil nil "Abbrev table for `achievements-list-mode'." "achievements-functions.el") (achievements-timer nil t t (nil nil) nil nil "Holds the idle timer." "achievements-functions.el") (achievements-display-when-earned t t t (t t) boolean nil "If non-nil, print messages when achievements are earned." "achievements-functions.el") (achievements-idle-time 10 t t (t 10) number nil "Seconds for Emacs to be idle before checking if achievements have been earned." "achievements-functions.el") (achievements-mode nil t t (t nil) boolean nil "Non-nil if Achievements mode is enabled.\nSee the `achievements-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `achievements-mode'." "achievements-functions.el") (achievements-mode-hook nil t t (t nil) hook nil "Hook run after entering or leaving `achievements-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "achievements-functions.el") (achievements--arrow-keys-needing-replacements (right left up down) t t (nil nil) nil nil nil "basic-achievements.el") (achievements--arrow-key-replacement-commands (right-char left-char previous-line next-line) t t (nil nil) nil nil nil "basic-achievements.el"))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_customize_group_mode_and_derived_mode_metadata_match() {
    let elisp_form = r##"(list
         (get 'achievements
              'group-documentation)
         (copy-tree
          (get 'achievements
               'custom-group))
         (and
          (member
           '(achievements custom-group)
           (get 'games
                'custom-group))
          t)
         (get 'achievements-mode
              'custom-type)
         (let ((standard
                (get
                 'achievements-mode
                 'standard-value)))
           (list
            (and standard t)
            (and
             standard
             (eval
              (car standard)
              t))))
         (get 'achievements-mode
              'custom-group)
         (assq
          'achievements-mode
          minor-mode-alist)
         (get 'achievements-list-mode
              'derived-mode-parent)
         (get 'achievements-list-mode
              'mode-class))"##;
    let expect = expect![[
        r#"OK ("A set of (hopefully) fun achievements to learn Emacs." ((achievements-debug custom-variable) (achievements-display-when-earned custom-variable) (achievements-idle-time custom-variable) (achievements-mode custom-variable)) t boolean (t nil) nil (achievements-mode " Achieve") tabulated-list-mode nil)"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'achievements
                        package-alist)))
                     (directory
                      (package-desc-dir
                       descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally
                       path)
                      (list
                       name
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("achievements.el"
                  "achievements-functions.el"
                  "basic-achievements.el"
                  "advanced-achievements.el"
                  "ideas-achievements.el"
                  "achievements-pkg.el"
                  "achievements-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("achievements.el" 991 "8231c00b7984b649dc56f15c7d5bff5db8d3b200c22cb22a4bf07a51f106a4b8") ("achievements-functions.el" 17508 "8719dfeb447f6f4cddfc02e099a1d5c0da2e803fa3b97b27b780bc0378397ab8") ("basic-achievements.el" 13270 "1692cfc8f0d3421d76809aa6c91aedc7ce3359b3167492d1c7955761b922f65c") ("advanced-achievements.el" 4233 "e33ecf77699b49f099807491534772ba3c6ed75216800a1dabc1f1410f09d0e5") ("ideas-achievements.el" 11201 "c68cd035a8ffc160eb0df36d1a5e7daebc4e682bbb4b9956093e25863d29e568") ("achievements-pkg.el" 426 "5263ed4065805d45f5a1e18f31134c150a401d912c8468f31dbca69a5e5fbb41") ("achievements-autoloads.el" 1146 "f4fe9578a0c2d0c30ca013c0470dd7507ec52a6666309ea63307e7fddf8f06c1") ("README-elpa" 205 "b7ac7c1759ebf54dbc9bb8874f14953adca03377d3785c0fa70a1b4e9e4923c3"))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_installation_byte_compiles_every_packaged_lisp_source() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'achievements
                        package-alist)))
                     (directory
                      (package-desc-dir
                       descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (list
                     name
                     (file-exists-p path)
                     (file-regular-p path)
                     (>
                      (file-attribute-size
                       (file-attributes
                        path))
                      0))))
                '("achievements.elc"
                  "achievements-functions.elc"
                  "basic-achievements.elc"
                  "advanced-achievements.elc"
                  "ideas-achievements.elc")))"##;
    let expect = expect![[
        r#"OK (("achievements.elc" t t t) ("achievements-functions.elc" t t t) ("basic-achievements.elc" t t t) ("advanced-achievements.elc" t t t) ("ideas-achievements.elc" t t t))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}
