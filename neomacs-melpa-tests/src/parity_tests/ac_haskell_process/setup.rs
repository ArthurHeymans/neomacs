use expect_test::expect;

use super::assert_ac_haskell_process_parity;

#[test]
fn ac_haskell_process_setup_prepends_source_is_idempotent_and_is_interactive() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing-source)))
               (list
                (ac-haskell-process-setup)
                ac-sources
                (ac-haskell-process-setup)
                ac-sources
                (interactive-form
                 #'ac-haskell-process-setup)))"##;
    let expect = expect![
        "OK (#1=(ac-source-haskell-process existing-source) #1# #1# #1# (interactive nil))"
    ];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_setup_keeps_existing_source_position() {
    let elisp_form = r##"(let ((ac-sources
                    '(before
                      ac-source-haskell-process
                      after)))
               (list
                (ac-haskell-process-setup)
                ac-sources))"##;
    let expect = expect!["OK (#1=(before ac-source-haskell-process after) #1#)"];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_setup_changes_only_each_buffers_local_sources() {
    let elisp_form = r##"(let ((first
                    (get-buffer-create
                     " *ac-haskell-first*"))
                   (second
                    (get-buffer-create
                     " *ac-haskell-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq-local
                        ac-sources
                        '(first-source))
                       (ac-haskell-process-setup))
                     (with-current-buffer second
                       (setq-local
                        ac-sources
                        '(second-source))
                       (ac-haskell-process-setup)
                       (ac-haskell-process-setup))
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
        "OK ((ac-source-haskell-process first-source) (ac-source-haskell-process second-source) (ac-source-words-in-same-mode-buffers))"
    ];

    assert_ac_haskell_process_parity(elisp_form, expect);
}
