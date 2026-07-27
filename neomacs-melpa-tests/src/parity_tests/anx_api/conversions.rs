use expect_test::expect;

use super::assert_anx_api_parity;

#[test]
fn anx_api_escape_json_quotes_compacts_and_escapes_practical_document() {
    let elisp_form = r##"(with-temp-buffer
         (insert "{\n  \"name\": \"Ada\",\n"
                 "  \"path\": \"C:\\\\work\\\\file\",\n"
                 "  \"note\": \"say \\\\\"hello\\\\\"\"\n"
                 "}\n")
         (goto-char 8)
         (let ((before-point (point)))
           (list (anx-escape-json)
                 (buffer-string)
                 before-point
                 (point)
                 (buffer-modified-p))))"##;
    let expect = expect![[
        r#"OK (nil "\"{\\\"name\\\": \\\"Ada\\\",\\\"path\\\": \\\"C:\\\\work\\\\file\\\",\\\"note\\\": \\\"say \\\\\\\\\"hello\\\\\\\\\"\\\"\n}\n\"" 8 7 t)"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_unescape_json_removes_outer_quotes_and_two_escape_layers() {
    let elisp_form = r##"(with-temp-buffer
         (insert "\"{\\\"name\\\":\\\"Ada\\\","
                 "\\\"path\\\":\\\"C:\\\\\\\\work\\\\\\\\file\\\","
                 "\\\"quote\\\":\\\"hello\\\"}\"")
         (goto-char 10)
         (let ((before-point (point)))
           (list (anx-unescape-json)
                 (buffer-string)
                 before-point
                 (point)
                 (buffer-modified-p))))"##;
    let expect = expect![[
        r#"OK (nil "{\"name\":\"Ada\",\"path\":\"C:\\\\work\\\\file\",\"quote\":\"hello\"}" 10 7 t)"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_escape_then_unescape_roundtrip_pins_whitespace_normalization() {
    let elisp_form = r##"(with-temp-buffer
         (insert "{\n    \"items\": [1, 2],\n"
                 "    \"nested\": {\"ok\": true}\n"
                 "}\n")
         (let ((original (buffer-string)))
           (anx-escape-json)
           (let ((escaped (buffer-string)))
             (anx-unescape-json)
             (list original escaped (buffer-string)
                   (equal original (buffer-string))))))"##;
    let expect = expect![[
        r#"OK ("{\n    \"items\": [1, 2],\n    \"nested\": {\"ok\": true}\n}\n" "\"{\\\"items\\\": [1, 2],\\\"nested\\\": {\\\"ok\\\": true}\n}\n\"" "{\"items\": [1, 2],\"nested\": {\"ok\": true}\n}\n" nil)"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_lisp_to_json_reads_real_buffer_and_routes_encoded_value() {
    let elisp_form = r##"(with-temp-buffer
         (rename-buffer "member-fixture" t)
         (insert "(:member (:id 7 :name \"Ada\" :active t)"
                 " :tags [\"ops\" \"api\"] :empty nil)")
         (let (calls)
           (cl-letf (((symbol-function 'anx--pop-up-buffer)
                      (lambda (&rest args)
                        (push args calls)
                        'shown)))
             (list (anx-lisp-to-json)
                   (nreverse calls)
                   (buffer-string)
                   (buffer-name)))))"##;
    let expect = expect![[
        r#"OK (shown (("member-fixture.json" "{\"member\":{\"id\":7,\"name\":\"Ada\",\"active\":true},\"tags\":[\"ops\",\"api\"],\"empty\":null}" js-mode)) "(:member (:id 7 :name \"Ada\" :active t) :tags [\"ops\" \"api\"] :empty nil)" "member-fixture")"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_json_to_lisp_decodes_escaped_json_and_switches_named_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (rename-buffer "campaign.json" t)
         (insert "\"{\\\"campaign\\\":{\\\"id\\\":9,"
                 "\\\"active\\\":true},\\\"items\\\":[1,2]}\"")
         (let (calls)
           (cl-letf (((symbol-function 'anx--pop-up-buffer)
                      (lambda (&rest args)
                        (push (cons 'popup args) calls)
                        'shown))
                     ((symbol-function 'switch-to-buffer)
                      (lambda (&rest args)
                        (push (cons 'switch args) calls)
                        'switched)))
             (list (anx-json-to-lisp)
                   (nreverse calls)
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (switched ((popup "campaign.json.el" ((campaign (id . 9) (active . t)) (items . [1 2])) emacs-lisp-mode) (switch "campaign.json.el")) "\"{\\\"campaign\\\":{\\\"id\\\":9,\\\"active\\\":true},\\\"items\\\":[1,2]}\"")"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_pop_up_buffer_creates_mode_buffer_prints_and_marks_save_offer() {
    let elisp_form = r##"(let (shown buffer)
         (unwind-protect
             (cl-letf (((symbol-function 'switch-to-buffer-other-window)
                        (lambda (candidate)
                          (setq shown candidate)
                          'window)))
               (anx--pop-up-buffer
                " *anx-result*"
                '(:response (:status "OK" :count 2))
                'emacs-lisp-mode)
               (setq buffer (get-buffer " *anx-result*"))
               (with-current-buffer buffer
                 (list (eq shown buffer)
                       major-mode
                       buffer-offer-save
                       (get 'buffer-offer-save 'permanent-local)
                       (buffer-modified-p)
                       (point)
                       (buffer-string))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect =
        expect![[r#"OK (t emacs-lisp-mode t t t 38 "\n(:response (:status \"OK\" :count 2))\n")"#]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_conversion_commands_preserve_native_reader_errors() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "(:unterminated")
           (condition-case err
               (anx-lisp-to-json)
             (error (list (car err) (cdr err)))))
         (with-temp-buffer
           (insert "\"{not valid json}\"")
           (condition-case err
               (anx-json-to-lisp)
             (error (list (car err) (cdr err))))))"##;
    let expect = expect!["OK ((end-of-file nil) (json-end-of-file nil))"];
    assert_anx_api_parity(elisp_form, expect);
}
