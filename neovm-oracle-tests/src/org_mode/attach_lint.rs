use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_attach_copy_buffer_delete_sync_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-attach)
  (let* ((root (make-temp-file "org-attach-copy" t))
         (org-file (expand-file-name "notes.org" root))
         (source (expand-file-name "source.txt" root))
         (org-attach-id-dir (expand-file-name "data" root))
         (org-attach-store-link-p 'attached)
         (org-attach-auto-tag "ATTACH")
         (org-stored-links nil)
         (events nil)
         (org-attach-after-change-hook
          (list (lambda (dir)
                  (push (file-relative-name dir root) events)))))
    (unwind-protect
        (progn
          (with-temp-file source (insert "from source\n"))
          (with-temp-file org-file
            (insert "* Task\n:PROPERTIES:\n:ID: attach-fixed-id\n:END:\n"))
          (with-current-buffer (find-file-noselect org-file)
            (org-mode)
            (goto-char (point-min))
            (let ((payload (get-buffer-create "payload.txt")))
              (with-current-buffer payload
                (erase-buffer)
                (insert "from buffer\n"))
              (org-attach-attach source nil 'cp)
              (org-attach-buffer "payload.txt")
              (let* ((dir (org-attach-dir))
                     (files-after-add (sort (org-attach-file-list dir) #'string<))
                     (source-content
                      (with-temp-buffer
                        (insert-file-contents (expand-file-name "source.txt" dir))
                        (buffer-string)))
                     (payload-content
                      (with-temp-buffer
                        (insert-file-contents (expand-file-name "payload.txt" dir))
                        (buffer-string))))
                (org-attach-delete-one "source.txt")
                (let ((files-after-delete (sort (org-attach-file-list dir) #'string<)))
                  (delete-file (expand-file-name "payload.txt" dir))
                  (let ((org-attach-sync-delete-empty-dir nil))
                    (org-attach-sync))
                  (list (file-relative-name dir root)
                        files-after-add
                        files-after-delete
                        source-content
                        payload-content
                        org-stored-links
                        (sort events #'string<)
                        (org-get-tags nil t)
                        (buffer-substring-no-properties
                         (point-min) (point-max))))))))
      (when (get-buffer "payload.txt") (kill-buffer "payload.txt"))
      (when (get-file-buffer org-file) (kill-buffer (get-file-buffer org-file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_attach_dir_inheritance_expand_links_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-attach)
  (let* ((root (make-temp-file "org-attach-dir" t))
         (org-file (expand-file-name "notes.org" root))
         (attach-dir (expand-file-name "shared" root))
         (org-attach-use-inheritance t))
    (unwind-protect
        (progn
          (make-directory attach-dir)
          (with-temp-file (expand-file-name "doc.txt" attach-dir)
            (insert "attached document\n"))
          (with-temp-file org-file
            (insert "* Parent\n")
            (insert ":PROPERTIES:\n:DIR: " attach-dir "\n:END:\n")
            (insert "** Child\n")
            (insert "[[attachment:doc.txt][Doc]] and [[attachment:missing.txt]]\n"))
          (with-current-buffer (find-file-noselect org-file)
            (org-mode)
            (goto-char (point-min))
            (search-forward "** Child")
            (beginning-of-line)
            (let ((child-dir (org-attach-dir))
                  (expanded-doc (file-relative-name
                                 (org-attach-expand "doc.txt")
                                 root))
                  (complete-link
                   (cl-letf (((symbol-function 'read-file-name)
                              (lambda (&rest _)
                                (expand-file-name "doc.txt" attach-dir))))
                     (org-attach-complete-link))))
              (org-attach-expand-links nil)
              (list (file-relative-name child-dir root)
                    expanded-doc
                    complete-link
                    (replace-regexp-in-string
                     (regexp-quote root)
                     "<root>"
                     (buffer-substring-no-properties
                      (point-min) (point-max)))))))
      (when (get-file-buffer org-file) (kill-buffer (get-file-buffer org-file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_lint_multiple_checker_report_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-lint)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: dup\n")
    (insert "#+begin_src emacs-lisp :bad yes\n")
    (insert "(+ 1 2)\n")
    (insert "#+end_src\n")
    (insert "#+NAME: dup\n")
    (insert "#+begin_src\n")
    (insert "body\n")
    (insert "#+end_src\n")
    (insert "[[coderef:missing]] [[#missing-custom]] [[file:no-such-file.txt]]\n")
    (insert "[fn:lost]\n")
    (insert "* H\n:PROPERTIES:\n:EFFORT: invalid\n:ID: bad::id\n:END:\n")
    (let ((reports (org-lint
                    '(duplicate-name
                      missing-language-in-src-block
                      invalid-coderef-link
                      invalid-custom-id-link
                      link-to-local-file
                      undefined-footnote-reference
                      invalid-effort-property
                      invalid-id-property
                      wrong-header-argument))))
      (mapcar (lambda (entry)
                (let ((row (cadr entry)))
                  (list (aref row 0)
                        (aref row 1)
                        (aref row 2)
                        (org-lint-checker-name (aref row 3)))))
              reports))))"##,
    );
}
