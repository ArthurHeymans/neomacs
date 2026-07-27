use expect_test::expect;

use super::assert_artbollocks_mode_parity;

#[test]
fn artbollocks_lexical_illusion_regex_matches_repeated_words_with_punctuation_spacing_case_and_boundaries()
 {
    let elisp_form = r##"(let ((case-fold-search
                t))
         (mapcar
          (lambda (text)
            (if
                (string-match
                 artbollocks-lexical-illusions-regex
                 text)
                (list
                 text
                 :match
                 (match-string 0 text)
                 (match-string 1 text)
                 (match-string 2 text)
                 (match-beginning 0)
                 (match-end 0))
              (list
               text
               :no-match)))
          '("the the"
            "The, THE"
            "work—work"
            "α α"
            "art artful"
            "the theme"
            "one\none"
            "_name_ _name_"
            "word word word")))"##;
    let expect = expect![[
        r#"OK (("the the" :match "the the" "the" "the" 0 7) ("The, THE" :match "The, THE" "The" "THE" 0 8) ("work—work" :match "work—work" "work" "work" 0 9) ("α α" :match "α α" "α" "α" 0 3) ("art artful" :no-match) ("the theme" :no-match) ("one\none" :match "one\none" "one" "one" 0 7) ("_name_ _name_" :match "name_ _name" "name" "name" 1 12) ("word word word" :match "word word" "word" "word" 0 9))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_lexical_illusion_regex_respects_case_fold_setting_and_returns_second_word_capture() {
    let elisp_form = r##"(mapcar
         (lambda (fold)
           (let ((case-fold-search
                  fold)
                 (text
                  "Alpha alpha ALPHA"))
             (list
              fold
              (when
                  (string-match
                   artbollocks-lexical-illusions-regex
                   text)
                (list
                 (match-string 0 text)
                 (match-string 1 text)
                 (match-string 2 text)
                 (match-data))))))
         '(nil t))"##;
    let expect = expect![[r#"OK ((nil nil) (t ("Alpha alpha" "Alpha" "alpha" (0 11 0 5 6 11))))"#]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_passive_voice_regex_matches_auxiliary_participle_pairs_and_rejects_active_lookalikes()
 {
    let elisp_form = r##"(let ((regex
                (artbollocks-passive-voice-regex))
               (case-fold-search
                t))
         (list
          (secure-hash
           'sha256
           regex)
          (mapcar
           (lambda (text)
             (if
                 (string-match
                  regex
                  text)
                 (list
                  text
                  :match
                  (match-string 0 text)
                  (match-string 1 text)
                  (match-string 2 text)
                  (match-data))
               (list
                text
                :no-match)))
           '("The work was completed."
             "The pieces are broken."
             "It IS KNOWN."
             "They were being watched."
             "It has been written."
             "It was read."
             "We completed the work."
             "The broken work remains."
             "This is readable."
             "wasnot completed"
             "was completedly"))))"##;
    let expect = expect![[
        r#"OK ("62a5073eb869fc1c9e1c85dd1bde084c817f5e44f477cac4895cccb92d1fe238" (("The work was completed." :no-match) ("The pieces are broken." :match "are broken" "are" "broken" (11 21 11 14 15 21)) ("It IS KNOWN." :match "IS KNOWN" "IS" "KNOWN" (3 11 3 5 6 11)) ("They were being watched." :no-match) ("It has been written." :match "been written" "been" "written" (7 19 7 11 12 19)) ("It was read." :match "was read" "was" "read" (3 11 3 6 7 11)) ("We completed the work." :no-match) ("The broken work remains." :no-match) ("This is readable." :no-match) ("wasnot completed" :no-match) ("was completedly" :no-match)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_passive_voice_regex_tracks_custom_word_order_regex_entries_and_empty_dictionary() {
    let elisp_form = r##"(mapcar
         (lambda (words)
           (let* ((artbollocks-passive-voice-words
                   words)
                  (regex
                   (artbollocks-passive-voice-regex))
                  (case-fold-search
                   t)
                  (texts
                   '("is crafted"
                     "was hand-made"
                     "were α"
                     "is completed")))
             (list
              words
              regex
              (mapcar
               (lambda (text)
                 (and
                  (string-match
                   regex
                   text)
                  (list
                   (match-string 0 text)
                   (match-string 2 text))))
               texts))))
         '(("crafted")
           ("hand-made"
            "crafted")
           ("\\(?:α\\|β\\)")
           ()
           ("\\w+ed")))"##;
    let expect = expect![[
        r#"OK ((("crafted") "\\b\\(am\\|are\\|were\\|being\\|is\\|been\\|was\\|be\\)\\s-+\\(\\(?:crafted\\)\\)\\b" (("is crafted" "crafted") nil nil nil)) (("hand-made" "crafted") "\\b\\(am\\|are\\|were\\|being\\|is\\|been\\|was\\|be\\)\\s-+\\(\\(?:crafted\\|hand-made\\)\\)\\b" (("is crafted" "crafted") ("was hand-made" "hand-made") nil nil)) (("\\(?:α\\|β\\)") "\\b\\(am\\|are\\|were\\|being\\|is\\|been\\|was\\|be\\)\\s-+\\(\\(?:\\\\(\\?:α\\\\|β\\\\)\\)\\)\\b" (nil nil nil nil)) (nil "\\b\\(am\\|are\\|were\\|being\\|is\\|been\\|was\\|be\\)\\s-+\\(\\(?:\\`a\\`\\)\\)\\b" (nil nil nil nil)) (("\\w+ed") "\\b\\(am\\|are\\|were\\|being\\|is\\|been\\|was\\|be\\)\\s-+\\(\\(?:\\\\w\\+ed\\)\\)\\b" (nil nil nil nil)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_weasel_words_regex_matches_default_single_phrase_case_and_word_boundaries() {
    let elisp_form = r##"(let ((regex
                (artbollocks-weasel-words-regex))
               (case-fold-search
                t))
         (list
          (secure-hash
           'sha256
           regex)
          (mapcar
           (lambda (text)
             (if
                 (string-match
                  regex
                  text)
                 (list
                  text
                  :match
                  (match-string 0 text)
                  (match-string 1 text)
                  (match-data))
               (list
                text
                :no-match)))
           '("Many critics agree."
             "There are a number of readings."
             "This is a number of examples."
             "It is VERY clear."
             "completely finished"
             "notmany"
             "variously described"
             "few"
             "a fewish detail"))))"##;
    let expect = expect![[
        r#"OK ("f2b7a3bbd58114f97ab229c00585f7f509951f173c2176b014de622eec27febe" (("Many critics agree." :match "Many" "Many" (0 4 0 4)) ("There are a number of readings." :no-match) ("This is a number of examples." :no-match) ("It is VERY clear." :match "VERY" "VERY" (6 10 6 10)) ("completely finished" :match "completely" "completely" (0 10 0 10)) ("notmany" :no-match) ("variously described" :no-match) ("few" :match "few" "few" (0 3 0 3)) ("a fewish detail" :no-match)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_weasel_words_regex_tracks_custom_literal_regex_unicode_and_empty_lists() {
    let elisp_form = r##"(mapcar
         (lambda (words)
           (let* ((artbollocks-weasel-words-list
                   words)
                  (regex
                   (artbollocks-weasel-words-regex))
                  (case-fold-search
                   t))
             (list
              words
              regex
              (mapcar
               (lambda (text)
                 (and
                  (string-match
                   regex
                   text)
                  (match-string 0 text)))
               '("possibly"
                 "may or may not"
                 "σχεδόν"
                 "very")))))
         '(("possibly")
           ("may\\(?: or may not\\)?")
           ("σχεδόν")
           ()
           ("very"
            "possibly")))"##;
    let expect = expect![[
        r#"OK ((("possibly") "\\b\\(\\(?:possibly\\)\\)\\b" ("possibly" nil nil nil)) (("may\\(?: or may not\\)?") "\\b\\(\\(?:may\\\\(\\?: or may not\\\\)\\?\\)\\)\\b" (nil nil nil nil)) (("σχεδόν") "\\b\\(\\(?:σχεδόν\\)\\)\\b" (nil nil "σχεδόν" nil)) (nil "\\b\\(\\(?:\\`a\\`\\)\\)\\b" (nil nil nil nil)) (("very" "possibly") "\\b\\(\\(?:\\(?:possibl\\|ver\\)y\\)\\)\\b" ("possibly" nil nil "very")))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_jargon_regex_matches_multiword_hyphen_unicode_case_and_overlapping_vocabulary() {
    let elisp_form = r##"(let ((regex
                (artbollocks-jargon-regex))
               (case-fold-search
                t))
         (list
          (secure-hash
           'sha256
           regex)
          (mapcar
           (lambda (text)
             (if
                 (string-match
                  regex
                  text)
                 (list
                  text
                  :match
                  (match-string 0 text)
                  (match-string 1 text)
                  (match-data))
               (list
                text
                :no-match)))
           '("A priori assumptions."
             "The death of the author unfolds."
             "A mise en abyme appears."
             "POST-INTERNET practice"
             "zižekian critique"
             "ZIZEKIAN critique"
             "simulationism"
             "simulationisms"
             "working"
             "work"
             "contextualization"
             "contextualizations"))))"##;
    let expect = expect![[
        r#"OK ("25ee29bab71152524be42bd1b036d39431a102b211fd639eece06527387edbb2" (("A priori assumptions." :match "A priori" "A priori" (0 8 0 8)) ("The death of the author unfolds." :match "death of the author" "death of the author" (4 23 4 23)) ("A mise en abyme appears." :match "mise en abyme" "mise en abyme" (2 15 2 15)) ("POST-INTERNET practice" :match "POST-INTERNET" "POST-INTERNET" (0 13 0 13)) ("zižekian critique" :match "zižekian" "zižekian" (0 8 0 8)) ("ZIZEKIAN critique" :match "ZIZEKIAN" "ZIZEKIAN" (0 8 0 8)) ("simulationism" :match "simulationism" "simulationism" (0 13 0 13)) ("simulationisms" :no-match) ("working" :no-match) ("work" :match "work" "work" (0 4 0 4)) ("contextualization" :match "contextualization" "contextualization" (0 17 0 17)) ("contextualizations" :no-match)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_jargon_regex_tracks_custom_order_regex_metacharacters_and_empty_dictionary() {
    let elisp_form = r##"(mapcar
         (lambda (words)
           (let* ((artbollocks-jargon-words
                   words)
                  (regex
                   (artbollocks-jargon-regex))
                  (case-fold-search
                   t))
             (list
              words
              regex
              (mapcar
               (lambda (text)
                 (and
                  (string-match
                   regex
                   text)
                  (list
                   (match-string 0 text)
                   (match-string 1 text))))
               '("meta"
                 "meta-critical"
                 "a+b"
                 "λ discourse")))))
         '(("meta")
           ("meta-critical"
            "meta")
           ("a\\+b")
           ("λ discourse")
           ()))"##;
    let expect = expect![[
        r#"OK ((("meta") "\\b\\(\\(?:meta\\)\\)\\b" (("meta" "meta") ("meta" "meta") nil nil)) (("meta-critical" "meta") "\\b\\(\\(?:meta\\(?:-critical\\)?\\)\\)\\b" (("meta" "meta") ("meta-critical" "meta-critical") nil nil)) (("a\\+b") "\\b\\(\\(?:a\\\\\\+b\\)\\)\\b" (nil nil nil nil)) (("λ discourse") "\\b\\(\\(?:λ discourse\\)\\)\\b" (nil nil nil ("λ discourse" "λ discourse"))) (nil "\\b\\(\\(?:\\`a\\`\\)\\)\\b" (nil nil nil nil)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}
