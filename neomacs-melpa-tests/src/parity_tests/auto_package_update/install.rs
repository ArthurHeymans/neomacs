use expect_test::expect;

use super::assert_auto_package_update_parity;

#[test]
fn auto_package_update_old_version_queue_uses_installed_descriptor_directory_and_deduplicates() {
    let elisp_form = r##"(let*
                             ((first
                               (auto-package-update-test-desc
                                'alpha
                                '(1 0)
                                nil
                                "/packages/alpha-1/"))
                              (second
                               (auto-package-update-test-desc
                                'alpha
                                '(0 9)
                                nil
                                "/packages/alpha-0.9/"))
                              (package-alist
                               `((alpha ,first ,second)))
                              (apu--old-versions-dirs-list
                               '("/packages/existing/")))
                           (list
                            (apu--add-to-old-versions-dirs-list
                             'alpha)
                            (apu--add-to-old-versions-dirs-list
                             'alpha)
                            apu--old-versions-dirs-list))"##;
    let expect = expect![[r#"OK (#1=("/packages/alpha-1/" "/packages/existing/") #1# #1#)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_old_version_cleanup_recursively_deletes_real_sandbox_directories() {
    let elisp_form = r##"(let*
                             ((root
                               (auto-package-update-test-root
                                "delete-old"))
                              (alpha
                               (auto-package-update-test-path
                                root
                                "alpha-1.0/"))
                              (beta
                               (auto-package-update-test-path
                                root
                                "nested/beta-2.0/"))
                              (alpha-file
                               (auto-package-update-test-path
                                alpha
                                "alpha.el"))
                              (beta-file
                               (auto-package-update-test-path
                                beta
                                "data/state")))
                           (auto-package-update-test-write
                            alpha-file
                            "alpha")
                           (auto-package-update-test-write
                            beta-file
                            "beta")
                           (let
                               ((apu--old-versions-dirs-list
                                 (list alpha beta)))
                             (let ((result
                                    (apu--delete-old-versions-dirs-list)))
                               (list
                                result
                                apu--old-versions-dirs-list
                                (file-exists-p alpha)
                                (file-exists-p beta)
                                (file-exists-p alpha-file)
                                (file-exists-p beta-file)
                                (file-directory-p root)))))"##;
    let expect = expect!["OK (nil nil nil nil nil nil t)"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_safe_install_computes_dependency_transaction_and_downloads_it() {
    let elisp_form = r##"(let*
                             ((descriptor
                               (auto-package-update-test-desc
                                'alpha
                                '(2 1)
                                '((dep-one (1 0))
                                  (dep-two (3 4)))))
                              (package-archive-contents
                               `((alpha ,descriptor)))
                              (auto-package-update-delete-old-versions
                               nil)
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'package-compute-transaction)
                                 (lambda (packages requirements)
                                   (push
                                    (list
                                     :compute
                                     (mapcar
                                      #'package-desc-name
                                      packages)
                                     requirements)
                                    calls)
                                   (list
                                    descriptor
                                    'dependency-descriptor)))
                                ((symbol-function
                                  'package-download-transaction)
                                 (lambda (transaction)
                                   (push
                                    (list
                                     :download
                                     (mapcar
                                      (lambda (entry)
                                        (if
                                            (package-desc-p entry)
                                            (package-desc-name entry)
                                          entry))
                                      transaction))
                                    calls)
                                   :downloaded)))
                             (list
                              (apu--safe-package-install
                               'alpha)
                              (nreverse calls)
                              apu--old-versions-dirs-list)))"##;
    let expect = expect![[
        r#"OK ("alpha up to date." ((:compute (alpha) ((dep-one (1 0)) (dep-two (3 4)))) (:download (alpha dependency-descriptor))) nil)"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_safe_install_converts_missing_and_download_errors_to_reports() {
    let elisp_form = r##"(let*
                             ((good
                               (auto-package-update-test-desc
                                'good
                                '(1 0)))
                              (broken
                               (auto-package-update-test-desc
                                'broken
                                '(2 0)))
                              (package-archive-contents
                               `((good ,good)
                                 (broken ,broken)))
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'package-compute-transaction)
                                 (lambda (packages _requirements)
                                   (let ((name
                                          (package-desc-name
                                           (car packages))))
                                     (push
                                      (list :compute name)
                                      calls)
                                     (list (car packages)))))
                                ((symbol-function
                                  'package-download-transaction)
                                 (lambda (transaction)
                                   (let ((name
                                          (package-desc-name
                                           (car transaction))))
                                     (push
                                      (list :download name)
                                      calls)
                                     (if
                                         (eq name 'broken)
                                         (error
                                          "fixture download failure")
                                       :downloaded)))))
                             (list
                              (apu--safe-package-install
                               'good)
                              (apu--safe-package-install
                               'broken)
                              (apu--safe-package-install
                               'missing)
                              (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("good up to date." "Error installing broken" "Error installing missing" ((:compute good) (:download good) (:compute broken) (:download broken)))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_safe_install_packages_deletes_all_queued_old_versions_after_downloads() {
    let elisp_form = r##"(let*
                             ((root
                               (auto-package-update-test-root
                                "install-and-delete"))
                              (alpha-old
                               (auto-package-update-test-path
                                root
                                "alpha-1/"))
                              (beta-old
                               (auto-package-update-test-path
                                root
                                "beta-1/"))
                              (alpha-installed
                               (auto-package-update-test-desc
                                'alpha
                                '(1 0)
                                nil
                                alpha-old))
                              (beta-installed
                               (auto-package-update-test-desc
                                'beta
                                '(1 0)
                                nil
                                beta-old))
                              (alpha-new
                               (auto-package-update-test-desc
                                'alpha
                                '(2 0)))
                              (beta-new
                               (auto-package-update-test-desc
                                'beta
                                '(3 0)))
                              (package-alist
                               `((alpha ,alpha-installed)
                                 (beta ,beta-installed)))
                              (package-archive-contents
                               `((alpha ,alpha-new)
                                 (beta ,beta-new)))
                              (auto-package-update-delete-old-versions
                               t)
                              (apu--old-versions-dirs-list nil)
                              calls)
                           (auto-package-update-test-write
                            (auto-package-update-test-path
                             alpha-old
                             "alpha.el")
                            "old alpha")
                           (auto-package-update-test-write
                            (auto-package-update-test-path
                             beta-old
                             "beta.el")
                            "old beta")
                           (cl-letf
                               (((symbol-function
                                  'package-compute-transaction)
                                 (lambda (packages _requirements)
                                   packages))
                                ((symbol-function
                                  'package-download-transaction)
                                 (lambda (transaction)
                                   (push
                                    (package-desc-name
                                     (car transaction))
                                    calls)
                                   :downloaded)))
                             (list
                              (apu--safe-install-packages
                               '(alpha beta))
                              (nreverse calls)
                              apu--old-versions-dirs-list
                              (file-exists-p alpha-old)
                              (file-exists-p beta-old)
                              (file-directory-p root))))"##;
    let expect =
        expect![[r#"OK (("beta up to date." "alpha up to date.") (alpha beta) nil nil nil t)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_safe_install_packages_preserves_attempt_order_but_reports_unique_results() {
    let elisp_form = r##"(let (calls)
                           (cl-letf
                               (((symbol-function
                                  'apu--safe-package-install)
                                 (lambda (package)
                                   (push package calls)
                                   (if
                                       (memq
                                        package
                                        '(alpha beta))
                                       "shared report"
                                     (format
                                      "%s report"
                                      package)))))
                             (let
                                 ((auto-package-update-delete-old-versions
                                   nil))
                               (list
                                (apu--safe-install-packages
                                 '(alpha beta gamma alpha))
                                (nreverse calls)))))"##;
    let expect = expect![[r#"OK (("gamma report" "shared report") (alpha beta gamma alpha))"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_cleanup_is_skipped_when_delete_old_versions_option_is_nil() {
    let elisp_form = r##"(let
                             ((auto-package-update-delete-old-versions
                               nil)
                              (apu--old-versions-dirs-list
                               '("/must/remain"))
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'apu--safe-package-install)
                                 (lambda (package)
                                   (push package calls)
                                   (format "%s done" package)))
                                ((symbol-function
                                  'apu--delete-old-versions-dirs-list)
                                 (lambda ()
                                   (error
                                    "cleanup must be skipped"))))
                             (list
                              (apu--safe-install-packages
                               '(alpha beta))
                              (nreverse calls)
                              apu--old-versions-dirs-list)))"##;
    let expect = expect![[r#"OK (("beta done" "alpha done") (alpha beta) ("/must/remain"))"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}
