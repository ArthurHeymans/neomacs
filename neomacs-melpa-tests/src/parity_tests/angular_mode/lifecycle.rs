use expect_test::expect;

use super::assert_angular_mode_parity;

#[test]
fn angular_mode_switching_between_js_html_and_fundamental_resets_local_state() {
    let elisp_form = r##"(let* ((directory
                          (file-name-directory
                           (getenv
                            "NEOMACS_PACKAGE_SOURCE")))
               (html-source
                (expand-file-name
                 "angular-html-mode.el"
                 directory)))
         (load html-source nil t t)
         (with-temp-buffer
           (angular-mode)
           (let ((javascript
                  (list
                   major-mode mode-name
                   comment-start
                   indent-line-function
                   (local-variable-p
                    'font-lock-keywords))))
             (angular-html-mode)
             (let ((html
                    (list
                     major-mode mode-name
                     comment-start
                     indent-line-function
                     (local-variable-p
                      'font-lock-defaults))))
               (fundamental-mode)
               (list
                javascript html
                (list
                 major-mode mode-name
                 comment-start
                 indent-line-function
                 (local-variable-p
                  'font-lock-keywords)
                 (local-variable-p
                  'font-lock-defaults)))))))"##;
    let expect = expect![[
        r#"OK ((angular-mode "JavaScript[Angular]" "// " js-indent-line t) (angular-html-mode "HTML[Angular]" "<!-- " sgml-indent-line t) (fundamental-mode "Fundamental" nil indent-relative nil nil))"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_hooks_run_once_with_fully_initialized_buffer_contract() {
    let elisp_form = r##"(let* ((events nil)
                (angular-mode-hook
                 (list
                  (lambda ()
                    (push
                     (list
                      'angular
                      major-mode mode-name
                      (derived-mode-p
                       'javascript-mode))
                     events)))))
         (with-temp-buffer
           (angular-mode)
           (angular-mode)
           (list
            (nreverse events)
            major-mode mode-name)))"##;
    let expect = expect![[
        r#"OK (((angular angular-mode "JavaScript[Angular]" javascript-mode) (angular angular-mode "JavaScript[Angular]" javascript-mode)) angular-mode "JavaScript[Angular]")"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_reentry_does_not_duplicate_custom_font_lock_rules() {
    let elisp_form = r##"(with-temp-buffer
         (angular-mode)
         (font-lock-set-defaults)
         (let ((first
                (copy-tree
                 font-lock-keywords))
               (first-counts
                (mapcar
                 (lambda (rule)
                   (cl-count
                    rule font-lock-keywords
                    :test #'equal))
                 angular-font-lock-keywords)))
           (angular-mode)
           (font-lock-set-defaults)
           (list
            (length first)
            first-counts
            (length font-lock-keywords)
            (mapcar
             (lambda (rule)
               (cl-count
                rule font-lock-keywords
                :test #'equal))
             angular-font-lock-keywords)
            (equal first
                   font-lock-keywords))))"##;
    let expect = expect!["OK (36 (0 0 0 0 0) 36 (0 0 0 0 0) t)"];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_source_reload_rebuilds_keyword_regexps_from_public_tables() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (before
                (list
                 (length
                  angular-font-lock-keywords)
                 (secure-hash
                  'sha256
                  (prin1-to-string
                   angular-font-lock-keywords)))))
         (setq angular-global-api-keywords
               '("custom.angular.api")
               angular-services-keywords
               '("$customService")
               angular-mocha-keywords
               '("specify(")
               angular-directive-definition-keywords
               '("customDirective:"))
         (load source nil t t)
         (list
          before
          angular-global-api-keywords
          angular-services-keywords
          angular-mocha-keywords
          angular-directive-definition-keywords
          (length
           angular-font-lock-keywords)
          (secure-hash
           'sha256
           (prin1-to-string
            angular-font-lock-keywords))))"##;
    let expect = expect![[
        r#"OK ((5 "29cc67207d868c397b1e225e520fb1967cdf6ed643fe0f7ea681aefd7fbe3608") ("custom.angular.api") ("$customService") ("specify(") ("customDirective:") 5 "29cc67207d868c397b1e225e520fb1967cdf6ed643fe0f7ea681aefd7fbe3608")"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}
