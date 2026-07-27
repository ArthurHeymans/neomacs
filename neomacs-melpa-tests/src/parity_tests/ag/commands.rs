use expect_test::expect;

use super::assert_ag_parity;

#[test]
fn ag_public_search_commands_delegate_complete_literal_regexp_project_and_file_options() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ag/search)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      (cons 'search arguments)))
                   ((symbol-function 'ag/project-root)
                    (lambda (directory)
                      (push (list 'project-root directory) calls)
                      "/detected/project/")))
           (let ((default-directory "/work/current/"))
             (list
              (ag "literal" "/chosen/")
              (ag-files
               "literal-files"
               '(:file-type "rust")
               "/chosen/")
              (ag-regexp "r(e+)" "/chosen/")
              (ag-project "project literal")
              (ag-project-files
               "project files"
               '(:file-regex "\\.el$"))
              (ag-project-regexp "project.*regexp")
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((search . #1=("literal" "/chosen/")) (search . #2=("literal-files" "/chosen/" :file-type "rust")) (search . #3=("r(e+)" "/chosen/" :regexp t)) (search . #4=("project literal" "/detected/project/")) (search . #5=("project files" "/detected/project/" :file-regex "\\.el$")) (search . #6=("project.*regexp" "/detected/project/" :regexp t)) (#1# #2# #3# (project-root "/work/current/") #4# (project-root "/work/current/") #5# (project-root "/work/current/") #6#))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_read_from_minibuffer_builds_dwim_prompt_history_and_empty_input_fallback() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ag/dwim-at-point)
                    (lambda () "alpha-beta"))
                   ((symbol-function 'read-from-minibuffer)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      (if (= (length calls) 1)
                          ""
                        "explicit"))))
           (let ((first
                  (ag/read-from-minibuffer "Search string"))
                 (second
                  (ag/read-from-minibuffer "Search regexp")))
             (cl-letf (((symbol-function 'ag/dwim-at-point)
                        (lambda () nil))
                       ((symbol-function 'read-from-minibuffer)
                        (lambda (&rest arguments)
                          (push arguments calls)
                          "")))
               (list
                first
                second
                (ag/read-from-minibuffer "No suggestion")
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK ("alpha-beta" "explicit" nil (("Search string (default alpha-beta): " nil nil nil nil "alpha-beta") ("Search regexp (default alpha-beta): " nil nil nil nil "alpha-beta") ("No suggestion: " nil nil nil nil nil)))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_supported_types_parses_realistic_external_output_into_extension_groups() {
    let elisp_form = r##"(let (commands)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (command)
                      (push command commands)
                      "--rust\n.rs  .rlib\n--web\n.html  .htm  .css\nnoise\n--elisp\n.el\n")))
           (let ((ag-executable "/opt/ag binary"))
             (list
              (ag/get-supported-types)
              (nreverse commands)))))"##;
    let expect = expect![[
        r#"OK ((("rust" ".rs" ".rlib") ("web" ".html" ".htm" ".css") ("elisp" ".el")) ("/opt/ag binary --list-file-types"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_read_file_type_handles_known_type_and_custom_pcre_from_current_extension() {
    let elisp_form = r##"(let (completion-calls minibuffer-calls)
         (cl-letf (((symbol-function 'ag/get-supported-types)
                    (lambda ()
                      '(("rust" ".rs" ".rlib")
                        ("elisp" ".el"))))
                   ((symbol-function 'completing-read)
                    (lambda (&rest arguments)
                      (push arguments completion-calls)
                      (if (= (length completion-calls) 1)
                          "rust"
                        "custom (provide a PCRE regex)")))
                   ((symbol-function 'read-from-minibuffer)
                    (lambda (&rest arguments)
                      (push arguments minibuffer-calls)
                      "\\.generated\\.el$")))
           (with-temp-buffer
             (setq buffer-file-name
                   "/work/source.generated.el")
             (list
              (ag/read-file-type)
              (ag/read-file-type)
              (nreverse completion-calls)
              (nreverse minibuffer-calls)))))"##;
    let expect = expect![[
        r#"OK ((:file-type "rust") (:file-regex "\\.generated\\.el$") (("Select file type: " ("custom (provide a PCRE regex)" "rust" "elisp")) ("Select file type: " ("custom (provide a PCRE regex)" "rust" "elisp"))) (("Filenames which match PCRE: " "\\.el$")))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_kill_buffer_commands_operate_on_real_mode_buffers_and_preserve_unrelated_buffers() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *ag-first*"))
               (second (generate-new-buffer " *ag-second*"))
               (plain (generate-new-buffer " *ag-plain*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (ag-mode))
               (with-current-buffer second
                 (ag-mode))
               (with-current-buffer plain
                 (fundamental-mode))
               (with-current-buffer first
                 (ag-kill-other-buffers))
               (let ((after-other
                      (list
                       (buffer-live-p first)
                       (buffer-live-p second)
                       (buffer-live-p plain))))
                 (ag-kill-buffers)
                 (list
                  after-other
                  (buffer-live-p first)
                  (buffer-live-p second)
                  (buffer-live-p plain))))
           (dolist (buffer (list first second plain))
             (when (buffer-live-p buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect!["OK ((t nil t) nil nil t)"];
    assert_ag_parity(elisp_form, expect);
}
