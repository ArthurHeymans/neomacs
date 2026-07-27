use expect_test::expect;

use super::assert_alectryon_parity;

#[test]
fn alectryon_unknown_mode_and_configuration_failures_have_precise_actionable_messages() {
    let elisp_form = r##"(mapcar
 (lambda (thunk)
   (condition-case err
       (funcall thunk)
     (error (list (car err) (error-message-string err)))))
 (list
  (lambda () (with-temp-buffer
          (setq-local alectryon-prog-mode 'haskell-mode)
          (alectryon--prog-plist)))
  (lambda () (with-temp-buffer
          (setq-local alectryon-text-mode 'org-mode)
          (alectryon--text-plist)))
  (lambda () (alectryon--prog-mode-p 'fundamental-mode))
  (lambda () (with-temp-buffer
          (fundamental-mode)
          (alectryon--config :tag)))
  (lambda () (with-temp-buffer
          (setq-local alectryon-prog-mode 'haskell-mode)
          (alectryon--config-code+markup)))))"##;
    let expect = expect![[
        r#"OK ((error "Unrecognized Alectryon programming mode: haskell-mode") (error "Unrecognized Alectryon markup mode: org-mode") (error "Unrecognized mode: fundamental-mode (expecting one of (rst-mode markdown-mode typst-ts-mode coq-mode lean4-mode dafny-mode))") (error "Unrecognized mode: fundamental-mode (expecting one of (rst-mode markdown-mode typst-ts-mode coq-mode lean4-mode dafny-mode))") (error "Unrecognized Alectryon programming mode: haskell-mode"))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_read_mode_rejects_uninstalled_choices_and_empty_supported_sets() {
    let elisp_form = r##"(let ((original-prog alectryon-prog-modes)
      (original-text alectryon-text-modes))
  (unwind-protect
      (list
       (cl-letf (((symbol-function 'completing-read)
                  (lambda (&rest _) "missing-mode")))
         (condition-case err
             (alectryon--read-mode t)
           (error (list (car err) (error-message-string err)))))
       (progn
         (setq alectryon-text-modes
               '((missing-one :tag "one") (missing-two :tag "two")))
         (condition-case err
             (alectryon--read-text-mode)
           (error (list (car err) (error-message-string err))))))
    (setq alectryon-prog-modes original-prog
          alectryon-text-modes original-text)))"##;
    let expect = expect![[
        r#"OK ((user-error "Not installed: missing-mode") (error "No supported text mode found"))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_atomic_rolls_back_failed_complex_edits_and_groups_successful_edits_for_undo() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (buffer-enable-undo)
   (insert "stable")
   (setq buffer-undo-list nil)
   (let ((failure
          (condition-case err
              (alectryon--atomic
                (goto-char (point-max))
                (insert "-partial")
                (delete-region 1 3)
                (error "conversion exploded"))
            (error (list (car err) (error-message-string err))))))
     (list failure (buffer-string) (point) buffer-undo-list)))
 (with-temp-buffer
   (buffer-enable-undo)
   (insert "base")
   (setq buffer-undo-list nil)
   (alectryon--atomic
     (goto-char (point-max))
     (insert "-one")
     (insert "-two")
     (upcase-region 1 5))
   (let ((after (buffer-string))
         (undo-records (copy-tree buffer-undo-list)))
     (undo)
     (list after (buffer-string) undo-records))))"##;
    let expect = expect![[
        r#"OK (((error "conversion exploded") "stable" 7 nil) ("BASE-one-two" "base" (nil (1 . 5) ("base" . 1) (5 . 13))))"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}

#[test]
fn alectryon_mode_recording_does_not_poison_buffers_in_unsupported_major_modes() {
    let elisp_form = r##"(with-temp-buffer
  (fundamental-mode)
  (setq-local alectryon--original-mode nil
              alectryon-prog-mode 'coq-mode
              alectryon-text-mode nil)
  (let ((failure
         (condition-case err
             (alectryon-mode 1)
           (error (list (car err) (error-message-string err))))))
    (list failure
          major-mode alectryon-mode
          alectryon--original-mode
          alectryon-prog-mode alectryon-text-mode
          (memq #'alectryon--save write-contents-functions))))"##;
    let expect = expect![[
        r#"OK ((error "Unrecognized mode: fundamental-mode (expecting one of (rst-mode markdown-mode typst-ts-mode coq-mode lean4-mode dafny-mode))") fundamental-mode t nil coq-mode nil nil)"#
    ]];
    assert_alectryon_parity(elisp_form, expect);
}
