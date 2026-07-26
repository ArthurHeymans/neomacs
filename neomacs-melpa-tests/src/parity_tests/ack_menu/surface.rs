use super::{assert_ack_menu_parity, assert_ack_menu_with_prelude_parity};
use expect_test::expect;

#[test]
fn ack_menu_callable_surface_arglists_commands_interactivity_docs_and_sources_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((interactive
                  (interactive-form
                   symbol)))
             (list
              symbol
              (fboundp symbol)
              (help-function-arglist
               symbol t)
              (commandp symbol)
              (cond
               ((null interactive)
                nil)
               ((let ((spec
                       (cadr interactive)))
                  (or
                   (byte-code-function-p
                    spec)
                   (eq
                    (type-of spec)
                    'interpreted-function)))
                '(interactive
                  compiled))
               (t interactive))
              (documentation
               symbol)
              (let ((file
                     (symbol-file
                      symbol
                      'defun)))
                (and file
                     (file-name-nondirectory
                      file))))))
         '(ack-create-type
           ack-type-for-major-mode
           ack-guess-project-root
           ack-count-matches
           ack-sentinel
           ack-parse-sgr-fragment
           ack-parse-sgr-sequences
           ack-parse-sgr-sequences-finish
           ack-apply-faces
           ack-filter
           ack-abort
           ack-option
           ack-run-impl
           ack-version-string
           ack-uses-line-color
           ack-check-version
           ack-list-files
           ack--read
           ack--initial-contents-for-read
           ack--default-for-read
           ack--use-region-p
           ack-read-dir
           ack-xor
           ack-type
           ack-read-file
           ack-find-same-file
           ack-find-file
           ack-again
           ack--again-buffer-name
           ack-previous-property-value
           ack-property-beg
           ack-property-end
           ack-next-marker
           ack-previous-marker
           ack-next-match
           ack-previous-match
           ack-next-file
           ack-previous-file
           ack-next-error-function
           ack-create-marker
           ack--move-to-line
           ack-find-match
           ack-mode
           ack-buffer-major-mode
           ack-buffer-dir
           ack-get-current-word
           ack-menu-current-project-switch
           ack-menu-buffer-dir-switch
           ack-menu-buffer-project-dir-switch
           ack-menu-only-print-files-switch
           ack-menu
           ack-filter-args
           ack-form-args-list
           ack-process-args
           ack-menu-action))"##;
    let expect = expect![[
        r#"OK ((ack-create-type t (extensions) nil nil nil "ack-menu.el") (ack-type-for-major-mode t (mode) nil nil "Return the --type and --type-set arguments for major mode MODE." "ack-menu.el") (ack-guess-project-root t nil nil nil "A function to guess the project root directory.\nThis can be used in ‘ack-root-directory-functions’." "ack-menu.el") (ack-count-matches t nil nil nil "Count the matches printed by ‘ack’ in the current buffer." "ack-menu.el") (ack-sentinel t (proc result) nil nil nil "ack-menu.el") (ack-parse-sgr-fragment t (string &optional start) nil nil "Returns a pair of the form (string . sgr-fragment)" "ack-menu.el") (ack-parse-sgr-sequences t (string fn) nil nil "This function filters ansi escape codes (see\nhttp://en.wikipedia.org/wiki/ANSI_escape_code), specifically\nsearching for Select Graphic Rendition (sgr) sequences. Ack\ncolor-codes certain parts of the output (file names, line\nnumbers, and matches) using sgr sequences. By finding the sgr\nsequences we can easily extract the file names and line numbers\nof the matches, and apply Emacs faces to the output to colorize\nit however we want. Any ansi escape codes other than sgr\nsequences are removed from the string.\n\nThis function takes a new STRING of ack process output, and a\ncallback FN which is called with two parameters for every color\ncoded string it finds: the string and the sgr color code (of the\nform ‘1;33m’, or ‘30;43m’, etc). The color code will have already\nbeen removed from the string. The callback function should return\na string with the appropriate text properties added.\n\nack-parse-sgr-sequences will return a string with ansi escape\nsequences removed, and text properties added to the sgr-colored\nportions of the string. The returned string may not represent the\nentire input string, as some of the input string may be processed\nduring subsequent calls to ack-parse-sgr-sequences.\n\nThis function uses ack-parse-sgr-context to store temporary\nparsing data between calls to ack-parse-sgr-sequences while\nprocessing ack process output. When the ack process is finished,\nack-parse-sgr-sequences-finish must be called to finish\nprocessing the temporary parsing data and reset\nack-parse-sgr-context.\n\nThis function is inspired by ansi-color-apply, which\nunfortunately isn’t generic enough for us to use. This function\ndoes however use two values defined in ansi-color.el:\nansi-color-drop-regexp and ansi-color-regexp." "ack-menu.el") (ack-parse-sgr-sequences-finish t (fn) nil nil "This function finishes processing any remaining ack output\nremaining from previous calls to ack-parse-sgr-sequences. It\ntakes a callback function that should work the same as the\ncallback supplied to ack-parse-sgr-sequences. This function\nreturns a string representing the last of the ack process\noutput." "ack-menu.el") (ack-apply-faces t (string sgr-code) nil nil "The function passed to ack-parse-sgr-sequences to add our text\nproperties. The text properties that may be added:\n  - font-lock-face: The face to use for the text. One of\n    ack-line, ack-file, or ack-match.\n  - ack-line: The line number (as a string).\n  - ack-file: The file name.\n  - ack-match: Set to t if this string represents an ack match.\n  - mouse-face: Will be set to ‘highlight’ for matches.\n  - follow-line: Will be set to t for matches." "ack-menu.el") (ack-filter t (proc output) nil nil nil "ack-menu.el") (ack-abort t nil t (interactive nil) "Abort the running ‘ack’ process." "ack-menu.el") (ack-option t (name enabled) nil nil nil "ack-menu.el") (ack-run-impl t (directory &rest arguments) nil nil "Run ack in DIRECTORY with ARGUMENTS." "ack-menu.el") (ack-version-string t nil nil nil "Return the ack version string." "ack-menu.el") (ack-uses-line-color t nil nil nil nil "ack-menu.el") (ack-check-version t nil nil nil nil "ack-menu.el") (ack-list-files t (directory &rest arguments) nil nil nil "ack-menu.el") (ack--read t (regexp) nil nil nil "ack-menu.el") (ack--initial-contents-for-read t nil nil nil nil "ack-menu.el") (ack--default-for-read t nil nil nil nil "ack-menu.el") (ack--use-region-p t nil nil nil nil "ack-menu.el") (ack-read-dir t nil nil nil nil "ack-menu.el") (ack-xor t (a b) nil nil nil "ack-menu.el") (ack-type t nil nil nil nil "ack-menu.el") (ack-read-file t (prompt choices) nil nil nil "ack-menu.el") (ack-find-same-file t (&optional directory) t (interactive (list (ack-read-dir))) "Prompt to find a file found by ack in DIRECTORY." "ack-menu.el") (ack-find-file t (&optional directory) t (interactive (list (ack-read-dir))) "Prompt to find a file found by ack in DIRECTORY." "ack-menu.el") (ack-again t nil t (interactive nil) "Run the last ack search in the same directory." "ack-menu.el") (ack--again-buffer-name t nil nil nil nil "ack-menu.el") (ack-previous-property-value t (property pos) nil nil "Find the value of PROPERTY at or somewhere before POS." "ack-menu.el") (ack-property-beg t (pos property) nil nil "Move to the first char of consecutive sequence with PROPERTY set." "ack-menu.el") (ack-property-end t (pos property) nil nil "Move to the last char of consecutive sequence with PROPERTY set." "ack-menu.el") (ack-next-marker t (pos arg marker marker-name) nil nil nil "ack-menu.el") (ack-previous-marker t (pos arg marker marker-name) nil nil nil "ack-menu.el") (ack-next-match t (pos arg) t (interactive "d\np") "Move to the next match in the *ack* buffer." "ack-menu.el") (ack-previous-match t (pos arg) t (interactive "d\np") "Move to the previous match in the *ack* buffer." "ack-menu.el") (ack-next-file t (pos arg) t (interactive "d\np") "Move to the next file in the *ack* buffer." "ack-menu.el") (ack-previous-file t (pos arg) t (interactive "d\np") "Move to the previous file in the *ack* buffer." "ack-menu.el") (ack-next-error-function t (arg reset) nil nil nil "ack-menu.el") (ack-create-marker t (pos &optional force) nil nil nil "ack-menu.el") (ack--move-to-line t (line) nil nil nil "ack-menu.el") (ack-find-match t (pos) t (interactive (list (let ((posn (event-start last-input-event))) (set-buffer (window-buffer (posn-window posn))) (posn-point posn)))) "Jump to the match at POS." "ack-menu.el") (ack-mode t nil t (interactive nil) #("Major mode for ack output.\n\nThis mode runs the hook ‘ack-mode-hook’, as the final or penultimate\nstep during initialization.\n\n\nKey             Binding\n-------------------------------------------------------------------------------\nRET\11\11ack-find-match\ng\11\11ack-again\nn\11\11ack-next-match\np\11\11ack-previous-match\nr\11\11ack-again\n<mouse-2>\11ack-find-match\n\nM-n\11\11ack-next-file\nM-p\11\11ack-previous-file\n" 151 230 (face separator-line) 231 234 (font-lock-face help-key-binding face help-key-binding) 236 250 (help-args (ack-find-match) category help-function-button button (t)) 251 252 (font-lock-face help-key-binding face help-key-binding) 254 263 (help-args (ack-again) category help-function-button button (t)) 264 265 (font-lock-face help-key-binding face help-key-binding) 267 281 (help-args (ack-next-match) category help-function-button button (t)) 282 283 (font-lock-face help-key-binding face help-key-binding) 285 303 (help-args (ack-previous-match) category help-function-button button (t)) 304 305 (font-lock-face help-key-binding face help-key-binding) 307 316 (help-args (ack-again) category help-function-button button (t)) 317 326 (font-lock-face help-key-binding face help-key-binding) 327 341 (help-args (ack-find-match) category help-function-button button (t)) 343 346 (font-lock-face help-key-binding face help-key-binding) 348 361 (help-args (ack-next-file) category help-function-button button (t)) 362 365 (font-lock-face help-key-binding face help-key-binding) 367 384 (help-args (ack-previous-file) category help-function-button button (t))) "ack-menu.el") (ack-buffer-major-mode t (buffer) nil nil nil "ack-menu.el") (ack-buffer-dir t (buffer) nil nil nil "ack-menu.el") (ack-get-current-word t (default) nil nil nil "ack-menu.el") (ack-menu-current-project-switch t (option-name options) nil nil nil "ack-menu.el") (ack-menu-buffer-dir-switch t (option-name options) nil nil nil "ack-menu.el") (ack-menu-buffer-project-dir-switch t (option-name options) nil nil nil "ack-menu.el") (ack-menu-only-print-files-switch t (option-name options) nil nil nil "ack-menu.el") (ack-menu t nil t (interactive nil) "Invoke the ack menu. When finished, ack will be run with the\nspecified options." "ack-menu.el") (ack-filter-args t (args args-to-remove) nil nil nil "ack-menu.el") (ack-form-args-list t (args) nil nil nil "ack-menu.el") (ack-process-args t (args) nil nil nil "ack-menu.el") (ack-menu-action t (options) t (interactive nil) nil "ack-menu.el"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_variables_defaults_custom_metadata_docs_locality_and_sources_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (symbol-value
                   symbol))
                 (standard
                  (get symbol
                       'standard-value)))
             (list
              symbol
              (cond
               ((eq symbol
                    'ack-mode-default-type-alist)
                (list
                 (length value)
                 (car value)
                 (car
                  (last value))))
               ((eq symbol
                    'ack-mode-map)
                (list
                 (copy-tree value)
                 (keymap-parent
                  value)))
               ((eq symbol
                    'ack-menu-group)
                (list
                 (car value)
                 (length value)))
               ((eq symbol
                    'ack-mode-syntax-table)
                (cl-labels
                    ((digest
                      (table)
                      (with-temp-buffer
                        (let ((count
                               0))
                          (map-char-table
                           (lambda (range syntax)
                             (setq count
                                   (1+
                                    count))
                             (prin1
                              (list range
                                    syntax)
                              (current-buffer))
                             (insert
                              "\n"))
                           table)
                          (list
                           count
                           (secure-hash
                            'sha256
                            (current-buffer)))))))
                  (let ((fundamental
                         (with-temp-buffer
                           (fundamental-mode)
                           (syntax-table))))
                    (list
                     (type-of value)
                     (char-table-subtype
                      value)
                     (eq value
                         fundamental)
                     (equal value
                            fundamental)
                     (char-table-range
                      value nil)
                     (digest value)
                     (let ((parent
                            (char-table-parent
                             value)))
                       (list
                        (and parent
                             (type-of parent))
                        (eq parent
                            fundamental)
                        (equal parent
                               fundamental)
                        (and parent
                             (digest
                              parent))))))))
               ((eq symbol
                    'ack-mode-abbrev-table)
                (let ((fundamental
                       (with-temp-buffer
                         (fundamental-mode)
                         local-abbrev-table))
                      symbols)
                  (mapatoms
                   (lambda (entry)
                     (push
                      (list
                       (symbol-name entry)
                       (and
                        (boundp entry)
                        (symbol-value
                         entry))
                       (copy-tree
                        (symbol-plist
                         entry)))
                      symbols))
                   value)
                  (list
                   (type-of value)
                   (abbrev-table-p
                    value)
                   (eq value
                       fundamental)
                   (equal value
                          fundamental)
                   (sort symbols
                         (lambda (left right)
                           (string<
                            (car left)
                            (car right)))))))
               (t
                (copy-tree value)))
              (default-boundp
               symbol)
              (special-variable-p
               symbol)
              (local-variable-if-set-p
               symbol)
              (list
               (and standard t)
               (and standard
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
                (and file
                     (file-name-nondirectory
                      file))))))
         '(ack-executable
           ack-arguments
           ack-mode-type-alist
           ack-mode-extension-alist
           ack-display-buffer
           ack-root-directory-functions
           ack-project-root-file-patterns
           ack-prompt-for-directory
           ack-current-project-directory
           ack-pushy-match-prompt
           ack-mode-default-type-alist
           ack-mode-default-extension-alist
           ack-buffer-name
           ack-process
           ack-buffer--rerun-args
           ack-parse-sgr-context
           ack-directory-history
           ack-literal-history
           ack-regexp-history
           ack-error-pos
           ack-mode-map
           ack-mode-hook
           ack-mode-syntax-table
           ack-mode-abbrev-table
           ack-menu-group
           ack-menu-current-state
           ack-menu-options
           ack-menu-match-history))"##;
    let expect = expect![[
        r#"OK ((ack-executable nil t t nil (t nil) file nil "*The location of the ack executable." "ack-menu.el") (ack-arguments nil t t nil (t nil) (repeat (string)) nil "*The arguments to use when running ack." "ack-menu.el") (ack-mode-type-alist nil t t nil (t nil) (repeat (cons (symbol :tag "Major mode") (repeat (string :tag "ack type")))) nil "*Matches major modes to searched file types.\nThis overrides values in `ack-mode-default-type-alist'.  The car in each\nlist element is a major mode, the rest are strings representing values of\nthe --type argument used by `ack-same'." "ack-menu.el") (ack-mode-extension-alist nil t t nil (t nil) (repeat (cons (symbol :tag "Major mode") (repeat :tag "File extensions" (string :tag "extension")))) nil "*Matches major modes to searched file extensions.\nThis overrides values in `ack-mode-default-extension-alist'.  The car in\neach list element is a major mode, the rest is a list of file extensions\nthat that should be searched in addition to the type defined in\n`ack-mode-type-alist' by `ack-same'." "ack-menu.el") (ack-display-buffer t t t nil (t t) (choice (const :tag "Don't display" nil) (const :tag "Display immediately" t) (const :tag "Display when done" 'after)) nil "*Determines whether `ack' should display the result buffer.\nSpecial value 'after means display the buffer only after a successful search." "ack-menu.el") (ack-root-directory-functions (ack-guess-project-root) t t nil (t (ack-guess-project-root)) (repeat function) nil "*A list of functions used to find the ack base directory.\nThese functions are called until one returns a directory.  If successful,\n`ack' is run from that directory instead of `default-directory'.  The\ndirectory is verified by the user depending on `ack-promtp-for-directory'." "ack-menu.el") (ack-project-root-file-patterns (".project\\'" ".xcodeproj\\'" ".sln\\'" "\\`Project.ede\\'" "\\`.git\\'" "\\`.bzr\\'" "\\`_darcs\\'" "\\`.hg\\'") t t nil (t (".project\\'" ".xcodeproj\\'" ".sln\\'" "\\`Project.ede\\'" "\\`.git\\'" "\\`.bzr\\'" "\\`_darcs\\'" "\\`.hg\\'")) (repeat (string :tag "Regular expression")) nil "A list of project file patterns for `ack-guess-project-root'.\nEach element is a regular expression.  If a file matching either element is\nfound in a directory, that directory is assumed to be the project root by\n`ack-guess-project-root'." "ack-menu.el") (ack-prompt-for-directory nil t t nil (t nil) (choice (const :tag "Don't prompt" nil) (const :tag "Don't Prompt when guessed " unless-guessed) (const :tag "Prompt" t)) nil "*Determines whether `ack' asks the user for the root directory.\nIf this is 'unless-guessed, the value determined by\n`ack-root-directory-functions' is used without confirmation.  If it is\nnil, the directory is never confirmed." "ack-menu.el") (ack-current-project-directory nil t t nil (t nil) directory nil "*The current project directory, which will be available in the\nmenu as a switch." "ack-menu.el") (ack-pushy-match-prompt nil t t nil (t nil) boolean nil "Prompt for match as soon as ack-menu is run." "ack-menu.el") (ack-mode-default-type-alist (50 (actionscript-mode "actionscript") (yaml-mode "yaml")) t t nil (nil nil) nil nil "Default values for `ack-mode-type-alist', which see." "ack-menu.el") (ack-mode-default-extension-alist ((d-mode "d")) t t nil (nil nil) nil nil "Default values for `ack-mode-extension-alist', which see." "ack-menu.el") (ack-buffer-name "*ack*" t t nil (nil nil) nil nil nil "ack-menu.el") (ack-process nil t t nil (nil nil) nil nil nil "ack-menu.el") (ack-buffer--rerun-args nil t t nil (nil nil) nil nil nil "ack-menu.el") (ack-parse-sgr-context nil t t t (nil nil) nil nil "A dotted pair of the form (sgr-code . unfinished-string).\nBoth values are strings. This is used to store unfinished\ncolorized regions while parsing the ack output." "ack-menu.el") (ack-directory-history nil t t nil (nil nil) nil nil "Directories recently searched with `ack'." "ack-menu.el") (ack-literal-history nil t t nil (nil nil) nil nil "Strings recently searched for with `ack'." "ack-menu.el") (ack-regexp-history nil t t nil (nil nil) nil nil "Regular expressions recently searched for with `ack'." "ack-menu.el") (ack-error-pos nil t t t (nil nil) nil nil nil "ack-menu.el") (ack-mode-map ((keymap (114 . ack-again) (103 . ack-again) (27 keymap (112 . ack-previous-file) (110 . ack-next-file)) (112 . ack-previous-match) (110 . ack-next-match) (13 . ack-find-match) (mouse-2 . ack-find-match)) nil) t t nil (nil nil) nil nil "Keymap for `ack-mode'." "ack-menu.el") (ack-mode-hook nil t t nil (nil nil) nil nil "Hook run after entering `ack-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "ack-menu.el") (ack-mode-syntax-table (char-table syntax-table nil nil nil (394 "5c7922fd802c1d56981d410f398c05618e88ff8c9883dd29bbedd6c9cb34dc4d") (char-table t t (394 "5c7922fd802c1d56981d410f398c05618e88ff8c9883dd29bbedd6c9cb34dc4d"))) t t nil (nil nil) nil nil "Syntax table for `ack-mode'." "ack-menu.el") (ack-mode-abbrev-table (obarray t nil nil (("" nil (:abbrev-table-modiff 0)))) t t nil (nil nil) nil nil "Abbrev table for `ack-mode'." "ack-menu.el") (ack-menu-group (ack 5) t t nil (nil nil) nil nil nil "ack-menu.el") (ack-menu-current-state nil t t nil (nil nil) nil nil nil "ack-menu.el") (ack-menu-options (("--ignore-case")) t t nil (nil nil) nil nil nil "ack-menu.el") (ack-menu-match-history nil t t nil (nil nil) nil nil nil "ack-menu.el"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_faces_have_exact_specs_docs_groups_and_source_ownership() {
    let elisp_form = r##"(mapcar
         (lambda (face)
            (list
             face
            (and
             (facep face)
             t)
            (get face
                 'face-defface-spec)
            (get face
                 'face-documentation)
            (get face
                 'custom-group)
            (let ((file
                   (symbol-file
                    face
                    'defface)))
              (and file
                   (file-name-nondirectory
                    file)))))
         '(ack-separator
           ack-file
           ack-line
           ack-match))"##;
    let expect = expect![[
        r#"OK ((ack-separator t ((default (:foreground "gray50"))) "*Face for the group separator \"--\" in `ack' output." nil "ack-menu.el") (ack-file t ((((background dark)) (:foreground "green1")) (((background light)) (:foreground "green4"))) "*Face for file names in `ack' output." nil "ack-menu.el") (ack-line t ((((background dark)) (:foreground "LightGoldenrod")) (((background dark)) (:foreground "DarkGoldenrod"))) "*Face for line numbers in `ack' output." nil "ack-menu.el") (ack-match t ((default (:foreground "black")) (((background dark)) (:background "yellow")) (((background light)) (:background "yellow"))) "*Face for matched text in `ack' output." nil "ack-menu.el"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_group_and_debug_ignored_error_registrations_match() {
    let elisp_form = r##"(list
         (get 'ack-menu
              'group-documentation)
         (copy-tree
          (get 'ack-menu
               'custom-group))
         (and
          (member
           '(ack-menu custom-group)
           (get 'tools
                'custom-group))
          t)
         (and
          (member
           '(ack-menu custom-group)
           (get 'matching
                'custom-group))
          t)
         (cl-remove-if-not
          (lambda (value)
            (and
             (stringp value)
             (or
              (string-match-p
               "Moved"
               value)
              (string-match-p
               "File .* not found"
               value))))
          debug-ignored-errors))"##;
    let expect = expect![[
        r#"OK ("A front-end for ack." ((ack-executable custom-variable) (ack-arguments custom-variable) (ack-mode-type-alist custom-variable) (ack-mode-extension-alist custom-variable) (ack-display-buffer custom-variable) (ack-root-directory-functions custom-variable) (ack-project-root-file-patterns custom-variable) (ack-prompt-for-directory custom-variable) (ack-current-project-directory custom-variable) (ack-pushy-match-prompt custom-variable) (ack-separator custom-face) (ack-file custom-face) (ack-line custom-face) (ack-match custom-face)) t t ("^File .* not found$" "^Moved \\(back before fir\\|past la\\)st match$"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_packaged_source_descriptor_autoload_readme_and_bytecode_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'ack-menu
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
              (if
                  (equal
                   name
                   "ack-menu.elc")
                  (list
                   name
                   (file-exists-p path)
                   (file-regular-p path)
                   (and
                    (>
                     (nth
                      7
                      (file-attributes
                       path))
                     0)
                    t))
                (with-temp-buffer
                  (set-buffer-multibyte nil)
                  (insert-file-contents-literally
                   path)
                  (list
                   name
                   (file-exists-p path)
                   (file-regular-p path)
                   (buffer-size)
                   (secure-hash
                    'sha256
                    (current-buffer)))))))
          '("ack-menu.el"
            "ack-menu.elc"
            "ack-menu-autoloads.el"
            "ack-menu-pkg.el"
            "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ack-menu.el" t t 33686 "130a26d6e12e753eaaea1cfa9a87046ee4f3522f793b6c9684b44049bbe273b7") ("ack-menu.elc" t t t) ("ack-menu-autoloads.el" t t 1031 "6458a9c78ab6161272c6d7d588e0d4398642780b024b1f6628dbc326f2f333dd") ("ack-menu-pkg.el" t t 329 "fd616fc6821d18f7c12dce1b576d22c51ab2fec1fef67bf02ac4fa46029e355f") ("README-elpa" t t 262 "3c64b6cabd54e005d242a248a8acb41026c55eddc048bb53fbbf6f15267b7801"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_executable_initializer_prefers_ack_and_sets_matching_man_page() {
    let prelude = r##"(progn
         (defvar ack-menu-executable-find-calls nil)
         (fset
          'executable-find
          (lambda (command)
            (setq ack-menu-executable-find-calls
                  (append
                   ack-menu-executable-find-calls
                   (list command)))
            (and
             (equal
              command
              "ack")
             "/fixture/bin/ack")))
         (fset
          'file-truename
          (lambda (path)
            path)))"##;
    let elisp_form = r##"(list
         ack-executable
         ack-menu-executable-find-calls
         (cadr
          (assq
           'man-page
           (cdr ack-menu-group))))"##;
    let expect = expect![[r#"OK ("/fixture/bin/ack" ("ack") "ack")"#]];
    assert_ack_menu_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ack_menu_executable_initializer_falls_back_to_ack_grep_and_sets_matching_man_page() {
    let prelude = r##"(progn
         (defvar ack-menu-executable-find-calls nil)
         (fset
          'executable-find
          (lambda (command)
            (setq ack-menu-executable-find-calls
                  (append
                   ack-menu-executable-find-calls
                   (list command)))
            (and
             (equal
              command
              "ack-grep")
             "/fixture/bin/ack-grep")))
         (fset
          'file-truename
          (lambda (path)
            path)))"##;
    let elisp_form = r##"(list
         ack-executable
         ack-menu-executable-find-calls
         (cadr
          (assq
           'man-page
           (cdr ack-menu-group))))"##;
    let expect = expect![[r#"OK ("/fixture/bin/ack-grep" ("ack" "ack-grep") "ack-grep")"#]];
    assert_ack_menu_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ack_menu_executable_initializer_records_both_misses_and_omits_man_page() {
    let prelude = r##"(progn
         (defvar ack-menu-executable-find-calls nil)
         (fset
          'executable-find
          (lambda (command)
            (setq ack-menu-executable-find-calls
                  (append
                   ack-menu-executable-find-calls
                   (list command)))
            nil)))"##;
    let elisp_form = r##"(list
         ack-executable
         ack-menu-executable-find-calls
         (cadr
          (assq
           'man-page
           (cdr ack-menu-group))))"##;
    let expect = expect![[r#"OK (nil ("ack" "ack-grep") nil)"#]];
    assert_ack_menu_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ack_menu_runtime_and_autoload_source_descriptors_resolve_exact_files() {
    let elisp_form = r##"(list
         (file-name-nondirectory
          (locate-library
           "ack-menu"))
         (let ((file
                (symbol-file
                 'ack-menu
                 'defun)))
           (and file
                (file-name-nondirectory
                 file)))
         (file-name-nondirectory
          (locate-library
           "ack-menu-autoloads"))
         (featurep
          'ack-menu)
         (featurep
          'ack-menu-autoloads))"##;
    let expect = expect![[r#"OK ("ack-menu.el" "ack-menu.el" "ack-menu-autoloads.el" t t)"#]];
    assert_ack_menu_parity(elisp_form, expect);
}
