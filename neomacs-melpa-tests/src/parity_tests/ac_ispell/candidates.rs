use expect_test::expect;

use super::assert_ac_ispell_parity;

#[test]
fn ac_ispell_case_function_classifies_ascii_prefix_shapes_exactly() {
    let elisp_form = r##"(mapcar
               (lambda (input)
                 (list
                  input
                  (ac-ispell--case-function
                   input)))
               '(""
                 "a"
                 "ab"
                 "A"
                 "Ab"
                 "AB"
                 "ABC"
                 "aB"
                 "1A"
                 "ÄB"))"##;
    let expect = expect![[
        r#"OK (("" identity) ("a" identity) ("ab" identity) ("A" capitalize) ("Ab" capitalize) ("AB" upcase) ("ABC" upcase) ("aB" identity) ("1A" identity) ("ÄB" identity))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_candidates_downcases_lookup_and_restores_lower_capital_and_upper_case() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 4))
                   (ispell-complete-word-dict
                    "words.dict")
                   events)
               (cl-letf
                   (((symbol-function
                      'ispell-lookup-words)
                     (lambda (&rest arguments)
                       (push arguments events)
                       '("wording" "wonder"))))
                 (let ((cases
                        (mapcar
                         (lambda (prefix)
                           (setq
                            ac-prefix prefix
                            ac-ispell--cache
                            (make-ring 4))
                           (list
                            prefix
                            (ac-ispell--candidates)))
                         '("word" "Word" "WORD"))))
                   (list
                    cases
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ((("word" ("wording" "wonder")) ("Word" ("Wording" "Wonder")) ("WORD" ("WORDING" "WONDER"))) (("word*" "words.dict") ("word*" "words.dict") ("word*" "words.dict")))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_candidates_rejects_non_ascii_word_prefixes_without_lookup() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 4))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ispell-lookup-words)
                     (lambda (&rest _arguments)
                       (setq calls
                             (1+ (or calls 0)))
                       '("unexpected"))))
                 (list
                  (mapcar
                   (lambda (prefix)
                     (let ((ac-prefix prefix))
                       (ac-ispell--candidates)))
                   '(""
                     "wo-rd"
                     "word2"
                     "two words"
                     "λword"))
                  calls
                  (ring-length
                   ac-ispell--cache))))"##;
    let expect = expect![[r#"OK ((nil nil nil nil nil) nil 0)"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_candidates_reuses_a_shorter_cached_prefix_without_second_lookup() {
    let elisp_form = r##"(let ((ac-ispell--cache
                    (make-ring 4))
                   (ispell-complete-word-dict
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ispell-lookup-words)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       '("wording" "wordless"))))
                 (let ((ac-prefix
                        "word"))
                   (ac-ispell--candidates))
                 (let ((ac-prefix
                        "wordl"))
                   (list
                    (ac-ispell--candidates)
                    (nreverse calls)
                    (ring-elements
                     ac-ispell--cache)))))"##;
    let expect =
        expect![[r#"OK (("wording" "wordless") (("word*" nil)) (("word" "wording" "wordless")))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_candidates_falls_back_to_legacy_lookup_words_when_needed() {
    let elisp_form = r##"(let ((ac-prefix
                    "legacy")
                   (ac-ispell--cache
                    (make-ring 2))
                   (ispell-complete-word-dict
                    "legacy.dict")
                   events)
               (fmakunbound
                'ispell-lookup-words)
               (cl-letf
                   (((symbol-function
                      'lookup-words)
                     (lambda (&rest arguments)
                       (push arguments events)
                       '("legacy" "legacies"))))
                 (list
                  (ac-ispell--candidates)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (("legacy" "legacies") (("legacy*" "legacy.dict")))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}
