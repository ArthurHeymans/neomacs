use expect_test::expect;

use super::assert_anki_vocabulary_parity;

#[test]
fn complete_note_workflow_translates_selects_glossary_expands_every_template_and_runs_hooks_in_order()
 {
    let elisp_form = r##"(let (events)
  (setq anki-vocabulary-deck-name "Language::English"
        anki-vocabulary-model-name "Vocabulary with Context"
        anki-vocabulary-field-alist
        '(("Front" . "${expression:单词}")
          ("Meaning" . "${glossary:释义}")
          ("Pronunciation" . "/${phonetic:音标}/")
          ("Context" . "${sentence:原文例句}")
          ("Highlighted" . "${sentence_bold:标粗的原文例句}")
          ("Translated context" . "${translation:翻译例句}")
          ("Replay" . "${sound:发声}")
          ("Composite" . "${expression:单词} — ${glossary:释义}"))
        anki-vocabulary-audio-fileds nil
        anki-vocabulary-sentence-translator
        (lambda (sentence)
          (push (list 'translate sentence) events)
          "我们每天练习。")
        anki-vocabulary-word-searcher
        (lambda (word)
          (push (list 'search word) events)
          '((expression . "practice")
            (glossary . ("n. 实践" "v. 练习" "n. 惯例"))
            (phonetic . "ˈpræktɪs")))
        anki-vocabulary-before-addnote-functions
        (list
         (lambda (&rest arguments)
           (push (cons 'before arguments) events)))
        anki-vocabulary-after-addnote-functions
        (list
         (lambda (&rest arguments)
           (push (cons 'after arguments) events))))
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (prompt collection &rest arguments)
          (push (list 'choose prompt collection arguments) events)
          (nth 1 collection)))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (expression)
          (push (list 'voice expression) events)
          "https://audio.example/practice?voice=2"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (&rest arguments)
          (push (cons 'add-note arguments) events)
          4242)))
    (let ((result
           (anki-vocabulary
            "Practice makes progress; practice builds confidence."
            "practice")))
      (list result (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil ((translate "Practice makes progress; practice builds confidence.") (search "practice") (choose "我们每天练习。(practice):" ("n. 实践" "v. 练习" "n. 惯例") nil) (voice "practice") (before "practice" "Practice makes progress; practice builds confidence." "<B>Practice</B> makes progress; <b>practice</b> builds confidence." "我们每天练习。" "v. 练习" "ˈpræktɪs") (add-note "Language::English" "Vocabulary with Context" (("Front" . "practice") ("Meaning" . "v. 练习") ("Pronunciation" . "/ˈpræktɪs/") ("Context" . "Practice makes progress; practice builds confidence.") ("Highlighted" . "<B>Practice</B> makes progress; <b>practice</b> builds confidence.") ("Translated context" . "我们每天练习。") ("Replay" . "[sound:youdao-5044ec878204a12779189fa408c03809.mp3]") ("Composite" . "practice — v. 练习"))) (after "practice" "Practice makes progress; practice builds confidence." "<B>Practice</B> makes progress; <b>practice</b> builds confidence." "我们每天练习。" "v. 练习" "ˈpræktɪs")))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn list_audio_configuration_builds_one_download_contract_for_multiple_model_fields() {
    let elisp_form = r##"(let (add-note-call)
  (setq anki-vocabulary-deck-name "Listening"
        anki-vocabulary-model-name "Audio Vocabulary"
        anki-vocabulary-field-alist
        '(("Word" . "${expression:单词}")
          ("Example" . "${sentence_bold:标粗的原文例句}"))
        anki-vocabulary-audio-fileds
        '("Front audio" "Back audio")
        anki-vocabulary-sentence-translator
        (lambda (_sentence) "一张卡片。")
        anki-vocabulary-word-searcher
        (lambda (_word)
          '((expression . "card")
            (glossary . ("n. card"))
            (phonetic . "kɑːd"))))
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (_prompt collection &rest _arguments)
          (car collection)))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (_expression)
          "https://audio.example/card.mp3"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (&rest arguments)
          (setq add-note-call arguments)
          'created)))
    (list
     (anki-vocabulary "A card can capture one practical idea." "card")
     add-note-call)))"##;
    let expect = expect![[
        r#"OK (nil ("Listening" "Audio Vocabulary" (("Word" . "card") ("Example" . "A <b>card</b> can capture one practical idea.")) (("url" . "https://audio.example/card.mp3") ("filename" . "youdao-b568be3ca5d0a52105b1f8d6aab276c3.mp3") ("fields" . ["Front audio" "Back audio"]))))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn string_audio_configuration_targets_exactly_one_model_field() {
    let elisp_form = r##"(let (add-note-call)
  (setq anki-vocabulary-deck-name "Deck"
        anki-vocabulary-model-name "Model"
        anki-vocabulary-field-alist
        '(("Front" . "${expression:单词}"))
        anki-vocabulary-audio-fileds "Audio"
        anki-vocabulary-sentence-translator
        (lambda (_sentence) "翻译")
        anki-vocabulary-word-searcher
        (lambda (_word)
          '((expression . "workflow")
            (glossary . ("工作流"))
            (phonetic . "ˈwɜːkfləʊ"))))
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (_prompt collection &rest _arguments)
          (car collection)))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (_expression) "voice://workflow"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (&rest arguments)
          (setq add-note-call arguments)
          7)))
    (anki-vocabulary "This workflow is reproducible." "workflow")
    add-note-call))"##;
    let expect = expect![[
        r#"OK ("Deck" "Model" (("Front" . "workflow")) (("url" . "voice://workflow") ("filename" . "youdao-7bba0bb0cf98ac6dbcf742240bbe0341.mp3") ("fields" . ["Audio"])))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn truthy_non_list_audio_configuration_keeps_audio_metadata_but_targets_no_fields() {
    let elisp_form = r##"(let (add-note-call)
  (setq anki-vocabulary-deck-name "Deck"
        anki-vocabulary-model-name "Model"
        anki-vocabulary-field-alist nil
        anki-vocabulary-audio-fileds 'misconfigured
        anki-vocabulary-sentence-translator
        (lambda (_sentence) "翻译")
        anki-vocabulary-word-searcher
        (lambda (_word)
          '((expression . "edge")
            (glossary . ("边缘"))
            (phonetic . "edʒ"))))
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (_prompt collection &rest _arguments)
          (car collection)))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (_expression) "voice://edge"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (&rest arguments)
          (setq add-note-call arguments)
          'created)))
    (anki-vocabulary "An edge case should remain observable." "edge")
    add-note-call))"##;
    let expect = expect![[
        r#"OK ("Deck" "Model" nil (("url" . "voice://edge") ("filename" . "youdao-1ac99c94b36cbe993beaf962bca75c6e.mp3") ("fields" . [])))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn interactive_workflow_acquires_sentence_and_word_then_passes_them_through_the_full_pipeline() {
    let elisp_form = r##"(progn
  (setq anki-vocabulary-test-events nil
        anki-vocabulary-deck-name "Inbox"
        anki-vocabulary-model-name "Basic"
        anki-vocabulary-field-alist
        '(("Front" . "${expression:单词}")
          ("Back" . "${translation:翻译例句} | ${glossary:释义}"))
        anki-vocabulary-audio-fileds nil
        anki-vocabulary-sentence-translator
        (lambda (sentence)
          (push (list 'translated sentence)
                anki-vocabulary-test-events)
          "确定性的测试会揭示差异。")
        anki-vocabulary-word-searcher
        (lambda (word)
          (push (list 'searched word)
                anki-vocabulary-test-events)
          (list
           (cons 'expression word)
           (cons 'glossary '("揭示；显露"))
           (cons 'phonetic "sɜːˈfeɪs"))))
  (cl-letf
      (((symbol-function 'anki-vocabulary--get-text)
        (lambda ()
          (push '(get-text) anki-vocabulary-test-events)
          "Deterministic tests surface meaningful divergences."))
       ((symbol-function 'anki-vocabulary--get-word)
        (lambda ()
          (push '(get-word) anki-vocabulary-test-events)
          "surface"))
       ((symbol-function 'anki-vocabulary--select-word-in-string)
        (lambda (sentence default)
          (push (list 'select sentence default)
                anki-vocabulary-test-events)
          default))
       ((symbol-function 'completing-read)
        (lambda (prompt collection &rest _arguments)
          (push (list 'glossary prompt collection)
                anki-vocabulary-test-events)
          (car collection)))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (expression)
          (push (list 'voice expression)
                anki-vocabulary-test-events)
          "voice://surface"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (&rest arguments)
          (push (cons 'add-note arguments)
                anki-vocabulary-test-events)
          91)))
    (let ((result (anki-vocabulary)))
      (list result
            (nreverse anki-vocabulary-test-events)))))"##;
    let expect = expect![[
        r#"OK (nil ((get-text) (get-word) (select "Deterministic tests surface meaningful divergences." "surface") (translated "Deterministic tests surface meaningful divergences.") (searched "surface") (glossary "确定性的测试会揭示差异。(surface):" ("揭示；显露")) (voice "surface") (add-note "Inbox" "Basic" (("Front" . "surface") ("Back" . "确定性的测试会揭示差异。 | 揭示；显露")))))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn missing_search_fields_become_empty_strings_without_losing_the_original_sentence() {
    let elisp_form = r##"(let (completion add-note-call voice-expression)
  (setq anki-vocabulary-deck-name "Fallbacks"
        anki-vocabulary-model-name "Sparse Result"
        anki-vocabulary-field-alist
        '(("Expression" . "<${expression:单词}>")
          ("Glossary" . "<${glossary:释义}>")
          ("Phonetic" . "<${phonetic:音标}>")
          ("Sentence" . "${sentence:原文例句}"))
        anki-vocabulary-audio-fileds nil
        anki-vocabulary-sentence-translator
        (lambda (_sentence) "translated")
        anki-vocabulary-word-searcher
        (lambda (_word) nil))
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (&rest arguments)
          (setq completion arguments)
          "manually supplied glossary"))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (expression)
          (setq voice-expression expression)
          "voice://empty"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (&rest arguments)
          (setq add-note-call arguments)
          nil)))
    (list
     (anki-vocabulary "Original sparse sentence." "sparse")
     completion
     voice-expression
     add-note-call)))"##;
    let expect = expect![[
        r#"OK (nil ("translated():" "") "" ("Fallbacks" "Sparse Result" (("Expression" . "<>") ("Glossary" . "<manually supplied glossary>") ("Phonetic" . "<>") ("Sentence" . "Original sparse sentence."))))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn highlighting_quotes_regular_expression_characters_and_respects_word_boundaries() {
    let elisp_form = r##"(let (sentences)
  (setq anki-vocabulary-deck-name "Deck"
        anki-vocabulary-model-name "Model"
        anki-vocabulary-field-alist
        '(("Highlighted" . "${sentence_bold:标粗的原文例句}"))
        anki-vocabulary-audio-fileds nil
        anki-vocabulary-sentence-translator
        (lambda (_sentence) "translated")
        anki-vocabulary-word-searcher
        (lambda (word)
          `((expression . ,word)
            (glossary . ("meaning"))
            (phonetic . "phonetic"))))
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (_prompt collection &rest _arguments)
          (car collection)))
       ((symbol-function 'youdao-dictionary--format-voice-url)
        (lambda (_expression) "voice://quoted"))
       ((symbol-function 'anki-connect-add-note)
        (lambda (_deck _model fields &rest _audio)
          (push (cdr (assoc "Highlighted" fields)) sentences))))
    (dolist
        (case
         '(("card" "Card card discard card-reader postcard.")
           ("a.b" "Use a.b, not axb or pre-a.b-post.")
           ("C++" "C++ and C+ are distinct tokens.")))
      (anki-vocabulary (cadr case) (car case)))
    (nreverse sentences)))"##;
    let expect = expect![[
        r#"OK ("<B>Card</B> <b>card</b> discard <b>card</b>-reader postcard." "Use <b>a.b</b>, not axb or pre-<b>a.b</b>-post." "C++ and C+ are distinct tokens.")"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}
