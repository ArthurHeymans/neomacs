use expect_test::expect;

use super::{assert_ac_js2_parity, assert_ac_js2_signal_parity};

#[test]
fn ac_js2_has_function_calls_distinguishes_references_declarations_and_calls() {
    let elisp_form = r##"(mapcar
               (lambda (source)
                 (list
                  source
                  (ac-js2-has-function-calls
                   source)))
               '("var value = 1;"
                 "function declared(arg) { return arg; }"
                 "declared"
                 "declared()"
                 "object.method(1)"
                 "(function () { return 1; })()"))"##;
    let expect = expect![[
        r#"OK (("var value = 1;" nil) ("function declared(arg) { return arg; }" nil) ("declared" nil) ("declared()" t) ("object.method(1)" t) ("(function () { return 1; })()" t))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_add_extra_completions_obeys_each_flag_and_caches_keyword_strings() {
    let elisp_form = r##"(let ((js2-keywords
                    '(if return))
                   (js2-ecma-262-externs
                    '("Array" "Object"))
                   (js2-browser-externs
                    '("window" "document"))
                   (ac-js2-keywords
                    nil)
                   (ac-js2-add-keywords
                    t)
                   (ac-js2-add-ecma-262-externs
                    t)
                   (ac-js2-add-browser-externs
                    t))
               (let ((all
                      (ac-js2-add-extra-completions
                       '("local"))))
                 (setq
                  js2-keywords
                  '(changed)
                  ac-js2-add-ecma-262-externs
                  nil
                  ac-js2-add-browser-externs
                  nil)
                 (let ((cached-keywords
                        (ac-js2-add-extra-completions
                         nil)))
                   (setq
                    ac-js2-add-keywords nil
                    ac-js2-add-browser-externs t)
                   (list
                    all
                    cached-keywords
                    (ac-js2-add-extra-completions
                     '("base"))
                    ac-js2-keywords))))"##;
    let expect = expect![[
        r#"OK (("local" "if" "return" "Array" "Object" . #1=("window" "document")) ("if" "return") ("base" . #1#) ("if" "return"))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_format_function_accepts_only_function_prefixes_and_stops_at_first_close_paren() {
    let elisp_form = r##"(mapcar
               #'ac-js2-format-function
               '("function alpha(first, second) { return first; }"
                 "function () {\n  return 1;\n}"
                 "function nested(callback = () => value) { return callback; }"
                 "prefix function ignored()"
                 ""
                 nil
                 42))"##;
    let expect = expect![[
        r#"OK ("function alpha(first, second)" "function ()" "function nested(callback = ()" nil nil nil nil)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_format_function_signals_for_a_function_prefix_without_a_close_paren() {
    let elisp_form = r##"(ac-js2-format-function
               "function malformed")"##;
    let expect = expect![[r#"ERR (wrong-type-argument number-or-marker-p nil)"#]];

    assert_ac_js2_signal_parity(elisp_form, expect);
}

#[test]
fn ac_js2_format_comment_strips_leading_comment_decoration_and_trailing_space() {
    let elisp_form = r##"(mapcar
               #'ac-js2-format-comment
               '("// single line   "
                 "/* block line */"
                 "\n  /**\n   * first\n   * second\n   */"
                 "plain text"
                 ""))"##;
    let expect =
        expect![[r#"OK ("single line  " "block line */" "first\nsecond\n" "plain text" "")"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_real_js2_ast_formats_variables_functions_and_object_literals() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var plain = 42;\nvar callable = function(first, second) { return first; };\nfunction declared(arg) { return arg; }\nvar object = {alpha: 1, beta: function(value) { return value; }, empty: {}};\n")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (goto-char
                (point-max))
               (mapcar
                (lambda (name)
                  (let ((node
                         (ac-js2-initialized-node
                          name)))
                    (list
                     name
                     (cond
                      ((js2-function-node-p
                        node)
                       'function)
                      ((js2-object-node-p
                        node)
                       'object)
                      ((js2-number-node-p
                        node)
                       'number)
                      (node 'other)
                      (t nil))
                     (ac-js2-format-node
                      name node))))
                '("plain"
                  "callable"
                  "declared"
                  "object"
                  "missing")))"##;
    let expect = expect![[
        r#"OK (("plain" number ("plain" . "42")) ("callable" function ("callable" . "function(first, second)")) ("declared" function ("declared" . "function declared(arg)")) ("object" object ("object" . "alpha : 1\nbeta : function(value)\nempty : {}")) ("missing" nil ("missing" . "")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_format_object_node_doc_rejects_a_non_object_node() {
    let elisp_form = r##"(ac-js2-format-object-node-doc
               (make-js2-name-node
                :pos 0
                :len 4
                :name "name"))"##;
    let expect = expect![[r#"ERR (error "Node is not an object node")"#]];

    assert_ac_js2_signal_parity(elisp_form, expect);
}

#[test]
fn ac_js2_format_object_property_doc_rejects_a_non_property_node() {
    let elisp_form = r##"(ac-js2-format-js2-object-prop-doc
               (make-js2-name-node
                :pos 0
                :len 4
                :name "name"))"##;
    let expect = expect![[r#"ERR (error "Node is not an object property node")"#]];

    assert_ac_js2_signal_parity(elisp_form, expect);
}
