use expect_test::expect;

use super::assert_ac_rtags_parity;

#[test]
fn ac_rtags_trim_whitespace_handles_blanks_tabs_empty_internal_and_newline_edges() {
    let elisp_form = r##"(mapcar
               #'ac-rtags-trim-leading-trailing-whitespace
               '("  alpha  "
                 "\talpha beta\t"
                 ""
                 "   "
                 "alpha  beta"
                 "\n alpha \n"
                 " alpha "))"##;
    let expect =
        expect![[r#"OK ("alpha" "alpha beta" "" "" "alpha  beta" "\n alpha \n" "alpha")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_action_dispatches_all_function_namespace_other_and_expand_disabled_types() {
    let elisp_form = r##"(let ((types
                    '("CXXMethod"
                      "FunctionDecl"
                      "FunctionTemplate"
                      "Namespace"
                      "NamespaceAlias"
                      "VarDecl"))
                   (ac-rtags-expand-functions
                    t)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-rtags-action-function)
                     (lambda (tag)
                       (push
                        (list
                         'function-call tag)
                        calls)
                       'function-result))
                    ((symbol-function
                      'ac-rtags-action-namespace)
                     (lambda (tag)
                       (push
                        (list
                         'namespace-call tag)
                        calls)
                       'namespace-result)))
                 (let ((enabled
                        (mapcar
                         (lambda (type)
                           (let ((ac-last-completion
                                  (cons
                                   "prefix"
                                   (propertize
                                    "candidate"
                                    'ac-rtags-full
                                    "full"
                                    'ac-rtags-type
                                    type))))
                             (ac-rtags-action)))
                         types)))
                   (setq
                    ac-rtags-expand-functions
                    nil)
                   (list
                    enabled
                    (let ((ac-last-completion
                           (cons
                            "prefix"
                            (propertize
                             "candidate"
                             'ac-rtags-full
                             "disabled function"
                             'ac-rtags-type
                             "FunctionDecl"))))
                      (ac-rtags-action))
                    (let ((ac-last-completion
                           (cons
                            "prefix"
                            (propertize
                             "candidate"
                             'ac-rtags-full
                             "disabled namespace"
                             'ac-rtags-type
                             "NamespaceAlias"))))
                      (ac-rtags-action))
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((function-result function-result function-result namespace-result namespace-result nil) nil namespace-result ((function-call "full") (function-call "full") (function-call "full") (namespace-call "full") (namespace-call "full") (namespace-call "disabled namespace")))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_action_function_plain_insertion_formats_arguments_and_places_point_inside_parentheses()
{
    let elisp_form = r##"(mapcar
               (lambda (signature)
                 (with-temp-buffer
                   (insert
                    "call")
                   (goto-char
                    (point-max))
                   (let ((features
                          (delq
                           'yasnippet
                           (copy-sequence
                            features))))
                     (list
                      signature
                      (ac-rtags-action-function
                       signature)
                      (buffer-string)
                      (point)))))
               '("void fn( int x, const T& y ) const"
                 "void fn()"
                 "no-parentheses"
                 "void fn( one ,  two,three )"))"##;
    let expect = expect![[
        r#"OK (("void fn( int x, const T& y ) const" nil "call(int x, const T& y)" 6) ("void fn()" nil "call()" 6) ("no-parentheses" nil "call(no-parentheses)" 6) ("void fn( one ,  two,three )" nil "call(one, two, three)" 6))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_action_function_yasnippet_wraps_each_argument_and_forwards_exact_snippet() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'featurep)
                     (lambda (feature)
                       (eq
                        feature
                        'yasnippet)))
                    ((symbol-function
                      'yas-expand-snippet)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'expanded)))
                 (list
                  (ac-rtags-action-function
                   "Result make( int x, const T& value )")
                  (ac-rtags-action-function
                   "void empty()")
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (expanded expanded (("(${int x}, ${const T& value})") ("()")))"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_namespace_action_inserts_scope_operator_and_ignores_original_tag() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "std")
               (list
                (ac-rtags-action-namespace
                 "ignored")
                (buffer-string)
                (point)))"##;
    let expect = expect![[r#"OK (nil "std::" 6)"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_prefix_prefers_auto_complete_symbol_then_handles_member_operators_and_plain_text() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (car fixture))
                   (goto-char
                    (point-max))
                   (cl-letf
                       (((symbol-function
                          'ac-prefix-symbol)
                         (lambda ()
                           (cadr fixture))))
                     (list
                      (car fixture)
                      (cadr fixture)
                      (ac-rtags-prefix)
                      (point)))))
               '(("object."
                  nil)
                 ("pointer->"
                  nil)
                 ("Type::"
                  nil)
                 ("plain"
                  nil)
                 ("anything"
                  3)
                 ("single:"
                  nil)
                 ("greater>"
                  nil)))"##;
    let expect = expect![[
        r#"OK (("object." nil 8 8) ("pointer->" nil 10 10) ("Type::" nil 7 7) ("plain" nil nil 6) ("anything" 3 3 9) ("single:" nil nil 8) ("greater>" nil nil 9))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_init_and_completion_hook_return_exact_noop_and_start_values() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-start)
                     (lambda ()
                       (push
                        'start
                        calls)
                       'started)))
                 (list
                  (ac-rtags-init)
                  (ac-rtags-completions-hook)
                  (nreverse calls))))"##;
    let expect = expect!["OK (nil started (start))"];

    assert_ac_rtags_parity(elisp_form, expect);
}
