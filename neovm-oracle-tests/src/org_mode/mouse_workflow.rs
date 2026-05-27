use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_mouse_insert_menu_priority_checkbox_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-mouse)
  (with-temp-buffer
    (let ((org-priority-lowest ?D)
          (org-priority-default ?B)
          (org-todo-keywords '((sequence "TODO" "WAIT" "|" "DONE")))
          menu-after-actions replace-menu)
      (org-mode)
      (insert "* TODO [#A] Alpha :work:urgent:\n")
      (insert "- item one\n")
      (insert "- [ ] item two\n")
      (insert "Middle line\n")
      (insert "\n")
      (let ((line-states nil)
            (menus nil))
        (goto-char (point-min))
        (search-forward "Alpha")
        (push (list 'headline-middle
                    (org-mouse-line-position)
                    (org-mouse-get-priority)
                    (org-mouse-get-priority t))
              line-states)
        (org-mouse-end-headline)
        (push (list 'headline-end
                    (point)
                    (buffer-substring-no-properties
                     (line-beginning-position) (point)))
              line-states)
        (goto-char (point-min))
        (search-forward "item one")
        (beginning-of-line)
        (push (list 'item-begin (org-mouse-line-position))
              line-states)
        (org-mouse-insert-checkbox)
        (goto-char (point-min))
        (search-forward "item two")
        (beginning-of-line)
        (org-mouse-for-each-item 'org-mouse-insert-checkbox)
        (goto-char (point-max))
        (org-mouse-insert-heading)
        (insert "Inserted heading")
        (goto-char (point-min))
        (search-forward "Middle")
        (org-mouse-insert-item "dropped text")
        (goto-char (point-max))
        (org-mouse-insert-item "tail text")
        (goto-char (point-min))
        (search-forward "TODO")
        (push (list 'todo-menu
                    (mapcar (lambda (item)
                              (and (vectorp item)
                                   (list (aref item 0)
                                         (aref item 2)
                                         (aref item 4))))
                            (org-mouse-todo-menu "TODO")))
              menus)
        (goto-char (point-min))
        (search-forward ":work:")
        (push (list 'tag-menu
                    (mapcar (lambda (item)
                              (cond ((vectorp item)
                                     (list (aref item 0)
                                           (aref item 2)
                                           (aref item 4)))
                                    (t item)))
                            (org-mouse-tag-menu)))
              menus)
        (goto-char (point-min))
        (search-forward "[#A]")
        (setq replace-menu
              (mapcar (lambda (item)
                        (cond ((vectorp item)
                               (list (aref item 0)
                                     (aref item 2)
                                     (aref item 4)))
                              (t item)))
                      (org-mouse-keyword-replace-menu
                       (org-mouse-priority-list) 1 "Priority %s" t)))
        (funcall (aref (nth 2 (org-mouse-keyword-replace-menu
                               (org-mouse-priority-list) 1
                               "Priority %s" t))
                       1))
        (setq menu-after-actions
              (buffer-substring-no-properties (point-min) (point-max)))
        (list (nreverse line-states)
              (nreverse menus)
              replace-menu
              menu-after-actions
              (org-mouse-clip-text "abcdefghijklmnopqrstuvwxyz" 12)
              (org-mouse-agenda-type 'todo-tree)
              (org-mouse-agenda-type 'unknown)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}
