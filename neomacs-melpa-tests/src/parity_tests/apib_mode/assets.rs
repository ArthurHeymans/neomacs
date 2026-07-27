use expect_test::expect;

use super::assert_apib_mode_parity;

#[test]
fn parse_to_plist_quotes_a_whitespace_metacharacter_filename_and_decodes_refract_json() {
    let elisp_form = r##"(let ((apib-drafter-executable "/opt/drafter bin/drafter")
      commands)
  (cl-letf
      (((symbol-function 'shell-command-to-string)
        (lambda (command)
          (push command commands)
          "{\"element\":\"parseResult\",\"content\":[{\"element\":\"asset\",\"attributes\":{\"contentType\":{\"element\":\"string\",\"content\":\"application/json\"}},\"content\":\"{\\\"status\\\":\\\"ok\\\"}\"}]}")))
    (let ((result
           (apib-parse-to-plist
            "/workspace/API Specs/users & teams.apib")))
      (list
       result
       (apib-refract-element-p result "parseResult")
       (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK ((:element "parseResult" :content [(:element "asset" :attributes (:contentType (:element "string" :content "application/json")) :content "{\"status\":\"ok\"}")]) t ("/opt/drafter bin/drafter -f json -u /workspace/API\\ Specs/users\\ \\&\\ teams.apib"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn parse_to_plist_warns_and_returns_nil_when_drafter_returns_an_error_element() {
    let elisp_form = r##"(let ((apib-drafter-executable "drafter")
      events)
  (cl-letf
      (((symbol-function 'shell-command-to-string)
        (lambda (command)
          (push (list 'shell command) events)
          "{\"element\":\"annotation\",\"content\":\"syntax error\"}"))
       ((symbol-function 'display-warning)
        (lambda (type message &rest arguments)
          (push (list 'warning type message arguments) events)
          'warned)))
    (list
     (apib-parse-to-plist "/workspace/broken.apib")
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil ((shell "drafter -f json -u /workspace/broken.apib") (warning apib-mode "Could not parse the document" nil)))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn drafter_guard_warns_without_executing_the_wrapped_parse_expression() {
    let elisp_form = r##"(let ((apib-drafter-executable nil)
      events)
  (cl-letf
      (((symbol-function 'shell-command-to-string)
        (lambda (command)
          (push (list 'unexpected-shell command) events)
          "{}"))
       ((symbol-function 'display-warning)
        (lambda (type message &rest arguments)
          (push (list 'warning type message arguments) events)
          'warned)))
    (list
     (apib-parse-to-plist "/workspace/not-run.apib")
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (warned ((warning apib-mode "drafter binary not found, please install it in your exec-path" nil)))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn get_assets_extracts_only_matching_content_types_from_a_nested_api_document() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/orders.apib")
  (let
      ((document
        '(:element "parseResult"
          :content
          [(:element "category"
            :content
            [(:element "asset"
              :attributes
              (:contentType
               (:element "string" :content "application/json"))
              :content "{\"order\":1}")
             (:element "asset"
              :attributes
              (:contentType
               (:element "string" :content "application/schema+json"))
              :content "{\"type\":\"object\"}")])
           (:element "asset"
            :attributes
            (:contentType
             (:element "string" :content "application/json"))
            :content "{\"order\":2}")]))
       calls)
    (cl-letf
        (((symbol-function 'apib-parse-to-plist)
          (lambda (filename)
            (push filename calls)
            document)))
      (list
       (apib-get-assets "application/json")
       (apib-get-assets "application/schema+json")
       (apib-get-assets "text/plain")
       (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (("{\"order\":2}" "{\"order\":1}") ("{\"type\":\"object\"}") nil ("/workspace/orders.apib" "/workspace/orders.apib" "/workspace/orders.apib"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn get_assets_returns_nil_for_failed_parses_and_assets_without_content_type() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/incomplete.apib")
  (let ((results
         (list
          nil
          '(:element "parseResult"
            :content
            [(:element "asset" :content "{}")
             (:element "asset"
              :attributes
              (:contentType (:element "string" :content nil))
              :content "{\"ignored\":true}")]))))
    (cl-letf
        (((symbol-function 'apib-parse-to-plist)
          (lambda (_filename) (pop results))))
      (list
       (apib-get-assets "application/json")
       (apib-get-assets "application/json")))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn print_assets_replaces_the_named_buffer_and_separates_multiple_payloads() {
    let elisp_form = r##"(let ((apib-asset-buffer "*apib-practical-assets*")
      (payloads
       '("{\"id\":1,\"active\":true}"
         "{\"id\":2,\"active\":false}"))
      requested)
  (with-current-buffer (get-buffer-create apib-asset-buffer)
    (erase-buffer)
    (insert "stale output"))
  (cl-letf
      (((symbol-function 'apib-get-assets)
        (lambda (content-type)
          (setq requested content-type)
          payloads))
       ((symbol-function 'display-buffer)
        (lambda (&rest _arguments) nil)))
    (apib-print-assets "application/json")
    (with-current-buffer apib-asset-buffer
      (list
       (buffer-string)
       (buffer-modified-p)
       major-mode
       (point)
       requested))))"##;
    let expect = expect![[
        r#"OK ("{\"id\":1,\"active\":true}\n\n{\"id\":2,\"active\":false}\n\n" nil help-mode 1 "application/json")"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn interactive_json_and_schema_commands_select_the_exact_refract_media_types() {
    let elisp_form = r##"(let (calls)
  (cl-letf
      (((symbol-function 'apib-print-assets)
        (lambda (content-type)
          (push content-type calls)
          (concat "printed:" content-type))))
    (list
     (apib-get-json)
     (apib-get-json-schema)
     (nreverse calls)
     (commandp 'apib-get-json)
     (commandp 'apib-get-json-schema))))"##;
    let expect = expect![[
        r#"OK ("printed:application/json" "printed:application/schema+json" ("application/json" "application/schema+json") t t)"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}
