use expect_test::expect;

use super::{assert_ac_alchemist_parity, assert_ac_alchemist_signal_parity};

#[test]
fn ac_alchemist_get_prefixed_string_accepts_exact_identifier_characters_and_preserves_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "before Alpha.beta_gamma:delta after")
               (goto-char 4)
               (let ((end
                      (progn
                        (search-forward
                         "delta")
                        (point))))
                 (goto-char 4)
                 (list
                  (ac-alchemist--get-prefixed-string
                   end)
                  (point)
                  (ac-alchemist--get-prefixed-string
                   (- end 6))
                  (point)
                  (ac-alchemist--get-prefixed-string
                   (point-min)))))"##;
    let expect = expect![[r#"OK ("Alpha.beta_gamma:delta" 4 "Alpha.beta_gamma" 4 "")"#]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_get_prefixed_string_rejects_an_end_beyond_the_buffer() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abc")
               (ac-alchemist--get-prefixed-string
                (+ (point-max) 1)))"##;
    let expect = expect!["ERR (args-out-of-range (:buffer nil) 1 5)"];

    assert_ac_alchemist_signal_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_complete_request_stores_prefix_and_sends_exact_context_and_callback() {
    let elisp_form = r##"(with-temp-buffer
               (insert "ignore Alpha.beta")
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'alchemist-server-complete-candidates)
                       (lambda (request callback)
                         (push
                          (list request callback)
                          calls)
                         'request-result)))
                   (list
                    (ac-alchemist--complete-request)
                    ac-alchemist--prefix
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (request-result "Alpha.beta" (("{ \"Alpha.beta\", [ context: [], imports: [], aliases: [] ] }" ac-alchemist--complete-filter)))"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_document_filter_waits_for_marker_then_prepares_filters_and_stores_output() {
    let elisp_form = r##"(let ((ac-alchemist--document 'old)
                    events)
               (cl-letf
                   (((symbol-function
                      'alchemist-server-contains-end-marker-p)
                     (lambda (output)
                       (push
                        (list 'contains output)
                        events)
                       (string-match-p
                        "END-OF"
                        output)))
                    ((symbol-function
                      'alchemist-server-prepare-filter-output)
                     (lambda (outputs)
                       (push
                        (list 'prepare outputs)
                        events)
                       "prepared"))
                    ((symbol-function
                      'ansi-color-filter-apply)
                     (lambda (output)
                       (push
                        (list 'ansi output)
                        events)
                       "filtered")))
                 (list
                  (alchemist-company-doc-buffer-filter
                   'process
                   "partial")
                  ac-alchemist--document
                  (alchemist-company-doc-buffer-filter
                   'process
                   "final END-OF")
                  ac-alchemist--document
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil old "filtered" "filtered" ((contains "partial") (contains "final END-OF") (prepare ("final END-OF")) (ansi "prepared")))"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_document_query_preserves_plain_candidate_identity_and_builds_qualified_arity() {
    let elisp_form = r##"(with-temp-buffer
               (insert "Alias.Module.fun")
               (let* ((ac-point
                       (point-max))
                      (plain
                       (copy-sequence "plain"))
                      (qualified
                       (propertize
                        "member"
                        'symbol "/3"))
                      plain-result
                      qualified-result)
                 (let ((ac-alchemist--prefix
                        "plain"))
                   (setq plain-result
                         (ac-alchemist--document-query
                          plain)))
                 (let ((ac-alchemist--prefix
                        "Alias.Module."))
                   (setq qualified-result
                         (ac-alchemist--document-query
                          qualified)))
                 (list
                  plain-result
                  (eq plain plain-result)
                  qualified-result
                  (substring-no-properties
                   qualified)
                  (get-text-property
                   0 'symbol qualified))))"##;
    let expect = expect![[
        r#"OK ("plain" t #("Alias.Module.funmember/3" 16 22 (symbol "/3")) "member" "/3")"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_show_document_resets_state_builds_query_requests_help_waits_and_returns_document() {
    let elisp_form = r##"(let ((ac-alchemist--document
                    'stale)
                   (alchemist-company-doc-lookup-done
                    'stale-done)
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-alchemist--document-query)
                     (lambda (candidate)
                       (push
                        (list
                         'query
                         candidate
                         ac-alchemist--document)
                        events)
                       "query"))
                    ((symbol-function
                      'alchemist-help--prepare-search-expr)
                     (lambda (query)
                       (push
                        (list 'prepare query)
                        events)
                       "prepared-query"))
                    ((symbol-function
                      'alchemist-help--server-arguments)
                     (lambda (query)
                       (push
                        (list 'arguments query)
                        events)
                       'server-arguments))
                    ((symbol-function
                      'alchemist-server-help)
                     (lambda (arguments callback)
                       (push
                        (list
                         'help
                         arguments
                         callback
                         alchemist-company-doc-lookup-done)
                        events)
                       (setq ac-alchemist--document
                             "documentation")
                       'help-result))
                    ((symbol-function 'sit-for)
                     (lambda (seconds)
                       (push
                        (list
                         'sit
                         seconds
                         ac-alchemist--document)
                        events)
                       t)))
                 (list
                  (ac-alchemist--show-document
                   "candidate")
                  alchemist-company-doc-lookup-done
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("documentation" stale-done ((query "candidate" nil) (prepare "query") (arguments "prepared-query") (help server-arguments alchemist-company-doc-buffer-filter stale-done) (sit 0.1 "documentation")))"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_prefix_returns_start_after_last_separator_only_for_matching_suffixes() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "Alpha.beta\n"
                "  gamma_delta:tail\n"
                "no-match!\n")
               (list
                (progn
                  (goto-char (point-min))
                  (search-forward "beta")
                  (list
                   (ac-alchemist--prefix)
                   (point)))
                (progn
                  (search-forward "tail")
                  (list
                   (ac-alchemist--prefix)
                   (point)))
                (progn
                  (search-forward "!")
                  (list
                   (ac-alchemist--prefix)
                   (point)))
                (progn
                  (goto-char (point-min))
                  (list
                   (ac-alchemist--prefix)
                   (point)))))"##;
    let expect = expect!["OK ((7 11) (14 30) (nil 40) (nil 1))"];

    assert_ac_alchemist_parity(elisp_form, expect);
}
