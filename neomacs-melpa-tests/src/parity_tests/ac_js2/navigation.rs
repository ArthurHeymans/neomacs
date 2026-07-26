use expect_test::expect;

use super::assert_ac_js2_parity;

#[test]
fn ac_js2_property_name_helpers_follow_real_nested_js2_property_nodes() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "foo.bar.baz;")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (let* ((foo
                       (js2-node-at-point
                        2))
                      (bar
                       (js2-node-at-point
                        6))
                      (baz
                       (js2-node-at-point
                        10))
                      (inner
                       (js2-node-parent
                        bar))
                      (outer
                       (js2-node-parent
                        baz)))
                 (list
                  (mapcar
                   #'js2-name-node-name
                   (list
                    foo bar baz))
                  (ac-js2-build-prop-name-list
                   inner)
                  (ac-js2-build-prop-name-list
                   outer)
                  (ac-js2-prop-names-left
                   foo)
                  (ac-js2-prop-names-left
                   bar)
                  (ac-js2-prop-names-left
                   baz))))"##;
    let expect = expect![[
        r#"OK (("foo" "bar" "baz") ("baz" "bar" "foo") ("baz" nil) "foo" ("bar" "foo") ("baz" "bar" "foo"))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_find_property_locates_assignment_and_object_literal_definitions() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "foo.bar = 3;\nvar holder = {baz: 4};\nfoo.bar;\n")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (mapcar
                (lambda (names)
                  (let ((node
                         (ac-js2-find-property
                          names)))
                    (list
                     names
                     (and node
                          (js2-node-string
                           node))
                     (and node
                          (js2-node-abs-pos
                           node))
                     (and node
                          (cond
                           ((js2-prop-get-node-p
                             node)
                            'property)
                           ((js2-name-node-p
                             node)
                            'name)
                           (t 'other))))))
                '(("bar" "foo")
                  ("baz")
                  ("missing"))))"##;
    let expect = expect![[
        r#"OK ((("bar" "foo") "foo.bar" 1 property) (("baz") "baz" 28 name) (("missing") nil nil nil))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_get_function_name_handles_declarations_assigned_functions_and_anonymous_values() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "function declared(arg) { return arg; }\nvar held = function(value) { return value; };\n(function () { return 1; });\n")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (let (functions)
                 (js2-visit-ast-root
                  js2-mode-ast
                  (lambda (node end-p)
                    (unless end-p
                      (when
                          (js2-function-node-p
                           node)
                        (push node functions))
                      t)))
                 (mapcar
                  (lambda (node)
                    (list
                     (js2-node-string
                      node)
                     (ac-js2-get-function-name
                      node)))
                  (nreverse functions))))"##;
    let expect = expect![[
        r#"OK (("function declared(arg) { return arg; }" "declared") ("function(value) { return value; }" "held") ("function () { return 1; }" nil))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_initialized_node_resolves_property_list_assignment_initializers() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var object = {};\nobject.property = function(value) { return value; };\n")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (goto-char
                (point-max))
               (let ((node
                      (ac-js2-initialized-node
                       '("property"
                         "object"))))
                 (list
                  (js2-function-node-p
                   node)
                  (js2-node-string
                   node)
                  (ac-js2-format-function
                   node))))"##;
    let expect = expect![[r#"OK (t "function(value) { return value; }" "function(value)")"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_get_function_node_searches_real_scope_and_returns_first_named_match() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "function first() {}\nfunction wanted(arg) { return arg; }\nfunction wanted(second) { return second; }\n")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (let ((node
                      (ac-js2-get-function-node
                       "wanted"
                       js2-mode-ast)))
                 (list
                  (and node
                       (js2-node-string
                        node))
                  (ac-js2-get-function-node
                   "missing"
                   js2-mode-ast))))"##;
    let expect = expect![[r#"OK ("function wanted(arg) { return arg; }" nil)"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_jump_to_definition_navigates_variables_functions_and_object_properties() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var variable = 1;\nfunction callable(arg) { return arg; }\nvar object = {property: 3};\nvariable;\ncallable();\nobject.property;\n")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (let ((find-tag-marker-ring
                      (make-ring 8))
                     results)
                 (mapc
                  (lambda (name)
                    (goto-char
                     (point-max))
                    (search-backward
                     name)
                    (let ((origin
                           (point))
                          (return
                           (ac-js2-jump-to-definition)))
                      (push
                       (list
                        name
                        origin
                        return
                        (point)
                        (buffer-substring-no-properties
                         (point)
                         (min
                          (point-max)
                          (+
                           (point)
                           (length name)))))
                       results)))
                  '("variable"
                    "callable"
                    "property"))
                 (list
                  (nreverse results)
                  (ring-length
                   find-tag-marker-ring)
                  (mapcar
                   #'marker-position
                   (ring-elements
                    find-tag-marker-ring)))))"##;
    let expect = expect![[
        r#"OK ((("variable" 86 5 5 "variable") ("callable" 96 19 19 "function") ("property" 115 72 72 "property")) 3 (115 96 86))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_jump_to_definition_rolls_back_marker_before_signaling_for_missing_name() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "missing;")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (goto-char
                2)
               (let ((find-tag-marker-ring
                      (make-ring 4))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'pop-tag-mark)
                       (lambda ()
                         (push '(pop) calls)
                         (ring-remove
                          find-tag-marker-ring
                          0)
                         'popped)))
                   (list
                    (condition-case error-data
                        (list
                         :ok
                         (ac-js2-jump-to-definition))
                      (error
                       (cons
                        :error
                        error-data)))
                    (ring-length
                     find-tag-marker-ring)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK ((:error error "No jump location found") 0 ((pop)))"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_jump_to_definition_rejects_unsupported_nodes_after_recording_origin() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "42;")
               (js2-mode)
               (setq
                js2-mode-ast
                (js2-parse))
               (goto-char
                2)
               (let ((find-tag-marker-ring
                      (make-ring 4)))
                 (list
                  (condition-case error-data
                      (list
                       :ok
                       (ac-js2-jump-to-definition))
                    (error
                     (cons
                      :error
                      error-data)))
                  (ring-length
                   find-tag-marker-ring)
                  (mapcar
                   #'marker-position
                   (ring-elements
                    find-tag-marker-ring)))))"##;
    let expect = expect![[r#"OK ((:error error "Node is not a supported jump node") 1 (2))"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_build_prop_name_list_rejects_a_non_property_node() {
    let elisp_form = r##"(ac-js2-build-prop-name-list
               (make-js2-name-node
                :pos 0
                :len 4
                :name "name"))"##;
    let expect = expect![[r#"ERR (error "Node is not a property prop-node")"#]];

    super::assert_ac_js2_signal_parity(elisp_form, expect);
}

#[test]
fn ac_js2_prop_names_left_rejects_a_non_name_without_property_parent() {
    let elisp_form = r##"(with-temp-buffer
               (ac-js2-prop-names-left
                (make-js2-ast-root
                 :pos 0
                 :len 0
                 :buffer
                 (current-buffer))))"##;
    let expect =
        expect![[r#"ERR (error "Not a name node or doesn’t have a prop-get-node as parent")"#]];

    super::assert_ac_js2_signal_parity(elisp_form, expect);
}
