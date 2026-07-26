use expect_test::expect;

use super::assert_abs_mode_parity;

#[test]
fn abs_mode_inside_string_or_comment_predicate_recognizes_templates_comments_and_code() {
    let elisp_form = r##"(with-temp-buffer
               (set-syntax-table abs-mode-syntax-table)
               (insert
                "value = 1; // comment text\n"
                "text = `template text`;\n"
                "/* block text */ value = 2;\n")
               (mapcar
                (lambda (needle)
                  (goto-char (point-min))
                  (search-forward needle)
                  (list
                   needle
                   (not
                    (null
                     (abs--inside-string-or-comment-p)))))
                '("value = 1" "comment" "template"
                  "block" "value = 2")))"##;
    let expect = expect![[
        r#"OK (("value = 1" nil) ("comment" t) ("template" t) ("block" t) ("value = 2" nil))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_beginning_of_definition_skips_tokens_inside_comments_and_strings() {
    let elisp_form = r##"(with-temp-buffer
               (set-syntax-table abs-mode-syntax-table)
               (insert
                "interface First {}\n"
                "// class Commented {}\n"
                "String text = `def fake()`;\n"
                "  class Real {\n"
                "    Unit method() { skip; }\n"
                "  }\n")
               (goto-char (point-max))
               (list
                (abs-beginning-of-definition)
                (point)
                (buffer-substring-no-properties
                 (line-beginning-position)
                 (line-end-position))
                (progn
                  (abs-beginning-of-definition)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position)))))"##;
    let expect = expect![[r#"OK (70 70 "  class Real {" "interface First {}")"#]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_end_of_definition_finds_closing_brace_or_semicolon_before_the_next_definition() {
    let elisp_form = r##"(with-temp-buffer
               (set-syntax-table abs-mode-syntax-table)
               (insert
                "def Int first() = 1;\n"
                "class Second {\n"
                "  String text = \"};\";\n"
                "  Unit method() { skip; }\n"
                "}\n"
                "data Third = T;\n")
               (goto-char (point-min))
               (list
                (progn
                  (goto-char (point-min))
                  (abs-end-of-definition)
                  (list (point) (char-before)))
                (progn
                  (goto-char (point-min))
                  (search-forward "class Second")
                  (abs-end-of-definition)
                  (list (point) (char-before)))
                (progn
                  (goto-char (point-min))
                  (search-forward "data Third")
                  (abs-end-of-definition)
                  (list (point) (char-before)))))"##;
    let expect = expect!["OK ((86 125) (102 59) (102 59))"];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_initialization_sets_cc_imenu_outline_syntax_keymap_and_snippet_state() {
    let elisp_form = r##"(let ((abs-mode-hook nil)
                    (yas-snippet-dirs nil)
                    events)
               (cl-letf
                   (((symbol-function
                      'yas--load-snippet-dirs)
                     (lambda ()
                       (push '(load-snippets) events)))
                    ((symbol-function
                      'speedbar-add-supported-extension)
                     (lambda (extension)
                       (push
                        (list 'speedbar extension)
                        events))))
                 (with-temp-buffer
                   (abs-mode)
                   (list
                    major-mode
                    mode-name
                    c-buffer-is-cc-mode
                    c-basic-offset
                    comment-start
                    comment-end
                    comment-start-skip
                    font-lock-defaults
                    (eq imenu-generic-expression
                        abs-imenu-generic-expression)
                    imenu-syntax-alist
                    outline-regexp
                    (functionp outline-level)
                    outline-minor-mode
                    (lookup-key
                     (current-local-map)
                     (kbd "C-c C-c"))
                    (memq
                     abs--yas-snippets-dir
                     yas-snippet-dirs)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (abs-mode "Abs//l" abs-mode 4 "//" "" "//+\\s-*" ((abs-font-lock-keywords abs-font-lock-keywords-1 abs-font-lock-keywords-2 abs-font-lock-keywords-3) nil nil ((95 . "w") (36 . "w")) c-beginning-of-syntax (font-lock-mark-block-function . c-mark-function)) t (("." . "_")) "^\\(?:class\\|d\\(?:ata\\|e\\(?:f\\|lta\\)\\)\\|exception\\|\\(?:interfac\\|modul\\|typ\\)e\\)" t t abs-next-action ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/abs-mode/20260415.813/home/.emacs.d/elpa/abs-mode-20260415.813/snippets") ((speedbar ".abs") (load-snippets)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_imenu_extracts_every_supported_definition_category_and_qualified_module() {
    let elisp_form = r##"(let ((abs-mode-hook nil))
               (cl-letf
                   (((symbol-function
                      'yas--load-snippet-dirs)
                     (lambda () nil))
                    ((symbol-function
                      'speedbar-add-supported-extension)
                     (lambda (&rest _) nil)))
                 (with-temp-buffer
                   (insert
                    "module Demo.Qualified;\n"
                    "delta Change;\n"
                    "def List<Int> compute() = Nil;\n"
                    "data Result = Ok;\n"
                    "type Alias = Int;\n"
                    "exception Failed;\n"
                    "class Worker {}\n"
                    "interface Service {}\n")
                   (abs-mode)
                   (imenu--generic-function
                    imenu-generic-expression))))"##;
    let expect = expect![[
        r#"OK (("Modules" ("Demo.Qualified" :marker nil nil)) ("Interfaces" ("Service" :marker nil nil)) ("Classes" ("Worker" :marker nil nil)) ("Exceptions" ("Failed" :marker nil nil)) ("Datatypes" ("Result" :marker nil nil) ("Alias" :marker nil nil)) ("Functions" ("compute" :marker nil nil)) ("Deltas" ("Change" :marker nil nil)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_outline_level_uses_current_indentation_and_obsolete_indent_alias() {
    let elisp_form = r##"(with-temp-buffer
               (insert "        class Deep {}\n")
               (goto-char (point-min))
               (let ((c-basic-offset 4))
                 (list
                  (funcall abs--outline-level)
                  abs-indent
                  c-basic-offset)))"##;
    let expect = expect!["OK (3 4 4)"];

    assert_abs_mode_parity(elisp_form, expect);
}
