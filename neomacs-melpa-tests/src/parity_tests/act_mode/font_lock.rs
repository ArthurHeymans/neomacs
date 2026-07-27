use expect_test::expect;

use super::assert_act_mode_parity;

#[test]
fn act_mode_font_lock_highlights_every_keyword_function_type_constant_and_comment_category() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "export import\n")
         (insert
          "defproc deftype defchan prs\n")
         (insert
          "preal pint bool int e1of e2of e3of c1of globals globals_np\n")
         (insert
          "<0> <42>\n")
         (insert
          "// export defproc int <7> comment\n")
         (act-mode)
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
                    position
                    next)
                   face)
                  runs))
               (setq position next)))
           (nreverse runs)))"##;
    let expect = expect![[
        r#"OK ((1 7 "export" font-lock-keyword-face) (8 14 "import" font-lock-keyword-face) (15 22 "defproc" font-lock-function-name-face) (23 30 "deftype" font-lock-function-name-face) (31 38 "defchan" font-lock-function-name-face) (39 42 "prs" font-lock-function-name-face) (43 48 "preal" font-lock-type-face) (49 53 "pint" font-lock-type-face) (54 58 "bool" font-lock-type-face) (59 62 "int" font-lock-type-face) (63 67 "e1of" font-lock-type-face) (68 72 "e2of" font-lock-type-face) (73 77 "e3of" font-lock-type-face) (78 82 "c1of" font-lock-type-face) (83 90 "globals" font-lock-type-face) (91 101 "globals_np" font-lock-type-face) (102 105 "<0>" font-lock-constant-face) (106 110 "<42>" font-lock-constant-face) (111 144 "// export defproc int <7> comment" font-lock-comment-face))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_font_lock_boundary_and_malformed_inputs_match_exact_partial_highlights() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "exported ximport import-x _int int2 globals_npx\n")
         (insert
          "<> <-1> < 2> <2a> <<3>> <003>\n")
         (act-mode)
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
                   (buffer-substring-no-properties
                    position
                    next)
                   face)
                  runs))
               (setq position next)))
           (list
            (nreverse runs)
            (buffer-substring-no-properties
             (point-min)
             (point-max)))))"##;
    let expect = expect![[
        r#"OK ((("import" font-lock-keyword-face) ("int" font-lock-type-face) ("globals" font-lock-type-face) ("<3>" font-lock-constant-face) ("<003>" font-lock-constant-face)) "exported ximport import-x _int int2 globals_npx\n<> <-1> < 2> <2a> <<3>> <003>\n")"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_font_lock_is_case_sensitive_for_every_named_token_category() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "export EXPORT Export import IMPORT Import\n")
         (insert
          "defproc DEFPROC Defproc prs PRS Prs\n")
         (insert
          "preal PREAL Preal globals_np GLOBALS_NP Globals_np\n")
         (act-mode)
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
                   (buffer-substring-no-properties
                    position
                    next)
                   face)
                  runs))
               (setq position next)))
           (list
            case-fold-search
            font-lock-keywords-case-fold-search
            (nreverse runs))))"##;
    let expect = expect![[
        r#"OK (t nil (("export" font-lock-keyword-face) ("import" font-lock-keyword-face) ("defproc" font-lock-function-name-face) ("prs" font-lock-function-name-face) ("preal" font-lock-type-face) ("globals_np" font-lock-type-face)))"#
    ]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_comment_fontification_does_not_change_prog_mode_syntax_parsing() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "int x; // comment\n")
         (act-mode)
         (font-lock-ensure)
         (goto-char
          (point-min))
         (search-forward
          "comment")
         (list
          (get-text-property
           (match-beginning 0)
           'face)
          (nth 4
               (syntax-ppss))
          comment-start
          comment-end
          (char-syntax
           ?/)
          (syntax-class
           (syntax-after
            (-
             (match-beginning 0)
             3)))))"##;
    let expect = expect![[r#"OK (font-lock-comment-face nil nil "" 95 3)"#]];
    assert_act_mode_parity(elisp_form, expect);
}

#[test]
fn act_mode_fontlock_registry_is_load_time_snapshot_independent_of_later_word_list_mutation() {
    let elisp_form = r##"(let ((original-keywords
                act-keywords)
               (original-fontlock
                act-fontlock))
         (unwind-protect
             (progn
               (setq act-keywords
                     '("changed"))
               (with-temp-buffer
                 (insert
                  "export changed")
                 (act-mode)
                 (font-lock-ensure)
                 (list
                  (eq act-fontlock
                      original-fontlock)
                  (get-text-property
                   1
                   'face)
                  (get-text-property
                   8
                   'face))))
           (setq act-keywords
                 original-keywords)))"##;
    let expect = expect!["OK (t font-lock-keyword-face nil)"];
    assert_act_mode_parity(elisp_form, expect);
}
