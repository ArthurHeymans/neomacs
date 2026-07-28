use expect_test::expect;

use super::assert_auto_package_update_parity;

#[test]
fn auto_package_update_visible_write_creates_real_read_only_results_buffer_and_mode() {
    let elisp_form = r##"(let ((name
                                " *apu-visible-results*"))
                           (unwind-protect
                               (save-window-excursion
                                 (let ((result
                                        (apu--write-buffer
                                         "alpha up to date.\nbeta failed"
                                         name
                                         nil)))
                                   (with-current-buffer name
                                     (list
                                      result
                                      (buffer-name)
                                      (buffer-string)
                                      buffer-read-only
                                      auto-package-update-minor-mode
                                      (key-binding (kbd "q"))
                                      (buffer-modified-p)
                                      (eq
                                       (current-buffer)
                                       (get-buffer name))))))
                             (auto-package-update-test-kill-buffers
                              name)))"##;
    let expect = expect![[
        r#"OK (t " *apu-visible-results*" "alpha up to date.\nbeta failed" t t quit-window t t)"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_write_replaces_existing_read_only_contents_without_duplicate_state() {
    let elisp_form = r##"(let ((name
                                " *apu-overwrite-results*"))
                           (unwind-protect
                               (progn
                                 (with-current-buffer
                                     (get-buffer-create name)
                                   (insert "stale contents")
                                   (read-only-mode 1)
                                   (auto-package-update-minor-mode 1))
                                 (save-window-excursion
                                   (apu--write-buffer
                                    "fresh\nreport"
                                    name))
                                 (with-current-buffer name
                                   (list
                                    (buffer-string)
                                    buffer-read-only
                                    auto-package-update-minor-mode
                                    (key-binding (kbd "q"))
                                    (buffer-size)
                                    (local-variable-p
                                     'auto-package-update-minor-mode))))
                             (auto-package-update-test-kill-buffers
                              name)))"##;
    let expect = expect![[r#"OK ("fresh\nreport" t t quit-window 12 t)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_hidden_write_avoids_popup_and_buries_named_buffer() {
    let elisp_form = r##"(let
                             ((name
                               " *apu-hidden-results*")
                              events)
                           (unwind-protect
                               (cl-letf
                                   (((symbol-function
                                      'pop-to-buffer)
                                     (lambda (&rest arguments)
                                       (error
                                        "must not pop: %S"
                                        arguments)))
                                    ((symbol-function
                                      'bury-buffer)
                                     (lambda (&optional buffer)
                                       (push
                                        (list
                                         :bury
                                         (buffer-name
                                          (or
                                           (and
                                            (bufferp buffer)
                                            buffer)
                                           (current-buffer))))
                                        events)
                                       :buried)))
                                 (let ((result
                                        (apu--write-buffer
                                         "quiet report"
                                         name
                                         t)))
                                   (with-current-buffer name
                                     (list
                                      result
                                      (nreverse events)
                                      (buffer-string)
                                      buffer-read-only
                                      auto-package-update-minor-mode))))
                             (auto-package-update-test-kill-buffers
                              name)))"##;
    let expect = expect![[r#"OK (t ((:bury " *apu-hidden-results*")) "quiet report" t t)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_results_and_preview_wrappers_route_exact_names_and_visibility() {
    let elisp_form = r##"(let
                             ((auto-package-update-buffer-name
                               "results-name")
                              (auto-package-preview-buffer-name
                               "preview-name")
                              (auto-package-update-hide-results
                               t)
                              calls)
                           (cl-letf
                               (((symbol-function
                                  'apu--write-buffer)
                                 (lambda
                                     (contents name
                                               &optional hide)
                                   (push
                                    (list
                                     contents
                                     name
                                     hide)
                                    calls)
                                   (list :written name))))
                             (list
                              (apu--write-results-buffer
                               "installed")
                              (apu--write-preview-buffer
                               "pending")
                              (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((:written "results-name") (:written "preview-name") (("installed" "results-name" t) ("pending" "preview-name" nil)))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_hide_preview_selects_and_kills_existing_preview_window_buffer() {
    let elisp_form = r##"(let
                             ((auto-package-preview-buffer-name
                               " *apu-hide-preview*")
                              events)
                           (get-buffer-create
                            auto-package-preview-buffer-name)
                           (cl-letf
                               (((symbol-function
                                  'kill-buffer-and-window)
                                 (lambda ()
                                   (push
                                    (list
                                     :kill
                                     (buffer-name)
                                     (eq
                                      (current-buffer)
                                      (get-buffer
                                       auto-package-preview-buffer-name)))
                                    events)
                                   (kill-buffer
                                    (current-buffer))
                                   :killed)))
                             (list
                              (apu--hide-preview)
                              (nreverse events)
                              (get-buffer
                               auto-package-preview-buffer-name))))"##;
    let expect = expect![[r#"OK (:killed ((:kill " *apu-hide-preview*" t)) nil)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_hide_preview_is_noop_when_preview_buffer_does_not_exist() {
    let elisp_form = r##"(let
                             ((auto-package-preview-buffer-name
                               " *apu-absent-preview*")
                              calls)
                           (auto-package-update-test-kill-buffers
                            auto-package-preview-buffer-name)
                           (cl-letf
                               (((symbol-function
                                  'kill-buffer-and-window)
                                 (lambda ()
                                   (setq calls
                                         (1+ (or calls 0))))))
                             (list
                              (apu--hide-preview)
                              calls
                              (get-buffer
                               auto-package-preview-buffer-name))))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_auto_package_update_parity(elisp_form, expect);
}
