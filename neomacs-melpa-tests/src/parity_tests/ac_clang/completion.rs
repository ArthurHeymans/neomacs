use expect_test::expect;

use super::assert_ac_clang_parity;

#[test]
fn ac_clang_build_completion_candidates_filters_literal_start_text_and_merges_adjacent_overloads() {
    let elisp_form = r##"(let* ((case-fold-search nil)
                    (data
                     '(:Results
                       [(:Name "alpha"
                         :Prototype "int alpha(int)")
                        (:Name "alpha"
                         :Prototype "float alpha(float)")
                        (:Name "beta"
                         :Prototype "void beta(void)")
                        (:Name "my-alpha-tail"
                         :Prototype "long my-alpha-tail")
                        (:Name "ALPHA"
                         :Prototype "upper")
                        (:Name "alpha"
                         :Prototype nil)
                        (:Name "a+b"
                         :Prototype "literal plus")
                        (:Name "aaab"
                         :Prototype "regexp-looking")])))
               (mapcar
                (lambda (candidate)
                  (list
                   (substring-no-properties candidate)
                   (get-text-property
                    0 :detail candidate)
                   (get-text-property
                    0 :indices candidate)
                   (text-properties-at
                    0 candidate)))
                (append
                 (ac-clang--build-completion-candidates
                  data "alpha")
                 (ac-clang--build-completion-candidates
                  data "a+b"))))"##;
    let expect = expect![[
        r#"OK (("alpha" "int alpha(int)\nfloat alpha(float)" #1=(0 1) (:detail "int alpha(int)\nfloat alpha(float)" :indices #1#)) ("my-alpha-tail" "long my-alpha-tail" #2=(3) (:detail "long my-alpha-tail" :indices #2#)) ("alpha" nil nil nil) ("a+b" "literal plus" #3=(6) (:detail "literal plus" :indices #3#)))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_receive_completion_updates_all_buffer_state_before_starting_auto_complete() {
    let elisp_form = r##"(let ((ac-clang--candidates nil)
                    (ac-clang--start-point nil)
                    (ac-clang--completion-command-result-data
                     nil)
                    events)
               (cl-letf
                   (((symbol-function
                      'ac-clang--build-completion-candidates)
                     (lambda (data start-word)
                       (push
                        (list
                         'build data start-word)
                        events)
                       '("one" "two")))
                    ((symbol-function
                      'ac-complete-clang-async)
                     (lambda ()
                       (push
                        (list
                         'complete
                         ac-clang--candidates
                         ac-clang--start-point
                         ac-clang--completion-command-result-data)
                        events)
                       'started)))
                 (let ((data
                        '(:Results [fixture]))
                       (arguments
                        '(:start-word "fi"
                          :start-point 7)))
                   (list
                    (ac-clang--receive-completion
                     data arguments)
                    ac-clang--candidates
                    ac-clang--start-point
                    ac-clang--completion-command-result-data
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (started #2=("one" "two") 7 #1=(:Results [fixture]) ((build #1# "fi") (complete #2# 7 #1#)))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_auto_trigger_start_point_recognizes_only_dot_arrow_and_double_colon_suffixes() {
    let elisp_form = r##"(mapcar
               (lambda (text)
                 (with-temp-buffer
                   (insert text)
                   (list
                    text
                    (point)
                    (ac-clang--get-autotrigger-start-point)
                    (ac-clang--get-autotrigger-start-point
                     (point-min)))))
               '("" "." "x." ">" "->" "x->"
                 ":" "::" "x::" "-" "x" " .x"))"##;
    let expect = expect![[
        r#"OK (("" 1 nil nil) ("." 2 2 nil) ("x." 3 3 nil) (">" 2 nil nil) ("->" 3 3 nil) ("x->" 4 4 nil) (":" 2 nil nil) ("::" 3 3 nil) ("x::" 4 4 nil) ("-" 2 nil nil) ("x" 2 nil nil) (" .x" 4 nil nil))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_manual_trigger_prefers_symbol_start_then_accepts_operator_or_space_boundaries() {
    let elisp_form = r##"(with-temp-buffer
               (insert "obj.member ")
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'ac-prefix-symbol)
                       (lambda ()
                         (pop events))))
                   (setq events
                         (list 5 nil 2 nil))
                   (list
                    (ac-clang--get-manualtrigger-start-point)
                    (ac-clang--get-manualtrigger-start-point)
                    (progn
                      (delete-char -1)
                      (ac-clang--get-manualtrigger-start-point))
                    (ac-clang--get-manualtrigger-start-point)))))"##;
    let expect = expect!["OK (5 12 nil nil)"];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_async_completion_sends_exact_word_and_start_point_and_ignores_nil() {
    let elisp_form = r##"(with-temp-buffer
               (insert "obj.μέλος")
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'clang-server-request-transaction)
                       (lambda
                         (sender receiver arguments)
                         (push
                          (list
                           sender receiver arguments)
                          events)
                         'requested)))
                   (list
                    (ac-clang--async-completion nil)
                    (ac-clang--async-completion 5)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil requested ((clang-server-send-completion-command ac-clang--receive-completion (:start-word "μέλος" :start-point 5))))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_auto_and_manual_commands_apply_their_distinct_insertion_and_enable_policies() {
    let elisp_form = r##"(let ((ac-clang-async-autocompletion-automatically-p
                    t)
                   events)
               (cl-letf
                   (((symbol-function
                      'self-insert-command)
                     (lambda (count)
                       (push
                        (list 'insert count)
                        events)
                       'inserted))
                    ((symbol-function
                      'ac-clang--get-autotrigger-start-point)
                     (lambda ()
                       (push 'auto-point events)
                       4))
                    ((symbol-function
                      'ac-clang--get-manualtrigger-start-point)
                     (lambda ()
                       (push 'manual-point events)
                       6))
                    ((symbol-function
                      'ac-clang--async-completion)
                     (lambda (point)
                       (push
                        (list 'complete point)
                        events)
                       'requested)))
                 (list
                  (ac-clang-async-autocomplete-autotrigger)
                  (let ((ac-clang-async-autocompletion-automatically-p
                         nil))
                    (ac-clang-async-autocomplete-autotrigger))
                  (ac-clang-async-autocomplete-manualtrigger)
                  (nreverse events))))"##;
    let expect = expect![
        "OK (requested nil requested ((insert 1) auto-point (complete 4) (insert 1) manual-point (complete 6)))"
    ];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_document_cleanup_balance_and_argument_splitting_cover_nested_and_unbalanced_forms() {
    let elisp_form = r##"(list
               (mapcar
                #'ac-clang--clean-document
                '(nil
                  ""
                  "[#int#] fn(<#arg#>, {#default#})"
                  "left[#tail#]"))
               (mapcar
                (lambda (entry)
                  (list
                   (ac-clang--same-count-in-string
                    ?< ?> entry)
                   (ac-clang--same-count-in-string
                    ?\( ?\) entry)))
                '("" "<>" "<" "(a)" "(a"))
               (mapcar
                #'ac-clang--split-args
                '("a, b, c"
                  "std::pair<int, float>, tail"
                  "void (*cb)(int, char), end"
                  "outer<std::pair<int, float>, X>, (a, b), z"
                  "broken<a, b")))"##;
    let expect = expect![[
        r#"OK ((nil "" "int  fn(arg, default)" "lefttail ") ((t t) (t t) (nil t) (t t) (t nil)) (("a" "b" "c") ("std::pair<int, float>" "tail") ("void (*cb)(int, char)" "end") ("outer<std::pair<int, float>, X>" "(a, b)" "z") nil))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_action_builds_c_cpp_default_variadic_objective_c_and_function_pointer_templates() {
    let elisp_form = r##"(let* ((detail
                      (concat
                       "[#int#]foo(<#int x#>, {#int y = 1#}, ...)\n"
                       "[#void#]foo(<#double z#>)\n"
                       "[#id#]foo:<#id value#>\n"
                       "[#void (*)(int, char)#]handler"))
                    (candidate
                     (propertize
                      "foo"
                      :detail detail
                      :indices '(2 3 4 5)))
                    (ac-clang--template-candidates nil)
                    (ac-clang--template-start-point nil)
                    events)
               (cl-progv
                   '(ac-last-completion)
                   (list
                    (cons 'source candidate))
                 (cl-letf
                     (((symbol-function
                        'ac-complete-clang-template)
                       (lambda ()
                         (push
                          (list
                           'complete
                           ac-clang--template-start-point)
                          events)
                         'started))
                      ((symbol-function
                        'message)
                       (lambda (format-string &rest args)
                         (push
                          (apply
                           #'format format-string args)
                          events)
                         nil)))
                   (with-temp-buffer
                     (insert "foo")
                     (list
                      (ac-clang--action)
                      (mapcar
                       (lambda (item)
                         (list
                          (substring-no-properties
                           item)
                          (get-text-property
                           0 :detail item)
                          (get-text-property
                           0 :args item)
                          (get-text-property
                           0 :indices item)))
                       ac-clang--template-candidates)
                      ac-clang--template-start-point
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (nil (("(int x, int y = 1, ...)" "int" "(<#int x#>, {#int y = 1#}, ...)" (2)) ("(int x, , ...)" "int" "(<#int x#>, , ...)" (2)) ("(int x, )" "int" "(<#int x#>, )" (2)) ("(double z)" "void" "(<#double z#>)" (3)) (":id value" "id" ":<#id value#>" (4)) ("(int, char)" "void " "" (5))) 4 ((complete 4)))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_document_uses_the_first_candidate_index_and_reports_brief_comment() {
    let elisp_form = r##"(let* ((candidate
                      (propertize
                       "name"
                       :detail
                       "[#int#]name(<#x#>)"
                       :indices '(1 0)))
                    (ac-clang--completion-command-result-data
                     '(:Results
                       [(:BriefComment "zero")
                        (:BriefComment "one")]))
                    events)
               (cl-letf
                   (((symbol-function
                      'message)
                     (lambda (format-string &rest args)
                       (push
                        (apply
                         #'format format-string args)
                        events)
                       nil)))
                 (list
                  (ac-clang--document candidate)
                  (ac-clang--document 'not-a-string)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK ("int name(x)" nil ("BriefComment : one"))"#]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_template_action_expands_function_function_pointer_and_empty_argument_variants() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'yas-expand-snippet)
                     (lambda (snippet start end)
                       (push
                        (list snippet start end)
                        events)
                       'expanded)))
                 (list
                  (with-temp-buffer
                    (insert "foo")
                    (let ((ac-clang--template-start-point
                           1)
                          (candidate
                           (propertize
                            "foo"
                            :args
                            "(<#int x#>, {#const char *s = \"x\"#}, ...)")))
                      (cl-progv
                          '(ac-last-completion)
                          (list
                           (cons 'source candidate))
                        (ac-clang--template-action))))
                  (with-temp-buffer
                    (insert "fn")
                    (let ((ac-clang--template-start-point
                           1)
                          (candidate
                           (propertize
                            "(int, std::pair<A, B>)"
                            :args "")))
                      (cl-progv
                          '(ac-last-completion)
                          (list
                           (cons 'source candidate))
                        (ac-clang--template-action))))
                  (with-temp-buffer
                    (insert "empty")
                    (let ((ac-clang--template-start-point
                           1)
                          (candidate
                           (propertize
                            "empty"
                            :args "()")))
                      (cl-progv
                          '(ac-last-completion)
                          (list
                           (cons 'source candidate))
                        (ac-clang--template-action))))
                  (let ((ac-clang--template-start-point
                         nil))
                    (ac-clang--template-action))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (expanded expanded nil nil (("(${int x}, ${const char *s = \\\"x\\\"}}, ${...)" 1 4) (#("(${int}, ${std::pair<A, B>})" 3 6 (:args "") 11 22 (:args "") 24 26 (:args "")) 1 3)))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_accessors_and_syntax_context_distinguish_code_strings_and_comments() {
    let elisp_form = r##"(with-temp-buffer
               (c++-mode)
               (insert
                "value;\n"
                "\"string\";\n"
                "// comment\n"
                "/* block */\n")
               (syntax-propertize
                (point-max))
               (let ((ac-clang--candidates
                      '("one" "two"))
                     (ac-clang--start-point 7)
                     (ac-clang--template-candidates
                      '("template"))
                     (ac-clang--template-start-point
                      9))
                 (list
                  (ac-clang--candidates)
                  (ac-clang--prefix)
                  (ac-clang--template-candidates)
                  (ac-clang--template-prefix)
                  (mapcar
                   (lambda (needle)
                     (goto-char
                      (point-min))
                     (search-forward needle)
                     (ac-clang--in-string/comment))
                   '("value" "string" "comment"
                     "block")))))"##;
    let expect = expect![[r#"OK (("one" "two") 7 ("template") 9 (nil 8 18 29))"#]];

    assert_ac_clang_parity(elisp_form, expect);
}

#[test]
fn ac_clang_diagnostics_forwards_output_and_rebuilds_flymake_status_in_official_order() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'flymake-log)
                     (lambda (&rest arguments)
                       (push
                        (cons 'log arguments)
                        events)))
                    ((symbol-function
                      'flymake-parse-output-and-residual)
                     (lambda (diagnostics)
                       (push
                        (list
                         'parse-output diagnostics)
                        events)))
                    ((symbol-function
                      'flymake-parse-residual)
                     (lambda ()
                       (push 'parse-residual events)))
                    ((symbol-function
                      'flymake-fix-line-numbers)
                     (lambda
                       (info minimum lines)
                       (push
                        (list
                         'fix info minimum lines)
                        events)
                       'fixed-info))
                    ((symbol-function
                      'flymake-delete-own-overlays)
                     (lambda ()
                       (push 'delete-overlays events)))
                    ((symbol-function
                      'flymake-highlight-err-lines)
                     (lambda (info)
                       (push
                        (list 'highlight info)
                        events)))
                    ((symbol-function
                      'flymake-get-err-count)
                     (lambda (_info kind)
                       (if (string= kind "e")
                           2
                         1)))
                    ((symbol-function
                      'flymake-report-status)
                     (lambda (&rest status)
                       (push
                        (cons 'status status)
                        events))))
                 (list
                  (with-temp-buffer
                    (insert "one\ntwo\n")
                    (let ((buffer-file-name
                           "fixture.cpp"))
                      (cl-progv
                          '(flymake-err-info
                            flymake-new-err-info)
                          '(old new)
                        (list
                         (ac-clang--receive-diagnostics
                          '(:Results
                            (:Diagnostics
                             "diagnostic text"))
                          '(:start-time 0.0))
                         (symbol-value
                          'flymake-err-info)
                         (symbol-value
                          'flymake-new-err-info)
                         (nreverse events)))))
                  (let ((before events))
                    (with-temp-buffer
                      (list
                       (ac-clang--receive-diagnostics
                        '(:Results
                          (:Diagnostics "ignored"))
                        '(:start-time 0.0))
                       (eq before events)))))))"##;
    let expect = expect![[
        r#"OK ((#1=((status "2/1" "")) fixed-info nil ((parse-output "diagnostic text") parse-residual (fix new 1 2) delete-overlays (highlight fixed-info) . #1#)) (nil t))"#
    ]];

    assert_ac_clang_parity(elisp_form, expect);
}
