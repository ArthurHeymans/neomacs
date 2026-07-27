use expect_test::expect;

use super::{assert_acton_mode_autoload_parity, assert_acton_mode_parity};

#[test]
fn acton_mode_exact_pin_metadata_version_feature_group_and_dependencies_match() {
    let elisp_form = r##"(progn
         (require
          'lisp-mnt)
         (let* ((descriptor
                  (cadr
                   (assq
                    'acton-mode
                    package-alist)))
                 (requirements
                  (package-desc-reqs
                   descriptor)))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-summary descriptor)
          (package-desc-kind descriptor)
          requirements
          (package-desc-extras descriptor)
          (featurep
           'acton-mode)
          (boundp
           'acton-mode-version)
          (with-temp-buffer
            (insert-file-contents
             (getenv
              "NEOMACS_PACKAGE_SOURCE"))
            (lm-header
             "version"))
          (get
           'acton
           'custom-group)
          (get
           'acton
           'group-documentation)
          (get
           'acton
           'custom-prefix)
          (and
           (member
            '(acton custom-group)
            (get
             'languages
             'custom-group))
           t))))"##;
    let expect = expect![[
        r#"OK (acton-mode "20250113.1059" "Major mode for editing Acton source code." nil ((emacs (25 1))) ((:keywords "languages" "programming") (:revdesc . "5a1a8509fb84") (:commit . "5a1a8509fb84dad4f8a02da47519ed7399c26d7f") (:url . "https://github.com/actonlang/acton-mode")) t nil nil ((acton-indent-offset custom-variable)) "Major mode for editing Acton source code." "acton-" t)"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_keyword_category_variables_values_docs_locality_and_sources_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (symbol-value symbol)
            (default-boundp symbol)
            (local-variable-if-set-p
             symbol)
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
                    file)))))
         '(acton-keywords
           acton-declarations
           acton-constants
           acton-effects
           acton-builtin-functions
           acton-decorators))"##;
    let expect = expect![[
        r#"OK ((acton-keywords ("if" "elif" "else" "while" "for" "in" "try" "except" "finally" "with" "return" "break" "continue" "pass" "raise" "yield" "from" "import" "as" "global" "nonlocal" "assert" "await" "async" "del" "lambda") t nil nil "acton-mode.el") (acton-declarations ("def" "class" "actor" "protocol" "extension" "var") t nil nil "acton-mode.el") (acton-constants ("True" "False" "None" "NotImplemented" "...") t nil nil "acton-mode.el") (acton-effects ("proc" "mut" "pure" "action") t nil nil "acton-mode.el") (acton-builtin-functions ("isinstance") t nil nil "acton-mode.el") (acton-decorators ("@property" "@staticmethod" "@static") t nil nil "acton-mode.el"))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_indent_custom_default_type_safety_group_and_source_match() {
    let elisp_form = r##"(list
         acton-indent-offset
         (default-value
          'acton-indent-offset)
         (let ((standard
                (get
                 'acton-indent-offset
                 'standard-value)))
           (list
            (and standard t)
            (and standard
                 (eval
                  (car standard)
                  t))))
         (get
          'acton-indent-offset
          'custom-type)
         (get
          'acton-indent-offset
          'safe-local-variable)
         (get
          'acton-indent-offset
          'custom-group)
         (documentation-property
          'acton-indent-offset
          'variable-documentation
          t)
         (file-name-nondirectory
          (symbol-file
           'acton-indent-offset
           'defvar)))"##;
    let expect = expect![[
        r#"OK (4 4 (t 4) integer integerp nil "Number of spaces for each indentation step in `acton-mode'." "acton-mode.el")"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_complete_callable_command_and_generated_mode_surface_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((doc
                  (documentation
                   symbol
                   t)))
             (list
              symbol
              (fboundp symbol)
              (help-function-arglist
               symbol
               t)
              (commandp symbol)
              (interactive-form
               symbol)
              doc
              (let ((file
                     (symbol-file
                      symbol
                      'defun)))
                (and file
                     (file-name-nondirectory
                      file))))))
         '(acton-indent-line
           acton-calculate-indentation
           acton-handle-colon
           acton-mode))"##;
    let expect = expect![[
        r#"OK ((acton-indent-line t nil t (interactive nil) "Indent current line as Acton code." "acton-mode.el") (acton-calculate-indentation t nil nil nil "Calculate the indentation for the current line." "acton-mode.el") (acton-handle-colon t nil nil nil "Handle colon insertion for auto-indentation.\nDe-indents else/elif/except/finally lines when colon is typed." "acton-mode.el") (acton-mode t nil t (interactive nil) "Major mode for editing Acton source code.\n\nIn addition to any hooks its parent mode `prog-mode' might have run,\nthis mode runs the hook `acton-mode-hook', as the final or penultimate\nstep during initialization.\n\n\\{acton-mode-map}" "acton-mode.el"))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_generated_variables_tables_hooks_maps_and_registration_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (boundp symbol)
             (and
              (boundp symbol)
              (let ((value
                     (default-value
                      symbol)))
                (cond
                 ((keymapp value)
                  (let ((count 0))
                    (map-keymap
                     (lambda
                       (_event _binding)
                       (setq count
                             (1+
                              count)))
                     value)
                    (list
                     'keymap
                     count)))
                 ((abbrev-table-p value)
                  (list
                   'abbrev-table
                   (abbrev-table-empty-p
                    value)))
                 ((syntax-table-p value)
                  'syntax-table)
                 (t value))))
             (local-variable-if-set-p
              symbol)
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
                     file)))))
          '(acton-mode-hook
            acton-mode-map
            acton-mode-abbrev-table
            acton-mode-syntax-table))
         (derived-mode-p
          'prog-mode)
         (get
          'acton-mode
          'derived-mode-parent)
         (get
          'acton-mode
          'mode-class)
         (rassq
          'acton-mode
          auto-mode-alist)
         (length
          (seq-filter
           (lambda (entry)
             (equal entry
                    '("\\.act\\'" . acton-mode)))
           auto-mode-alist)))"##;
    let expect = expect![[
        r#"OK (((acton-mode-hook t nil nil "Hook run after entering `acton-mode'.\nNo problems result if this variable is not bound.\n`add-hook' automatically binds it.  (This is true for all hook variables.)" "acton-mode.el") (acton-mode-map t (keymap 0) nil "Keymap for `acton-mode'." "acton-mode.el") (acton-mode-abbrev-table t (abbrev-table t) nil "Abbrev table for `acton-mode'." "acton-mode.el") (acton-mode-syntax-table t syntax-table nil nil "acton-mode.el")) prog-mode prog-mode nil ("\\.act\\'" . acton-mode) 1)"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_generated_autoload_surface_registers_without_loading_runtime() {
    let elisp_form = r##"(let ((before
                (list
                 (featurep
                  'acton-mode)
                 (featurep
                  'acton-mode-autoloads)
                 (fboundp
                  'acton-mode)
                 (autoloadp
                  (symbol-function
                   'acton-mode))
                 (rassq
                  'acton-mode
                  auto-mode-alist))))
         (list
          before
          (help-function-arglist
           'acton-mode
           t)
          (commandp
           'acton-mode)
          (featurep
           'acton-mode)
          (autoloadp
           (symbol-function
            'acton-mode))))"##;
    let expect = expect![[
        r#"OK ((nil t t t ("\\.act\\'" . acton-mode)) "[Arg list not available until function definition is loaded.]" t nil t)"#
    ]];
    assert_acton_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn acton_mode_installed_package_inventory_and_content_hashes_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'acton-mode
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor))
                 (names
                  (sort
                   (directory-files
                    directory
                    nil
                    "^[^.].*")
                   #'string<)))
         (list
          names
          (mapcar
           (lambda (name)
             (let ((path
                    (expand-file-name
                     name
                     directory)))
               (list
                name
                (file-regular-p path)
                (if
                    (string-suffix-p
                     ".elc"
                     name)
                    t
                  (with-temp-buffer
                    (insert-file-contents-literally
                     path)
                    (list
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer))))))))
           names)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "acton-mode-autoloads.el" "acton-mode-pkg.el" "acton-mode.el" "acton-mode.elc") (("README-elpa" t (56 "a38c9383867706a11eef293102a650734bc4d93f28795485a3daab7e10cbe154")) ("acton-mode-autoloads.el" t (1020 "0af0981ef08a6f52325f85f828de25bd32b3ea424426c0df600a48b18491a953")) ("acton-mode-pkg.el" t (333 "c70b68e213249d4a0d124fa06c9e78b854b5db1f9f7d0d50a08a4c13a5e2c515")) ("acton-mode.el" t (13442 "e38d9e04853fc986123fb775bb37c49d04a14a48657784674607e1e70ad6a869")) ("acton-mode.elc" t t)))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_direct_reload_preserves_defvars_customization_and_registration() {
    let elisp_form = r##"(let ((source
                (getenv
                 "NEOMACS_PACKAGE_SOURCE"))
               (custom-table
                (make-syntax-table))
               (original-font-lock
                acton-font-lock-keywords))
         (setq acton-keywords
               '("user-keyword")
               acton-indent-offset
               9
               acton-mode-syntax-table
               custom-table
               acton-font-lock-keywords
               'user-font-lock)
         (load source nil t t)
         (list
          acton-keywords
          acton-indent-offset
          (eq
           acton-mode-syntax-table
           custom-table)
          (eq
           acton-font-lock-keywords
           original-font-lock)
          acton-font-lock-keywords
          (length
           (seq-filter
            (lambda (entry)
              (equal entry
                     '("\\.act\\'" . acton-mode)))
            auto-mode-alist))
          (featurep
           'acton-mode)))"##;
    let expect = expect![[
        r##"OK (("user-keyword") 9 t nil (("\\(?:\"\"\"[\\s\\S]*?\"\"\"\\|'''[\\s\\S]*?'''\\)" 0 font-lock-string-face) ("\\<r\\(?:\"[^\"\\]*\\(?:\\\\.[^\"\\]*\\)*\"\\|'[^'\\]*\\(?:\\\\.[^'\\]*\\)*'\\)" 0 font-lock-string-face) ("\\<b\\(?:\"[^\"\\]*\\(?:\\\\.[^\"\\]*\\)*\"\\|'[^'\\]*\\(?:\\\\.[^'\\]*\\)*'\\)" 0 font-lock-string-face) ("\\(?:\"[^\"\\]*\\(?:\\\\.[^\"\\]*\\)*\"\\|'[^'\\]*\\(?:\\\\.[^'\\]*\\)*'\\)" 0 font-lock-string-face) ("\\\\[abfnrtv'\"\\\\]\\|\\\\[0-7]\\{1,3\\}\\|\\\\x[[:xdigit:]]\\{2\\}\\|\\\\u[[:xdigit:]]\\{4\\}\\|\\\\U[[:xdigit:]]\\{8\\}" 0 font-lock-escape-face t) ("\\<\\(?:[0-9]+\\.[0-9]*\\|[0-9]*\\.[0-9]+\\)\\(?:[eE][+-]?[0-9]+\\)?[jJ]\\>" . font-lock-constant-face) ("\\<[0-9]+[jJ]\\>" . font-lock-constant-face) ("\\<0x[0-9a-fA-F]+\\>" . font-lock-constant-face) ("\\<0o[0-7]+\\>" . font-lock-constant-face) ("\\<\\(?:[0-9]+\\.[0-9]*\\|[0-9]*\\.[0-9]+\\)\\(?:[eE][+-]?[0-9]+\\)?\\>" . font-lock-constant-face) ("\\<[0-9]+\\>" . font-lock-constant-face) ("[-+*/%@]\\|//\\|\\*\\*" . font-lock-builtin-face) ("[&|^~]\\|<<\\|>>" . font-lock-builtin-face) ("==\\|!=\\|<=\\|>=\\|<\\|>" . font-lock-builtin-face) ("\\(?:[-+*/%&|^]\\|//\\|\\*\\*\\|<<\\|>>\\)=" . font-lock-builtin-face) ("\\_<\\(?:is\\s-+not\\|not\\s-+in\\)\\_>" . font-lock-builtin-face) ("\\(?:->\\|=>\\)" . font-lock-builtin-face) ("\\_<[A-Z]\\d*\\_>" . font-lock-type-face) (":\\s-*\\([A-Z][a-zA-Z0-9_]*\\(?:\\[[^]]*\\]\\)?\\)" 1 font-lock-type-face) ("^[ \11]*\\(?:@\\(?:property\\|static\\(?:method\\)?\\)\\)[ \11]*\\(?:$\\|[^[:alnum:]_]\\)" . font-lock-preprocessor-face) ("\\_<\\(user-keyword\\)\\_>" . font-lock-keyword-face) ("\\_<\\(actor\\|class\\|def\\|extension\\|protocol\\|var\\)\\_>" . font-lock-keyword-face) ("\\_<\\(\\.\\.\\.\\|False\\|No\\(?:ne\\|tImplemented\\)\\|True\\)\\_>" . font-lock-constant-face) ("\\_<\\(action\\|mut\\|p\\(?:roc\\|ure\\)\\)\\_>" . font-lock-builtin-face) ("\\_<\\(isinstance\\)\\_>" . font-lock-builtin-face) ("\\<def\\>[ \11]+\\([a-zA-Z_][a-zA-Z0-9_]*\\)" (1 font-lock-function-name-face)) (":[ \11]*\\([A-Z][a-zA-Z0-9_]*\\)" (1 font-lock-type-face)) ("\\<\\(?:class\\|protocol\\|extension\\)\\>[ \11]+\\([A-Z][a-zA-Z0-9_]*\\)" (1 font-lock-type-face)) ("\\<actor\\>[ \11]+\\([A-Z][a-zA-Z0-9_]*\\)" (1 font-lock-type-face)) ("\\<[A-Z]\\d*\\>" . font-lock-type-face) ("#.*$" . font-lock-comment-face)) 1 t)"##
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}
