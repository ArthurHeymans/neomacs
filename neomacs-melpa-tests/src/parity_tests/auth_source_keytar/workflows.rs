use expect_test::expect;

use super::assert_auth_source_keytar_batch;

#[test]
fn workflows_public_surface_batch() {
    assert_auth_source_keytar_batch(&[
        (
            "auth_source_keytar_enabled_backend_dispatches_real_host_user_lookup_through_search_function",
            r##"(let ((auth-sources nil)
                               calls)
          (cl-letf
              (((symbol-function
                 'auth-source-forget-all-cached)
                (lambda ()
                  :cleared))
               ((symbol-function 'keytar-get-password)
                (lambda (service account)
                  (push
                   (list service account)
                   calls)
                  "workflow-secret")))
            (auth-source-keytar-enable)
            (let* ((backend
                    (auth-source-backend-parse
                     (car auth-sources)))
                   (search
                    (slot-value
                     backend
                     'search-function)))
              (list
               auth-sources
               (auth-source-keytar-test-backend-data
                backend)
               (funcall
                search
                :host "git.internal"
                :user "deploy")
               (nreverse calls)))))"##,
            true,
            expect![[
        r#"OK ((keytar) ("Keytar" keytar auth-source-keytar-search) "workflow-secret" (("git.internal" "deploy")))"#
    ]],
        ),
        (
            "auth_source_keytar_real_auth_source_search_exposes_direct_password_result_contract",
            r##"(let ((auth-sources
                                '(keytar))
                               calls)
          (auth-source-forget-all-cached)
          (cl-letf
              (((symbol-function 'keytar-get-password)
                (lambda (service account)
                  (push
                   (list service account)
                   calls)
                  "direct-secret")))
            (list
             (auth-source-keytar-test-error-data
              (lambda ()
                (auth-source-search
                 :host "registry.internal"
                 :user "release"
                 :max 1
                 :require '(:secret))))
             (nreverse calls))))"##,
            true,
            expect![[r#"OK ((:ok "direct-secret") (("registry.internal" "release")))"#]],
        ),
        (
            "auth_source_keytar_real_auth_source_search_handles_service_wide_credential_listing",
            r##"(let ((auth-sources
                                '(keytar))
                               calls)
          (auth-source-forget-all-cached)
          (cl-letf
              (((symbol-function
                 'keytar-find-credentials)
                (lambda (service)
                  (push service calls)
                  "[\n{ account: 'one', password: 'alpha' },\n{ account: 'two', password: 'beta' }\n]")))
            (list
             (auth-source-keytar-test-error-data
              (lambda ()
                (auth-source-search
                 :service "artifact-service"
                 :max 10
                 :require '(:secret))))
             (nreverse calls))))"##,
            true,
            expect![[r#"OK ((:ok ((:secret "beta") (:secret "alpha"))) ("artifact-service"))"#]],
        ),
        (
            "auth_source_keytar_real_pick_first_password_workflow_surfaces_backend_result_shape",
            r##"(let ((auth-sources
                                '(keytar))
                               calls)
          (auth-source-forget-all-cached)
          (cl-letf
              (((symbol-function 'keytar-get-password)
                (lambda (service account)
                  (push
                   (list service account)
                   calls)
                  "picked-secret")))
            (list
             (auth-source-keytar-test-error-data
              (lambda ()
                (auth-source-pick-first-password
                 :host "database.internal"
                 :user "backup")))
             (nreverse calls))))"##,
            true,
            expect![[
        r#"OK ((:error wrong-type-argument (listp "picked-secret")) (("database.internal" "backup")))"#
    ]],
        ),
        (
            "auth_source_keytar_repeated_real_search_and_enable_exercise_auth_source_cache_invalidation",
            r##"(let ((auth-sources
                                '(keytar))
                               (calls 0))
          (auth-source-forget-all-cached)
          (cl-letf
              (((symbol-function 'keytar-get-password)
                (lambda (_service _account)
                  (setq calls
                        (1+ calls))
                  (format
                   "secret-%d"
                   calls))))
            (let ((first
                   (auth-source-keytar-test-error-data
                    (lambda ()
                      (auth-source-search
                       :host "cache.internal"
                       :user "cache-user"
                       :max 1))))
                  (second
                   (auth-source-keytar-test-error-data
                    (lambda ()
                      (auth-source-search
                       :host "cache.internal"
                       :user "cache-user"
                       :max 1)))))
              (auth-source-keytar-enable)
              (let ((third
                     (auth-source-keytar-test-error-data
                      (lambda ()
                        (auth-source-search
                         :host "cache.internal"
                         :user "cache-user"
                         :max 1)))))
                (list
                 first
                 second
                 third
                 calls
                 auth-sources)))))"##,
            true,
            expect![[
        r#"OK ((:ok "\0\0\0\0\0\0\0\0") (:ok "\0\0\0\0\0\0\0\0") (:ok "secret-2") 2 (keytar))"#
    ]],
        ),
        (
            "auth_source_keytar_two_independent_services_keep_provider_arguments_and_secrets_separate",
            r##"(let (calls)
          (cl-letf
              (((symbol-function 'keytar-get-password)
                (lambda (service account)
                  (push
                   (list service account)
                   calls)
                  (format
                   "%s::%s"
                   service
                   account))))
            (list
             (auth-source-keytar-search
              :service "git"
              :account "alice")
             (auth-source-keytar-search
              :host "database"
              :user "reader")
             (auth-source-keytar-search
              :service "git"
              :account "bob")
             (nreverse calls))))"##,
            true,
            expect![[
        r#"OK ("git::alice" "database::reader" "git::bob" (("git" "alice") ("database" "reader") ("git" "bob")))"#
    ]],
        ),
        (
            "auth_source_keytar_listing_to_selection_workflow_preserves_provider_reverse_order",
            r##"(cl-letf
          (((symbol-function
             'keytar-find-credentials)
            (lambda (_)
              "[\n{ account: 'old', password: 'old-secret' },\n{ account: 'current', password: 'current-secret' }\n]")))
          (let* ((entries
                  (auth-source-keytar-search
                   :service "deploy"))
                 (selected
                  (car entries)))
            (list
             entries
             selected
             (plist-get
              selected
              :secret)
             (mapcar
              (lambda (entry)
                (plist-get
                 entry
                 :secret))
              entries))))"##,
            true,
            expect![[
        r#"OK ((#1=(:secret "current-secret") (:secret "old-secret")) #1# "current-secret" ("current-secret" "old-secret"))"#
    ]],
        ),
        (
            "auth_source_keytar_provider_rotation_is_observed_by_uncached_direct_searches",
            r##"(let ((passwords
                                '("version-one"
                                  "version-two"
                                  "version-three"))
                               calls)
          (cl-letf
              (((symbol-function 'keytar-get-password)
                (lambda (service account)
                  (push
                   (list service account)
                   calls)
                  (pop passwords))))
            (list
             (auth-source-keytar-search
              :host "rotating.internal"
              :user "service")
             (auth-source-keytar-search
              :host "rotating.internal"
              :user "service")
             (auth-source-keytar-search
              :host "rotating.internal"
              :user "service")
             passwords
             (nreverse calls))))"##,
            true,
            expect![[
        r#"OK ("version-one" "version-two" "version-three" nil (("rotating.internal" "service") ("rotating.internal" "service") ("rotating.internal" "service")))"#
    ]],
        ),
    ]);
}
