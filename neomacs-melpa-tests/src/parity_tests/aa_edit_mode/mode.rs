use expect_test::expect;

use super::assert_aa_edit_mode_parity;

#[test]
fn aa_edit_mode_runs_setup_before_selecting_face_and_sets_complete_local_mode_state() {
    let elisp_form = r##"(let ((aa-edit-delimiter-pattern
                    "^CUT$")
                   (navi2ch-mona-face-variable
                    'before-face)
                   events)
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function
                        'navi2ch-mona-setup)
                       (lambda ()
                         (push
                          'setup events)
                         (setq
                          navi2ch-mona-face-variable
                          'after-face)))
                      ((symbol-function
                        'buffer-face-set)
                       (lambda (face)
                         (push
                          (list 'face face)
                          events))))
                   (aa-edit-mode)
                   (list
                    (nreverse events)
                    major-mode
                    mode-name
                    (derived-mode-p
                     'text-mode)
                    (eq
                     (current-local-map)
                     aa-edit-mode-map)
                    (local-variable-p
                     'page-delimiter)
                    page-delimiter))))"##;
    let expect = expect![[
        r#"OK ((setup (face after-face)) aa-edit-mode "（´д｀）" text-mode t t "^CUT$")"#
    ]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_setup_face_and_mode_hook_execute_in_that_order() {
    let elisp_form = r##"(let* ((events nil)
                   (navi2ch-mona-face-variable
                    t)
                   (aa-edit-mode-hook
                    (list
                     (lambda ()
                       (push 'hook events)))))
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function
                        'navi2ch-mona-setup)
                       (lambda ()
                         (push
                          'setup events)))
                      ((symbol-function
                        'buffer-face-set)
                       (lambda (face)
                         (push
                          (list 'face face)
                          events))))
                   (aa-edit-mode)
                   (nreverse events))))"##;
    let expect = expect!["OK (setup (face navi2ch-mona16-face) hook)"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_skips_optional_setup_when_the_function_is_unbound() {
    let elisp_form = r##"(let ((original
                    (and
                     (fboundp
                      'navi2ch-mona-setup)
                     (symbol-function
                      'navi2ch-mona-setup)))
                   face-events)
               (unwind-protect
                   (progn
                     (fmakunbound
                      'navi2ch-mona-setup)
                     (with-temp-buffer
                       (cl-letf
                           (((symbol-function
                              'buffer-face-set)
                             (lambda (face)
                               (push face
                                     face-events))))
                         (aa-edit-mode)
                         (list
                          (fboundp
                           'navi2ch-mona-setup)
                          major-mode
                          (nreverse
                           face-events)))))
                 (when original
                   (fset
                    'navi2ch-mona-setup
                    original))))"##;
    let expect = expect!["OK (nil aa-edit-mode (navi2ch-mona16-face))"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_uses_buffer_face_set_to_enable_the_real_buffer_local_face() {
    let elisp_form = r##"(let ((navi2ch-mona-face-variable
                    t))
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function
                        'navi2ch-mona-setup)
                       #'ignore))
                   (aa-edit-mode)
                   (list
                    buffer-face-mode
                    buffer-face-mode-face
                    (local-variable-p
                     'buffer-face-mode-face)))))"##;
    let expect = expect!["OK (t navi2ch-mona16-face t)"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_reentry_refreshes_page_delimiter_from_the_current_configuration() {
    let elisp_form = r##"(let ((aa-edit-delimiter-pattern
                    "^FIRST$")
                   first second)
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function
                        'navi2ch-mona-setup)
                       #'ignore)
                      ((symbol-function
                        'buffer-face-set)
                       #'ignore))
                   (aa-edit-mode)
                   (setq first
                         page-delimiter)
                   (setq
                    aa-edit-delimiter-pattern
                    "^SECOND$")
                   (aa-edit-mode)
                   (setq second
                         page-delimiter)
                   (list
                    first second
                    (local-variable-p
                     'page-delimiter)))))"##;
    let expect = expect![[r#"OK ("^FIRST$" "^SECOND$" t)"#]];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_selects_generated_map_and_syntax_table_and_reports_text_derivation() {
    let elisp_form = r##"(list
              (eq
               (keymap-parent
                aa-edit-mode-map)
               text-mode-map)
              (eq
               (char-table-parent
                aa-edit-mode-syntax-table)
               text-mode-syntax-table)
              (with-temp-buffer
                (cl-letf
                    (((symbol-function
                       'navi2ch-mona-setup)
                      #'ignore)
                     ((symbol-function
                       'buffer-face-set)
                      #'ignore))
                  (aa-edit-mode)
                  (list
                   (eq
                    (current-local-map)
                    aa-edit-mode-map)
                   (eq
                    (syntax-table)
                    aa-edit-mode-syntax-table)
                   (derived-mode-p
                    'text-mode
                    'aa-edit-mode)))))"##;
    let expect = expect!["OK (nil nil (t t text-mode))"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}

#[test]
fn aa_edit_mode_page_delimiter_drives_forward_and_backward_page_navigation() {
    let elisp_form = r##"(with-temp-buffer
             (insert
              "first\n[SPLIT]\nsecond\n[SPLIT]\nthird")
             (cl-letf
                 (((symbol-function
                    'navi2ch-mona-setup)
                   #'ignore)
                  ((symbol-function
                    'buffer-face-set)
                   #'ignore))
               (aa-edit-mode)
               (goto-char
                (point-min))
               (let ((start
                      (point)))
                 (forward-page 1)
                 (let ((after-forward
                        (point)))
                   (backward-page 1)
                   (list
                    start
                    after-forward
                    (point)
                    (line-number-at-pos
                     after-forward)
                    (char-before
                     after-forward)
                    (char-after
                     after-forward))))))"##;
    let expect = expect!["OK (1 14 1 2 93 10)"];

    assert_aa_edit_mode_parity(elisp_form, expect);
}
