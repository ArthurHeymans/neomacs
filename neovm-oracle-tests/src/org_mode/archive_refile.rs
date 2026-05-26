use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_archive_subtree_file_context_properties_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-archive)
  (let* ((file (make-temp-file "org-archive-source" nil ".org"))
         (archive (concat file "_archive")))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+CATEGORY: Work\n")
            (insert "* Parent :client:\n")
            (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
            (insert "** TODO Ship feature :urgent:\n")
            (insert "DEADLINE: <2026-06-01 Mon>\n")
            (insert "Body\n")
            (insert "** TODO Keep\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (let ((org-archive-location (concat archive "::* Archive"))
                  (org-archive-stamp-time nil)
                  (org-archive-subtree-add-inherited-tags t)
                  (org-archive-save-context-info '(file olpath category todo itags))
                  (org-archive-subtree-save-file-p nil))
              (goto-char (point-min))
              (search-forward "Ship feature")
              (beginning-of-line)
              (org-archive-subtree)
              (save-buffer)
              (let ((source (buffer-substring-no-properties
                             (point-min) (point-max)))
                    (archived (with-current-buffer
                                  (find-file-noselect archive)
                                (buffer-substring-no-properties
                                 (point-min) (point-max)))))
                (list source
                      (replace-regexp-in-string
                       (regexp-quote file)
                       "<source-file>"
                       archived))))))
      (dolist (buf (list (get-file-buffer file)
                         (get-file-buffer archive)))
        (when buf (kill-buffer buf)))
      (when (file-exists-p file) (delete-file file))
      (when (file-exists-p archive) (delete-file archive)))))"##,
    );
}

#[test]
fn org_refile_copy_with_logbook_and_bookmark_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let ((file (make-temp-file "org-refile-copy" nil ".org"
                              "* Inbox\n** TODO Task :inbox:\nBody\n* Projects\n** Target\n")))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (let ((org-refile-keep t)
                (org-log-refile 'time)
                (org-log-into-drawer t))
            (goto-char (point-min))
            (search-forward "Task")
            (beginning-of-line)
            (let ((target-pos (save-excursion
                                (goto-char (point-min))
                                (search-forward "Target")
                                (line-beginning-position))))
              (org-refile nil nil (list "Target" file nil target-pos)))
            (save-buffer)
            (list (plist-get org-bookmark-names-plist :last-refile)
                  (replace-regexp-in-string
                   "- Refiled on \\[.*\\]"
                   "- Refiled on [stamp]"
                   (buffer-substring-no-properties
                    (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_sparse_tree_match_visibility_and_map_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+TODO: TODO WAIT | DONE\n")
    (insert "* TODO Alpha :work:\nAlpha body\n")
    (insert "** WAIT Hidden child :work:\nChild body\n")
    (insert "* TODO Beta :home:\nBeta body\n")
    (insert "** TODO Matched child :work:\nChild body\n")
    (insert "* DONE Gamma :work:\nGamma body\n")
    (goto-char (point-min))
    (org-match-sparse-tree nil "+work+TODO=\"TODO\"")
    (list
     (org-map-entries
      (lambda ()
        (list (org-get-heading t t t t)
              (org-get-tags nil t)
              (not (null (org-invisible-p (line-end-position))))))
      nil
      nil)
     (let (states)
       (goto-char (point-min))
       (while (re-search-forward "^\\*+ " nil t)
         (push (list (org-get-heading t t t t)
                     (not (null (org-invisible-p (line-end-position)))))
               states))
       (nreverse states)))))"##,
    );
}
