use expect_test::expect;

use super::assert_artbollocks_mode_parity;

#[test]
fn artbollocks_inside_code_predicate_distinguishes_lisp_code_comments_strings_and_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(setq active \"very contextual\")\n"
          ";; many works are completed\n"
          "(message \"a priori work\")\n"
          "#| fairly hidden discourse |#\n")
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (goto-char
             (point-min))
            (search-forward
             needle)
            (let ((inside
                   (1-
                    (point)))
                  (after
                   (point)))
              (list
               needle
               inside
               (syntax-ppss inside)
               (artbollocks-inside-code-p
                inside)
               after
               (artbollocks-inside-code-p
                after))))
          '("setq"
            "active"
            "very"
            "many"
            "message"
            "a priori"
            "fairly")))"##;
    let expect = expect![[
        r#"OK (("setq" 5 (1 1 2 nil nil nil 0 nil nil (1) nil) t 6 t) ("active" 12 (1 1 7 nil nil nil 0 nil nil (1) nil) t 13 t) ("very" 18 (1 1 7 34 nil nil 0 nil 14 (1) nil) nil 19 nil) ("many" 39 (0 nil 1 nil t nil 0 nil 33 nil nil) nil 40 nil) ("message" 68 (1 61 62 nil nil nil 0 nil nil (61) nil) t 69 t) ("a priori" 78 (1 61 62 34 nil nil 0 nil 70 (61) nil) nil 79 nil) ("fairly" 95 (0 nil 90 nil nil nil 0 nil nil nil nil) t 96 t))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_generic_search_skips_code_and_finds_case_insensitive_matches_in_comments_and_strings()
 {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(setq many 1)\n"
          ";; MANY critics find many works\n"
          "(message \"many readings\")\n"
          "(let ((many 2)) many)\n")
         (artbollocks-test-match
          (lambda (limit)
            (artbollocks-search-for-keyword
             "\\bmany\\b"
             limit))))"##;
    let expect =
        expect![[r#"OK (("MANY" nil 18 22 22) ("many" nil 36 40 40) ("many" nil 57 61 61))"#]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_generic_search_preserves_winning_match_data_advances_point_and_skips_earlier_code_hits()
 {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(context code)\n"
          ";; contextual context\n")
         (goto-char
          (point-min))
         (string-match
          "\\(seed\\)"
          "seed")
         (let ((before-match
                (match-data))
               (before-point
                (point))
               (result
                (artbollocks-search-for-keyword
                 "\\b\\(context\\(?:ual\\)?\\)\\b"
                 (point-max))))
           (list
            before-point
            before-match
            result
            (match-string-no-properties
             0)
            (match-string-no-properties
             1)
            (match-data)
            (point)
            (char-after)
            (buffer-substring-no-properties
             (line-beginning-position)
             (line-end-position)))))"##;
    let expect = expect![[
        r#"OK (1 (0 4 0 4) t "contextual" "contextual" ((:marker nil nil) (:marker nil nil) (:marker nil nil) (:marker nil nil)) 29 32 ";; contextual context")"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_generic_search_honors_limit_and_no_match_point_and_match_data_contracts() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; alpha target\n"
          ";; beta target\n")
         (let ((limit
                (progn
                  (goto-char
                   (point-min))
                  (forward-line 1)
                  (point))))
           (mapcar
            (lambda (start)
              (goto-char start)
              (string-match
               "\\(seed\\)"
               "seed")
              (let ((before
                     (match-data)))
                (list
                 start
                 limit
                 (condition-case error-data
                     (list
                      :ok
                      (artbollocks-search-for-keyword
                       "\\btarget\\b"
                       limit))
                   (error
                    (list
                     :error
                     (car error-data)
                     (cdr error-data))))
                 (point)
                 before
                 (match-data))))
            (list
             (point-min)
             limit
             (point-max)))))"##;
    let expect = expect![[
        r#"OK ((1 17 (:ok t) 16 (0 4 0 4) ((:marker nil nil) (:marker nil nil))) (17 17 (:ok nil) 17 (0 4 0 4) (0 4 0 4)) (32 17 (:error error ("Invalid search bound (wrong side of point)")) 32 (0 4 0 4) (0 4 0 4)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_search_wrappers_find_real_lexical_passive_weasel_and_jargon_comment_matches() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; the the work was completed in a very contextual discourse\n")
         (mapcar
          (lambda (function)
            (list
             function
             (artbollocks-test-match
              function)))
          '(artbollocks-lexical-illusions-search-for-keyword
            artbollocks-passive-voice-search-for-keyword
            artbollocks-weasel-words-search-for-keyword
            artbollocks-search-for-jargon)))"##;
    let expect = expect![[
        r#"OK ((artbollocks-lexical-illusions-search-for-keyword (("the the" ("the" "the") 4 11 11))) (artbollocks-passive-voice-search-for-keyword nil) (artbollocks-weasel-words-search-for-keyword (("very" ("very") 36 40 40))) (artbollocks-search-for-jargon (("work" ("work") 12 16 16) ("contextual" ("contextual") 41 51 51) ("discourse" ("discourse") 52 61 61))))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_search_wrappers_use_current_dynamic_custom_dictionaries_on_every_call() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; was crafted, perhaps meta-critical and very\n")
         (let ((artbollocks-passive-voice-words
                '("crafted"))
               (artbollocks-weasel-words-list
                '("perhaps"))
               (artbollocks-jargon-words
                '("meta-critical")))
           (mapcar
            (lambda (function)
              (list
               function
               (artbollocks-test-match
                function)))
            '(artbollocks-passive-voice-search-for-keyword
              artbollocks-weasel-words-search-for-keyword
              artbollocks-search-for-jargon))))"##;
    let expect = expect![[
        r#"OK ((artbollocks-passive-voice-search-for-keyword (("was crafted" ("was" "crafted") 4 15 15))) (artbollocks-weasel-words-search-for-keyword (("perhaps" ("perhaps") 17 24 24))) (artbollocks-search-for-jargon (("meta-critical" ("meta-critical") 25 38 38))))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_search_in_text_mode_skips_plain_prose_but_finds_quoted_text_with_string_syntax() {
    let elisp_form = r##"(mapcar
         (lambda (mode)
           (with-temp-buffer
             (funcall mode)
             (insert
              "many contextual works were completed")
             (list
              mode
              (artbollocks-test-match
               'artbollocks-weasel-words-search-for-keyword)
              (artbollocks-test-match
               'artbollocks-passive-voice-search-for-keyword)
              (artbollocks-test-match
               'artbollocks-search-for-jargon))))
         '(text-mode
           fundamental-mode
           emacs-lisp-mode))"##;
    let expect = expect![
        "OK ((text-mode nil nil nil) (fundamental-mode nil nil nil) (emacs-lisp-mode nil nil nil))"
    ];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_search_respects_narrowing_and_never_returns_matches_outside_accessible_region() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; many before\n"
          ";; very inside one\n"
          ";; very inside two\n"
          ";; many after\n")
         (let ((start
                (progn
                  (goto-char
                   (point-min))
                  (forward-line 1)
                  (point)))
               (end
                (progn
                  (forward-line 2)
                  (point))))
           (narrow-to-region
            start
            end)
           (list
            (point-min)
            (point-max)
            (buffer-string)
            (artbollocks-test-match
             'artbollocks-weasel-words-search-for-keyword)
            (point-min)
            (point-max))))"##;
    let expect = expect![[
        r#"OK (16 54 ";; very inside one\n;; very inside two\n" (("very" ("very") 19 23 23) ("very" ("very") 38 42 42)) 16 54)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_search_handles_multiline_comments_strings_and_unicode_without_losing_match_boundaries()
 {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "#| opening\n"
          "   a priori λ discourse\n"
          "   was written very clearly\n"
          "|#\n"
          "(message \"mise en abyme\n"
          "many works\")\n")
         (mapcar
          (lambda (function)
            (list
             function
             (artbollocks-test-match
              function)))
          '(artbollocks-passive-voice-search-for-keyword
            artbollocks-weasel-words-search-for-keyword
            artbollocks-search-for-jargon)))"##;
    let expect = expect![[
        r#"OK ((artbollocks-passive-voice-search-for-keyword nil) (artbollocks-weasel-words-search-for-keyword (("many" ("many") 91 95 95))) (artbollocks-search-for-jargon (("mise en abyme" ("mise en abyme") 77 90 90) ("works" ("works") 96 101 101))))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}
