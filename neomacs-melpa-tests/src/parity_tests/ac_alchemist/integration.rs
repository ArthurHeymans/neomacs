use expect_test::expect;

use super::assert_ac_alchemist_parity;

#[test]
fn ac_alchemist_exact_pin_dependencies_surface_defaults_group_and_source_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-alchemist package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(auto-complete
                   alchemist
                   cl-lib
                   ac-alchemist))
                (mapcar
                 #'fboundp
                 '(ac-alchemist--candidates
                   ac-alchemist--merge-candidates
                   ac-alchemist--complete-filter
                   ac-alchemist--do-complete
                   ac-alchemist--get-prefixed-string
                   ac-alchemist--complete-request
                   alchemist-company-doc-buffer-filter
                   ac-alchemist--document-query
                   ac-alchemist--show-document
                   ac-alchemist--prefix
                   ac-alchemist-setup))
                (mapcar
                 #'commandp
                 '(ac-alchemist--candidates
                   ac-alchemist--merge-candidates
                   ac-alchemist--complete-filter
                   ac-alchemist--do-complete
                   ac-alchemist--complete-request
                   ac-alchemist--show-document
                   ac-alchemist--prefix
                   ac-alchemist-setup))
                (mapcar
                 #'symbol-value
                 '(ac-alchemist--output-cache
                   ac-alchemist--candidate-cache
                   ac-alchemist--prefix
                   ac-alchemist--document))
                (get 'ac-alchemist 'group-documentation)
                (get 'ac-alchemist 'custom-group)
                ac-source-alchemist))"##;
    let expect = expect![[
        r#"OK (ac-alchemist "20150908.656" ((auto-complete (1 5 0)) (alchemist (1 5 0)) (cl-lib (0 5))) (t t t t) (t t t t t t t t t t t) (nil nil nil nil nil nil nil t) (nil nil nil nil) "auto complete source of alchemist" nil ((init . ac-alchemist--complete-request) (prefix . ac-alchemist--prefix) (candidates . ac-alchemist--candidates) (document . ac-alchemist--show-document) (requires . -1)))"#
    ]];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_setup_enables_auto_complete_and_prepends_source_once() {
    let elisp_form = r##"(with-temp-buffer
               (let ((ac-sources
                      '(existing-source)))
                 (list
                  (ac-alchemist-setup)
                  auto-complete-mode
                  ac-sources
                  (ac-alchemist-setup)
                  auto-complete-mode
                  ac-sources
                  (local-variable-p
                   'auto-complete-mode)
                  (local-variable-p
                   'ac-sources))))"##;
    let expect = expect!["OK (#1=(ac-source-alchemist existing-source) t #1# #1# t #1# t nil)"];

    assert_ac_alchemist_parity(elisp_form, expect);
}

#[test]
fn ac_alchemist_setup_calls_mode_before_source_registration_and_returns_the_new_list() {
    let elisp_form = r##"(let ((ac-sources '(existing))
                    events)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (argument)
                       (push
                        (list
                         'mode
                         argument
                         (copy-sequence ac-sources))
                        events)
                       'mode-result)))
                 (list
                  (ac-alchemist-setup)
                  ac-sources
                  (nreverse events))))"##;
    let expect = expect!["OK (#1=(ac-source-alchemist existing) #1# ((mode 1 (existing))))"];

    assert_ac_alchemist_parity(elisp_form, expect);
}
