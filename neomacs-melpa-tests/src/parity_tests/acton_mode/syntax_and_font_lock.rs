use expect_test::expect;

use super::assert_acton_mode_parity;

#[test]
fn acton_mode_syntax_table_classifies_comments_strings_pairs_operators_and_symbols() {
    let elisp_form = r##"(with-temp-buffer
         (set-syntax-table
          acton-mode-syntax-table)
         (list
          (mapcar
           (lambda (character)
             (list
              character
              (string
               (char-syntax
                character))))
           '(?# ?\n ?\" ?' ?\\
             ?\( ?\) ?\[ ?\] ?\{ ?\}
             ?+ ?- ?* ?/ ?% ?& ?| ?^ ?!
             ?< ?> ?= ?~ ?_ ?a))
          (progn
            (insert
             "name_1 # comment\n\"double\" 'single' (a [b] {c})")
            (let ((comment-state
                   (progn
                     (goto-char
                      (point-min))
                     (search-forward
                      "comment")
                     (syntax-ppss)))
                  (double-state
                   (progn
                     (goto-char
                      (point-min))
                     (search-forward
                      "double")
                     (syntax-ppss)))
                  (single-state
                   (progn
                     (goto-char
                      (point-min))
                     (search-forward
                      "single")
                     (syntax-ppss))))
              (list
               (nth 4 comment-state)
               (nth 3 double-state)
               (nth 3 single-state)
               (progn
                 (goto-char
                  (point-min))
                 (search-forward
                  "name_1")
                 (skip-syntax-backward
                  "w_")
                 (buffer-substring-no-properties
                  (point)
                  (progn
                    (skip-syntax-forward
                     "w_")
                    (point))))
               (progn
                 (goto-char
                  (point-min))
                 (search-forward
                  "(")
                 (buffer-substring-no-properties
                  (1-
                   (point))
                  (scan-sexps
                   (1-
                    (point))
                   1))))))))"##;
    let expect = expect![[
        r#"OK (((35 "<") (10 ">") (34 "\"") (39 "\"") (92 "\\") (40 "(") (41 ")") (91 "(") (93 ")") (123 "(") (125 ")") (43 ".") (45 ".") (42 ".") (47 ".") (37 ".") (38 ".") (124 ".") (94 ".") (33 ".") (60 ".") (62 ".") (61 ".") (126 ".") (95 "_") (97 "w")) (t 34 39 "name_1" "(a [b] {c})"))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_font_lock_keyword_specification_matches_exactly() {
    let elisp_form = r##"(list
         (length
          acton-font-lock-keywords)
         acton-font-lock-keywords
         (get
          'acton-font-lock-keywords
          'risky-local-variable)
         (documentation-property
          'acton-font-lock-keywords
          'variable-documentation
          t)
         (file-name-nondirectory
          (symbol-file
           'acton-font-lock-keywords
           'defvar)))"##;
    let expect = expect![[
        r##"OK (31 (("\\(?:\"\"\"[\\s\\S]*?\"\"\"\\|'''[\\s\\S]*?'''\\)" 0 font-lock-string-face) ("\\<r\\(?:\"[^\"\\]*\\(?:\\\\.[^\"\\]*\\)*\"\\|'[^'\\]*\\(?:\\\\.[^'\\]*\\)*'\\)" 0 font-lock-string-face) ("\\<b\\(?:\"[^\"\\]*\\(?:\\\\.[^\"\\]*\\)*\"\\|'[^'\\]*\\(?:\\\\.[^'\\]*\\)*'\\)" 0 font-lock-string-face) ("\\(?:\"[^\"\\]*\\(?:\\\\.[^\"\\]*\\)*\"\\|'[^'\\]*\\(?:\\\\.[^'\\]*\\)*'\\)" 0 font-lock-string-face) ("\\\\[abfnrtv'\"\\\\]\\|\\\\[0-7]\\{1,3\\}\\|\\\\x[[:xdigit:]]\\{2\\}\\|\\\\u[[:xdigit:]]\\{4\\}\\|\\\\U[[:xdigit:]]\\{8\\}" 0 font-lock-escape-face t) ("\\<\\(?:[0-9]+\\.[0-9]*\\|[0-9]*\\.[0-9]+\\)\\(?:[eE][+-]?[0-9]+\\)?[jJ]\\>" . font-lock-constant-face) ("\\<[0-9]+[jJ]\\>" . font-lock-constant-face) ("\\<0x[0-9a-fA-F]+\\>" . font-lock-constant-face) ("\\<0o[0-7]+\\>" . font-lock-constant-face) ("\\<\\(?:[0-9]+\\.[0-9]*\\|[0-9]*\\.[0-9]+\\)\\(?:[eE][+-]?[0-9]+\\)?\\>" . font-lock-constant-face) ("\\<[0-9]+\\>" . font-lock-constant-face) ("[-+*/%@]\\|//\\|\\*\\*" . font-lock-builtin-face) ("[&|^~]\\|<<\\|>>" . font-lock-builtin-face) ("==\\|!=\\|<=\\|>=\\|<\\|>" . font-lock-builtin-face) ("\\(?:[-+*/%&|^]\\|//\\|\\*\\*\\|<<\\|>>\\)=" . font-lock-builtin-face) ("\\_<\\(?:is\\s-+not\\|not\\s-+in\\)\\_>" . font-lock-builtin-face) ("\\(?:->\\|=>\\)" . font-lock-builtin-face) ("\\_<[A-Z]\\d*\\_>" . font-lock-type-face) (":\\s-*\\([A-Z][a-zA-Z0-9_]*\\(?:\\[[^]]*\\]\\)?\\)" 1 font-lock-type-face) ("^[ \11]*\\(?:@\\(?:property\\|static\\(?:method\\)?\\)\\)[ \11]*\\(?:$\\|[^[:alnum:]_]\\)" . font-lock-preprocessor-face) ("\\_<\\(a\\(?:s\\(?:sert\\|ync\\)?\\|wait\\)\\|break\\|continue\\|del\\|e\\(?:l\\(?:if\\|se\\)\\|xcept\\)\\|f\\(?:inally\\|or\\|rom\\)\\|global\\|i\\(?:mport\\|[fn]\\)\\|lambda\\|nonlocal\\|pass\\|r\\(?:aise\\|eturn\\)\\|try\\|w\\(?:hile\\|ith\\)\\|yield\\)\\_>" . font-lock-keyword-face) ("\\_<\\(actor\\|class\\|def\\|extension\\|protocol\\|var\\)\\_>" . font-lock-keyword-face) ("\\_<\\(\\.\\.\\.\\|False\\|No\\(?:ne\\|tImplemented\\)\\|True\\)\\_>" . font-lock-constant-face) ("\\_<\\(action\\|mut\\|p\\(?:roc\\|ure\\)\\)\\_>" . font-lock-builtin-face) ("\\_<\\(isinstance\\)\\_>" . font-lock-builtin-face) ("\\<def\\>[ \11]+\\([a-zA-Z_][a-zA-Z0-9_]*\\)" (1 font-lock-function-name-face)) (":[ \11]*\\([A-Z][a-zA-Z0-9_]*\\)" (1 font-lock-type-face)) ("\\<\\(?:class\\|protocol\\|extension\\)\\>[ \11]+\\([A-Z][a-zA-Z0-9_]*\\)" (1 font-lock-type-face)) ("\\<actor\\>[ \11]+\\([A-Z][a-zA-Z0-9_]*\\)" (1 font-lock-type-face)) ("\\<[A-Z]\\d*\\>" . font-lock-type-face) ("#.*$" . font-lock-comment-face)) t nil "acton-mode.el")"##
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_fontifies_string_number_and_operator_categories_strictly() {
    let elisp_form = r##"(progn
         (defvar font-lock-escape-face
           'font-lock-escape-face)
         (with-temp-buffer
           (acton-mode)
           (insert
            "\"\"\"triple-text\"\"\"\n"
            "r\"raw-text\"\n"
            "b'byte-text'\n"
            "\"escape-\\n-\\x41\"\n"
            "3.5j 7J 0x1f 0o77 2.5e-2 42\n"
            "+ // ** & << == += //= is not not in -> =>\n")
           (font-lock-ensure)
           (mapcar
            (lambda (needle)
              (goto-char
               (point-min))
              (search-forward
               needle)
              (list
               needle
               (get-text-property
                (match-beginning 0)
                'face)
               (get-text-property
                (1-
                 (match-end 0))
                'face)))
            '("triple-text"
              "raw-text"
              "byte-text"
              "\\n"
              "\\x41"
              "3.5j"
              "7J"
              "0x1f"
              "0o77"
              "2.5e-2"
              "42"
              "+"
              "//"
              "**"
              "&"
              "<<"
              "=="
              "+="
              "//="
              "is not"
              "not in"
              "->"
              "=>"))))"##;
    let expect = expect![[
        r#"OK (("triple-text" font-lock-string-face font-lock-string-face) ("raw-text" font-lock-string-face font-lock-string-face) ("byte-text" font-lock-string-face font-lock-string-face) ("\\n" font-lock-escape-face font-lock-escape-face) ("\\x41" font-lock-escape-face font-lock-escape-face) ("3.5j" font-lock-constant-face font-lock-constant-face) ("7J" font-lock-constant-face font-lock-constant-face) ("0x1f" font-lock-constant-face font-lock-constant-face) ("0o77" font-lock-constant-face font-lock-constant-face) ("2.5e-2" font-lock-constant-face font-lock-constant-face) ("42" font-lock-constant-face font-lock-constant-face) ("+" font-lock-builtin-face font-lock-builtin-face) ("//" font-lock-builtin-face font-lock-builtin-face) ("**" font-lock-builtin-face font-lock-builtin-face) ("&" font-lock-builtin-face font-lock-builtin-face) ("<<" font-lock-builtin-face font-lock-builtin-face) ("==" font-lock-builtin-face font-lock-builtin-face) ("+=" font-lock-builtin-face nil) ("//=" font-lock-builtin-face nil) ("is not" font-lock-builtin-face font-lock-builtin-face) ("not in" font-lock-builtin-face font-lock-builtin-face) ("->" font-lock-builtin-face font-lock-builtin-face) ("=>" nil font-lock-builtin-face))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_escape_fontification_reports_the_missing_face_variable_exactly() {
    let elisp_form = r##"(let ((initially-bound
                (boundp
                 'font-lock-escape-face)))
         (with-temp-buffer
           (acton-mode)
           (insert
            "\"escape-\\n\"")
           (list
            initially-bound
            (condition-case error-data
                (progn
                  (font-lock-ensure)
                  'fontified)
              (error
               (list
                (car error-data)
                (cdr error-data))))
            (boundp
             'font-lock-escape-face))))"##;
    let expect = expect!["OK (nil (void-variable (font-lock-escape-face)) nil)"];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_fontifies_language_declarations_types_decorators_and_comments_strictly() {
    let elisp_form = r##"(with-temp-buffer
         (acton-mode)
         (insert
          "@property\n@static\n@staticmethod\n"
          "if elif else while for in try except finally with return break continue pass raise yield from import as global nonlocal assert await async del lambda\n"
          "def class actor protocol extension var\n"
          "True False None NotImplemented ...\n"
          "proc mut pure action isinstance\n"
          "def function_name(value: Result[List], item: T0):\n"
          "class ClassName:\nactor ActorName:\nprotocol ProtocolName:\nextension ExtensionName:\n"
          "# comment-text\n")
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward
             needle)
            (list
             needle
             (get-text-property
              (match-beginning 0)
              'face)))
          '("@property"
            "@staticmethod"
            "@static"
            "if"
            "await"
            "lambda"
            "def"
            "class"
            "actor"
            "protocol"
            "extension"
            "var"
            "True"
            "NotImplemented"
            "proc"
            "action"
            "isinstance"
            "function_name"
            "Result"
            "List"
            "T0"
            "ClassName"
            "ActorName"
            "ProtocolName"
            "ExtensionName"
            "comment-text")))"##;
    let expect = expect![[
        r#"OK (("@property" font-lock-builtin-face) ("@staticmethod" font-lock-builtin-face) ("@static" font-lock-builtin-face) ("if" font-lock-keyword-face) ("await" font-lock-keyword-face) ("lambda" font-lock-keyword-face) ("def" font-lock-keyword-face) ("class" font-lock-keyword-face) ("actor" font-lock-keyword-face) ("protocol" font-lock-keyword-face) ("extension" font-lock-keyword-face) ("var" font-lock-keyword-face) ("True" font-lock-constant-face) ("NotImplemented" font-lock-constant-face) ("proc" font-lock-builtin-face) ("action" font-lock-builtin-face) ("isinstance" font-lock-builtin-face) ("function_name" font-lock-function-name-face) ("Result" font-lock-type-face) ("List" font-lock-type-face) ("T0" font-lock-type-face) ("ClassName" font-lock-type-face) ("ActorName" font-lock-type-face) ("ProtocolName" font-lock-type-face) ("ExtensionName" font-lock-type-face) ("comment-text" font-lock-comment-face))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_font_lock_boundaries_and_nonmatches_match() {
    let elisp_form = r##"(with-temp-buffer
         (acton-mode)
         (insert
          "gift elifx xTrue isinstance_extra procValue\n"
          "TT T00 lower_type value: lower 0xZ 0o9 1e3\n"
          " @propertyExtra\nnotable inside\n")
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward
             needle)
            (list
             needle
             (get-text-property
              (match-beginning 0)
              'face)
             (get-text-property
              (1-
               (match-end 0))
              'face)))
          '("gift"
            "elifx"
            "xTrue"
            "isinstance_extra"
            "procValue"
            "TT"
            "T00"
            "lower_type"
            "lower"
            "0xZ"
            "0o9"
            "1e3"
            "@propertyExtra"
            "notable"
            "inside")))"##;
    let expect = expect![[
        r#"OK (("gift" nil nil) ("elifx" nil nil) ("xTrue" nil nil) ("isinstance_extra" nil nil) ("procValue" nil nil) ("TT" nil nil) ("T00" nil nil) ("lower_type" nil nil) ("lower" nil nil) ("0xZ" nil nil) ("0o9" nil nil) ("1e3" nil nil) ("@propertyExtra" font-lock-builtin-face nil) ("notable" nil nil) ("inside" nil nil))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_initializes_all_buffer_local_mode_configuration_and_hooks() {
    let elisp_form = r##"(with-temp-buffer
         (let ((acton-mode-hook
                (list
                 (lambda ()
                   (setq-local
                    acton-hook-ran
                    (list
                     major-mode
                     mode-name))))))
           (acton-mode)
           (list
            major-mode
            mode-name
            (derived-mode-p
             'prog-mode)
            (eq
             (syntax-table)
             acton-mode-syntax-table)
            comment-start
            comment-start-skip
            comment-column
            comment-use-syntax
            indent-line-function
            tab-width
            indent-tabs-mode
            font-lock-defaults
            paragraph-start
            paragraph-separate
            paragraph-ignore-fill-prefix
            imenu-generic-expression
            (local-variable-p
             'post-self-insert-hook)
            post-self-insert-hook
            acton-hook-ran)))"##;
    let expect = expect![[
        r##"OK (acton-mode "Acton" prog-mode t "#" "#+\\s-*" 40 t acton-indent-line 4 nil (acton-font-lock-keywords nil nil nil beginning-of-defun) "^[ \11]*$\\|^\f" "^[ \11]*$\\|^\f" t (("Class" "^[ \11]*class[ \11]+\\([A-Za-z_][A-Za-z0-9_]*\\)" 1) ("Actor" "^[ \11]*actor[ \11]+\\([A-Za-z_][A-Za-z0-9_]*\\)" 1) ("Protocol" "^[ \11]*protocol[ \11]+\\([A-Za-z_][A-Za-z0-9_]*\\)" 1) ("Extension" "^[ \11]*extension[ \11]+\\([A-Za-z_][A-Za-z0-9_]*\\)" 1) ("Function" "^[ \11]*def[ \11]+\\([A-Za-z_][A-Za-z0-9_]*\\)" 1)) t (acton-handle-colon t) (acton-mode "Acton"))"##
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_imenu_expression_indexes_all_declared_definition_categories() {
    let elisp_form = r##"(progn
         (require
          'imenu)
         (with-temp-buffer
           (acton-mode)
           (insert
            "class Alpha:\n"
            "actor Worker_2:\n"
            "protocol Proto:\n"
            "extension Extra:\n"
            "def compute_3(value):\n"
            "  pass\n")
           (mapcar
            (lambda (entry)
              (list
               (car entry)
               (mapcar
                (lambda (item)
                  (list
                   (car item)
                   (marker-position
                    (cdr item))))
                (cdr entry))))
            (imenu--generic-function
             imenu-generic-expression))))"##;
    let expect = expect![[
        r#"OK (("Function" (("compute_3" 63))) ("Extension" (("Extra" 46))) ("Protocol" (("Proto" 30))) ("Actor" (("Worker_2" 14))) ("Class" (("Alpha" 1))))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}
