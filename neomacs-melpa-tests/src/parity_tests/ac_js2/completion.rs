use expect_test::expect;

use super::{assert_ac_js2_parity, assert_ac_js2_signal_parity};

#[test]
fn ac_js2_document_prefers_local_list_or_string_then_falls_back_to_skewer() {
    let elisp_form = r##"(let ((ac-js2-candidates
                    '(("local-list"
                       "first documentation"
                       "second documentation")
                      ("local-string"
                       . "direct documentation")
                      ("local-nil")))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-js2-skewer-document-candidates)
                     (lambda (name)
                       (push name calls)
                       (concat
                        "remote:" name))))
                 (list
                  (ac-js2-document
                   "local-list")
                  (ac-js2-document
                   "local-string")
                  (ac-js2-document
                   "local-nil")
                  (ac-js2-document
                   "missing")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("first documentation" "direct documentation" "remote:local-nil" "remote:missing" ("local-nil" "missing"))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_auto_complete_adapters_forward_candidates_document_and_prefix() {
    let elisp_form = r##"(let (events
                   (default-prefix
                    "default")
                   (dot-prefix
                    "dot"))
               (cl-letf
                   (((symbol-function
                      'ac-js2-candidates)
                     (lambda ()
                       (push '(candidates) events)
                       '(alpha beta)))
                    ((symbol-function
                      'ac-js2-document)
                     (lambda (name)
                       (push
                        (list 'document name)
                        events)
                       (concat
                        "doc:" name)))
                    ((symbol-function
                      'ac-prefix-default)
                     (lambda ()
                       (push '(default) events)
                       default-prefix))
                    ((symbol-function
                      'ac-prefix-c-dot)
                     (lambda ()
                       (push '(dot) events)
                       dot-prefix)))
                 (let ((first-prefix
                        (ac-js2-ac-prefix)))
                   (setq
                    default-prefix nil)
                   (list
                    (ac-js2-ac-candidates)
                    (ac-js2-ac-document
                     "fixture")
                    first-prefix
                    (ac-js2-ac-prefix)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ((alpha beta) "doc:fixture" "default" "dot" (#1=(default) (candidates) (document "fixture") #1# (dot)))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_save_string_valued_js2_mode_diverges_on_gnu_symbol_check() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var answer = 42;")
               (let ((major-mode
                      "js2-mode")
                     events)
                 (cl-letf
                     (((symbol-function
                        'ac-js2-skewer-eval-wrapper)
                       (lambda (string
                                &optional extras)
                         (push
                          (list string extras)
                          events)
                         'evaluated)))
                   (list
                    (ac-js2-save)
                    (nreverse events)))))"##;
    let expect = expect![[r#"ERR (wrong-type-argument symbolp "js2-mode")"#]];

    assert_ac_js2_signal_parity(elisp_form, expect);
}

#[test]
fn ac_js2_save_symbol_valued_js2_mode_evaluates_the_exact_buffer() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "var answer = 42;")
               (let ((major-mode
                      'js2-mode)
                     events)
                 (cl-letf
                     (((symbol-function
                        'ac-js2-skewer-eval-wrapper)
                       (lambda (string
                                &optional extras)
                         (push
                          (list string extras)
                          events)
                         'evaluated)))
                   (list
                    (ac-js2-save)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (t (("var answer = 42;" nil)))"#]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_save_string_valued_non_js2_mode_diverges_on_gnu_symbol_check() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-js2-skewer-eval-wrapper)
                     (lambda (string
                              &optional extras)
                       (push
                        (list string extras)
                        events)
                       'evaluated)))
                 (with-temp-buffer
                    (insert
                     "ignored")
                    (let ((major-mode
                           "other-mode"))
                      (list
                       (ac-js2-save)
                       events)))))"##;
    let expect = expect![[r#"ERR (wrong-type-argument symbolp "other-mode")"#]];

    assert_ac_js2_signal_parity(elisp_form, expect);
}

#[test]
fn ac_js2_completion_function_reports_word_or_dot_bounds_and_preserves_point() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-js2-candidates)
                     (lambda ()
                       (push
                        (list
                         (point)
                         (buffer-string))
                        events)
                       '("alpha"
                         "alphabet"))))
                 (list
                  (with-temp-buffer
                    (insert
                     "alpha bet")
                    (let ((before
                           (point))
                          (result
                           (ac-js2-completion-function)))
                      (list
                       result
                       before
                       (point))))
                  (with-temp-buffer
                    (insert
                     "object.")
                    (let ((before
                           (point))
                          (result
                           (ac-js2-completion-function)))
                      (list
                       result
                       before
                       (point))))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (((7 10 #1=("alpha" "alphabet")) 10 10) ((8 8 #1#) 8 8) ((10 "alpha bet") (8 "object.")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_company_without_company_reports_the_exact_message_for_every_command() {
    let elisp_form = r##"(let ((features
                    (delq
                     'company
                     (copy-sequence
                      features)))
                   events)
               (cl-letf
                   (((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push arguments events)
                       'reported)))
                 (list
                  (ac-js2-company
                   'candidates
                   "al"
                   'ignored)
                  (ac-js2-company
                   'duplicates)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (reported reported (("Company is not installed") ("Company is not installed")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_company_dispatches_interactive_prefix_candidates_duplicates_and_meta() {
    let elisp_form = r##"(let ((already-company
                    (featurep
                     'company))
                   (ac-js2-mode
                    nil)
                   (grabbed-prefix
                    nil)
                   events)
               (provide
                'company)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'company-begin-backend)
                         (lambda (backend)
                           (push
                            (list
                             'begin backend)
                            events)
                           'begun))
                        ((symbol-function
                          'company-grab-symbol)
                         (lambda ()
                           (push '(grab) events)
                           grabbed-prefix))
                        ((symbol-function
                          'ac-js2-candidates)
                         (lambda ()
                           (push '(candidates) events)
                           '("alpha"
                             "beta"
                             "alphabet")))
                        ((symbol-function
                          'ac-js2-document)
                         (lambda (name)
                           (push
                            (list
                             'document name)
                            events)
                           (and
                            (string=
                             name
                             "alpha")
                            "function alpha(value)")))
                        ((symbol-function
                          'js-mode)
                         (lambda ()
                           (push '(js-mode) events)))
                        ((symbol-function
                          'font-lock-ensure)
                         (lambda (&rest arguments)
                           (push
                            (cons
                             'font-lock
                             arguments)
                            events))))
                     (let ((disabled-prefix
                            (ac-js2-company
                             'prefix)))
                       (setq
                        ac-js2-mode t)
                       (let ((stopped-prefix
                              (ac-js2-company
                               'prefix)))
                         (setq
                          grabbed-prefix
                          "alp")
                         (list
                          (ac-js2-company
                           'interactive)
                          disabled-prefix
                          stopped-prefix
                          (ac-js2-company
                           'prefix)
                          (ac-js2-company
                           'candidates
                           "al")
                          (ac-js2-company
                           'duplicates)
                          (ac-js2-company
                           'meta
                           "alpha")
                          (ac-js2-company
                           'meta
                           "missing")
                          (ac-js2-company
                           'unknown
                           "ignored")
                          (nreverse events)))))
                 (unless already-company
                   (setq
                    features
                    (delq
                     'company
                     features)))))"##;
    let expect = expect![[
        r#"OK (begun nil stop "alp" ("alpha" "alphabet") t "function alpha(value)" nil nil (#1=(grab) (begin ac-js2-company) #1# (candidates) (document "alpha") (js-mode) (font-lock) (document "missing")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_setup_auto_complete_prepends_source_enables_mode_and_defines_exact_source() {
    let elisp_form = r##"(progn
               (defvar ac-sources)
               (let ((ac-sources
                    '(fixture-source))
                   events)
                 (cl-letf
                     (((symbol-function
                        'auto-complete-mode)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'mode arguments)
                          events)
                         'mode-enabled))
                      ((symbol-function
                        'ac-define-source)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'define arguments)
                          events)
                         'source-defined)))
                   (list
                    (ac-js2-setup-auto-complete-mode)
                    ac-sources
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (source-defined (ac-source-js2 fixture-source) ((mode) (define "js2" ((candidates . ac-js2-ac-candidates) (document . ac-js2-ac-document) (prefix . ac-js2-ac-prefix) (requires . -1)))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_expand_function_completes_and_expands_exact_yasnippet_parameters() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "call")
               (let ((original-featurep
                      (symbol-function
                       'featurep))
                     events)
                 (cl-letf
                     (((symbol-function
                        'featurep)
                       (lambda (feature)
                         (if
                             (memq
                              feature
                              '(auto-complete
                                yasnippet))
                             t
                           (funcall
                            original-featurep
                            feature))))
                      ((symbol-function
                        'ac-complete)
                       (lambda ()
                         (push '(complete) events)
                         'completed))
                      ((symbol-function
                        'ac-js2-ac-document)
                       (lambda (name)
                         (push
                          (list 'document name)
                          events)
                         "function call(first, second)"))
                      ((symbol-function
                        'yas-expand-snippet)
                       (lambda (snippet)
                         (push
                          (list
                           'expand snippet)
                          events)
                         'expanded)))
                   (list
                    (ac-js2-expand-function)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (expanded ((complete) (document "call") (expand "(${first}, ${second})$0")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}
