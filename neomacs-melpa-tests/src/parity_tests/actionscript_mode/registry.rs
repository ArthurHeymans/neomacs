use expect_test::expect;

use super::{assert_actionscript_mode_autoload_parity, assert_actionscript_mode_parity};

#[test]
fn actionscript_mode_exact_pin_metadata_version_group_and_custom_options_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq
                  'actionscript-mode
                  package-alist))))
         (list
          (package-desc-name
           descriptor)
          (package-version-join
           (package-desc-version
            descriptor))
          (package-desc-reqs
           descriptor)
          (package-desc-summary
           descriptor)
          (copy-tree
           (package-desc-extras
            descriptor))
          actionscript-mode-version
          (featurep
           'actionscript-mode)
          (get
           'actionscript
           'group-documentation)
          (copy-tree
           (get
            'actionscript
            'custom-group))
          (mapcar
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
                (get
                 symbol
                 'custom-type)
                (get
                 symbol
                 'safe-local-variable)
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
           '(actionscript-mode-hook
             actionscript-indent-level
             actionscript-font-lock-level))))"##;
    let expect = expect![[
        r#"OK (actionscript-mode "20180527.1701" nil "A simple mode for editing Actionscript 3 files." ((:keywords "language" "modes") (:revdesc . "65abd58e1984") (:commit . "65abd58e198458a8e46748c5962c41d80d60c4ea") (:url . "https://codeberg.org/austinhaas/actionscript-mode")) "7.2.2" t "Major mode for editing Actionscript code." ((actionscript-mode-hook custom-variable) (actionscript-indent-level custom-variable) (actionscript-font-lock-level custom-variable)) ((actionscript-mode-hook nil nil hook nil "*Hook called by `actionscript-mode'." "actionscript-mode.el") (actionscript-indent-level 4 4 integer integerp "Number of spaces for each indentation step in `actionscript-mode'." "actionscript-mode.el") (actionscript-font-lock-level 2 2 (radio (const :tag "Only keywords." 1) (const :tag "Keywords and contextual tags." 2) (const :tag "All of the above plus all of Actionscript's builtin classes. (not implemented)" 3)) nil "*What level of syntax highlighting do we want. 1-3" "actionscript-mode.el")))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_exact_language_word_registries_and_derived_regexes_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (default-value
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
         '(preprocessor-kwds
           actionscript-constant-kwds
           actionscript-global-funcs
           actionscript-global-classes
           actionscript-global-props
           actionscript-symbol-operators
           actionscript-word-operators
           actionscript-specifier-kwds
           actionscript-class-kwds
           actionscript-package-kwds
           actionscript-other-decl-kwds
           actionscript-other-decl-2-kwds
           actionscript-decl-level-kwds
           actionscript-conditional-kwds
           actionscript-block-stmt-1-kwds
           actionscript-simple-stmt-kwds
           actionscript-label-kwds
           actionscript-expr-kwds
           actionscript-other-kwds
           actionscript-keywords
           actionscript-identifier-re
           as-function-re
           as-attribute-re
           as-imenu-generic-expression))"##;
    let expect = expect![[
        r##"OK ((preprocessor-kwds ("#include" "#define" "#else" "#endif" "#ifdef" "#ifndef") nil "actionscript-mode.el") (actionscript-constant-kwds ("true" "false" "null" "undefined" "NaN" "Infinity" "-Infinity") nil "actionscript-mode.el") (actionscript-global-funcs ("Array" "Boolean" "decodeURI" "decodeURIComponent" "encodeURI" "encodeURIComponent" "escape" "int" "isFinite" "isNaN" "isXMLName" "Number" "Object" "parseFloat" "parseInt" "String" "trace" "uint" "unescape" "XML" "XMLList") nil "actionscript-mode.el") (actionscript-global-classes ("ArgumentError" "arguments" "Array" "Boolean" "Class" "Date" "DefinitionError" "Error" "EvalError" "Function" "int" "Math" "Namespace" "Number" "Object" "QName" "RangeError" "ReferenceError" "RegExp" "SecurityError" "String" "SyntaxError" "TypeError" "uint" "URIError" "VerifyError" "XML" "XMLList") nil "actionscript-mode.el") (actionscript-global-props ("this") nil "actionscript-mode.el") (actionscript-symbol-operators ("+" "+=" "[]" "=" "&" "&=" "<<" "<<=" "~" "|" "|=" ">>" ">>=" ">>>" ">>>=" "^" "^=" "/*" "*/" "," "?:" "--" "/" "/=" "." "==" ">" ">=" "++" "!=" "<>" "<" "<=" "//" "&&" "!" "||" "%" "%=" "*" "*=" "{}" "()" "===" "!==" "\"" "-" "-=" ":") nil "actionscript-mode.el") (actionscript-word-operators ("as" "is" "instanceof" "new" "typeof" "void") nil "actionscript-mode.el") (actionscript-specifier-kwds ("override" "instrinsic" "private" "protected" "public" "static" "dynamic") nil "actionscript-mode.el") (actionscript-class-kwds ("class" "interface") nil "actionscript-mode.el") (actionscript-package-kwds ("package") nil "actionscript-mode.el") (actionscript-other-decl-kwds ("import") nil "actionscript-mode.el") (actionscript-other-decl-2-kwds ("var" "function" "const") nil "actionscript-mode.el") (actionscript-decl-level-kwds ("extends" "implements") nil "actionscript-mode.el") (actionscript-conditional-kwds ("for" "for each" "if" "while" "switch" "catch") nil "actionscript-mode.el") (actionscript-block-stmt-1-kwds ("do" "else" "finally" "try") nil "actionscript-mode.el") (actionscript-simple-stmt-kwds ("break" "continue" "return" "throw") nil "actionscript-mode.el") (actionscript-label-kwds ("case" "default") nil "actionscript-mode.el") (actionscript-expr-kwds ("super") nil "actionscript-mode.el") (actionscript-other-kwds ("delete" "get" "set" "with") nil "actionscript-mode.el") (actionscript-keywords "\\<\\(-Infinity\\|Ar\\(?:gumentError\\|ray\\)\\|Boolean\\|Class\\|D\\(?:ate\\|efinitionError\\)\\|E\\(?:\\(?:valE\\)?rror\\)\\|Function\\|Infinity\\|Math\\|N\\(?:a\\(?:N\\|mespace\\)\\|umber\\)\\|Object\\|QName\\|R\\(?:angeError\\|e\\(?:ferenceError\\|gExp\\)\\)\\|S\\(?:ecurityError\\|tring\\|yntaxError\\)\\|TypeError\\|URIError\\|VerifyError\\|XML\\(?:List\\)?\\|arguments\\|break\\|c\\(?:a\\(?:se\\|tch\\)\\|lass\\|on\\(?:st\\|tinue\\)\\)\\|d\\(?:e\\(?:codeURI\\(?:Component\\)?\\|fault\\|lete\\)\\|o\\|ynamic\\)\\|e\\(?:lse\\|ncodeURI\\(?:Component\\)?\\|scape\\|xtends\\)\\|f\\(?:alse\\|inally\\|or\\(?: each\\)?\\|unction\\)\\|get\\|i\\(?:f\\|mp\\(?:lements\\|ort\\)\\|n\\(?:strinsic\\|t\\(?:erface\\)?\\)\\|s\\(?:Finite\\|NaN\\|XMLName\\)\\)\\|null\\|override\\|p\\(?:a\\(?:ckage\\|rse\\(?:\\(?:Floa\\|In\\)t\\)\\)\\|r\\(?:ivate\\|otected\\)\\|ublic\\)\\|return\\|s\\(?:et\\|tatic\\|uper\\|witch\\)\\|t\\(?:h\\(?:is\\|row\\)\\|r\\(?:ace\\|ue\\|y\\)\\)\\|u\\(?:int\\|n\\(?:defined\\|escape\\)\\)\\|var\\|w\\(?:hile\\|ith\\)\\)\\>" nil "actionscript-mode.el") (actionscript-identifier-re "[a-zA-Z_$][a-zA-Z0-9_$]*" "Regexp to match any valid identifier in actionscript." "actionscript-mode.el") (as-function-re "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\<function\\>[ \11\n]+\\(?:\\(?:[gs]et\\)[ \11\n]+\\)?\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)[ \11\n]*([ \11\n]*\\([\"a-zA-Z-0-9_$*,:= \11\n]*?\\(?:\\.\\.\\.[a-zA-Z-0-9_$]+\\)?\\))[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?[ \11\n]*{" "A regexp that matches a function signature in Actionscript 3.0." "actionscript-mode.el") (as-attribute-re "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\<\\(\\(?:const\\|var\\)\\)\\>[ \11\n]+\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?" "A regexp that matches a class attribute definition in Actionscript 3.0." "actionscript-mode.el") (as-imenu-generic-expression ((nil "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\<function\\>[ \11\n]+\\(?:\\(?:[gs]et\\)[ \11\n]+\\)?\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)[ \11\n]*([ \11\n]*\\([\"a-zA-Z-0-9_$*,:= \11\n]*?\\(?:\\.\\.\\.[a-zA-Z-0-9_$]+\\)?\\))[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?[ \11\n]*{" 3)) nil "actionscript-mode.el"))"##
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_font_lock_level_registries_and_symbol_metadata_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (length
             (default-value
              symbol))
            (default-value
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
         '(actionscript-font-lock-keywords-1
           actionscript-font-lock-keywords-2
           actionscript-font-lock-keywords-3))"##;
    let expect = expect![[
        r#"OK ((actionscript-font-lock-keywords-1 17 (#1=("\\<\\(#\\(?:define\\|e\\(?:lse\\|ndif\\)\\|i\\(?:f\\(?:n?def\\)\\|nclude\\)\\)\\)\\>" 0 'font-lock-proceprocessor-face) #2=("\\<\\(-Infinity\\|Infinity\\|NaN\\|false\\|null\\|true\\|undefined\\)\\>" 0 'font-lock-constant-face) #3=("\\<\\(Array\\|Boolean\\|Number\\|Object\\|String\\|XML\\(?:List\\)?\\|decodeURI\\(?:Component\\)?\\|e\\(?:ncodeURI\\(?:Component\\)?\\|scape\\)\\|i\\(?:nt\\|s\\(?:Finite\\|NaN\\|XMLName\\)\\)\\|parse\\(?:\\(?:Floa\\|In\\)t\\)\\|trace\\|u\\(?:int\\|nescape\\)\\)\\>" 0 'font-lock-function-name-face) #4=("\\<\\(this\\)\\>" 0 'font-lock-variable-name-face) #5=("\\<\\(as\\|i\\(?:nstanceof\\|s\\)\\|new\\|typeof\\|void\\)\\>" 0 'font-lock-keyword-face) #6=("\\<\\(dynamic\\|instrinsic\\|override\\|p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\|static\\)\\>" 0 'font-lock-keyword-face) #7=("\\<\\(class\\|interface\\)\\>" 0 'font-lock-keyword-face) #8=("\\<\\(package\\)\\>" 0 'font-lock-keyword-face) #9=("\\<\\(import\\)\\>" 0 'font-lock-keyword-face) #10=("\\<\\(const\\|function\\|var\\)\\>" 0 'font-lock-keyword-face) #11=("\\<\\(\\(?:extend\\|implement\\)s\\)\\>" 0 'font-lock-keyword-face) #12=("\\<\\(catch\\|for\\(?: each\\)?\\|if\\|switch\\|while\\)\\>" 0 'font-lock-keyword-face) #13=("\\<\\(do\\|else\\|\\(?:finall\\|tr\\)y\\)\\>" 0 'font-lock-keyword-face) #14=("\\<\\(break\\|continue\\|return\\|throw\\)\\>" 0 'font-lock-keyword-face) #15=("\\<\\(case\\|default\\)\\>" 0 'font-lock-constant-face) #16=("\\<\\(super\\)\\>" 0 'font-lock-keyword-face) #17=("\\<\\(delete\\|get\\|set\\|with\\)\\>" 0 'font-lock-keyword-face)) "Subdued level highlighting for Actionscript mode." "actionscript-mode.el") (actionscript-font-lock-keywords-2 23 (#1# #2# #3# #4# #5# #6# #7# #8# #9# #10# #11# #12# #13# #14# #15# #16# #17# #18=("\\<\\(import\\)\\>[ \11]*\\(?:[a-zA-Z_$][a-zA-Z0-9_$]*\\.\\)*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)?" (2 'font-lock-type-face nil t) ("[ \11]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)\\." (goto-char (match-end 1)) (goto-char (match-end 0)) (1 'font-lock-constant-face nil t))) #19=("\\<\\(package\\)\\>[ \11]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)?" (2 'font-lock-constant-face nil t)) #20=("\\<\\(class\\|extends\\|i\\(?:mplements\\|nterface\\)\\)\\>[ \11]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)?" (2 'font-lock-type-face nil t)) #21=("\\<function\\>[ \11\n]+\\(?:\\(?:get\\|set\\)[ \11\n]+\\)?\\(?:\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)\\)?" (1 'font-lock-function-name-face nil t)) #22=("\\<for\\>[ \11\n]*([ \11\n]*\\(?:var[ \11\n]+\\)?[a-zA-Z_$][a-zA-Z0-9_$]*[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z0-9_$*]*\\)\\)?[ \11\n]+\\(in\\)[ \11\n]+" (2 'font-lock-keyword-face nil t)) #23=("\\<var\\>\\([ \11]*[a-zA-Z_$][a-zA-Z0-9_$]*\\)" (font-lock-match-c-style-declaration-item-and-skip-to-next (goto-char (match-beginning 1)) (goto-char (match-beginning 1)) (1 'font-lock-variable-name-face)))) "Medium level highlighting for Actionscript mode." "actionscript-mode.el") (actionscript-font-lock-keywords-3 23 (#1# #2# #3# #4# #5# #6# #7# #8# #9# #10# #11# #12# #13# #14# #15# #16# #17# #18# #19# #20# #21# #22# #23#) "Gaudy level highlighting for Actionscript mode." "actionscript-mode.el"))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_map_and_syntax_table_variable_contracts_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (default-value
                   symbol)))
             (list
              symbol
              (boundp symbol)
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
                      file)))
              (if
                  (eq symbol
                      'actionscript-mode-map)
                  (list
                   (keymapp value)
                   (keymap-parent value)
                   (let (bindings)
                     (map-keymap
                      (lambda
                        (event binding)
                        (push
                         (cons event binding)
                         bindings))
                      value)
                     (nreverse bindings)))
                (list
                 (char-table-p value)
                 (eq
                  (char-table-parent value)
                  (standard-syntax-table)))))))
         '(actionscript-mode-map
           actionscript-mode-syntax-table))"##;
    let expect = expect![[
        r#"OK ((actionscript-mode-map t t nil "Keymap used in actionscript-mode buffers." "actionscript-mode.el" (t nil ((3 keymap (21 . uncomment-region) (3 . comment-region)) (27 keymap (8 . as-mark-defun) (5 . as-end-of-defun) (1 . as-beginning-of-defun))))) (actionscript-mode-syntax-table t t nil "Syntax table used in actionscript-mode buffers." "actionscript-mode.el" (t t)))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_complete_callable_surface_arglists_commands_docs_and_sources_match() {
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
               (and doc
                    (car
                     (split-string
                      doc
                      "\n")))
               (let ((file
                      (symbol-file
                       symbol
                       'defun)))
                 (and file
                      (file-name-nondirectory
                       file))))))
          '(as-get-function-re
            as-get-attribute-re
            as-imenu-init
            as-get-beginning-of-defun
            as-get-end-of-defun
            as-get-end-of-defun2
            as-beginning-of-defun
            as-inside-defun?
            as-end-of-defun
            as-mark-defun
            actionscript-indent-line
            as3-calculate-indentation
            as3-maybe-skip-leading-close-delim
            as3-face-at-point
            as3-count-scope-depth
            actionscript-mode
            reload-actionscript-mode))"##;
    let expect = expect![[
        r#"OK ((as-get-function-re t (&optional function-name) nil nil "Returns a regular expression that will match the function signature" "actionscript-mode.el") (as-get-attribute-re t (&optional attribute-name) nil nil "Returns a regular expression that will match the class attribute" "actionscript-mode.el") (as-imenu-init t (mode-generic-expression) nil nil nil "actionscript-mode.el") (as-get-beginning-of-defun t nil nil nil nil "actionscript-mode.el") (as-get-end-of-defun t nil nil nil nil "actionscript-mode.el") (as-get-end-of-defun2 t nil nil nil nil "actionscript-mode.el") (as-beginning-of-defun t nil t (interactive nil) nil "actionscript-mode.el") (as-inside-defun? t nil nil nil nil "actionscript-mode.el") (as-end-of-defun t nil t (interactive nil) nil "actionscript-mode.el") (as-mark-defun t nil t (interactive nil) nil "actionscript-mode.el") (actionscript-indent-line t nil t (interactive nil) "Indent current line of As3 code. Delete any trailing" "actionscript-mode.el") (as3-calculate-indentation t nil nil nil "Return the column to which the current line should be indented." "actionscript-mode.el") (as3-maybe-skip-leading-close-delim t nil nil nil nil "actionscript-mode.el") (as3-face-at-point t (pos) nil nil "Return face descriptor for char at point." "actionscript-mode.el") (as3-count-scope-depth t (rstart rend) nil nil "Return difference between open and close scope delimeters." "actionscript-mode.el") (actionscript-mode t nil t (interactive nil) "Major mode for editing Actionscript files." "actionscript-mode.el") (reload-actionscript-mode t nil t (interactive nil) nil "actionscript-mode.el"))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_installed_package_inventory_and_content_assets_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'actionscript-mode
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
        r#"OK (("README-elpa" "actionscript-mode-autoloads.el" "actionscript-mode-pkg.el" "actionscript-mode.el" "actionscript-mode.elc") (("README-elpa" t (405 "f7421927a48fe142628207e6949cd9d90d20182c09f319ca976888de2c3c4f85")) ("actionscript-mode-autoloads.el" t (967 "a4870586ca3dab2c2659eaf0f02c844b76351ccfcde6f1a632f741fada9b5f9c")) ("actionscript-mode-pkg.el" t (334 "80224dcd1b50a47458490e8e686e7694c7ea13202684da8221da9d872ccb9120")) ("actionscript-mode.el" t (23132 "760995eaaaafeabe9a4d0e4824993245fd8eeb97c85dc0e1af01c5b1c5a0ad30")) ("actionscript-mode.elc" t t)))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_generated_autoload_registers_mode_and_extension_without_runtime() {
    let elisp_form = r##"(let ((before
                (list
                 (featurep
                  'actionscript-mode)
                 (featurep
                  'actionscript-mode-autoloads)
                 (autoloadp
                  (symbol-function
                   'actionscript-mode))
                 (rassq
                  'actionscript-mode
                  auto-mode-alist)
                 (boundp
                  'actionscript-mode-version)
                 (fboundp
                  'as-get-function-re))))
         (list
          before
          (commandp
           'actionscript-mode)
          (interactive-form
           'actionscript-mode)
          (help-function-arglist
           'actionscript-mode
           t)
          (let ((file
                 (symbol-file
                  'actionscript-mode
                  'defun)))
            (and file
                 (file-name-nondirectory
                  file)))
          (list
           (featurep
            'actionscript-mode)
           (autoloadp
            (symbol-function
             'actionscript-mode))
           (boundp
            'actionscript-mode-version)
           (fboundp
            'as-get-function-re))))"##;
    let expect = expect![[
        r#"OK ((nil t t ("\\.as\\'" . actionscript-mode) nil nil) t (interactive nil) nil "actionscript-mode.el" (t nil t t))"#
    ]];
    assert_actionscript_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_direct_reload_preserves_map_and_syntax_objects_and_deduplicates_registrations()
{
    let elisp_form = r##"(let ((map
                actionscript-mode-map)
               (syntax
                actionscript-mode-syntax-table)
               (hook
                actionscript-mode-hook))
         (unwind-protect
             (progn
               (setq actionscript-mode-hook
                     '(sentinel-hook))
               (load
                (getenv
                 "NEOMACS_PACKAGE_SOURCE")
                nil t t)
               (list
                (eq map
                    actionscript-mode-map)
                (eq syntax
                    actionscript-mode-syntax-table)
                actionscript-mode-hook
                (length
                 (seq-filter
                  (lambda (entry)
                    (equal entry
                           '("\\.as\\'" . actionscript-mode)))
                  auto-mode-alist))
                (length
                 (seq-filter
                  (lambda (entry)
                    (equal entry
                           '(actionscript-mode
                             "{"
                             "}"
                             "/[*/]"
                             nil
                             hs-c-like-adjust-block-beginning)))
                  hs-special-modes-alist))))
           (setq actionscript-mode-hook
                 hook)))"##;
    let expect = expect!["OK (t t (sentinel-hook) 1 1)"];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_direct_reload_preserves_custom_values_but_rebuilds_constants_and_font_lock() {
    let elisp_form = r##"(let ((original-indent
                actionscript-indent-level)
               (original-level
                actionscript-font-lock-level)
               (original-version
                actionscript-mode-version)
               (original-constants
                actionscript-constant-kwds)
               (original-identifier
                actionscript-identifier-re)
               (original-font-lock
                actionscript-font-lock-keywords-1))
         (unwind-protect
             (progn
               (setq actionscript-indent-level
                     9
                     actionscript-font-lock-level
                     3
                     actionscript-mode-version
                     "sentinel-version"
                     actionscript-constant-kwds
                     '(sentinel-constant)
                     actionscript-identifier-re
                     "sentinel-identifier"
                     actionscript-font-lock-keywords-1
                     '(sentinel-font-lock))
               (load
                (getenv
                 "NEOMACS_PACKAGE_SOURCE")
                nil t t)
               (list
                actionscript-indent-level
                actionscript-font-lock-level
                actionscript-mode-version
                actionscript-constant-kwds
                actionscript-identifier-re
                (eq
                 (car actionscript-font-lock-keywords-1)
                 'sentinel-font-lock)
                (length
                 actionscript-font-lock-keywords-1)
                (length
                 actionscript-font-lock-keywords-2)
                (length
                 actionscript-font-lock-keywords-3)))
           (setq actionscript-indent-level
                 original-indent
                 actionscript-font-lock-level
                 original-level
                 actionscript-mode-version
                 original-version
                 actionscript-constant-kwds
                 original-constants
                 actionscript-identifier-re
                 original-identifier
                 actionscript-font-lock-keywords-1
                 original-font-lock)))"##;
    let expect = expect![[
        r#"OK (9 3 "7.2.2" ("true" "false" "null" "undefined" "NaN" "Infinity" "-Infinity") "[a-zA-Z_$][a-zA-Z0-9_$]*" nil 17 23 23)"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}
