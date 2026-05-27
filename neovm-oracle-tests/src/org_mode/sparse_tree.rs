use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_occur_stacked_sparse_tree_visibility_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fold-show-context-detail '((occur-tree . lineage)))
          (org-highlight-sparse-tree-matches t)
          (org-remove-highlights-with-change nil)
          (org-occur-hook nil))
      (org-mode)
      (insert "* TODO Alpha :work:\nneedle alpha\n** WAIT Child\nchild needle\n")
      (insert "* DONE Beta :home:\nno match\n** TODO Grand\nneedle grand\n")
      (insert "* TODO Gamma :work:\nother text\n")
      (let ((first (org-occur "needle"))
            (second (org-occur "TODO" t
                               (lambda ()
                                 (save-excursion
                                   (org-back-to-heading t)
                                   (member "work" (org-get-tags)))))))
        (list
         first
         second
         (length org-occur-highlights)
         (mapcar (lambda (needle)
                   (let ((pos (save-excursion
                                (goto-char (point-min))
                                (search-forward needle)
                                (point))))
                     (list needle (not (null (org-invisible-p pos))))))
                 '("Alpha" "needle alpha" "Child" "child needle"
                   "Beta" "Grand" "needle grand" "Gamma" "other text"))
         (mapcar (lambda (ov)
                   (list (overlay-start ov)
                         (overlay-end ov)
                         (overlay-get ov 'org-type)))
                 org-occur-highlights)
         (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_tags_sparse_tree_property_archive_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-use-tag-inheritance t)
          (org-sparse-tree-open-archived-trees nil))
      (org-mode)
      (insert "* Project :work:\n")
      (insert "** TODO Active :urgent:\n:PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "** DONE Closed :urgent:\n:PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "* Archived :work:ARCHIVE:\n")
      (insert "** TODO Old :urgent:\n:PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "* Other :home:\n")
      (insert "** TODO Home :urgent:\n:PROPERTIES:\n:Owner: Ada\n:END:\n")
      (org-match-sparse-tree nil "+work+urgent+TODO=\"TODO\"+Owner=\"Ada\"")
      (list
       (mapcar (lambda (needle)
                 (let ((pos (save-excursion
                              (goto-char (point-min))
                              (search-forward needle)
                              (point))))
                   (list needle (not (null (org-invisible-p pos))))))
               '("Project" "Active" "Closed" "Archived" "Old" "Other" "Home"))
       (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_occur_highlight_removal_after_buffer_change_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-highlight-sparse-tree-matches t)
          (org-remove-highlights-with-change t)
          (org-occur-hook nil))
      (org-mode)
      (insert "* One\nneedle one\n* Two\nneedle two\n")
      (let ((count (org-occur "needle"))
            (before (mapcar #'overlay-buffer org-occur-highlights)))
        (goto-char (point-max))
        (insert "\nchanged\n")
        (let ((after (mapcar #'overlay-buffer org-occur-highlights)))
          (list count
                (length before)
                before
                (length org-occur-highlights)
                after
                org-occur-parameters
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"#,
    );
}

#[test]
fn org_sparse_todo_occur_navigation_highlight_lifecycle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-highlight-sparse-tree-matches t)
          (org-remove-highlights-with-change nil)
          (org-occur-hook nil)
          (org-sparse-tree-open-archived-trees nil)
          (org-use-tag-inheritance t))
      (org-mode)
      (insert "#+TODO: TODO NEXT WAIT | DONE CANCELED\n")
      (insert "* TODO Project :work:\nneedle root\n")
      (insert "** NEXT Alpha :urgent:\nalpha needle\n")
      (insert "*** WAIT Alpha child :blocked:\nchild needle\n")
      (insert "** DONE Finished :urgent:\nfinished needle\n")
      (insert "* TODO Archived :work:ARCHIVE:\narchived needle\n")
      (insert "** NEXT Old :urgent:\nold needle\n")
      (insert "* WAIT Home :home:\nhome needle\n")
      (let (states)
        (let ((snapshot
               (lambda (label)
                 (list label
                       (mapcar (lambda (needle)
                                 (save-excursion
                                   (goto-char (point-min))
                                   (search-forward needle)
                                   (list needle
                                         (line-number-at-pos)
                                         (not (null
                                               (org-invisible-p
                                                (point))))
                                         (org-element-type
                                          (org-element-at-point)))))
                               '("Project" "needle root" "Alpha"
                                 "alpha needle" "Alpha child"
                                 "child needle" "Finished"
                                 "finished needle" "Archived"
                                 "archived needle" "Old" "old needle"
                                 "Home" "home needle"))
                       (mapcar (lambda (ov)
                                 (list (overlay-start ov)
                                       (overlay-end ov)
                                       (overlay-get ov 'org-type)))
                               org-occur-highlights)
                       (buffer-substring-no-properties
                        (point-min) (point-max)))))))
          (push (funcall snapshot 'initial) states)
          (org-show-todo-tree nil)
          (push (funcall snapshot 'todo-tree) states)
          (let ((occur-count (org-occur "needle" t
                                        (lambda ()
                                          (save-excursion
                                            (org-back-to-heading t)
                                            (member "urgent"
                                                    (org-get-tags)))))))
            (push (funcall snapshot 'occur-urgent) states)
            (let (moves)
              (dotimes (i 4)
                (condition-case err
                    (progn
                      (org-occur-next-match 1)
                      (push (list i
                                  (line-number-at-pos)
                                  (buffer-substring-no-properties
                                   (line-beginning-position)
                                   (line-end-position))
                                  (org-element-type
                                   (org-element-at-point)))
                            moves))
                  (error (push (list i (cons (car err) (cdr err)))
                               moves))))
              (goto-char (point-min))
              (search-forward "Alpha child")
              (beginning-of-line)
              (org-todo "DONE")
              (insert "edited ")
              (push (funcall snapshot 'after-edit) states)
              (let ((highlight-count-before-clear
                     (length org-occur-highlights)))
                (org-remove-occur-highlights)
                (push (funcall snapshot 'after-clear) states)
                (list occur-count
                      (nreverse moves)
                      highlight-count-before-clear
                      (nreverse states)
                      (length org-occur-highlights)
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))"##,
    );
}
