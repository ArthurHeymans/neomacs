use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_title_editing_covers_promote_demote_toggle_adjust_and_wraparound() {
    let elisp_form = r##"(cl-labels
         ((transform
           (text command)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (goto-char
              (or (search-backward "!" nil t) (point-max)))
             (when (eq (char-after) ?!)
               (delete-char 1))
             (condition-case error
                 (progn
                   (eval command t)
                   (buffer-string))
               (error
                (list (car error) (cadr error) (buffer-string)))))))
       (mapcar
        (lambda (case)
          (transform (car case) (cadr case)))
        '(("= foo!" (adoc-promote-title 1))
          ("====== foo!" (adoc-promote-title 1))
          ("== foo ==!" (adoc-promote-title 2))
          ("foo!\n+++" (adoc-promote-title 1))
          ("= foo!" (adoc-demote-title 1))
          ("foo!\n===" (adoc-demote-title 1))
          ("= one!" (adoc-toggle-title-type))
          ("two!\n===" (adoc-toggle-title-type))
          ("= five!" (adoc-toggle-title-type t))
          ("lorem!\n========" (adoc-adjust-title-del)))))"##;
    let expect = expect![[
        r#"OK ("== foo" "= foo" "==== foo ====" "foo\n===" "====== foo" "foo\n+++" "one\n===" "= two" "= five =" "lorem\n=====")"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_list_level_and_insertion_commands_cover_marker_families_and_errors() {
    let elisp_form = r##"(cl-labels
         ((transform
           (text command)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (goto-char
              (or (search-backward "!" nil t) (point-min)))
             (when (eq (char-after) ?!)
               (delete-char 1))
             (condition-case error
                 (progn
                   (eval command t)
                   (buffer-string))
               (error
                (list (car error) (cadr error) (buffer-string)))))))
       (mapcar
        (lambda (case)
          (transform (car case) (cadr case)))
        '(("* foo!" (adoc-promote 1))
          ("** foo!" (adoc-demote 1))
          ("- foo!" (adoc-promote 1))
          ("- foo!" (adoc-demote 1))
          ("***** foo!" (adoc-promote 1))
          ("  ** foo!" (adoc-promote 1))
          (". foo!" (adoc-promote 1))
          (".. foo!" (adoc-demote 1))
          ("1. foo!" (adoc-promote 1))
          ("* foo!" (adoc-insert-list-item))
          ("9. foo!" (adoc-insert-list-item))
          ("a. foo!" (adoc-insert-list-item))
          ("i) foo!" (adoc-insert-list-item))
          ("z. foo!" (adoc-insert-list-item))
          ("plain!" (adoc-insert-list-item)))))"##;
    let expect = expect![[
        r#"OK ("** foo" "* foo" "* foo" "- foo" "***** foo" "  *** foo" ".. foo" ". foo" (user-error "Cannot change the nesting level of a explicit-numbered list item" "1. foo") "* foo\n* " "9. foo\n10. " "a. foo\nb. " "i) foo\ni) " "z. foo\nz. " (user-error "Not on a list item" "plain"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_list_motion_carries_nested_items_and_rejects_non_siblings() {
    let elisp_form = r##"(cl-labels
         ((transform
           (text command)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (goto-char (point-min))
             (search-forward "!" nil t)
             (delete-char -1)
             (condition-case error
                 (progn
                   (eval command t)
                   (buffer-string))
               (error
                (list (car error) (cadr error) (buffer-string)))))))
       (mapcar
        (lambda (case)
          (transform (car case) (cadr case)))
        '(("* a!\n* b\n" (adoc-move-list-item-down))
          ("* a!\n* b" (adoc-move-list-item-down))
          ("* one!\n** sub\n* two\n" (adoc-move-list-item-down))
          ("* a\n* b!\n" (adoc-move-list-item-up))
          ("* one\n** sub\n* two!\n" (adoc-move-list-item-up))
          ("1. alpha!\n- bullet\n2. beta\n" (adoc-move-list-item-down))
          ("* only!\n" (adoc-move-list-item-down))
          ("** a!\n* b\n" (adoc-move-list-item-down)))))"##;
    let expect = expect![[
        r#"OK ("* b\n* a\n" "* b\n* a\n" "* two\n* one\n** sub\n" "* b\n* a\n" "* two\n* one\n** sub\n" (user-error "No next item at this level" "1. alpha\n- bullet\n2. beta\n") (user-error "No next item at this level" "* only\n") (user-error "No next item at this level" "** a\n* b\n"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_list_renumbering_covers_arabic_alpha_limits_and_unsupported_markers() {
    let elisp_form = r##"(cl-labels
         ((transform
           (text)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (goto-char
              (or (search-backward "!" nil t) (point-min)))
             (when (eq (char-after) ?!)
               (delete-char 1))
             (condition-case error
                 (progn
                   (adoc-renumber-list)
                   (buffer-string))
               (error
                (list (car error) (cadr error) (buffer-string)))))))
       (list
        (mapcar
         #'transform
         '("1. a!\n1. b\n1. c\n"
           "1. a\n5. b!\n9. c\n"
           "3. a!\n1. b\n"
           "a. x!\na. y\n"
           "1. a!\n9. b"
           "i) a!\nv) b\n"
           "* a!\n* b\n"))
        (with-temp-buffer
          (adoc-mode)
          (dotimes (_ 27) (insert "a. item\n"))
          (goto-char (point-min))
          (condition-case error
              (progn (adoc-renumber-list) (buffer-string))
            (error (list (car error) (cadr error)))))))"##;
    let expect = expect![[
        r#"OK (("1. a\n2. b\n3. c\n" "1. a\n2. b\n3. c\n" "3. a\n4. b\n" "a. x\nb. y\n" "1. a\n2. b" (user-error "Can only renumber arabic or alphabetic lists" "i) a\nv) b\n") (user-error "Not on an explicitly-numbered list item" "* a\n* b\n")) (user-error "Alphabetic lists cannot have more than 26 items"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_markup_commands_cover_word_region_toggle_empty_and_link_forms() {
    let elisp_form = r##"(cl-labels
         ((word-transform
           (text command)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (goto-char (point-max))
             (funcall command)
             (list (buffer-string) (point) (mark t))))
          (region-transform
           (text command)
           (with-temp-buffer
             (adoc-mode)
             (insert text)
             (set-mark (point-min))
             (activate-mark)
             (funcall command)
             (list (buffer-string) (point) (mark t)))))
       (list
        (mapcar
         (lambda (command)
           (word-transform "foo" command))
         '(adoc-insert-bold adoc-insert-italic adoc-insert-monospace
           adoc-insert-highlight adoc-insert-superscript
           adoc-insert-subscript))
        (mapcar
         (lambda (command)
           (region-transform "two words" command))
         '(adoc-insert-bold adoc-insert-italic adoc-insert-monospace))
        (region-transform "*wrapped*" #'adoc-insert-bold)
        (with-temp-buffer
          (adoc-mode)
          (adoc-insert-bold)
          (list (buffer-string) (point)))
        (mapcar
         (lambda (args)
           (with-temp-buffer
             (adoc-mode)
             (apply #'adoc-insert-link args)
             (buffer-string)))
         '(("https://example.test")
           ("https://example.test" "Example")
           ("mailto:user@example.test" "Mail")))))"##;
    let expect = expect![[
        r##"OK ((("*foo*" 6 nil) ("_foo_" 6 nil) ("`foo`" 6 nil) ("#foo#" 6 nil) ("^foo^" 6 nil) ("~foo~" 6 nil)) (("*two words*" 12 1) ("_two words_" 12 1) ("`two words`" 12 1)) ("wrapped" 8 1) ("**" 2) ("https://example.test" "https://example.test[Example]" "mailto:user@example.test[Mail]"))"##
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_insert_indented_respects_tab_width_level_and_tabs_policy() {
    let elisp_form = r##"(cl-labels
         ((insert-at-level
           (width tabs level)
           (with-temp-buffer
             (let ((tab-width width)
                   (indent-tabs-mode tabs))
               (adoc-mode)
               (adoc-insert-indented "x" level)
               (buffer-string)))))
       (list
        (insert-at-level 2 nil 1)
        (insert-at-level 2 nil 2)
        (insert-at-level 3 t 1)
        (insert-at-level 3 t 2)))"##;
    let expect = expect![[r#"OK (" x" "   x" "  x" "\11  x")"#]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_title_face_scaling_updates_all_levels_and_can_be_disabled() {
    let elisp_form = r##"(let ((original
                (mapcar
                 (lambda (face)
                   (face-attribute face :height nil t))
                 adoc--title-faces)))
         (unwind-protect
             (let ((adoc-title-scaling t)
                   (adoc-title-scaling-values
                    '(2.5 2.0 1.6 1.3 1.1 1.0)))
               (adoc-update-title-faces)
               (let ((scaled
                      (mapcar
                       (lambda (face)
                         (face-attribute face :height nil t))
                       adoc--title-faces)))
                 (setq adoc-title-scaling nil)
                 (adoc-update-title-faces)
                 (list
                  scaled
                  (mapcar
                   (lambda (face)
                     (face-attribute face :height nil t))
                   adoc--title-faces))))
           (cl-mapc
            (lambda (face height)
              (set-face-attribute face nil :height height))
            adoc--title-faces original)))"##;
    let expect = expect![[r#"OK ((2.5 2.0 1.6 1.3 1.1 1.0) (1.0 1.0 1.0 1.0 1.0 1.0))"#]];
    assert_adoc_mode_parity(elisp_form, expect);
}
