use expect_test::expect;

use super::assert_age_parity;

#[test]
fn age_configuration_parses_semver_and_uses_documented_fallback() {
    let elisp_form = r##"(let (commands messages)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (command)
                      (push command commands)
                      (if (string-match-p "rage" command)
                          "rage 0.9.2\n"
                        "development build\n")))
                   ((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages))))
           (list
            (age-config--make-age-configuration "/opt/bin/rage")
            (age-config--make-age-configuration "/opt/bin/age")
            (nreverse commands)
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (((program . "/opt/bin/rage") (version . "0.9.2")) ((program . "/opt/bin/age") (version . "9.9.9")) ("/opt/bin/rage --version" "/opt/bin/age --version") ("WARNING: age.el could not determine version for /opt/bin/age, falling back to 9.9.9"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_configuration_wrapper_uses_current_program_value_for_fresh_probe() {
    let elisp_form = r##"(let ((age-program "/custom/bin/age")
               calls)
         (cl-letf (((symbol-function
                     'age-config--make-age-configuration)
                    (lambda (program)
                      (push program calls)
                      `((program . ,program)
                        (version . "1.9.0")))))
           (list
            (age-configuration)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (((program . "/custom/bin/age") (version . "1.9.0")) ("/custom/bin/age"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_check_configuration_handles_minimum_ranges_alternatives_and_errors() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (pcase-let ((`(,version ,requirements) case))
             (list
              case
              (condition-case error-data
                  (age-check-configuration
                   `((program . "age")
                     (version . ,version))
                   requirements)
                (error
                 (list (car error-data)
                       (cadr error-data)))))))
         '(("1.0.0" nil)
           ("1.0.0" "1.0.0")
           ("1.2.3" "1.1.0")
           ("0.9.9" "1.0.0")
           ("1.5.0" (("1.0.0" . "1.5.0")))
           ("1.4.9" (("1.0.0" . "1.5.0")))
           ("2.1.0" (("1.0.0" . "1.5.0")
                     ("2.0.0" . "3.0.0")))
           (nil "1.0.0")))"##;
    let expect = expect![[
        r#"OK ((("1.0.0" nil) (error "Unsupported version: 1.0.0")) (("1.0.0" "1.0.0") t) (("1.2.3" "1.1.0") t) (("0.9.9" "1.0.0") (error "Unsupported version: 0.9.9")) (("1.5.0" (("1.0.0" . "1.5.0"))) (error "Unsupported version: 1.5.0")) (("1.4.9" (("1.0.0" . "1.5.0"))) t) (("2.1.0" (("1.0.0" . "1.5.0") ("2.0.0" . "3.0.0"))) t) ((nil "1.0.0") (error "Undetermined version: nil")))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_find_configuration_prefers_customized_program_and_caches_it() {
    let elisp_form = r##"(let ((age--configurations nil)
               (age-program "/custom/rage")
               constructor-calls
               checks)
         (put 'age-program 'customized-value '("/custom/rage"))
         (unwind-protect
             (cl-letf (((symbol-function
                         'age-config--make-age-configuration)
                        (lambda (program)
                          (push program constructor-calls)
                          `((program . ,program)
                            (version . "9.1.0"))))
                       ((symbol-function 'age-check-configuration)
                        (lambda (&rest arguments)
                          (push arguments checks)
                          t)))
               (let ((first (age-find-configuration 'Age))
                     (second (age-find-configuration 'Age)))
                 (list first
                       second
                       age--configurations
                       (nreverse constructor-calls)
                       checks)))
           (put 'age-program 'customized-value nil)))"##;
    let expect = expect![[
        r#"OK (#1=((program . "/custom/rage") (version . "9.1.0")) #1# ((Age . #1#)) ("/custom/rage") nil)"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_find_configuration_searches_candidates_rejects_old_versions_and_honors_no_cache() {
    let elisp_form = r##"(let ((age--configurations nil)
               executables constructors checks)
         (cl-letf (((symbol-function 'executable-find)
                    (lambda (program)
                      (push program executables)
                      (concat "/bin/" program)))
                   ((symbol-function
                     'age-config--make-age-configuration)
                    (lambda (program)
                      (push program constructors)
                      `((program . ,program)
                        (version . ,(if (string-match-p "rage" program)
                                        "0.8.0"
                                      "1.2.0")))))
                   ((symbol-function 'age-check-configuration)
                    (lambda (configuration required)
                      (push (list configuration required) checks)
                      (unless (version<= required
                                         (alist-get 'version configuration))
                        (error "too old"))
                      t)))
           (let ((first (age-find-configuration 'Age))
                 (cached (age-find-configuration 'Age))
                 (fresh (age-find-configuration 'Age t)))
             (list first
                   cached
                   fresh
                   age--configurations
                   (nreverse executables)
                   (nreverse constructors)
                   (nreverse checks)))))"##;
    let expect = expect![[
        r#"OK (#1=((program . "/bin/age") (version . "1.2.0")) #1# #2=((program . "/bin/age") (version . "1.2.0")) ((Age . #1#)) ("rage" "age" "rage" "age") ("/bin/rage" "/bin/age" "/bin/rage" "/bin/age") ((((program . "/bin/rage") (version . "0.8.0")) "0.9.0") (#1# "1.0.0") (((program . "/bin/rage") (version . "0.8.0")) "0.9.0") (#2# "1.0.0")))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_find_configuration_reports_unknown_protocol_and_no_usable_candidate() {
    let elisp_form = r##"(let ((age--configurations nil))
         (cl-letf (((symbol-function 'executable-find)
                    (lambda (_program) nil)))
           (list
            (condition-case error-data
                (age-find-configuration 'OpenPGP)
              (error
               (list (car error-data) (cadr error-data))))
            (age-find-configuration 'Age)
            age--configurations)))"##;
    let expect = expect![[r#"OK ((error "Unknown protocol ‘OpenPGP’") nil nil)"#]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_required_version_uses_protocol_configuration_for_threshold_checks() {
    let elisp_form = r##"(let (protocols)
         (cl-letf (((symbol-function 'age-find-configuration)
                    (lambda (protocol)
                      (push protocol protocols)
                      '((program . "age")
                        (version . "1.3.2")))))
           (list
            (age-required-version-p 'Age "1.0.0")
            (age-required-version-p 'Age "1.3.2")
            (age-required-version-p 'Age "1.3.3")
            (nreverse protocols))))"##;
    let expect = expect!["OK (t t nil (Age Age Age))"];
    assert_age_parity(elisp_form, expect);
}
