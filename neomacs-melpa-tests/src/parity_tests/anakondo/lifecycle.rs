use expect_test::expect;

use super::assert_anakondo_parity;

#[test]
fn initializing_new_project_builds_all_four_typed_caches_before_analysis() {
    let elisp_form = r##"(let ((anakondo--cache nil)
                          observed)
                      (cl-letf
                          (((symbol-function
                             'anakondo--project-analyse-sync)
                            (lambda
                                (vars
                                 namespaces
                                 usages
                                 java)
                              (setq observed
                                    (list
                                     (hash-table-p vars)
                                     (hash-table-p namespaces)
                                     (hash-table-p usages)
                                     (hash-table-p java)))
                              (puthash :from-analysis
                                       'var vars)
                              (puthash :from-analysis
                                       'ns namespaces)
                              (puthash :from-analysis
                                       'usage usages)
                              (puthash :from-analysis
                                       'java java)
                              'analyzed)))
                        (let* ((result
                                (anakondo--init-project-cache
                                 "workspace/"))
                               (root-cache
                                (anakondo--get-project-cache
                                 "workspace/")))
                          (list
                           result
                           observed
                           (hash-table-test
                            anakondo--cache)
                           (hash-table-count
                            anakondo--cache)
                           (mapcar
                            (lambda (key)
                              (gethash
                               :from-analysis
                               (gethash
                                key root-cache)))
                            '(:var-def-cache
                              :ns-def-cache
                              :ns-usage-cache
                              :java-classes-cache))))))"##;
    let expect = expect!["OK (analyzed (t t t t) equal 1 (var ns usage java))"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn initializing_existing_project_reanalyzes_only_current_buffer_and_reuses_cache_identity() {
    let elisp_form = r##"(let* ((root
                            (file-name-as-directory
                             (expand-file-name
                              "existing-project"
                              (getenv
                               "NEOMACS_TEST_SANDBOX_ROOT"))))
                           (anakondo--cache
                            (make-hash-table
                             :test 'equal))
                           (root-cache
                            (make-hash-table))
                           (vars (make-hash-table))
                           (namespaces
                            (make-hash-table))
                           (usages (make-hash-table))
                           (java (make-hash-table))
                           events)
                      (make-directory root t)
                      (puthash :var-def-cache
                               vars root-cache)
                      (puthash :ns-def-cache
                               namespaces root-cache)
                      (puthash :ns-usage-cache
                               usages root-cache)
                      (puthash :java-classes-cache
                               java root-cache)
                      (anakondo--set-project-cache
                       root root-cache)
                      (let ((default-directory root))
                        (cl-letf
                            (((symbol-function
                               'anakondo--project-analyse-sync)
                              (lambda (&rest caches)
                                (push
                                 (cons 'project caches)
                                 events)))
                             ((symbol-function
                               'anakondo--clj-kondo-buffer-analyse-sync)
                              (lambda (&rest caches)
                                (push
                                 (cons 'buffer caches)
                                 events)
                                :app.core)))
                          (list
                           (anakondo--init-project-cache
                            root)
                           (nreverse events)
                           (eq
                            root-cache
                            (anakondo--get-project-cache
                             root))
                           (eq
                            java
                            (gethash
                             :java-classes-cache
                             root-cache))))))"##;
    let expect =
        expect!["OK (:app.core ((buffer #s(hash-table) #s(hash-table) #s(hash-table))) t t)"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn deleting_project_cache_removes_only_requested_root_and_handles_missing_or_nil_cache() {
    let elisp_form = r##"(let ((anakondo--cache
                           (make-hash-table
                            :test 'equal)))
                      (puthash "one/"
                               'one anakondo--cache)
                      (puthash "two/"
                               'two anakondo--cache)
                      (let ((present
                             (anakondo--delete-project-cache
                              "one/"))
                            (missing
                             (anakondo--delete-project-cache
                              "missing/")))
                        (let ((remaining
                               (list
                                (gethash
                                 "one/"
                                 anakondo--cache)
                                (gethash
                                 "two/"
                                 anakondo--cache)
                                (hash-table-count
                                 anakondo--cache))))
                          (setq anakondo--cache nil)
                          (list
                           present
                           missing
                           remaining
                           (anakondo--delete-project-cache
                            "two/")))))"##;
    let expect = expect!["OK (nil nil (nil two 1) nil)"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn minor_mode_lifecycle_is_buffer_local_manages_capf_cache_and_project_callbacks() {
    let elisp_form = r##"(let ((first
                           (generate-new-buffer
                            " *anakondo-first*"))
                          (second
                           (generate-new-buffer
                            " *anakondo-second*"))
                          events)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'anakondo--init-project-cache)
                                (lambda (root)
                                  (push
                                   (list
                                    'init
                                    (buffer-name)
                                    root)
                                   events)
                                  nil))
                               ((symbol-function
                                 'anakondo--delete-project-cache)
                                (lambda (root)
                                  (push
                                   (list
                                    'delete
                                    (buffer-name)
                                    root)
                                   events)
                                  nil)))
                            (with-current-buffer first
                              (setq default-directory
                                    "project-one/")
                              (anakondo-minor-mode 1)
                              (setq
                               anakondo--completion-candidates-cache
                               '(10 "cached")))
                            (with-current-buffer second
                              (setq default-directory
                                    "project-two/")
                              (anakondo-minor-mode 1))
                            (with-current-buffer first
                              (anakondo-minor-mode -1))
                            (list
                             (with-current-buffer first
                               (list
                                anakondo-minor-mode
                                (memq
                                 #'anakondo-completion-at-point
                                 completion-at-point-functions)
                                anakondo--completion-candidates-cache))
                             (with-current-buffer second
                               (list
                                anakondo-minor-mode
                                (and
                                 (memq
                                  #'anakondo-completion-at-point
                                  completion-at-point-functions)
                                 t)
                                anakondo--completion-candidates-cache))
                             (nreverse events)))
                        (kill-buffer first)
                        (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((nil nil nil) (t t nil) ((init " *anakondo-first*" "project-one/") (init " *anakondo-second*" "project-two/") (delete " *anakondo-first*" "project-one/")))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn refresh_command_rejects_disabled_mode_then_passes_every_live_cache_to_analysis() {
    let elisp_form = r##"(with-temp-buffer
                      (let (calls)
                        (let ((disabled
                               (condition-case error-data
                                   (anakondo-refresh-project-cache)
                                 (error error-data))))
                          (setq-local
                           anakondo-minor-mode t)
                          (cl-letf
                              (((symbol-function
                                 'anakondo--get-project-var-def-cache)
                                (lambda () 'vars))
                               ((symbol-function
                                 'anakondo--get-project-ns-def-cache)
                                (lambda () 'namespaces))
                               ((symbol-function
                                 'anakondo--get-project-ns-usage-cache)
                                (lambda () 'usages))
                               ((symbol-function
                                 'anakondo--get-project-java-classes-cache)
                                (lambda () 'java))
                               ((symbol-function
                                 'anakondo--project-analyse-sync)
                                (lambda (&rest caches)
                                  (setq calls caches)
                                  'refreshed)))
                            (list
                             disabled
                             (anakondo-refresh-project-cache)
                             calls)))))"##;
    let expect = expect![[
        r#"OK ((error "Anakondo minor mode not on in current buffer") refreshed (vars namespaces usages java))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}
