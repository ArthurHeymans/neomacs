use expect_test::expect;

use super::assert_anki_mode_parity;

#[test]
fn anki_mode_activates_real_gfm_editing_state() {
    let elisp_form = r##"(with-temp-buffer
         (insert "# Card\n\nQuestion_with_math")
         (anki-mode)
         (list major-mode
               mode-name
               (derived-mode-p 'anki-mode 'gfm-mode 'markdown-mode)
               comment-start
               comment-end
               (buffer-string)
               (local-variable-p 'anki-mode--deck)
               (local-variable-p 'anki-mode--card-type)))"##;
    let expect = expect![[
        r##"OK (anki-mode "Anki" anki-mode "<!-- " " -->" #("# Card\n\nQuestion_with_math" 0 6 (markdown-heading-1-atx (1 7 nil nil nil nil nil nil 1 3 3 7 #<killed buffer>) markdown-heading (1 7 nil nil nil nil nil nil 1 3 3 7 #<killed buffer>))) nil nil)"##
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_inserts_latex_template_and_places_point_inside_it() {
    let elisp_form = r##"(with-temp-buffer
         (insert "Euler: ")
         (anki-mode-insert-latex-math)
         (insert "e^{i\\pi}+1=0")
         (list (buffer-string)
               (point)
               (buffer-substring-no-properties
                (line-beginning-position)
                (line-end-position))))"##;
    let expect =
        expect![[r#"OK ("Euler: [$]e^{i\\pi}+1=0[/$]" 23 "Euler: [$]e^{i\\pi}+1=0[/$]")"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_wraps_active_multiline_region_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
         (transient-mark-mode 1)
         (insert "before\n\\alpha +\n\\beta\nafter")
         (goto-char 8)
         (push-mark 23 t t)
         (setq mark-active t)
         (let ((point-before (point)))
           (anki-mode-insert-latex-math)
           (list (buffer-string)
                 point-before
                 (point)
                 (mark)
                 mark-active)))"##;
    let expect = expect![[r#"OK ("before\n[$]\\alpha +\n\\beta\n[/$]after" 8 8 26 t)"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_next_field_cycles_through_indented_practical_card() {
    let elisp_form = r##"(with-temp-buffer
         (insert "  @Front\nWhat is 2 + 2?\n\n\t@Back\n4\n\n@Extra\nInteger arithmetic")
         (goto-char (point-min))
         (let (visits)
           (dotimes (_ 5)
             (anki-mode-next-field)
             (push (list (point)
                         (buffer-substring-no-properties
                          (line-beginning-position)
                          (line-end-position)))
                   visits))
           (nreverse visits)))"##;
    let expect = expect![[
        r#"OK ((10 "What is 2 + 2?") (33 "4") (43 "Integer arithmetic") (10 "What is 2 + 2?") (33 "4"))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_next_field_without_markers_exposes_exact_error() {
    let elisp_form = r##"(with-temp-buffer
         (insert "ordinary markdown without fields")
         (condition-case err
             (progn (anki-mode-next-field) 'unexpected-success)
           (error (list (car err) (cdr err) (point)))))"##;
    let expect = expect!["OK (wrong-type-argument (integer-or-marker-p nil) 33)"];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_next_field_handles_final_marker_without_newline() {
    let elisp_form = r##"(with-temp-buffer
         (insert "@Front\nquestion\n@Back")
         (goto-char (point-min))
         (anki-mode-next-field)
         (let ((first (list (point) (thing-at-point 'line t))))
           (anki-mode-next-field)
           (list first
                 (point)
                 (thing-at-point 'line t)
                 (buffer-string))))"##;
    let expect = expect![[r#"OK ((8 "question\n") 22 "@Back" "@Front\nquestion\n@Back")"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_parse_fields_handles_markdown_blank_and_multiline_values() {
    let elisp_form = r##"(list
         (anki-mode--parse-fields
          "preamble ignored\n@Front\n**Question** with @inline token\nsecond line\n\n@Back\nAnswer\n\n@Hint\n")
         (anki-mode--parse-fields
          "  @One\n alpha \n\t@Two\n beta\n gamma  ")
         (anki-mode--parse-fields "no field markers")
         (anki-mode--parse-fields "@Only"))"##;
    let expect = expect![[
        r#"OK ((("preamble ignored" . "") ("Front" . "**Question** with @inline token\nsecond line") ("Back" . "Answer") ("Hint" . "")) (("One" . " alpha") ("Two" . " beta\n gamma")) (("no field markers" . "")) (("Only" . "")))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_list_to_pair_preserves_missing_empty_and_extra_items() {
    let elisp_form = r##"(mapcar
         #'anki-mode--list-to-pair
         '(nil ("Front") ("Back" "") ("One" "body" "ignored")))"##;
    let expect = expect![[r#"OK ((nil . "") ("Front" . "") ("Back" . "") ("One" . "body"))"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_max_cloze_handles_none_duplicates_sparse_and_multidigit() {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (insert text)
             (anki-mode--max-cloze)))
         '("plain card"
           "{{c1::one}} and {{c1::again}}"
           "{{c2::two}} {{c9::nine}} {{c3::three}}"
           "prefix {{c12::twelve::hint}} suffix"
           "{{c0::zero}} {{c007::bond}}"
           "{{cX::invalid}} {{c4:malformed}}"))"##;
    let expect = expect!["OK (0 1 9 12 7 0)"];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_cloze_region_builds_sequential_real_card() {
    let elisp_form = r##"(with-temp-buffer
         (insert "Paris is the capital of France; Berlin is the capital of Germany.")
         (let ((paris-start (point-min))
               (paris-end (+ (point-min) 5)))
           (anki-mode-cloze-region paris-start paris-end))
         (let ((berlin-start (save-excursion
                               (goto-char (point-min))
                               (search-forward "Berlin")
                               (- (point) 6)))
               (berlin-end (save-excursion
                             (goto-char (point-min))
                             (search-forward "Berlin")
                             (point))))
           (anki-mode-cloze-region berlin-start berlin-end))
         (list (buffer-string)
               (anki-mode--max-cloze)
               (point)))"##;
    let expect = expect![[
        r#"OK ("{{c1::Paris}} is the capital of France; {{c2::Berlin}} is the capital of Germany." 2 82)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}
