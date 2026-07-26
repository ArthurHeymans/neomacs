use expect_test::expect;

use super::assert_ac_alchemist_parity;

#[test]
fn ac_alchemist_candidates_split_numeric_arities_and_preserve_literal_fallbacks() {
    let elisp_form = r##"(let ((ac-alchemist--candidate-cache
                    '("map/2"
                      "plain"
                      "Nested.name/12"
                      "slash/x"
                      "/3"
                      "trail/4 extra")))
               (mapcar
                (lambda (candidate)
                  (list
                   (substring-no-properties
                    candidate)
                   (get-text-property
                    0 'symbol candidate)
                   (text-properties-at
                    0 candidate)))
                (ac-alchemist--candidates)))"##;
    let expect = expect![[
        r#"OK (("map" "/2" (symbol "/2")) ("plain" "  " (symbol "  ")) ("Nested.name" "/12" (symbol "/12")) ("slash/x" "  " (symbol "  ")) ("/3" "  " (symbol "  ")) ("trail" "/4" (symbol "/4")))"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_candidates_leave_cache_unchanged_and_expose_final_match_data() {
    let elisp_form = r##"(let* ((first
                     (copy-sequence "one/1"))
                    (second
                     (copy-sequence "two/22"))
                    (ac-alchemist--candidate-cache
                     (list first "plain" second))
                    (cache
                     ac-alchemist--candidate-cache)
                    (result
                     (ac-alchemist--candidates)))
               (list
                (eq cache
                    ac-alchemist--candidate-cache)
                (eq first
                    (car
                     ac-alchemist--candidate-cache))
                ac-alchemist--candidate-cache
                (mapcar
                 #'substring-no-properties
                 result)
                (match-data)))"##;
    let expect =
        expect![[r#"OK (t t ("one/1" "plain" "two/22") ("one" "plain" "two") (0 6 0 3 3 6))"#]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_merge_candidates_concatenates_chunks_stops_at_marker_and_reverses_lines() {
    let elisp_form = r##"(list
               (ac-alchemist--merge-candidates
                '("alpha\nbe"
                  "ta\n\nEND-OF-CANDIDATES\nignored\n"))
               (ac-alchemist--merge-candidates
                '("END-OF\nignored"))
               (ac-alchemist--merge-candidates
                '("one\nEND-OF"
                  "-NOT-A-MARKER\ntwo\nEND-OF\n")))"##;
    let expect = expect![[r#"OK (("" "beta" "alpha") nil ("one"))"#]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_complete_filter_accumulates_reverse_chunks_then_flushes_on_marker() {
    let elisp_form = r##"(let ((ac-alchemist--output-cache nil)
                    (ac-alchemist--candidate-cache
                     '(old))
                    checks)
               (cl-letf
                   (((symbol-function
                      'alchemist-server-contains-end-marker-p)
                     (lambda (output)
                       (push output checks)
                       (string-match-p
                        "END-OF"
                        output))))
                 (list
                  (ac-alchemist--complete-filter
                   'process
                   "first\n")
                  ac-alchemist--output-cache
                  ac-alchemist--candidate-cache
                  (ac-alchemist--complete-filter
                   'process
                   "second\n")
                  ac-alchemist--output-cache
                  (ac-alchemist--complete-filter
                   'process
                   "third\nEND-OF\n")
                  ac-alchemist--output-cache
                  ac-alchemist--candidate-cache
                  (nreverse checks))))"##;
    let expect = expect![[
        r#"OK (nil #1=("first\n") (old) nil ("second\n" . #1#) #2=("third") nil #2# ("first\n" "second\n" "third\nEND-OF\n"))"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_do_complete_filters_before_starting_completion_and_returns_start_value() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-alchemist--complete-filter)
                     (lambda (process output)
                       (push
                        (list
                         'filter
                         process
                         output)
                        events)
                       'filter-result))
                    ((symbol-function 'ac-start)
                     (lambda ()
                       (push '(start) events)
                       'start-result)))
                 (list
                  (ac-alchemist--do-complete
                   'process
                   "output")
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (start-result ((filter process "output") (start)))"#]];

    assert_ac_alchemist_parity(elisp_form, expect);
}
