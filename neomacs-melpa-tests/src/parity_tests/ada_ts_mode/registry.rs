use expect_test::expect;

use super::{assert_ada_ts_mode_autoload_parity, assert_ada_ts_mode_parity};

#[test]
fn ada_ts_mode_exact_pin_metadata_version_features_group_and_dependencies_match() {
    let elisp_form = r##"(progn
         (require
          'lisp-mnt)
         (let ((descriptor
                (cadr
                 (assq
                  'ada-ts-mode
                  package-alist))))
           (list
            (package-desc-name
             descriptor)
            (package-version-join
             (package-desc-version
              descriptor))
            (package-desc-summary
             descriptor)
            (package-desc-kind
             descriptor)
            (package-desc-reqs
             descriptor)
            (package-desc-extras
             descriptor)
            (with-temp-buffer
              (insert-file-contents
               (getenv
                "NEOMACS_PACKAGE_SOURCE"))
              (lm-header
               "version"))
            (mapcar
             #'featurep
             '(ada-ts-als
               ada-ts-casing
               ada-ts-common
               ada-ts-imenu
               ada-ts-indentation
               ada-ts-lspclient
               ada-ts-mode
               treesit))
            (get
             'ada-ts
             'group-documentation)
            (get
             'ada-ts
             'custom-prefix)
            (and
             (member
              '(ada-ts custom-group)
              (get
               'languages
               'custom-group))
             t))))"##;
    let expect = expect![[
        r#"OK (ada-ts-mode "20260627.1553" "Major mode for Ada using Tree-sitter." nil ((emacs (29 1))) ((:maintainers ("Troy Brown" . "brownts@troybrown.dev")) (:authors ("Troy Brown" . "brownts@troybrown.dev")) (:keywords "ada" "languages" "tree-sitter") (:revdesc . "32fcf68dba74") (:commit . "32fcf68dba7463902481b256cdecad08e4b5b0a7") (:url . "https://github.com/brownts/ada-ts-mode")) nil (t t t t t t t t) "Major mode for Ada, using Tree-Sitter." "ada-ts-mode-" t)"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_complete_runtime_callable_command_and_documentation_surface_matches() {
    let elisp_form = r##"(let ((source-files
                '("ada-ts-als"
                  "ada-ts-casing"
                  "ada-ts-common"
                  "ada-ts-imenu"
                  "ada-ts-indentation"
                  "ada-ts-lspclient"
                  "ada-ts-mode"))
               symbols)
         (mapatoms
          (lambda (symbol)
            (when
                (and
                 (fboundp symbol)
                 (string-prefix-p
                  "ada-ts-"
                  (symbol-name
                   symbol))
                 (let ((file
                        (symbol-file
                         symbol
                         'defun)))
                   (and file
                        (member
                         (file-name-base
                          file)
                         source-files))))
              (push
               symbol
               symbols))))
         (mapcar
          (lambda (symbol)
            (let ((arglist
                   (help-function-arglist
                    symbol
                    t))
                  (interactive
                   (interactive-form
                    symbol))
                  (doc
                   (documentation
                    symbol
                    t)))
              (list
               symbol
               (mapcar
                (lambda (argument)
                  (if
                      (and
                       (symbolp argument)
                       (>
                        (length
                         (symbol-name
                          argument))
                        1)
                       (string-prefix-p
                        "_"
                        (symbol-name
                         argument)))
                      (intern
                       (substring
                        (symbol-name
                         argument)
                        1))
                    argument))
                arglist)
               (commandp
                symbol)
               (and
                interactive
                (list
                 'interactive
                 (let ((spec
                        (cadr
                         interactive)))
                   (cond
                    ((null spec)
                     nil)
                    ((stringp spec)
                     spec)
                    (t
                     'form)))))
               (let* ((doc
                       (replace-regexp-in-string
                        "\n\n(fn [^\n]*)\\'"
                        ""
                        doc))
                      (doc
                       (if
                           (eq
                            symbol
                            'ada-ts-auto-case-mode)
                           (car
                            (split-string
                             doc
                             "\n\n"))
                         doc))
                      (hash
                       (seq-reduce
                        (lambda (state character)
                          (mod
                           (+
                            (*
                             state
                             33)
                            character)
                           2147483647))
                        doc
                        5381)))
                 (list
                  (length
                   doc)
                  hash))
               (concat
                (file-name-base
                 (symbol-file
                  symbol
                  'defun))
                ".el"))))
          (sort
           symbols
           (lambda (left right)
             (string<
              (symbol-name
               left)
              (symbol-name
               right))))))"##;
    let expect = expect![[
        r#"OK ((ada-ts-als--format-region (client beg end indent-offset) nil nil (73 735162439) "ada-ts-als.el") (ada-ts-als--lsp-session-setup nil nil nil (297 719332340) "ada-ts-als.el") (ada-ts-als--project-file-absolute-path (project-file-path-or-uri) nil nil (55 1896099280) "ada-ts-als.el") (ada-ts-als--project-root nil nil nil (20 1540188563) "ada-ts-als.el") (ada-ts-als--read-json-file (file &optional false) nil nil (167 1140263703) "ada-ts-als.el") (ada-ts-als--uri-to-path (uri) nil nil (25 57481439) "ada-ts-als.el") (ada-ts-als-composite-config (&optional false) nil nil (148 279806408) "ada-ts-als.el") (ada-ts-als-executables nil nil nil (55 1176520099) "ada-ts-als.el") (ada-ts-als-find-user-config-file nil t (interactive nil) (33 1222923345) "ada-ts-als.el") (ada-ts-als-find-workspace-config-file nil t (interactive nil) (38 458992273) "ada-ts-als.el") (ada-ts-als-format-line (indent-offset) nil nil (426 1664360950) "ada-ts-als.el") (ada-ts-als-format-region (beg end indent-offset) nil nil (339 1150779877) "ada-ts-als.el") (ada-ts-als-get-project-attribute-value (attribute &optional package index) nil nil (362 962963690) "ada-ts-als.el") (ada-ts-als-mains nil nil nil (42 223749788) "ada-ts-als.el") (ada-ts-als-object-dir nil nil nil (53 518247947) "ada-ts-als.el") (ada-ts-als-other-file nil nil nil (61 1853446512) "ada-ts-als.el") (ada-ts-als-project-file nil nil nil (225 802086380) "ada-ts-als.el") (ada-ts-als-show-composite-config nil t (interactive nil) (423 331439427) "ada-ts-als.el") (ada-ts-als-source-dirs nil nil nil (38 1578618140) "ada-ts-als.el") (ada-ts-als-user-config-file nil nil nil (56 1526858311) "ada-ts-als.el") (ada-ts-als-workspace-config-file nil nil nil (61 1707496845) "ada-ts-als.el") (ada-ts-auto-case-mode (&optional arg) t (interactive form) (42 1245200550) "ada-ts-casing.el") (ada-ts-imenu nil nil nil (42 1040198291) "ada-ts-imenu.el") (ada-ts-imenu--index (tree item-p branch-p item-name-fn branch-name-fn) nil nil (444 1703822184) "ada-ts-imenu.el") (ada-ts-indent--after-change (beg end length) nil nil (363 1640432797) "ada-ts-indentation.el") (ada-ts-indent--electric-indent-p (&optional char) nil nil (143 462932211) "ada-ts-indentation.el") (ada-ts-indent--location-after-keyword-begin (node) nil nil (143 1710384576) "ada-ts-indentation.el") (ada-ts-indent--location-after-keyword-loop (node) nil nil (57 1777294319) "ada-ts-indentation.el") (ada-ts-indent--location-after-keyword-record (node) nil nil (59 810657758) "ada-ts-indentation.el") (ada-ts-indent--location-for-keyword-begin (node) nil nil (51 33513497) "ada-ts-indentation.el") (ada-ts-indent--maybe-electric-indent nil nil nil (35 624442853) "ada-ts-indentation.el") (ada-ts-indent--node-at-indentation-p (node) nil nil (28 943688036) "ada-ts-indentation.el") (ada-ts-indent--point-at-indentation (node) nil nil (48 117304110) "ada-ts-indentation.el") (ada-ts-indent--setup nil nil nil (29 865898456) "ada-ts-indentation.el") (ada-ts-lspclient-command-execute (client command &rest arguments) nil nil (53 939531365) "ada-ts-lspclient.el") (ada-ts-lspclient-command-supported-p (client command) nil nil (46 224970826) "ada-ts-lspclient.el") (ada-ts-lspclient-current nil nil nil (50 1849191725) "ada-ts-lspclient.el") (ada-ts-lspclient-document-id (client) nil nil (48 1715604236) "ada-ts-lspclient.el") (ada-ts-lspclient-format-region (client beg end) nil nil (47 1308366257) "ada-ts-lspclient.el") (ada-ts-lspclient-workspace-configuration (client scope &optional false) nil nil (603 389562783) "ada-ts-lspclient.el") (ada-ts-lspclient-workspace-dirs-add (client dirs) nil nil (30 487325567) "ada-ts-lspclient.el") (ada-ts-lspclient-workspace-root (client path) nil nil (34 94365612) "ada-ts-lspclient.el") (ada-ts-mode nil t (interactive nil) (241 1809403098) "ada-ts-mode.el") (ada-ts-mode--adjust-text-properties (value) nil nil (803 1311069003) "ada-ts-common.el") (ada-ts-mode--advice-treesit--indent-rules-optimize (oldfun &rest r) nil nil (400 223044466) "ada-ts-indentation.el") (ada-ts-mode--alire-project-file nil nil nil (49 966267320) "ada-ts-mode.el") (ada-ts-mode--anchor-best-effort nil nil nil (29 26412914) "ada-ts-indentation.el") (ada-ts-mode--anchor-catch-all nil nil nil (27 604872014) "ada-ts-indentation.el") (ada-ts-mode--anchor-first-sibling (type &rest types) nil nil (159 103742588) "ada-ts-indentation.el") (ada-ts-mode--anchor-first-sibling-bol (type &rest types) nil nil (159 103742588) "ada-ts-indentation.el") (ada-ts-mode--anchor-gp-skip-label-bol nil nil nil (63 289448844) "ada-ts-indentation.el") (ada-ts-mode--anchor-next-sibling-not-matching (type &rest types) nil nil (72 40325846) "ada-ts-indentation.el") (ada-ts-mode--anchor-prev-sibling (type) nil nil (38 378837273) "ada-ts-indentation.el") (ada-ts-mode--basic-declaration-p (node) nil nil (41 1585796127) "ada-ts-indentation.el") (ada-ts-mode--basic-declarative-item-p (node) nil nil (46 487101228) "ada-ts-indentation.el") (ada-ts-mode--case-dictionary-load (file) nil nil (21 1011368547) "ada-ts-casing.el") (ada-ts-mode--case-format-word (beg end formatter &optional dictionary) nil nil (196 964174199) "ada-ts-casing.el") (ada-ts-mode--case-format-word-try (_) nil nil (54 169343658) "ada-ts-casing.el") (ada-ts-mode--case-settings-process (symbol newval operation where) nil nil (410 2010677081) "ada-ts-casing.el") (ada-ts-mode--casing-prev-node (node) nil nil (58 614738985) "ada-ts-casing.el") (ada-ts-mode--compilation-unit-p (node) nil nil (40 150568189) "ada-ts-indentation.el") (ada-ts-mode--compound-statement-p (node) nil nil (42 1926680975) "ada-ts-indentation.el") (ada-ts-mode--declarative-item-p (node) nil nil (40 1466495751) "ada-ts-indentation.el") (ada-ts-mode--default-project-file nil nil nil (65 2103263261) "ada-ts-mode.el") (ada-ts-mode--defun-name (node &optional no-property) nil nil (164 201002327) "ada-ts-common.el") (ada-ts-mode--defun-p (node) nil nil (41 159530484) "ada-ts-common.el") (ada-ts-mode--do-keyword-anchor (node) nil nil (43 1070296400) "ada-ts-indentation.el") (ada-ts-mode--indent-best-effort (node parent bol) nil nil (60 1517643198) "ada-ts-indentation.el") (ada-ts-mode--indent-line nil nil nil (25 1027087557) "ada-ts-indentation.el") (ada-ts-mode--indent-recompute (symbol newval operation where) nil nil (488 1258778993) "ada-ts-indentation.el") (ada-ts-mode--indent-region (beg end) nil nil (47 584259153) "ada-ts-indentation.el") (ada-ts-mode--indent-verbosity-config (symbol newval operation where) nil nil (498 865213822) "ada-ts-indentation.el") (ada-ts-mode--is-keyword-anchor (node) nil nil (43 1969759226) "ada-ts-indentation.el") (ada-ts-mode--matching-prev-node (start matches) nil nil (265 286361021) "ada-ts-indentation.el") (ada-ts-mode--mismatched-names-p (node) nil nil (39 507880250) "ada-ts-indentation.el") (ada-ts-mode--mode-in-p (node) nil nil (35 1814042969) "ada-ts-mode.el") (ada-ts-mode--named-function-call-p (node) nil nil (221 37018572) "ada-ts-mode.el") (ada-ts-mode--named-procedure-call-p (node) nil nil (40 1444065091) "ada-ts-mode.el") (ada-ts-mode--next-leaf-node (start) nil nil (66 879356078) "ada-ts-indentation.el") (ada-ts-mode--next-node (start &optional include-comments) nil nil (92 1950552986) "ada-ts-indentation.el") (ada-ts-mode--next-sibling-not-matching (type &rest types) nil nil (47 1705216750) "ada-ts-indentation.el") (ada-ts-mode--next-sibling-not-matching-exists-p (type &rest types) nil nil (47 1705216750) "ada-ts-indentation.el") (ada-ts-mode--node-to-name (node &optional no-property) nil nil (108 1912637191) "ada-ts-common.el") (ada-ts-mode--not-matching-prev-node (start matches) nil nil (263 1961094641) "ada-ts-indentation.el") (ada-ts-mode--offset-best-effort nil nil nil (29 617954406) "ada-ts-indentation.el") (ada-ts-mode--offset-catch-all nil nil nil (27 1196413506) "ada-ts-indentation.el") (ada-ts-mode--offset-next-sibling-not-matching (type &rest types) nil nil (72 1270649300) "ada-ts-indentation.el") (ada-ts-mode--package-p (node) nil nil (96 2091552266) "ada-ts-common.el") (ada-ts-mode--prev-leaf-node (start) nil nil (67 690494856) "ada-ts-indentation.el") (ada-ts-mode--prev-node (start &optional include-comments) nil nil (93 1011019720) "ada-ts-indentation.el") (ada-ts-mode--prev-token (start) nil nil (63 911502152) "ada-ts-indentation.el") (ada-ts-mode--project-file nil nil nil (47 985750836) "ada-ts-mode.el") (ada-ts-mode--protected-p (node) nil nil (70 1114077442) "ada-ts-common.el") (ada-ts-mode--root-project-file nil nil nil (63 2086704042) "ada-ts-mode.el") (ada-ts-mode--simple-statement-p (node) nil nil (40 1954779873) "ada-ts-indentation.el") (ada-ts-mode--statement-p (node) nil nil (33 164315250) "ada-ts-indentation.el") (ada-ts-mode--subprogram-p (node) nil nil (99 383082370) "ada-ts-common.el") (ada-ts-mode--syntax-propertize (beg end) nil nil (519 306608784) "ada-ts-mode.el") (ada-ts-mode--task-p (node) nil nil (62 293640068) "ada-ts-common.el") (ada-ts-mode--then-keyword-anchor (node) nil nil (45 1360037709) "ada-ts-indentation.el") (ada-ts-mode--type-declaration-name (node) nil nil (41 1279840289) "ada-ts-common.el") (ada-ts-mode--type-declaration-p (node) nil nil (40 1305926592) "ada-ts-common.el") (ada-ts-mode--with-clause-name-p (node) nil nil (62 1510214213) "ada-ts-common.el") (ada-ts-mode-case-category-p (category node &optional last-input pos) nil nil (207 1557943518) "ada-ts-casing.el") (ada-ts-mode-case-format-at-point nil t (interactive nil) (31 1870291335) "ada-ts-casing.el") (ada-ts-mode-case-format-buffer nil t (interactive nil) (39 1454172833) "ada-ts-casing.el") (ada-ts-mode-case-format-dwim nil t (interactive nil) (36 1885747186) "ada-ts-casing.el") (ada-ts-mode-case-format-region (beg end) t (interactive "r") (55 1742000955) "ada-ts-casing.el") (ada-ts-mode-defun-comment-box nil t (interactive nil) (56 1198674398) "ada-ts-mode.el") (ada-ts-mode-fill-reindent-defun (&optional argument) t (interactive "P") (301 229118306) "ada-ts-mode.el") (ada-ts-mode-find-other-file nil t (interactive nil) (20 1375977657) "ada-ts-mode.el") (ada-ts-mode-find-project-file nil t (interactive nil) (23 476132785) "ada-ts-mode.el") (ada-ts-mode-imenu-nesting-strategy-before (item-name marker subtrees) nil nil (199 1451519890) "ada-ts-imenu.el") (ada-ts-mode-imenu-nesting-strategy-within (item-name marker subtrees) nil nil (245 1670241487) "ada-ts-imenu.el") (ada-ts-mode-imenu-sort-alphabetically (items) nil nil (33 1893215349) "ada-ts-imenu.el") (ada-ts-mode-indent (strategy) nil nil (57 1313826334) "ada-ts-indentation.el") (ada-ts-mode-indent-line (backend) nil nil (26 1734184358) "ada-ts-indentation.el") (ada-ts-mode-indent-region (backend beg end) nil nil (48 449833610) "ada-ts-indentation.el") (ada-ts-mode-menu (arg1) t (interactive "@e") (30 1768520927) "ada-ts-mode.el"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_auto_case_generated_documentation_keymap_and_signature_match() {
    let elisp_form = r##"(let* ((doc
                  (documentation
                   'ada-ts-auto-case-mode
                   t))
                 (hash
                  (seq-reduce
                   (lambda (state character)
                     (mod
                      (+
                       (*
                        state
                        33)
                       character)
                      2147483647))
                   doc
                   5381)))
         (list
          (car
           (split-string
            doc
            "\n\n"))
          (and
           (string-match-p
            "\\\\{ada-ts-auto-case-mode-map}"
            doc)
           t)
          (and
           (string-match-p
            "(fn &optional ARG)"
            doc)
           t)
          (length
           doc)
          hash))"##;
    let expect =
        expect![[r#"OK ("Minor mode for auto-casing in Ada buffers." t nil 632 1430096192)"#]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_complete_runtime_variable_metadata_surface_matches() {
    let elisp_form = r##"(let ((source-files
                '("ada-ts-als"
                  "ada-ts-casing"
                  "ada-ts-common"
                  "ada-ts-imenu"
                  "ada-ts-indentation"
                  "ada-ts-lspclient"
                  "ada-ts-mode"))
               symbols)
         (mapatoms
          (lambda (symbol)
            (when
                (and
                 (boundp symbol)
                 (string-prefix-p
                  "ada-ts-"
                  (symbol-name
                   symbol))
                 (let ((file
                        (symbol-file
                         symbol
                         'defvar)))
                   (and file
                        (member
                         (file-name-base
                          file)
                         source-files))))
              (push
               symbol
               symbols))))
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (default-boundp
              symbol)
             (local-variable-if-set-p
              symbol)
             (documentation-property
              symbol
              'variable-documentation
              t)
             (copy-tree
              (get
               symbol
               'custom-type))
             (get
              symbol
              'custom-group)
             (let ((safe
                    (get
                     symbol
                     'safe-local-variable)))
               (cond
                ((null safe)
                 nil)
                ((symbolp safe)
                 safe)
                ((functionp safe)
                 'function)
                (t
                 safe)))
             (get
              symbol
              'risky-local-variable)
             (concat
              (file-name-base
               (symbol-file
                symbol
                'defvar))
              ".el")))
          (sort
           symbols
           (lambda (left right)
             (string<
              (symbol-name
               left)
              (symbol-name
               right))))))"##;
    let expect = expect![[
        r#"OK ((ada-ts-als--config-verbose t nil nil nil nil nil nil "ada-ts-als.el") (ada-ts-auto-case-mode t t "Non-nil if Ada-Ts-Auto-Case mode is enabled.\nUse the command `ada-ts-auto-case-mode' to change this variable." nil nil nil nil "ada-ts-casing.el") (ada-ts-auto-case-mode-hook t nil "Hook run after entering or leaving `ada-ts-auto-case-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" hook nil nil nil "ada-ts-casing.el") (ada-ts-auto-case-mode-map t nil nil nil nil nil nil "ada-ts-casing.el") (ada-ts-indent--electric-indent-check-needed t t nil nil nil nil nil "ada-ts-indentation.el") (ada-ts-indent--electric-keywords t nil "Ada keywords which should trigger electric indentation.\n\nThe specified keywords are only considered if they are the only thing on\nthe line." nil nil nil t "ada-ts-indentation.el") (ada-ts-indent--electric-punctuation t nil "Ada punctuation which should trigger electric indentation.\n\nThe specified punctuation is only considered if it is entered at the end\nof the line." nil nil nil t "ada-ts-indentation.el") (ada-ts-indent--last-indent-tick t t nil nil nil nil nil "ada-ts-indentation.el") (ada-ts-lspclient-find-functions t nil "Special hook to find the LSP client for a given buffer.\n\nEach function on this hook is called in turn and should return either\nnil to mean that it is not applicable, or a client instance.  The exact\nform of the client instance is up to each respective function; the only\npractical limitation is to use values that `cl-defmethod' can dispatch\non." nil nil nil nil "ada-ts-lspclient.el") (ada-ts-lspclient-session-hook t nil "Hook called when an LSP session is established." nil nil nil nil "ada-ts-lspclient.el") (ada-ts-lspclient-setup-hook t nil "LSP client hooks to run when major mode is setup." nil nil nil nil "ada-ts-lspclient.el") (ada-ts-mode--case-dictionary-file-alist t nil nil nil nil nil nil "ada-ts-casing.el") (ada-ts-mode--case-formatting t nil nil nil nil nil nil "ada-ts-casing.el") (ada-ts-mode--casing-identifier-keywords-regex t nil nil nil nil nil nil "ada-ts-casing.el") (ada-ts-mode--casing-keyword-keywords-regex t nil nil nil nil nil nil "ada-ts-casing.el") (ada-ts-mode--font-lock-settings t nil "Font-lock settings for `ada-ts-mode'." nil nil nil nil "ada-ts-mode.el") (ada-ts-mode--indent-rules t nil "Tree-sitter indent rules for `ada-ts-mode'." nil nil nil nil "ada-ts-indentation.el") (ada-ts-mode--indent-verbose t nil "If non-nil, log process when indenting." nil nil nil nil "ada-ts-indentation.el") (ada-ts-mode--keywords t nil "Ada keywords for tree-sitter font-locking." nil nil nil nil "ada-ts-common.el") (ada-ts-mode--preproc-keywords t nil "Ada preprocessor keywords for tree-sitter font-locking." nil nil nil nil "ada-ts-mode.el") (ada-ts-mode-abbrev-table t nil "Abbrev table for `ada-ts-mode'." nil nil nil nil "ada-ts-mode.el") (ada-ts-mode-alire-program t nil "Name of Alire executable program." string nil nil t "ada-ts-mode.el") (ada-ts-mode-case-formatting t nil "Case formatting rules for casing commands and modes.\n\nEach rule should be of the form (CATEGORY . PROPS), where CATEGORY is\nthe category to which the formatting should be applied.  PROPS should\nhave the form:\n\n   [KEYWORD VALUE]...\n\nThe following keywords are meaningful:\n\n:formatter\n\n   VALUE must be a function which takes a string and returns the\n   formatted string.  This is a required property.\n\n:dictionary\n\n   Dictionary entries take precedence over the formatting function.\n   This is an optional property.\n\n   VALUE may be a list of strings whose exact casing is applied to\n   candidate words and subwords.\n\n   VALUE may also be a property list, having the form:\n\n      [KEYWORD VALUE]...\n\n   The following keywords are meaningful:\n\n   :words\n\n      VALUE must be a list of strings whose exact casing is applied to\n      candidate words and subwords.  This is an optional property.\n\n   :files\n\n      VALUE must be a list of files where the content of each file\n      contains a word or subword per line whose exact casing is applied\n      to candidate words and subwords.  This is an optional property." (alist :key-type (symbol :tag "Category") :value-type (plist :tag "Properties" :key-type symbol :options ((:formatter (choice :tag "Function" (function-item :tag "Mixed-Case (strict)" capitalize) (function-item :tag "Mixed-Case (loose)" upcase-initials) (function-item :tag "Upper-Case" upcase) (function-item :tag "Lower-Case" downcase) (function :tag "Custom"))) (:dictionary (choice :tag "Dictionary" (repeat :tag "Words" (string :tag "Word")) (plist :tag "Words/Files" :key-type symbol :options ((:words (repeat :tag "Words" (string :tag "Word"))) (:files (repeat :tag "Files" (file :tag "File")))))))))) nil function nil "ada-ts-casing.el") (ada-ts-mode-grammar t nil "Configuration for downloading and installing the tree-sitter language grammar.\n\nAdditional settings beyond the git repository can also be\nspecified.  See `treesit-language-source-alist' for full details." (choice (string :tag "Git Repository") (list :tag "All Options" (string :tag "Git Repository") (choice :tag "Revision" (const :tag "Default" nil) string) (choice :tag "Source Directory" (const :tag "Default" nil) string) (choice :tag "C Compiler" (const :tag "Default" nil) string) (choice :tag "C++ Compiler" (const :tag "Default" nil) string))) nil nil nil "ada-ts-mode.el") (ada-ts-mode-grammar-install t nil "Configuration for installation of tree-sitter language grammar library." (choice (const :tag "Automatically Install" auto) (const :tag "Prompt to Install" prompt) (const :tag "Do not install" nil)) nil nil nil "ada-ts-mode.el") (ada-ts-mode-hook t nil "Hook run after entering `ada-ts-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" nil nil nil nil "ada-ts-mode.el") (ada-ts-mode-imenu-categories t nil "Configuration of Imenu categories." (repeat :tag "Categories" (choice :tag "Category" (const :tag "Package" package) (const :tag "Subprogram" subprogram) (const :tag "Protected" protected) (const :tag "Task" task) (const :tag "Type Declaration" type-declaration) (const :tag "With Clause" with-clause))) nil nil nil "ada-ts-imenu.el") (ada-ts-mode-imenu-category-name-alist t nil "Configuration of Imenu category names." (alist :key-type symbol :value-type string) nil nil nil "ada-ts-imenu.el") (ada-ts-mode-imenu-nesting-strategy-function t nil "Configuration for Imenu nesting strategy function." (choice (const :tag "Place Before Nested Entries" ada-ts-mode-imenu-nesting-strategy-before) (const :tag "Place Within Nested Entries" ada-ts-mode-imenu-nesting-strategy-within) (function :tag "Custom function")) nil nil nil "ada-ts-imenu.el") (ada-ts-mode-imenu-nesting-strategy-placeholder t nil "Placeholder for an item used in some Imenu nesting strategies." string nil nil nil "ada-ts-imenu.el") (ada-ts-mode-imenu-sort-function t nil "Configuration for Imenu sorting function." (choice (const :tag "In Buffer Order" identity) (const :tag "Alphabetically" ada-ts-mode-imenu-sort-alphabetically) (function :tag "Custom function")) nil nil nil "ada-ts-imenu.el") (ada-ts-mode-indent-backend t nil "Backend used for indentation." (choice (const :tag "Tree-sitter" tree-sitter) (const :tag "Language Server" lsp)) nil symbolp nil "ada-ts-indentation.el") (ada-ts-mode-indent-broken-offset t nil "Indentation for the continuation of a broken line." integer nil nil nil "ada-ts-indentation.el") (ada-ts-mode-indent-exp-item-offset t nil "Indentation for the continuation of an expression." integer nil nil nil "ada-ts-indentation.el") (ada-ts-mode-indent-label-offset t nil "Indentation for block and loop statements containing a label." integer nil nil nil "ada-ts-indentation.el") (ada-ts-mode-indent-offset t nil "Indentation of statements." integer nil integerp nil "ada-ts-indentation.el") (ada-ts-mode-indent-record-offset t nil "Indentation of record definition in a type or representation clause." integer nil nil nil "ada-ts-indentation.el") (ada-ts-mode-indent-strategy t nil "Indentation strategy utilized with tree-sitter backend." (choice :tag "Indentation Strategy" (const :tag "Aggressive" aggressive) (const :tag "Line" line)) nil nil nil "ada-ts-indentation.el") (ada-ts-mode-indent-subprogram-is-offset t nil "Indentation of \\='is\\=' in a null procedure or expression function." integer nil nil nil "ada-ts-indentation.el") (ada-ts-mode-indent-when-offset t nil "Indentation of \\='when\\=' relative to \\='case\\='." integer nil nil nil "ada-ts-indentation.el") (ada-ts-mode-keymap-prefix t nil "Keymap prefix for `ada-ts-mode'." string nil nil nil "ada-ts-mode.el") (ada-ts-mode-map t nil "Keymap for `ada-ts-mode'." nil nil nil nil "ada-ts-mode.el") (ada-ts-mode-menu t nil "Menu keymap for `ada-ts-mode'." nil nil nil nil "ada-ts-mode.el") (ada-ts-mode-other-file-alist t nil "Ada file extension mapping for \\='find other file\\='." (repeat (list regexp (choice (repeat string) function))) nil nil nil "ada-ts-mode.el") (ada-ts-mode-syntax-table t nil "Syntax table for `ada-ts-mode'." nil nil nil nil "ada-ts-mode.el"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_all_core_custom_defaults_standard_values_types_groups_and_safety_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((standard
                  (get
                   symbol
                   'standard-value)))
             (list
              symbol
              (default-value
               symbol)
              (and standard
                   (eval
                    (car standard)
                    t))
              (copy-tree
               (get
                symbol
                'custom-type))
              (get
               symbol
               'custom-group)
              (let ((safe
                     (get
                      symbol
                      'safe-local-variable)))
                (cond
                 ((null safe)
                  nil)
                 ((symbolp safe)
                  safe)
                 ((functionp safe)
                  'function)
                 (t
                  safe)))
              (get
               symbol
               'risky-local-variable)
              (documentation-property
               symbol
               'variable-documentation
               t)
              (concat
               (file-name-base
                (symbol-file
                 symbol
                 'defvar))
               ".el"))))
         '(ada-ts-mode-alire-program
           ada-ts-mode-grammar
           ada-ts-mode-grammar-install
           ada-ts-mode-keymap-prefix
           ada-ts-mode-other-file-alist
           ada-ts-mode-case-formatting
           ada-ts-mode-imenu-categories
           ada-ts-mode-imenu-category-name-alist
           ada-ts-mode-imenu-nesting-strategy-function
           ada-ts-mode-imenu-nesting-strategy-placeholder
           ada-ts-mode-imenu-sort-function
           ada-ts-mode-indent-backend
           ada-ts-mode-indent-strategy
           ada-ts-mode-indent-offset
           ada-ts-mode-indent-when-offset
           ada-ts-mode-indent-broken-offset
           ada-ts-mode-indent-exp-item-offset
           ada-ts-mode-indent-subprogram-is-offset
           ada-ts-mode-indent-record-offset
           ada-ts-mode-indent-label-offset))"##;
    let expect = expect![[
        r#"OK ((ada-ts-mode-alire-program "alr" "alr" string nil nil t "Name of Alire executable program." "ada-ts-mode.el") (ada-ts-mode-grammar "https://github.com/briot/tree-sitter-ada" "https://github.com/briot/tree-sitter-ada" (choice (string :tag "Git Repository") (list :tag "All Options" (string :tag "Git Repository") (choice :tag "Revision" (const :tag "Default" nil) string) (choice :tag "Source Directory" (const :tag "Default" nil) string) (choice :tag "C Compiler" (const :tag "Default" nil) string) (choice :tag "C++ Compiler" (const :tag "Default" nil) string))) nil nil nil "Configuration for downloading and installing the tree-sitter language grammar.\n\nAdditional settings beyond the git repository can also be\nspecified.  See `treesit-language-source-alist' for full details." "ada-ts-mode.el") (ada-ts-mode-grammar-install prompt prompt (choice (const :tag "Automatically Install" auto) (const :tag "Prompt to Install" prompt) (const :tag "Do not install" nil)) nil nil nil "Configuration for installation of tree-sitter language grammar library." "ada-ts-mode.el") (ada-ts-mode-keymap-prefix "C-c" "C-c" string nil nil nil "Keymap prefix for `ada-ts-mode'." "ada-ts-mode.el") (ada-ts-mode-other-file-alist (("\\.ads\\'" . #1=((".adb"))) ("\\.adb\\'" . #2=((".ads"))) ("\\.1\\.ada\\'" . #3=((".2.ada"))) ("\\.2\\.ada\\'" . #4=((".1.ada"))) ("_\\.ada\\'" . #5=((".ada"))) ("\\.ada\\'" . #6=(("_.ada")))) (("\\.ads\\'" . #1#) ("\\.adb\\'" . #2#) ("\\.1\\.ada\\'" . #3#) ("\\.2\\.ada\\'" . #4#) ("_\\.ada\\'" . #5#) ("\\.ada\\'" . #6#)) (repeat (list regexp (choice (repeat string) function))) nil nil nil "Ada file extension mapping for \\='find other file\\='." "ada-ts-mode.el") (ada-ts-mode-case-formatting #7=((identifier :formatter upcase-initials :dictionary ("ASCII" "GNAT" "IO")) (keyword :formatter downcase)) #7# (alist :key-type (symbol :tag "Category") :value-type (plist :tag "Properties" :key-type symbol :options ((:formatter (choice :tag "Function" (function-item :tag "Mixed-Case (strict)" capitalize) (function-item :tag "Mixed-Case (loose)" upcase-initials) (function-item :tag "Upper-Case" upcase) (function-item :tag "Lower-Case" downcase) (function :tag "Custom"))) (:dictionary (choice :tag "Dictionary" (repeat :tag "Words" (string :tag "Word")) (plist :tag "Words/Files" :key-type symbol :options ((:words (repeat :tag "Words" (string :tag "Word"))) (:files (repeat :tag "Files" (file :tag "File")))))))))) nil function nil "Case formatting rules for casing commands and modes.\n\nEach rule should be of the form (CATEGORY . PROPS), where CATEGORY is\nthe category to which the formatting should be applied.  PROPS should\nhave the form:\n\n   [KEYWORD VALUE]...\n\nThe following keywords are meaningful:\n\n:formatter\n\n   VALUE must be a function which takes a string and returns the\n   formatted string.  This is a required property.\n\n:dictionary\n\n   Dictionary entries take precedence over the formatting function.\n   This is an optional property.\n\n   VALUE may be a list of strings whose exact casing is applied to\n   candidate words and subwords.\n\n   VALUE may also be a property list, having the form:\n\n      [KEYWORD VALUE]...\n\n   The following keywords are meaningful:\n\n   :words\n\n      VALUE must be a list of strings whose exact casing is applied to\n      candidate words and subwords.  This is an optional property.\n\n   :files\n\n      VALUE must be a list of files where the content of each file\n      contains a word or subword per line whose exact casing is applied\n      to candidate words and subwords.  This is an optional property." "ada-ts-casing.el") (ada-ts-mode-imenu-categories #8=(package subprogram protected task type-declaration with-clause) #8# (repeat :tag "Categories" (choice :tag "Category" (const :tag "Package" package) (const :tag "Subprogram" subprogram) (const :tag "Protected" protected) (const :tag "Task" task) (const :tag "Type Declaration" type-declaration) (const :tag "With Clause" with-clause))) nil nil nil "Configuration of Imenu categories." "ada-ts-imenu.el") (ada-ts-mode-imenu-category-name-alist #9=((package . "Package") (subprogram . "Subprogram") (protected . "Protected") (task . "Task") (type-declaration . "Type Declaration") (with-clause . "With Clause")) #9# (alist :key-type symbol :value-type string) nil nil nil "Configuration of Imenu category names." "ada-ts-imenu.el") (ada-ts-mode-imenu-nesting-strategy-function ada-ts-mode-imenu-nesting-strategy-before ada-ts-mode-imenu-nesting-strategy-before (choice (const :tag "Place Before Nested Entries" ada-ts-mode-imenu-nesting-strategy-before) (const :tag "Place Within Nested Entries" ada-ts-mode-imenu-nesting-strategy-within) (function :tag "Custom function")) nil nil nil "Configuration for Imenu nesting strategy function." "ada-ts-imenu.el") (ada-ts-mode-imenu-nesting-strategy-placeholder "<<parent>>" "<<parent>>" string nil nil nil "Placeholder for an item used in some Imenu nesting strategies." "ada-ts-imenu.el") (ada-ts-mode-imenu-sort-function identity identity (choice (const :tag "In Buffer Order" identity) (const :tag "Alphabetically" ada-ts-mode-imenu-sort-alphabetically) (function :tag "Custom function")) nil nil nil "Configuration for Imenu sorting function." "ada-ts-imenu.el") (ada-ts-mode-indent-backend tree-sitter tree-sitter (choice (const :tag "Tree-sitter" tree-sitter) (const :tag "Language Server" lsp)) nil symbolp nil "Backend used for indentation." "ada-ts-indentation.el") (ada-ts-mode-indent-strategy aggressive aggressive (choice :tag "Indentation Strategy" (const :tag "Aggressive" aggressive) (const :tag "Line" line)) nil nil nil "Indentation strategy utilized with tree-sitter backend." "ada-ts-indentation.el") (ada-ts-mode-indent-offset 3 3 integer nil integerp nil "Indentation of statements." "ada-ts-indentation.el") (ada-ts-mode-indent-when-offset 3 3 integer nil nil nil "Indentation of \\='when\\=' relative to \\='case\\='." "ada-ts-indentation.el") (ada-ts-mode-indent-broken-offset 2 2 integer nil nil nil "Indentation for the continuation of a broken line." "ada-ts-indentation.el") (ada-ts-mode-indent-exp-item-offset 0 0 integer nil nil nil "Indentation for the continuation of an expression." "ada-ts-indentation.el") (ada-ts-mode-indent-subprogram-is-offset 2 2 integer nil nil nil "Indentation of \\='is\\=' in a null procedure or expression function." "ada-ts-indentation.el") (ada-ts-mode-indent-record-offset 3 3 integer nil nil nil "Indentation of record definition in a type or representation clause." "ada-ts-indentation.el") (ada-ts-mode-indent-label-offset 3 3 integer nil nil nil "Indentation for block and loop statements containing a label." "ada-ts-indentation.el"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_keymap_menu_syntax_and_global_registration_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              ada-ts-mode-map
              (kbd key))))
          '("M-q"
            "C-c C-b"
            "C-c C-o"
            "C-c C-p"
            "C-c"
            "C-x"))
         (keymapp
          ada-ts-mode-menu)
         (mapcar
          (lambda (character)
            (let ((syntax
                   (with-syntax-table
                       ada-ts-mode-syntax-table
                     (char-syntax
                      character))))
              (list
               character
               syntax
               (char-to-string
                syntax))))
          '(?-
            ?=
            ?&
            ?|
            ?>
            ?'
            ?\\
            ?\n
            ?_
            ?\"))
         (seq-filter
          (lambda (entry)
            (eq
             (cdr entry)
             'ada-ts-mode))
          auto-mode-alist)
         (and
          (boundp
           'major-mode-remap-defaults)
          (assq
           'ada-mode
           major-mode-remap-defaults))
         (assq
          'ada-mode
          major-mode-remap-alist)
         (assoc
          'ada
          treesit-language-source-alist)
         (get
          'ada-ts-mode
          'derived-mode-parent)
         (get
          'ada-ts-mode
          'derived-mode-extra-parents))"##;
    let expect = expect![[
        r#"OK ((("M-q" ada-ts-mode-fill-reindent-defun) ("C-c C-b" ada-ts-mode-defun-comment-box) ("C-c C-o" ada-ts-mode-find-other-file) ("C-c C-p" ada-ts-mode-find-project-file) ("C-c" (keymap (16 . ada-ts-mode-find-project-file) (15 . ada-ts-mode-find-other-file) (2 . ada-ts-mode-defun-comment-box))) ("C-x" nil)) t ((45 46 ".") (61 46 ".") (38 46 ".") (124 46 ".") (62 46 ".") (39 46 ".") (92 46 ".") (10 62 ">") (95 95 "_") (34 34 "\"")) (("\\(?:\\.ad[abcs]\\)\\'" . ada-ts-mode)) (ada-mode . ada-ts-mode) nil (ada "https://github.com/briot/tree-sitter-ada") prog-mode (ada-mode))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_generated_autoload_surface_registers_without_loading_runtime() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (feature)
            (list
             feature
             (featurep
              feature)))
          '(ada-ts-mode
            ada-ts-casing
            ada-ts-indentation
            ada-ts-lspclient-eglot
            ada-ts-lspclient-lsp-mode))
         (mapcar
          (lambda (symbol)
            (let ((definition
                   (and
                    (fboundp symbol)
                    (symbol-function
                     symbol))))
              (list
               symbol
               (and
                definition
                t)
               (autoloadp
                definition)
               (and
                (autoloadp
                 definition)
                (nth
                 1
                 definition))
               (commandp
                symbol)
               (let ((interactive
                      (interactive-form
                       symbol)))
                 (and
                  interactive
                  (list
                   'interactive
                   (let ((spec
                          (cadr
                           interactive)))
                     (cond
                      ((null spec)
                       nil)
                      ((stringp spec)
                       spec)
                      (t
                       'form)))))))))
          '(ada-ts-mode
            ada-ts-auto-case-mode))
         (mapcar
          (lambda (symbol)
            (let ((safe
                   (get
                    symbol
                    'safe-local-variable)))
              (list
               symbol
               (cond
                ((null safe)
                 nil)
                ((symbolp safe)
                 safe)
                ((functionp safe)
                 'function)
                (t
                 safe)))))
          '(ada-ts-mode-case-formatting
            ada-ts-mode-indent-backend
            ada-ts-mode-indent-offset))
         (seq-filter
          (lambda (entry)
            (eq
             (cdr entry)
             'ada-ts-mode))
          auto-mode-alist)
         (and
          (boundp
           'major-mode-remap-defaults)
          (assq
           'ada-mode
           major-mode-remap-defaults))
         (assq
          'ada-mode
          major-mode-remap-alist))"##;
    let expect = expect![[
        r#"OK (((ada-ts-mode nil) (ada-ts-casing nil) (ada-ts-indentation nil) (ada-ts-lspclient-eglot nil) (ada-ts-lspclient-lsp-mode nil)) ((ada-ts-mode t t "ada-ts-mode" t (interactive nil)) (ada-ts-auto-case-mode t nil nil t (interactive form))) ((ada-ts-mode-case-formatting function) (ada-ts-mode-indent-backend symbolp) (ada-ts-mode-indent-offset integerp)) (("\\(?:\\.ad[abcs]\\)\\'" . ada-ts-mode)) (ada-mode . ada-ts-mode) nil)"#
    ]];
    assert_ada_ts_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_installed_package_inventory_names_and_source_sizes_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                 (cadr
                  (assq
                   'ada-ts-mode
                   package-alist)))
                (directory
                 (package-desc-dir
                  descriptor))
                (files
                 (sort
                  (directory-files
                   directory
                   nil
                   directory-files-no-dot-files-regexp)
                  #'string<)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (list
               file
               (file-regular-p
                path)
               (and
                (file-regular-p
                 path)
                (file-attribute-size
                 (file-attributes
                  path))))))
          files))"##;
    let expect = expect![[
        r#"OK (("README-elpa" t 277) ("ada-ts-als.el" t 15332) ("ada-ts-als.elc" t 11411) ("ada-ts-casing.el" t 21043) ("ada-ts-casing.elc" t 14259) ("ada-ts-common.el" t 11376) ("ada-ts-common.elc" t 8798) ("ada-ts-imenu.el" t 10419) ("ada-ts-imenu.elc" t 5699) ("ada-ts-indentation.el" t 86979) ("ada-ts-indentation.elc" t 52474) ("ada-ts-lspclient-eglot.el" t 11773) ("ada-ts-lspclient-eglot.elc" t 8418) ("ada-ts-lspclient-lsp-mode.el" t 8052) ("ada-ts-lspclient-lsp-mode.elc" t 5773) ("ada-ts-lspclient.el" t 3383) ("ada-ts-lspclient.elc" t 3113) ("ada-ts-mode-autoloads.el" t 2888) ("ada-ts-mode-pkg.el" t 449) ("ada-ts-mode.el" t 37313) ("ada-ts-mode.elc" t 29645) ("ada-ts-mode.info" t 45193) ("dir" t 653))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_installed_elisp_sources_and_generated_metadata_sha256_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                 (cadr
                  (assq
                   'ada-ts-mode
                   package-alist)))
                (directory
                 (package-desc-dir
                  descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (list
               file
               (file-attribute-size
                (file-attributes
                 path))
               (secure-hash
                'sha256
                path))))
          '("ada-ts-als.el"
            "ada-ts-casing.el"
            "ada-ts-common.el"
            "ada-ts-imenu.el"
            "ada-ts-indentation.el"
            "ada-ts-lspclient-eglot.el"
            "ada-ts-lspclient-lsp-mode.el"
            "ada-ts-lspclient.el"
            "ada-ts-mode.el"
            "ada-ts-mode-autoloads.el"
            "ada-ts-mode-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("ada-ts-als.el" 15332 "93835d0b5b98c4479ce152e2251cea5af11dfe23d1d19050efe1d409d97b7035") ("ada-ts-casing.el" 21043 "bf20a785655488b515522f3f9984f9bcec6fe5a44a60c7018f5c127ad313e50c") ("ada-ts-common.el" 11376 "308284f686d58f104311c796c8b0871b2d6ce6b0d3606bb107384f02a497642e") ("ada-ts-imenu.el" 10419 "b6f69a1923f3ed90b75b93d5cfaf4760fb7c4ec8de0b04390169a93d33013ff0") ("ada-ts-indentation.el" 86979 "cb2edb983f1b9a8b84563801cd9754ea1c60e31b9db0aae068b770a3ab56c3f7") ("ada-ts-lspclient-eglot.el" 11773 "c866a0121275a30287428b8430046a0e1604cebb7a93b1372f01fc625744837d") ("ada-ts-lspclient-lsp-mode.el" 8052 "4422c7d503a718f8d99aaebceb1934cea5f3ed3c74cee3bbc45b84e0257a5828") ("ada-ts-lspclient.el" 3383 "8cfdbad95495ee404566b4d2e1cc475321894c1518d743598210cd8f29444d96") ("ada-ts-mode.el" 37313 "e5a17bf59b85cdf5a7d4710ad654bba0f1d407b87f479230a91d5f35689316e0") ("ada-ts-mode-autoloads.el" 2888 "425af299e816d69206f268a3f15a7752d236657a1b6a752eee75fc628e946e06") ("ada-ts-mode-pkg.el" 449 "2cf080bbd42620e3e630ff4748e2511cfb53a6fbda2510fa350d857bb519fce1"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
