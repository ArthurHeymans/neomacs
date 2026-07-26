use expect_test::expect;

use super::assert_ac_html_csswatcher_parity;

#[test]
fn ac_html_csswatcher_activation_prepends_project_once_and_makes_sources_local() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                web-completion-data-sources
                '(("Fixture" . fixture-source)))
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'ac-html-csswatcher-setup-html-stuff-async)
                       (lambda ()
                         (push
                          (list
                           'async
                           (buffer-file-name))
                          events)
                         'started)))
                   (let ((first
                          (ac-html-csswatcher+))
                         (after-first
                          web-completion-data-sources))
                     (let ((second
                            (ac-html-csswatcher+)))
                       (list
                        first
                        after-first
                        second
                        web-completion-data-sources
                        (eq
                         after-first
                         web-completion-data-sources)
                        (local-variable-p
                         'web-completion-data-sources)
                        (nreverse events)))))))"##;
    let expect = expect![[
        r#"OK (started #1=(("Project" . ac-html-csswatcher-source-dir) ("Fixture" . fixture-source)) started #1# t t ((async nil) (async nil)))"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_activation_preserves_existing_project_entry_and_position() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                web-completion-data-sources
                '(("Before" . before-source)
                  ("Project" . custom-project-source)
                  ("After" . after-source)))
               (let ((before
                      web-completion-data-sources)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ac-html-csswatcher-setup-html-stuff-async)
                       (lambda ()
                         (setq calls
                               (1+ (or calls 0)))
                         'refreshed)))
                   (list
                    (ac-html-csswatcher+)
                    web-completion-data-sources
                    (eq
                     before
                     web-completion-data-sources)
                    calls
                    (local-variable-p
                     'web-completion-data-sources)))))"##;
    let expect = expect![[
        r#"OK (refreshed (("Before" . before-source) ("Project" . custom-project-source) ("After" . after-source)) t 1 t)"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_activation_is_buffer_local_and_preserves_the_default() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'web-completion-data-sources))
                   (first
                    (get-buffer-create
                     " *ac-html-csswatcher first*"))
                   (second
                    (get-buffer-create
                     " *ac-html-csswatcher second*")))
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'ac-html-csswatcher-setup-html-stuff-async)
                         #'ignore))
                     (with-current-buffer first
                       (ac-html-csswatcher+))
                     (list
                      (with-current-buffer first
                        web-completion-data-sources)
                      (with-current-buffer second
                        web-completion-data-sources)
                      (eq
                       default-before
                       (default-value
                        'web-completion-data-sources))
                      (default-value
                       'web-completion-data-sources)))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((("Project" . ac-html-csswatcher-source-dir) . #1=(("html" . web-completion-data-html-source-dir))) #1# t #1#)"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_refresh_and_company_alias_forward_each_call_exactly_once() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-html-csswatcher-setup-html-stuff-async)
                     (lambda ()
                       (push
                        (length events)
                        events)
                       (length events))))
                 (list
                  (ac-html-csswatcher-refresh)
                  (company-web-csswatcher-refresh)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (1 2 (0 1))"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_async_setup_is_a_noop_without_a_visited_file() {
    let elisp_form = r##"(with-temp-buffer
               (let (calls)
                 (cl-letf
                     (((symbol-function
                        'start-process)
                       (lambda (&rest arguments)
                         (push arguments calls)
                         (error
                          "start-process must not run"))))
                   (list
                    (ac-html-csswatcher-setup-html-stuff-async)
                    calls
                    ac-html-csswatcher-source-dir))))"##;
    let expect = expect![[r#"OK (nil nil nil)"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}
