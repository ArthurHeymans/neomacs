use expect_test::expect;

use super::assert_anakondo_parity;

#[test]
fn project_root_macro_uses_clojure_projectile_project_and_directory_priority_order() {
    let elisp_form = r##"(let (active project-value)
                      (cl-letf
                          (((symbol-function 'featurep)
                            (lambda (feature)
                              (memq feature active)))
                           ((symbol-function
                             'clojure-project-dir)
                            (lambda () "clojure-root/"))
                           ((symbol-function
                             'projectile-project-root)
                            (lambda () "projectile-root/"))
                           ((symbol-function
                             'anakondo--project-get-project-root)
                            (lambda () project-value)))
                        (mapcar
                         (lambda (scenario)
                           (setq active (car scenario))
                           (setq project-value
                                 (cadr scenario))
                           (let ((default-directory
                                  (caddr scenario)))
                             (anakondo--with-project-root
                              root)))
                         '(((clojure-mode projectile)
                            "project-a/"
                            "fallback-a/")
                           ((projectile)
                            "project-b/"
                            "fallback-b/")
                           (nil
                            "project-root/"
                            "fallback-c/")
                           (nil
                            nil
                            "fallback-d/")))))"##;
    let expect =
        expect![[r#"OK ("clojure-root/" "projectile-root/" "project-root/" "fallback-d/")"#]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn project_el_wrapper_supports_new_legacy_and_missing_project_apis() {
    let elisp_form = r##"(let ((real-fboundp
                           (symbol-function 'fboundp))
                          old-api
                          project-present)
                      (cl-letf
                          (((symbol-function 'featurep)
                            (lambda (feature)
                              (eq feature 'project)))
                           ((symbol-function 'project-current)
                            (lambda ()
                              (and
                               project-present
                               'project-object)))
                           ((symbol-function 'project-root)
                            (lambda (project)
                              (list 'new project)))
                           ((symbol-function 'project-roots)
                            (lambda (project)
                              (list
                               (list 'legacy project)
                               'ignored)))
                           ((symbol-function 'fboundp)
                            (lambda (function)
                              (if
                                  (eq function
                                      'project-root)
                                  (not old-api)
                                (funcall
                                 real-fboundp
                                 function)))))
                        (setq project-present t)
                        (let ((new
                               (anakondo--project-get-project-root)))
                          (setq old-api t)
                          (let ((legacy
                                 (anakondo--project-get-project-root)))
                            (setq project-present nil)
                            (list
                             new
                             legacy
                             (anakondo--project-get-project-root))))))"##;
    let expect = expect!["OK ((new project-object) (legacy project-object) nil)"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn project_cache_set_get_and_typed_accessors_share_the_exact_root_cache() {
    let elisp_form = r##"(let* ((anakondo--cache
                            (make-hash-table :test 'equal))
                           (root "workspace/project/")
                           (root-cache (make-hash-table))
                           (var-cache (make-hash-table))
                           (ns-cache (make-hash-table))
                           (usage-cache (make-hash-table))
                           (java-cache (make-hash-table))
                           (default-directory root))
                      (puthash :var-def-cache
                               var-cache root-cache)
                      (puthash :ns-def-cache
                               ns-cache root-cache)
                      (puthash :ns-usage-cache
                               usage-cache root-cache)
                      (puthash :java-classes-cache
                               java-cache root-cache)
                      (list
                       (eq
                        (anakondo--set-project-cache
                         root root-cache)
                        root-cache)
                       (eq
                        (anakondo--get-project-cache root)
                        root-cache)
                       (eq
                        (anakondo--get-project-var-def-cache)
                        var-cache)
                       (eq
                        (anakondo--get-project-ns-def-cache)
                        ns-cache)
                       (eq
                        (anakondo--get-project-ns-usage-cache)
                        usage-cache)
                       (eq
                        (anakondo--get-project-java-classes-cache)
                        java-cache)
                       (anakondo--get-project-cache
                        "missing/")))"##;
    let expect = expect!["OK (t t t t t t nil)"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn var_definition_cache_merges_overwrites_and_invalidates_one_namespace_only() {
    let elisp_form = r##"(let ((table
                           (make-hash-table)))
                      (cl-labels
                          ((definition
                             (namespace name line)
                             (let ((value
                                    (make-hash-table)))
                               (puthash :ns
                                        namespace value)
                               (puthash :name name value)
                               (puthash :line line value)
                               value)))
                        (let ((alpha-one
                               (definition
                                "app.core" "one" 1))
                              (alpha-two
                               (definition
                                "app.core" "two" 2))
                              (beta-one
                               (definition
                                "app.other" "one" 3)))
                          (anakondo--upsert-var-def-cache
                           table
                           (list
                            alpha-one
                            alpha-two
                            beta-one))
                          (let ((updated
                                 (definition
                                  "app.core" "one" 10)))
                            (anakondo--upsert-var-def-cache
                             table (list updated)))
                          (let ((replacement
                                 (definition
                                  "app.core" "fresh" 20)))
                            (list
                             (eq
                              (anakondo--upsert-var-def-cache
                               table
                               (list replacement)
                               :app.core)
                              table)
                             (hash-table-count table)
                             (let ((alpha
                                    (gethash
                                     :app.core table)))
                               (list
                                (hash-table-count alpha)
                                (gethash :one alpha)
                                (gethash :two alpha)
                                (gethash
                                 :line
                                 (gethash
                                  :fresh alpha))))
                             (let ((beta
                                    (gethash
                                     :app.other table)))
                               (list
                                (hash-table-count beta)
                                (gethash
                                 :line
                                 (gethash
                                  :one beta)))))))))"##;
    let expect = expect!["OK (t 2 (1 nil nil 20) (1 3))"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn namespace_definition_and_usage_caches_preserve_metadata_and_replace_stale_edges() {
    let elisp_form = r##"(let ((definitions
                           (make-hash-table))
                          (usages
                           (make-hash-table)))
                      (cl-labels
                          ((record
                             (&rest pairs)
                             (let ((value
                                    (make-hash-table)))
                               (while pairs
                                 (puthash
                                  (pop pairs)
                                  (pop pairs)
                                  value))
                               value)))
                        (anakondo--upsert-ns-def-cache
                         definitions
                         (list
                          (record
                           :name "app.core"
                           :filename "old.clj")
                          (record
                           :name "app.other"
                           :filename "other.clj")
                          (record
                           :name "app.core"
                           :filename "new.clj")))
                        (anakondo--upsert-ns-usage-cache
                         usages
                         (list
                          (record
                           :from "app.core"
                           :to "lib.util"
                           :alias "u")
                          (record
                           :from "app.core"
                           :to "lib.db"
                           :alias nil)
                          (record
                           :from "app.other"
                           :to "lib.util"
                           :alias "util")))
                        (anakondo--upsert-ns-usage-cache
                         usages
                         (list
                          (record
                           :from "app.core"
                           :to "lib.api"
                           :alias "api"))
                         :app.core)
                        (list
                         (hash-table-count definitions)
                         (gethash
                          :filename
                          (gethash
                           :app.core definitions))
                         (gethash
                          :filename
                          (gethash
                           :app.other definitions))
                         (hash-table-count usages)
                         (let ((core
                                (gethash
                                 :app.core usages)))
                           (list
                            (hash-table-count core)
                            (gethash
                             :lib.util core)
                            (gethash
                             :lib.db core)
                            (gethash
                             :alias
                             (gethash
                              :lib.api core))))
                         (gethash
                          :alias
                          (gethash
                           :lib.util
                           (gethash
                            :app.other usages))))))"##;
    let expect = expect![[r#"OK (2 "new.clj" "other.clj" 2 (1 nil nil "api") "util")"#]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn project_and_buffer_analysis_integrate_real_analysis_maps_into_all_caches() {
    let elisp_form = r##"(let ((var-cache (make-hash-table))
                          (ns-cache (make-hash-table))
                          (usage-cache (make-hash-table))
                          responses
                          calls)
                      (cl-labels
                          ((record
                             (&rest pairs)
                             (let ((value
                                    (make-hash-table)))
                               (while pairs
                                 (puthash
                                  (pop pairs)
                                  (pop pairs)
                                  value))
                               value))
                           (analysis
                             (vars namespaces usages)
                             (record
                              :var-definitions vars
                              :namespace-definitions namespaces
                              :namespace-usages usages)))
                        (setq
                         responses
                         (list
                          (analysis
                           (list
                            (record
                             :ns "project.core"
                             :name "project-fn"))
                           (list
                            (record
                             :name "project.core"))
                           (list
                            (record
                             :from "project.core"
                             :to "lib.one"
                             :alias "one")))
                          (analysis
                           (list
                            (record
                             :ns "buffer.core"
                             :name "buffer-fn"))
                           (list
                            (record
                             :name "buffer.core"))
                           nil)
                          (analysis
                           (list
                            (record
                             :ns "user"
                             :name "scratch-fn"))
                           nil
                           nil)))
                        (cl-letf
                            (((symbol-function
                               'anakondo--get-project-path)
                              (lambda () "project-cp"))
                             ((symbol-function
                               'anakondo--get-buffer-lang)
                              (lambda () "clj"))
                             ((symbol-function
                               'anakondo--clj-kondo-analyse-sync)
                              (lambda (path lang)
                                (push
                                 (list path lang)
                                 calls)
                                (pop responses))))
                          (let ((default-directory
                                 "workspace/"))
                            (list
                             (anakondo--clj-kondo-project-analyse-sync
                              var-cache
                              ns-cache
                              usage-cache)
                             (anakondo--clj-kondo-buffer-analyse-sync
                              var-cache
                              ns-cache
                              usage-cache)
                             (anakondo--clj-kondo-buffer-analyse-sync
                              var-cache
                              ns-cache
                              usage-cache)
                             (nreverse calls)
                             (gethash
                              :name
                              (gethash
                               :project-fn
                               (gethash
                                :project.core
                                var-cache)))
                             (gethash
                              :name
                              (gethash
                               :buffer-fn
                               (gethash
                                :buffer.core
                                var-cache)))
                             (gethash
                              :name
                              (gethash
                               :scratch-fn
                               (gethash
                                :user
                                var-cache)))
                             (gethash
                              :alias
                              (gethash
                               :lib.one
                               (gethash
                                :project.core
                                usage-cache)))
                             (hash-table-count
                              ns-cache))))))"##;
    let expect = expect![[
        r#"OK ("workspace/" :buffer.core :user (("project-cp" "clj") ("-" "clj") ("-" "clj")) "project-fn" "buffer-fn" "scratch-fn" "one" 2)"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}
