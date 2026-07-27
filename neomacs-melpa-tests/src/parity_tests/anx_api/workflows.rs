use expect_test::expect;

use super::assert_anx_api_parity;

#[test]
fn anx_api_send_buffer_reads_lisp_payload_and_routes_verb_service_and_popup() {
    let elisp_form = r##"(with-temp-buffer
         (rename-buffer "campaign-update" t)
         (insert "(:campaign (:id 9 :name \"Launch\" :active :json-false))")
         (let ((*anx-current-url* "https://api.example")
               calls)
           (cl-letf (((symbol-function 'anx--send-request)
                      (lambda (&rest args)
                        (push (cons 'send args) calls)
                        '(:response (:status "OK"))))
                     ((symbol-function 'anx--pop-up-buffer)
                      (lambda (&rest args)
                        (push (cons 'popup args) calls)
                        'shown)))
             (list (anx-send-buffer "PUT" "campaign/9")
                   (nreverse calls)
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (shown ((send "PUT" "campaign/9" (:campaign (:id 9 :name "Launch" :active :json-false))) (popup "https://api.example/campaign/9[PUT]" (:response (:status "OK")) emacs-lisp-mode)) "(:campaign (:id 9 :name \"Launch\" :active :json-false))")"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_credentials_command_updates_username_and_password_from_prompt() {
    let elisp_form = r##"(let ((anx-username nil)
               (anx-password nil)
               calls)
         (cl-letf (((symbol-function 'read-passwd)
                    (lambda (&rest args)
                      (push args calls)
                      "s3cr3t")))
           (list
            (anx-get-user-authentication-credentials "alice@example")
            anx-username anx-password
            *anx-authentication-credentials*
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("s3cr3t" "alice@example" "s3cr3t" (:auth (:username nil :password nil)) (("password: ")))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_toggle_cycles_sandbox_production_and_unknown_back_to_sandbox() {
    let elisp_form = r##"(let ((*anx-sandbox-url* "https://sandbox")
               (*anx-production-url* "https://production"))
         (let ((*anx-current-url* "https://sandbox"))
           (list
            (progn (anx-toggle-current-api-url) *anx-current-url*)
            (progn (anx-toggle-current-api-url) *anx-current-url*)
            (progn
              (setq *anx-current-url* "https://custom")
              (anx-toggle-current-api-url)
              *anx-current-url*))))"##;
    let expect = expect![[r#"OK ("https://production" "https://sandbox" "https://sandbox")"#]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_display_current_url_formats_exact_message_and_return_value() {
    let elisp_form = r##"(let ((*anx-current-url* "https://sandbox.example/v1")
               calls)
         (cl-letf (((symbol-function 'message)
                    (lambda (&rest args)
                      (push args calls)
                      (apply #'format args))))
           (list (anx-display-current-api-url)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("current api url is https://sandbox.example/v1" (("current api url is %s" "https://sandbox.example/v1")))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_browse_docs_builds_real_query_from_symbol_at_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "campaign")
         (goto-char 4)
         (let (calls)
           (cl-letf (((symbol-function 'browse-url)
                      (lambda (&rest args)
                        (push args calls)
                        'opened)))
             (list (anx-browse-api-docs)
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (opened (("https://wiki.appnexus.com/dosearchsite.action?searchQuery.spaceKey=api&searchQuery.queryString=ancestorIds%3A27984339+AND+campaign")))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_browse_docs_without_symbol_preserves_native_signal() {
    let elisp_form = r##"(with-temp-buffer
         (insert "   ")
         (goto-char 2)
         (condition-case err
             (anx-browse-api-docs)
           (error (list (car err) (cdr err)))))"##;
    let expect = expect!["OK nil"];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_save_buffer_builds_deterministic_informative_filename() {
    let elisp_form = r##"(with-temp-buffer
         (rename-buffer "https://api.example/member/7?stats=true" t)
         (insert "payload")
         (let ((anx-save-directory "/archive/anx/")
               calls)
           (cl-letf (((symbol-function 'current-time-string)
                      (lambda ()
                        "Mon Jan 02 03:04:05 2006"))
                     ((symbol-function 'write-file)
                      (lambda (&rest args)
                        (push args calls)
                        'saved)))
             (list (anx-save-buffer-contents)
                   (nreverse calls)
                   (buffer-name)
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (saved (("/archive/anx/https:__api.example_member_7?stats=true_Mon_Jan_02_03:04:05_2006")) "https://api.example/member/7?stats=true" "payload")"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_default_load_leaves_all_documented_global_keys_unbound() {
    let elisp_form = r##"(mapcar
         (lambda (key)
           (list key (key-binding (kbd key))))
         '("C-x C-a A" "C-x C-a a" "C-x C-a S"
           "C-x C-a W" "C-x C-a w" "C-x C-a T"
           "C-x C-a J" "C-x C-a L" "C-x C-a P"
           "C-x C-a G" "C-x C-a g" "C-x C-a D"
           "C-x C-a U" "C-x C-a E" "C-x C-a s"
           "C-x C-a d"))"##;
    let expect = expect![[
        r#"OK (("C-x C-a A" nil) ("C-x C-a a" nil) ("C-x C-a S" nil) ("C-x C-a W" nil) ("C-x C-a w" nil) ("C-x C-a T" nil) ("C-x C-a J" nil) ("C-x C-a L" nil) ("C-x C-a P" nil) ("C-x C-a G" nil) ("C-x C-a g" nil) ("C-x C-a D" nil) ("C-x C-a U" nil) ("C-x C-a E" nil) ("C-x C-a s" nil) ("C-x C-a d" nil))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_opt_in_reload_installs_every_documented_global_binding() {
    let elisp_form = r##"(let ((anx-use-global-keybindings t)
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (mapcar
          (lambda (key)
            (list key (key-binding (kbd key))))
          '("C-x C-a A" "C-x C-a a" "C-x C-a S"
            "C-x C-a W" "C-x C-a w" "C-x C-a T"
            "C-x C-a J" "C-x C-a L" "C-x C-a P"
            "C-x C-a G" "C-x C-a g" "C-x C-a D"
            "C-x C-a U" "C-x C-a E" "C-x C-a s"
            "C-x C-a d")))"##;
    let expect = expect![[
        r#"OK (("C-x C-a A" anx-authenticate) ("C-x C-a a" anx-get-user-authentication-credentials) ("C-x C-a S" anx-switch-users) ("C-x C-a W" anx-who-am-i) ("C-x C-a w" anx-display-current-api-url) ("C-x C-a T" anx-toggle-current-api-url) ("C-x C-a J" anx-lisp-to-json) ("C-x C-a L" anx-json-to-lisp) ("C-x C-a P" anx-send-buffer) ("C-x C-a G" anx-get) ("C-x C-a g" anx-raw-get) ("C-x C-a D" anx-delete) ("C-x C-a U" anx-unescape-json) ("C-x C-a E" anx-escape-json) ("C-x C-a s" anx-save-buffer-contents) ("C-x C-a d" anx-browse-api-docs))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}
