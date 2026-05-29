use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_cycle_visibility_state_transitions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nBody A.\n\n")
    (insert "** A1\nBody A1.\n\n")
    (insert "*** A1a\nBody A1a.\n\n")
    (insert "*** A1b\nBody A1b.\n\n")
    (insert "** A2\nBody A2.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle
                                (line-number-at-pos)
                                (invisible-p (point))
                                (org-outline-level))
                          (list needle 'not-found nil nil))))
                  '("A" "A1" "A1a" "A1b" "A2")))))
      ;; Cycle at A: overview->children->subtree->overview
      (goto-char (point-min))
      (search-forward "A")
      (beginning-of-line)
      (let ((v0 (funcall vis)))
        (org-cycle nil)  ;; overview: only top-level visible
        (let ((v1 (funcall vis)))
          (org-cycle nil)  ;; children: show A1, A2
          (let ((v2 (funcall vis)))
            (org-cycle nil)  ;; subtree: show all
            (let ((v3 (funcall vis)))
              (org-cycle nil)  ;; back to overview
              (let ((v4 (funcall vis)))
                (list v0 v1 v2 v3 v4
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_cycle_then_edit_preserves_visibility_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* P\n")
    (insert "** P1\nBody P1.\n\n")
    (insert "** P2\nBody P2.\n\n")
    (insert "** P3\nBody P3.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle (invisible-p (point)))
                          (list needle 'not-found))))
                  '("P" "P1" "P2" "P3")))))
      ;; Cycle P to show children
      (goto-char (point-min))
      (search-forward "P")
      (beginning-of-line)
      (org-cycle nil)  ;; overview
      (org-cycle nil)  ;; children visible
      (let ((after-cycle (funcall vis)))
        ;; Edit: insert P4 under P
        (goto-char (point-max))
        (insert "** P4\nBody P4.\n")
        (let ((after-edit (funcall vis)))
          ;; Re-cycle P
          (goto-char (point-min))
          (search-forward "P\n")
          (beginning-of-line)
          (org-cycle nil)  ;; overview
          (org-cycle nil)  ;; children visible
          (let ((after-re-cycle (funcall vis)))
            (list after-cycle after-edit after-re-cycle
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))))"##,
    );
}

#[test]
fn org_global_cycle_with_hidden_edits_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Project\n")
    (insert "** DONE Task-A\nBody A.\n\n")
    (insert "** TODO Task-B\nBody B.\n\n")
    (insert "** WAIT Task-C\nBody C.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle
                                (line-number-at-pos)
                                (invisible-p (point))
                                (org-outline-level))
                          (list needle 'not-found nil nil))))
                  '("Project" "Task-A" "Task-B" "Task-C")))))
      ;; Global cycle: overview
      (org-global-cycle nil)
      (let ((v1 (funcall vis)))
        ;; Global cycle: children
        (org-global-cycle nil)
        (let ((v2 (funcall vis)))
          ;; Global cycle: all
          (org-global-cycle nil)
          (let ((v3 (funcall vis)))
            ;; Edit: insert Task-D
            (goto-char (point-max))
            (insert "** NEXT Task-D\nBody D.\n")
            ;; Re-global-cycle: overview
            (org-global-cycle nil)
            (let ((v4 (funcall vis)))
              ;; Re-global-cycle: children
              (org-global-cycle nil)
              (let ((v5 (funcall vis)))
                (list v1 v2 v3 v4 v5
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_cycle_property_drawer_visibility_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Alpha\n")
    (insert ":PROPERTIES:\n:Effort: 2h\n:Owner: Alice\n:END:\n")
    (insert "Body alpha.\n\n")
    (insert "** DONE Beta\n")
    (insert ":PROPERTIES:\n:Effort: 1h\n:Owner: Bob\n:END:\n")
    (insert "Body beta.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle (invisible-p (point)))
                          (list needle 'not-found))))
                  '("Alpha" "Effort" "Owner" "Body alpha" "Beta")))))
      ;; Cycle Alpha: overview
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-cycle nil)
      (let ((v1 (funcall vis)))
        ;; Cycle Alpha: children
        (org-cycle nil)
        (let ((v2 (funcall vis)))
          ;; Cycle Alpha: subtree
          (org-cycle nil)
          (let ((v3 (funcall vis)))
            ;; Edit: set property
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (org-set-property "Status" "active")
            ;; Re-cycle
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (org-cycle nil)
            (let ((v4 (funcall vis)))
              (list v1 v2 v3 v4
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))))))"##,
    );
}

#[test]
fn org_cycle_clock_logbook_visibility_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task\n")
    (insert ":LOGBOOK:\nCLOCK: [2026-05-28 Wed 09:00]--[2026-05-28 Wed 11:00] =>  2:00\n:END:\n")
    (insert ":PROPERTIES:\n:Effort: 3h\n:END:\n")
    (insert "Body.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle (invisible-p (point)))
                          (list needle 'not-found))))
                  '("Task" "LOGBOOK" "CLOCK" "Effort" "Body")))))
      ;; Cycle: overview
      (goto-char (point-min))
      (search-forward "Task")
      (beginning-of-line)
      (org-cycle nil)
      (let ((v1 (funcall vis)))
        ;; Cycle: children
        (org-cycle nil)
        (let ((v2 (funcall vis)))
          ;; Cycle: subtree
          (org-cycle nil)
          (let ((v3 (funcall vis)))
            ;; Edit: add clock
            (goto-char (point-min))
            (search-forward "Task")
            (end-of-line)
            (insert "\nCLOCK: [2026-05-28 Wed 14:00]--[2026-05-28 Wed 15:00] =>  1:00\n")
            ;; Re-cycle
            (goto-char (point-min))
            (search-forward "Task")
            (beginning-of-line)
            (org-cycle nil)
            (let ((v4 (funcall vis)))
              (list v1 v2 v3 v4
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))))))"##,
    );
}

#[test]
fn org_cycle_tag_toggle_visibility_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Alpha :work:\nBody alpha.\n\n")
    (insert "** DONE Beta :home:\nBody beta.\n\n")
    (insert "** TODO Gamma :work:urgent:\nBody gamma.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle
                                (invisible-p (point))
                                (org-get-tags nil t))
                          (list needle 'not-found nil))))
                  '("Alpha" "Beta" "Gamma")))))
      ;; Cycle Alpha: overview
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-cycle nil)
      (let ((v1 (funcall vis)))
        ;; Cycle: children
        (org-cycle nil)
        (let ((v2 (funcall vis)))
          ;; Toggle tag on Alpha
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-toggle-tag "review" 'on)
          ;; Re-cycle
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-cycle nil)
          (let ((v3 (funcall vis)))
            (list v1 v2 v3
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))))"##,
    );
}

#[test]
fn org_cycle_font_lock_after_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Alpha\nBody alpha.\n\n")
      (insert "** DONE Beta\nBody beta.\n\n")
      (insert "** TODO Gamma\nBody gamma.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (invisible-p (point))
                                   (get-text-property (line-beginning-position) 'face))
                             (list needle 'not-found nil))))
                     '("Alpha" "Beta" "Gamma")))))
        ;; Initial
        (let ((v0 (funcall snap)))
          ;; Cycle Alpha: overview
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-cycle nil)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Cycle: children
            (org-cycle nil)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Edit: change Beta to TODO
              (goto-char (point-min))
              (search-forward "DONE Beta")
              (replace-match "TODO Beta")
              (font-lock-ensure (point-min) (point-max))
              ;; Re-cycle
              (goto-char (point-min))
              (search-forward "Alpha")
              (beginning-of-line)
              (org-cycle nil)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_cycle_multi_level_nested_visibility_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* L1\n")
    (insert "** L2\n")
    (insert "*** L3\n")
    (insert "**** L4\n")
    (insert "***** L5\nBody L5.\n\n")
    (insert "**** L4b\nBody L4b.\n\n")
    (insert "*** L3b\nBody L3b.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle
                                (line-number-at-pos)
                                (invisible-p (point))
                                (org-outline-level))
                          (list needle 'not-found nil nil))))
                  '("L1" "L2" "L3" "L4" "L5" "L4b" "L3b")))))
      ;; Cycle L1: overview
      (goto-char (point-min))
      (search-forward "L1")
      (beginning-of-line)
      (org-cycle nil)
      (let ((v1 (funcall vis)))
        ;; Cycle L1: children
        (org-cycle nil)
        (let ((v2 (funcall vis)))
          ;; Cycle L1: subtree (all visible)
          (org-cycle nil)
          (let ((v3 (funcall vis)))
            ;; Cycle L2 locally
            (goto-char (point-min))
            (search-forward "L2")
            (beginning-of-line)
            (org-cycle nil)
            (let ((v4 (funcall vis)))
              ;; Edit: insert L4c under L3
              (goto-char (point-min))
              (search-forward "L3b")
              (end-of-line)
              (insert "\n**** L4c\nBody L4c.\n")
              ;; Re-cycle L1
              (goto-char (point-min))
              (search-forward "L1")
              (beginning-of-line)
              (org-cycle nil)
              (let ((v5 (funcall vis)))
                (list v1 v2 v3 v4 v5
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_cycle_after_hide_all_show_all_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nBody A.\n\n")
    (insert "** A1\nBody A1.\n\n")
    (insert "** A2\nBody A2.\n\n")
    (let ((vis (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle (invisible-p (point)))
                          (list needle 'not-found))))
                  '("A" "A1" "A2")))))
      ;; Hide all
      (org-fold-hide-all)
      (let ((v1 (funcall vis)))
        ;; Show all
        (org-fold-show-all)
        (let ((v2 (funcall vis)))
          ;; Cycle A: overview
          (goto-char (point-min))
          (search-forward "A\n")
          (beginning-of-line)
          (org-cycle nil)
          (let ((v3 (funcall vis)))
            ;; Cycle A: children
            (org-cycle nil)
            (let ((v4 (funcall vis)))
              ;; Hide all again
              (org-fold-hide-all)
              (let ((v5 (funcall vis)))
                ;; Show all again
                (org-fold-show-all)
                (let ((v6 (funcall vis)))
                  (list v1 v2 v3 v4 v5 v6
                        (buffer-substring-no-properties
                         (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_cycle_then_refile_preserves_state_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (require 'org-refile)
  (let* ((root (make-temp-file "org-cycle-refile-" t))
         (file-a (expand-file-name "a.org" root))
         (file-b (expand-file-name "b.org" root))
         (org-refile-targets `((,file-b :maxlevel . 2))))
    (unwind-protect
        (progn
          (with-temp-file file-a
            (insert "* Source\n")
            (insert "** Item-1\nBody 1.\n\n")
            (insert "** Item-2\nBody 2.\n\n"))
          (with-temp-file file-b
            (insert "* Target\n"))
          (let* ((buf-a (find-file-noselect file-a))
                 (vis (lambda ()
                        (with-current-buffer buf-a
                          (mapcar
                           (lambda (needle)
                             (save-excursion
                               (goto-char (point-min))
                               (if (search-forward needle nil t)
                                   (list needle (invisible-p (point)))
                                   (list needle 'not-found))))
                           '("Source" "Item-1" "Item-2"))))))
            ;; Cycle Source: overview
            (with-current-buffer buf-a
              (org-mode)
              (goto-char (point-min))
              (search-forward "Source")
              (beginning-of-line)
              (org-cycle nil))
            (let ((v1 (funcall vis)))
              ;; Cycle Source: children
              (with-current-buffer buf-a
                (goto-char (point-min))
                (search-forward "Source")
                (beginning-of-line)
                (org-cycle nil))
              (let ((v2 (funcall vis)))
                ;; Refile Item-1
                (with-current-buffer buf-a
                  (goto-char (point-min))
                  (search-forward "Item-1")
                  (beginning-of-line)
                  (org-refile nil nil (list "Target" file-b nil nil)))
                (let ((v3 (funcall vis)))
                  ;; Re-cycle Source
                  (with-current-buffer buf-a
                    (goto-char (point-min))
                    (search-forward "Source")
                    (beginning-of-line)
                    (org-cycle nil))
                  (let ((v4 (funcall vis)))
                    (list v1 v2 v3 v4
                          (with-current-buffer buf-a
                            (buffer-substring-no-properties
                             (point-min) (point-max)))))))))))
      (dolist (f (list file-a file-b))
        (when (get-file-buffer f) (kill-buffer (get-file-buffer f)))
        (when (file-exists-p f) (delete-file f)))
      (delete-directory root t))))"##,
    );
}
