use expect_test::expect;

use super::assert_apiwrap_parity;

#[test]
fn apiwrap_upstream_resolution_matrix_covers_root_middle_and_trailing_parameters() {
    let elisp_form = r##"(let ((object
                '((name . "Hello-World")
                  (owner (login . "octocat")))))
         (mapcar
          (lambda (url)
            (let ((form (apiwrap-genform-resolve-api-params object url)))
              (list url form (eval form))))
          '("/repos/:owner.login/:name/issues"
            "/repos/:owner.login/:name"
            "/:owner.login/:name/issues"
            "/:owner.login/:name"
            ":owner.login"
            "/:owner.login"
            "/:owner.login/")))"##;
    let expect = expect![[
        r#"OK (("/repos/:owner.login/:name/issues" (let ((alist '#1=((name . "Hello-World") (owner (login . "octocat"))))) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist))))) (.name (cdr (assq 'name alist)))) (format "/repos/%s/%s/issues" (apiwrap--encode-url .owner.login) (apiwrap--encode-url .name)))) "/repos/octocat/Hello-World/issues") ("/repos/:owner.login/:name" (let ((alist '#1#)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist))))) (.name (cdr (assq 'name alist)))) (format "/repos/%s/%s" (apiwrap--encode-url .owner.login) (apiwrap--encode-url .name)))) "/repos/octocat/Hello-World") ("/:owner.login/:name/issues" (let ((alist '#1#)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist))))) (.name (cdr (assq 'name alist)))) (format "/%s/%s/issues" (apiwrap--encode-url .owner.login) (apiwrap--encode-url .name)))) "/octocat/Hello-World/issues") ("/:owner.login/:name" (let ((alist '#1#)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist))))) (.name (cdr (assq 'name alist)))) (format "/%s/%s" (apiwrap--encode-url .owner.login) (apiwrap--encode-url .name)))) "/octocat/Hello-World") (":owner.login" (let ((alist '#1#)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist)))))) (format "%s" (apiwrap--encode-url .owner.login)))) "octocat") ("/:owner.login" (let ((alist '#1#)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist)))))) (format "/%s" (apiwrap--encode-url .owner.login)))) "/octocat") ("/:owner.login/" (let ((alist '#1#)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist)))))) (format "/%s/" (apiwrap--encode-url .owner.login)))) "/octocat/"))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_uses_live_symbol_object_values_at_evaluation_time() {
    let elisp_form = r##"(let* ((form (apiwrap-genform-resolve-api-params
                       'repo
                       "/repos/:owner.login/:name/issues"))
              (first
               (let ((repo '((owner (login . "alpha"))
                             (name . "one"))))
                 (eval `(let ((repo ',repo)) ,form))))
              (second
               (let ((repo '((owner (login . "beta"))
                             (name . "two"))))
                 (eval `(let ((repo ',repo)) ,form)))))
         (list form first second))"##;
    let expect = expect![[
        r#"OK ((let ((alist repo)) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist))))) (.name (cdr (assq 'name alist)))) (format "/repos/%s/%s/issues" (apiwrap--encode-url .owner.login) (apiwrap--encode-url .name)))) "/repos/alpha/one/issues" "/repos/beta/two/issues")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_combines_named_objects_and_numeric_identifiers() {
    let elisp_form = r##"(let* ((objects
                '((repo
                   (owner (login . "gnu-emacs"))
                   (name . "emacs"))
                  (issue (number . 163))))
              (url
               "/repos/:repo.owner.login/:repo.name/issues/:issue.number/comments")
              (form (apiwrap-genform-resolve-api-params objects url)))
         (list form (eval form)))"##;
    let expect = expect![[
        r#"OK ((let ((alist '((repo (owner (login . "gnu-emacs")) (name . "emacs")) (issue (number . 163))))) (let ((.repo.owner.login (cdr (assq 'login (cdr (assq 'owner (cdr (assq 'repo alist))))))) (.repo.name (cdr (assq 'name (cdr (assq 'repo alist))))) (.issue.number (cdr (assq 'number (cdr (assq 'issue alist)))))) (format "/repos/%s/%s/issues/%s/comments" (apiwrap--encode-url .repo.owner.login) (apiwrap--encode-url .repo.name) (apiwrap--encode-url .issue.number)))) "/repos/gnu-emacs/emacs/issues/163/comments")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_percent_encodes_real_world_path_segments() {
    let elisp_form = r##"(let ((object
                '((owner (login . "team docs"))
                  (name . "api^wrap/日本語")
                  (label . "bug + help"))))
         (mapcar
          (lambda (url)
            (eval (apiwrap-genform-resolve-api-params object url)))
          '("/repos/:owner.login/:name"
            "/labels/:label"
            "/:name/:label/")))"##;
    let expect = expect![[
        r#"OK ("/repos/team%20docs/api%5Ewrap/%E6%97%A5%E6%9C%AC%E8%AA%9E" "/labels/bug%20+%20help" "/api%5Ewrap/%E6%97%A5%E6%9C%AC%E8%AA%9E/bug%20+%20help/")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_encode_url_distinguishes_numbers_strings_empty_and_unicode() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (list value (apiwrap--encode-url value)))
         '(0 -17 3.5 "" "plain" "a b+c/d?" "λ-日本語"))"##;
    let expect = expect![[
        r#"OK ((0 "0") (-17 "-17") (3.5 "3.5") ("" "") ("plain" "plain") ("a b+c/d?" "a%20b+c/d?") ("λ-日本語" "%CE%BB-%E6%97%A5%E6%9C%AC%E8%AA%9E"))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_recognizes_only_documented_parameter_alphabet() {
    let elisp_form = r##"(let ((object
                '((valid-name . "ok")
                  (under_score . "not-used")
                  (digits123 . "not-used")
                  (nested (key . "yes")))))
         (mapcar
          (lambda (url)
            (let ((form (apiwrap-genform-resolve-api-params object url)))
              (list url form
                    (condition-case error
                        (eval form)
                      (error (list 'error (car error)))))))
          '("/:valid-name"
            "/:under_score"
            "/:digits123"
            "/:nested.key"
            "/literal:valid-name-tail")))"##;
    let expect = expect![[
        r#"OK (("/:valid-name" (let ((alist '#1=((valid-name . "ok") (under_score . "not-used") (digits123 . "not-used") (nested (key . "yes"))))) (let ((.valid-name (cdr (assq 'valid-name alist)))) (format "/%s" (apiwrap--encode-url .valid-name)))) "/ok") ("/:under_score" (let ((alist '#1#)) (let nil (format "/:under_score"))) "/:under_score") ("/:digits123" (let ((alist '#1#)) (let nil (format "/:digits123"))) "/:digits123") ("/:nested.key" (let ((alist '#1#)) (let ((.nested.key (cdr (assq 'key (cdr (assq 'nested alist)))))) (format "/%s" (apiwrap--encode-url .nested.key)))) "/yes") ("/literal:valid-name-tail" (let ((alist '#1#)) (let ((.valid-name-tail (cdr (assq 'valid-name-tail alist)))) (format "/literal%s" (apiwrap--encode-url .valid-name-tail)))) "/literal/"))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_without_parameters_returns_plain_format_form() {
    let elisp_form = r##"(mapcar
         (lambda (object)
           (let ((form
                  (apiwrap-genform-resolve-api-params
                   object
                   "/repos/static/issues?state=open")))
             (list object
                   form
                   (eval form
                         '((ignored . ((name . "bound-but-unused"))))))))
         '(nil ignored ((name . "unused"))))"##;
    let expect = expect![[
        r#"OK ((nil (format "/repos/static/issues?state=open") "/repos/static/issues?state=open") (ignored (let ((alist ignored)) (let nil (format "/repos/static/issues?state=open"))) "/repos/static/issues?state=open") (#1=((name . "unused")) (let ((alist '#1#)) (let nil (format "/repos/static/issues?state=open"))) "/repos/static/issues?state=open"))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_does_not_leak_or_modify_caller_match_data() {
    let elisp_form = r##"(progn
         (string-match "\\(alpha\\)-\\(beta\\)" "alpha-beta")
         (let ((before (match-data))
               (before-one (match-string 1 "alpha-beta")))
           (let ((form
                  (apiwrap-genform-resolve-api-params
                   '((owner (login . "octocat")))
                   "/users/:owner.login/repos")))
             (list before
                   before-one
                   form
                   (eval form)
                   (match-data)
                   (match-string 2 "alpha-beta")))))"##;
    let expect = expect![[
        r#"OK ((0 10 0 5 6 10) "alpha" (let ((alist '((owner (login . "octocat"))))) (let ((.owner.login (cdr (assq 'login (cdr (assq 'owner alist)))))) (format "/users/%s/repos" (apiwrap--encode-url .owner.login)))) "/users/octocat/repos" (0 10 0 5 6 10) "beta")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_form_reuses_repeated_and_nested_parameters_in_order() {
    let elisp_form = r##"(let* ((object
                '((org (login . "gnu"))
                  (repo (name . "emacs"))
                  (branch . "feature one")))
              (form
               (apiwrap-genform-resolve-api-params
                object
                "/:org.login/:repo.name/compare/:branch...:branch")))
         (list form (eval form)))"##;
    let expect = expect![[
        r#"OK ((let ((alist '((org (login . "gnu")) (repo (name . "emacs")) (branch . "feature one")))) (let ((.org.login (cdr (assq 'login (cdr (assq 'org alist))))) (.repo.name (cdr (assq 'name (cdr (assq 'repo alist))))) (.branch (cdr (assq 'branch alist)))) (format "/%s/%s/compare/:branch...%s" (apiwrap--encode-url .org.login) (apiwrap--encode-url .repo.name) (apiwrap--encode-url .branch)))) "/gnu/emacs/compare/:branch...feature%20one")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_resolution_reflects_missing_nested_values_as_native_url_error() {
    let elisp_form = r##"(mapcar
         (lambda (object)
           (condition-case error
               (eval
                (apiwrap-genform-resolve-api-params
                 object
                 "/repos/:owner.login/:name"))
             (error
              (list (car error)
                    (mapcar
                     (lambda (value)
                       (if (stringp value)
                           (replace-regexp-in-string
                            "0x[0-9a-f]+" "<address>" value)
                         value))
                     (cdr error))))))
         '(((name . "known"))
           ((owner (login . "known")))
           nil))"##;
    let expect =
        expect![[r#"OK ("/repos///known" "/repos/known//" (void-variable (.owner.login)))"#]];
    assert_apiwrap_parity(elisp_form, expect);
}
