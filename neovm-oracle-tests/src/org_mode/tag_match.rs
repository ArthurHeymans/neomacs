use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_make_tags_matcher_scan_properties_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-use-tag-inheritance t))
      (org-mode)
      (insert "#+TODO: TODO WAIT | DONE\n")
      (insert "* TODO Parent :work:\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "** TODO Alpha :urgent:\n")
      (insert ":PROPERTIES:\n:Effort: 1.5\n:END:\n")
      (insert "** WAIT Beta :urgent:\n")
      (insert ":PROPERTIES:\n:Effort: 2.0\n:END:\n")
      (insert "* TODO Gamma :home:\n")
      (insert ":PROPERTIES:\n:Effort: 3.0\n:END:\n")
      (goto-char (point-min))
      (let* ((compiled (org-make-tags-matcher
                        "+work+urgent+TODO=\"TODO\"+Effort>=1"))
             (matcher (cdr compiled)))
        (list
         (car compiled)
         org--matcher-tags-todo-only
         (org-scan-tags
          (lambda ()
            (list (org-get-heading t t t t)
                  (org-get-tags nil t)
                  (org-entry-get nil "Owner" t)
                  (org-entry-get nil "Effort")))
          matcher
          nil))))))"##,
    );
}

#[test]
fn org_scan_tags_sparse_tree_visibility_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Alpha :work:\nBody A\n")
    (insert "** TODO Child A :work:\nBody child A\n")
    (insert "* TODO Beta :home:\nBody B\n")
    (insert "** TODO Child B :work:\nBody child B\n")
    (insert "* DONE Gamma :work:\nBody G\n")
    (goto-char (point-min))
    (let ((matcher (cdr (org-make-tags-matcher "+work+TODO=\"TODO\""))))
      (org-scan-tags 'sparse-tree matcher nil)
      (list
       (let (out)
         (goto-char (point-min))
         (while (re-search-forward "^\\*+ " nil t)
           (push (list (org-get-heading t t t t)
                       (not (null (org-invisible-p
                                   (line-end-position)))))
                 out))
         (nreverse out))
       (buffer-substring-no-properties
        (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_global_tags_completion_table_files_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((one (make-temp-file "org-tags-one" nil ".org"
                             "#+FILETAGS: :project:
* TODO Alpha :work:urgent:
* DONE Beta :done:
"))
        (two (make-temp-file "org-tags-two" nil ".org"
                             "* WAIT Gamma :home:
:PROPERTIES:
:CATEGORY: House
:END:
")))
    (unwind-protect
        (let* ((org-agenda-files (list one two))
               (table (org-global-tags-completion-table (list one two))))
          (sort
           (mapcar (lambda (entry)
                     (if (consp entry) (car entry) entry))
                   table)
           #'string<))
      (dolist (file (list one two))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
        (when (file-exists-p file) (delete-file file))))))"##,
    );
}
