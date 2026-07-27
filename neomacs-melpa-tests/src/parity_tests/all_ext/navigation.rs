use expect_test::expect;

use super::assert_all_ext_parity;

#[test]
fn all_ext_next_error_without_visible_all_buffer_reports_exact_user_error() {
    let elisp_form = r##"(progn
                      (when (get-buffer "*All*")
                        (kill-buffer "*All*"))
                      (condition-case error
                          (all-next-error 1 nil)
                        (error
                         (list
                          (car error)
                          (error-message-string error)))))"##;
    let expect = expect![[r#"OK (error "Cannot find *All* buffer window.")"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_next_error_moves_through_visible_rows_and_invokes_all_mode_navigation() {
    let elisp_form = r##"(let* ((source
                            (generate-new-buffer
                             "all-ext-navigation-source"))
                           (all-buffer-value
                            (generate-new-buffer "*All*"))
                           (window (selected-window))
                           calls)
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert
                               "alpha\nbeta\ngamma\n"))
                            (with-current-buffer all-buffer-value
                              (insert
                               "From practical navigation\n"
                               "--------\n"
                               "alpha\nbeta\ngamma\n")
                              (all-mode)
                              (setq all-buffer source)
                              (goto-char (point-min)))
                            (set-window-buffer
                             window all-buffer-value)
                            (cl-letf
                                (((symbol-function 'all-mode-goto)
                                  (lambda ()
                                    (push
                                     (list
                                      (line-number-at-pos)
                                      (thing-at-point
                                       'line t))
                                     calls)
                                    :visited)))
                              (all-next-error 1 nil)
                              (all-next-error 2 t))
                            (with-current-buffer all-buffer-value
                              (list
                               (line-number-at-pos)
                               (thing-at-point 'line t)
                               (nreverse calls))))
                        (set-window-buffer
                         window
                         (get-buffer-create " *all-ext-restored*"))
                        (kill-buffer source)
                        (kill-buffer all-buffer-value)))"##;
    let expect = expect![[r#"OK (5 "gamma\n" ((3 "alpha\n") (5 "gamma\n")))"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_next_error_uses_real_row_markers_to_visit_successive_source_lines() {
    let elisp_form = r##"(let* ((source
                            (generate-new-buffer
                             "all-ext-real-navigation-source"))
                           (results
                            (generate-new-buffer "*All*"))
                           (window (selected-window))
                           visits)
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert
                               "alpha record\n"
                               "beta record\n"
                               "gamma record\n"))
                            (with-current-buffer results
                              (insert
                               "From practical navigation\n"
                               "--------\n")
                              (all-mode)
                              (setq all-buffer source))
                            (with-current-buffer source
                              (let ((standard-output results)
                                    (all-initialization-p t))
                                (goto-char (point-min))
                                (all-from-anything-occur-insert
                                 (line-beginning-position 2)
                                 (line-beginning-position 3)
                                 2 "beta record" nil)
                                (all-from-anything-occur-insert
                                 (line-beginning-position 3)
                                 (point-max)
                                 3 "gamma record" nil)))
                            (set-window-buffer window results)
                            (set-window-point
                             window
                             (with-current-buffer results
                               (point-min)))
                            (all-next-error 1 nil)
                            (with-current-buffer source
                              (push
                               (list
                                (line-number-at-pos)
                                (thing-at-point 'line t)
                                (eq
                                 (window-buffer window)
                                 source))
                               visits))
                            (set-window-buffer window results)
                            (all-next-error 1 nil)
                            (with-current-buffer source
                              (push
                               (list
                                (line-number-at-pos)
                                (thing-at-point 'line t)
                                (eq
                                 (window-buffer window)
                                 source))
                               visits))
                            (nreverse visits))
                        (set-window-buffer
                         window
                         (get-buffer-create
                          " *all-ext-restored*"))
                        (kill-buffer source)
                        (kill-buffer results)))"##;
    let expect = expect![[r#"OK ((2 "beta record\n" nil) (3 "gamma record\n" nil))"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_next_error_advice_resets_navigation_function_each_time_all_mode_runs() {
    let elisp_form = r##"(with-temp-buffer
                      (setq-local
                       next-error-function 'previous-line)
                      (all-mode)
                      (let ((first next-error-function))
                        (setq-local
                         next-error-function 'next-line)
                        (all-mode)
                        (list
                         first
                         next-error-function
                         (eq major-mode 'all-mode))))"##;
    let expect = expect!["OK (all-next-error all-next-error t)"];
    assert_all_ext_parity(elisp_form, expect);
}
