use expect_test::expect;

use super::assert_aa_edit_mode_parity;

#[test]
fn aa_edit_mode_public_surface_dependencies_defaults_and_custom_metadata_match_the_pin() {
    let elisp_form = r##"(list
              (featurep 'aa-edit-mode)
              (featurep 'navi2ch)
              (featurep 'navi2ch-mona)
              (fboundp 'aa-edit-mode)
              (fboundp
               'aa-edit-mode--face)
              aa-edit-mlt-delimiter-regexp
              aa-edit-delimiter-pattern
              (get
               'aa-edit-delimiter-pattern
               'custom-type)
              (let ((standard
                     (get
                      'aa-edit-delimiter-pattern
                      'standard-value)))
                (list
                 (and
                  (consp standard)
                  t)
                 (eval
                  (car standard))))
              (get
               'aa-edit-delimiter-pattern
               'custom-group)
              (get 'aa-edit-mode
                   'derived-mode-parent)
              (car
               (split-string
                (documentation
                 'aa-edit-mode)
                "\n")))"##;
    let expect = expect![[
        r#"OK (t t t t t "^\\[SPLIT]" "^\\[SPLIT]" regexp (t "^\\[SPLIT]") nil text-mode "Major mode for editing AA")"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_delimiter_regexp_matches_only_literal_split_markers_at_line_start() {
    let elisp_form = r##"(mapcar
             (lambda (text)
               (string-match
                aa-edit-mlt-delimiter-regexp
                text))
             '("[SPLIT]"
               "[SPLIT]tail"
               "x\n[SPLIT]\n"
               " [SPLIT]"
               "x[SPLIT]"))"##;
    let expect = expect!["OK (0 0 2 nil nil)"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_face_selection_covers_unbound_true_custom_and_nil_values() {
    let elisp_form = r##"(let ((was-bound
                    (boundp
                     'navi2ch-mona-face-variable))
                   (old-value
                    (and
                     (boundp
                      'navi2ch-mona-face-variable)
                     navi2ch-mona-face-variable)))
               (unwind-protect
                   (progn
                     (makunbound
                      'navi2ch-mona-face-variable)
                     (list
                      (aa-edit-mode--face)
                      (progn
                        (set
                         'navi2ch-mona-face-variable
                         t)
                        (aa-edit-mode--face))
                      (progn
                        (set
                         'navi2ch-mona-face-variable
                         'custom-face)
                        (aa-edit-mode--face))
                      (progn
                        (set
                         'navi2ch-mona-face-variable
                         nil)
                        (aa-edit-mode--face))))
                 (if was-bound
                     (set
                      'navi2ch-mona-face-variable
                      old-value)
                   (makunbound
                    'navi2ch-mona-face-variable))))"##;
    let expect = expect![[r#"OK ("" navi2ch-mona16-face custom-face nil)"#]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_auto_mode_registration_targets_mlt_files_and_invokes_the_mode() {
    let elisp_form = r##"(let ((entry
                    (assoc
                     "\\.mlt\\'"
                     auto-mode-alist))
                   setup-events
                   face-events)
               (list
                (car entry)
                (eq
                 (cdr entry)
                 'aa-edit-mode)
                (with-temp-buffer
                  (setq buffer-file-name
                        "/workspace/sample.mlt")
                  (cl-letf
                      (((symbol-function
                         'navi2ch-mona-setup)
                        (lambda ()
                          (push
                           'setup
                           setup-events)))
                       ((symbol-function
                         'buffer-face-set)
                        (lambda (face)
                          (push face
                                face-events))))
                    (set-auto-mode)
                    (list
                     major-mode
                     mode-name
                     (nreverse
                      setup-events)
                     (nreverse
                      face-events))))))"##;
    let expect =
        expect![[r#"OK ("\\.mlt\\'" t (aa-edit-mode "（´д｀）" (setup) (navi2ch-mona16-face)))"#]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_auto_mode_entry_does_not_claim_non_mlt_suffixes() {
    let elisp_form = r##"(with-temp-buffer
             (setq buffer-file-name
                   "/workspace/sample.mlt.txt")
             (set-auto-mode)
             major-mode)"##;
    let expect = expect!["OK text-mode"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}
