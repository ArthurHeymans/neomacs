use expect_test::expect;

use super::assert_apheleia_parity;

#[test]
fn apheleia_minor_mode_adds_and_removes_only_its_local_after_save_hook() {
    let elisp_form = r##"(with-temp-buffer
         (let ((apheleia-mode-lighter
                " Format"))
           (add-hook
            'after-save-hook
            (lambda ()
              (setq apheleia-test-hook-events
                    (append
                     apheleia-test-hook-events
                     '(existing))))
            nil
            t)
           (list
            (progn
              (apheleia-mode 1)
              (list
               apheleia-mode
               after-save-hook
               (assq
                'apheleia-mode
                minor-mode-alist)))
            (progn
              (apheleia-mode -1)
              (list
               apheleia-mode
               after-save-hook
               (local-variable-p
                'after-save-hook))))))"##;
    let expect = expect![
        "OK ((t (apheleia-format-after-save . #1=(#[nil ((setq apheleia-test-hook-events (append apheleia-test-hook-events '(existing)))) (t)] t)) (apheleia-mode apheleia-mode-lighter)) (nil #1# t))"
    ];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_mode_maybe_obeys_buffer_inhibition_and_each_inhibit_function() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (setq-local
              apheleia-inhibit
              (car case))
             (let ((apheleia-inhibit-functions
                    (cadr case)))
               (apheleia-mode-maybe)
               (list
                case
                apheleia-mode
                (memq
                 #'apheleia-format-after-save
                 after-save-hook)))))
         '((nil nil)
           (t nil)
           (nil ((lambda () nil)))
           (nil ((lambda () 'project-policy)))
           (t ((lambda () nil)))))"##;
    let expect = expect![
        "OK (((nil nil) t (apheleia-format-after-save t)) ((t nil) nil nil) ((nil ((lambda nil nil))) t (apheleia-format-after-save t)) ((nil ((lambda nil 'project-policy))) nil nil) ((t ((lambda nil nil))) nil nil))"
    ];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_global_mode_applies_to_existing_and_new_eligible_buffers_only() {
    let elisp_form = r##"(let ((normal
                (generate-new-buffer
                 "apheleia-global-normal"))
               (inhibited
                (generate-new-buffer
                 "apheleia-global-inhibited"))
               created)
         (unwind-protect
             (progn
               (with-current-buffer inhibited
                 (setq-local
                  apheleia-inhibit
                  t))
               (apheleia-global-mode 1)
               (setq created
                     (generate-new-buffer
                      "apheleia-global-created"))
               (with-current-buffer created
                 (fundamental-mode))
               (list
                apheleia-global-mode
                (mapcar
                 (lambda (buffer)
                   (with-current-buffer buffer
                     (list
                      (buffer-name)
                      apheleia-mode
                      (and
                       (memq
                        #'apheleia-format-after-save
                        after-save-hook)
                       t))))
                 (list normal inhibited created))
                (progn
                  (apheleia-global-mode -1)
                  (mapcar
                   (lambda (buffer)
                     (buffer-local-value
                      'apheleia-mode
                      buffer))
                   (list normal inhibited created)))))
           (apheleia-global-mode -1)
           (mapc
            (lambda (buffer)
              (when
                  (buffer-live-p buffer)
                (kill-buffer buffer)))
            (list normal inhibited created))))"##;
    let expect = expect![[
        r#"OK (t (("apheleia-global-normal" t t) ("apheleia-global-inhibited" nil nil) ("apheleia-global-created" t t)) (nil nil nil))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_disallowed_policy_reports_skip_hooks_and_remote_cancel_precisely() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (let ((apheleia-skip-functions
                  nil))
             (apheleia--disallowed-p)))
         (with-temp-buffer
           (let ((apheleia-skip-functions
                  '((lambda () nil)
                    (lambda () 'generated-file))))
             (apheleia--disallowed-p)))
         (with-temp-buffer
           (setq-local
            buffer-file-name
            "/ssh:user@example.test:/src/main.c")
           (let ((apheleia-remote-algorithm
                  'cancel)
                 (apheleia-skip-functions
                  '((lambda () 'skip))))
             (apheleia--disallowed-p)))
         (with-temp-buffer
           (setq-local
            buffer-file-name
            "/ssh:user@example.test:/src/main.c")
           (let ((apheleia-remote-algorithm
                  'local)
                 (apheleia-skip-functions
                  nil))
             (apheleia--disallowed-p))))"##;
    let expect = expect![[
        r#"OK (nil "Apheleia skipped running formatter due to `apheleia-skip-functions'" "Apheleia refused to run formatter due to `apheleia-remote-algorithm'" nil)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_error_boundary_returns_success_or_routes_full_error_objects_to_handler() {
    let elisp_form = r##"(let (seen)
         (list
          (apheleia--with-on-error
              (lambda (error)
                (setq seen
                      (append seen
                              (list error)))
                :handled)
            (+ 20 22))
          (apheleia--with-on-error
              (lambda (error)
                (setq seen
                      (append seen
                              (list error)))
                :handled)
            (error
             "formatter %s failed"
             "demo"))
          (condition-case error
              (apheleia--with-on-error
                  nil
                (signal
                 'wrong-type-argument
                 '(stringp 17)))
            (error error))
          seen))"##;
    let expect = expect![[
        r#"OK (42 :handled (wrong-type-argument stringp 17) ((error "formatter demo failed")))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_buffer_hash_is_stable_for_identical_text_and_changes_after_real_edits() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha\nbeta\n")
         (let ((first
                (apheleia--buffer-hash)))
           (goto-char
            (point-min))
           (forward-line 1)
           (insert
            "new-")
           (let ((second
                  (apheleia--buffer-hash)))
             (delete-region
              (line-beginning-position)
              (+ (line-beginning-position) 4))
             (list
              first
              second
              (apheleia--buffer-hash)
              (equal
               first
               second)
              (equal
               first
               (apheleia--buffer-hash))))))"##;
    let expect = expect![[
        r#"OK ("9269a71477ce057095d7e6bb5238b4bd6e13c051" "f9b92f01574ad30ad78a662180d151b52a9c71ce" "9269a71477ce057095d7e6bb5238b4bd6e13c051" nil t)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_after_save_skips_disabled_narrowed_and_unconfigured_buffers() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'apheleia-format-buffer)
               (lambda (&rest args)
                 (setq calls
                       (append
                        calls
                        (list args))))))
           (list
            (with-temp-buffer
              (let ((apheleia-mode nil))
                (apheleia-format-after-save)
                calls))
            (with-temp-buffer
              (insert "one\ntwo\n")
              (narrow-to-region 1 4)
              (let ((apheleia-mode t)
                    (apheleia-formatter 'upper))
                (apheleia-format-after-save)
                calls))
            (with-temp-buffer
              (let ((apheleia-mode t)
                    (apheleia-formatter nil)
                    (apheleia-mode-alist nil))
                (apheleia-format-after-save)
                calls))
            (with-temp-buffer
              (let ((apheleia-mode t)
                    (apheleia-formatter
                     '(first second)))
                (apheleia-format-after-save)
                calls))
            calls)))"##;
    let expect = expect![[
        r#"OK (nil nil nil #1=(((first second) #[nil ((condition-case err (progn (if buffer-file-name (progn (let ((apheleia-format-after-save-in-progress t)) (apheleia--save-buffer-silently)))) (run-hooks 'apheleia-post-format-hook)) ((debug error) (message "Apheleia: %s" err) nil))) (t)])) #1#)"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}
