use expect_test::expect;

use super::assert_apiwrap_parity;

#[test]
fn apiwrap_new_backend_registers_backend_and_six_usable_macros() {
    let elisp_form = r##"(progn
         (defun awforge-request (&rest _args) nil)
         (let ((apiwrap-backends nil))
         (apiwrap-new-backend
             "Forge" "awforge"
             '((repo . "REPO is a repository."))
           :request #'awforge-request)
         (list apiwrap-backends
               (mapcar
                (lambda (symbol)
                  (list symbol
                        (macrop symbol)
                        (help-function-arglist symbol t)
                        (get symbol 'lisp-indent-function)))
                '(defapiget-awforge
                  defapiput-awforge
                  defapihead-awforge
                  defapipost-awforge
                  defapipatch-awforge
                  defapidelete-awforge)))))"##;
    let expect = expect![[
        r#"OK ((("Forge" . "awforge")) ((defapiget-awforge t #1=(resource doc link &optional objects internal-resource &rest config) defun) (defapiput-awforge t #1# defun) (defapihead-awforge t #1# defun) (defapipost-awforge t #1# defun) (defapipatch-awforge t #1# defun) (defapidelete-awforge t #1# defun)))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_simple_get_wrapper_routes_params_and_data_to_request_primitive() {
    let elisp_form = r##"(progn
         (defun awget-request (&rest _args) nil)
         (apiwrap-new-backend "Forge" "awget" nil
           :request #'awget-request)
         (defapiget-awget "/issues"
           "List issues."
           "issues/list")
         (let (calls)
           (cl-letf (((symbol-function 'awget-request)
                      (lambda (method resource params data)
                        (push (list method resource params data) calls)
                        (list :response (length calls)))))
             (list
              (awget-get-issues)
              (awget-get-issues :state "closed" :page 2)
              (awget-get-issues
               '((title . "Bug") (labels . ["urgent"]))
               :notify t)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((:response 1) (:response 2) (:response 3) ((get "/issues" nil nil) (get "/issues" (:state "closed" :page 2) nil) (get "/issues" (:notify t) ((title . "Bug") (labels . ["urgent"])))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_all_six_generated_methods_route_their_exact_http_symbol() {
    let elisp_form = r##"(progn
         (defun awmethods-request (&rest _args) nil)
         (apiwrap-new-backend "Forge" "awmethods" nil
           :request #'awmethods-request)
         (defapiget-awmethods "/resource" "Get." "get")
         (defapiput-awmethods "/resource" "Put." "put")
         (defapihead-awmethods "/resource" "Head." "head")
         (defapipost-awmethods "/resource" "Post." "post")
         (defapipatch-awmethods "/resource" "Patch." "patch")
         (defapidelete-awmethods "/resource" "Delete." "delete")
         (let (calls)
           (cl-letf (((symbol-function 'awmethods-request)
                      (lambda (&rest args)
                        (push args calls)
                        (car args))))
             (list
              (awmethods-get-resource :case 1)
              (awmethods-put-resource :case 2)
              (awmethods-head-resource :case 3)
              (awmethods-post-resource :case 4)
              (awmethods-patch-resource :case 5)
              (awmethods-delete-resource :case 6)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (get put head post patch delete ((get "/resource" (:case 1) nil) (put "/resource" (:case 2) nil) (head "/resource" (:case 3) nil) (post "/resource" (:case 4) nil) (patch "/resource" (:case 5) nil) (delete "/resource" (:case 6) nil)))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_repository_issue_workflow_resolves_multiple_real_objects() {
    let elisp_form = r##"(progn
         (defun awcomments-request (&rest _args) nil)
         (apiwrap-new-backend
             "Forge" "awcomments"
             '((repo . "REPO is a repository.")
               (issue . "ISSUE is an issue."))
           :request #'awcomments-request)
         (defapipost-awcomments
             "/repos/:owner/:repo/issues/:number/comments"
           "Create an issue comment."
           "issues/comments#create"
           (repo issue)
           "/repos/:repo.owner.login/:repo.name/issues/:issue.number/comments")
         (let (call)
           (cl-letf (((symbol-function 'awcomments-request)
                      (lambda (&rest args)
                        (setq call args)
                        'created)))
             (let ((repo
                    '((owner (login . "GNU Project"))
                      (name . "emacs/core")))
                   (issue '((number . 163))))
               (list
                (awcomments-post-repos-owner-repo-issues-number-comments
                 repo issue
                 '((body . "Works on GNU and Neo"))
                 :notify "watchers")
                call)))))"##;
    let expect = expect![[
        r#"OK (created (post "/repos/GNU%20Project/emacs/core/issues/163/comments" (:notify "watchers") ((body . "Works on GNU and Neo"))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_keyword_first_argument_becomes_params_instead_of_request_data() {
    let elisp_form = r##"(progn
         (defun awkeyword-request (&rest _args) nil)
         (apiwrap-new-backend "Forge" "awkeyword" nil
           :request #'awkeyword-request)
         (defapipost-awkeyword "/search" "Search." "search")
         (let (calls)
           (cl-letf (((symbol-function 'awkeyword-request)
                      (lambda (method resource params data)
                        (push (list method resource params data) calls)
                        'ok)))
             (awkeyword-post-search :query "gc roots" :limit 20)
             (awkeyword-post-search
              '((query . "gc roots"))
              :limit 20)
             (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((post "/search" (:query "gc roots" :limit 20) nil) (post "/search" (:limit 20) ((query . "gc roots"))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_backend_preprocessors_transform_params_and_data_independently() {
    let elisp_form = r##"(progn
         (defun awprocess-request (&rest _args) nil)
         (defun awprocess-params (value) value)
         (defun awprocess-data (value) value)
         (apiwrap-new-backend "Forge" "awprocess" nil
           :request #'awprocess-request
           :pre-process-params #'awprocess-params
           :pre-process-data #'awprocess-data)
         (defapipatch-awprocess "/issues/7" "Update." "issues#update")
         (let (events)
           (cl-letf (((symbol-function 'awprocess-params)
                      (lambda (params)
                        (push (list 'params params) events)
                        (list :processed-params params)))
                     ((symbol-function 'awprocess-data)
                      (lambda (data)
                        (push (list 'data data) events)
                        (list :processed-data data)))
                     ((symbol-function 'awprocess-request)
                      (lambda (&rest args)
                        (push (cons 'request args) events)
                        'updated)))
             (list
              (awprocess-patch-issues-7
               '((state . "closed"))
               :notify t)
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (updated ((params #1=(:notify t)) (data #2=((state . "closed"))) (request patch "/issues/7" (:processed-params #1#) (:processed-data #2#))))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_endpoint_configuration_overrides_backend_preprocessor() {
    let elisp_form = r##"(progn
         (defun awoverride-request (&rest _args) nil)
         (defun awoverride-default (value) value)
         (defun awoverride-special (value) value)
         (apiwrap-new-backend "Forge" "awoverride" nil
           :request #'awoverride-request
           :pre-process-params #'awoverride-default)
         (defapiget-awoverride "/default" "Default." "default")
         (defapiget-awoverride "/special" "Special." "special"
           :pre-process-params #'awoverride-special)
         (let (events)
           (cl-letf (((symbol-function 'awoverride-default)
                      (lambda (params)
                        (push (cons 'default params) events)
                        (cons :default params)))
                     ((symbol-function 'awoverride-special)
                      (lambda (params)
                        (push (cons 'special params) events)
                        (cons :special params)))
                     ((symbol-function 'awoverride-request)
                      (lambda (&rest args)
                        (push (cons 'request args) events)
                        args)))
             (list
              (awoverride-get-default :value 1)
              (awoverride-get-special :value 2)
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (#2=(get "/default" (:default . #1=(:value 1)) nil) #4=(get "/special" (:special . #3=(:value 2)) nil) ((default . #1#) (request . #2#) (special . #3#) (request . #4#)))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_around_macro_wraps_the_complete_runtime_request() {
    let elisp_form = r##"(progn
         (defmacro awaround-wrapper (form)
           `(list 'before ,form 'after))
         (defun awaround-request (&rest _args) nil)
         (apiwrap-new-backend "Forge" "awaround" nil
           :request #'awaround-request
           :around #'awaround-wrapper)
         (defapiget-awaround "/events" "Events." "events")
         (let (calls)
           (cl-letf (((symbol-function 'awaround-request)
                      (lambda (&rest args)
                        (push args calls)
                        'payload)))
             (list
              (awaround-get-events :page 3)
              (nreverse calls)))))"##;
    let expect = expect![[r#"OK ((before payload after) ((get "/events" (:page 3) nil)))"#]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_condition_case_converts_configured_error_and_preserves_success() {
    let elisp_form = r##"(progn
         (define-error 'awmissing "Missing API object")
         (defun awerrors-request (&rest _args) nil)
         (apiwrap-new-backend "Forge" "awerrors" nil
           :request #'awerrors-request)
         (defapiget-awerrors "/objects/:id"
           "Get object."
           "objects#get"
           :condition-case
           ((awmissing (list 'not-found (cadr it)))))
         (let ((behavior 'missing))
           (cl-letf (((symbol-function 'awerrors-request)
                      (lambda (&rest args)
                        (if (eq behavior 'missing)
                            (signal 'awmissing (list (nth 1 args)))
                          (list 'found args)))))
             (let ((missing (awerrors-get-objects-id)))
               (setq behavior 'present)
               (list missing
                     (awerrors-get-objects-id :expand t))))))"##;
    let expect = expect![[
        r#"OK ((not-found "/objects/:id") (found (get "/objects/:id" (:expand t) nil)))"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_generated_wrapper_exposes_exact_docs_args_indent_and_metadata() {
    let elisp_form = r##"(progn
         (defun awmeta-request (&rest _args) nil)
         (apiwrap-new-backend
             "Forge" "awmeta"
             '((repo . "REPO is the selected repository."))
           :request #'awmeta-request
           :link (lambda (properties)
                   (format "https://docs.test/%s/%s"
                           (alist-get 'method properties)
                           (alist-get 'link properties))))
         (defapiget-awmeta "/repos/:owner/:repo/issues"
           "List practical repository issues."
           "issues/list"
           (repo)
           "/repos/:repo.owner.login/:repo.name/issues")
         (let ((symbol 'awmeta-get-repos-owner-repo-issues))
           (list
            (help-function-arglist symbol t)
            (get symbol 'lisp-indent-function)
            (get symbol 'apiwrap)
            (documentation symbol)
            (commandp symbol))))"##;
    let expect = expect![[
        r#"OK ((repo &optional data &rest params) 1 ((prefix . "awmeta") (method . get) (endpoint . "/repos/:owner/:repo/issues") (link . "issues/list")) "List practical repository issues.\n\nDATA is a data structure to be sent with this request.  If it’s\nnot required, it can simply be omitted.\n\nPARAMS is a plist of parameters appended to the method call.\n\n--------------------\n\nThis generated function wraps the Forge API endpoint\n\n    GET /repos/:owner/:repo/issues\n\nwhich is documented at\n\n    URL ‘https://docs.test/get/issues/list’" nil)"#
    ]];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_apropos_filters_generated_endpoints_by_backend_and_pattern() {
    let elisp_form = r##"(progn
         (apiwrap-new-backend "Alpha" "awalpha" nil
           :request #'ignore)
         (apiwrap-new-backend "Beta" "awbeta" nil
           :request #'ignore)
         (defapiget-awalpha "/issues/open" "Open." "open")
         (defapiget-awalpha "/issues/closed" "Closed." "closed")
         (defapiget-awalpha "/users" "Users." "users")
         (defapiget-awbeta "/issues/open" "Open." "open")
         (list
          (sort (mapcar #'car (apropos-api-endpoint "awalpha" "issues"))
                #'string-lessp)
          (sort (mapcar #'car (apropos-api-endpoint "awbeta" "issues"))
                #'string-lessp)
          (apropos-api-endpoint "awalpha" "missing")))"##;
    let expect = expect![
        "OK ((awalpha-get-issues-closed awalpha-get-issues-open) (awbeta-get-issues-open) nil)"
    ];
    assert_apiwrap_parity(elisp_form, expect);
}

#[test]
fn apiwrap_redefining_same_backend_is_idempotent_but_new_prefix_is_distinct() {
    let elisp_form = r##"(let ((apiwrap-backends nil))
         (apiwrap-new-backend "Forge" "awsame" nil
           :request #'ignore)
         (apiwrap-new-backend "Forge" "awsame" nil
           :request #'ignore)
         (apiwrap-new-backend "Forge" "awother" nil
           :request #'ignore)
         (list apiwrap-backends
               (length apiwrap-backends)
               (macrop 'defapiget-awsame)
               (macrop 'defapiget-awother)))"##;
    let expect = expect![[r#"OK ((("Forge" . "awother") ("Forge" . "awsame")) 2 t t)"#]];
    assert_apiwrap_parity(elisp_form, expect);
}
