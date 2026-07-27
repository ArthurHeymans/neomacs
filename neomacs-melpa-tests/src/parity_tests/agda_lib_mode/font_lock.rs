use expect_test::expect;

use super::assert_agda_lib_mode_parity;

#[test]
fn agda_lib_mode_fontifies_a_complete_practical_library_document_with_exact_spans() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name: standard-library\n"
          "include: src\n"
          "         experimental\n"
          "depend: base\n"
          "flags: --safe --without-K\n"
          "-- whole-line comment\n"
          "include: test -- trailing explanation\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (let ((position
                (point-min))
               runs)
           (while
               (< position
                  (point-max))
             (let* ((face
                     (get-text-property
                      position
                      'face))
                    (next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max))))
               (when face
                 (push
                  (list
                   position
                   next
                   (buffer-substring-no-properties
                    position next)
                   face)
                  runs))
               (setq position next)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK ((1 6 "name:" font-lock-keyword-face) (24 32 "include:" font-lock-keyword-face) (59 66 "depend:" font-lock-keyword-face) (72 78 "flags:" font-lock-keyword-face) (98 119 "-- whole-line comment" font-lock-comment-face) (120 128 "include:" font-lock-keyword-face) (133 157 " -- trailing explanation" font-lock-comment-face))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_field_fontification_covers_empty_values_punctuation_and_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name: value\n"
          "include:\n"
          "Upper-Field: value\n"
          "a:b:c rest\n"
          "with space: value\n"
          " leading: value\n"
          ": value\n"
          "\tfield: value\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (let ((position
                (point-min))
               runs)
           (while
               (< position
                  (point-max))
             (let* ((face
                     (get-text-property
                      position
                      'face))
                    (next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max))))
               (when face
                 (push
                  (list
                   (line-number-at-pos
                    position)
                   (save-excursion
                     (goto-char position)
                     (current-column))
                   (buffer-substring-no-properties
                    position next)
                   face)
                  runs))
               (setq position next)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK ((1 0 "name:" font-lock-keyword-face) (2 0 "include:\nUpper-Field:" font-lock-keyword-face) (4 0 "a:b:" font-lock-keyword-face) (8 0 "\11field:" font-lock-keyword-face))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_comment_fontification_obeys_space_and_line_start_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "-- comment\n"
          "--  double-space\n"
          "--compact\n"
          "value -- trailing\n"
          "value--joined\n"
          "  -- indented\n"
          "\t-- tab-indented\n"
          "value \t -- mixed\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (let ((position
                (point-min))
               runs)
           (while
               (< position
                  (point-max))
             (let* ((face
                     (get-text-property
                      position
                      'face))
                    (next
                     (next-single-property-change
                      position
                      'face
                      nil
                      (point-max))))
               (when face
                 (push
                  (list
                   (line-number-at-pos
                    position)
                   (save-excursion
                     (goto-char position)
                     (current-column))
                   (buffer-substring-no-properties
                    position next)
                   face)
                  runs))
               (setq position next)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK ((1 0 "-- comment" font-lock-comment-face) (2 0 "--  double-space" font-lock-comment-face) (4 5 " -- trailing" font-lock-comment-face) (6 1 " -- indented" font-lock-comment-face) (8 8 " -- mixed" font-lock-comment-face))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_font_lock_precedence_keeps_field_and_trailing_comment_faces_distinct() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "flags: --safe -- trailing\n"
          "name: value -- explanation\n"
          "-- comment: still comment\n"
          "comment: -- begins here\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward needle)
            (let ((position
                   (match-beginning 0)))
              (list
               needle
               position
               (get-text-property
                position
                'face)
               (get-text-property
                (1- (match-end 0))
                'face))))
          '("flags:"
            "--safe"
            "-- trailing"
            "name:"
            "-- explanation"
            "comment:"
            "still comment"
            "begins here")))"##;
    let expect = expect![[
        r#"OK (("flags:" 1 font-lock-keyword-face font-lock-keyword-face) ("--safe" 8 nil nil) ("-- trailing" 15 font-lock-comment-face font-lock-comment-face) ("name:" 27 font-lock-keyword-face font-lock-keyword-face) ("-- explanation" 39 font-lock-comment-face font-lock-comment-face) ("comment:" 57 font-lock-comment-face font-lock-comment-face) ("still comment" 66 font-lock-comment-face font-lock-comment-face) ("begins here" 92 font-lock-comment-face font-lock-comment-face))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_incremental_edits_recompute_field_and_comment_faces() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name value\ninclude: src\nvalue--note\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (let ((before
                (mapcar
                 (lambda (position)
                   (get-text-property
                    position
                    'face))
                 '(1 12 30))))
           (goto-char
            (point-min))
           (search-forward
            "name")
           (insert ":")
           (search-forward
            "value--")
           (backward-char 2)
           (insert " ")
           (forward-char 2)
           (insert " ")
           (font-lock-flush)
           (font-lock-ensure)
           (list
            before
            (buffer-string)
            (mapcar
             (lambda (needle)
               (goto-char
                (point-min))
               (search-forward needle)
               (list
                needle
                (get-text-property
                 (match-beginning 0)
                 'face)
                (get-text-property
                 (1- (match-end 0))
                 'face)))
             '("name:"
               "include:"
               "-- note")))))"##;
    let expect = expect![[
        r#"OK ((nil font-lock-keyword-face nil) #("name: value\ninclude: src\nvalue -- note\n" 0 4 (face font-lock-keyword-face) 4 5 (face font-lock-keyword-face) 12 20 (face font-lock-keyword-face) 30 31 (face font-lock-comment-face) 31 33 (face font-lock-comment-face) 33 34 (face font-lock-comment-face) 34 38 (face font-lock-comment-face)) (("name:" font-lock-keyword-face font-lock-keyword-face) ("include:" font-lock-keyword-face font-lock-keyword-face) ("-- note" font-lock-comment-face font-lock-comment-face)))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_buffer_local_keyword_override_drives_real_refontification() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "name: value\ncustom-token other\n-- comment\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (let ((before
                (mapcar
                 (lambda (needle)
                   (goto-char
                    (point-min))
                   (search-forward needle)
                   (get-text-property
                    (match-beginning 0)
                    'face))
                 '("name:"
                   "custom-token"
                   "-- comment"))))
           (setq-local
            agda-lib-font-lock-keywords
            '(("custom-token"
               . font-lock-warning-face)))
           (font-lock-refresh-defaults)
           (font-lock-ensure)
           (list
            before
            (local-variable-p
             'agda-lib-font-lock-keywords)
            (mapcar
             (lambda (needle)
               (goto-char
                (point-min))
               (search-forward needle)
               (get-text-property
                (match-beginning 0)
                'face))
             '("name:"
               "custom-token"
               "-- comment")))))"##;
    let expect = expect![[
        r#"OK ((font-lock-keyword-face nil font-lock-comment-face) t (nil font-lock-warning-face nil))"#
    ]];

    assert_agda_lib_mode_parity(elisp_form, expect);
}

#[test]
fn agda_lib_mode_font_lock_is_case_folded_and_keywords_only_without_syntactic_comments() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "NAME: upper\nname: lower\n-- comment\n")
         (agda-lib-mode)
         (font-lock-ensure)
         (goto-char
          (point-min))
         (search-forward
          "comment")
         (list
          font-lock-keywords-only
          font-lock-keywords-case-fold-search
          (mapcar
           (lambda (position)
             (get-text-property
              position
              'face))
           '(1 13 25))
          (nth 4
               (syntax-ppss))
          (char-syntax ?-)
          (syntax-class
           (syntax-after
            (match-beginning 0)))))"##;
    let expect = expect![
        "OK (t t (font-lock-keyword-face font-lock-keyword-face font-lock-comment-face) nil 95 2)"
    ];

    assert_agda_lib_mode_parity(elisp_form, expect);
}
