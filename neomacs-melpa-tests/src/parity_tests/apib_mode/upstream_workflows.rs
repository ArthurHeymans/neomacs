use expect_test::expect;

use super::assert_apib_mode_parity;

#[test]
fn upstream_validate_workflow_runs_real_command_rendering_and_compilation_setup() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/apibs/test-validate.apib")
  (let ((apib-drafter-executable "drafter")
        (apib-result-buffer "*apib-upstream-validate*")
        calls)
    (cl-letf
        (((symbol-function 'call-process)
          (lambda (program infile destination display &rest arguments)
            (push (list program infile destination display arguments) calls)
            (with-current-buffer destination
              (insert "OK: valid API Blueprint\n"))
            0))
         ((symbol-function 'display-buffer)
          (lambda (&rest _arguments) nil)))
      (let ((return (apib-validate)))
        (with-current-buffer apib-result-buffer
          (list return major-mode (buffer-string)
                (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil compilation-mode "drafter -lu /workspace/apibs/test-validate.apib\nOK: valid API Blueprint\n" (("drafter" nil "*apib-upstream-validate*" t ("-lu" "/workspace/apibs/test-validate.apib"))))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_parse_workflow_runs_real_json_refract_arguments_and_preserves_filename() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/apibs/test-validate.apib")
  (let ((apib-drafter-executable "drafter")
        (apib-result-buffer "*apib-upstream-parse*")
        calls)
    (cl-letf
        (((symbol-function 'call-process)
          (lambda (program infile destination display &rest arguments)
            (push (list program infile destination display arguments) calls)
            (with-current-buffer destination
              (insert "{\"element\":\"parseResult\",\"content\":[]}\n"))
            0))
         ((symbol-function 'display-buffer)
          (lambda (&rest _arguments) nil)))
      (let ((return (apib-parse)))
        (with-current-buffer apib-result-buffer
          (list return major-mode (buffer-string)
                (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil compilation-mode "drafter -f json -u /workspace/apibs/test-validate.apib\n{\"element\":\"parseResult\",\"content\":[]}\n" (("drafter" nil "*apib-upstream-parse*" t ("-f" "json" "-u" "/workspace/apibs/test-validate.apib"))))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_json_asset_workflow_extracts_and_round_trips_the_complete_body() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/apibs/test-assets.apib")
  (let ((apib-drafter-executable "drafter")
        (apib-asset-buffer "*apib-upstream-json*")
        commands)
    (cl-letf
        (((symbol-function 'shell-command-to-string)
          (lambda (command)
            (push command commands)
            "{\"element\":\"parseResult\",\"content\":[{\"element\":\"asset\",\"attributes\":{\"contentType\":{\"element\":\"string\",\"content\":\"application/json\"}},\"content\":\"{\\\"id\\\":6161,\\\"user\\\":\\\"wvi\\\",\\\"active\\\":false,\\\"social\\\":{\\\"github\\\":{\\\"active\\\":true,\\\"id\\\":1234,\\\"uri\\\":\\\"wvi\\\"}}}\"}]}"))
         ((symbol-function 'display-buffer)
          (lambda (&rest _arguments) nil)))
      (apib-get-json)
      (with-current-buffer apib-asset-buffer
        (list
         (json-encode (json-read-from-string (buffer-string)))
         (buffer-string)
         (nreverse commands))))))"##;
    let expect = expect![[
        r#"OK ("{\"id\":6161,\"user\":\"wvi\",\"active\":false,\"social\":{\"github\":{\"active\":true,\"id\":1234,\"uri\":\"wvi\"}}}" "{\"id\":6161,\"user\":\"wvi\",\"active\":false,\"social\":{\"github\":{\"active\":true,\"id\":1234,\"uri\":\"wvi\"}}}\n\n" ("drafter -f json -u /workspace/apibs/test-assets.apib"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_json_schema_workflow_extracts_and_round_trips_nested_properties() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/apibs/test-assets.apib")
  (let ((apib-drafter-executable "drafter")
        (apib-asset-buffer "*apib-upstream-schema*")
        commands)
    (cl-letf
        (((symbol-function 'shell-command-to-string)
          (lambda (command)
            (push command commands)
            "{\"element\":\"parseResult\",\"content\":[{\"element\":\"asset\",\"attributes\":{\"contentType\":{\"element\":\"string\",\"content\":\"application/schema+json\"}},\"content\":\"{\\\"$schema\\\":\\\"http://json-schema.org/draft-04/schema#\\\",\\\"type\\\":\\\"object\\\",\\\"properties\\\":{\\\"id\\\":{\\\"type\\\":\\\"number\\\"},\\\"user\\\":{\\\"type\\\":\\\"string\\\"},\\\"active\\\":{\\\"type\\\":\\\"boolean\\\"},\\\"social\\\":{\\\"type\\\":\\\"object\\\",\\\"properties\\\":{\\\"github\\\":{\\\"type\\\":\\\"object\\\",\\\"properties\\\":{\\\"active\\\":{\\\"type\\\":\\\"boolean\\\"},\\\"id\\\":{\\\"type\\\":\\\"number\\\"},\\\"uri\\\":{\\\"type\\\":\\\"string\\\"}}}}}}}}\"}]}"))
         ((symbol-function 'display-buffer)
          (lambda (&rest _arguments) nil)))
      (apib-get-json-schema)
      (with-current-buffer apib-asset-buffer
        (list
         (json-encode (json-read-from-string (buffer-string)))
         (buffer-string)
         (nreverse commands))))))"##;
    let expect = expect![[
        r#"OK ("{\"$schema\":\"http://json-schema.org/draft-04/schema#\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"number\"},\"user\":{\"type\":\"string\"},\"active\":{\"type\":\"boolean\"},\"social\":{\"type\":\"object\",\"properties\":{\"github\":{\"type\":\"object\",\"properties\":{\"active\":{\"type\":\"boolean\"},\"id\":{\"type\":\"number\"},\"uri\":{\"type\":\"string\"}}}}}}}" "{\"$schema\":\"http://json-schema.org/draft-04/schema#\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"number\"},\"user\":{\"type\":\"string\"},\"active\":{\"type\":\"boolean\"},\"social\":{\"type\":\"object\",\"properties\":{\"github\":{\"type\":\"object\",\"properties\":{\"active\":{\"type\":\"boolean\"},\"id\":{\"type\":\"number\"},\"uri\":{\"type\":\"string\"}}}}}}}}\n\n" ("drafter -f json -u /workspace/apibs/test-assets.apib"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_whitespace_filename_workflow_keeps_one_process_argument_and_shell_quotes() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/apibs/test whitespace.apib")
  (let ((apib-drafter-executable "drafter")
        (apib-result-buffer "*apib-upstream-space-parse*")
        (apib-asset-buffer "*apib-upstream-space-assets*")
        process-calls shell-calls)
    (cl-letf
        (((symbol-function 'call-process)
          (lambda (program infile destination display &rest arguments)
            (push (list program infile destination display arguments)
                  process-calls)
            0))
         ((symbol-function 'shell-command-to-string)
          (lambda (command)
            (push command shell-calls)
            "{\"element\":\"parseResult\",\"content\":[{\"element\":\"asset\",\"attributes\":{\"contentType\":{\"element\":\"string\",\"content\":\"application/json\"}},\"content\":\"{\\\"id\\\":6161,\\\"user\\\":\\\"wvi\\\",\\\"active\\\":false}\"}]}"))
         ((symbol-function 'display-buffer)
          (lambda (&rest _arguments) nil)))
      (apib-parse)
      (apib-get-json)
      (list
       (with-current-buffer apib-result-buffer (buffer-string))
       (with-current-buffer apib-asset-buffer
         (json-encode (json-read-from-string (buffer-string))))
       (nreverse process-calls)
       (nreverse shell-calls)))))"##;
    let expect = expect![[
        r#"OK ("drafter -f json -u /workspace/apibs/test whitespace.apib\n" "{\"id\":6161,\"user\":\"wvi\",\"active\":false}" (("drafter" nil "*apib-upstream-space-parse*" t ("-f" "json" "-u" "/workspace/apibs/test whitespace.apib"))) ("drafter -f json -u /workspace/apibs/test\\ whitespace.apib"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}
