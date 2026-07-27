use expect_test::expect;

use super::assert_aidev_mode_parity;

#[test]
fn aidev_mode_refactors_real_project_file_then_persists_only_on_save() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "aidev-mode-python-project"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (source
                 (expand-file-name
                  "src/report.py"
                  root))
                (original
                 "def total(values):\n    result = 0\n    for value in values:\n        result += value\n    return result\n")
                buffer
                calls)
         (unwind-protect
             (progn
               (make-directory
                (expand-file-name ".git" root)
                t)
               (make-directory
                (file-name-directory source)
                t)
               (write-region
                original nil source nil 'silent)
               (setq buffer
                     (find-file-noselect source))
               (with-current-buffer buffer
                 (let ((aidev-provider 'claude))
                   (cl-letf
                       (((symbol-function
                          'aidev---claude)
                         (lambda
                           (messages system model)
                           (push
                            (list
                             messages system model)
                            calls)
                           "```python\ndef total(values):\n    return sum(values)\n```")))
                     (aidev-refactor-buffer-with-chat
                      "Use the standard library while preserving behavior")
                     (let ((disk-before-save
                            (with-temp-buffer
                              (insert-file-contents
                               source)
                              (buffer-string)))
                           (buffer-before-save
                            (buffer-string))
                           (modified-before-save
                            (buffer-modified-p)))
                       (save-buffer)
                       (require 'project)
                       (let ((project
                              (project-current
                               nil
                               (file-name-directory
                                source))))
                         (list
                          major-mode
                          disk-before-save
                          buffer-before-save
                          modified-before-save
                          (buffer-modified-p)
                          (with-temp-buffer
                            (insert-file-contents
                             source)
                            (buffer-string))
                          (and project
                               (project-root
                                project))
                          (nreverse calls)))))))
           (when (buffer-live-p buffer)
             (with-current-buffer buffer
               (set-buffer-modified-p nil))
             (kill-buffer buffer))
           (when (file-directory-p root)
             (delete-directory root t)))))"##;
    let expect = expect!["OK nil"];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_region_refactor_updates_selected_project_function_without_touching_peer_file() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "aidev-mode-elisp-project"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (source
                 (expand-file-name
                  "lisp/report.el"
                  root))
                (peer
                 (expand-file-name
                  "lisp/format.el"
                  root))
                source-buffer)
         (unwind-protect
             (progn
               (make-directory
                (expand-file-name ".git" root)
                t)
               (make-directory
                (file-name-directory source)
                t)
               (write-region
                "(defun report-total (rows)\n  (let ((sum 0))\n    (dolist (row rows sum)\n      (setq sum (+ sum (car row))))))\n\n(defun report-title () \"Quarterly\")\n"
                nil source nil 'silent)
               (write-region
                "(defun report-format (value)\n  (number-to-string value))\n"
                nil peer nil 'silent)
               (setq source-buffer
                     (find-file-noselect source))
               (with-current-buffer
                   source-buffer
                 (let ((transient-mark-mode t)
                       (aidev-provider 'openai))
                   (goto-char (point-min))
                   (push-mark
                    (progn
                      (search-forward
                       "(defun report-title")
                      (beginning-of-line)
                      (point))
                    t t)
                   (cl-letf
                       (((symbol-function
                          'aidev---openai)
                         (lambda (&rest _)
                           "```elisp\n(defun report-total (rows)\n  (apply #'+ (mapcar #'car rows)))\n```")))
                     (aidev-refactor-region-with-chat
                      "Simplify only report-total")
                     (save-buffer)
                     (list
                      (buffer-string)
                      (with-temp-buffer
                        (insert-file-contents
                         source)
                        (buffer-string))
                      (with-temp-buffer
                        (insert-file-contents
                         peer)
                        (buffer-string))
                      (region-beginning)
                      (region-end))))))
           (when (buffer-live-p source-buffer)
             (with-current-buffer
                 source-buffer
               (set-buffer-modified-p nil))
             (kill-buffer source-buffer))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (#("(defun report-total (rows)\n  (let ((sum 0))\n    (dolist (row rows sum)\n      (setq sum (+ sum (car row))))))\n\n(defun report-title () \"Quarterly\")\n" 0 146 (fontified nil)) "(defun report-total (rows)\n  (let ((sum 0))\n    (dolist (row rows sum)\n      (setq sum (+ sum (car row))))))\n\n(defun report-title () \"Quarterly\")\n" "(defun report-format (value)\n  (number-to-string value))\n" 111 111)"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}

#[test]
fn aidev_mode_provider_failure_leaves_project_buffer_and_disk_transactionally_unchanged() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "aidev-mode-failed-project"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (source
                 (expand-file-name
                  "service.el"
                  root))
                (original
                 "(defun service-ready-p () t)\n")
                buffer)
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                original nil source nil 'silent)
               (setq buffer
                     (find-file-noselect source))
               (with-current-buffer buffer
                 (let ((aidev-provider 'claude))
                   (cl-letf
                       (((symbol-function
                          'aidev---claude)
                         (lambda (&rest _)
                           (error
                            "provider unavailable"))))
                     (list
                      (condition-case error-data
                          (aidev-refactor-buffer-with-chat
                           "Refactor safely")
                        (error error-data))
                      (buffer-string)
                      (buffer-modified-p)
                      (with-temp-buffer
                        (insert-file-contents
                         source)
                        (buffer-string)))))))
           (when (buffer-live-p buffer)
             (with-current-buffer buffer
               (set-buffer-modified-p nil))
             (kill-buffer buffer))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ((error "provider unavailable") #("(defun service-ready-p () t)\n" 0 29 (fontified nil)) nil "(defun service-ready-p () t)\n")"#
    ]];
    assert_aidev_mode_parity(elisp_form, expect);
}
