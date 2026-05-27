use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_mobile_files_index_checksums_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-index" t))
         (org-directory (expand-file-name "org" root))
         (stage (expand-file-name "stage" root))
         (nested (expand-file-name "nested" org-directory))
         (main (expand-file-name "main.org" org-directory))
         (extra (expand-file-name "extra.org" nested))
         (skip (expand-file-name "skip.org" nested))
         (outside (expand-file-name "outside.org" root))
         (org-mobile-directory stage)
         (org-mobile-index-file "index.org")
         (org-mobile-files
          (list 'org-agenda-files nested outside skip))
         (org-mobile-files-exclude-regexp "skip\\.org\\'")
         (org-agenda-files (list main))
         (org-tag-alist '(("work" . ?w) ("home" . ?h)))
         (org-todo-keywords '((sequence "TODO(t)" "WAIT(w)" "|" "DONE(d)")))
         (org-mobile-allpriorities "A B C")
         (org-mobile-checksum-files nil))
    (unwind-protect
        (progn
          (make-directory nested t)
          (make-directory stage t)
          (with-temp-file main
            (insert "#+FILETAGS: :work:\n* TODO Main :alpha:\n"))
          (with-temp-file extra
            (insert "* WAIT Extra :beta:\n"))
          (with-temp-file skip
            (insert "* TODO Skip\n"))
          (with-temp-file outside
            (insert "* DONE Outside :zeta:\n"))
          (let* ((alist (org-mobile-files-alist))
                 (org-mobile-files-alist alist))
            (org-mobile-create-index-file)
            (let ((index
                   (with-temp-buffer
                     (insert-file-contents
                      (expand-file-name "index.org" stage))
                     (buffer-string))))
              (list (mapcar (lambda (entry)
                              (cons (file-relative-name (car entry) root)
                                    (cdr entry)))
                            alist)
                    (sort org-mobile-checksum-files
                          (lambda (a b) (string< (car a) (car b))))
                    index))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_move_capture_apply_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-id)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-apply" t))
         (org-directory root)
         (stage (expand-file-name "stage" root))
         (target (expand-file-name "tasks.org" root))
         (inbox (expand-file-name "from-mobile.org" root))
         (capture (expand-file-name org-mobile-capture-file stage))
         (checksums (expand-file-name "checksums.dat" stage))
         (org-mobile-directory stage)
         (org-mobile-inbox-for-pull inbox)
         (org-id-locations-file (expand-file-name "ids.el" root))
         (org-id-track-globally t)
         (org-mobile-force-mobile-change nil)
         (org-log-done nil))
    (unwind-protect
        (progn
          (make-directory stage t)
          (with-temp-file target
            (insert "#+TITLE: Tasks\n")
            (insert "* TODO Task :old:\n")
            (insert ":PROPERTIES:\n:ID: mobile-task\n:END:\n")
            (insert "Old body line\n"))
          (with-temp-file inbox (insert "* Existing inbox\n"))
          (with-temp-file capture
            (insert "* F(edit:todo) [[id:mobile-task][Task]]\n")
            (insert "** Old value\nTODO\n** New value\nDONE\n")
            (insert "* F(edit:tags) [[id:mobile-task][Task]]\n")
            (insert "** Old value\nold\n** New value\nnew:mobile\n")
            (insert "* F(edit:body) [[id:mobile-task][Task]]\n")
            (insert "** Old value\nOld body line\n")
            (insert "** New value\nNew body line\nSecond line\n"))
          (with-temp-file checksums
            (insert "00000000000000000000000000000000  mobileorg.org\n"))
          (org-id-update-id-locations (list target) t)
          (let ((marker (org-mobile-move-capture)))
            (with-current-buffer (marker-buffer marker)
              (save-restriction
                (org-mode)
                (org-mobile-apply marker (point-max))))
            (let ((target-text
                   (with-current-buffer (find-file-noselect target)
                     (buffer-substring-no-properties
                      (point-min) (point-max))))
                  (inbox-text
                   (with-current-buffer (find-file-noselect inbox)
                     (buffer-substring-no-properties
                      (point-min) (point-max))))
                  (capture-text
                   (with-temp-buffer
                     (insert-file-contents capture)
                     (buffer-string)))
                  (checksum-text
                   (with-temp-buffer
                     (insert-file-contents checksums)
                     (buffer-string))))
              (list (markerp marker)
                    (replace-regexp-in-string
                     "^#\\+LAST_MOBILE_CHANGE:.*\n"
                     "#+LAST_MOBILE_CHANGE: <time>\n"
                     target-text)
                    inbox-text
                    capture-text
                    checksum-text))))
      (when (get-file-buffer target) (kill-buffer (get-file-buffer target)))
      (when (get-file-buffer inbox) (kill-buffer (get-file-buffer inbox)))
      (when (file-exists-p org-id-locations-file)
        (delete-file org-id-locations-file))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_mobile_olp_locate_edit_refile_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mobile)
  (let* ((root (make-temp-file "org-mobile-olp" t))
         (org-directory root)
         (file (expand-file-name "space file.org" root))
         (org-mobile-force-mobile-change '(heading priority body tags))
         (org-archive-location "::* Archived")
         (org-log-done nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Parent/One\n")
            (insert "** TODO [#B] Child :old:\n")
            (insert "Body one\n")
            (insert "* Inbox\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (goto-char (point-min))
            (search-forward "Child")
            (beginning-of-line)
            (let* ((olp (org-mobile-get-outline-path-link (point-marker)))
                   (found (org-mobile-locate-entry olp))
                   before after)
              (setq before (list olp
                                 (markerp found)
                                 (and (markerp found)
                                      (with-current-buffer (marker-buffer found)
                                        (org-get-heading t t t t)))))
              (org-mobile-edit "heading" "Different" "Renamed Child")
              (org-mobile-edit "priority" "A" "C")
              (org-mobile-edit "tags" "wrong" "new:mobile")
              (org-mobile-edit "body" "wrong"
                               "Replacement body\nwith whitespace\n")
              (setq after (buffer-substring-no-properties
                           (point-min) (point-max)))
              (goto-char (point-min))
              (search-forward "Renamed Child")
              (beginning-of-line)
              (org-mobile-edit "addheading" nil "Inserted sibling")
              (list before
                    after
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}
