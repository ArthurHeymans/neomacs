use expect_test::expect;

use super::assert_anki_vocabulary_parity;

#[test]
fn youdao_word_search_formats_tagged_and_untagged_explanations_into_selectable_meanings() {
    let elisp_form = r##"(let (requests explain-inputs)
  (cl-letf
      (((symbol-function 'youdao-dictionary--request)
        (lambda (word)
          (push word requests)
          '((query . "practice")
            (translation . ["实践"])
            (basic . ((phonetic . "ˈpræktɪs")))
            (web . [((key . "unused")
                     (value . ["unused web result"]))]))))
       ((symbol-function 'youdao-dictionary--explains)
        (lambda (json)
          (push json explain-inputs)
          '("n. 实践；练习；惯例"
            "v. 实行；反复练习"
            "without part of speech"))))
    (list
     (anki-vocabulary--word-searcher-youdao "practice")
     (nreverse requests)
     (length explain-inputs)
     (caar explain-inputs))))"##;
    let expect = expect![[
        r#"OK (((expression . "practice") (glossary "n. 实践" "n. 练习" "n. 惯例" "v. 实行" "v. 反复练习" "without part of speech") (phonetic . "ˈpræktɪs")) ("practice") 1 (query . "practice"))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn youdao_word_search_uses_structured_web_results_when_basic_explanations_are_absent() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'youdao-dictionary--request)
      (lambda (_word)
        '((query . "deck")
          (translation . ["甲板"])
          (basic . ((us-phonetic . "dɛk")
                    (uk-phonetic . "dek")))
          (web . [((key . "deck building")
                   (value . ["牌组构筑" "甲板建造"]))
                  ((key . "upper deck")
                   (value . ["上层甲板"]))]))))
     ((symbol-function 'youdao-dictionary--explains)
      (lambda (_json) nil)))
  (anki-vocabulary--word-searcher-youdao "deck"))"##;
    let expect = expect![[
        r#"OK ((expression . "deck") (glossary "- deck building :: 牌组构筑; 甲板建造" "- upper deck :: 上层甲板") (phonetic . "dɛk"))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn youdao_word_search_falls_back_to_direct_translations_when_other_glossaries_are_empty() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'youdao-dictionary--request)
      (lambda (_word)
        '((query . "context")
          (translation . ["上下文" "语境"])
          (basic . ((uk-phonetic . "ˈkɒntekst")))
          (web . []))))
     ((symbol-function 'youdao-dictionary--explains)
      (lambda (_json) nil)))
  (anki-vocabulary--word-searcher-youdao "context"))"##;
    let expect = expect![[
        r#"OK ((expression . "context") (glossary "上下文" "语境") (phonetic . "ˈkɒntekst"))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn youdao_glossary_precedence_prefers_explanations_over_web_and_translation_results() {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'youdao-dictionary--request)
      (lambda (_word)
        '((query . "card")
          (translation . ["translation result"])
          (basic . ((phonetic . "primary")))
          (web . [((key . "web key")
                   (value . ["web result"]))]))))
     ((symbol-function 'youdao-dictionary--explains)
      (lambda (_json)
        '("n. explanation result"))))
  (anki-vocabulary--word-searcher-youdao "card"))"##;
    let expect = expect![[
        r#"OK ((expression . "card") (glossary "n. explanation result") (phonetic . "primary"))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn youdao_phonetic_selection_obeys_general_then_us_then_uk_precedence_and_handles_missing_basic_data()
 {
    let elisp_form = r##"(cl-letf
    (((symbol-function 'youdao-dictionary--request)
      (lambda (word)
        (pcase word
          ("all"
           '((query . "all")
             (translation . ["all"])
             (basic . ((phonetic . "general")
                       (us-phonetic . "american")
                       (uk-phonetic . "british")))))
          ("regional"
           '((query . "regional")
             (translation . ["regional"])
             (basic . ((us-phonetic . "american")
                       (uk-phonetic . "british")))))
          ("british"
           '((query . "british")
             (translation . ["british"])
             (basic . ((uk-phonetic . "british")))))
          (_
           '((query . "missing")
             (translation . ["missing"]))))))
     ((symbol-function 'youdao-dictionary--explains)
      (lambda (_json) nil)))
  (mapcar
   (lambda (word)
     (let ((result
            (anki-vocabulary--word-searcher-youdao word)))
       (list word
             (cdr (assq 'phonetic result))
             result)))
   '("all" "regional" "british" "missing")))"##;
    let expect = expect![[
        r#"OK (("all" "general" ((expression . "all") (glossary "all") (phonetic . "general"))) ("regional" "american" ((expression . "regional") (glossary "regional") (phonetic . "american"))) ("british" "british" ((expression . "british") (glossary "british") (phonetic . "british"))) ("missing" nil ((expression . "missing") (glossary "missing") (phonetic))))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn sentence_translation_requests_the_full_real_sentence_and_returns_the_first_candidate() {
    let elisp_form = r##"(let (requests)
  (cl-letf
      (((symbol-function 'youdao-dictionary--request)
        (lambda (sentence)
          (push sentence requests)
          '((translation
             . ["实际的第一条翻译"
                "unused alternative"])
            (query
             . "A practical sentence with punctuation!")))))
    (list
     (anki-vocabulary--sentence-translator-youdao
      "A practical sentence with punctuation!")
     (nreverse requests))))"##;
    let expect = expect![[r#"OK ("实际的第一条翻译" ("A practical sentence with punctuation!"))"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}
