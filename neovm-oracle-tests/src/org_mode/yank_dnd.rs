use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_yank_image_attach_and_directory_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-attach)
  (let* ((root (make-temp-file "org-yank-image" t))
         (org-file (expand-file-name "notes.org" root))
         (image-dir (expand-file-name "images" root))
         (org-attach-id-dir (expand-file-name "data" root))
         (org-attach-store-link-p nil)
         (org-yank-image-file-name-function
          (lambda () "fixed-image")))
    (unwind-protect
        (progn
          (with-temp-file org-file
            (insert "* Task\n:PROPERTIES:\n:ID: yank-image-id\n:END:\n"))
          (with-current-buffer (find-file-noselect org-file)
            (org-mode)
            (goto-char (point-max))
            (let ((org-yank-image-save-method 'attach))
              (org--image-yank-media-handler "image/png" "PNGDATA"))
            (let* ((attach-buffer
                    (buffer-substring-no-properties (point-min) (point-max)))
                   (attach-dir (org-attach-dir))
                   (attach-file (expand-file-name "fixed-image.png" attach-dir))
                   (attach-data (with-temp-buffer
                                  (insert-file-contents-literally attach-file)
                                  (buffer-string))))
              (goto-char (point-max))
              (insert "\n")
              (let ((org-yank-image-save-method image-dir))
                (org--image-yank-media-handler "image/jpeg" "JPEGDATA"))
              (let* ((dir-buffer
                      (replace-regexp-in-string
                       (regexp-quote root) "<root>"
                       (buffer-substring-no-properties
                        (point-min) (point-max))))
                     (dir-file
                      (car (directory-files image-dir 'absolute
                                            "\\`fixed-image\\.")))
                     (dir-data (with-temp-buffer
                                 (insert-file-contents-literally dir-file)
                                 (buffer-string))))
                (list (replace-regexp-in-string
                       (regexp-quote root) "<root>" attach-buffer)
                      (file-relative-name attach-dir root)
                      (file-exists-p attach-file)
                      attach-data
                      dir-buffer
                      (file-exists-p dir-file)
                      dir-data)))))
      (when (get-file-buffer org-file) (kill-buffer (get-file-buffer org-file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_copied_files_dnd_file_link_and_attach_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-attach)
  (require 'dnd)
  (let* ((root (make-temp-file "org-dnd" t))
         (org-file (expand-file-name "notes.org" root))
         (a (expand-file-name "a.txt" root))
         (b (expand-file-name "b.txt" root))
         (org-attach-id-dir (expand-file-name "data" root))
         (org-attach-store-link-p nil)
         (org-attach-method 'cp))
    (unwind-protect
        (progn
          (with-temp-file a (insert "A\n"))
          (with-temp-file b (insert "B\n"))
          (with-temp-file org-file
            (insert "* Task\n:PROPERTIES:\n:ID: dnd-id\n:END:\n"))
          (with-current-buffer (find-file-noselect org-file)
            (org-mode)
            (goto-char (point-max))
            (let ((org-yank-dnd-method 'file-link))
              (org--dnd-multi-local-file-handler
               (list (concat "file://" a) (concat "file://" b))
               'copy))
            (let ((after-links
                   (replace-regexp-in-string
                    (regexp-quote root) "<root>"
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))
              (goto-char (point-max))
              (insert "\n")
              (let ((org-yank-dnd-method 'attach))
                (org--dnd-local-file-handler (concat "file://" a) 'copy ""))
              (let* ((dir (org-attach-dir))
                     (files (sort (org-attach-file-list dir) #'string<))
                     (attached-data
                      (with-temp-buffer
                        (insert-file-contents-literally
                         (expand-file-name "a.txt" dir))
                        (buffer-string))))
                (list after-links
                      (replace-regexp-in-string
                       (regexp-quote root) "<root>"
                       (buffer-substring-no-properties
                        (point-min) (point-max)))
                      (file-relative-name dir root)
                      files
                      attached-data)))))
      (when (get-file-buffer org-file) (kill-buffer (get-file-buffer org-file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_xds_direct_save_attach_and_file_link_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-attach)
  (let* ((root (make-temp-file "org-xds" t))
         (org-file (expand-file-name "notes.org" root))
         (target (expand-file-name "linked.bin" root))
         (org-attach-id-dir (expand-file-name "data" root))
         (org-attach-store-link-p nil))
    (unwind-protect
        (progn
          (with-temp-file org-file
            (insert "* Task\n:PROPERTIES:\n:ID: xds-id\n:END:\n"))
          (with-current-buffer (find-file-noselect org-file)
            (org-mode)
            (goto-char (point-max))
            (let ((org-yank-dnd-method 'attach))
              (let ((attach-name (org--dnd-xds-function t "drop.txt")))
                (with-temp-file attach-name (insert "DROP\n"))
                (org--dnd-xds-function nil attach-name)
                (let ((after-attach
                       (buffer-substring-no-properties
                        (point-min) (point-max)))
                      (attach-exists (file-exists-p attach-name)))
                  (goto-char (point-max))
                  (insert "\n")
                  (let ((org-yank-dnd-method 'file-link))
                    (cl-letf (((symbol-function 'read-file-name)
                               (lambda (&rest _) target)))
                      (let ((link-name (org--dnd-xds-function t "linked.bin")))
                        (with-temp-file link-name (insert "LINK\n"))
                        (org--dnd-xds-function nil link-name)
                        (list (file-relative-name attach-name root)
                              attach-exists
                              (replace-regexp-in-string
                               (regexp-quote root) "<root>"
                               after-attach)
                              (file-relative-name link-name root)
                              (file-exists-p link-name)
                              (replace-regexp-in-string
                               (regexp-quote root) "<root>"
                               (buffer-substring-no-properties
                                (point-min) (point-max)))))))))))
      (when (get-file-buffer org-file) (kill-buffer (get-file-buffer org-file)))
      (delete-directory root t))))"##,
    );
}
