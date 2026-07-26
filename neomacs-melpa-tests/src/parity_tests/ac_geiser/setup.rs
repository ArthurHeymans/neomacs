use expect_test::expect;

use super::assert_ac_geiser_parity;

#[test]
fn ac_geiser_setup_prepends_source_is_idempotent_and_is_interactive() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing-source)))
               (list
                (ac-geiser-setup)
                ac-sources
                (ac-geiser-setup)
                ac-sources
                (interactive-form
                 #'ac-geiser-setup)))"##;
    let expect =
        expect!["OK (#1=(ac-source-geiser existing-source) #1# #1# #1# (interactive nil))"];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_setup_keeps_an_existing_source_at_its_original_position() {
    let elisp_form = r##"(let ((ac-sources
                    '(before
                      ac-source-geiser
                      after)))
               (list
                (ac-geiser-setup)
                ac-sources))"##;
    let expect = expect!["OK (#1=(before ac-source-geiser after) #1#)"];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_setup_changes_only_each_buffers_local_source_list() {
    let elisp_form = r##"(let ((first
                    (get-buffer-create
                     " *ac-geiser-first*"))
                   (second
                    (get-buffer-create
                     " *ac-geiser-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq-local
                        ac-sources
                        '(first-source))
                       (ac-geiser-setup))
                     (with-current-buffer second
                       (setq-local
                        ac-sources
                        '(second-source))
                       (ac-geiser-setup)
                       (ac-geiser-setup))
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
        "OK ((ac-source-geiser first-source) (ac-source-geiser second-source) (ac-source-words-in-same-mode-buffers))"
    ];

    assert_ac_geiser_parity(elisp_form, expect);
}
