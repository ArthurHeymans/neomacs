use expect_test::expect;

use super::assert_apib_mode_parity;

#[test]
fn compile_with_drafter_builds_exact_process_arguments_output_and_compilation_mode() {
    let elisp_form = r##"(let ((apib-drafter-executable "/opt/drafter")
      (apib-result-buffer "*apib-compile-practical*")
      calls)
  (cl-letf
      (((symbol-function 'call-process)
        (lambda (program infile destination display &rest arguments)
          (push
           (list program infile
                 (if (bufferp destination)
                     (buffer-name destination)
                   destination)
                 display arguments)
           calls)
          (with-current-buffer destination
            (insert
             "error: API description parse error, line 8, column 3 - line 8, column 12\n"))
          2))
       ((symbol-function 'display-buffer)
        (lambda (&rest _arguments) nil)))
    (let ((return
           (apib-compile-with-drafter
            "/workspace/API Specs/orders.apib"
            "-f" "json" "-u")))
      (with-current-buffer apib-result-buffer
        (list
         return
         major-mode mode-name
         (buffer-string)
         (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (nil compilation-mode "Compilation" "/opt/drafter -f json -u /workspace/API Specs/orders.apib\nerror: API description parse error, line 8, column 3 - line 8, column 12\n" (("/opt/drafter" nil "*apib-compile-practical*" t ("-f" "json" "-u" "/workspace/API Specs/orders.apib"))))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn validate_ports_and_strengthens_the_upstream_command_workflow() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/contracts/test-validate.apib")
  (let ((apib-drafter-executable "drafter")
        calls)
    (cl-letf
        (((symbol-function 'apib-compile-with-drafter)
          (lambda (filename &rest arguments)
            (push (cons filename arguments) calls)
            'compiled)))
      (list
       (apib-validate)
       (nreverse calls)
       (commandp 'apib-validate)))))"##;
    let expect =
        expect![[r#"OK (compiled (("/workspace/contracts/test-validate.apib" "-lu")) t)"#]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn parse_ports_and_strengthens_the_upstream_json_refract_command_workflow() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/contracts/test-validate.apib")
  (let ((apib-drafter-executable "drafter")
        calls)
    (cl-letf
        (((symbol-function 'apib-compile-with-drafter)
          (lambda (filename &rest arguments)
            (push (cons filename arguments) calls)
            'parsed)))
      (list
       (apib-parse)
       (nreverse calls)
       (commandp 'apib-parse)))))"##;
    let expect = expect![[
        r#"OK (parsed (("/workspace/contracts/test-validate.apib" "-f" "json" "-u")) t)"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn valid_predicate_covers_success_failure_and_the_upstream_process_contract() {
    let elisp_form = r##"(with-temp-buffer
  (setq buffer-file-name "/workspace/contracts/orders.apib")
  (let ((apib-drafter-executable "/opt/drafter")
        (statuses '(0 1 127))
        calls)
    (cl-letf
        (((symbol-function 'call-process)
          (lambda (program infile destination display &rest arguments)
            (push
             (list program infile destination display arguments)
             calls)
            (pop statuses))))
      (list
       (apib-valid-p)
       (apib-valid-p)
       (apib-valid-p)
       (nreverse calls)
       (commandp 'apib-valid-p)))))"##;
    let expect = expect![[
        r#"OK (t nil nil (("/opt/drafter" "/workspace/contracts/orders.apib" nil nil ("-lu")) ("/opt/drafter" "/workspace/contracts/orders.apib" nil nil ("-lu")) ("/opt/drafter" "/workspace/contracts/orders.apib" nil nil ("-lu"))) t)"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn missing_drafter_warns_and_never_spawns_a_process_when_result_buffer_exists() {
    let elisp_form = r##"(let ((apib-drafter-executable nil)
      (apib-result-buffer "*apib-missing-drafter*")
      events)
  (get-buffer-create apib-result-buffer)
  (cl-letf
      (((symbol-function 'call-process)
        (lambda (&rest arguments)
          (push (cons 'unexpected-process arguments) events)
          0))
       ((symbol-function 'display-warning)
        (lambda (type message &rest arguments)
          (push (list 'warning type message arguments) events)
          'warned))
       ((symbol-function 'display-buffer)
        (lambda (&rest _arguments) nil)))
    (list
     (apib-compile-with-drafter "/workspace/not-run.apib" "-lu")
     (with-current-buffer apib-result-buffer major-mode)
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil compilation-mode ((warning apib-mode "drafter binary not found, please install it in your exec-path" nil)))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn error_filename_recovers_absolute_paths_with_spaces_from_drafter_command_lines() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "drafter -f json -u /workspace/API Specs/orders.apib\n"
   "warning: API description parse warning, line 3, column 2 - line 3, column 9\n")
  (goto-char (point-max))
  (list
   (apib-error-filename)
   (match-data)
   (progn
     (erase-buffer)
     (insert "ordinary compiler output\n")
     (goto-char (point-max))
     (apib-error-filename))))"##;
    let expect = expect![[r#"OK (("/workspace/API Specs/orders.apib") (0 70 62 70 63 70) nil)"#]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn compilation_regexp_extracts_start_and_end_locations_for_errors_and_warnings() {
    let elisp_form = r##"(let ((regexp
       (nth 1 (assq 'apib compilation-error-regexp-alist-alist))))
  (mapcar
   (lambda (line)
     (with-temp-buffer
       (insert "drafter -lu /workspace/contracts/orders.apib\n" line)
       (goto-char (point-max))
       (let ((matched (re-search-backward regexp nil t)))
         (list
          line
          (and matched
               (list
                (match-string 1)
                (match-string 2)
                (match-string 3)
                (match-string 4)
                (apib-error-filename)))))))
   '("error: API description parse error, line 8, column 3 - line 8, column 12"
     "warning: API description parse warning, line 2, column 1 - line 4, column 7"
     "note: no location")))"##;
    let expect = expect![[
        r#"OK (("error: API description parse error, line 8, column 3 - line 8, column 12" ("8" "3" "8" "12" ("/workspace/contracts/orders.apib"))) ("warning: API description parse warning, line 2, column 1 - line 4, column 7" ("2" "1" "4" "7" ("/workspace/contracts/orders.apib"))) ("note: no location" nil))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}
