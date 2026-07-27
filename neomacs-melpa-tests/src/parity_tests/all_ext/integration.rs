use expect_test::expect;

use super::assert_all_ext_parity;

#[test]
fn all_ext_anything_command_schedules_real_conversion_with_live_buffers() {
    let elisp_form = r##"(let ((candidates
                           (generate-new-buffer
                            " *all-ext-anything*"))
                          (source
                           (generate-new-buffer
                            " *all-ext-source*"))
                          event)
                      (unwind-protect
                          (cl-progv
                              '(anything-buffer
                                anything-current-buffer)
                              (list candidates source)
                            (cl-letf
                                (((symbol-function
                                   'anything-run-after-quit)
                                  (lambda (&rest arguments)
                                    (setq event arguments)
                                    :scheduled)))
                              (list
                               (all-from-anything-occur)
                               (car event)
                               (cadr event)
                               (eq (nth 2 event) candidates)
                               (eq
                                (nth 3 event)
                                source))))
                        (kill-buffer candidates)
                        (kill-buffer source)))"##;
    let expect =
        expect![[r#"OK (:scheduled all-from-anything-occur-internal "anything-occur" t t)"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_helm_command_schedules_real_conversion_with_live_buffers() {
    let elisp_form = r##"(let ((candidates
                           (generate-new-buffer
                            " *all-ext-helm*"))
                          (source
                           (generate-new-buffer
                            " *all-ext-source*"))
                          event)
                      (unwind-protect
                          (cl-progv
                              '(helm-buffer
                                helm-current-buffer)
                              (list candidates source)
                            (cl-letf
                                (((symbol-function
                                   'helm-run-after-exit)
                                  (lambda (&rest arguments)
                                    (setq event arguments)
                                    :scheduled)))
                              (list
                               (all-from-helm-occur)
                               (car event)
                               (cadr event)
                               (eq (nth 2 event) candidates)
                               (eq
                                (nth 3 event)
                                source))))
                        (kill-buffer candidates)
                        (kill-buffer source)))"##;
    let expect = expect![[r#"OK (:scheduled all-from-anything-occur-internal "helm-occur" t t)"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_deferred_anything_and_helm_loads_install_expected_keys_and_source_shape() {
    let elisp_form = r##"(let* ((sandbox
                           (file-name-as-directory
                            (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                          (anything-file
                           (expand-file-name
                            "anything-config.el" sandbox))
                          (helm-file
                           (expand-file-name "helm.el" sandbox))
                          (regexp-file
                           (expand-file-name
                            "helm-regexp.el" sandbox))
                          (anything-map
                           (make-sparse-keymap))
                          (helm-map
                           (make-sparse-keymap))
                          (helm-source-occur
                           '((name . "Occur")
                             (nomark)
                             (candidates . identity))))
                      (with-temp-file anything-file
                        (insert
                         "(provide 'anything-config)\n"))
                      (with-temp-file helm-file
                        (insert "(provide 'helm)\n"))
                      (with-temp-file regexp-file
                        (insert "(provide 'helm-regexp)\n"))
                      (cl-progv
                          '(anything-map helm-map
                            helm-source-occur)
                          (list anything-map helm-map
                                helm-source-occur)
                        (load anything-file nil t)
                        (load helm-file nil t)
                        (load regexp-file nil t)
                        (list
                         (lookup-key
                          anything-map (kbd "C-c C-a"))
                         (lookup-key helm-map (kbd "C-c C-a"))
                         (symbol-value
                          'helm-source-occur))))"##;
    let expect = expect![[
        r#"OK (all-from-anything-occur all-from-helm-occur ((name . "Occur") (candidates . identity)))"#
    ]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_conversion_display_policy_schedules_pop_to_all_buffer_only_when_enabled() {
    let elisp_form = r##"(let ((source
                           (generate-new-buffer
                            "all-ext-display-source"))
                          (candidates
                           (generate-new-buffer
                            " *all-ext-display-candidates*"))
                          events)
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert "alpha\n"))
                            (with-current-buffer candidates
                              (insert
                               "Occur\n"
                               "all-ext-display-source:1:alpha\n"))
                            (cl-letf
                                (((symbol-function
                                   'kill-All-buffer-maybe)
                                  (lambda (&rest _)
                                    (when (get-buffer "*All*")
                                      (kill-buffer "*All*"))))
                                 ((symbol-function 'run-with-timer)
                                  (lambda
                                      (seconds repeat function
                                       &rest arguments)
                                    (push
                                     (list
                                      seconds repeat function
                                      (buffer-name
                                       (car arguments)))
                                     events)
                                    :timer)))
                              (let
                                  ((all-from-occur-select-window-flag
                                    t))
                                (all-from-anything-occur-internal
                                 "helm-occur"
                                 candidates source))
                              (let
                                  ((all-from-occur-select-window-flag
                                    nil))
                                (all-from-anything-occur-internal
                                 "helm-occur"
                                 candidates source)))
                            (nreverse events))
                        (when (get-buffer "*All*")
                          (kill-buffer "*All*"))
                        (kill-buffer source)
                        (kill-buffer candidates)))"##;
    let expect = expect![[r#"OK ((0 nil pop-to-buffer "*All*"))"#]];
    assert_all_ext_parity(elisp_form, expect);
}
