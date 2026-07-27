use expect_test::expect;

use super::assert_actionscript_mode_parity;

#[test]
fn actionscript_mode_keymap_syntax_table_and_global_registrations_match_exactly() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              actionscript-mode-map
              (kbd key))))
          '("C-M-a"
            "C-M-e"
            "C-M-h"
            "C-c C-c"
            "C-c C-u"
            "C-c C-t"))
         (keymap-parent
          actionscript-mode-map)
         (with-temp-buffer
           (set-syntax-table
            actionscript-mode-syntax-table)
           (mapcar
            (lambda (character)
              (let ((descriptor
                     (char-table-range
                      actionscript-mode-syntax-table
                      character)))
                (list
                 character
                 (char-syntax
                  character)
                 (syntax-class
                  descriptor)
                 descriptor)))
            '(?_ ?$ ?\\ ?+ ?- ?= ?% ?< ?> ?& ?| ?' ?\240 ?/ ?* ?\n ?\r ?\( ?\) ?\")))
         (list
          (char-table-p
           actionscript-mode-syntax-table)
          (eq
           (char-table-parent
            actionscript-mode-syntax-table)
           (standard-syntax-table)))
         (rassq
          'actionscript-mode
          auto-mode-alist)
         (assq
          'actionscript-mode
          hs-special-modes-alist))"##;
    let expect = expect![[
        r#"OK ((("C-M-a" as-beginning-of-defun) ("C-M-e" as-end-of-defun) ("C-M-h" as-mark-defun) ("C-c C-c" comment-region) ("C-c C-u" uncomment-region) ("C-c C-t" nil)) nil ((95 119 2 #1=(2)) (36 119 2 #1#) (92 92 9 (9)) (43 46 1 #2=(1)) (45 46 1 #2#) (61 46 1 #2#) (37 46 1 #2#) (60 46 1 #2#) (62 46 1 #2#) (38 46 1 #2#) (124 46 1 #2#) (39 34 7 (7)) (160 46 1 #2#) (47 46 1 (2818049)) (42 46 1 (393217)) (10 62 12 (2097164)) (13 62 12 (2097164)) (40 40 4 (4 . 41)) (41 41 5 (5 . 40)) (34 34 7 (7))) (t t) ("\\.as\\'" . actionscript-mode) (actionscript-mode "{" "}" "/[*/]" nil hs-c-like-adjust-block-beginning))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_activation_installs_exact_local_state_for_each_font_lock_level() {
    let elisp_form = r##"(mapcar
         (lambda (level)
           (let ((actionscript-font-lock-level
                  level))
             (with-temp-buffer
               (let ((before-abbrev
                      local-abbrev-table))
                 (actionscript-mode)
                 (list
                  level
                  major-mode
                  mode-name
                  (derived-mode-p
                   'prog-mode)
                  (eq
                   (current-local-map)
                   actionscript-mode-map)
                  (eq
                   (syntax-table)
                   actionscript-mode-syntax-table)
                  (eq
                   local-abbrev-table
                   before-abbrev)
                  indent-line-function
                  parse-sexp-ignore-comments
                  comment-start
                  comment-start-skip
                  font-lock-defaults
                  (mapcar
                   #'local-variable-p
                   '(indent-line-function
                     parse-sexp-ignore-comments
                     comment-start
                     comment-start-skip
                     font-lock-defaults))
                  (buffer-modified-p))))))
         '(1 2 3))"##;
    let expect = expect![[
        r#"OK ((1 actionscript-mode "Actionscript" nil t t t actionscript-indent-line t "//" "\\(//+\\|/\\*+\\)\\s *" ((actionscript-font-lock-keywords-1) . #1=(nil nil)) (t t t t t) nil) (2 actionscript-mode "Actionscript" nil t t t actionscript-indent-line t "//" "\\(//+\\|/\\*+\\)\\s *" ((actionscript-font-lock-keywords-2) . #1#) (t t t t t) nil) (3 actionscript-mode "Actionscript" nil t t t actionscript-indent-line t "//" "\\(//+\\|/\\*+\\)\\s *" ((actionscript-font-lock-keywords-3) . #1#) (t t t t t) nil))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_invalid_font_lock_levels_activate_or_signal_exactly() {
    let elisp_form = r##"(mapcar
         (lambda (level)
           (let ((actionscript-font-lock-level
                  level))
             (with-temp-buffer
               (list
                level
                (condition-case error
                    (progn
                      (actionscript-mode)
                      (list
                       major-mode
                       font-lock-defaults))
                  (error
                   (list
                    (car error)
                    (cdr error))))))))
         '(0 4 -1 nil "2"))"##;
    let expect = expect![[
        r#"OK ((0 (actionscript-mode ((nil) . #1=(nil nil)))) (4 (actionscript-mode ((nil) . #1#))) (-1 (actionscript-mode ((nil) . #1#))) (nil (wrong-type-argument (number-or-marker-p nil))) ("2" (wrong-type-argument (number-or-marker-p "2"))))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_hook_runs_each_activation_after_local_state_reset() {
    let elisp_form = r##"(let (calls)
         (with-temp-buffer
           (let ((actionscript-mode-hook
                  (list
                   (lambda ()
                     (push
                      (list
                       major-mode
                       mode-name
                       indent-line-function
                       comment-start
                       font-lock-defaults
                       (local-variable-p
                        'sentinel))
                      calls)))))
             (setq-local sentinel
                         'before)
             (actionscript-mode)
             (setq-local sentinel
                         'between)
             (actionscript-mode)
             (list
              (nreverse calls)
              (boundp
               'sentinel)
              (local-variable-p
               'sentinel)))))"##;
    let expect = expect![[
        r#"OK (((actionscript-mode "Actionscript" actionscript-indent-line "//" ((actionscript-font-lock-keywords-2) . #1=(nil nil)) nil) (actionscript-mode "Actionscript" actionscript-indent-line "//" ((actionscript-font-lock-keywords-2) . #1#) nil)) nil nil)"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_auto_selection_matches_case_folded_as_suffix_and_backup_names() {
    let elisp_form = r##"(mapcar
         (lambda (name)
           (with-temp-buffer
             (setq buffer-file-name
                   name)
             (set-auto-mode)
             (list
              name
              major-mode
              mode-name)))
         '("/fixture/main.as"
           "/fixture/UPPER.AS"
           "/fixture/main.as~"
           "/fixture/as"
           "/fixture/main.as.txt"))"##;
    let expect = expect![[
        r#"OK (("/fixture/main.as" actionscript-mode "Actionscript") ("/fixture/UPPER.AS" actionscript-mode "Actionscript") ("/fixture/main.as~" actionscript-mode "Actionscript") ("/fixture/as" fundamental-mode "Fundamental") ("/fixture/main.as.txt" text-mode "Text"))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_level_one_fontifies_every_registered_named_token() {
    let elisp_form = r##"(let ((actionscript-font-lock-level
                1)
               (registries
                `((preprocessor
                   ,@preprocessor-kwds)
                  (constant
                   ,@actionscript-constant-kwds)
                  (global-function
                   ,@actionscript-global-funcs)
                  (global-class
                   ,@actionscript-global-classes)
                  (global-property
                   ,@actionscript-global-props)
                  (word-operator
                   ,@actionscript-word-operators)
                  (specifier
                   ,@actionscript-specifier-kwds)
                  (class
                   ,@actionscript-class-kwds)
                  (package
                   ,@actionscript-package-kwds)
                  (other-declaration
                   ,@actionscript-other-decl-kwds)
                  (variable-declaration
                   ,@actionscript-other-decl-2-kwds)
                  (declaration-level
                   ,@actionscript-decl-level-kwds)
                  (conditional
                   ,@actionscript-conditional-kwds)
                  (block-statement
                   ,@actionscript-block-stmt-1-kwds)
                  (simple-statement
                   ,@actionscript-simple-stmt-kwds)
                  (label
                   ,@actionscript-label-kwds)
                  (expression
                   ,@actionscript-expr-kwds)
                  (other
                   ,@actionscript-other-kwds))))
         (mapcar
          (lambda (registry)
            (with-temp-buffer
              (insert
               (mapconcat
                #'identity
                (cdr registry)
                " "))
              (actionscript-mode)
              (font-lock-ensure)
              (goto-char
               (point-min))
              (cons
               (car registry)
               (mapcar
                (lambda (token)
                  (search-forward
                   token)
                  (list
                   token
                   (get-text-property
                    (match-beginning 0)
                    'face)))
                (cdr registry)))))
          registries))"##;
    let expect = expect![[
        r##"OK ((preprocessor ("#include" nil) ("#define" nil) ("#else" nil) ("#endif" nil) ("#ifdef" nil) ("#ifndef" nil)) (constant ("true" font-lock-constant-face) ("false" font-lock-constant-face) ("null" font-lock-constant-face) ("undefined" font-lock-constant-face) ("NaN" font-lock-constant-face) ("Infinity" font-lock-constant-face) ("-Infinity" nil)) (global-function ("Array" font-lock-function-name-face) ("Boolean" font-lock-function-name-face) ("decodeURI" font-lock-function-name-face) ("decodeURIComponent" font-lock-function-name-face) ("encodeURI" font-lock-function-name-face) ("encodeURIComponent" font-lock-function-name-face) ("escape" font-lock-function-name-face) ("int" font-lock-function-name-face) ("isFinite" font-lock-function-name-face) ("isNaN" font-lock-function-name-face) ("isXMLName" font-lock-function-name-face) ("Number" font-lock-function-name-face) ("Object" font-lock-function-name-face) ("parseFloat" font-lock-function-name-face) ("parseInt" font-lock-function-name-face) ("String" font-lock-function-name-face) ("trace" font-lock-function-name-face) ("uint" font-lock-function-name-face) ("unescape" font-lock-function-name-face) ("XML" font-lock-function-name-face) ("XMLList" font-lock-function-name-face)) (global-class ("ArgumentError" nil) ("arguments" nil) ("Array" font-lock-function-name-face) ("Boolean" font-lock-function-name-face) ("Class" nil) ("Date" nil) ("DefinitionError" nil) ("Error" nil) ("EvalError" nil) ("Function" nil) ("int" font-lock-function-name-face) ("Math" nil) ("Namespace" nil) ("Number" font-lock-function-name-face) ("Object" font-lock-function-name-face) ("QName" nil) ("RangeError" nil) ("ReferenceError" nil) ("RegExp" nil) ("SecurityError" nil) ("String" font-lock-function-name-face) ("SyntaxError" nil) ("TypeError" nil) ("uint" font-lock-function-name-face) ("URIError" nil) ("VerifyError" nil) ("XML" font-lock-function-name-face) ("XMLList" font-lock-function-name-face)) (global-property ("this" font-lock-variable-name-face)) (word-operator ("as" font-lock-keyword-face) ("is" font-lock-keyword-face) ("instanceof" font-lock-keyword-face) ("new" font-lock-keyword-face) ("typeof" font-lock-keyword-face) ("void" font-lock-keyword-face)) (specifier ("override" font-lock-keyword-face) ("instrinsic" font-lock-keyword-face) ("private" font-lock-keyword-face) ("protected" font-lock-keyword-face) ("public" font-lock-keyword-face) ("static" font-lock-keyword-face) ("dynamic" font-lock-keyword-face)) (class ("class" font-lock-keyword-face) ("interface" font-lock-keyword-face)) (package ("package" font-lock-keyword-face)) (other-declaration ("import" font-lock-keyword-face)) (variable-declaration ("var" font-lock-keyword-face) ("function" font-lock-keyword-face) ("const" font-lock-keyword-face)) (declaration-level ("extends" font-lock-keyword-face) ("implements" font-lock-keyword-face)) (conditional ("for" font-lock-keyword-face) ("for each" font-lock-keyword-face) ("if" font-lock-keyword-face) ("while" font-lock-keyword-face) ("switch" font-lock-keyword-face) ("catch" font-lock-keyword-face)) (block-statement ("do" font-lock-keyword-face) ("else" font-lock-keyword-face) ("finally" font-lock-keyword-face) ("try" font-lock-keyword-face)) (simple-statement ("break" font-lock-keyword-face) ("continue" font-lock-keyword-face) ("return" font-lock-keyword-face) ("throw" font-lock-keyword-face)) (label ("case" font-lock-constant-face) ("default" font-lock-constant-face)) (expression ("super" font-lock-keyword-face)) (other ("delete" font-lock-keyword-face) ("get" font-lock-keyword-face) ("set" font-lock-keyword-face) ("with" font-lock-keyword-face)))"##
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_level_two_context_rules_fontify_imports_packages_classes_functions_for_in_and_vars()
 {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "import alpha.beta.Widget;\n")
         (insert
          "package alpha.beta {\n")
         (insert
          "public class Demo extends Base implements Contract {\n")
         (insert
          "function plain(value:String):void {}\n")
         (insert
          "function get item():Widget {}\n")
         (insert
          "for (var key:String in values) {}\n")
         (insert
          "var first:int, second:String;\n")
         (insert
          "}\n}\n")
         (let ((actionscript-font-lock-level
                2))
           (actionscript-mode))
         (font-lock-ensure)
         (let ((position
                (point-min))
               runs)
           (while
               (< position
                  (point-max))
             (let* ((face
                     (get-text-property
                      position
                      'face))
                    (next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max))))
               (when face
                 (push
                  (list
                   (buffer-substring-no-properties
                    position
                    next)
                   face)
                  runs))
               (setq position next)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("import" font-lock-keyword-face) ("alpha" font-lock-constant-face) ("beta" font-lock-constant-face) ("Widget" font-lock-type-face) ("package" font-lock-keyword-face) ("alpha" font-lock-constant-face) ("public" font-lock-keyword-face) ("class" font-lock-keyword-face) ("Demo" font-lock-type-face) ("extends" font-lock-keyword-face) ("Base" font-lock-type-face) ("implements" font-lock-keyword-face) ("Contract" font-lock-type-face) ("function" font-lock-keyword-face) ("plain" font-lock-function-name-face) ("String" font-lock-function-name-face) ("void" font-lock-keyword-face) ("function" font-lock-keyword-face) ("get" font-lock-keyword-face) ("item" font-lock-function-name-face) ("for" font-lock-keyword-face) ("var" font-lock-keyword-face) ("key" font-lock-variable-name-face) ("String" font-lock-function-name-face) ("in" font-lock-keyword-face) ("var" font-lock-keyword-face) ("first" font-lock-variable-name-face) ("int" font-lock-function-name-face) ("String" font-lock-function-name-face))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_font_lock_levels_case_and_identifier_boundaries_match() {
    let elisp_form = r##"(mapcar
         (lambda (level)
           (let ((actionscript-font-lock-level
                  level))
             (with-temp-buffer
               (insert
                "true TRUE True xtrue true_x\n")
               (insert
                "function named():void {} FUNCTION upper():VOID {}\n")
               (insert
                "package Alpha.Beta { class Demo extends Base {} }\n")
               (actionscript-mode)
               (font-lock-ensure)
               (let ((position
                      (point-min))
                     runs)
                 (while
                     (< position
                        (point-max))
                   (let* ((face
                           (get-text-property
                            position
                            'face))
                          (next
                           (next-single-property-change
                            position
                            'face
                            nil
                            (point-max))))
                     (when face
                       (push
                        (list
                         (buffer-substring-no-properties
                          position
                          next)
                         face)
                        runs))
                     (setq position next)))
                 (list
                  level
                  case-fold-search
                  font-lock-keywords-case-fold-search
                  (nreverse runs))))))
         '(1 2 3))"##;
    let expect = expect![[
        r#"OK ((1 t nil (("true" font-lock-constant-face) ("function" font-lock-keyword-face) ("void" font-lock-keyword-face) ("package" font-lock-keyword-face) ("class" font-lock-keyword-face) ("extends" font-lock-keyword-face))) (2 t nil (("true" font-lock-constant-face) ("function" font-lock-keyword-face) ("named" font-lock-function-name-face) ("void" font-lock-keyword-face) ("package" font-lock-keyword-face) ("Alpha" font-lock-constant-face) ("class" font-lock-keyword-face) ("Demo" font-lock-type-face) ("extends" font-lock-keyword-face) ("Base" font-lock-type-face))) (3 t nil (("true" font-lock-constant-face) ("function" font-lock-keyword-face) ("named" font-lock-function-name-face) ("void" font-lock-keyword-face) ("package" font-lock-keyword-face) ("Alpha" font-lock-constant-face) ("class" font-lock-keyword-face) ("Demo" font-lock-type-face) ("extends" font-lock-keyword-face) ("Base" font-lock-type-face))))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_comment_string_and_word_syntax_parse_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "var snake_name$ = 'single'; // line\n")
         (insert
          "var quoted = \"double\"; /* block */\n")
         (actionscript-mode)
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward
             needle)
            (let ((position
                   (match-beginning 0)))
              (list
               needle
               (char-syntax
                (char-after position))
               (syntax-ppss
                position)
               (get-text-property
                position
                'face))))
          '("snake_name$"
            "'single'"
            "// line"
            "\"double\""
            "/* block */")))"##;
    let expect = expect![[
        r#"OK (("snake_name$" 119 (0 nil 1 nil nil nil 0 nil nil nil nil) nil) ("'single'" 34 (0 nil 5 nil nil nil 0 nil nil nil nil) font-lock-string-face) ("// line" 46 (0 nil 19 nil nil nil 0 nil nil nil nil) font-lock-comment-delimiter-face) ("\"double\"" 34 (0 nil 41 nil nil nil 0 nil nil nil nil) font-lock-string-face) ("/* block */" 46 (0 nil 50 nil nil nil 0 nil nil nil nil) font-lock-comment-delimiter-face))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_font_lock_never_overrides_matching_tokens_inside_strings_or_comments() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "\"true function named import alpha.beta.Widget class Demo\"\n")
         (insert
          "'false var local extends Base'\n")
         (insert
          "// true function named import alpha.beta.Widget class Demo\n")
         (insert
          "/* false var local extends Base */\n")
         (let ((actionscript-font-lock-level
                2))
           (actionscript-mode))
         (font-lock-ensure)
         (let ((position
                (point-min))
               runs)
           (while
               (< position
                  (point-max))
             (let* ((face
                     (get-text-property
                      position
                      'face))
                    (next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max))))
               (when face
                 (push
                  (list
                   position
                   next
                   (buffer-substring-no-properties
                    position
                    next)
                   face
                   (nth 3
                        (syntax-ppss
                         position))
                   (nth 4
                        (syntax-ppss
                         position)))
                  runs))
               (setq position next)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK ((1 58 "\"true function named import alpha.beta.Widget class Demo\"" font-lock-string-face nil nil) (59 89 "'false var local extends Base'" font-lock-string-face nil nil) (90 93 "// " font-lock-comment-delimiter-face nil nil) (93 149 "true function named import alpha.beta.Widget class Demo\n" font-lock-comment-face nil t) (149 152 "/* " font-lock-comment-delimiter-face nil nil) (152 183 "false var local extends Base */" font-lock-comment-face nil t))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_reload_command_refreshes_named_source_and_test_buffers() {
    let elisp_form = r##"(let ((source
                (generate-new-buffer
                 "actionscript-mode.el"))
               (test
                (generate-new-buffer
                 "test.as"))
               calls)
         (unwind-protect
             (progn
               (with-current-buffer source
                 (insert
                  "(setq reload-sentinel (1+ (or (and (boundp 'reload-sentinel) reload-sentinel) 0)))"))
               (with-current-buffer test
                 (fundamental-mode))
               (let ((actionscript-mode-hook
                      (list
                       (lambda ()
                         (push
                          (buffer-name)
                          calls))))
                     messages)
                 (cl-letf
                     (((symbol-function
                        'message)
                       (lambda
                         (format-string
                          &rest arguments)
                         (when
                             (equal
                              format-string
                              "actionscript-mode reloaded.")
                           (push
                            (list
                             format-string
                             arguments
                             (apply
                              #'format
                              format-string
                              arguments))
                            messages)))))
                   (with-current-buffer test
                     (reload-actionscript-mode)))
                 (list
                  (with-current-buffer source
                    reload-sentinel)
                  (with-current-buffer test
                    (list
                     major-mode
                     mode-name))
                  calls
                  (nreverse messages))))
           (when
               (buffer-live-p source)
             (kill-buffer source))
           (when
               (buffer-live-p test)
             (kill-buffer test))))"##;
    let expect = expect![[
        r#"OK (1 (actionscript-mode "Actionscript") ("test.as") (("actionscript-mode reloaded." nil "actionscript-mode reloaded.")))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_reload_command_missing_buffer_signals_match() {
    let elisp_form = r##"(progn
         (when
             (get-buffer
              "actionscript-mode.el")
           (kill-buffer
            "actionscript-mode.el"))
         (when
             (get-buffer
              "test.as")
           (kill-buffer
            "test.as"))
         (condition-case error
             (reload-actionscript-mode)
           (error
            (list
             (car error)
             (cdr error)))))"##;
    let expect = expect!["OK (wrong-type-argument (stringp nil))"];
    assert_actionscript_mode_parity(elisp_form, expect);
}
