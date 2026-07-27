use expect_test::expect;

use super::assert_anki_vocabulary_parity;

#[test]
fn interactive_configuration_maps_a_real_note_model_and_preserves_field_ordering_rules() {
    let elisp_form = r##"(let ((answers
       '("Language"
         "Vocabulary"
         "${expression:单词}"
         "${glossary:释义}"
         "${phonetic:音标}"
         "${sentence:原文例句}"
         "${sentence_bold:标粗的原文例句}"
         "${translation:翻译例句}"
         "${sound:发声}"
         "SKIP"))
      events)
  (cl-letf
      (((symbol-function 'anki-connect-deck-names)
        (lambda ()
          (push '(deck-names) events)
          '("Inbox" "Language" "Archive")))
       ((symbol-function 'anki-connect-model-names)
        (lambda ()
          (push '(model-names) events)
          '("Basic" "Vocabulary")))
       ((symbol-function 'anki-connect-model-field-names)
        (lambda (model)
          (push (list 'model-fields model) events)
          '("Word" "Meaning" "IPA" "Context" "Highlighted"
            "Translation" "Audio" "Notes")))
       ((symbol-function 'completing-read)
        (lambda (prompt collection &rest arguments)
          (let ((answer (pop answers)))
            (push (list 'complete prompt
                        (copy-sequence collection)
                        arguments answer)
                  events)
            answer))))
    (let ((result (anki-vocabulary-set-ankiconnect)))
      (list
       result
       answers
       anki-vocabulary-deck-name
       anki-vocabulary-model-name
       anki-vocabulary-field-alist
       anki-vocabulary-audio-fileds
       (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil nil "Language" "Vocabulary" (("Translation" . "${translation:翻译例句}") ("Highlighted" . "${sentence_bold:标粗的原文例句}") ("Context" . "${sentence:原文例句}") ("IPA" . "${phonetic:音标}") ("Meaning" . "${glossary:释义}") ("Word" . "${expression:单词}")) ("Audio") ((deck-names) (model-names) (complete "Select the Deck Name:" ("Inbox" "Language" "Archive") nil "Language") (complete "Select the Model Name:" ("Basic" "Vocabulary") nil "Vocabulary") (model-fields "Vocabulary") (complete "Word" ("${expression:单词}" "${glossary:释义}" "${phonetic:音标}" "${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP") nil "${expression:单词}") (complete "Meaning" ("${glossary:释义}" "${phonetic:音标}" "${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP") nil "${glossary:释义}") (complete "IPA" ("${phonetic:音标}" "${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP") nil "${phonetic:音标}") (complete "Context" ("${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP") nil "${sentence:原文例句}") (complete "Highlighted" ("${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP") nil "${sentence_bold:标粗的原文例句}") (complete "Translation" ("${translation:翻译例句}" "${sound:发声}" "SKIP") nil "${translation:翻译例句}") (complete "Audio" ("${sound:发声}" "SKIP") nil "${sound:发声}") (complete "Notes" ("${sound:发声}" "SKIP") nil "SKIP")))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn reconfiguration_discards_stale_fields_and_supports_multiple_audio_destinations() {
    let elisp_form = r##"(let ((answers
       '("Fresh deck"
         "Fresh model"
         "${sound:发声}"
         "${expression:单词}"
         "${sound:发声}"
         "SKIP")))
  (setq anki-vocabulary-deck-name "Stale deck"
        anki-vocabulary-model-name "Stale model"
        anki-vocabulary-field-alist
        '(("Old" . "${glossary:释义}"))
        anki-vocabulary-audio-fileds
        '("OldAudio"))
  (cl-letf
      (((symbol-function 'anki-connect-deck-names)
        (lambda () '("Fresh deck")))
       ((symbol-function 'anki-connect-model-names)
        (lambda () '("Fresh model")))
       ((symbol-function 'anki-connect-model-field-names)
        (lambda (_model)
          '("FrontAudio" "Word" "BackAudio" "Unused")))
       ((symbol-function 'completing-read)
        (lambda (&rest _arguments)
          (pop answers))))
    (list
     (anki-vocabulary-set-ankiconnect)
     anki-vocabulary-deck-name
     anki-vocabulary-model-name
     anki-vocabulary-field-alist
     anki-vocabulary-audio-fileds
     answers)))"##;
    let expect = expect![[
        r#"OK (nil "Fresh deck" "Fresh model" (("Word" . "${expression:单词}")) ("BackAudio" "FrontAudio") nil)"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn used_text_templates_are_removed_from_later_completion_choices_but_sound_and_skip_remain() {
    let elisp_form = r##"(let ((answers
       '("Deck"
         "Model"
         "${expression:单词}"
         "${sound:发声}"
         "${sentence:原文例句}"
         "SKIP"))
      field-choices)
  (cl-letf
      (((symbol-function 'anki-connect-deck-names)
        (lambda () '("Deck")))
       ((symbol-function 'anki-connect-model-names)
        (lambda () '("Model")))
       ((symbol-function 'anki-connect-model-field-names)
        (lambda (_model)
          '("Front" "Audio" "Example" "Extra")))
       ((symbol-function 'completing-read)
        (lambda (prompt collection &rest _arguments)
          (when (member prompt '("Front" "Audio" "Example" "Extra"))
            (push (list prompt (copy-sequence collection)) field-choices))
          (pop answers))))
    (anki-vocabulary-set-ankiconnect)
    (list
     (nreverse field-choices)
     anki-vocabulary-field-alist
     anki-vocabulary-audio-fileds)))"##;
    let expect = expect![[
        r#"OK ((("Front" ("${expression:单词}" "${glossary:释义}" "${phonetic:音标}" "${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP")) ("Audio" ("${glossary:释义}" "${phonetic:音标}" "${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP")) ("Example" ("${glossary:释义}" "${phonetic:音标}" "${sentence:原文例句}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP")) ("Extra" ("${glossary:释义}" "${phonetic:音标}" "${sentence_bold:标粗的原文例句}" "${translation:翻译例句}" "${sound:发声}" "SKIP"))) (("Example" . "${sentence:原文例句}") ("Front" . "${expression:单词}")) ("Audio"))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn cancellation_like_empty_choices_are_stored_exactly_as_the_completion_api_returns_them() {
    let elisp_form = r##"(let ((answers '("" "" "")))
  (cl-letf
      (((symbol-function 'anki-connect-deck-names)
        (lambda () nil))
       ((symbol-function 'anki-connect-model-names)
        (lambda () nil))
       ((symbol-function 'anki-connect-model-field-names)
        (lambda (model)
          (list (format "Field-for-%s" model))))
       ((symbol-function 'completing-read)
        (lambda (&rest _arguments)
          (pop answers))))
    (list
     (anki-vocabulary-set-ankiconnect)
     anki-vocabulary-deck-name
     anki-vocabulary-model-name
     anki-vocabulary-field-alist
     anki-vocabulary-audio-fileds)))"##;
    let expect = expect![[r#"OK (nil "" "" (("Field-for-" . "")) nil)"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}
