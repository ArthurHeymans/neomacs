use expect_test::expect;

use super::assert_anki_connect_parity;

#[test]
fn model_listing_and_field_lookup_convert_vectors_and_send_exact_model_param() {
    let elisp_form = r##"(let (requests)
                      (cl-letf
                          (((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              (cond
                               ((equal
                                 action
                                 "modelNames")
                                ["Basic"
                                 "Basic (and reversed card)"
                                 "Cloze"])
                               ((equal
                                 action
                                 "modelFieldNames")
                                ["Front"
                                 "Back"
                                 "Extra"])))))
                        (list
                         (anki-connect-model-names)
                         (anki-connect-model-field-names
                          "Basic (and reversed card)")
                         (nreverse requests))))"##;
    let expect = expect![[
        r#"OK (("Basic" "Basic (and reversed card)" "Cloze") ("Front" "Back" "Extra") (("modelNames" nil) ("modelFieldNames" (("modelName" . "Basic (and reversed card)")))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn add_note_builds_exact_required_fields_empty_tags_and_optional_audio_payload() {
    let elisp_form = r##"(let (requests)
                      (cl-letf
                          (((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              (if
                                  (= (length requests) 1)
                                  7001
                                7002))))
                        (list
                         (anki-connect-add-note
                          "Study::Japanese"
                          "Basic"
                          '(("Front"
                             . "猫 <b>ねこ</b>")
                            ("Back"
                             . "cat")))
                         (anki-connect-add-note
                          "Study::Japanese"
                          "Basic"
                          '(("Front"
                             . "犬")
                            ("Back"
                             . "dog"))
                          '[(("url"
                              . "https://example.invalid/dog.mp3")
                             ("filename"
                              . "dog.mp3")
                             ("skipHash"
                              . "0123456789abcdef")
                             ("fields"
                              . ["Back"]))])
                         (nreverse requests))))"##;
    let expect = expect![[
        r#"OK (7001 7002 (("addNote" (("note" ("deckName" . "Study::Japanese") ("modelName" . "Basic") ("fields" ("Front" . "猫 <b>ねこ</b>") ("Back" . "cat")) #1=("tags" . [])))) ("addNote" (("note" ("deckName" . "Study::Japanese") ("modelName" . "Basic") ("fields" ("Front" . "犬") ("Back" . "dog")) #1# ("audio" . [(("url" . "https://example.invalid/dog.mp3") ("filename" . "dog.mp3") ("skipHash" . "0123456789abcdef") ("fields" . ["Back"]))]))))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn update_note_defaults_to_empty_tag_vector_and_preserves_explicit_tags_and_field_values() {
    let elisp_form = r##"(let (requests)
                      (cl-letf
                          (((symbol-function
                             'anki-connect-request)
                            (lambda (action params)
                              (push
                               (list action params)
                               requests)
                              nil)))
                        (list
                         (anki-connect-update-note
                          1234567890
                          '(("Front"
                             . "updated question")
                            ("Back"
                             . "updated answer")))
                         (anki-connect-update-note
                          9876543210
                          '(("Front"
                             . "第二問")
                            ("Back"
                             . "second answer"))
                          '["reviewed"
                            "language::japanese"])
                         (nreverse requests))))"##;
    let expect = expect![[
        r#"OK (nil nil (("updateNote" (("note" ("id" . 1234567890) ("fields" ("Front" . "updated question") ("Back" . "updated answer")) ("tags" . [])))) ("updateNote" (("note" ("id" . 9876543210) ("fields" ("Front" . "第二問") ("Back" . "second answer")) ("tags" . ["reviewed" "language::japanese"]))))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn sequential_model_add_and_update_workflow_round_trips_through_json_transport() {
    let elisp_form = r##"(let (requests buffers)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'url-retrieve-synchronously)
                                (lambda (_url)
                                  (let* ((request-text
                                          (decode-coding-string
                                           url-request-data
                                           'utf-8))
                                         (json-object-type
                                          'alist)
                                         (json-array-type
                                          'vector)
                                         (json-key-type
                                          'symbol)
                                         (request
                                          (json-read-from-string
                                           request-text))
                                         (action
                                          (cdr
                                           (assoc
                                            'action
                                            request)))
                                         (result
                                          (cond
                                           ((equal
                                             action
                                             "modelNames")
                                            ["Basic"
                                             "Cloze"])
                                           ((equal
                                             action
                                             "modelFieldNames")
                                            ["Front"
                                             "Back"])
                                           ((equal
                                             action
                                             "addNote")
                                            424242)
                                           ((equal
                                             action
                                             "updateNote")
                                            nil)))
                                         (buffer
                                          (generate-new-buffer
                                           " *anki-workflow-response*")))
                                    (push request requests)
                                    (push buffer buffers)
                                    (with-current-buffer
                                        buffer
                                      (insert
                                       "HTTP/1.1 200 OK\nContent-Type: application/json\n\n")
                                      (insert
                                       (json-encode
                                        `(("result"
                                           . ,result)
                                          ("error"
                                           . nil)))))
                                    buffer))))
                            (list
                             (anki-connect-model-names)
                             (anki-connect-model-field-names
                              "Basic")
                             (anki-connect-add-note
                              "Default"
                              "Basic"
                              '(("Front"
                                 . "What is 2 + 2?")
                                ("Back"
                                 . "4")))
                             (anki-connect-update-note
                              424242
                              '(("Front"
                                 . "What is 2 + 3?")
                                ("Back"
                                 . "5"))
                              '["corrected"])
                             (nreverse requests)))
                        (mapc
                         (lambda (buffer)
                           (when
                               (buffer-live-p buffer)
                             (kill-buffer buffer)))
                         buffers)))"##;
    let expect = expect![[
        r#"OK (("Basic" "Cloze") ("Front" "Back") 424242 nil (((action . "modelNames") (version . 6)) ((action . "modelFieldNames") (version . 6) (params (modelName . "Basic"))) ((action . "addNote") (version . 6) (params (note (deckName . "Default") (modelName . "Basic") (fields (Front . "What is 2 + 2?") (Back . "4")) (tags . [])))) ((action . "updateNote") (version . 6) (params (note (id . 424242) (fields (Front . "What is 2 + 3?") (Back . "5")) (tags . ["corrected"]))))))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}
