use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_todo_dependency_blockers_and_noblocking_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-enforce-todo-dependencies t)
          (org-todo-keywords '((sequence "TODO" "WAIT" "|" "DONE"))))
      (org-mode)
      (insert "* TODO Parent\n")
      (insert "** TODO Child A\n")
      (insert "** TODO Child B\n")
      (insert "* TODO Ordered\n")
      (insert ":PROPERTIES:\n:ORDERED: t\n:END:\n")
      (insert "** TODO First\n")
      (insert "** TODO Second\n")
      (goto-char (point-min))
      (let ((parent-blocked (org-entry-blocked-p))
            parent-done-attempt)
        (setq parent-done-attempt
              (condition-case err
                  (progn (org-todo "DONE") 'ok)
                (error (cons (car err) (cdr err)))))
        (goto-char (point-min))
        (search-forward "Second")
        (beginning-of-line)
        (let ((second-blocked (org-entry-blocked-p))
              second-done-attempt)
          (setq second-done-attempt
                (condition-case err
                    (progn (org-todo "DONE") 'ok)
                  (error (cons (car err) (cdr err)))))
          (org-entry-put nil "NOBLOCKING" "t")
          (let ((second-unblocked (org-entry-blocked-p))
                (second-done-unblocked
                 (condition-case err
                     (progn (org-todo "DONE") 'ok)
                   (error (cons (car err) (cdr err))))))
            (list parent-blocked
                  parent-done-attempt
                  second-blocked
                  second-done-attempt
                  second-unblocked
                  second-done-unblocked
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_checkbox_dependency_statistics_cookie_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-enforce-todo-checkbox-dependencies t)
          (org-todo-keywords '((sequence "TODO" "|" "DONE"))))
      (org-mode)
      (insert "* TODO Checklist [0/3] [0%]\n")
      (insert "- [X] Done item\n")
      (insert "- [ ] Open item\n")
      (insert "- [-] Partial item\n")
      (insert "  - [X] Nested done\n")
      (insert "  - [ ] Nested open\n")
      (goto-char (point-min))
      (let ((initial-blocked (org-entry-blocked-p))
            (initial-attempt
             (condition-case err
                 (progn (org-todo "DONE") 'ok)
               (error (cons (car err) (cdr err))))))
        (search-forward "Open item")
        (org-ctrl-c-ctrl-c)
        (search-forward "Nested open")
        (org-ctrl-c-ctrl-c)
        (goto-char (point-min))
        (org-update-statistics-cookies t)
        (let ((after-checks (buffer-substring-no-properties
                             (point-min) (point-max)))
              (after-blocked (org-entry-blocked-p))
              (after-attempt
               (condition-case err
                   (progn (org-todo "DONE") 'ok)
                 (error (cons (car err) (cdr err))))))
          (list initial-blocked
                initial-attempt
                after-checks
                after-blocked
                after-attempt
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_todo_state_tag_triggers_statistics_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-todo-keywords '((sequence "TODO" "WAIT" "|" "DONE" "CANCELED")))
          (org-todo-state-tags-triggers
           '(("WAIT" ("waiting" . t) ("active"))
             ("DONE" ("done" . t) ("waiting") ("active"))
             ("CANCELED" ("canceled" . t) ("waiting") ("active"))))
          (org-log-done nil))
      (org-mode)
      (insert "* Project [0/2]\n")
      (insert "** TODO Alpha :active:\n")
      (insert "** TODO Beta :active:\n")
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-todo "WAIT")
      (let ((after-wait (buffer-substring-no-properties
                         (point-min) (point-max)))
            (alpha-tags-wait (org-get-tags nil t)))
        (org-todo "DONE")
        (let ((after-done (buffer-substring-no-properties
                           (point-min) (point-max)))
              (alpha-tags-done (org-get-tags nil t)))
          (goto-char (point-min))
          (search-forward "Beta")
          (beginning-of-line)
          (org-todo "CANCELED")
          (goto-char (point-min))
          (org-update-statistics-cookies t)
          (list after-wait
                alpha-tags-wait
                after-done
                alpha-tags-done
                (org-element-map (org-element-parse-buffer) 'headline
                  (lambda (h)
                    (list (org-element-property :todo-keyword h)
                          (org-element-property :raw-value h)
                          (org-element-property :tags h))))
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}
