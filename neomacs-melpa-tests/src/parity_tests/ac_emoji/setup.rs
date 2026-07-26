use expect_test::expect;

use super::assert_ac_emoji_parity;

#[test]
fn ac_emoji_setup_prepends_source_is_idempotent_and_is_interactive() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing-source)))
               (list
                (ac-emoji-setup)
                ac-sources
                (ac-emoji-setup)
                ac-sources
                (interactive-form
                 #'ac-emoji-setup)))"##;
    let expect = expect!["OK (#1=(ac-source-emoji existing-source) #1# #1# #1# (interactive nil))"];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_setup_changes_only_each_buffers_local_source_list() {
    let elisp_form = r##"(let ((first
                    (get-buffer-create
                     " *ac-emoji-first*"))
                   (second
                    (get-buffer-create
                     " *ac-emoji-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq-local
                        ac-sources
                        '(first-source))
                       (ac-emoji-setup))
                     (with-current-buffer second
                       (setq-local
                        ac-sources
                        '(second-source))
                       (ac-emoji-setup)
                       (ac-emoji-setup))
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
        "OK ((ac-source-emoji first-source) (ac-source-emoji second-source) (ac-source-words-in-same-mode-buffers))"
    ];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_source_candidates_are_live_variable_lookup_not_a_copied_list() {
    let elisp_form = r##"(let ((original
                    ac-emoji--candidates)
                   (replacement
                    '("fixture")))
               (unwind-protect
                   (progn
                     (setq ac-emoji--candidates
                           replacement)
                     (list
                      (cdr
                       (assq
                        'candidates
                        ac-source-emoji))
                      (symbol-value
                       (cdr
                        (assq
                         'candidates
                         ac-source-emoji)))
                      (eq
                       replacement
                       (symbol-value
                        (cdr
                         (assq
                          'candidates
                          ac-source-emoji))))))
                 (setq ac-emoji--candidates
                       original)))"##;
    let expect = expect![[r#"OK (ac-emoji--candidates ("fixture") t)"#]];

    assert_ac_emoji_parity(elisp_form, expect);
}
