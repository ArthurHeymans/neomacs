use expect_test::expect;

use super::assert_anakondo_parity;

#[test]
fn package_defaults_buffer_local_state_and_clojure_import_maps_match() {
    let elisp_form = r##"(list
                      (featurep 'anakondo)
                      anakondo-minor-mode-lighter
                      anakondo--cache
                      anakondo--completion-candidates-cache
                      (local-variable-if-set-p
                       'anakondo--completion-candidates-cache)
                      (list
                       (get
                        'anakondo-minor-mode-lighter
                        'custom-type)
                       (get
                        'anakondo-minor-mode-lighter
                        'custom-group))
                      (hash-table-test
                       anakondo--clojure-default-imports)
                      (hash-table-count
                       anakondo--clojure-default-imports)
                      (mapcar
                       (lambda (name)
                         (cons
                          name
                          (gethash
                           name
                           anakondo--clojure-default-imports)))
                       '("String"
                         "Math"
                         "BigDecimal"
                         "concurrent.Callable"))
                      (mapcar
                       (lambda (name)
                         (cons
                          name
                          (gethash
                           name
                           anakondo--clojure-default-imports-reverse)))
                       '("java.lang.String"
                         "java.lang.Math"
                         "java.math.BigDecimal"
                         "java.util.concurrent.Callable")))"##;
    let expect = expect![[
        r#"OK (t " k" nil nil t (string nil) equal 96 (("String" . "java.lang.String") ("Math" . "java.lang.Math") ("BigDecimal" . "java.math.BigDecimal") ("concurrent.Callable" . "java.util.concurrent.Callable")) (("java.lang.String" . "String") ("java.lang.Math" . "Math") ("java.math.BigDecimal" . "BigDecimal") ("java.util.concurrent.Callable" . "concurrent.Callable")))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn complete_shipped_callable_surface_has_exact_arglists_macro_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (symbol)
                        (list
                         symbol
                         (fboundp symbol)
                         (help-function-arglist symbol t)
                         (macrop symbol)
                         (commandp symbol)))
                      '(anakondo--with-project-root
                        anakondo--project-get-project-root
                        anakondo--get-project-cache
                        anakondo--set-project-cache
                        anakondo--get-project-var-def-cache
                        anakondo--get-project-ns-def-cache
                        anakondo--get-project-ns-usage-cache
                        anakondo--get-project-java-classes-cache
                        anakondo--completion-symbol-bounds
                        anakondo--get-buffer-lang
                        anakondo--clj-kondo-analyse-sync
                        anakondo--get-project-path
                        anakondo--string->keyword
                        anakondo--upsert-var-def-cache
                        anakondo--upsert-ns-def-cache
                        anakondo--upsert-ns-usage-cache
                        anakondo--clj-kondo-project-analyse-sync
                        anakondo--clj-kondo-buffer-analyse-sync
                        anakondo--jar-analize-sync
                        anakondo--make-class-map
                        anakondo--java-analyze-class-map
                        anakondo--get-java-boot-classpath-list
                        anakondo--get-java-analysis-classpath
                        anakondo--java-project-analyse-sync
                        anakondo--safe-hash-table-values
                        anakondo--get-clj-kondo-completion-candidates
                        anakondo--get-local-completion-candidates
                        anakondo--get-java-completion-candidates
                        anakondo-completion-at-point
                        anakondo--project-analyse-sync
                        anakondo--init-project-cache
                        anakondo--delete-project-cache
                        anakondo-minor-mode
                        anakondo-refresh-project-cache
                        anakondo--minor-mode-enter
                        anakondo--minor-mode-exit
                        anakondo--minor-mode-guard))"##;
    let expect = expect![
        "OK ((anakondo--with-project-root t (&rest body) t nil) (anakondo--project-get-project-root t nil nil nil) (anakondo--get-project-cache t (root) nil nil) (anakondo--set-project-cache t (root root-cache) nil nil) (anakondo--get-project-var-def-cache t nil nil nil) (anakondo--get-project-ns-def-cache t nil nil nil) (anakondo--get-project-ns-usage-cache t nil nil nil) (anakondo--get-project-java-classes-cache t nil nil nil) (anakondo--completion-symbol-bounds t nil nil nil) (anakondo--get-buffer-lang t nil nil nil) (anakondo--clj-kondo-analyse-sync t (path default-lang) nil nil) (anakondo--get-project-path t nil nil nil) (anakondo--string->keyword t (str) nil nil) (anakondo--upsert-var-def-cache t (var-def-cache-table var-defs &optional invalidation-ns) nil nil) (anakondo--upsert-ns-def-cache t (ns-def-cache-table ns-defs) nil nil) (anakondo--upsert-ns-usage-cache t (ns-usage-cache-table ns-usages &optional invalidation-ns) nil nil) (anakondo--clj-kondo-project-analyse-sync t (var-def-cache-table ns-def-cache-table ns-usage-cache-table) nil nil) (anakondo--clj-kondo-buffer-analyse-sync t (var-def-cache-table ns-def-cache-table ns-usage-cache-table) nil nil) (anakondo--jar-analize-sync t (classpath-list) nil nil) (anakondo--make-class-map t (class-name methods-and-fields) nil nil) (anakondo--java-analyze-class-map t (classpath class) nil nil) (anakondo--get-java-boot-classpath-list t nil nil nil) (anakondo--get-java-analysis-classpath t (as) nil nil) (anakondo--java-project-analyse-sync t (java-classes-cache) nil nil) (anakondo--safe-hash-table-values t (hash-table) nil nil) (anakondo--get-clj-kondo-completion-candidates t nil nil nil) (anakondo--get-local-completion-candidates t (prefix prefix-start) nil nil) (anakondo--get-java-completion-candidates t (prefix) nil nil) (anakondo-completion-at-point t nil nil nil) (anakondo--project-analyse-sync t (var-def-cache ns-def-cache ns-usage-cache java-classes-cache) nil nil) (anakondo--init-project-cache t (root) nil nil) (anakondo--delete-project-cache t (root) nil nil) (anakondo-minor-mode t (&optional arg) nil t) (anakondo-refresh-project-cache t nil nil t) (anakondo--minor-mode-enter t nil nil nil) (anakondo--minor-mode-exit t nil nil nil) (anakondo--minor-mode-guard t nil nil nil))"
    ];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn minor_mode_registration_lighter_and_empty_keymap_contract_match() {
    let elisp_form = r##"(list
                      (keymapp anakondo-minor-mode-map)
                      (keymap-prompt anakondo-minor-mode-map)
                      (where-is-internal
                       'anakondo-refresh-project-cache
                       anakondo-minor-mode-map)
                      (assq 'anakondo-minor-mode
                            minor-mode-alist)
                      (assq 'anakondo-minor-mode
                            minor-mode-map-alist)
                      (local-variable-if-set-p
                       'anakondo-minor-mode)
                      (commandp 'anakondo-minor-mode)
                      (commandp
                       'anakondo-refresh-project-cache))"##;
    let expect = expect![[
        r#"OK (t "Anakondo minor mode map" nil (anakondo-minor-mode anakondo-minor-mode-lighter) (anakondo-minor-mode keymap "Anakondo minor mode map") t t t)"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}
