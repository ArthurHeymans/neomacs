use expect_test::expect;

use super::assert_apples_mode_parity;

#[test]
fn parse_error_extracts_locations_type_message_number_buffer_and_overlay() {
    let elisp_form = r##"(with-temp-buffer
                (insert "set value to 1\nset value to missing\nreturn value\n")
                (let ((overlay (make-overlay 1 1)))
                  (setq apples-plist
                        (list :run-info (cons 17 (current-buffer))
                              :err-ov overlay))
                  (let ((parsed
                         (multiple-value-list
                          (apples-parse-error
                           "8:15: execution error: The variable missing is not defined. (-2753)"))))
                    (list
                     (butlast parsed 2)
                     (eq (nth 6 parsed) (current-buffer))
                     (eq (nth 7 parsed) overlay)))))"##;
    let expect = expect![[
        r#"OK ((nil 25 32 "execution error:" "The variable missing is not defined." -2753) t t)"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn parse_error_preserves_unknown_output_and_nil_location_contract() {
    let elisp_form = r##"(let ((apples-plist
                         (list :run-info (cons nil nil)
                               :err-ov (make-overlay 1 1))))
                (multiple-value-list
                 (apples-parse-error
                  "osascript: command not found\nunexpected diagnostic")))"##;
    let expect = expect!["OK (t nil nil nil nil nil nil #<overlay from 1 to 1 in *scratch*>)"];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn successful_result_unquotes_output_unescapes_quotes_and_records_raw_and_display_values() {
    let elisp_form = r##"(with-temp-buffer
                (setq major-mode 'apples-mode
                      apples-plist (list :run-info (cons 1 (current-buffer))))
                (let ((first
                       (apples-result
                        "\"A \\\"quoted\\\" value: 100%\""
                        0
                        "return 1")))
                  (list
                   first
                   (substring-no-properties first)
                   (text-properties-at 0 first)
                   (apples-plist-get :last-raw-result)
                   (apples-plist-get :last-result)
                   (memq 'apples-display-result pre-command-hook)
                   (memq 'apples-delete-result after-change-functions))))"##;
    let expect = expect![[
        r#"OK (#("Result: A \"quoted\" value: 100%" 0 8 (face apples-result-prompt)) "Result: A \"quoted\" value: 100%" (face apples-result-prompt) "\"A \\\"quoted\\\" value: 100%\"" #("Result: A \"quoted\" value: 100%%" 0 8 (face apples-result-prompt)) (apples-display-result t) (apples-delete-result t))"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn structured_execution_error_formats_prompts_and_highlights_the_source_range() {
    let elisp_form = r##"(with-temp-buffer
                (insert "set total to 0\nset total to missing\nreturn total\n")
                (let ((overlay (make-overlay 1 1))
                      (apples-follow-error-position nil))
                  (setq major-mode 'apples-mode
                        apples-plist
                        (list :run-info (cons 1 (current-buffer))
                              :err-ov overlay))
                  (let ((result
                         (apples-result
                          "15:34: execution error: The variable missing is not defined. (-2753)"
                          1
                          "script")))
                    (list
                     (substring-no-properties result)
                     (text-properties-at 0 result)
                     (text-properties-at (1- (length result)) result)
                     (overlay-start overlay)
                     (overlay-end overlay)
                     (eq (overlay-buffer overlay) (current-buffer))
                     (apples-plist-get :last-raw-result)))))"##;
    let expect = expect![[
        r#"OK ("execution error: The variable missing is not defined. [-2753]" (face apples-error-prompt) nil 16 35 t "15:34: execution error: The variable missing is not defined. (-2753)")"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn unknown_execution_error_is_displayed_verbatim_and_escapes_message_percent_sequences() {
    let elisp_form = r##"(with-temp-buffer
                (setq major-mode 'apples-mode
                      apples-plist
                      (list :run-info (cons nil nil)
                            :err-ov (make-overlay 1 1)))
                (let ((result
                       (apples-result
                        "transport failed at 100%: no response"
                        1
                        "script")))
                  (list result
                        (apples-plist-get :last-result)
                        (apples-plist-get :last-raw-result))))"##;
    let expect = expect![[
        r#"OK ("transport failed at 100%: no response" "transport failed at 100%%: no response" "transport failed at 100%: no response")"#
    ]];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn run_info_and_result_cleanup_manage_overlays_and_buffer_local_hooks_as_one_lifecycle() {
    let elisp_form = r##"(with-temp-buffer
                (insert "return 42\n")
                (let ((overlay (make-overlay 2 5)))
                  (setq major-mode 'apples-mode
                        apples-plist (list :err-ov overlay))
                  (apples-set-run-info t 7)
                  (apples-display-result "Result: 42")
                  (let ((during
                         (list
                          (car (apples-plist-get :run-info))
                          (eq (cdr (apples-plist-get :run-info))
                              (current-buffer))
                          (memq 'apples-display-result pre-command-hook)
                          (memq 'apples-delete-result after-change-functions)
                          (overlay-buffer overlay))))
                    (apples-delete-result)
                    (list
                     during
                     (overlay-buffer overlay)
                     (memq 'apples-display-result pre-command-hook)
                     (memq 'apples-delete-result after-change-functions)))))"##;
    let expect =
        expect!["OK ((7 t (apples-display-result t) (apples-delete-result t) nil) nil nil nil)"];
    assert_apples_mode_parity(elisp_form, expect);
}

#[test]
fn nested_overlay_cleanup_deletes_every_real_overlay_and_ignores_other_values() {
    let elisp_form = r##"(with-temp-buffer
                (let ((one (make-overlay 1 1))
                      (two (make-overlay 1 1))
                      (three (make-overlay 1 1)))
                  (apples-delete-overlay
                   (list one (cons two (list three)) nil 'not-an-overlay))
                  (mapcar #'overlay-buffer (list one two three))))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_apples_mode_parity(elisp_form, expect);
}
