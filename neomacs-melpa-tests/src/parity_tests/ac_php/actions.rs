use expect_test::expect;

use super::assert_ac_php_parity;

#[test]
fn ac_php_action_expands_required_and_optional_arguments_in_order() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "call")
               (let ((candidate
                      (propertize
                       "method("
                       'ac-php-help
                       "$required,&$optional = null,$last=42"
                       'ac-php-tag-type
                       "f"))
                     calls)
                 (setq
                  ac-last-completion
                  (cons nil candidate))
                 (cl-letf
                     (((symbol-function
                        'ac-complete-php-template)
                       (lambda ()
                         (push
                          (list
                           ac-php-template-start-point
                           (mapcar
                            (lambda (item)
                              (list
                               item
                               (get-text-property
                                0
                                'raw-args
                                item)
                               (get-text-property
                                0
                                'ac-php-help
                                item)))
                            ac-php-template-candidates))
                          calls)
                         'completed))
                      ((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'message
                           arguments)
                          calls)
                         'messaged)))
                   (list
                    (ac-php-action)
                    ac-php-template-start-point
                    (mapcar
                     #'substring-no-properties
                     ac-php-template-candidates)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil 5 ("$required)" "$required,&$optional )" "$required,&$optional ,$last)") ((5 ((#("$required)" 0 10 (raw-args #("$required)" 0 10 (ac-php-help #1=#("method($required,&$optional = null,$last=42)" 0 7 (ac-php-tag-type #2="f" ac-php-help #3="$required,&$optional = null,$last=42")))) ac-php-help #1#)) #("$required)" 0 10 (ac-php-help #1#)) #("method($required,&$optional = null,$last=42)" 0 7 (ac-php-tag-type #2# ac-php-help #3#))) (#("$required,&$optional )" 0 22 (raw-args #("$required,&$optional )" 0 22 (ac-php-help #1#)) ac-php-help #1#)) #("$required,&$optional )" 0 22 (ac-php-help #1#)) #("method($required,&$optional = null,$last=42)" 0 7 (ac-php-tag-type #2# ac-php-help #3#))) (#("$required,&$optional ,$last)" 0 28 (raw-args #("$required,&$optional ,$last)" 0 28 (ac-php-help #1#)) ac-php-help #1#)) #("$required,&$optional ,$last)" 0 28 (ac-php-help #1#)) #("method($required,&$optional = null,$last=42)" 0 7 (ac-php-tag-type #2# ac-php-help #3#)))))))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_action_parses_overloads_deduplicates_candidates_and_skips_non_signatures() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "x")
               (let ((candidate
                      (propertize
                       "method"
                       'ac-php-help
                       "not a signature\nmethod($one)\nmethod($one)\nmethod($one,$two=2)"
                       'ac-php-tag-type
                       "f"))
                     calls)
                 (setq
                  ac-last-completion
                  (cons nil candidate))
                 (cl-letf
                     (((symbol-function
                        'ac-complete-php-template)
                       (lambda ()
                         (push
                          (mapcar
                           (lambda (item)
                             (list
                              item
                              (get-text-property
                               0
                               'raw-args
                               item)))
                           ac-php-template-candidates)
                          calls)
                         'completed))
                      ((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'message
                           arguments)
                          calls)
                         'messaged)))
                   (list
                    (ac-php-action)
                    (mapcar
                     #'substring-no-properties
                     ac-php-template-candidates)
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (nil ("$one)" "$one,$two)") (((#("$one)" 0 5 (raw-args #("$one)" 0 5 (ac-php-help #1="method($one,$two=2)")) ac-php-help #1#)) #("$one)" 0 5 (ac-php-help #1#))) (#("$one,$two)" 0 10 (raw-args #("$one,$two)" 0 10 (ac-php-help #1#)) ac-php-help #1#)) #("$one,$two)" 0 10 (ac-php-help #1#))))))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_action_handles_first_optional_malformed_and_qualified_signatures() {
    let elisp_form = r##"(mapcar
               (lambda (help)
                 (with-temp-buffer
                   (insert
                    "x")
                   (let ((candidate
                          (propertize
                           "fixture"
                           'ac-php-help
                           help
                           'ac-php-tag-type
                           "f"))
                         calls)
                     (setq
                      ac-last-completion
                      (cons nil candidate)
                      ac-php-template-candidates
                      nil
                      ac-php-template-start-point
                      nil)
                     (cl-letf
                         (((symbol-function
                            'ac-complete-php-template)
                           (lambda ()
                             (push
                              'complete
                              calls)
                             'completed))
                          ((symbol-function
                            'message)
                           (lambda (&rest arguments)
                             (push
                              (cons
                               'message
                               arguments)
                              calls)
                             'messaged)))
                       (let ((return
                              (ac-php-action)))
                         (list
                          help
                          return
                          ac-php-template-start-point
                          (mapcar
                           (lambda (item)
                             (list
                              (substring-no-properties
                               item)
                              (substring-no-properties
                               (get-text-property
                                0
                                'raw-args
                                item))))
                           ac-php-template-candidates)
                          (nreverse calls)))))))
               '("firstOptional($one=1,$two=2)"
                 "broken($one) trailing"
                 "\\Acme\\Tools\\build($x,$y=2)"))"##;
    let expect = expect![[
        r#"OK (("firstOptional($one=1,$two=2)" nil 2 ((")" ")") ("$one)" "$one)") ("$one,$two)" "$one,$two)")) (complete)) ("broken($one) trailing" messaged 2 (("$one)" "$one)")) (complete (message "broken($one) trailing"))) ("\\Acme\\Tools\\build($x,$y=2)" nil 2 (("$x)" "$x)") ("$x,$y)" "$x,$y)")) (complete)))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_action_without_signature_reports_cleaned_help_and_does_not_complete() {
    let elisp_form = r##"(let ((candidate
                    (propertize
                     "property"
                     'ac-php-help
                     "[#first line#]\n<#second line#>"
                     'ac-php-tag-type
                     "p"))
                   (stale-candidates
                    (list
                     "stale"))
                   calls)
               (setq
                ac-last-completion
                (cons nil candidate)
                ac-php-template-candidates
                stale-candidates
                ac-php-template-start-point
                77)
               (cl-letf
                   (((symbol-function
                      'ac-complete-php-template)
                     (lambda ()
                       (push
                        'unexpected-completion
                        calls)))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'message
                         arguments)
                        calls)
                       'messaged)))
                 (list
                 (ac-php-action)
                  ac-php-template-candidates
                  (eq
                   ac-php-template-candidates
                   stale-candidates)
                  ac-php-template-start-point
                  (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (messaged ("stale") t 77 ((message "first line  ;    second line")))"#]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_action_single_signature_completes_then_echoes_cleaned_help() {
    let elisp_form = r##"(let ((candidate
                    (propertize
                     "single"
                     'ac-php-help
                     "single($only)"
                     'ac-php-tag-type
                     "f"))
                   calls)
               (setq
                ac-last-completion
                (cons nil candidate))
               (cl-letf
                   (((symbol-function
                      'ac-complete-php-template)
                     (lambda ()
                       (push
                        '(complete)
                        calls)
                       'completed))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'message
                         arguments)
                        calls)
                       'messaged)))
                 (list
                  (ac-php-action)
                  (mapcar
                   #'substring-no-properties
                   ac-php-template-candidates)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (messaged ("$only)") ((complete) (message "single($only)")))"#]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_template_action_builds_yasnippet_fields_and_removes_reference_markers() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "method")
               (let ((ac-php-template-start-point
                      2)
                     (candidate
                      (propertize
                       "$first,&$second)"
                       'raw-args
                       "$first,&$second)"))
                     calls)
                 (setq
                  ac-last-completion
                  (cons nil candidate))
                 (cl-letf
                     (((symbol-function
                        'yas-expand-snippet)
                       (lambda
                           (snippet start end)
                         (push
                          (list
                           snippet start end)
                          calls)
                         'expanded)))
                   (list
                    (ac-php-template-action)
                    (point)
                    (buffer-string)
                    (nreverse calls)))))"##;
    let expect = expect![[r#"OK (expanded 7 "method" (("${$first},${$second})" 2 7)))"#]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_template_action_handles_zero_arguments_and_missing_yasnippet() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "f")
               (let ((ac-php-template-start-point
                      1)
                     (candidate
                      (propertize
                       ")"
                       'raw-args
                       ")"))
                     (had-yasnippet
                      (featurep
                       'yasnippet))
                     calls)
                 (setq
                  ac-last-completion
                  (cons nil candidate))
                 (cl-letf
                     (((symbol-function
                        'yas-expand-snippet)
                       (lambda
                           (snippet start end)
                         (push
                          (list
                           'snippet
                           snippet start end)
                          calls)
                         'expanded))
                      ((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'message
                           arguments)
                          calls)
                         'messaged)))
                   (let ((with-yasnippet
                          (ac-php-template-action)))
                     (setq
                      features
                      (delq
                       'yasnippet
                       features))
                     (unwind-protect
                         (list
                          with-yasnippet
                          (ac-php-template-action)
                          (nreverse calls))
                       (when had-yasnippet
                         (provide
                          'yasnippet)))))))"##;
    let expect = expect![[
        r#"OK (expanded messaged ((snippet ")" 1 2) (message "Dude! You are too out! Please install a yasnippet or a snippet script:)")))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_template_action_with_nil_start_is_a_noop() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "unchanged")
               (let ((ac-php-template-start-point
                      nil)
                     calls)
                 (setq
                  ac-last-completion
                  (cons
                   nil
                   (propertize
                    "$arg)"
                    'raw-args
                    "$arg)")))
                 (cl-letf
                     (((symbol-function
                        'yas-expand-snippet)
                       (lambda (&rest arguments)
                         (push arguments calls)))
                      ((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push arguments calls))))
                   (list
                    (ac-php-template-action)
                    (point)
                    (buffer-string)
                    calls))))"##;
    let expect = expect![[r#"OK (nil 10 "unchanged" nil)"#]];

    assert_ac_php_parity(elisp_form, expect);
}
