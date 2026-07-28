use expect_test::expect;

use super::assert_auto_package_update_parity;

#[test]
fn auto_package_update_up_to_date_predicate_compares_archive_installed_and_builtin_versions() {
    let elisp_form = r##"(let
                             ((run-case
                               (lambda
                                   (archive-version
                                    installed-version
                                    installed-p
                                    builtin)
                                 (let*
                                     ((archive-desc
                                       (and
                                        archive-version
                                        (auto-package-update-test-desc
                                         'fixture
                                         archive-version)))
                                      (installed-desc
                                       (and
                                        installed-version
                                        (auto-package-update-test-desc
                                         'fixture
                                         installed-version)))
                                      (package-archive-contents
                                       (and
                                        archive-desc
                                        (list
                                         (list
                                          'fixture
                                          archive-desc))))
                                      (package-alist
                                       (and
                                        installed-desc
                                        (not builtin)
                                        (list
                                         (list
                                          'fixture
                                          installed-desc))))
                                      (package--builtins
                                       (and
                                        installed-desc
                                        builtin
                                        (list
                                         (list
                                          'fixture
                                          installed-desc)))))
                                   (cl-letf
                                       (((symbol-function
                                          'package-installed-p)
                                         (lambda (_package)
                                           installed-p)))
                                     (list
                                      (apu--package-up-to-date-p
                                       'fixture)
                                      (apu--package-out-of-date-p
                                       'fixture)))))))
                           (list
                            (funcall run-case
                                     '(1 0) '(1 0) t nil)
                            (funcall run-case
                                     '(1 0) '(2 0) t nil)
                            (funcall run-case
                                     '(2 0) '(1 9) t nil)
                            (funcall run-case
                                     '(1 0) '(1 0) nil nil)
                            (funcall run-case
                                     nil '(1 0) t nil)
                            (funcall run-case
                                     '(2 0) '(3 0) t t)
                            (funcall run-case
                                     '(2 0 0 90)
                                     '(2 0 0)
                                     t
                                     nil)))"##;
    let expect = expect!["OK ((t nil) (t nil) (nil t) (nil t) (nil t) (t nil) (nil t))"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_packages_to_install_filters_exclusions_deduplicates_and_preserves_order() {
    let elisp_form = r##"(let*
                             ((desc
                               (lambda (name version)
                                 (auto-package-update-test-desc
                                  name
                                  version)))
                              (alpha-old
                               (funcall desc 'alpha '(1 0)))
                              (alpha-new
                               (funcall desc 'alpha '(2 0)))
                              (beta
                               (funcall desc 'beta '(1 0)))
                              (gamma-old
                               (funcall desc 'gamma '(1 0)))
                              (gamma-new
                               (funcall desc 'gamma '(3 0)))
                              (builtin
                               (funcall desc 'builtin '(5 0)))
                              (package-activated-list
                               '(alpha beta gamma alpha builtin missing))
                              (auto-package-update-excluded-packages
                               '(gamma ignored))
                              (package-alist
                               `((alpha ,alpha-old)
                                 (beta ,beta)
                                 (gamma ,gamma-old)))
                              (package--builtins
                               `((builtin ,builtin)))
                              (package-archive-contents
                               `((alpha ,alpha-new)
                                 (beta ,beta)
                                 (gamma ,gamma-new)
                                 (builtin ,builtin))))
                           (cl-letf
                               (((symbol-function
                                  'package-installed-p)
                                 (lambda (package)
                                   (memq
                                    package
                                    '(alpha beta gamma builtin)))))
                             (list
                              (apu--packages-to-install)
                              package-activated-list
                              auto-package-update-excluded-packages)))"##;
    let expect =
        expect!["OK ((alpha missing) (alpha beta gamma alpha builtin missing) (gamma ignored))"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_package_selection_handles_empty_and_all_excluded_sets() {
    let elisp_form = r##"(let
                             ((package-archive-contents nil)
                              (package-alist nil)
                              (package--builtins nil))
                           (list
                            (let
                                ((package-activated-list nil)
                                 (auto-package-update-excluded-packages
                                  nil))
                              (apu--packages-to-install))
                            (let
                                ((package-activated-list
                                  '(alpha beta alpha))
                                 (auto-package-update-excluded-packages
                                  '(alpha beta)))
                              (apu--packages-to-install))
                            (let
                                ((package-activated-list
                                  '(alpha alpha beta))
                                 (auto-package-update-excluded-packages
                                  nil))
                              (cl-letf
                                  (((symbol-function
                                     'package-installed-p)
                                    (lambda (_package) nil)))
                                (apu--packages-to-install)))))"##;
    let expect = expect!["OK (nil nil (alpha beta))"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_quelpa_filter_without_quelpa_returns_packages_unchanged() {
    let elisp_form = r##"(let ((packages
                                (list
                                 'alpha
                                 'beta
                                 'alpha)))
                           (let ((result
                                  (apu--filter-quelpa-packages
                                   packages)))
                             (list
                              result
                              packages
                              (eq result packages)
                              (featurep 'quelpa))))"##;
    let expect = expect!["OK (#1=(alpha beta alpha) #1# t nil)"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_quelpa_filter_reads_cache_and_removes_each_cached_name() {
    let elisp_form = r##"(let
                             ((packages
                               (list
                                'alpha
                                'beta
                                'gamma
                                'delta))
                              calls)
                           (provide 'quelpa)
                           (cl-letf
                               (((symbol-function
                                  'quelpa-read-cache)
                                 (lambda ()
                                   (push :read-cache calls)
                                   (setq
                                    quelpa-cache
                                    '((beta . first)
                                      (delta . second)
                                      (absent . third))))))
                             (let ((result
                                    (apu--filter-quelpa-packages
                                     packages)))
                               (list
                                result
                                packages
                                (nreverse calls)
                                quelpa-cache
                                (featurep 'quelpa)))))"##;
    let expect = expect![
        "OK (#1=(alpha gamma) #1# (:read-cache) ((beta . first) (delta . second) (absent . third)) t)"
    ];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_preview_refreshes_and_renders_real_pending_package_buffer() {
    let elisp_form = r##"(let
                             ((auto-package-preview-buffer-name
                               " *apu-pending-preview*")
                              refreshes)
                           (unwind-protect
                               (cl-letf
                                   (((symbol-function
                                      'package-refresh-contents)
                                     (lambda (&rest arguments)
                                       (push arguments refreshes)
                                       :refreshed))
                                    ((symbol-function
                                      'apu--packages-to-install)
                                     (lambda ()
                                       '(alpha beta-long gamma))))
                                 (let ((up-to-date
                                        (apu--show-preview)))
                                   (with-current-buffer
                                       auto-package-preview-buffer-name
                                     (list
                                      up-to-date
                                      (nreverse refreshes)
                                      (buffer-string)
                                      buffer-read-only
                                      auto-package-update-minor-mode
                                      (key-binding (kbd "q"))
                                      (buffer-modified-p)))))
                             (auto-package-update-test-kill-buffers
                              auto-package-preview-buffer-name)))"##;
    let expect = expect![[
        r#"OK (nil (nil) "[PACKAGES TO UPDATE]:\nalpha\nbeta-long\ngamma" t t quit-window t)"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_preview_reports_all_up_to_date_without_package_lines() {
    let elisp_form = r##"(let
                             ((auto-package-preview-buffer-name
                               " *apu-empty-preview*")
                              refreshes)
                           (unwind-protect
                               (cl-letf
                                   (((symbol-function
                                      'package-refresh-contents)
                                     (lambda (&rest arguments)
                                       (push arguments refreshes)))
                                    ((symbol-function
                                      'apu--packages-to-install)
                                     (lambda () nil)))
                                 (let ((up-to-date
                                        (apu--show-preview)))
                                   (with-current-buffer
                                       auto-package-preview-buffer-name
                                     (list
                                      up-to-date
                                      (nreverse refreshes)
                                      (buffer-string)
                                      buffer-read-only
                                      auto-package-update-minor-mode))))
                             (auto-package-update-test-kill-buffers
                              auto-package-preview-buffer-name)))"##;
    let expect = expect![[r#"OK (t (nil) "[PACKAGES TO UPDATE]:\nAll packages up to date" t t)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}
