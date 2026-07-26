use expect_test::expect;

use super::assert_ac_inf_ruby_parity;

#[test]
fn ac_inf_ruby_exact_pin_dependencies_features_and_source_descriptor_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-inf-ruby
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-inf-ruby
                   inf-ruby
                   auto-complete))
                ac-source-inf-ruby
                (mapcar
                 (lambda (property)
                   (cdr
                    (assq
                     property
                     ac-source-inf-ruby)))
                 '(available
                   candidates
                   symbol
                   prefix))))"##;
    let expect = expect![[
        r#"OK (ac-inf-ruby "20131115.1150" ((inf-ruby (2 3 2)) (auto-complete (1 4))) (t t t) ((available . ac-inf-ruby-available) (candidates . ac-inf-ruby-candidates) (symbol . "r") (prefix . ac-inf-ruby-prefix)) (ac-inf-ruby-available ac-inf-ruby-candidates "r" ac-inf-ruby-prefix))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_function_surface_arities_interactivity_docs_and_definitions_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (functionp function)
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)
                  (let ((definition
                         (symbol-function
                          function)))
                    (cond
                     ((symbolp definition)
                      definition)
                     ((byte-code-function-p
                       definition)
                      'byte-code)
                     (t 'interpreted)))))
               '(ac-inf-ruby-candidates
                 ac-inf-ruby-prefix
                 ac-inf-ruby-available
                 ac-inf-ruby-enable))"##;
    let expect = expect![[
        r#"OK ((ac-inf-ruby-candidates t nil nil "Return completion candidates for `ac-prefix'." interpreted) (ac-inf-ruby-prefix t nil nil "Return starting position of completion prefix." interpreted) (ac-inf-ruby-available t nil nil "Return t if inf-ruby completions are available, otherwise nil." interpreted) (ac-inf-ruby-enable t nil nil "Add `ac-source-inf-ruby' to `ac-sources' for this buffer." interpreted))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_source_callbacks_resolve_the_live_function_cells() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-inf-ruby-available)
                     (lambda ()
                       (push 'available events)
                       'available-result))
                    ((symbol-function
                      'ac-inf-ruby-candidates)
                     (lambda ()
                       (push 'candidates events)
                       '(one two)))
                    ((symbol-function
                      'ac-inf-ruby-prefix)
                     (lambda ()
                       (push 'prefix events)
                       17)))
                 (list
                  (funcall
                   (cdr
                    (assq
                     'available
                     ac-source-inf-ruby)))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-inf-ruby)))
                  (cdr
                   (assq
                    'symbol
                    ac-source-inf-ruby))
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-inf-ruby)))
                  (nreverse events))))"##;
    let expect =
        expect![[r#"OK (available-result (one two) "r" 17 (available candidates prefix))"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}
