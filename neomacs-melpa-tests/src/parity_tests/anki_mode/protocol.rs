use expect_test::expect;

use super::assert_anki_mode_parity;

#[test]
fn anki_mode_connect_builds_exact_sync_json_request() {
    let elisp_form = r##"(let (captured)
         (cl-letf (((symbol-function 'request)
                    (lambda (&rest args)
                      (setq captured args)
                      'request-return))
                   ((symbol-function 'anki-mode--http-success-factory)
                    (lambda (callback) (list 'wrapped callback))))
           (list
            (anki-mode-connect 'done "modelFieldNames"
                               '(("modelName" . "Basic & Reverse")) t)
            captured
            (decode-coding-string (plist-get (cdr captured) :data) 'utf-8))))"##;
    let expect = expect![[
        r#"OK (request-return ("http://localhost:8765" :type "POST" :data "{\"action\":\"modelFieldNames\",\"version\":6,\"params\":{\"modelName\":\"Basic & Reverse\"}}" :headers (("Content-Type" . "application/json")) :parser json-read :sync t :success (wrapped done) :error #[(&rest --cl-rest--) ((let* ((error-thrown (car (cdr (plist-member --cl-rest-- ':error-thrown))))) (message "Anki mode http request failed, is anki running and is the anki-connect extension installed?\nrequest.el error was: %S, request.el normally uses the curl backend so check the curl manual for the meaning of exit codes." error-thrown))) (t) nil "\n\n(fn &key ERROR-THROWN &allow-other-keys)"]) "{\"action\":\"modelFieldNames\",\"version\":6,\"params\":{\"modelName\":\"Basic & Reverse\"}}")"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_connect_omits_params_and_preserves_async_flag() {
    let elisp_form = r##"(let (captured)
         (cl-letf (((symbol-function 'request)
                    (lambda (&rest args) (setq captured args)))
                   ((symbol-function 'anki-mode--http-success-factory)
                    (lambda (callback) (list 'success-for callback))))
           (anki-mode-connect #'identity "version" nil nil)
           (list (car captured)
                 (plist-get (cdr captured) :type)
                 (plist-get (cdr captured) :data)
                 (plist-get (cdr captured) :headers)
                 (plist-get (cdr captured) :parser)
                 (plist-get (cdr captured) :sync)
                 (plist-get (cdr captured) :success)
                 (functionp (plist-get (cdr captured) :error)))))"##;
    let expect = expect![[
        r#"OK ("http://localhost:8765" "POST" "{\"action\":\"version\",\"version\":6}" (("Content-Type" . "application/json")) json-read nil (success-for identity) t)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_connect_logs_wire_payload_and_error_callback_message() {
    let elisp_form = r##"(let ((anki-mode--log-requests t)
               messages error-function)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages)))
                   ((symbol-function 'request)
                    (lambda (&rest args)
                      (setq error-function (plist-get (cdr args) :error)))))
           (anki-mode-connect #'ignore "deckNames" nil t)
           (funcall error-function :error-thrown '(error . "connection refused")
                    :ignored 42)
           (nreverse messages)))"##;
    let expect = expect![[
        r#"OK ("Anki connect sending \"{\\\"action\\\":\\\"deckNames\\\",\\\"version\\\":6}\"" "Anki mode http request failed, is anki running and is the anki-connect extension installed?\nrequest.el error was: (error . \"connection refused\"), request.el normally uses the curl backend so check the curl manual for the meaning of exit codes.")"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_http_success_callback_returns_result_and_logs_response() {
    let elisp_form = r##"(let ((anki-mode--log-requests t)
               messages results)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages))))
           (let ((callback
                  (anki-mode--http-success-factory
                   (lambda (result) (push result results) 'callback-return))))
             (list (funcall callback
                            :data '((result . ["Deck A" "Deck B"]) (error))
                            :ignored 'value)
                   (nreverse results)
                   (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (callback-return (["Deck A" "Deck B"]) ("Anki connect recv ((result . [\"Deck A\" \"Deck B\"]) (error))"))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_http_success_callback_surfaces_server_error_before_callback() {
    let elisp_form = r##"(let (called)
         (let ((callback
                (anki-mode--http-success-factory
                 (lambda (result) (setq called result)))))
           (condition-case err
               (funcall callback
                        :data '((result . 123)
                                (error . "cannot create note")))
             (error (list (car err) (cdr err) called)))))"##;
    let expect =
        expect![[r#"OK (error ("Anki connect returned error: \"cannot create note\"") nil)"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_http_success_callback_handles_null_response_contract() {
    let elisp_form = r##"(let (messages called)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages))))
           (let ((callback
                  (anki-mode--http-success-factory
                   (lambda (result) (setq called (list 'called result))))))
             (funcall callback :data nil)
             (list called (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK ((called nil) ("Warning: anki-mode-connect got null data, this probably means a bad query was sent"))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_refresh_invokes_three_protocol_stages_in_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'anki-mode-check-version)
                    (lambda () (push 'version calls)))
                   ((symbol-function 'anki-mode-update-decks)
                    (lambda () (push 'decks calls)))
                   ((symbol-function 'anki-mode-update-card-types)
                    (lambda () (push 'models calls))))
           (anki-mode-refresh)
           (nreverse calls)))"##;
    let expect = expect!["OK (version decks models)"];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_protocol_entrypoints_pass_exact_actions_callbacks_and_sync() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'anki-mode-connect)
                    (lambda (&rest args) (push args calls))))
           (anki-mode-check-version)
           (anki-mode-update-decks)
           (anki-mode-update-card-types)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((anki-mode--check-version-cb "version" nil t) (anki-mode--update-decks-cb "deckNames" nil t) (anki-mode--update-card-types-cb-1 "modelNames" nil t))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_version_callback_warns_only_for_mismatch() {
    let elisp_form = r##"(let ((anki-mode--required-anki-connect-version 6)
               messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages))))
           (list (anki-mode--check-version-cb 6)
                 (anki-mode--check-version-cb 5)
                 (anki-mode--check-version-cb 7)
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (nil #2=("Warning you have anki connect version 5 installed, but 6 is required" . #1=("Warning you have anki connect version 7 installed, but 6 is required")) #1# #2#)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_deck_and_model_callbacks_build_and_update_real_catalog() {
    let elisp_form = r##"(let ((anki-mode--decks nil)
               (anki-mode--card-types
                '(("Basic" "Old Front" "Old Back"))))
         (anki-mode--update-decks-cb
          ["Default" "Japanese::Mining" "Work"])
         (anki-mode--update-card-types-cb-2
          "Basic" ["Front" "Back"])
         (anki-mode--update-card-types-cb-2
          "Cloze" ["Text" "Back Extra"])
         (anki-mode--update-card-types-cb-2
          "Basic" ["Question" "Answer" "Source"])
         (list anki-mode--decks anki-mode--card-types))"##;
    let expect = expect![[
        r#"OK (("Default" "Japanese::Mining" "Work") (("Cloze" "Text" "Back Extra") ("Basic" "Question" "Answer" "Source")))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_model_names_callback_requests_each_field_list_in_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'anki-mode-connect)
                    (lambda (&rest args) (push args calls))))
           (anki-mode--update-card-types-cb-1
            ["Basic" "Cloze" "Image Occlusion"])
           (mapcar
            (lambda (call)
              (list (cadr call)
                    (caddr call)
                    (cadddr call)
                    (functionp (car call))))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("modelFieldNames" (("modelName" . "Basic")) t t) ("modelFieldNames" (("modelName" . "Cloze")) t t) ("modelFieldNames" (("modelName" . "Image Occlusion")) t t))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_model_field_closures_retain_distinct_model_names() {
    let elisp_form = r##"(let (callbacks
               (anki-mode--card-types nil))
         (cl-letf (((symbol-function 'anki-mode-connect)
                    (lambda (callback &rest _)
                      (push callback callbacks))))
           (anki-mode--update-card-types-cb-1 ["Basic" "Cloze"])
           (setq callbacks (nreverse callbacks))
           (funcall (car callbacks) ["Front" "Back"])
           (funcall (cadr callbacks) ["Text" "Back Extra"])
           anki-mode--card-types))"##;
    let expect = expect![[r#"OK (("Cloze" "Text" "Back Extra") ("Basic" "Front" "Back"))"#]];
    assert_anki_mode_parity(elisp_form, expect);
}
