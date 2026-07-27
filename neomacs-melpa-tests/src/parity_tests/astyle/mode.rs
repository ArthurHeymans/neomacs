use super::assert_astyle_parity;
use expect_test::{Expect, expect};

#[test]
fn enabling_disabling_and_toggling_mode_updates_only_local_before_save_hook() {
    let elisp_form = r##"
(with-temp-buffer
  (let ((global-before-save
         (default-value
          'before-save-hook)))
    (list
     (progn
       (astyle-on-save-mode 1)
       (list
        astyle-on-save-mode
        (local-variable-p
         'before-save-hook)
        before-save-hook))
     (progn
       (astyle-on-save-mode 1)
       (list
        astyle-on-save-mode
        (cl-count
         'astyle-buffer
         before-save-hook)))
     (progn
       (astyle-on-save-mode -1)
       (list
        astyle-on-save-mode
        before-save-hook))
     (progn
       (astyle-on-save-mode)
       (list
        astyle-on-save-mode
        before-save-hook))
     (progn
       (astyle-on-save-mode)
       (list
        astyle-on-save-mode
        before-save-hook))
     (equal
      global-before-save
      (default-value
       'before-save-hook)))))
"##;
    let expect: Expect =
        expect!["OK ((t t (astyle-buffer t)) (t 1) (nil nil) (t #1=(astyle-buffer t)) (t #1#) t)"];
    assert_astyle_parity(elisp_form, expect);
}

#[test]
fn lighter_customization_is_reflected_by_enabled_minor_mode_without_changing_hook_behavior() {
    let elisp_form = r##"
(with-temp-buffer
  (let ((astyle-on-save-mode-lighter
         " Format[C++]"))
    (astyle-on-save-mode 1)
    (list
     astyle-on-save-mode
     astyle-on-save-mode-lighter
     (assq
      'astyle-on-save-mode
      minor-mode-alist)
     (memq
      'astyle-buffer
      before-save-hook)
     (get
      'astyle-on-save-mode-lighter
      'custom-type)
     (get
      'astyle-on-save-mode-lighter
      'custom-group))))
"##;
    let expect: Expect = expect![[
        r#"OK (t " Format[C++]" (astyle-on-save-mode astyle-on-save-mode-lighter) (astyle-buffer t) string nil)"#
    ]];
    assert_astyle_parity(elisp_form, expect);
}

#[test]
fn mode_instances_and_hooks_remain_independent_across_two_c_buffers() {
    let elisp_form = r##"
(let ((first
       (generate-new-buffer
        " *astyle-mode-first*"))
      (second
       (generate-new-buffer
        " *astyle-mode-second*")))
  (unwind-protect
      (progn
        (with-current-buffer first
          (astyle-on-save-mode 1))
        (with-current-buffer second
          (astyle-on-save-mode 1)
          (astyle-on-save-mode -1))
        (list
         (with-current-buffer first
           (list
            astyle-on-save-mode
            before-save-hook))
         (with-current-buffer second
           (list
            astyle-on-save-mode
            before-save-hook))
         (default-value
          'astyle-on-save-mode)))
    (kill-buffer first)
    (kill-buffer second)))
"##;
    let expect: Expect = expect!["OK ((t (astyle-buffer t)) (nil nil) nil)"];
    assert_astyle_parity(elisp_form, expect);
}

#[test]
fn enabled_mode_formats_buffer_before_save_and_persists_formatted_content() {
    let elisp_form = r##"
(let* ((installation
        (astyle-test-install-formatter))
       (argument-log
        (cadr installation))
       (source
        (astyle-test-path
         "on-save/source.c"))
       buffer)
  (make-directory
   (file-name-directory source)
   t)
  (with-temp-file source
    (insert
     "int main(){\nreturn 0;\n}\n"))
  (setq buffer
        (find-file-noselect
         source))
  (unwind-protect
      (with-current-buffer buffer
        (setq c-basic-offset 4
              astyle-style "google"
              astyle-custom-args
              '("--suffix=none"))
        (astyle-on-save-mode 1)
        (goto-char (point-max))
        (insert
         "/* saved */\n")
        (save-buffer)
        (list
         astyle-on-save-mode
         (substring-no-properties
          (buffer-string))
         (astyle-test-read-file
          source)
         (astyle-test-read-file
          argument-log)
         (buffer-modified-p)
         (memq
          'astyle-buffer
          before-save-hook)))
    (when
        (buffer-live-p buffer)
      (with-current-buffer buffer
        (set-buffer-modified-p nil))
      (kill-buffer buffer))
    (astyle-test-kill-error-buffer)))
"##;
    let expect: Expect = expect![[
        r#"OK (t "int main() {\n    return 0;\n}\n/* saved */\n" "int main() {\n    return 0;\n}\n/* saved */\n" "--style=google\n--indent=spaces=4\n--suffix=none\n" nil (astyle-buffer t))"#
    ]];
    assert_astyle_parity(elisp_form, expect);
}

#[test]
fn disabled_mode_saves_original_unformatted_content_and_does_not_run_program() {
    let elisp_form = r##"
(let* ((installation
        (astyle-test-install-formatter))
       (argument-log
        (cadr installation))
       (source
        (astyle-test-path
         "disabled-save/source.c"))
       buffer)
  (make-directory
   (file-name-directory source)
   t)
  (with-temp-file source
    (insert
     "int main(){\nreturn 0;\n}\n"))
  (setq buffer
        (find-file-noselect
         source))
  (unwind-protect
      (with-current-buffer buffer
        (setq c-basic-offset 4)
        (astyle-on-save-mode 1)
        (astyle-on-save-mode -1)
        (goto-char (point-max))
        (insert
         "/* raw */\n")
        (save-buffer)
        (list
         astyle-on-save-mode
         (buffer-string)
         (astyle-test-read-file
          source)
         (file-exists-p
          argument-log)
         (memq
          'astyle-buffer
          before-save-hook)))
    (when
        (buffer-live-p buffer)
      (with-current-buffer buffer
        (set-buffer-modified-p nil))
      (kill-buffer buffer))
    (astyle-test-kill-error-buffer)))
"##;
    let expect: Expect = expect![[
        r#"OK (nil #("int main(){\nreturn 0;\n}\n/* raw */\n" 0 24 (fontified nil) 24 34 (fontified nil)) "int main(){\nreturn 0;\n}\n/* raw */\n" nil nil)"#
    ]];
    assert_astyle_parity(elisp_form, expect);
}

#[test]
fn formatter_failure_during_save_keeps_raw_content_but_save_still_completes() {
    let elisp_form = r##"
(let* ((installation
        (astyle-test-install-formatter))
       (argument-log
        (cadr installation))
       (source
        (astyle-test-path
         "failed-save/source.c"))
       buffer)
  (setenv
   "ASTYLE_TEST_FAIL"
   "1")
  (make-directory
   (file-name-directory source)
   t)
  (with-temp-file source
    (insert
     "int main(){\nreturn 0;\n}\n"))
  (setq buffer
        (find-file-noselect
         source))
  (unwind-protect
      (with-current-buffer buffer
        (setq c-basic-offset 4)
        (astyle-on-save-mode 1)
        (goto-char (point-max))
        (insert
         "/* failure still saves */\n")
        (save-buffer)
        (list
         (buffer-string)
         (astyle-test-read-file
          source)
         (astyle-test-read-file
          argument-log)
         (buffer-modified-p)
         (current-message)
         (with-current-buffer
             (get-buffer
              "*astyle errors*")
           (substring-no-properties
            (buffer-string)))))
    (setenv
     "ASTYLE_TEST_FAIL"
     nil)
    (when
        (buffer-live-p buffer)
      (with-current-buffer buffer
        (set-buffer-modified-p nil))
      (kill-buffer buffer))
    (astyle-test-kill-error-buffer)))
"##;
    let expect: Expect = expect![[
        r#"OK (#("int main(){\nreturn 0;\n}\n/* failure still saves */\n" 0 24 (fontified nil) 24 50 (fontified nil)) "int main(){\nreturn 0;\n}\n/* failure still saves */\n" "--style=google\n--indent=spaces=4\n--pad-oper\n--pad-header\n--break-blocks\n--delete-empty-lines\n--align-pointer=type\n--align-reference=name\n" nil nil "fixture formatter failed\n")"#
    ]];
    assert_astyle_parity(elisp_form, expect);
}
