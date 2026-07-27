use expect_test::expect;

use super::assert_artbollocks_mode_parity;

#[test]
fn artbollocks_add_keywords_registers_exact_enabled_feature_tables_in_source_order() {
    let elisp_form = r##"(mapcar
         (lambda (flags)
           (let ((artbollocks-lexical-illusions
                  (nth 0 flags))
                 (artbollocks-passive-voice
                  (nth 1 flags))
                 (artbollocks-weasel-words
                  (nth 2 flags))
                 (artbollocks-jargon
                  (nth 3 flags))
                 calls)
             (cl-letf
                 (((symbol-function
                    'font-lock-add-keywords)
                   (lambda (major-mode keywords &optional how)
                     (push
                      (list
                       major-mode
                       keywords
                       how)
                      calls)
                     :added)))
               (list
                flags
                (artbollocks-add-keywords)
                (nreverse calls)))))
         '((t t t t)
           (t nil nil nil)
           (nil t nil t)
           (nil nil t nil)
           (nil nil nil nil)))"##;
    let expect = expect![
        "OK (((t t t t) :added ((nil #1=((artbollocks-lexical-illusions-search-for-keyword (2 'artbollocks-lexical-illusions-face t))) nil) (nil #2=((artbollocks-passive-voice-search-for-keyword (0 'artbollocks-passive-voice-face t))) nil) (nil #4=((artbollocks-weasel-words-search-for-keyword (0 'artbollocks-weasel-words-face t))) nil) (nil #3=((artbollocks-search-for-jargon (0 'artbollocks-face t))) nil))) ((t nil nil nil) nil ((nil #1# nil))) ((nil t nil t) :added ((nil #2# nil) (nil #3# nil))) ((nil nil t nil) nil ((nil #4# nil))) ((nil nil nil nil) nil nil))"
    ];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_remove_keywords_unregisters_every_table_regardless_of_enabled_feature_flags() {
    let elisp_form = r##"(let ((artbollocks-lexical-illusions
                nil)
               (artbollocks-passive-voice
                nil)
               (artbollocks-weasel-words
                nil)
               (artbollocks-jargon
                nil)
               calls)
         (cl-letf
             (((symbol-function
                'font-lock-remove-keywords)
               (lambda (major-mode keywords)
                 (push
                  (list
                   major-mode
                   keywords)
                  calls)
                 :removed)))
           (list
            (artbollocks-remove-keywords)
            (nreverse calls))))"##;
    let expect = expect![
        "OK (:removed ((nil ((artbollocks-lexical-illusions-search-for-keyword (2 'artbollocks-lexical-illusions-face t)))) (nil ((artbollocks-passive-voice-search-for-keyword (0 'artbollocks-passive-voice-face t)))) (nil ((artbollocks-weasel-words-search-for-keyword (0 'artbollocks-weasel-words-face t)))) (nil ((artbollocks-search-for-jargon (0 'artbollocks-face t))))))"
    ];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_real_fontification_highlights_each_category_in_lisp_comments_and_strings_only() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "(setq very contextual)\n"
          ";; the the work was completed in a very contextual discourse\n"
          "(message \"many works were written with a priori narrative\")\n")
         (artbollocks-mode 1)
         (font-lock-ensure
          (point-min)
          (point-max))
         (list
          artbollocks-mode
          (artbollocks-test-face-runs)
          (buffer-substring-no-properties
           (point-min)
           (point-max))
          (text-property-not-all
           (point-min)
           (point-max)
           'face
           nil)))"##;
    let expect = expect![[
        r#"OK (t (("the" artbollocks-lexical-illusions-face) ("work" artbollocks-face) ("very" artbollocks-weasel-words-face) ("contextual" artbollocks-face) ("discourse" artbollocks-face) ("many" artbollocks-weasel-words-face) ("works" artbollocks-face) ("were written" artbollocks-passive-voice-face) ("a priori" artbollocks-face) ("narrative" artbollocks-face)) "(setq very contextual)\n;; the the work was completed in a very contextual discourse\n(message \"many works were written with a priori narrative\")\n" 2)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_real_fontification_lexical_illusion_marks_only_repeated_capture_not_first_word() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; the the, Work—WORK, alpha beta\n")
         (let ((artbollocks-lexical-illusions
                t)
               (artbollocks-passive-voice
                nil)
               (artbollocks-weasel-words
                nil)
               (artbollocks-jargon
                nil))
           (artbollocks-mode 1)
           (font-lock-ensure)
           (list
            (artbollocks-test-face-runs)
            (mapcar
             (lambda (needle)
               (goto-char
                (point-min))
               (search-forward
                needle)
               (list
                needle
                (get-text-property
                 (1-
                  (point))
                 'face)))
             '("the"
               "the,"
               "Work"
               "WORK")))))"##;
    let expect = expect![[
        r#"OK ((("the" artbollocks-lexical-illusions-face) ("WORK" artbollocks-lexical-illusions-face)) (("the" font-lock-comment-face) ("the," font-lock-comment-face) ("Work" font-lock-comment-face) ("WORK" font-lock-comment-face)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_real_fontification_selective_flags_enable_only_requested_visual_categories() {
    let elisp_form = r##"(mapcar
         (lambda (flags)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              ";; the the work was completed in a very contextual discourse\n")
             (let ((artbollocks-lexical-illusions
                    (nth 0 flags))
                   (artbollocks-passive-voice
                    (nth 1 flags))
                   (artbollocks-weasel-words
                    (nth 2 flags))
                   (artbollocks-jargon
                    (nth 3 flags)))
               (artbollocks-mode 1)
               (font-lock-ensure)
               (list
                flags
                (artbollocks-test-face-runs)))))
         '((t nil nil nil)
           (nil t nil nil)
           (nil nil t nil)
           (nil nil nil t)
           (nil nil nil nil)))"##;
    let expect = expect![[
        r#"OK (((t nil nil nil) (("the" artbollocks-lexical-illusions-face))) ((nil t nil nil) nil) ((nil nil t nil) (("very" artbollocks-weasel-words-face))) ((nil nil nil t) (("work" artbollocks-face) ("contextual" artbollocks-face) ("discourse" artbollocks-face))) ((nil nil nil nil) nil))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_real_fontification_is_case_insensitive_and_handles_multiline_comment_unicode_jargon()
{
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          "#| MANY WORKS WERE WRITTEN.\n"
          "   A PRIORI zižekian DISCOURSE.\n"
          "   THE the.\n"
          "|#\n")
         (artbollocks-mode 1)
         (font-lock-ensure)
         (list
          (artbollocks-test-face-runs)
          (buffer-string)))"##;
    let expect = expect![[
        r##"OK (nil "#| MANY WORKS WERE WRITTEN.\n   A PRIORI zižekian DISCOURSE.\n   THE the.\n|#\n")"##
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_disable_and_refontification_remove_package_faces_without_changing_text() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; many works were completed in contextual discourse\n")
         (let ((before
                (buffer-string)))
           (artbollocks-mode 1)
           (font-lock-ensure)
           (let ((enabled
                  (artbollocks-test-face-runs)))
             (artbollocks-mode -1)
             (font-lock-ensure)
             (list
              before
              enabled
              artbollocks-mode
              (artbollocks-test-face-runs)
              (buffer-string)
              (buffer-modified-p)))))"##;
    let expect = expect![[
        r#"OK (";; many works were completed in contextual discourse\n" (("many" artbollocks-weasel-words-face) ("works" artbollocks-face) ("contextual" artbollocks-face) ("discourse" artbollocks-face)) nil nil #(";; many works were completed in contextual discourse\n" 0 3 (face font-lock-comment-delimiter-face) 3 7 (face font-lock-comment-face) 7 8 (face font-lock-comment-face) 8 13 (face font-lock-comment-face) 13 32 (face font-lock-comment-face) 32 42 (face font-lock-comment-face) 42 43 (face font-lock-comment-face) 43 52 (face font-lock-comment-face) 52 53 (face font-lock-comment-face)) t)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_text_mode_real_fontification_preserves_current_source_behavior_of_skipping_plain_prose()
 {
    let elisp_form = r##"(with-temp-buffer
         (text-mode)
         (insert
          "The the work was completed in a very contextual discourse.")
         (artbollocks-mode 1)
         (font-lock-ensure)
         (list
          major-mode
          artbollocks-mode
          (artbollocks-test-face-runs)
          (buffer-string)
          font-lock-keywords))"##;
    let expect = expect![[
        r#"OK (text-mode t nil "The the work was completed in a very contextual discourse." (t (#1=(artbollocks-search-for-jargon (0 'artbollocks-face t)) #2=(artbollocks-weasel-words-search-for-keyword (0 'artbollocks-weasel-words-face t)) #3=(artbollocks-passive-voice-search-for-keyword (0 'artbollocks-passive-voice-face t)) #4=(artbollocks-lexical-illusions-search-for-keyword (2 'artbollocks-lexical-illusions-face t))) #1# #2# #3# #4#))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_dynamic_dictionary_changes_take_effect_after_flush_and_refontification() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert
          ";; perhaps meta-critical work was crafted\n")
         (let ((artbollocks-lexical-illusions
                nil)
               (artbollocks-passive-voice
                t)
               (artbollocks-weasel-words
                t)
               (artbollocks-jargon
                t)
               (artbollocks-passive-voice-words
                '("completed"))
               (artbollocks-weasel-words-list
                '("very"))
               (artbollocks-jargon-words
                '("context")))
           (artbollocks-mode 1)
           (font-lock-ensure)
           (let ((before
                  (artbollocks-test-face-runs)))
             (setq artbollocks-passive-voice-words
                   '("crafted")
                   artbollocks-weasel-words-list
                   '("perhaps")
                   artbollocks-jargon-words
                   '("meta-critical"))
             (font-lock-flush)
             (font-lock-ensure)
             (list
              before
              (artbollocks-test-face-runs)))))"##;
    let expect = expect![[
        r#"OK (nil (("perhaps" artbollocks-weasel-words-face) ("meta-critical" artbollocks-face) ("was crafted" artbollocks-passive-voice-face)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}
