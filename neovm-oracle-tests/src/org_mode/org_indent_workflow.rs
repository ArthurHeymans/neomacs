use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_indent_fold_edit_level_refresh_no_merge_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (require 'org-indent)
  (require 'org-inlinetask)
  (with-temp-buffer
    (let ((org-indent-indentation-per-level 3)
          (org-adapt-indentation 'headline-data)
          (org-indent-mode-turns-off-org-adapt-indentation nil)
          (org-indent-mode-turns-on-hiding-stars t)
          (org-hide-leading-stars t)
          (org-cycle-global-at-bob t)
          (org-cycle-separator-lines 0)
          (org-inlinetask-min-level 5)
          (org-inlinetask-show-first-star t))
      (org-mode)
      (insert "#+STARTUP: content indent\n")
      (insert "* TODO Project :work:\n")
      (insert "SCHEDULED: <2026-05-27 Wed>\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:Effort: 1:00\n:END:\n")
      (insert "Project paragraph\n")
      (insert "- [ ] root item\n")
      (insert "  - [X] child item\n")
      (insert "    child continuation\n")
      (insert "** NEXT Alpha\nAlpha body\n")
      (insert "*** WAIT Alpha child\nAlpha child body\n")
      (insert "**** TODO Alpha fourth\nAlpha fourth body\n")
      (insert "***** Inline task\nInline task body\n***** END\n")
      (insert "** TODO Beta\nBeta body\n*** TODO Beta child\nBeta child body\n")
      (org-indent-mode 1)
      (org-indent-indent-buffer)
      (font-lock-ensure (point-min) (point-max))
      (let ((needles
             '("Project" "SCHEDULED:" ":Owner:" "Project paragraph"
               "root item" "child item" "Alpha" "Alpha child"
               "Alpha fourth" "Inline task" "Inline task body"
               "Beta" "Beta child"))
            states)
        (let ((prefix-info
               (lambda (pos)
                 (let ((lp (get-text-property pos 'line-prefix))
                       (wp (get-text-property pos 'wrap-prefix)))
                   (list
                    (and (stringp lp)
                         (list (length lp)
                               (substring-no-properties lp)
                               (get-text-property 0 'face lp)))
                    (and (stringp wp)
                         (list (length wp)
                               (substring-no-properties wp)
                               (get-text-property 0 'face wp)))))))
              (snapshot
               (lambda (label)
                 (font-lock-ensure (point-min) (point-max))
                 (list label
                       org-cycle-global-status
                       org-cycle-subtree-status
                       (mapcar
                        (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (search-forward needle)
                            (let ((pos (line-beginning-position)))
                              (list needle
                                    (line-number-at-pos pos)
                                    (org-current-level)
                                    (org-at-heading-p)
                                    (invisible-p pos)
                                    (get-text-property pos 'face)
                                    (funcall prefix-info pos)))))
                        needles)
                       (save-excursion
                         (goto-char (point-min))
                         (let (rows)
                           (while (not (eobp))
                             (let ((pos (line-beginning-position)))
                               (push
                                (list (buffer-substring-no-properties
                                       pos (line-end-position))
                                      (funcall prefix-info pos)
                                      (get-text-property pos 'invisible))
                                rows))
                             (forward-line 1))
                           (nreverse rows)))
                       (count-matches "^\\*+ " (point-min) (point-max))
                       (count-lines (point-min) (point-max))))))
          (push (funcall snapshot 'initial) states)
          (org-cycle-set-startup-visibility)
          (push (funcall snapshot 'startup) states)
          (goto-char (point-min))
          (search-forward "Alpha fourth")
          (beginning-of-line)
          (dotimes (_ 4)
            (org-cycle)
            (push (funcall snapshot 'cycle-alpha-fourth) states))
          (org-fold-hide-subtree)
          (org-end-of-subtree t t)
          (insert "**** TODO Inserted sibling\nInserted body\n")
          (push (funcall snapshot 'after-hidden-insert) states)
          (org-fold-show-all)
          (goto-char (point-min))
          (search-forward "Beta child")
          (beginning-of-line)
          (org-demote-subtree)
          (search-forward "Beta child")
          (beginning-of-line)
          (org-promote-subtree)
          (goto-char (point-min))
          (search-forward "root item")
          (end-of-line)
          (insert "\n  - [ ] inserted child\n    inserted continuation")
          (goto-char (point-min))
          (search-forward ":Effort:")
          (end-of-line)
          (insert "\n:Priority: A")
          (push (funcall snapshot 'after-edits) states)
          (org-indent-indent-buffer)
          (push (funcall snapshot 'after-reindent) states)
          (goto-char (point-min))
          (dotimes (_ 5)
            (org-cycle-global)
            (push (funcall snapshot 'global-cycle) states))
          (org-fold-show-all)
          (let* ((copied (filter-buffer-substring
                          (point-min) (point-max) nil))
                 (merged nil)
                 (prop-leak
                  (list (text-property-any 0 (length copied)
                                           'line-prefix nil copied)
                        (text-property-any 0 (length copied)
                                           'wrap-prefix nil copied))))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list (nreverse states)
                  prop-leak
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}
