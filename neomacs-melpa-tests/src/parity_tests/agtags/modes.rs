use expect_test::expect;

use super::assert_agtags_parity;

#[test]
fn agtags_grep_mode_initializes_complete_compilation_navigation_state() {
    let elisp_form = r##"(with-temp-buffer
         (agtags-grep-mode)
         (list
          major-mode
          mode-name
          (derived-mode-p 'grep-mode)
          grep-scroll-output
          grep-highlight-matches
          compilation-always-kill
          compilation-disable-input
          compilation-error-screen-columns
          compilation-scroll-output
          compilation-error-regexp-alist
          compilation-finish-functions
          (local-variable-p
           'compilation-error-regexp-alist)
          (eq
           (current-local-map)
           agtags--global-mode-map)))"##;
    let expect = expect![[
        r#"OK (agtags-grep-mode "Global Grep" grep-mode nil nil t t nil first-error (("^\\(.+?\\):\\([0-9]+\\):\\(?:$\\|[^0-9\n]\\|[0-9][^0-9\n]\\|[0-9][0-9].\\)" 1 2 (#[nil ((let* ((start (1+ (match-end 2))) (mbeg (text-property-any start (line-end-position) 'global-color t))) (and mbeg (- mbeg start)))) (agtags-mode t)]) nil 1)) agtags--global-mode-finished t t)"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_path_mode_initializes_file_navigation_face_and_compilation_state() {
    let elisp_form = r##"(with-temp-buffer
         (agtags-path-mode)
         (list
          major-mode
          mode-name
          (derived-mode-p
           'compilation-mode)
          compilation-error-face
          compilation-always-kill
          compilation-disable-input
          compilation-error-screen-columns
          compilation-scroll-output
          compilation-error-regexp-alist
          compilation-finish-functions
          (local-variable-p
           'compilation-error-face)
          (eq
           (current-local-map)
           agtags--global-mode-map)))"##;
    let expect = expect![[
        r#"OK (agtags-path-mode "Global Files" compilation-mode compilation-info t t nil first-error (("^\\(?:[^\"'\n]*/\\)?[^ )\11\n]+$" 0)) agtags--global-mode-finished t t)"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_modes_apply_real_status_font_lock_faces_to_global_process_summaries() {
    let elisp_form = r##"(mapcar
         (lambda (mode)
           (with-temp-buffer
             (funcall mode)
             (let ((inhibit-read-only t))
               (insert
                "Global exited abnormally with code 23\n"
                "Global found 17 definitions\n"))
             (font-lock-ensure)
             (goto-char (point-min))
             (search-forward "exited abnormally")
             (let ((failure-face
                    (get-text-property
                     (match-beginning 0)
                     'face))
                   (failure-font-lock-face
                    (get-text-property
                     (match-beginning 0)
                     'font-lock-face)))
               (search-forward "23")
               (let ((code-face
                      (get-text-property
                       (match-beginning 0)
                       'face))
                     (code-font-lock-face
                      (get-text-property
                       (match-beginning 0)
                       'font-lock-face)))
                 (search-forward "17")
                 (list
                  mode
                  failure-face
                  failure-font-lock-face
                  code-face
                  code-font-lock-face
                  (get-text-property
                   (match-beginning 0)
                   'face)
                  (get-text-property
                   (match-beginning 0)
                   'font-lock-face))))))
         '(agtags-grep-mode
           agtags-path-mode))"##;
    let expect = expect![
        "OK ((agtags-grep-mode nil nil nil nil nil nil) (agtags-path-mode compilation-error nil compilation-error nil compilation-info nil))"
    ];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_finished_callback_kills_only_the_opposite_result_buffer() {
    let elisp_form = r##"(let ((grep-buffer
                (get-buffer-create
                 "*agtags-grep*"))
               (path-buffer
                (get-buffer-create
                 "*agtags-path*"))
               events)
         (unwind-protect
             (cl-letf (((symbol-function
                         'delete-windows-on)
                        (lambda (buffer)
                          (push
                           (list
                            'delete-windows
                            (buffer-name buffer))
                           events))))
               (let ((first
                      (agtags--global-mode-finished
                       grep-buffer
                       "finished\n")))
                 (setq path-buffer
                       (get-buffer-create
                        "*agtags-path*"))
                 (let ((second
                        (agtags--global-mode-finished
                         path-buffer
                         "finished\n")))
                   (list
                    first
                    second
                    (buffer-live-p
                     grep-buffer)
                    (buffer-live-p
                     path-buffer)
                    (nreverse events)))))
           (when (buffer-live-p grep-buffer)
             (kill-buffer grep-buffer))
           (when (buffer-live-p path-buffer)
             (kill-buffer path-buffer))))"##;
    let expect = expect![[
        r#"OK (t t nil t ((delete-windows "*agtags-path*") (delete-windows "*agtags-grep*")))"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}

#[test]
fn agtags_compile_goto_advice_scopes_same_window_display_action_and_preserves_arguments() {
    let elisp_form = r##"(let (events)
         (list
          (agtags--compile-goto-error
           (lambda (&rest arguments)
             (push
              (list
               arguments
               display-buffer-overriding-action)
              events)
             'navigated)
           3 'reset)
          display-buffer-overriding-action
          (nreverse events)
          (advice-member-p
           #'agtags--compile-goto-error
           'compile-goto-error)))"##;
    let expect = expect![[
        r#"OK (navigated (nil) (((3 reset) ((display-buffer-reuse-window display-buffer-same-window) (inhibit-same-window)))) #[128 "������\3#��" [agtags--compile-goto-error #[(&optional event) ((if event (posn-set-point (event-end event))) (or (compilation-buffer-p (current-buffer)) (error "Not in a compilation buffer")) (compilation--ensure-parse (point)) (if (get-text-property (point) 'compilation-directory) (dired-other-window (car (get-text-property (point) 'compilation-directory))) (setq compilation-current-error (point)) (next-error-internal))) (cl-struct-compilation--message-tags t) nil "Visit the source for the error message at point.\nUse this command in a compilation log buffer." (list last-input-event)] :around nil apply] 5 advice])"#
    ]];
    assert_agtags_parity(elisp_form, expect);
}
