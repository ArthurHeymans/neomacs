use expect_test::expect;

use super::{assert_anx_api_autoload_parity, assert_anx_api_parity};

#[test]
fn anx_api_registers_exact_feature_group_variables_and_complete_function_surface() {
    let elisp_form = r##"(list
         (featurep 'anx-api)
         (get 'anx 'custom-group)
         (mapcar
          (lambda (symbol)
            (list symbol (boundp symbol)
                  (and (boundp symbol) (symbol-value symbol))
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)))
          '(anx-username anx-password anx-save-directory
            anx-use-global-keybindings *anx-authentication-credentials*
            *anx-production-url* *anx-sandbox-url*
            *anx-current-url*))
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol) (commandp symbol)
                  (help-function-arglist symbol t)))
          '(anx--parse-response anx--send-request anx--pop-up-buffer
            anx-authenticate anx-lisp-to-json anx-escape-json
            anx-unescape-json anx-json-to-lisp anx-send-buffer
            anx-get anx-raw-get anx-delete anx-switch-users
            anx-who-am-i anx-browse-api-docs
            anx-get-user-authentication-credentials
            anx-display-current-api-url anx-toggle-current-api-url
            anx-save-buffer-contents)))"##;
    let expect = expect![[
        r#"OK (t ((anx-username custom-variable) (anx-password custom-variable) (anx-save-directory custom-variable) (anx-use-global-keybindings custom-variable)) ((anx-username t nil (string) nil) (anx-password t nil (string) nil) (anx-save-directory t nil (string) nil) (anx-use-global-keybindings t nil (string) nil) (*anx-authentication-credentials* t (:auth (:username nil :password nil)) nil nil) (*anx-production-url* t "http://api.appnexus.com" nil nil) (*anx-sandbox-url* t "http://sand.api.appnexus.com" nil nil) (*anx-current-url* t "http://sand.api.appnexus.com" nil nil)) ((anx--parse-response t nil (buffer)) (anx--send-request t nil (verb path &optional payload)) (anx--pop-up-buffer t nil (bufname stuff mode)) (anx-authenticate t t nil) (anx-lisp-to-json t t nil) (anx-escape-json t t nil) (anx-unescape-json t t nil) (anx-json-to-lisp t t nil) (anx-send-buffer t t (verb service-and-params)) (anx-get t t (service-and-params)) (anx-raw-get t t (url)) (anx-delete t t (service-and-params)) (anx-switch-users t t (user-id)) (anx-who-am-i t t nil) (anx-browse-api-docs t t nil) (anx-get-user-authentication-credentials t t (username)) (anx-display-current-api-url t t nil) (anx-toggle-current-api-url t t nil) (anx-save-buffer-contents t t nil)))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_initial_credentials_capture_custom_defaults_at_load_time() {
    let elisp_form = r##"(list
         *anx-authentication-credentials*
         (eq (plist-get *anx-authentication-credentials* :auth)
             (plist-get *anx-authentication-credentials* :auth))
         (plist-get
          (plist-get *anx-authentication-credentials* :auth)
          :username)
         (plist-get
          (plist-get *anx-authentication-credentials* :auth)
          :password)
         anx-username anx-password)"##;
    let expect = expect!["OK ((:auth (:username nil :password nil)) t nil nil nil nil)"];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_installs_cookie_trust_as_source_buffer_local_state() {
    let elisp_form = r##"(let ((source-buffer (get-file-buffer
                              (getenv "NEOMACS_PACKAGE_SOURCE"))))
         (list
          url-cookie-trusted-urls
          (local-variable-p 'url-cookie-trusted-urls)
          (and source-buffer
               (buffer-local-value
                'url-cookie-trusted-urls source-buffer))
          (and source-buffer
               (with-current-buffer source-buffer
                 (local-variable-p 'url-cookie-trusted-urls)))))"##;
    let expect = expect![[r#"OK ((".*adnxs.net" ".*adnxs.com" ".*appnexus.com") t nil nil)"#]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_descriptor_records_exact_pin_and_installed_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'anx-api package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (let ((relative (file-relative-name file directory)))
                (list relative
                      (file-attribute-size (file-attributes file))
                      (secure-hash 'sha256 file))))
            (directory-files-recursively directory "." nil))
           (lambda (a b) (string< (car a) (car b))))))"##;
    let expect = expect![[
        r#"OK (anx-api "20140208.1514" nil "Interact with the AppNexus API from Emacs." nil (("README-elpa" 3800 "eae36dad54a4f538ab17b00a6f256e3c4b74d1d1f2ac97d386f4c8194ab2bdb7") ("anx-api-autoloads.el" 681 "d0bd709a7406b3dce1c1d03bc6463af1e06061e905136b961546dc7e376b96d1") ("anx-api-pkg.el" 344 "845d767917c2c644a32d9a0e9769929e442bd3ae6efe6c7d05f650d58156c40f") ("anx-api.el" 18091 "f6311294eee125a4491f091025cbd6bc9c0e457bc39b826dec7d6c0ab3b7b49e") ("anx-api.elc" 11348 "e59899da11c2fd2e0aef28f94ce5ad48d873a82235d20031af0732ea14d554da")))"#
    ]];
    assert_anx_api_parity(elisp_form, expect);
}

#[test]
fn anx_api_autoloads_expose_only_authenticate_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'anx-api)
         (mapcar
          (lambda (symbol)
            (list symbol (fboundp symbol) (commandp symbol)
                  (and (fboundp symbol)
                       (autoloadp (symbol-function symbol)))
                  (and (fboundp symbol)
                       (symbol-function symbol))))
          '(anx-authenticate anx-get anx-lisp-to-json
            anx-toggle-current-api-url anx-save-buffer-contents))
         (boundp 'anx-username)
         (boundp '*anx-current-url*)
         (fboundp 'anx--send-request))"##;
    let expect = expect![
        "OK (nil ((anx-authenticate nil nil nil nil) (anx-get nil nil nil nil) (anx-lisp-to-json nil nil nil nil) (anx-toggle-current-api-url nil nil nil nil) (anx-save-buffer-contents nil nil nil nil)) nil nil nil)"
    ];
    assert_anx_api_autoload_parity(elisp_form, expect);
}

#[test]
fn anx_api_reload_preserves_custom_runtime_state_and_single_feature() {
    let elisp_form = r##"(let ((anx-username "alice")
               (anx-password "secret")
               (anx-save-directory "/save/")
               (*anx-current-url* "https://custom.invalid")
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list anx-username anx-password anx-save-directory
               *anx-current-url*
               (length
                (cl-remove-if-not
                 (lambda (feature) (eq feature 'anx-api))
                 features))))"##;
    let expect = expect![[r#"OK ("alice" "secret" "/save/" "https://custom.invalid" 1)"#]];
    assert_anx_api_parity(elisp_form, expect);
}
