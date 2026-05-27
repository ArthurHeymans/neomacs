use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_inlinetask_region_insert_promote_demote_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-inlinetask)
  (with-temp-buffer
    (let ((org-inlinetask-min-level 5)
          (org-inlinetask-default-state "TODO")
          (transient-mark-mode t))
      (org-mode)
      (insert "* Parent\n")
      (insert "Before text\n")
      (insert "Body line one\nBody line two\n")
      (insert "After text\n")
      (goto-char (point-min))
      (search-forward "Body line one")
      (beginning-of-line)
      (push-mark (point) nil t)
      (search-forward "Body line two")
      (end-of-line)
      (org-inlinetask-insert-task nil)
      (let ((after-insert
             (buffer-substring-no-properties (point-min) (point-max)))
            begin-pos end-pos)
        (org-inlinetask-goto-beginning)
        (setq begin-pos (point))
        (org-inlinetask-demote)
        (org-inlinetask-goto-beginning)
        (org-inlinetask-promote)
        (org-inlinetask-goto-end)
        (setq end-pos (point))
        (list after-insert
              begin-pos
              end-pos
              (org-inlinetask-at-task-p)
              (org-inlinetask-get-task-level)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_inlinetask_element_export_archive_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-inlinetask)
  (require 'ox-html)
  (with-temp-buffer
    (let ((org-inlinetask-min-level 4)
          (org-export-with-toc nil))
      (org-mode)
      (insert "#+TITLE: Inline\n")
      (insert "* Project\n")
      (insert "Plain before.\n")
      (insert "**** TODO Inline one :tag:\n")
      (insert "SCHEDULED: <2026-05-27 Wed>\n")
      (insert ":PROPERTIES:\n:Effort: 0:20\n:END:\n")
      (insert "Inline body with [[https://example.org][link]].\n")
      (insert "**** END\n")
      (insert "Plain after.\n")
      (let* ((tree (org-element-parse-buffer))
             (tasks
              (org-element-map tree 'inlinetask
                (lambda (task)
                  (list (org-element-property :todo-keyword task)
                        (org-element-property :raw-value task)
                        (org-element-property :tags task)
                        (org-element-property :level task)
                        (org-element-property :contents-begin task)))))
             (html (replace-regexp-in-string
                    "org[[:alnum:]]+"
                    "org-id"
                    (org-export-as 'html nil nil t nil))))
        (list tasks
              (not (null (string-match-p "Inline one" html)))
              (not (null (string-match-p "SCHEDULED" html)))
              (not (null (string-match-p "Effort" html)))
              html)))))"##,
    );
}

#[test]
fn org_inlinetask_visibility_and_remove_end_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-inlinetask)
  (with-temp-buffer
    (let ((org-inlinetask-min-level 4))
      (org-mode)
      (insert "* Parent\n")
      (insert "**** TODO Inline\n")
      (insert "Hidden body\n")
      (insert "**** END\n")
      (insert "** Child\nBody\n")
      (goto-char (point-min))
      (search-forward "Inline")
      (beginning-of-line)
      (let ((before (list (org-inlinetask-at-task-p)
                          (org-inlinetask-in-task-p)
                          (org-inlinetask-get-task-level))))
        (org-inlinetask-toggle-visibility 'fold)
        (let ((folded (org-fold-folded-p (line-end-position) 'headline)))
          (org-inlinetask-toggle-visibility 'unfold)
          (org-inlinetask-goto-end)
          (forward-line -1)
          (org-inlinetask-remove-END-maybe)
          (list before
                folded
                (org-fold-folded-p (line-end-position) 'headline)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_inlinetask_fontify_edit_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-inlinetask)
  (require 'ox-html)
  (with-temp-buffer
    (let ((org-inlinetask-min-level 4)
          (org-inlinetask-show-first-star t)
          (org-todo-keywords '((sequence "TODO" "WAIT" "|" "DONE")))
          (org-export-with-toc nil))
      (org-mode)
      (insert "#+TITLE: Inline Font\n")
      (insert "* Parent\n")
      (insert "Before.\n")
      (insert "**** TODO Inline Alpha :old:\n")
      (insert "SCHEDULED: <2026-05-27 Wed>\n")
      (insert "Body with *bold* and [[https://example.org][link]].\n")
      (insert "**** END\n")
      (insert "**** WAIT Inline Beta\n")
      (insert ":PROPERTIES:\n:Effort: 0:10\n:END:\n")
      (insert "Beta body.\n")
      (insert "**** END\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((before
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (search-forward needle)
                  (list needle
                        (org-inlinetask-in-task-p)
                        (get-text-property (match-beginning 0) 'face)
                        (get-text-property (match-beginning 0)
                                           'font-lock-fontified))))
              '("Inline Alpha" "Inline Beta" "bold" "link"))))
        (goto-char (point-min))
        (search-forward "Inline Alpha")
        (beginning-of-line)
        (org-inlinetask-toggle-visibility 'fold)
        (let ((folded (list (org-fold-folded-p (line-end-position)
                                               'headline)
                            (invisible-p
                             (save-excursion
                               (search-forward "Body with")
                               (point))))))
          (org-inlinetask-toggle-visibility 'unfold)
          (org-todo "DONE")
          (org-toggle-tag "old" 'off)
          (org-toggle-tag "new" 'on)
          (let* ((tree (org-element-parse-buffer))
                 (tasks
                  (org-element-map tree 'inlinetask
                    (lambda (task)
                      (list (org-element-property :todo-keyword task)
                            (org-element-property :raw-value task)
                            (org-element-property :tags task)
                            (org-element-property :scheduled task)))))
                 (html (replace-regexp-in-string
                        "org[[:alnum:]]+"
                        "org-id"
                        (org-export-as 'html nil nil t nil))))
            (list before
                  folded
                  tasks
                  (not (null (string-match-p "Inline Alpha" html)))
                  (not (null (string-match-p "DONE" html)))
                  (not (null (string-match-p "new" html)))
                  html
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}
