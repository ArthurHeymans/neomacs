use expect_test::expect;

use super::assert_ac_mozc_parity;

#[test]
fn ac_mozc_protobuf_shape_converters_and_pickers_cover_multiple_missing_and_nil_fields() {
    let elisp_form = r##"(let ((words
                    '((candidates
                       ((id . 1)
                        (value . "第一"))
                       ((id . 2)
                        (value . "second"))
                       ((id . 3)))))
                   (preedit
                    '((segment
                       ((key . "かな")
                        (value . "仮名"))
                       ((key . "ignored")
                        (value . "無視"))))))
               (list
                (ac-mozc-all-candidate-words-to-candidates
                 words)
                (ac-mozc-all-candidate-words-to-candidates
                 nil)
                (ac-mozc-pick-preedit
                 preedit)
                (ac-mozc-pick-preedit
                 nil)
                (ac-mozc-pick-candidates
                 (ac-mozc-all-candidate-words-to-candidates
                  words))
                (ac-mozc-pick-candidates
                 nil)))"##;
    let expect = expect![[
        r#"OK (((candidate ((id . 1) (value . "第一")) ((id . 2) (value . "second")) ((id . 3)))) ((candidate)) "かな" nil ("第一" "second" nil) nil)"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_kana_predicate_preserves_the_pinned_fullwidth_character_grammar() {
    let elisp_form = r##"(mapcar
               (lambda (string)
                 (list
                  string
                  (ac-mozc-kana-p
                   string)))
               '("かな"
                 "kana"
                 ""
                 "ａｉ"
                 "かなｂ"
                 "かなｂｙ"
                 "かなｙ"
                 "かなｈ"
                 "かなａ"
                 "かなｚ"
                 "ｂ"
                 "。！？"))"##;
    let expect = expect![[
        r#"OK (("かな" 0) ("kana" 0) ("" nil) ("ａｉ" nil) ("かなｂ" 0) ("かなｂｙ" 0) ("かなｙ" 0) ("かなｈ" 0) ("かなａ" nil) ("かなｚ" 0) ("ｂ" nil) ("。！？" 0))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_match_blocks_reentrancy_and_resets_the_sending_guard_after_success() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-mozc-send-word)
                     (lambda (word)
                       (push word calls)
                       (list
                        'sent
                        word))))
                 (let ((ac-mozc-sending
                        t))
                   (list
                    (ac-mozc-match
                     "blocked"
                     'ignored)
                    ac-mozc-sending
                    calls
                    (let ((ac-mozc-sending
                           nil))
                      (list
                       (ac-mozc-match
                        "allowed"
                        '(also ignored))
                       ac-mozc-sending
                       calls))))))"##;
    let expect = expect![[r#"OK (nil t nil ((sent "allowed") nil ("allowed")))"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_match_resets_the_sending_guard_when_the_session_signals() {
    let elisp_form = r##"(let ((ac-mozc-sending
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-mozc-send-word)
                     (lambda (word)
                       (push word calls)
                       (error
                        "session exploded"))))
                 (condition-case error-data
                     (ac-mozc-match
                      "boom"
                      nil)
                   (error
                    (list
                     error-data
                     ac-mozc-sending
                     calls)))))"##;
    let expect = expect![[r#"OK ((error "session exploded") nil ("boom"))"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_send_word_cleans_session_sends_each_character_then_appends_conversion_candidates() {
    let elisp_form = r##"(let (calls
                   (ac-mozc-preedit
                    nil)
                   (ac-mozc-candidates
                    nil))
               (cl-letf
                   (((symbol-function
                      'mozc-clean-up-session)
                     (lambda ()
                       (push
                        'cleanup
                        calls)))
                    ((symbol-function
                      'ac-mozc-handle-event)
                     (lambda (event)
                       (push
                        (list
                         'event
                         event)
                        calls)
                       (if (= event
                              ?\s)
                           (setq
                            ac-mozc-candidates
                            '((candidate
                               ((value . "変換一"))
                               ((value . "変換二")))))
                         (setq
                          ac-mozc-preedit
                          '((segment
                             ((key . "かな"))))
                          ac-mozc-candidates
                          '((candidate
                             ((value . "仮名"))
                             ((value . "かな"))))))
                       t)))
                 (list
                  (ac-mozc-send-word
                   "ab")
                  (nreverse calls)
                  ac-mozc-preedit
                  ac-mozc-candidates)))"##;
    let expect = expect![[
        r#"OK (("仮名" "かな" "変換一" "変換二") (cleanup (event 97) (event 98) (event 32)) ((segment ((key . "かな")))) ((candidate ((value . "変換一")) ((value . "変換二")))))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_send_word_stops_before_conversion_when_preedit_is_not_kana() {
    let elisp_form = r##"(let (calls
                   (ac-mozc-preedit
                    nil)
                   (ac-mozc-candidates
                    nil))
               (cl-letf
                   (((symbol-function
                      'mozc-clean-up-session)
                     (lambda ()
                       (push
                        'cleanup
                        calls)))
                    ((symbol-function
                      'ac-mozc-handle-event)
                     (lambda (event)
                       (push event calls)
                       (setq
                        ac-mozc-preedit
                        '((segment
                           ((key . "ａｉ"))))
                        ac-mozc-candidates
                        '((candidate
                           ((value . "unused")))))
                       t)))
                 (list
                  (ac-mozc-send-word
                   "xy")
                  (nreverse calls)
                  ac-mozc-candidates)))"##;
    let expect = expect![[r#"OK (nil (cleanup 120 121) ((candidate ((value . "unused")))))"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_send_word_missing_preedit_signals_after_characters_without_sending_space() {
    let elisp_form = r##"(let (calls
                   (ac-mozc-preedit
                    nil)
                   (ac-mozc-candidates
                    nil))
               (cl-letf
                   (((symbol-function
                      'mozc-clean-up-session)
                     (lambda ()
                       (push
                        'cleanup
                        calls)))
                    ((symbol-function
                      'ac-mozc-handle-event)
                     (lambda (event)
                       (push event calls)
                       t)))
                 (condition-case error-data
                     (ac-mozc-send-word
                      "ab")
                   (error
                    (list
                     error-data
                     (nreverse calls)
                     ac-mozc-preedit
                     ac-mozc-candidates)))))"##;
    let expect = expect![[r#"OK ((wrong-type-argument stringp nil) (cleanup 97 98) nil nil)"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_handle_event_consumed_output_updates_preedit_candidates_and_call_order() {
    let elisp_form = r##"(let ((ac-mozc-preedit
                    'old-preedit)
                   (ac-mozc-candidates
                    'old-candidates)
                   calls)
               (cl-letf
                   (((symbol-function
                      'mozc-key-event-to-key-and-modifiers)
                     (lambda (event)
                       (push
                        (list
                         'encode
                         event)
                        calls)
                       (list
                        'encoded
                        event)))
                    ((symbol-function
                      'mozc-session-sendkey)
                     (lambda (key)
                       (push
                        (list
                         'send
                         key)
                        calls)
                       'output))
                    ((symbol-function
                      'mozc-protobuf-get)
                     (lambda (_output field)
                       (push
                        (list
                         'get
                         field)
                        calls)
                       (pcase field
                         ('consumed t)
                         ('preedit
                          '((segment
                             ((key . "かな")))))
                         ('all-candidate-words
                          '((candidates
                             ((value . "候補一"))
                             ((value . "候補二")))))))))
                 (list
                  (ac-mozc-handle-event
                   ?a)
                  ac-mozc-preedit
                  ac-mozc-candidates
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (t ((segment ((key . "かな")))) ((candidate ((value . "候補一")) ((value . "候補二")))) ((encode 97) (send (encoded 97)) (get consumed) (get preedit) (get all-candidate-words)))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_handle_event_unconsumed_output_clears_state_without_reading_payloads() {
    let elisp_form = r##"(let ((ac-mozc-preedit
                    'old-preedit)
                   (ac-mozc-candidates
                    'old-candidates)
                   calls)
               (cl-letf
                   (((symbol-function
                      'mozc-key-event-to-key-and-modifiers)
                     (lambda (event)
                       (push
                        (list
                         'encode
                         event)
                        calls)
                       'encoded))
                    ((symbol-function
                      'mozc-session-sendkey)
                     (lambda (key)
                       (push
                        (list
                         'send
                         key)
                        calls)
                       'output))
                    ((symbol-function
                      'mozc-protobuf-get)
                     (lambda (_output field)
                       (push
                        (list
                         'get
                         field)
                        calls)
                       nil)))
                 (list
                  (ac-mozc-handle-event
                   ?!)
                  ac-mozc-preedit
                  ac-mozc-candidates
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil nil nil ((encode 33) (send encoded) (get consumed)))"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_handle_event_nil_output_cleans_aborts_and_returns_the_exact_error() {
    let elisp_form = r##"(let ((ac-mozc-preedit
                    'preserved-preedit)
                   (ac-mozc-candidates
                    'preserved-candidates)
                   calls)
               (cl-letf
                   (((symbol-function
                      'mozc-key-event-to-key-and-modifiers)
                     (lambda (event)
                       (push
                        (list
                         'encode
                         event)
                        calls)
                       'encoded))
                    ((symbol-function
                      'mozc-session-sendkey)
                     (lambda (key)
                       (push
                        (list
                         'send
                         key)
                        calls)
                       nil))
                    ((symbol-function
                      'mozc-clean-up-session)
                     (lambda ()
                       (push
                        'cleanup
                        calls)))
                    ((symbol-function
                      'mozc-abort)
                     (lambda ()
                       (push
                        'abort
                        calls))))
                 (condition-case error-data
                     (ac-mozc-handle-event
                      ??)
                   (error
                    (list
                     error-data
                     ac-mozc-preedit
                     ac-mozc-candidates
                     (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK ((error "Mozc session failed.") preserved-preedit preserved-candidates ((encode 63) (send encoded) cleanup abort))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}
