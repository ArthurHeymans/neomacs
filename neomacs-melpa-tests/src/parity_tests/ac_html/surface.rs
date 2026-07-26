use expect_test::expect;

use super::assert_ac_html_parity;

#[test]
fn ac_html_exact_pin_dependencies_features_group_and_defaults_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-html package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-html
                   ac-html-core
                   auto-complete
                   cl-lib
                   dash))
                (get 'auto-complete-html
                     'group-documentation)
                (get 'auto-complete-html
                     'custom-prefix)
                ac-html-data-providers
                ac-html-enabled-data-providers
                ac-html-current-tag-function
                ac-html-current-attr-function))"##;
    let expect = expect![[
        r#"OK (ac-html "20151005.731" ((auto-complete (1 4)) (s (1 9)) (f (0 17)) (dash (2 10))) (t t t t t) "HTML Auto Complete." "ac-html-" nil nil nil nil)"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_core_function_arities_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)))
               '(ac-html-enable-data-provider
                 ac-html-query-data-provider
                 ac-html-all-tag-candidates
                 ac-html-all-attr-candidates
                 ac-html-all-attrv-candidates
                 ac-html-all-id-candidates
                 ac-html-all-class-candidates
                 ac-html-tag-documentation
                 ac-html-attr-documentation
                 ac-html-attrv-documentation
                 ac-html-id-documentation
                 ac-html-class-documentation))"##;
    let expect = expect![[
        r#"OK ((ac-html-enable-data-provider (provider) nil "Enable data provider PROVIDER.") (ac-html-query-data-provider (provider key) nil nil) (ac-html-all-tag-candidates nil nil "All tag candidates get from data providers.") (ac-html-all-attr-candidates nil nil "All attr candidates get from data providers.") (ac-html-all-attrv-candidates nil nil "All attrv candidates get from data providers.") (ac-html-all-id-candidates nil nil "") (ac-html-all-class-candidates nil nil "") (ac-html-tag-documentation (tag) nil "Not documented yet.") (ac-html-attr-documentation (attr) nil "Not documented yet.") (ac-html-attrv-documentation (attrv) nil "Not documented yet.") (ac-html-id-documentation (id) nil "") (ac-html-class-documentation (class) nil ""))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_language_function_arities_and_documentation_match() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (require 'ac-jade)
               (require 'ac-haml)
               (mapcar
                (lambda (function)
                  (list
                   function
                   (help-function-arglist
                    function t)
                   (documentation function t)))
                '(ac-html--inside-attrv
                  ac-html--inside-comment
                  ac-html-tag-prefix
                  ac-html-attr-prefix
                  ac-html-value-prefix
                  ac-html-current-tag
                  ac-html-current-attr
                  ac-slim-inside-ruby-code
                  ac-slim-inside-non-slim-block
                  ac-slim-current-tag
                  ac-slim-current-attr
                  ac-jade-current-tag
                  ac-jade-current-attr
                  ac-haml-current-tag
                  ac-haml-current-attr)))"##;
    let expect = expect![[
        r#"OK ((ac-html--inside-attrv nil "Return t if cursor inside attrv aka string.\nHas bug for quoted quote.") (ac-html--inside-comment nil nil) (ac-html-tag-prefix nil nil) (ac-html-attr-prefix nil nil) (ac-html-value-prefix nil nil) (ac-html-current-tag nil "Return current html tag user is typing on.\nThere is a bug if attrv contains string like this <a") (ac-html-current-attr nil "Return current html tag's attribute user is typing on.\nThere is a bug if attrv contains string like this href=") (ac-slim-inside-ruby-code nil "Return t if inside ruby code.") (ac-slim-inside-non-slim-block nil "Return t if inside ruby block, coffee block.") (ac-slim-current-tag nil "Return current slim tag user is typing on.") (ac-slim-current-attr nil "Return current html tag's attribute user is typing on.") (ac-jade-current-tag nil "Return current jade tag user is typing on.") (ac-jade-current-attr nil "Return current html tag's attribute user is typing on.") (ac-haml-current-tag nil "Return current haml tag user is typing on.") (ac-haml-current-attr nil "Return current html tag's attribute user is typing on."))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_defines_exact_auto_complete_sources_and_setup_callbacks() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (require 'ac-jade)
               (require 'ac-haml)
               (list
                (mapcar
                 (lambda (symbol)
                   (list
                    symbol
                    (copy-tree
                     (symbol-value symbol))))
                 '(ac-source-html-tag
                   ac-source-html-attr
                   ac-source-html-attrv
                   ac-source-slim-tag
                   ac-source-slim-attr
                   ac-source-slim-attrv
                   ac-source-slim-id
                   ac-source-slim-class
                   ac-source-jade-tag
                   ac-source-jade-attr
                   ac-source-jade-attrv
                   ac-source-haml-tag
                   ac-source-haml-attr
                   ac-source-haml-attrv
                   ac-source-haml-id
                   ac-source-haml-class))
                (mapcar
                 (lambda (setup)
                   (with-temp-buffer
                     (funcall setup)
                     (list
                      setup
                      ac-html-current-tag-function
                      ac-html-current-attr-function)))
                 '(ac-html-setup
                   ac-slim-setup
                   ac-jade-setup
                   ac-haml-setup))))"##;
    let expect = expect![[
        r#"OK (((ac-source-html-tag ((candidates . ac-html-all-tag-candidates) (prefix . ac-html-tag-prefix) (document . ac-html-tag-documentation) (symbol . "t"))) (ac-source-html-attr ((candidates . ac-html-all-attr-candidates) (prefix . ac-html-attr-prefix) (document . ac-html-attr-documentation) (symbol . "a"))) (ac-source-html-attrv ((candidates . ac-html-all-attrv-candidates) (prefix . ac-html-value-prefix) (document . ac-html-attrv-documentation) (symbol . "v"))) (ac-source-slim-tag ((candidates . ac-html-all-tag-candidates) (prefix . ac-slim-tag-prefix) (document . ac-html-tag-documentation) (symbol . "t"))) (ac-source-slim-attr ((candidates . ac-html-all-attr-candidates) (prefix . ac-slim-attr-prefix) (document . ac-html-attr-documentation) (symbol . "a"))) (ac-source-slim-attrv ((candidates . ac-html-all-attrv-candidates) (prefix . ac-slim-attrv-prefix) (document . ac-html-attrv-documentation) (symbol . "v"))) (ac-source-slim-id ((candidates . ac-html-all-id-candidates) (prefix . ac-slim-id-prefix) (document . ac-html-id-documentation) (symbol . "i"))) (ac-source-slim-class ((candidates . ac-html-all-class-candidates) (prefix . ac-slim-class-prefix) (document . ac-html-class-documentation) (symbol . "c"))) (ac-source-jade-tag ((candidates . ac-html-all-tag-candidates) (prefix . "^[\11 ]*\\(.*\\)") (document . ac-html-tag-documentation) (symbol . "t"))) (ac-source-jade-attr ((candidates . ac-html-all-attr-candidates) (prefix . "\\(?:,\\|(\\)[ ]*\\(.*\\)") (document . ac-html-attr-documentation) (symbol . "a"))) (ac-source-jade-attrv ((candidates . ac-html-all-attrv-candidates) (prefix . ac-jade-attrv-prefix) (document . ac-html-attrv-documentation) (symbol . "v"))) (ac-source-haml-tag ((candidates . ac-html-all-tag-candidates) (prefix . "^[ \11]*%\\(.*\\)") (document . ac-html-tag-documentation) (symbol . "t"))) (ac-source-haml-attr ((candidates . ac-html-all-attr-candidates) (prefix . ac-haml-attr-prefix) (document . ac-html-attr-documentation) (symbol . "a"))) (ac-source-haml-attrv ((candidates . ac-html-all-attrv-candidates) (prefix . ac-haml-attrv-prefix) (document . ac-html-attrv-documentation) (symbol . "v"))) (ac-source-haml-id ((candidates . ac-html-all-id-candidates) (prefix . ac-haml-id-prefix) (document . ac-html-id-documentation) (symbol . "i"))) (ac-source-haml-class ((candidates . ac-html-all-class-candidates) (prefix . ac-haml-class-prefix) (document . ac-html-class-documentation) (symbol . "c")))) ((ac-html-setup ac-html-current-tag ac-html-current-attr) (ac-slim-setup ac-slim-current-tag ac-slim-current-attr) (ac-jade-setup ac-jade-current-tag ac-jade-current-attr) (ac-haml-setup ac-haml-current-tag ac-haml-current-attr)))"#
    ]];

    assert_ac_html_parity(elisp_form, expect);
}

#[test]
fn ac_html_generated_sources_share_constant_callback_cells_and_tails() {
    let elisp_form = r##"(progn
               (require 'ac-slim)
               (require 'ac-jade)
               (require 'ac-haml)
               (list
                (eq
                 (car ac-source-html-tag)
                 (car ac-source-slim-tag))
                (eq
                 (cddr ac-source-html-tag)
                 (cddr ac-source-slim-tag))
                (eq
                 (car ac-source-html-attr)
                 (car ac-source-jade-attr))
                (eq
                 (cddr ac-source-html-attr)
                 (cddr ac-source-haml-attr))
                (eq
                 (car ac-source-slim-id)
                 (car ac-source-haml-id))
                (eq
                 (cddr ac-source-slim-class)
                 (cddr ac-source-haml-class))))"##;
    let expect = expect!["OK (t t t t t t)"];

    assert_ac_html_parity(elisp_form, expect);
}
