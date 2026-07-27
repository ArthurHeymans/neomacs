use expect_test::expect;

use super::assert_ameba_parity;

#[test]
fn every_supported_marker_discovers_a_nested_real_project_root() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          rows)
                      (dolist
                          (marker
                           '(".projectile" ".git" ".hg"
                             ".ameba.yml" "shard.yml"))
                        (let* ((slug
                                (replace-regexp-in-string
                                 "[^[:alnum:]]" "-" marker))
                               (project
                                (file-name-as-directory
                                 (expand-file-name slug sandbox)))
                               (nested
                                (file-name-as-directory
                                 (expand-file-name
                                  "src/domain/models" project)))
                               (default-directory nested)
                               (ameba-project-root-files
                                (list marker)))
                          (make-directory nested t)
                          (if (member marker '(".git" ".hg"))
                              (make-directory
                               (expand-file-name marker project))
                            (with-temp-file
                                (expand-file-name marker project)
                              (insert marker)))
                          (push
                           (list
                            marker
                            (file-relative-name
                             (ameba-project-root) sandbox))
                           rows)))
                      (nreverse rows))"##;
    let expect = expect![[
        r#"OK ((".projectile" "-projectile/") (".git" "-git/") (".hg" "-hg/") (".ameba.yml" "-ameba-yml/") ("shard.yml" "shard-yml/"))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn configured_marker_order_beats_nearer_markers_and_custom_order_changes_selection() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (outer
                           (file-name-as-directory
                            (expand-file-name "outer" sandbox)))
                          (inner
                           (file-name-as-directory
                            (expand-file-name "packages/inner" outer)))
                          (nested
                           (file-name-as-directory
                            (expand-file-name "src/deep" inner)))
                          (default-directory nested))
                      (make-directory nested t)
                      (make-directory
                       (expand-file-name ".git" outer))
                      (with-temp-file
                          (expand-file-name ".ameba.yml" inner)
                        (insert "Lint/Formatting:\n  Enabled: true\n"))
                      (list
                       (file-relative-name
                        (ameba-project-root) sandbox)
                       (let ((ameba-project-root-files
                              '(".ameba.yml" ".git")))
                         (file-relative-name
                          (ameba-project-root) sandbox))
                       (let ((ameba-project-root-files
                              '("missing" ".git" ".ameba.yml")))
                         (file-relative-name
                          (ameba-project-root) sandbox))))"##;
    let expect = expect![[r#"OK ("outer/" "outer/packages/inner/" "outer/")"#]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn missing_project_supports_optional_probe_and_emits_the_exact_required_error() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (outside
                           (file-name-as-directory
                            (expand-file-name "outside/deep" sandbox)))
                          (default-directory outside)
                          (ameba-project-root-files
                           '("marker-that-does-not-exist")))
                      (make-directory outside t)
                      (list
                       (ameba-project-root t)
                       (ameba-project-root 'no-error)
                       (condition-case error-data
                           (ameba-project-root)
                         (error
                          (list
                           (car error-data)
                           (cdr error-data))))
                       (let ((ameba-project-root-files nil))
                         (ameba-project-root t))))"##;
    let expect = expect![[r#"OK (nil nil (error ("You’re not into a project")) nil)"#]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn project_root_normalizes_relative_and_parent_traversal_default_directories() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (project
                           (file-name-as-directory
                            (expand-file-name "normalize" sandbox)))
                          (nested
                           (file-name-as-directory
                            (expand-file-name "src/one/two" project))))
                      (make-directory nested t)
                      (with-temp-file
                          (expand-file-name ".projectile" project)
                        (insert "root"))
                      (let ((default-directory
                             (concat nested "../../one/./two/")))
                        (list
                         (ameba-project-root)
                         (file-relative-name
                          (ameba-project-root) sandbox)
                         (file-name-absolute-p
                          (ameba-project-root))
                         (string-suffix-p
                          "/" (ameba-project-root)))))"##;
    let expect = expect![[r#"OK ("[ORACLE-SANDBOX]/normalize/" "normalize/" t t)"#]];
    assert_ameba_parity(elisp_form, expect);
}
