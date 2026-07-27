use expect_test::expect;

use super::assert_anki_editor_parity;

#[test]
fn fetch_writes_utf8_payload_invokes_curl_contract_parses_json_and_cleans_resources() {
    let elisp_form = r##"(let (invocation request-text
                          request-file callback-data)
                      (cl-letf
                          (((symbol-function 'call-process)
                            (lambda
                                (program infile destination
                                 display &rest arguments)
                              (setq request-file
                                    (substring
                                     (car
                                      (last arguments))
                                     1))
                              (setq request-text
                                    (with-temp-buffer
                                      (insert-file-contents
                                       request-file)
                                      (buffer-string)))
                              (setq invocation
                                    (list
                                     program
                                     infile
                                     destination
                                     display
                                     (append
                                      (butlast arguments)
                                      '("@REQUEST-FILE"))))
                              (insert
                               "{\"result\":{\"created\":7,\"labels\":[\"日本語\",\"review\"]},\"error\":null}")
                              0)))
                        (let ((return-value
                               (anki-editor--fetch
                                "http://127.0.0.1:8765"
                                :type "POST"
                                :data
                                "{\"action\":\"addNote\",\"text\":\"猫\"}"
                                :parser 'json-read
                                :success
                                (lambda
                                    (&rest arguments)
                                  (setq callback-data
                                        (plist-get
                                         arguments
                                         :data))))))
                          (list
                           return-value
                           request-text
                           callback-data
                           invocation
                           (file-exists-p
                            request-file)
                           (get-buffer
                            " *anki-editor-curl*")))))"##;
    let expect = expect![[
        r#"OK (nil "{\"action\":\"addNote\",\"text\":\"猫\"}" ((result (created . 7) (labels . ["日本語" "review"])) (error)) ("curl" nil t nil ("http://127.0.0.1:8765" "--silent" "-X" "POST" "--data-binary" "@REQUEST-FILE")) nil nil)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn fetch_empty_json_and_process_failure_preserve_errors_and_cleanup() {
    let elisp_form = r##"(let (first-file second-file)
                      (list
                       (cl-letf
                           (((symbol-function 'call-process)
                             (lambda
                                 (_program _infile
                                  _destination _display
                                  &rest arguments)
                               (setq first-file
                                     (substring
                                      (car
                                       (last arguments))
                                      1))
                               0)))
                         (condition-case error-data
                             (anki-editor--fetch
                              "http://offline"
                              :type "POST"
                              :data "{}"
                              :parser 'json-read
                              :success #'ignore)
                           (error error-data)))
                       (file-exists-p first-file)
                       (cl-letf
                           (((symbol-function 'call-process)
                             (lambda
                                 (_program _infile
                                  _destination _display
                                  &rest arguments)
                               (setq second-file
                                     (substring
                                      (car
                                       (last arguments))
                                      1))
                               (signal
                                'file-error
                                '("curl unavailable")))))
                         (condition-case error-data
                             (anki-editor--fetch
                              "http://offline"
                              :type "POST"
                              :data "{\"action\":\"version\"}"
                              :parser 'json-read
                              :success #'ignore)
                           (error error-data)))
                       (file-exists-p second-file)
                       (get-buffer
                        " *anki-editor-curl*")))"##;
    let expect = expect![[
        r#"OK ((error "Failed to connect to Anki.  Is Anki running with the AnkiConnect add-on enabled?") nil (file-error "curl unavailable") nil nil)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn api_call_builds_exact_host_port_version_params_and_decodes_success_reply() {
    let elisp_form = r##"(let (captured)
                      (cl-letf
                          (((symbol-function
                             'anki-editor--fetch)
                            (lambda
                                (url &rest settings)
                              (let ((json-array-type
                                     'list))
                                (setq captured
                                      (list
                                       url
                                       (plist-get
                                        settings :type)
                                       (plist-get
                                        settings :parser)
                                       (plist-get
                                        settings :data)
                                       (json-read-from-string
                                        (plist-get
                                         settings :data))))
                                (funcall
                                 (plist-get
                                  settings :success)
                                 :data
                                 '((result
                                    101 202 303)
                                   (error)))
                                nil))))
                        (let ((anki-editor-api-host
                               "anki.internal")
                              (anki-editor-api-port
                               "9876"))
                          (list
                           (anki-editor-api-call
                            'findNotes
                            :query
                            "deck:\"Study\" tag:due"
                            :includeSuspended
                            :json-false)
                           captured))))"##;
    let expect = expect![[
        r#"OK (((result 101 202 303) (error)) ("http://anki.internal:9876" "POST" json-read "{\"action\":\"findNotes\",\"version\":6,\"params\":{\"query\":\"deck:\\\"Study\\\" tag:due\",\"includeSuspended\":false}}" ((action . "findNotes") (version . 6) (params (query . "deck:\"Study\" tag:due") (includeSuspended . :json-false)))))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn api_call_and_result_preserve_transport_empty_reply_and_protocol_errors() {
    let elisp_form = r##"(list
                      (cl-letf
                          (((symbol-function
                             'anki-editor--fetch)
                            (lambda
                                (_url &rest settings)
                              (funcall
                               (plist-get
                                settings :error)
                               :error-thrown
                               '(curl
                                 . " connection refused \n")))))
                        (condition-case error-data
                            (anki-editor-api-call
                             'version)
                          (error error-data)))
                      (cl-letf
                          (((symbol-function
                             'anki-editor--fetch)
                            (lambda (&rest _)
                              nil)))
                        (condition-case error-data
                            (anki-editor-api-call
                             'version)
                          (error error-data)))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api-call)
                            (lambda (&rest _)
                              '((result . 6)
                                (error)))))
                        (anki-editor-api-call-result
                          'version))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api-call)
                            (lambda (&rest _)
                              '((result)
                                (error
                                 . "model not found")))))
                        (condition-case error-data
                            (anki-editor-api-call-result
                              'modelFieldNames
                              :modelName
                              "Missing")
                          (error error-data))))"##;
    let expect = expect![[
        r#"OK ((error "Error communicating with AnkiConnect using cURL: connection refused") (error "Got empty reply from AnkiConnect") 6 (error "model not found"))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn collection_data_macro_builds_model_field_batch_reuses_cache_and_resets_state_after_errors() {
    let elisp_form = r##"(list
                      (let ((anki-editor--collection-data-updated
                             nil)
                            calls)
                        (cl-letf
                            (((symbol-function
                               'anki-editor-api-call-result)
                              (lambda (action &rest params)
                                (push
                                 (cons action params)
                                 calls)
                                (pcase action
                                  ('modelNames
                                   '("Basic"
                                     "Cloze"))
                                  ('multi
                                   '(((result
                                      "Front"
                                      "Back")
                                     (error))
                                    ((result
                                      "Text")
                                     (error))))))))
                          (list
                           (anki-editor--with-collection-data-updated
                             (list
                              anki-editor--collection-data-updated
                              anki-editor--model-names
                              anki-editor--model-fields))
                           anki-editor--collection-data-updated
                           (nreverse calls))))
                      (let ((anki-editor--collection-data-updated
                             t)
                            (anki-editor--model-names
                             '("Cached"))
                            (anki-editor--model-fields
                             '(("Cached"
                                "Question")))
                            calls)
                        (cl-letf
                            (((symbol-function
                               'anki-editor-api-call-result)
                              (lambda (&rest arguments)
                                (push arguments calls)
                                (error
                                 "must not refresh"))))
                          (list
                           (anki-editor--with-collection-data-updated
                             (list
                              anki-editor--model-names
                              anki-editor--model-fields))
                           anki-editor--collection-data-updated
                           calls)))
                      (let ((anki-editor--collection-data-updated
                             nil))
                        (cl-letf
                            (((symbol-function
                               'anki-editor-api-call-result)
                              (lambda (action &rest _)
                                (pcase action
                                  ('modelNames
                                   '("Basic"))
                                  ('multi
                                   '(((result
                                      "Front"
                                      "Back")
                                     (error))))))))
                          (list
                           (condition-case error-data
                               (anki-editor--with-collection-data-updated
                                 (error
                                  "body failed"))
                           (error error-data))
                           anki-editor--collection-data-updated))))"##;
    let expect = expect![[
        r#"OK (((t ("Basic" "Cloze") (("Basic" "Front" "Back") ("Cloze" "Text"))) nil ((modelNames) (multi :actions [(:action modelFieldNames :version 6 :params (:modelName "Basic")) (:action modelFieldNames :version 6 :params (:modelName "Cloze"))]))) ((("Cached") (("Cached" "Question"))) t nil) ((error "body failed") nil))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn note_serialization_exports_fields_duplicate_option_tags_and_numeric_id_exactly() {
    let elisp_form = r##"(let ((note
                           (make-anki-editor-note
                            :id "1700000000001"
                            :model "Basic"
                            :deck "Study::Japanese"
                            :fields
                            '(("Front"
                               . "猫")
                              ("Back"
                               . "cat"))
                            :tags
                            '("vocab"
                              "language::japanese"))))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-entry-format)
                            (lambda () nil)))
                        (let ((anki-editor-allow-duplicates
                               nil))
                          (list
                           (anki-editor-api--note
                            note)
                           (let ((anki-editor-allow-duplicates
                                  t))
                             (anki-editor-api--note
                              note))))))"##;
    let expect = expect![[
        r#"OK ((:id 1700000000001 :deckName "Study::Japanese" :modelName "Basic" :fields #1=(("Front" . "猫") ("Back" . "cat")) :options (:allowDuplicate :json-false) :tags ["vocab" "language::japanese"]) (:id 1700000000001 :deckName "Study::Japanese" :modelName "Basic" :fields #1# :options (:allowDuplicate t) :tags ["vocab" "language::japanese"]))"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}

#[test]
fn version_sync_browse_and_find_commands_issue_exact_requests_and_errors() {
    let elisp_form = r##"(let (result-calls raw-calls messages
                          (version 6))
                      (cl-letf
                          (((symbol-function
                             'anki-editor-api-call-result)
                            (lambda (action &rest params)
                              (push
                               (cons action params)
                               result-calls)
                              (pcase action
                                ('version version)
                                ('findNotes
                                 '(11 22 33))
                                (_ nil))))
                           ((symbol-function
                             'anki-editor-api-call)
                            (lambda (action &rest params)
                              (push
                               (cons action params)
                               raw-calls)
                              '((result)
                                (error))))
                           ((symbol-function 'message)
                            (lambda
                                (format-string
                                 &rest arguments)
                              (push
                               (apply
                                #'format
                                format-string
                                arguments)
                               messages))))
                        (let ((check
                               (anki-editor-api-check))
                              (sync
                               (anki-editor-sync-collection))
                              (find
                               (anki-editor-find-notes
                                "deck:Study")))
                          (let ((anki-editor-gui-browse-ensure-foreground
                                 t))
                            (anki-editor-gui-browse
                             "nid:42"))
                          (let ((anki-editor-gui-browse-ensure-foreground
                                 nil))
                            (anki-editor-gui-browse
                             "deck:Current"))
                          (setq version 5)
                          (list
                           check sync find
                           (condition-case error-data
                               (anki-editor-api-check)
                             (error error-data))
                           (nreverse result-calls)
                           (nreverse raw-calls)
                           (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (nil #1=("Synced local Anki collection with AnkiWeb.") (11 22 33) (user-error "anki-editor requires at least version 6 of AnkiConnect") ((version) (sync) (findNotes :query "deck:Study") (version)) ((guiBrowse :query "nid:42") (guiBrowse :query "nid:42") (guiBrowse :query "deck:Current")) #1#)"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}
