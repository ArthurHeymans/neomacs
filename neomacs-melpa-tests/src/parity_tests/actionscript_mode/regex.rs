use expect_test::expect;

use super::assert_actionscript_mode_parity;

#[test]
fn actionscript_mode_function_regex_builder_default_named_and_invalid_inputs_match() {
    let elisp_form = r##"(list
         (as-get-function-re)
         (as-get-function-re
          "render")
         (condition-case error
             (as-get-function-re
              "")
           (error
            (list
             (car error)
             (cdr error))))
         (condition-case error
             (as-get-function-re
              7)
           (error
            (list
             (car error)
             (cdr error)))))"##;
    let expect = expect![[
        r#"OK ("\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\<function\\>[ \11\n]+\\(?:\\(?:[gs]et\\)[ \11\n]+\\)?\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)[ \11\n]*([ \11\n]*\\([\"a-zA-Z-0-9_$*,:= \11\n]*?\\(?:\\.\\.\\.[a-zA-Z-0-9_$]+\\)?\\))[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?[ \11\n]*{" "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\<function\\>[ \11\n]+\\(?:\\(?:[gs]et\\)[ \11\n]+\\)?\\(render\\)[ \11\n]*([ \11\n]*\\([\"a-zA-Z-0-9_$*,:= \11\n]*?\\(?:\\.\\.\\.[a-zA-Z-0-9_$]+\\)?\\))[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?[ \11\n]*{" "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|\\(?:final\\|override\\|static\\)\\)[ \11\n]+\\)?\\<function\\>[ \11\n]+\\(?:\\(?:[gs]et\\)[ \11\n]+\\)?\\(\\)[ \11\n]*([ \11\n]*\\([\"a-zA-Z-0-9_$*,:= \11\n]*?\\(?:\\.\\.\\.[a-zA-Z-0-9_$]+\\)?\\))[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?[ \11\n]*{" (wrong-type-argument (sequencep 7)))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_function_regex_matches_modifiers_accessors_multiline_args_and_return_types() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (let ((matched
                  (string-match
                   as-function-re
                   source)))
             (list
              source
              matched
              (and matched
                   (mapcar
                    (lambda (index)
                      (match-string
                       index
                       source))
                    '(0 1 2 3 4 5))))))
         '("function plain() {"
           "  public function render(value:String, count:int = 2):void {"
           "private static function get item():Widget{"
           "override protected function set item(value:*):* {"
           "final\nfunction spread(first:String, ...rest):Object\n{"
           "function wrong-name!() {"
           "public public public function tooMany() {"
           "function missingParen {"
           "x function notAtLineStart() {"))"##;
    let expect = expect![[
        r#"OK (("function plain() {" 0 ("function plain() {" nil nil "plain" "" nil)) ("  public function render(value:String, count:int = 2):void {" 0 ("  public function render(value:String, count:int = 2):void {" "public" nil "render" "value:String, count:int = 2" "void")) ("private static function get item():Widget{" 0 ("private static function get item():Widget{" "private" "static" "item" "" "Widget")) ("override protected function set item(value:*):* {" 0 ("override protected function set item(value:*):* {" "override" "protected" "item" "value:*" "*")) ("final\nfunction spread(first:String, ...rest):Object\n{" 0 ("final\nfunction spread(first:String, ...rest):Object\n{" "final" nil "spread" "first:String, ...rest" "Object")) ("function wrong-name!() {" nil nil) ("public public public function tooMany() {" nil nil) ("function missingParen {" nil nil) ("x function notAtLineStart() {" nil nil))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_attribute_regex_builder_and_match_groups_cover_var_const_and_types() {
    let elisp_form = r##"(let ((default-regexp
                (as-get-attribute-re))
               (named-regexp
                (as-get-attribute-re
                 "count")))
         (list
          default-regexp
          named-regexp
          (as-get-attribute-re
           "")
          (condition-case error
              (as-get-attribute-re
               7)
            (error
             (list
              (car error)
              (cdr error))))
          (mapcar
           (lambda (source)
             (let ((matched
                    (string-match
                     default-regexp
                     source)))
               (list
                source
                matched
                (and matched
                     (mapcar
                      (lambda (index)
                        (match-string
                         index
                         source))
                      '(0 1 2 3 4 5))))))
           '("var plain"
             " public static var count:int"
             "private const NAME:*"
             "static protected const value : Widget"
             "public public public var tooMany:int"
             "let nope:int"
             "x var notAtLineStart"))
          (mapcar
           (lambda (source)
             (and
              (string-match
               named-regexp
               source)
              (match-string
               4
               source)))
           '("var count:int"
             "var counter:int"
             "const count:*"))))"##;
    let expect = expect![[
        r#"OK ("\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\<\\(\\(?:const\\|var\\)\\)\\>[ \11\n]+\\([a-zA-Z_$][a-zA-Z0-9_$]*\\)[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?" "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\<\\(\\(?:const\\|var\\)\\)\\>[ \11\n]+\\(count\\)[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?" "\\(?:^[ \11\n]*\\)\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\(?:\\(\\(?:p\\(?:r\\(?:ivate\\|otected\\)\\|ublic\\)\\)\\|static\\)[ \11\n]+\\)?\\<\\(\\(?:const\\|var\\)\\)\\>[ \11\n]+\\(\\)[ \11\n]*\\(?::[ \11\n]*\\([a-zA-Z_$][a-zA-Z0-9_$]*\\|\\*\\)\\)?" (wrong-type-argument (sequencep 7)) (("var plain" 0 ("var plain" nil nil "var" "plain" nil)) (" public static var count:int" 0 (" public static var count:int" "public" "static" "var" "count" "int")) ("private const NAME:*" 0 ("private const NAME:*" "private" nil "const" "NAME" "*")) ("static protected const value : Widget" 0 ("static protected const value : Widget" "static" "protected" "const" "value" "Widget")) ("public public public var tooMany:int" nil nil) ("let nope:int" nil nil) ("x var notAtLineStart" nil nil)) ("count" "count" "count"))"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}

#[test]
fn actionscript_mode_imenu_initializer_sets_exact_buffer_local_expression_and_case_policy() {
    let elisp_form = r##"(let ((expression
                '((functions
                   "^function \\([A-Za-z_$][A-Za-z0-9_$]*\\)"
                   1))))
         (with-temp-buffer
           (list
            (local-variable-p
             'imenu-generic-expression)
            (local-variable-p
             'imenu-case-fold-search)
            (as-imenu-init
             expression)
            imenu-generic-expression
            imenu-case-fold-search
            (local-variable-p
             'imenu-generic-expression)
            (local-variable-p
             'imenu-case-fold-search))))"##;
    let expect = expect![[
        r#"OK (nil nil nil ((functions "^function \\([A-Za-z_$][A-Za-z0-9_$]*\\)" 1)) nil t t)"#
    ]];
    assert_actionscript_mode_parity(elisp_form, expect);
}
