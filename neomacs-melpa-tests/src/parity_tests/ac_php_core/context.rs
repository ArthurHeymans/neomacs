use expect_test::expect;

use super::assert_ac_php_core_parity;

#[test]
fn ac_php_core_class_at_point_resolves_this_static_parent_annotations_parameters_and_instanceof() {
    let elisp_form = r##"(let ((functions
                    (make-hash-table
                     :test
                     'case-fold))
                   (tags-data
                    nil))
               (dolist
                   (item
                    '(["c" "\\App\\Current" "" "0:1" "\\App\\Current"]
                      ["c" "\\App\\Service" "" "0:2" "\\App\\Service"]
                      ["c" "\\Acme\\Request" "" "0:3" "\\Acme\\Request"]
                      ["c" "\\Extension" "" "0:4" "\\Extension"]
                      ["c" "\\RuntimeException" "" "0:5" "\\RuntimeException"]))
                 (puthash
                  (aref item 1)
                  item
                  functions))
               (setq
                tags-data
                (list
                 (make-hash-table
                  :test
                  'case-fold)
                 functions
                 (make-hash-table
                  :test
                  'case-fold)
                 []
                 "/project/"))
               (mapcar
                (lambda (fixture)
                  (with-temp-buffer
                    (insert
                     (cadr fixture))
                    (php-mode)
                    (goto-char
                     (point-min))
                    (search-forward
                     "§")
                    (delete-char
                     -1)
                    (list
                     (car fixture)
                     (ac-php-get-class-at-point
                      tags-data)
                     (point))))
                '((this
                   "<?php\nnamespace App;\nclass Current {\n function demo() { $this->value§; }\n}")
                  (self
                   "<?php\nnamespace App;\nclass Current {\n function demo() { self::instance()->value§; }\n}")
                  (static
                   "<?php\nnamespace App;\nclass Current {\n function demo() { static::instance()->value§; }\n}")
                  (parent
                   "<?php\nnamespace App;\nclass Current {\n function demo() { parent::base()->value§; }\n}")
                  (named-static
                   "<?php\nnamespace App;\nclass Current {\n function demo() { Service::make()->value§; }\n}")
                  (annotation
                   "<?php\nnamespace App;\nclass Current {\n function demo() {\n  /** @var \\Extension $extension */\n  $extension->value§;\n }\n}")
                  (parameter
                   "<?php\nnamespace App;\nclass Current {\n function demo(\\Acme\\Request $request) { $request->value§; }\n}")
                  (instanceof
                   "<?php\nnamespace App;\nclass Current {\n function demo($error) { if ($error instanceof \\RuntimeException) { $error->value§; } }\n}"))))"##;
    let expect = expect![[
        r#"OK ((this "\\App\\Current.value" 69) (self "\\App\\Current.instance(.value" 80) (static "\\App\\Current.instance(.value" 82) (parent #("\\App\\Current.__parent__.base(.value" 1 4 (pos 7) 5 12 (pos 22)) 78) (named-static "\\App\\Service.make(.value" 79) (annotation "\\Extension.value" 112) (parameter "\\Acme\\Request.value" 94) (instanceof "\\RuntimeException.value" 119))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_class_at_point_handles_multiline_chains_callable_arrays_and_suppressed_contexts() {
    let elisp_form = r##"(let ((functions
                    (make-hash-table
                     :test
                     'case-fold))
                   (tags-data
                    nil))
               (dolist
                   (item
                    '(["c" "\\App\\Current" "" "0:1" "\\App\\Current"]
                      ["c" "\\Acme\\Handler" "" "0:2" "\\Acme\\Handler"]))
                 (puthash
                  (aref item 1)
                  item
                  functions))
               (setq
                tags-data
                (list
                 (make-hash-table
                  :test
                  'case-fold)
                 functions
                 (make-hash-table
                  :test
                  'case-fold)
                 []
                 "/project/"))
               (mapcar
                (lambda (fixture)
                  (with-temp-buffer
                    (insert
                     (cadr fixture))
                    (php-mode)
                    (goto-char
                     (point-min))
                    (search-forward
                     "§")
                    (delete-char
                     -1)
                    (list
                     (car fixture)
                     (ac-php-get-class-at-point
                      tags-data))))
                '((multiline
                   "<?php\nnamespace App;\nclass Current {\n function demo() {\n  $this->first()\n    ->second()\n    ->value§;\n }\n}")
                  (array-callable
                   "<?php\nnamespace App;\nclass Current {\n function demo(\\Acme\\Handler $handler) { $callback = [$handler, \"run§\"]; }\n}")
                  (function-callable
                   "<?php\nnamespace App;\nclass Current {\n function demo(\\Acme\\Handler $handler) { $callback = array($handler, 'run§'); }\n}")
                  (string
                   "<?php\nnamespace App;\nclass Current {\n function demo() { $text = \"$this->value§\"; }\n}")
                  (comment
                   "<?php\nnamespace App;\nclass Current {\n function demo() { // $this->value§\n }\n}")
                  (empty
                   "<?php\nnamespace App;\nclass Current {\n function demo() { §\n }\n}"))))"##;
    let expect = expect![[
        r#"OK ((multiline "\\App\\Current.first(.second(.value") (array-callable "\\Acme\\Handler.run") (function-callable "\\Acme\\Handler.run") (string nil) (comment nil) (empty nil))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_class_at_point_follows_assignment_symbol_return_types_for_functions_and_properties()
{
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nnamespace App;\nclass Current {\n function demo() {\n  $service = makeService();\n  $service->value§;\n }\n}")
               (php-mode)
               (goto-char
                (point-min))
               (search-forward
                "§")
               (delete-char
                -1)
               (let* ((functions
                       (make-hash-table
                        :test
                        'case-fold))
                      (tags-data
                       (list
                        (make-hash-table
                         :test
                         'case-fold)
                        functions
                        (make-hash-table
                         :test
                         'case-fold)
                        []
                        "/project/"))
                      calls)
                 (puthash
                  "\\App\\Service"
                  ["c" "\\App\\Service" "" "0:1" "\\App\\Service"]
                  functions)
                 (cl-letf
                     (((symbol-function
                        'ac-php-find-symbol-at-point-pri)
                       (lambda
                           (_tags
                            &optional
                            as-function
                            as-identifier)
                         (push
                          (list
                           (point)
                           as-function
                           as-identifier)
                          calls)
                         (list
                          "user_function"
                          "0:2"
                          "\\App\\Service"
                          ["f" "\\App\\makeService(" "" "0:2" "\\App\\Service"]))))
                   (list
                    (ac-php-get-class-at-point
                     tags-data)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK ("\\App\\Service.value" ((80 nil nil)))"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}
