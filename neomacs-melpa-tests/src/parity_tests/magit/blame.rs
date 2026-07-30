use expect_test::expect;

use super::assert_magit_parity;

#[test]
fn magit_blame_addition_populates_commit_details_for_a_visited_file() {
    let elisp_form = r##"(let* ((root (make-temp-file "magit-blame-" t))
                    (default-directory (file-name-as-directory root))
                    (file (expand-file-name "tracked.txt" root))
                    (processes-before (process-list))
                    buffer)
               (unwind-protect
                   (progn
                     (magit-git "init" ".")
                     (with-temp-file file
                       (insert "first\nsecond\n"))
                     (magit-git "add" "tracked.txt")
                     (magit-git "commit" "-m" "initial")
                     (setq buffer (find-file-noselect file))
                     (switch-to-buffer buffer)
                     (magit-blame-addition nil)
                     (let ((deadline (+ (float-time) 3.0)))
                       (while (< (float-time) deadline)
                         (accept-process-output nil 0.05)))
                     (let ((display-text
                            (mapconcat
                             (lambda (overlay)
                               (let ((before
                                      (overlay-get
                                       overlay 'before-string))
                                     (after
                                      (overlay-get
                                       overlay 'after-string)))
                                 (concat
                                  (if (stringp before) before "")
                                  (if (stringp after) after ""))))
                             (overlays-in
                              (point-min) (point-max))
                             "")))
                       (list
                        magit-blame-mode
                        (and
                         (string-match-p
                          "A U Thor" display-text)
                         t)
                        (and
                         (string-match-p
                          "initial" display-text)
                         t)
                        (buffer-string)
                        (not
                         (seq-some
                          #'process-live-p
                          (seq-remove
                           (lambda (process)
                             (memq process processes-before))
                           (process-list)))))))
                 (when (buffer-live-p buffer)
                   (with-current-buffer buffer
                     (when magit-blame-mode
                       (magit-blame-quit)))
                   (kill-buffer buffer))
                 (dolist (process (process-list))
                   (unless (memq process processes-before)
                     (delete-process process)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (t t t "first\nsecond\n" t)"#]];

    assert_magit_parity(elisp_form, expect);
}
