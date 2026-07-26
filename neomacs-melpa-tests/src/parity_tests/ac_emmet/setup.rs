use expect_test::expect;

use super::assert_ac_emmet_parity;

#[test]
fn ac_emmet_html_setup_prepends_aliases_then_snippets_and_is_idempotent() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing-source)))
               (list
                (ac-emmet-html-setup)
                ac-sources
                (ac-emmet-html-setup)
                ac-sources
                (interactive-form
                 #'ac-emmet-html-setup)))"##;
    let expect = expect![
        "OK (#1=(ac-source-emmet-html-aliases ac-source-emmet-html-snippets existing-source) #1# #1# #1# (interactive nil))"
    ];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_css_setup_prepends_one_source_and_is_idempotent() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing-source)))
               (list
                (ac-emmet-css-setup)
                ac-sources
                (ac-emmet-css-setup)
                ac-sources
                (interactive-form
                 #'ac-emmet-css-setup)))"##;
    let expect = expect![
        "OK (#1=(ac-source-emmet-css-snippets existing-source) #1# #1# #1# (interactive nil))"
    ];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_html_and_css_setup_compose_in_call_order_without_duplicates() {
    let elisp_form = r##"(let ((ac-sources nil))
               (ac-emmet-html-setup)
               (ac-emmet-css-setup)
               (ac-emmet-html-setup)
               (ac-emmet-css-setup)
               ac-sources)"##;
    let expect = expect![
        "OK (ac-source-emmet-css-snippets ac-source-emmet-html-aliases ac-source-emmet-html-snippets)"
    ];

    assert_ac_emmet_parity(elisp_form, expect);
}

#[test]
fn ac_emmet_setup_changes_only_the_current_buffers_local_ac_sources() {
    let elisp_form = r##"(let ((first
                    (get-buffer-create
                     " *ac-emmet-first*"))
                   (second
                    (get-buffer-create
                     " *ac-emmet-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq-local
                        ac-sources
                        '(first-source))
                       (ac-emmet-html-setup))
                     (with-current-buffer second
                       (setq-local
                        ac-sources
                        '(second-source))
                       (ac-emmet-css-setup))
                     (list
                      (with-current-buffer first
                        ac-sources)
                      (with-current-buffer second
                        ac-sources)
                      (default-value
                       'ac-sources)))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect![
        "OK ((ac-source-emmet-html-aliases ac-source-emmet-html-snippets first-source) (ac-source-emmet-css-snippets second-source) (ac-source-words-in-same-mode-buffers))"
    ];

    assert_ac_emmet_parity(elisp_form, expect);
}
