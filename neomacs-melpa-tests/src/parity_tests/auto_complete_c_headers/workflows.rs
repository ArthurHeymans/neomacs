use expect_test::expect;

use super::assert_auto_complete_c_headers_batch;

#[test]
fn workflows_public_surface_batch() {
    assert_auto_complete_c_headers_batch(&[
        (
            "auto_complete_c_headers_documentation_returns_path_separator_and_exact_file_content",
            r##"(let* ((root
                 (expand-file-name
                  "achead-document"
                  default-directory))
                (file
                 (expand-file-name
                  "api.h" root)))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "api.h"
                "#ifndef API_H\n#define API_H\nint api(void);\n#endif\n")
               (let ((achead:ac-latest-results-alist
                      (list
                       (cons "api.h" file))))
                 (achead:documentation-for-candidate
                  "api.h")))
           (delete-directory root t)))"##,
            true,
            expect![[
        r#"OK "[ORACLE-SANDBOX]/achead-document/api.h\n--------------------------\n#ifndef API_H\n#define API_H\nint api(void);\n#endif\n""#
    ]],
        ),
        (
            "auto_complete_c_headers_directory_documentation_is_only_the_resolved_path",
            r##"(let* ((root
                 (expand-file-name
                  "achead-directory-document"
                  default-directory))
                (directory
                 (expand-file-name
                  "nested" root)))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (make-directory directory)
               (let ((achead:ac-latest-results-alist
                      (list
                       (cons "nested/"
                             directory))))
                 (list
                  (file-relative-name
                   (achead:documentation-for-candidate
                    "nested/")
                   root)
                  (string-match-p
                   "--------------------------"
                   (achead:documentation-for-candidate
                    "nested/")))))
           (delete-directory root t)))"##,
            true,
            expect![[r#"OK ("nested" nil)"#]],
        ),
        (
            "auto_complete_c_headers_documentation_uses_first_duplicate_and_suppresses_missing_candidates",
            r##"(let* ((root
                 (expand-file-name
                  "achead-document-shadow"
                  default-directory))
                (first
                 (expand-file-name
                  "first.h" root))
                (second
                 (expand-file-name
                  "second.h" root)))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "first.h" "FIRST")
               (achead-test-write-file
                root "second.h" "SECOND")
               (let ((achead:ac-latest-results-alist
                      (list
                       (cons "shared.h" first)
                       (cons "shared.h" second)
                       (cons "gone.h"
                             (expand-file-name
                              "gone.h" root)))))
                 (list
                  (achead:documentation-for-candidate
                   "shared.h")
                  (achead:documentation-for-candidate
                   "unknown.h")
                  (achead:documentation-for-candidate
                   "gone.h"))))
           (delete-directory root t)))"##,
            true,
            expect![[
        r#"OK ("[ORACLE-SANDBOX]/achead-document-shadow/first.h\n--------------------------\nFIRST" nil nil)"#
    ]],
        ),
        (
            "auto_complete_c_headers_documentation_preserves_unicode_and_no_trailing_newline",
            r##"(let* ((root
                 (expand-file-name
                  "achead-unicode-document"
                  default-directory))
                (file
                 (expand-file-name
                  "unicode.hpp" root)))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "unicode.hpp"
                "// café 日本語\nconst char* greeting = \"héllo\";")
               (let ((achead:ac-latest-results-alist
                      (list
                       (cons "unicode.hpp"
                             file))))
                 (achead:documentation-for-candidate
                  "unicode.hpp")))
           (delete-directory root t)))"##,
            true,
            expect![[
        r#"OK "[ORACLE-SANDBOX]/achead-unicode-document/unicode.hpp\n--------------------------\n// café 日本語\nconst char* greeting = \"héllo\";""#
    ]],
        ),
        (
            "auto_complete_c_headers_ac_candidates_scans_prefix_directory_and_updates_latest_alist",
            r##"(let* ((root
                 (expand-file-name
                  "achead-ac-candidates"
                  default-directory))
                (achead:include-directories
                 (list root))
                (achead:include-cache nil)
                (achead:ac-latest-results-alist
                 '(("stale.h" . "/stale")))
                (ac-prefix "project/ap"))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "project/api.h" "api")
               (achead-test-write-file
                root "project/application.hpp"
                "application")
               (achead-test-write-file
                root "other.h" "other")
               (let ((candidates
                      (achead:ac-candidates)))
                 (list
                  candidates
                  (achead-test-relative-results
                   achead:ac-latest-results-alist
                   root)
                  (achead:documentation-for-candidate
                   "project/api.h"))))
           (delete-directory root t)))"##,
            true,
            expect![[
        r#"OK (("project/api.h" "project/application.hpp") (("project/api.h" . "project/api.h") ("project/application.hpp" . "project/application.hpp")) "[ORACLE-SANDBOX]/achead-ac-candidates/project/api.h\n--------------------------\napi")"#
    ]],
        ),
        (
            "auto_complete_c_headers_ac_candidates_with_root_prefix_scans_include_root",
            r##"(let* ((root
                 (expand-file-name
                  "achead-ac-root-prefix"
                  default-directory))
                (achead:include-directories
                 (list root))
                (achead:include-cache nil)
                (ac-prefix "vec"))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "vector" "vector")
               (achead-test-write-file
                root "api.h" "api")
               (list
                (achead:ac-candidates)
                (mapcar
                 #'car
                 achead:ac-latest-results-alist)))
           (delete-directory root t)))"##,
            true,
            expect![[r#"OK (("api.h" "vector") ("api.h" "vector"))"#]],
        ),
        (
            "auto_complete_c_headers_ac_candidates_suppresses_scan_errors_and_preserves_previous_results",
            r##"(let ((ac-prefix "pkg/")
               (achead:ac-latest-results-alist
                '(("previous.h"
                   . "/previous.h"))))
         (cl-letf
             (((symbol-function
                'achead:get-include-file-candidates)
               (lambda (&optional _basedir)
                 (error "scan failed"))))
           (list
            (achead:ac-candidates)
            achead:ac-latest-results-alist)))"##,
            true,
            expect![[r#"OK (nil (("previous.h" . "/previous.h")))"#]],
        ),
        (
            "auto_complete_c_headers_source_init_clears_cache_then_candidates_and_document_form_work",
            r##"(let* ((root
                 (expand-file-name
                  "achead-source-forms"
                  default-directory))
                (achead:include-directories
                 (list root))
                (achead:include-cache
                 '(("stale" "stale.h")))
                (ac-prefix "sdk/ap")
                (init
                 (cdr
                  (assq 'init
                        ac-source-c-headers)))
                (candidate-function
                 (cdr
                  (assq 'candidates
                        ac-source-c-headers)))
                (document-function
                 (cdr
                  (assq 'document
                        ac-source-c-headers))))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "sdk/api.h"
                "int api(void);")
               (eval init)
               (let ((candidates
                      (funcall
                       candidate-function)))
                 (list
                  achead:include-cache
                  candidates
                  (funcall
                   document-function
                   "sdk/api.h"))))
           (delete-directory root t)))"##,
            true,
            expect![[
        r#"OK ((("[ORACLE-SANDBOX]/achead-source-forms/sdk/" "api.h")) ("sdk/api.h") "[ORACLE-SANDBOX]/achead-source-forms/sdk/api.h\n--------------------------\nint api(void);")"#
    ]],
        ),
        (
            "auto_complete_c_headers_source_action_invokes_real_ac_start_entry_point",
            r##"(let ((calls nil)
               (action
                (cdr
                 (assq 'action
                       ac-source-c-headers))))
         (cl-letf
             (((symbol-function 'ac-start)
               (lambda (&rest arguments)
                 (push arguments calls)
                 'started)))
           (list
            action
            (funcall action)
            (funcall action
                     'manual "candidate")
            (nreverse calls))))"##,
            true,
            expect![[r#"OK (ac-start started started (nil (manual "candidate")))"#]],
        ),
        (
            "auto_complete_c_headers_practical_include_completion_refreshes_after_source_init",
            r##"(let* ((root
                 (expand-file-name
                  "achead-practical-refresh"
                  default-directory))
                (achead:include-directories
                 (list root))
                (achead:include-cache nil))
         (unwind-protect
             (progn
               (achead-test-reset-directory
                root)
               (achead-test-write-file
                root "library/old.h" "OLD")
               (let ((ac-prefix "library/o"))
                 (let ((first
                        (achead:ac-candidates)))
                   (achead-test-write-file
                    root "library/new.hpp"
                    "NEW")
                   (let ((cached
                          (achead:ac-candidates)))
                     (eval
                      (cdr
                       (assq
                        'init
                        ac-source-c-headers)))
                     (let ((refreshed
                            (achead:ac-candidates)))
                       (list
                        first
                        cached
                        refreshed
                        (achead:documentation-for-candidate
                         "library/new.hpp")))))))
           (delete-directory root t)))"##,
            true,
            expect![[
        r#"OK (("library/old.h") ("library/old.h") ("library/new.hpp" "library/old.h") "[ORACLE-SANDBOX]/achead-practical-refresh/library/new.hpp\n--------------------------\nNEW")"#
    ]],
        ),
    ]);
}
