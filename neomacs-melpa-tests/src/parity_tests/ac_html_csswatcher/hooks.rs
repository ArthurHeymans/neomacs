use expect_test::expect;

use super::assert_ac_html_csswatcher_parity;

#[test]
fn ac_html_csswatcher_setup_installs_the_exact_mode_hook_families() {
    let elisp_form = r##"(progn
               (setq
                html-mode-hook '(html-existing)
                web-mode-hook '(web-existing)
                slim-mode-hook '(slim-existing)
                jade-mode-hook '(jade-existing)
                haml-mode-hook '(haml-existing)
                css-mode-hook '(css-existing)
                less-mode-hook '(less-existing))
               (let ((result
                      (ac-html-csswatcher-setup)))
                 (list
                  result
                  html-mode-hook
                  web-mode-hook
                  slim-mode-hook
                  jade-mode-hook
                  haml-mode-hook
                  (list
                   (length css-mode-hook)
                   (functionp
                    (car css-mode-hook))
                   (eq
                    (cadr css-mode-hook)
                    'css-existing))
                  (list
                   (length less-mode-hook)
                   (functionp
                    (car less-mode-hook))
                   (eq
                    (cadr less-mode-hook)
                    'less-existing)))))"##;
    let expect = expect![[
        r#"OK ((css-mode-hook less-mode-hook) (ac-html-csswatcher+ html-existing) (ac-html-csswatcher+ web-existing) (ac-html-csswatcher+ slim-existing) (ac-html-csswatcher+ jade-existing) (ac-html-csswatcher+ haml-existing) (2 t t) (2 t t))"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_setup_is_idempotent_for_every_installed_hook() {
    let elisp_form = r##"(progn
               (setq
                html-mode-hook nil
                web-mode-hook nil
                slim-mode-hook nil
                jade-mode-hook nil
                haml-mode-hook nil
                css-mode-hook nil
                less-mode-hook nil)
               (let ((first
                      (ac-html-csswatcher-setup))
                     (second
                      (ac-html-csswatcher-setup)))
                 (list
                  first
                  second
                  (mapcar
                   (lambda (hook)
                     (length
                      (symbol-value hook)))
                   '(html-mode-hook
                     web-mode-hook
                     slim-mode-hook
                     jade-mode-hook
                     haml-mode-hook
                     css-mode-hook
                     less-mode-hook)))))"##;
    let expect = expect![[r#"OK (#1=(css-mode-hook less-mode-hook) #1# (1 1 1 1 1 1 1))"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_style_hooks_install_a_buffer_local_after_save_refresh() {
    let elisp_form = r##"(progn
               (setq
                css-mode-hook nil
                less-mode-hook nil)
               (setq-default
                after-save-hook
                '(fixture-global-after-save))
               (ac-html-csswatcher-setup)
               (mapcar
                (lambda (hook)
                  (with-temp-buffer
                    (run-hooks hook)
                    (list
                     hook
                     (local-variable-p
                      'after-save-hook)
                     after-save-hook)))
                '(css-mode-hook
                  less-mode-hook)))"##;
    let expect = expect![[
        r#"OK ((css-mode-hook t (ac-html-csswatcher-setup-html-stuff-async t)) (less-mode-hook t (ac-html-csswatcher-setup-html-stuff-async t)))"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_web_hook_activates_project_completion_in_the_target_buffer() {
    let elisp_form = r##"(progn
               (setq
                html-mode-hook nil
                web-mode-hook nil
                slim-mode-hook nil
                jade-mode-hook nil
                haml-mode-hook nil)
               (ac-html-csswatcher-setup)
               (with-temp-buffer
                 (setq
                  web-completion-data-sources
                  nil)
                 (let (events)
                   (cl-letf
                       (((symbol-function
                          'ac-html-csswatcher-setup-html-stuff-async)
                         (lambda ()
                           (push 'refresh events)
                           'started)))
                     (list
                      (run-hooks
                       'html-mode-hook)
                      web-completion-data-sources
                      (local-variable-p
                       'web-completion-data-sources)
                      (nreverse events))))))"##;
    let expect = expect![[r#"OK (nil (("Project" . ac-html-csswatcher-source-dir)) t (refresh))"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}
