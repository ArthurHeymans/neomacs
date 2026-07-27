use expect_test::expect;

use super::assert_agda_lib_mode_parity;

#[test]
fn agda_lib_mode_comment_and_uncomment_region_round_trip_a_real_library_fragment() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "include: src\n"
          "depend: base\n"
          "\n"
          "flags: --safe\n")
         (agda-lib-mode)
         (goto-char
          (point-min))
         (set-mark
          (save-excursion
            (forward-line 2)
            (point)))
         (activate-mark)
         (comment-region
          (region-beginning)
          (region-end))
         (let ((commented
                (buffer-string))
               (comment-point
                (point))
               (comment-mark
                (mark)))
           (uncomment-region
            (point-min)
            (save-excursion
              (goto-char
               (point-min))
              (forward-line 2)
              (point)))
           (list
            commented
            (buffer-string)
            comment-point
            comment-mark
            (point)
            (mark)
            (buffer-modified-p))))"##;
    let expect = expect![[
        r#"OK ("-- include: src\n-- depend: base\n\nflags: --safe\n" "include: src\ndepend: base\n\nflags: --safe\n" 1 33 1 27 t)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_comment_line_handles_counts_blank_lines_and_existing_comment_text() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name: sample\n"
          "\n"
          "-- already descriptive\n"
          "include: src\n")
         (agda-lib-mode)
         (goto-char
          (point-min))
         (comment-line 2)
         (let ((after-forward
                (list
                 (buffer-string)
                 (line-number-at-pos)
                 (current-column))))
           (goto-char
            (point-max))
           (forward-line -1)
           (comment-line -2)
           (list
            after-forward
            (buffer-string)
            (line-number-at-pos)
            (current-column))))"##;
    let expect = expect![[
        r#"OK (("-- name: sample\n\n-- already descriptive\ninclude: src\n" 3 0) "-- name: sample\n\nalready descriptive\ninclude: src\n" 2 0)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_comment_dwim_repeats_end_of_line_markers_without_syntax_comments() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name: sample")
         (agda-lib-mode)
         (goto-char
          (point-max))
         (comment-dwim nil)
         (insert
          "library name")
         (let ((commented
                (buffer-string))
               (inside-comment
                (nth 4
                     (syntax-ppss))))
           (comment-dwim nil)
           (list
            commented
            (buffer-string)
            inside-comment
            (point)
            (current-column))))"##;
    let expect = expect![[
        r#"OK ("name: sample\11\11\11-- library name" "name: sample\11\11\11-- library name -- " nil 35 51)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_fill_paragraph_wraps_real_comment_text_with_agda_comment_prefixes() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "-- This library exposes algebraic structures and carefully selected experimental modules.")
         (agda-lib-mode)
         (setq-local
          fill-column
          32)
         (goto-char 45)
         (fill-paragraph)
         (list
          (buffer-string)
          (point)
          (line-number-at-pos)
          (current-column)
          fill-prefix
          adaptive-fill-mode))"##;
    let expect = expect![[
        r#"OK ("-- This library exposes\n-- algebraic structures and\n-- carefully selected\n-- experimental modules." 48 2 23 nil t)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_fill_paragraph_wraps_field_values_without_inventing_comment_prefixes() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "include: source generated experimental platform-specific integration")
         (agda-lib-mode)
         (setq-local
          fill-column
          28)
         (goto-char 40)
         (fill-paragraph)
         (list
          (buffer-string)
          (point)
          (line-number-at-pos)
          (current-indentation)))"##;
    let expect = expect![[
        r#"OK ("include: source generated\nexperimental\nplatform-specific\nintegration" 40 3 0)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_indent_for_tab_command_aligns_continuation_entries_practically() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "include: src\n"
          "generated\n"
          "experimental\n"
          "\n"
          "depend: base\n"
          "stdlib\n")
         (agda-lib-mode)
         (goto-char
          (point-min))
         (forward-line 1)
         (indent-for-tab-command)
         (forward-line 1)
         (indent-for-tab-command)
         (forward-line 3)
         (indent-for-tab-command)
         (list
          (buffer-string)
          (line-number-at-pos)
          (current-indentation)
          (current-column)))"##;
    let expect = expect![[
        r#"OK ("include: src\n\11 generated\n\11 experimental\n\ndepend: base\n\11stdlib\n" 6 8 8)"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_newline_and_indent_preserves_text_mode_editing_semantics() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "include: src")
         (agda-lib-mode)
         (goto-char
          (point-min))
         (search-forward
          "src")
         (newline-and-indent)
         (insert "generated")
         (newline-and-indent)
         (insert "experimental")
         (newline-and-indent)
         (insert "-- local extension")
         (list
          (buffer-string)
          (point)
          (line-number-at-pos)
          (current-indentation)
          (buffer-modified-p)))"##;
    let expect =
        expect![[r#"OK ("include: src\ngenerated\nexperimental\n-- local extension" 55 4 0 t)"#]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_real_edit_comment_and_refontify_workflow_updates_text_and_faces() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name sample\n"
          "include: src\n"
          "depend: base\n")
         (agda-lib-mode)
         (goto-char
          (point-min))
         (search-forward
          "name")
         (insert ":")
         (forward-line 1)
         (comment-line 1)
         (font-lock-ensure)
         (let ((faces
                (mapcar
                 (lambda (needle)
                   (goto-char
                    (point-min))
                   (search-forward needle)
                   (get-text-property
                    (match-beginning 0)
                    'face))
                 '("name:"
                   "-- include:"
                   "depend:"))))
           (uncomment-region
            (save-excursion
              (goto-char
               (point-min))
              (forward-line 1)
              (point))
            (save-excursion
              (goto-char
               (point-min))
              (forward-line 2)
              (point)))
           (font-lock-flush)
           (font-lock-ensure)
           (list
            faces
            (buffer-string)
            (mapcar
             (lambda (needle)
               (goto-char
                (point-min))
               (search-forward needle)
               (get-text-property
                (match-beginning 0)
                'face))
             '("name:"
               "include:"
               "depend:")))))"##;
    let expect = expect![[
        r#"OK ((font-lock-keyword-face font-lock-comment-face font-lock-keyword-face) #("name: sample\ninclude: src\ndepend: base\n" 0 5 (face font-lock-keyword-face) 13 21 (face font-lock-keyword-face) 26 33 (face font-lock-keyword-face)) (font-lock-keyword-face font-lock-keyword-face font-lock-keyword-face))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}
