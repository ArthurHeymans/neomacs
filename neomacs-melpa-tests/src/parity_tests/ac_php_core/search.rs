use expect_test::expect;

use super::assert_ac_php_core_parity;

#[test]
fn ac_php_core_ports_every_upstream_classlike_search_ert_fixture() {
    let elisp_form = r##"(mapcar
               (lambda (source)
                 (with-temp-buffer
                   (insert source)
                   (php-mode)
                   (goto-char
                    (point-max))
                   (list
                    source
                    (ac-php-get-cur-class-name))))
               '("class Abcde {}"
                 "<?php class Abcde {}"
                 "final class AdminLoginJob {}"
                 "abstract class Post {}"
                 "trait SharePost {}"
                 "class abcde {}"
                 "class Swift_SmtpTransport_2 {}"
                 "class Foo {}\nclass Bar {}"
                 "                   class             Abcde {}"
                 "class Ÿ¼ÿ©¥ {}"
                 "class !Ÿ¼ÿ©¥ {}"))"##;
    let expect = expect![[
        r#"OK (("class Abcde {}" #("Abcde" 0 5 (pos 1))) ("<?php class Abcde {}" #("Abcde" 0 5 (pos 1))) ("final class AdminLoginJob {}" #("AdminLoginJob" 0 13 (pos 1))) ("abstract class Post {}" #("Post" 0 4 (pos 1))) ("trait SharePost {}" #("SharePost" 0 9 (pos 1))) ("class abcde {}" #("abcde" 0 5 (pos 1))) ("class Swift_SmtpTransport_2 {}" #("Swift_SmtpTransport_2" 0 21 (pos 1))) ("class Foo {}\nclass Bar {}" #("Bar" 0 3 (pos 14))) ("                   class             Abcde {}" #("Abcde" 0 5 (pos 1))) ("class Ÿ¼ÿ©¥ {}" #("Ÿ¼ÿ©¥" 0 5 (pos 1))) ("class !Ÿ¼ÿ©¥ {}" nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_ports_every_upstream_namespace_search_ert_fixture() {
    let elisp_form = r##"(mapcar
               (lambda (source)
                 (with-temp-buffer
                   (insert source)
                   (php-mode)
                   (goto-char
                    (point-max))
                   (list
                    source
                    (ac-php-get-cur-namespace-name)
                    (ac-php-get-cur-namespace-name
                     t))))
               '("namespace Acme;"
                 "<?php namespace Acme;"
                 "namespace PhpParser\\Node_2\\Class_;"
                 "namespace \\Acme;"
                 "namespace Acme\\Foo\\;"
                 "                    namespace              Acme                 ;"
                 "namespace Ÿ¼ÿ©¥;"
                 "namespace !Ÿ¼ÿ©¥;"
                 "class Foo;"))"##;
    let expect = expect![[
        r#"OK (("namespace Acme;" #("\\Acme\\" 1 5 (pos 1)) #("\\Acme" 1 5 (pos 1))) ("<?php namespace Acme;" #("\\Acme\\" 1 5 (pos 1)) #("\\Acme" 1 5 (pos 1))) ("namespace PhpParser\\Node_2\\Class_;" #("\\PhpParser\\Node_2\\Class_\\" 1 24 (pos 1)) #("\\PhpParser\\Node_2\\Class_" 1 24 (pos 1))) ("namespace \\Acme;" #("\\Acme\\" 0 5 (pos 1)) #("\\Acme" 0 5 (pos 1))) ("namespace Acme\\Foo\\;" #("\\Acme\\Foo\\" 1 9 (pos 1)) #("\\Acme\\Foo" 1 9 (pos 1))) ("                    namespace              Acme                 ;" #("\\Acme\\" 1 5 (pos 1)) #("\\Acme" 1 5 (pos 1))) ("namespace Ÿ¼ÿ©¥;" #("\\Ÿ¼ÿ©¥\\" 1 6 (pos 1)) #("\\Ÿ¼ÿ©¥" 1 6 (pos 1))) ("namespace !Ÿ¼ÿ©¥;" "" "") ("class Foo;" "" ""))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_ports_every_upstream_fully_qualified_class_search_ert_fixture() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (car fixture))
                   (php-mode)
                   (goto-char
                    (point-max))
                   (let ((case-fold-search
                          (if
                              (cdr fixture)
                              nil
                            case-fold-search)))
                     (list
                      (ac-php-get-cur-class-name)
                      (ac-php-get-cur-namespace-name)
                      (ac-php-get-cur-full-class-name)))))
               '(("class Foo;")
                 ("<?php\n\nnamespace Symfony\\Component\\Console\\Descriptor;\n\n/*\n * This file is part of the Symfony package.\n *\n * (c) Fabien Potencier <fabien@symfony.com>\n *\n * For the full copyright and license information, please view the LICENSE\n * file that was distributed with this source code.\n */\n\nclass JsonDescriptor;"
                  . case-sensitive)
                 ("<?php namespace Acme;\n\nfunction helper() {}"
                  . case-sensitive)))"##;
    let expect = expect![[
        r#"OK ((#("Foo" 0 3 (pos 1)) "" #("\\Foo" 1 4 (pos 1))) (#("JsonDescriptor" 0 14 (pos 288)) #("\\Symfony\\Component\\Console\\Descriptor\\" 1 37 (pos 8)) #("\\Symfony\\Component\\Console\\Descriptor\\JsonDescriptor" 1 37 (pos 8) 38 52 (pos 288))) (nil #("\\Acme\\" 1 5 (pos 1)) nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_ports_every_upstream_annotated_variable_scope_ert_fixture() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (cadr fixture))
                   (php-mode)
                   (let ((case-fold-search
                          nil))
                     (goto-char
                      (pcase
                          (car fixture)
                        ('end
                         (point-max))
                        ('before-end
                         (1-
                          (point-max)))))
                     (list
                      (caddr fixture)
                      (point)
                      (ac-php-get-annotated-var-class
                       "extension")))))
               '((before-end
                  "<?php\n\nfunction hello() {\n    /** @var Extension $extension */\n}\n"
                  standard)
                 (before-end
                  "<?php\n\nfunction hello() {\n    /** @var \\Symfony\\Component\\Console\\Descriptor\\JsonDescriptor $extension */\n}\n"
                  complex)
                 (end
                  "<?php\n\nfunction hello() {\n    /** @var Extension $variable */\n}\n"
                  not-found)
                 (end
                  "<?php\n\nfunction hello() {\n    /** @var Extension $extension */\n}\n\nfunction test() {\n\n}\n"
                  out-of-scope)
                 (end
                  "<?php\n\nfunction hello() {\n    /** @var Extension $extension */\n}\n\nfunction test() {\n    /** @var Fake $extension */\n}\n"
                  incorrect-scope-at-end)
                 (before-end
                  "<?php\n\nfunction hello() {\n    /** @var Extension $extension */\n}\n\nfunction test() {\n    /** @var Fake $extension */\n}\n"
                  incorrect-scope-before-end)
                 (end
                  "<?php\n\n/** @var Fake $extension */\n$extension->bar();\n\nfunction hello($extension) {\n    /** @var Extension $extension */\n    $extension->foo();\n}\n\n$extension->bar();\n"
                  out-of-defun)))"##;
    let expect = expect![[
        r#"OK ((standard 65 #("Extension" 0 9 (pos 35))) (complex 108 #("\\Symfony\\Component\\Console\\Descriptor\\JsonDescriptor" 0 52 (pos 35))) (not-found 65 nil) (out-of-scope 88 nil) (incorrect-scope-at-end 119 nil) (incorrect-scope-before-end 118 #("Fake" 0 4 (pos 93))) (out-of-defun 167 #("Fake" 0 4 (pos 12))))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_ports_upstream_function_boundary_ert_fixture_and_movement() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nfunction foo () {\n\n}\n")
               (php-mode)
               (let ((case-fold-search
                      nil)
                     (positions
                      (mapcar
                       (lambda (position)
                         (list
                          position
                          (ac-php--in-function-p
                           position)))
                       (list
                        (point-max)
                        (1-
                         (point-max))
                        1
                        24))))
                 (goto-char
                  (point-max))
                 (let ((backward-one
                        (list
                         (ac-php--beginning-of-defun)
                         (point))))
                   (goto-char
                    (point-min))
                   (let ((forward-one
                          (list
                           (ac-php--end-of-defun)
                           (point))))
                     (list
                      positions
                      (ac-php--in-function-p
                       24)
                      backward-one
                      forward-one)))))"##;
    let expect = expect!["OK (((28 nil) (27 t) (1 nil) (24 t)) t (t 7) (nil 28))"];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_syntax_backward_filters_comment_string_and_function_contexts() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\n/** @var Outside $outside */\nfunction demo(Foo $arg) {\n  $literal = \"Foo $string\";\n  /** @var Inside $inside */\n  $inside->run();\n}\nFoo $tail;\n")
               (php-mode)
               (goto-char
                (point-max))
               (mapcar
                (lambda (arguments)
                  (let ((value
                         (apply
                          #'ac-php-get-syntax-backward
                          arguments)))
                    (list
                     (and value
                          (substring-no-properties
                           value))
                     (and value
                          (get-text-property
                           0
                           'pos
                           value)))))
                '(("Foo\\s-+\\$tail" :sexp 0)
                  ("@var\\s-+\\([A-Za-z]+\\)\\s-+\\$outside" :sexp 1 :comment t)
                  ("@var\\s-+\\([A-Za-z]+\\)\\s-+\\$inside" :sexp 1 :comment t :defun t)
                  ("Foo\\s-+\\$arg" :sexp 0 :defun t)
                  ("Foo\\s-+\\$string" :sexp 0 :defun t))))"##;
    let expect = expect![[
        r#"OK (("Foo $tail" 139) ("Outside" 11) ("Inside" 96) ("Foo $arg" 50) (nil nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_use_alias_and_namespace_cleanup_helpers_cover_all_forms() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace Current\\Space;\nuse Vendor\\Package\\Thing;\nuse Vendor\\Other\\Service as Alias;\nuse function Vendor\\Helpers\\run;\nclass Demo {}\n")
               (php-mode)
               (goto-char
                (point-max))
               (list
                (mapcar
                 #'ac-php-get-use-as-name
                 '("Thing"
                   "Alias"
                   "Alias("
                   "Missing"))
                (ac-php--get-all-use-as-name-in-cur-buffer)
                (mapcar
                 #'ac-php-clean-namespace-name
                 '("\\Acme"
                   "\\"
                   "Plain"
                   ""
                   nil))
                (mapcar
                 #'ac-php--get-namespace-from-classname
                 '("\\Acme\\Service"
                   "\\Root"
                   "Plain"))))"##;
    let expect = expect![[
        r#"OK ((#("Vendor\\Package\\Thing" 0 20 (pos 32)) #("Vendor\\Other\\Service" 0 20 (pos 58)) #("Vendor\\Other\\Service" 0 20 (pos 58)) nil) (("\\Vendor\\Other\\Service" "Alias") ("\\Vendor\\Package\\Thing" "Thing")) ("Acme" "\\" "Plain" "" nil) ("\\Acme" "" nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}
