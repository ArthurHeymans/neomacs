use expect_test::expect;

use super::assert_ac_math_parity;

#[test]
fn ac_math_make_candidates_formats_latex_and_unicode_values_and_filters_missing_unicode() {
    let elisp_form = r##"(let ((fixture
                    '(("Greek"
                       "\\alpha"
                       945)
                      ("Arrows"
                       "\\rightarrow"
                       8594)
                      ("Missing"
                       "\\without-code")
                      ("Invalid"
                       "\\invalid-code"
                       1114112))))
               (list
                (ac-math--make-candidates
                 fixture)
                (ac-math--make-candidates
                 fixture t)))"##;
    let expect = expect![[
        r#"OK ((("alpha α" . "alpha") ("rightarrow →" . "rightarrow") ("without-code " . "without-code") ("invalid-code " . "invalid-code")) (("alpha α" . "α") ("rightarrow →" . "→")))"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_make_candidates_uses_the_live_dummy_separator_and_preserves_duplicates() {
    let elisp_form = r##"(let ((ac-math--dummy
                    "::")
                   (fixture
                    '(("One"
                       "\\same"
                       945)
                      ("Two"
                       "\\same"
                       946))))
               (list
                (ac-math--make-candidates
                 fixture)
                (ac-math--make-candidates
                 fixture t)))"##;
    let expect = expect![[
        r#"OK ((("same::α" . "same") ("same::β" . "same")) (("same::α" . "α") ("same::β" . "β")))"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_packaged_candidate_tables_are_deduplicated_and_have_exact_snapshots() {
    let elisp_form = r##"(let ((print-length nil)
                    (print-level nil)
                    (print-circle t)
                    (print-quoted t))
               (list
                (length
                 math-symbol-list-basic)
                (length
                 math-symbol-list-extended)
                (length
                 ac-math-symbols-latex)
                (length
                 ac-math-symbols-unicode)
                (=
                 (length
                  ac-math-symbols-latex)
                 (length
                  (delete-dups
                   (copy-tree
                    ac-math-symbols-latex))))
                (=
                 (length
                  ac-math-symbols-unicode)
                 (length
                  (delete-dups
                   (copy-tree
                    ac-math-symbols-unicode))))
                (mapcar
                 (lambda (name)
                   (list
                    name
                    (assoc
                     name
                     ac-math-symbols-latex)
                    (assoc
                     name
                     ac-math-symbols-unicode)))
                 '("alpha α"
                   "rightarrow →"
                   "BbbA 𝔸"
                   "definitely-missing"))
                (secure-hash
                 'sha256
                 (prin1-to-string
                  ac-math-symbols-latex))
                (secure-hash
                 'sha256
                 (prin1-to-string
                  ac-math-symbols-unicode))))"##;
    let expect = expect![[
        r#"OK (279 2750 2824 2774 t t (("alpha α" ("alpha α" . "alpha") ("alpha α" . "α")) ("rightarrow →" ("rightarrow →" . "rightarrow") ("rightarrow →" . "→")) ("BbbA 𝔸" ("BbbA 𝔸" . "BbbA") ("BbbA 𝔸" . "𝔸")) ("definitely-missing" nil nil)) "fbad6f30d0b8125a9e1596f603974b47b0e09cfa8df582cff86af623ef2a8fdb" "87780f11f631c54bc5cfc4c6142bd607adba165ff8d401eedc2b9449ca795dd5")"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}
