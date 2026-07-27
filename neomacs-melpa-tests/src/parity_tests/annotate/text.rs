use expect_test::expect;

use super::assert_annotate_parity;

#[test]
fn annotate_wrap_and_unwrap_text_handle_custom_multichar_delimiters() {
    let elisp_form = r##"(list
         (annotate-wrap-text "note")
         (annotate-wrap-text "body" "**")
         (annotate-unwrap-text "\"quoted\"")
         (annotate-unwrap-text "**bold**" "**")
         (annotate-unwrap-text "**left-only" "**")
         (annotate-unwrap-text "right-only**" "**")
         (annotate-unwrap-text "*" "**")
         (annotate-unwrap-text "" "**"))"##;
    let expect = expect![[
        r#"OK ("\"note\"" "**body**" "quoted" "bold" "left-only" "right-only**" "*" "")"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_prefix_lines_preserves_empty_and_trailing_lines() {
    let elisp_form = r##"(list
         (annotate-prefix-lines "> " "one\ntwo\n")
         (annotate-prefix-lines "│ " "one\n\ntwo")
         (annotate-prefix-lines "# " "" nil)
         (annotate-prefix-lines "# " "one\n" t))"##;
    let expect = expect![[r##"OK ("> one\n> two\n> \n" "│ one\n│ \n│ two\n" "# \n" "# one\n")"##]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_safe_subseq_exposes_boundary_and_fallback_semantics() {
    let elisp_form = r##"(mapcar
         (lambda (args)
           (condition-case err
               (apply #'annotate-safe-subseq args)
             (error (list (car err) (cdr err)))))
         '(("abcdef" 1 4)
           ("abcdef" 0 99)
           ("abcdef" -3 4)
           ("abcdef" 5 2 "fallback")
           ((a b c d) 1 3)
           ([] 0 1 fallback)))"##;
    let expect = expect![[r#"OK ("bcd" "abcdef" "abcdef" "fallback" (b c) fallback)"#]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_split_lines_and_join_round_trip_practical_multiline_text() {
    let elisp_form = r##"(let* ((text "first line\n\nthird line\n")
               (lines (annotate--split-lines text)))
         (list lines
               (annotate--join-with-string lines "\n")
               (annotate--split-lines "single")
               (annotate--split-lines "")
               (annotate--split-lines "a|b||c" "|")))"##;
    let expect = expect![[
        r#"OK (("first line" "" "third line" "") "first line\n\nthird line\n" ("single") ("") ("a" "b" "" "c"))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_group_by_width_handles_ascii_long_words_and_wide_characters() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (annotate-group-by-width (car case) (cadr case)))
         '(("one two three four" 7)
           ("supercalifragilistic small" 5)
           ("αβ γδ 日本語 text" 6)
           ("" 4)
           ("one" 1)))"##;
    let expect = expect![[
        r#"OK (("one two" "three" "four") ("super" "calif" "ragil" "istic" " smal" "l") ("αβ γδ" "日本語" "text") nil ("o" "n" "e"))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_lineate_wraps_real_annotation_paragraphs_by_display_width() {
    let elisp_form = r##"(list
         (annotate-lineate "This is a practical annotation explaining a code review finding." 18)
         (annotate-lineate "日本語 text with wide glyphs" 10)
         (annotate-lineate "one\nmanual newline\nthree" 12)
         (condition-case err
             (annotate-lineate "" 8)
           (error (list (car err) (cdr err)))))"##;
    let expect = expect![[
        r#"OK ("This is a        \npractical        \nannotation       \nexplaining a code\nreview finding.  " "日本語   \ntext with\nwide     \nglyphs   " "one\nmanual   \nnewline\nthree" (wrong-number-of-arguments (#<subr max> 0)))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_comment_helpers_follow_major_mode_and_fallback_contract() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (emacs-lisp-mode)
           (list (annotate-actual-comment-start)
                 (annotate-actual-comment-end)
                 (annotate-comments-length)
                 (annotate-wrap-in-comment " ANNOTATION: " "review this")))
         (with-temp-buffer
           (setq comment-start nil
                 comment-end nil)
           (let ((annotate-fallback-comment "#"))
             (list (annotate-actual-comment-start)
                   (annotate-actual-comment-end)
                   (annotate-comments-length)
                   (annotate-wrap-in-comment "note")))))"##;
    let expect = expect![[r##"OK ((";" "" 1 "; ANNOTATION: review this") ("#" "" 1 "#note"))"##]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_expansion_map_applies_shell_substitutions_and_trim_policy() {
    let elisp_form = r##"(let ((annotate-annotation-expansion-map
               '(("%project" "printf neomacs" t)
                 ("%raw" "printf '  padded  '" nil))))
         (list
          (annotate--expand-annotation-text
           "Review %project: %raw.")
          (annotate--expand-annotation-text
           "No placeholders here")
          (annotate--expand-annotation-text
           "%project/%project")))"##;
    let expect =
        expect![[r#"OK ("Review neomacs:   padded  ." "No placeholders here" "neomacs/neomacs")"#]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_buffer_checksum_is_stable_for_string_buffer_and_region_changes() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha\nbeta\n")
         (let ((whole (annotate-buffer-checksum))
               (string (annotate-buffer-checksum "alpha\nbeta\n")))
           (delete-region 7 11)
           (list whole
                 string
                 (annotate-buffer-checksum)
                 (= (point-min) 1)
                 (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("852e77b490fb4e8653fbc11f4c6f89c2" "852e77b490fb4e8653fbc11f4c6f89c2" "77e1dedf90469036343396eb96c780bc" t "alpha\n\n")"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}
