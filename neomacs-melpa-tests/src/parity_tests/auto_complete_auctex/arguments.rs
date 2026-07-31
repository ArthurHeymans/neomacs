use expect_test::expect;

use super::assert_auto_complete_auctex_batch;

#[test]
fn arguments_public_surface_batch() {
    assert_auto_complete_auctex_batch(&[
        (
            "auto_complete_auctex_expand_argument_info_preserves_literal_and_optional_arguments",
            r##"(ac-auctex-expand-arg-info
          '("Required"
            ["Optional"]
            ""
            [""]))"##,
            true,
            expect![[r#"OK ("Required" ["Optional"] "" [""])"#]],
        ),
        (
            "auto_complete_auctex_expand_argument_info_resolves_direct_auctex_argument_functions",
            r##"(ac-auctex-expand-arg-info
          '(TeX-arg-file
            TeX-arg-ref
            LaTeX-arg-usepackage
            LaTeX-env-tabular*
            LaTeX-env-item))"##,
            true,
            expect![[
        r#"OK ("Filename" "Name" ["opt1,..."] "Package" "Width" ["htbp!"] "lcrpmb|><" "")"#
    ]],
        ),
        (
            "auto_complete_auctex_expand_argument_info_resolves_nested_function_specs_by_head",
            r##"(ac-auctex-expand-arg-info
          '((TeX-arg-file :extensions ("png" "pdf"))
            (TeX-arg-ref :prompt "Cross reference")
            (LaTeX-env-minipage :placement)
            (unknown-handler :metadata)))"##,
            true,
            expect![[r#"OK ("Filename" "Name" ["htbp!"] "Width" "")"#]],
        ),
        (
            "auto_complete_auctex_expand_argument_info_turns_vector_handlers_into_optional_arguments",
            r##"(ac-auctex-expand-arg-info
          '([TeX-arg-file]
            [(TeX-arg-ref :prompt "Reference")]
            [LaTeX-env-array]
            [unknown-handler]))"##,
            true,
            expect!["OK (#1=[item-2] #1# #1# #1# #1#)"],
        ),
        (
            "auto_complete_auctex_expand_argument_info_supports_numeric_auctex_arities",
            r##"(mapcar
          (lambda (arity)
            (list
             arity
             (ac-auctex-expand-arg-info
              (list arity))))
          '(2 5 9))"##,
            true,
            expect![[r#"OK ((2 ("" "")) (5 ("" "" "" "" "")) (9 ("" "" "" "" "" "" "" "" "")))"#]],
        ),
        (
            "auto_complete_auctex_expand_argument_info_defaults_unknown_and_empty_specs",
            r##"(list
          (ac-auctex-expand-arg-info
           '(unknown-handler))
          (ac-auctex-expand-arg-info
           '((unknown-handler :anything)))
          (ac-auctex-expand-arg-info nil))"##,
            true,
            expect![[r#"OK (("") ("") nil)"#]],
        ),
        (
            "auto_complete_auctex_snippet_argument_numbers_required_and_optional_fields_exactly",
            r##"(mapcar
          (lambda (fixture)
            (apply
             #'ac-auctex-snippet-arg
             fixture))
          '((1 "Title")
            (2 ["Short title"])
            (7 "")
            (9 [""])))"##,
            true,
            expect![[r#"OK ((2 "{${Title}}") (4 "${[${Short title}]}") (8 "{${}}") (11 "${[${}]}"))"#]],
        ),
        (
            "auto_complete_auctex_macro_snippet_builds_practical_section_graphics_and_tabular_fields",
            r##"(mapcar
          (lambda (fixture)
            (list
             (car fixture)
             (ac-auctex-macro-snippet
              (cdr fixture))))
          '(("section"
             ["Short title"]
             "Title")
            ("includegraphics"
             ["width=0.8\\textwidth"]
             TeX-arg-file)
            ("tabular*"
             LaTeX-env-tabular*)
            ("empty")))"##,
            true,
            expect![[
        r#"OK (("section" "${[${Short title}]}{${Title}}") ("includegraphics" "${[${width=0.8\\textwidth}]}{${Filename}}") ("tabular*" "{${Width}}${[${htbp!}]}{${lcrpmb|><}}") ("empty" ""))"#
    ]],
        ),
        (
            "auto_complete_auctex_macro_snippet_mixes_expanded_arity_handlers_and_literal_fields",
            r##"(ac-auctex-macro-snippet
          '(3
            ["Options"]
            TeX-arg-ref
            (TeX-arg-file :extensions ("tex"))
            "Tail"))"##,
            true,
            expect![[r#"OK "{${}}{${}}{${}}${[${Options}]}{${Name}}{${Filename}}{${Tail}}""#]],
        ),
        (
            "auto_complete_auctex_expand_args_finds_environment_entry_and_forwards_exact_snippet",
            r##"(let ((environment
                                '(("figure"
                                   ["htbp!"])
                                  ("table"
                                   ["tbp"]
                                   "Columns")))
                               captured)
          (fset
           'yas/expand-snippet
           (lambda (snippet &rest arguments)
             (setq captured
                   (list
                    snippet
                    arguments))
             :expanded))
          (list
           (ac-auctex-expand-args
            "table"
            environment)
           captured
           (ac-auctex-expand-args
            "missing"
            environment)
           captured))"##,
            true,
            expect![[r#"OK (:expanded ("${[${tbp}]}{${Columns}}" nil) :expanded ("" nil))"#]],
        ),
    ]);
}
