use expect_test::expect;

use super::assert_agitjo_parity;

#[test]
fn agitjo_loads_the_pinned_package_with_its_real_editor_and_git_dependencies() {
    let elisp_form = r##"(let ((descriptor
                                (cadr
                                 (assq
                                  'agitjo
                                  package-alist))))
                           (list
                            (package-version-join
                             (package-desc-version descriptor))
                            (package-desc-reqs descriptor)
                            (featurep
                             'agitjo)
                            (mapcar
                             (lambda (feature)
                               (and
                                (featurep feature)
                                feature))
                             '(magit
                               markdown-mode
                               transient))
                            (file-name-base
                             (locate-library
                              "agitjo"))
                            (mapcar
                             (lambda (command)
                               (list
                                command
                                (commandp command)))
                             '(agitjo-setup
                               agitjo-push
                               agitjo-post-confirm
                               agitjo-post-cancel))))"##;
    let expect = expect![[
        r#"OK ("20260523.2048" ((emacs (30 1)) (magit (4 3 8)) (markdown-mode (2 7)) (transient (0 9 1))) t (magit markdown-mode transient) "agitjo" ((agitjo-setup nil) (agitjo-push t) (agitjo-post-confirm t) (agitjo-post-cancel t)))"#
    ]];

    assert_agitjo_parity(elisp_form, expect);
}
