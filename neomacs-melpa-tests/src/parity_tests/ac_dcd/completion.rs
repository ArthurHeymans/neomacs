use expect_test::expect;

use super::assert_ac_dcd_parity;

#[test]
fn ac_dcd_cursor_bytes_complete_arguments_and_syntax_context_match() {
    let elisp_form = r##"(list
               (with-temp-buffer
                 (insert "aλz")
                 (mapcar
                  (lambda (position)
                    (goto-char position)
                    (list
                     position
                     (ac-dcd-cursor-position)))
                  (number-sequence
                   (point-min)
                   (point-max))))
               (let
                   ((ac-dcd-server-port
                     12345))
                 (mapcar
                  #'ac-dcd-build-complete-args
                  '(0 1 42)))
               (with-temp-buffer
                 (emacs-lisp-mode)
                 (insert
                  "(message \"inside\") ; comment\nsymbol")
                 (list
                  (progn
                    (search-backward "inside")
                    (ac-in-string/comment))
                  (progn
                    (search-forward "comment")
                    (ac-in-string/comment))
                  (progn
                    (goto-char (point-max))
                    (ac-in-string/comment)))))"##;
    let expect = expect![[
        r#"OK (((1 1) (2 2) (3 4) (4 5)) (("-c" "0" "-p" "12345") ("-c" "1" "-p" "12345") ("-c" "42" "-p" "12345")) (10 20 nil))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_cursor_adjustment_moves_to_query_boundary_and_preserves_text() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert fixture)
                   (let ((end (point)))
                     (list
                      fixture
                      (ac-dcd-adjust-cursor-on-completion
                       end)
                      (point)
                      (buffer-string)
                      (buffer-substring
                       (point)
                       end)))))
               '("object.member"
                 "alpha beta"
                 "line\nname"
                 "scope.item123"))"##;
    let expect = expect![[
        r#"OK (("object.member" nil 8 "object.member" "member") ("alpha beta" nil 7 "alpha beta" "beta") ("line\nname" nil 6 "line\nname" "name") ("scope.item123" nil 7 "scope.item123" "item123"))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_prefix_uses_auto_complete_symbol_or_exact_point_fallback() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abc.")
               (list
                (cl-letf
                    (((symbol-function
                       'ac-prefix-symbol)
                      (lambda ()
                        2)))
                  (ac-dcd-prefix))
                (cl-letf
                    (((symbol-function
                       'ac-prefix-symbol)
                      (lambda ()
                        nil)))
                  (ac-dcd-prefix))
                (progn
                  (goto-char (point-min))
                  (cl-letf
                      (((symbol-function
                         'ac-prefix-symbol)
                        (lambda ()
                          nil)))
                    (ac-dcd-prefix)))))"##;
    let expect = expect!["OK (2 5 1)"];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_get_candidates_widens_moves_to_prefix_and_parses_process_output() {
    let elisp_form = r##"(with-temp-buffer
               (insert "hidden\nobject.mem")
               (narrow-to-region 8 (point-max))
               (goto-char (point-max))
               (let ((ac-prefix "mem")
                     (ac-dcd-server-port 4242)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ac-dcd-call-process)
                       (lambda (args)
                         (push
                          (list
                           args
                           (point)
                           (buffer-string)
                           (buffer-narrowed-p))
                          calls)
                         (with-current-buffer
                             (get-buffer-create
                              ac-dcd-output-buffer-name)
                           (erase-buffer)
                           (insert
                            "member\tm\nmethod\tf\n")))))
                   (unwind-protect
                       (let
                           ((candidates
                             (ac-dcd-get-candidates)))
                         (list
                          (mapcar
                           (lambda (item)
                             (cons
                              (substring-no-properties
                               item)
                              (get-text-property
                               0 'ac-dcd-help item)))
                           candidates)
                          (nreverse calls)
                          (point)
                          (buffer-string)
                          (buffer-narrowed-p)))
                     (when
                         (get-buffer
                          ac-dcd-output-buffer-name)
                       (kill-buffer
                        ac-dcd-output-buffer-name))))))"##;
    let expect = expect![[
        r#"OK ((("method" . "f") ("member" . "m")) ((("-c" "15" "-p" "4242") 15 "hidden\nobject.mem" nil)) 18 "object.mem" t)"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_get_candidates_skips_process_inside_strings_and_comments() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (emacs-lisp-mode)
                   (insert fixture)
                   (goto-char
                    (if
                        (string-prefix-p
                         "\"" fixture)
                        3
                      (point-max)))
                   (let ((ac-prefix "x")
                         calls)
                     (cl-letf
                         (((symbol-function
                            'ac-dcd-call-process)
                           (lambda (args)
                             (push args calls))))
                       (list
                        (ac-dcd-get-candidates)
                        calls)))))
               '("\"text\"" "; comment"))"##;
    let expect = expect!["OK ((nil nil) (nil nil))"];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_action_dispatches_only_function_and_struct_candidates_with_yasnippet() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-complete-dcd-calltips)
                     (lambda ()
                       (push 'function events)))
                    ((symbol-function
                      'ac-complete-dcd-calltips-for-struct-constructor)
                     (lambda ()
                       (push 'struct events))))
                 (dolist
                     (kind
                      '("f" "s" "v" "T"))
                   (let
                       ((ac-last-completion
                         (cons
                          1
                          (propertize
                           "item"
                           'ac-dcd-help kind))))
                     (push
                      (list
                       kind
                       (ac-dcd-action))
                      events)))
                 (nreverse events)))"##;
    let expect = expect![[r#"OK (("f" nil) ("s" nil) ("v" nil) ("T" nil))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_calltip_process_probe_restores_source_and_uses_inserted_probe_position() {
    let elisp_form = r##"(with-temp-buffer
               (insert "foo")
               (let ((ac-dcd-server-port 9167)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ac-dcd-call-process)
                       (lambda (args)
                         (push
                          (list
                           args
                           (point)
                           (ac-dcd-cursor-position)
                           (buffer-string))
                          calls))))
                   (list
                    (ac-dcd-call-process-for-calltips)
                    (buffer-string)
                    (point)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK (nil "foo" 4 ((("-c" "5" "-p" "9167") 5 5 "foo( ;")))"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_calltip_action_expands_normal_and_template_argument_snippets() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert fixture)
                   (let ((ac-last-completion
                          (cons 1 fixture))
                         expansions)
                     (cl-letf
                         (((symbol-function
                            'yas-expand-snippet)
                           (lambda
                               (snippet &rest rest)
                             (push
                              (cons snippet rest)
                              expansions)
                             'expanded)))
                       (list
                        (ac-dcd-calltip-action)
                        (buffer-string)
                        (point)
                        expansions)))))
               '("foo(int x, string y)"
                 "templ(T)(T value)"))"##;
    let expect = expect![[
        r#"OK ((expanded "foo" 4 (("(${int x}, ${string y})"))) (expanded "templ" 6 (("(${T})(${T value})"))))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_struct_constructor_candidates_replace_this_before_parsing() {
    let elisp_form = r##"(let ((ac-last-completion
                    '(1 . "Widget"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-dcd-call-process-for-calltips)
                     (lambda ()
                       (push 'called calls)
                       (with-current-buffer
                           (get-buffer-create
                            ac-dcd-output-buffer-name)
                         (erase-buffer)
                         (insert
                          "this(int x)\n"
                          "this(string value)\n")))))
                 (unwind-protect
                     (list
                      (ac-dcd-calltip-candidate-for-struct-constructor)
                      (with-current-buffer
                          ac-dcd-output-buffer-name
                        (buffer-string))
                      calls)
                   (when
                       (get-buffer
                        ac-dcd-output-buffer-name)
                     (kill-buffer
                      ac-dcd-output-buffer-name)))))"##;
    let expect = expect![[
        r#"OK (("Widget(string value)" "Widget(int x)") "Widget(int x)\nWidget(string value)\n" (called))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_replace_this_rewrites_every_substring_and_leaves_point_at_end() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "this(int x)\n"
                "thisValue\n"
                "other.this()\n")
               (goto-char 7)
               (list
                (ac-dcd-replace-this-to-struct-name
                 "Widget")
                (buffer-string)
                (point)))"##;
    let expect = expect![[r#"OK (nil "Widget(int x)\nWidgetValue\nother.Widget()\n" 39)"#]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_calltip_candidate_and_completion_wrappers_preserve_sources_and_prefixes() {
    let elisp_form = r##"(let ((ac-last-completion
                    '(17 . "fixture"))
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-dcd-call-process-for-calltips)
                     (lambda ()
                       (push 'probe events)
                       (with-current-buffer
                           (get-buffer-create
                            ac-dcd-output-buffer-name)
                         (erase-buffer)
                         (insert
                          "int fixture(int x)\n"))))
                    ((symbol-function
                      'auto-complete)
                     (lambda (sources)
                       (push
                        (list 'complete sources)
                        events)
                       'completed)))
                 (unwind-protect
                     (list
                      (ac-dcd-get-calltip-candidates)
                      (ac-dcd-calltip-prefix)
                      (ac-complete-dcd-calltips)
                      (ac-complete-dcd-calltips-for-struct-constructor)
                      (nreverse events))
                   (when
                       (get-buffer
                        ac-dcd-output-buffer-name)
                     (kill-buffer
                      ac-dcd-output-buffer-name)))))"##;
    let expect = expect![[
        r#"OK (("fixture(int x)") 17 completed completed (probe (complete (dcd-calltips)) (complete (dcd-calltips-for-struct-constructor))))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}
