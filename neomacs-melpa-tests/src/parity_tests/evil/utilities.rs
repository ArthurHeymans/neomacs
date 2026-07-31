use expect_test::expect;

use super::{assert_evil_batch};

#[test]
fn utilities_public_surface_batch() {
    assert_evil_batch(&[
        (
            "evil_property_helpers_set_merge_and_query_single_and_all_symbols",
            r##"(progn
               (defvar neomacs-evil-properties nil)
               (setq neomacs-evil-properties nil)
               (evil-put-property
                'neomacs-evil-properties 'alpha :foo t)
               (evil-put-property
                'neomacs-evil-properties 'alpha :bar nil)
               (evil-put-property
                'neomacs-evil-properties 'beta
                :foo nil :bar 'value :baz t)
               (list
                neomacs-evil-properties
                (evil-get-property
                 neomacs-evil-properties 'alpha :foo)
                (evil-get-property
                 neomacs-evil-properties 'alpha :bar)
                (evil-get-property
                 neomacs-evil-properties 'alpha :missing)
                (evil-get-property
                 neomacs-evil-properties t :foo)
                (evil-get-property
                 neomacs-evil-properties t :bar)
                (evil-get-property
                 neomacs-evil-properties t :baz)))"##,
            true,
            expect![
        "OK (((beta :foo nil :bar value :baz t) (alpha :foo t :bar nil)) t nil nil ((alpha . t) (beta)) ((alpha) (beta . value)) ((beta . t)))"
    ],
        ),
        (
            "evil_filter_list_returns_and_destructively_relinks_nonmatching_items",
            r##"(let ((one '(nil))
                    (two '(nil 1 2 nil))
                    (three '(1 nil nil 2))
                    (four '(1 nil 2 nil 3)))
               (list
                (evil-filter-list #'null one)
                (evil-filter-list #'null two)
                (progn
                  (evil-filter-list #'null three)
                  three)
                (progn
                  (evil-filter-list #'null four)
                  four)))"##,
            true,
            expect!["OK (nil (1 2) (1 2) (1 2 3))"],
        ),
        (
            "evil_concat_helpers_deduplicate_lists_and_override_association_values",
            r##"(list
               (evil-concat-lists nil '(a b) '(b c))
               (evil-concat-lists '(a a b) nil '(b c) nil)
               (evil-concat-alists
                '((a . one) (b . two))
                '((a . three) (c . four)))
               (evil-concat-plists
                '(:a one :b two)
                '(:a three :c four)))"##,
            true,
            expect![
        "OK ((a b c) (a b c) ((b . two) (a . three) (c . four)) (:a three :b two :c four))"
    ],
        ),
        (
            "evil_sort_macro_orders_two_three_and_four_places_in_the_calling_scope",
            r##"(list
               (let ((a 2) (b 1))
                 (evil-sort a b)
                 (list a b))
               (let ((a 3) (b 1) (c 2))
                 (evil-sort a b c)
                 (list a b c))
               (let ((a 4) (b 3) (c 2) (d 1))
                 (evil-sort a b c d)
                 (list a b c d)))"##,
            true,
            expect!["OK ((1 2) (1 2 3) (1 2 3 4))"],
        ),
        (
            "evil_extract_count_parses_exact_commands_counts_and_trailing_keys",
            r##"(with-temp-buffer
               (evil-local-mode 1)
               (evil-normal-state)
               (list
                (evil-extract-count "x")
                (evil-extract-count "g0")
                (evil-extract-count "420x")
                (evil-extract-count "2301g0")
                (evil-extract-count "xAB")
                (evil-extract-count "2301g0CD")
                (evil-extract-count "0")
                (evil-extract-count "0XY")))"##,
            true,
            expect![[
        r#"OK ((nil evil-delete-char "x" nil) (nil evil-beginning-of-visual-line "g0" nil) (420 evil-delete-char "x" nil) (2301 evil-beginning-of-visual-line "g0" nil) (nil evil-delete-char "x" "AB") (2301 evil-beginning-of-visual-line "g0" "CD") (nil evil-beginning-of-line "0" nil) (nil evil-beginning-of-line "0" "XY"))"#
    ]],
        ),
        (
            "evil_extract_count_rejects_a_count_without_a_command",
            r##"(with-temp-buffer
               (evil-local-mode 1)
               (evil-normal-state)
               (evil-extract-count "1230"))"##,
            false,
            expect![[r#"ERR (user-error "Key sequence contains no complete binding")"#]],
        ),
        (
            "evil_vim_regexp_transform_handles_classes_escaped_backslashes_and_magic_modes",
            r##"(let ((patterns
                    '("x\\sx"
                      "x\\Dx"
                      "x\\wx"
                      "x\\Lx"
                      "x\\\\sx"
                      "x\\\\\\sx")))
               (list
                (let ((evil-magic t))
                  (mapcar
                   #'evil-transform-vim-style-regexp
                   patterns))
                (let ((evil-magic nil))
                  (mapcar
                   #'evil-transform-vim-style-regexp
                   patterns))))"##,
            true,
            expect![[
        r#"OK (("x[[:space:]]x" "x[^[:digit:]]x" "x\\wx" "x[^a-z]x" "x\\\\sx" "x\\\\[[:space:]]x") ("x[[:space:]]x" "x[^[:digit:]]x" "x\\wx" "x[^a-z]x" "x\\\\sx" "x\\\\[[:space:]]x"))"#
    ]],
        ),
        (
            "evil_digraph_lookup_handles_builtin_reverse_user_and_missing_pairs",
            r##"(progn
               (require 'evil-digraphs)
               (let ((evil-digraphs-table-user
                      '(((?x ?y) . 9731)
                        ((?a ?b) . 9733))))
                 (list
                  (evil-digraph '(?x ?y))
                  (evil-digraph '(?y ?x))
                  (evil-digraph '(?a ?a))
                  (evil-digraph '(?A ?:))
                  (evil-digraph '(?~ ?~))
                  (evil-digraph '(?q ?q)))))"##,
            true,
            expect!["OK (9731 9731 229 196 nil nil)"],
        ),
    ]);
}
