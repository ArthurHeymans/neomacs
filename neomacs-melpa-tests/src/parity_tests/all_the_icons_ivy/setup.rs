use expect_test::expect;

use super::assert_all_the_icons_ivy_parity;

#[test]
fn all_the_icons_ivy_setup_registers_every_default_command_in_declared_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'ivy-set-display-transformer)
               (lambda (command transformer)
                 (push (list command transformer)
                       calls))))
           (all-the-icons-ivy-setup)
           (nreverse calls)))"##;
    let expect = expect![
        "OK ((ivy-switch-buffer all-the-icons-ivy-buffer-transformer) (ivy-switch-buffer-other-window all-the-icons-ivy-buffer-transformer) (counsel-projectile-switch-to-buffer all-the-icons-ivy-buffer-transformer) (counsel-find-file all-the-icons-ivy-file-transformer) (counsel-file-jump all-the-icons-ivy-file-transformer) (counsel-recentf all-the-icons-ivy-file-transformer) (counsel-projectile all-the-icons-ivy-file-transformer) (counsel-projectile-find-file all-the-icons-ivy-file-transformer) (counsel-projectile-find-dir all-the-icons-ivy-file-transformer) (counsel-git all-the-icons-ivy-file-transformer))"
    ];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_setup_honors_custom_command_subsets_and_duplicates() {
    let elisp_form = r##"(let ((all-the-icons-ivy-buffer-commands
                '(switch-a switch-b switch-a))
               (all-the-icons-ivy-file-commands
                '(find-a find-b))
               calls)
         (cl-letf
             (((symbol-function 'ivy-set-display-transformer)
               (lambda (command transformer)
                 (push (list command transformer)
                       calls))))
           (all-the-icons-ivy-setup)
           (nreverse calls)))"##;
    let expect = expect![
        "OK ((switch-a all-the-icons-ivy-buffer-transformer) (switch-b all-the-icons-ivy-buffer-transformer) (switch-a all-the-icons-ivy-buffer-transformer) (find-a all-the-icons-ivy-file-transformer) (find-b all-the-icons-ivy-file-transformer))"
    ];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_setup_populates_real_ivy_transformer_registry() {
    let elisp_form = r##"(let ((all-the-icons-ivy-buffer-commands
                '(ivy-switch-buffer))
               (all-the-icons-ivy-file-commands
                '(counsel-find-file)))
         (setq ivy--display-transformers-alist nil)
         (all-the-icons-ivy-setup)
         (list
          ivy--display-transformers-alist
          (ivy-alist-setting
           ivy--display-transformers-alist
           'ivy-switch-buffer)
          (ivy-alist-setting
           ivy--display-transformers-alist
           'counsel-find-file)))"##;
    let expect = expect![
        "OK (((counsel-find-file . all-the-icons-ivy-file-transformer) (ivy-switch-buffer . all-the-icons-ivy-buffer-transformer)) all-the-icons-ivy-buffer-transformer all-the-icons-ivy-file-transformer)"
    ];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_registered_transformers_process_real_candidates() {
    let elisp_form = r##"(let ((all-the-icons-ivy-buffer-commands
                '(ivy-switch-buffer))
               (all-the-icons-ivy-file-commands
                '(counsel-find-file))
               (buffer
                (get-buffer-create "ivy-practical.el")))
         (unwind-protect
             (progn
               (setq ivy--display-transformers-alist nil)
               (with-current-buffer buffer
                 (emacs-lisp-mode))
               (all-the-icons-ivy-setup)
               (let* ((buffer-transformer
                       (ivy-alist-setting
                        ivy--display-transformers-alist
                        'ivy-switch-buffer))
                      (file-transformer
                       (ivy-alist-setting
                        ivy--display-transformers-alist
                        'counsel-find-file))
                      (buffer-result
                       (funcall buffer-transformer
                                (buffer-name buffer)))
                      (file-result
                       (funcall file-transformer
                                "src/main.rs")))
                 (list
                  (substring-no-properties
                   buffer-result)
                  (string-to-list
                   (get-text-property
                    0 'display buffer-result))
                  (substring-no-properties
                   file-result)
                  (string-to-list
                   (get-text-property
                    0 'display file-result)))))
           (kill-buffer buffer)))"##;
    let expect = expect![[r#"OK ("\11\11ivy-practical.el" (59686) "\11\11src/main.rs" (59692))"#]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_repeated_setup_replaces_existing_transformers() {
    let elisp_form = r##"(let ((all-the-icons-ivy-buffer-commands
                '(ivy-switch-buffer))
               (all-the-icons-ivy-file-commands
                '(counsel-find-file)))
         (setq ivy--display-transformers-alist
               '((ivy-switch-buffer . old-buffer)
                 (counsel-find-file . old-file)
                 (unrelated-command . untouched)))
         (all-the-icons-ivy-setup)
         (all-the-icons-ivy-setup)
         ivy--display-transformers-alist)"##;
    let expect = expect![
        "OK ((ivy-switch-buffer . all-the-icons-ivy-buffer-transformer) (counsel-find-file . all-the-icons-ivy-file-transformer) (unrelated-command . untouched))"
    ];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}
