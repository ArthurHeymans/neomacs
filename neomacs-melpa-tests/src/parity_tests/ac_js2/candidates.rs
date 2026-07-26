use expect_test::expect;

use super::assert_ac_js2_parity;

#[test]
fn ac_js2_candidates_global_branch_reparses_and_combines_remote_local_and_extra_names() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "fixture")
               (let* ((parent-node
                       (make-js2-ast-root
                        :pos 0
                        :len 7
                        :buffer
                        (current-buffer)))
                      (point-node
                       (make-js2-name-node
                        :pos 0
                        :len 7
                        :name "fixture"))
                      (ac-js2-force-reparse
                      t)
                     (ac-js2-skewer-candidates
                      nil)
                     events)
                 (setf
                  (js2-node-parent
                   point-node)
                  parent-node)
                 (cl-letf
                     (((symbol-function
                        'js2-reparse)
                       (lambda ()
                         (push '(reparse) events)))
                      ((symbol-function
                        'js2-node-at-point)
                       (lambda (&rest arguments)
                         (push
                         (cons
                           'node-at-point
                           arguments)
                          events)
                         point-node))
                      ((symbol-function
                        'ac-js2-skewer-eval-wrapper)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'skewer arguments)
                          events)
                         (setq
                          ac-js2-skewer-candidates
                          '((remote
                             . "remote-doc")))))
                      ((symbol-function
                        'ac-js2-get-names-in-scope)
                       (lambda ()
                         (push '(scope) events)
                         '(("local"
                            . "local-doc")
                           ("second"
                            . nil))))
                      ((symbol-function
                        'ac-js2-add-extra-completions)
                       (lambda (completions)
                         (push
                          (list
                           'extras completions)
                          events)
                         (append
                          completions
                          '("keyword")))))
                   (list
                    (ac-js2-candidates)
                    ac-js2-candidates
                    ac-js2-skewer-candidates
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (("remote" "local" "second" "keyword") nil ((remote . "remote-doc")) ((reparse) (node-at-point 7) (skewer "" ((method . 1))) (scope) (extras ("local" "second"))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_candidates_dot_branch_extracts_object_and_appends_local_then_remote_properties() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "object.")
               (let* ((parent-node
                       (make-js2-ast-root
                        :pos 0
                        :len 7
                        :buffer
                        (current-buffer)))
                      (point-node
                       (make-js2-name-node
                        :pos 0
                        :len 6
                        :name "object"))
                      (first-left
                       (make-js2-name-node
                        :pos 0
                        :len 7
                        :name "\"alpha\""))
                      (second-left
                       (make-js2-name-node
                        :pos 0
                        :len 4
                        :name "beta"))
                      (first-property
                       (make-js2-object-prop-node
                        :pos 0
                        :len 7
                        :left first-left))
                      (second-property
                       (make-js2-object-prop-node
                        :pos 0
                        :len 4
                        :left second-left))
                      (object-node
                       (make-js2-object-node
                        :pos 0
                        :len 7
                        :elems
                        (list
                         first-property
                         second-property)))
                      (ac-js2-force-reparse
                      nil)
                     (ac-js2-skewer-candidates
                      nil)
                     events)
                 (setf
                  (js2-node-parent
                   point-node)
                  parent-node)
                 (cl-letf
                     (((symbol-function
                        'js2-node-at-point)
                       (lambda (&rest arguments)
                         (push
                         (cons
                           'node-at-point
                           arguments)
                          events)
                         point-node))
                      ((symbol-function
                        'ac-js2-get-object-properties)
                       (lambda (name)
                         (push
                          (list
                           'properties name)
                          events)
                         (setq
                          ac-js2-skewer-candidates
                          '((remote
                             . "remote-doc")))))
                      ((symbol-function
                        'ac-js2-initialized-node)
                       (lambda (name)
                         (push
                         (list
                           'initialized name)
                          events)
                         object-node))
                      ((symbol-function
                        'js2-node-string)
                       (lambda (node)
                         (if
                             (eq
                              node
                              first-left)
                             "\"alpha\""
                           "beta")))
                      ((symbol-function
                        'ac-js2-format-node)
                       (lambda (name node)
                         (let ((label
                                (if
                                    (eq
                                     node
                                     first-property)
                                    'first-property
                                  'second-property)))
                           (push
                            (list
                             'format name label)
                            events)
                           (cons
                            (replace-regexp-in-string
                             "\"" ""
                             name)
                            label)))))
                   (list
                    (ac-js2-candidates)
                    ac-js2-candidates
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (("alpha" "beta" "remote") (("alpha" . first-property) ("beta" . second-property)) ((node-at-point 7) (properties "object") (initialized "object") (format "\"alpha\"" first-property) (format "beta" second-property)))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_candidates_property_node_branch_uses_left_node_text_and_remote_only() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "object.member")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (goto-char
                (point-max))
               (let* ((name-node
                       (js2-node-at-point
                        (1- (point))))
                      (ac-js2-force-reparse
                      nil)
                     (ac-js2-skewer-candidates
                      nil)
                     events)
                 (cl-letf
                     (((symbol-function
                        'js2-node-at-point)
                       (lambda (&rest arguments)
                         (push
                         (cons
                           'node-at-point
                           arguments)
                          events)
                         name-node))
                      ((symbol-function
                        'ac-js2-get-object-properties)
                       (lambda (name)
                         (push
                          (list
                           'properties name)
                          events)
                         (setq
                          ac-js2-skewer-candidates
                          '((first . "one")
                            (second . "two"))))))
                   (list
                    (ac-js2-candidates)
                    ac-js2-candidates
                    (nreverse events)))))"##;
    let expect =
        expect![[r#"OK (("first" "second") nil ((node-at-point 13) (properties "object")))"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_upstream_local_candidate_scenario_finds_declared_names_without_browser() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var temp = function(param1, param2) {\n  var localParam = 15;\n  return param1 + param2;\n};\nvar look;\ntem")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse)
                ac-js2-force-reparse
                nil
                ac-js2-add-keywords
                nil
                ac-js2-add-ecma-262-externs
                nil
                ac-js2-add-browser-externs
                nil
                skewer-clients
                nil)
               (goto-char
                (point-max))
               (let ((candidates
                      (ac-js2-candidates)))
                 (list
                  (sort
                   (copy-sequence
                    candidates)
                   #'string<)
                  (assoc
                   "temp"
                   ac-js2-candidates)
                  (assoc
                   "look"
                   ac-js2-candidates)
                  ac-js2-skewer-candidates)))"##;
    let expect = expect![[
        r#"OK (("look" "temp") ("temp" . "function(param1, param2)") ("look" . "") nil)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_real_object_literal_dot_candidates_include_local_properties_without_browser() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var temp = {alpha: 1, callable: function(value) { return value; }, empty: {}};\ntemp.")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse)
                ac-js2-force-reparse
                nil
                skewer-clients
                nil)
               (goto-char
                (point-max))
               (let ((candidates
                      (ac-js2-candidates)))
                 (list
                  candidates
                  ac-js2-candidates
                  ac-js2-skewer-candidates)))"##;
    let expect = expect![[
        r#"OK (("alpha" "callable" "empty") (("alpha" . "1") ("callable" . "function(value)") ("empty" . "{}")) nil)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_completion_at_point_surfaces_inserted_js2_error_property_extent() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var testComplete = function(param1, param2) {};\ntestComplet")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse)
                ac-js2-force-reparse
                nil
                ac-js2-add-keywords
                nil
                ac-js2-add-ecma-262-externs
                nil
                ac-js2-add-browser-externs
                nil
                skewer-clients
                nil)
               (setq-local
                completion-at-point-functions
                '(ac-js2-completion-function))
               (goto-char
                (point-max))
               (list
                (completion-at-point)
                (buffer-string)
                (substring-no-properties
                 (or
                  (thing-at-point
                   'word)
                  ""))))"##;
    let expect = expect![[
        r#"OK (t #("var testComplete = function(param1, param2) {};\ntestComplete" 0 3 (font-lock-face font-lock-keyword-face) 4 16 (font-lock-face font-lock-function-name-face) 19 27 (font-lock-face font-lock-keyword-face) 28 34 (font-lock-face js2-function-param) 36 42 (font-lock-face js2-function-param) 48 59 (cursor-sensor-functions (js2-echo-error) help-echo "missing ; after statement")) "testComplete")"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}
