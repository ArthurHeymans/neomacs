use expect_test::expect;

use super::assert_apiwrap_parity;

#[test]
fn apiwrap_gensym_normalizes_methods_and_real_endpoint_shapes() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (apply #'apiwrap-gensym case))
         '(("gh" get)
           ("gh" :post)
           ("client" delete "/issues")
           ("client" :patch "/repos/:owner/:repo/issues/:number")
           ("x" head "/users/:user.name/events")
           ("api-" put "/v1/items/")))"##;
    let expect = expect![
        "OK (defapiget-gh defapipost-gh client-delete-issues client-patch-repos-owner-repo-issues-number x-head-users-user.name-events api--put-v1-items-)"
    ];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_standard_link_returns_present_nil_and_duplicate_values() {
    let elisp_form = r##"(list
         (apiwrap-stdgenlink
          '((prefix . "x") (method . get)
            (link . "docs/issues") (endpoint . "/issues")))
         (apiwrap-stdgenlink '((endpoint . "/missing")))
         (apiwrap-stdgenlink
          '((link . "first") (link . "second"))))"##;
    let expect = expect![[r#"OK ("docs/issues" nil "first")"#]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_generated_function_documentation_is_exact_and_actionable() {
    let elisp_form = r##"(apiwrap--docfn
         "Git Forge"
         "Create an issue with labels and an optional milestone."
         "REPO is a repository object returned by the service."
         'post
         "/repos/:owner/:repo/issues"
         "https://docs.example/issues#create")"##;
    let expect = expect![[
        r#"OK "Create an issue with labels and an optional milestone.\n\nREPO is a repository object returned by the service.\n\nDATA is a data structure to be sent with this request.  If it's\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Git Forge API endpoint\n\n    POST /repos/:owner/:repo/issues\n\nwhich is documented at\n\n    URL `https://docs.example/issues#create'""#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_generated_function_documentation_omits_non_string_object_docs() {
    let elisp_form = r##"(mapcar
         (lambda (object-doc)
           (apiwrap--docfn
            "Inventory"
            "Fetch one item."
            object-doc
            'get
            "/items/:id"
            "https://docs.example/items"))
         '(nil missing 17))"##;
    let expect = expect![[
        r#"OK ("Fetch one item.\n\nDATA is a data structure to be sent with this request.  If it's\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Inventory API endpoint\n\n    GET /items/:id\n\nwhich is documented at\n\n    URL `https://docs.example/items'" "Fetch one item.\n\nDATA is a data structure to be sent with this request.  If it's\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Inventory API endpoint\n\n    GET /items/:id\n\nwhich is documented at\n\n    URL `https://docs.example/items'" "Fetch one item.\n\nDATA is a data structure to be sent with this request.  If it's\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Inventory API endpoint\n\n    GET /items/:id\n\nwhich is documented at\n\n    URL `https://docs.example/items'")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_generated_macro_documentation_covers_backend_and_override_contract() {
    let elisp_form = r##"(let ((doc (apiwrap--docmacro "Git Forge" 'patch)))
         (list (length doc)
               (substring doc 0 125)
               (string-match-p
                "defapiget-<prefix>.*List issues for a repository"
                doc)
               (string-match-p
                "CONFIG is a list of override configuration parameters"
                doc)
               (substring doc (- (length doc) 165))))"##;
    let expect = expect![[
        r#"OK (1781 "Define a new PATCH resource wrapper function.\n\nRESOURCE is the API endpoint as written in the Git Forge API\ndocumentation.  A" nil 1598 "f override configuration parameters.  Values\nset here (notably those explicitly set to nil) will take\nprecedence over the defaults provided to `apiwrap-new-backend'.")"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_maybe_apply_generates_direct_and_preprocessor_forms() {
    let elisp_form = r##"(list
         (apiwrap--maybe-apply nil '(cons data params))
         (apiwrap--maybe-apply 'identity '(cons data params))
         (apiwrap--maybe-apply #'identity 'params)
         (apiwrap--maybe-apply
          '(lambda (value) (list :wrapped value))
          'data))"##;
    let expect = expect![
        "OK ((cons data params) (funcall identity (cons data params)) (funcall identity params) (funcall (lambda (value) (list :wrapped value)) data))"
    ];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_genmacros_builds_six_primitive_macros_with_exact_signatures() {
    let elisp_form = r##"(let ((forms
                (apiwrap-genmacros
                 "Forge" "forge"
                 '((repo . "REPO docs"))
                 '((request . ignore)
                   (link . apiwrap-stdgenlink)))))
         (mapcar
          (lambda (form)
            (list (nth 1 form)
                  (nth 2 form)
                  (substring (nth 3 form) 0 31)
                  (nth 4 form)
                  (car (nth 5 form))
                  (length (nth 5 form))))
          forms))"##;
    let expect = expect![[
        r#"OK ((defapiget-forge #1=(resource doc link &optional objects internal-resource &rest config) "Define a new GET resource wrapp" #2=(declare (indent defun) (doc-string 2)) apiwrap-gendefun 12) (defapiput-forge #1# "Define a new PUT resource wrapp" #2# apiwrap-gendefun 12) (defapihead-forge #1# "Define a new HEAD resource wrap" #2# apiwrap-gendefun 12) (defapipost-forge #1# "Define a new POST resource wrap" #2# apiwrap-gendefun 12) (defapipatch-forge #1# "Define a new PATCH resource wra" #2# apiwrap-gendefun 12) (defapidelete-forge #1# "Define a new DELETE resource wr" #2# apiwrap-gendefun 12))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_genmacros_adds_default_link_function_to_mutable_config() {
    let elisp_form = r##"(let ((functions '((request . ignore))))
         (let ((forms
                (apiwrap-genmacros
                 "Forge" "forge" nil functions)))
           (list functions
                 (mapcar #'cadr forms)
                 (mapcar
                  (lambda (form)
                    (string-match-p "CONFIG is a list" (nth 3 form)))
                  forms))))"##;
    let expect = expect![
        "OK (((request . ignore) (link . apiwrap-stdgenlink)) (defapiget-forge defapiput-forge defapihead-forge defapipost-forge defapipatch-forge defapidelete-forge) (1582 1582 1584 1584 1586 1588))"
    ];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_gendefun_simple_endpoint_form_routes_optional_data_and_params() {
    let elisp_form = r##"(apiwrap-gendefun
         "Forge" "forge" nil 'get
         "/issues" "List issues." "issues/list"
         nil nil
         '((request . forge-request)
           (link . apiwrap-stdgenlink))
         nil)"##;
    let expect = expect![[
        r#"OK (prog1 (defun forge-get-issues (&optional data &rest params) "List issues.\n\nDATA is a data structure to be sent with this request.  If it's\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Forge API endpoint\n\n    GET /issues\n\nwhich is documented at\n\n    URL `issues/list'" (declare (indent 0)) (apply forge-request 'get "/issues" (if (keywordp data) (list (cons data params) nil) (list params data)))) (put 'forge-get-issues 'apiwrap '((prefix . "forge") (method . get) (endpoint . "/issues") (link . "issues/list"))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_gendefun_object_endpoint_form_includes_docs_metadata_and_processors() {
    let elisp_form = r##"(apiwrap-gendefun
         "Forge" "forge"
         '((repo . "REPO is a repository.")
           (issue . "ISSUE is an issue."))
         'post
         "/repos/:owner/:repo/issues/:number/comments"
         "Create a comment."
         "issues/comments#create"
         '(repo issue)
         "/repos/:repo.owner.login/:repo.name/issues/:issue.number/comments"
         '((request . forge-request)
           (pre-process-data . forge-data)
           (pre-process-params . forge-params)
           (link . apiwrap-stdgenlink))
         nil)"##;
    let expect = expect![[
        r#"OK (prog1 (defun forge-post-repos-owner-repo-issues-number-comments (repo issue &optional data &rest params) "Create a comment.\n\nDATA is a data structure to be sent with this request.  If it's\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Forge API endpoint\n\n    POST /repos/:owner/:repo/issues/:number/comments\n\nwhich is documented at\n\n    URL `issues/comments#create'" (declare (indent 2)) (apply forge-request 'post (let ((alist (list (cons 'repo repo) (cons 'issue issue)))) (let ((.repo.owner.login (cdr (assq 'login (cdr (assq 'owner (cdr (assq 'repo alist))))))) (.repo.name (cdr (assq 'name (cdr (assq 'repo alist))))) (.issue.number (cdr (assq 'number (cdr (assq 'issue alist)))))) (format "/repos/%s/%s/issues/%s/comments" (apiwrap--encode-url .repo.owner.login) (apiwrap--encode-url .repo.name) (apiwrap--encode-url .issue.number)))) (if (keywordp data) (list (funcall forge-params (cons data params)) nil) (list (funcall forge-params params) (funcall forge-data data))))) (put 'forge-post-repos-owner-repo-issues-number-comments 'apiwrap '((prefix . "forge") (method . post) (endpoint . "/repos/:owner/:repo/issues/:number/comments") (link . "issues/comments#create"))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}
