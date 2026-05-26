use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_repeated_cycle_preserves_visibility_and_text_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n* D\nbody D\n")
    (goto-char (point-min))
    (let ((snapshot
           (lambda ()
             (list
              (buffer-substring-no-properties (point-min) (point-max))
              (mapcar
               (lambda (needle)
                 (invisible-p
                  (save-excursion
                    (goto-char (point-min))
                    (search-forward needle)
                    (point))))
               '("body A" "B" "body B" "C" "body C" "D" "body D")))))
          states)
      (dotimes (_ 6)
        (org-cycle)
        (push (funcall snapshot) states))
      (org-fold-show-all)
      (push (funcall snapshot) states)
      (nreverse states))))"#,
    );
}

#[test]
fn org_fold_subtree_show_sublevels_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n* D\nbody D\n")
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (org-fold-hide-subtree)
    (let ((hidden-b
           (mapcar
            (lambda (needle)
              (invisible-p
               (save-excursion
                 (goto-char (point-min))
                 (search-forward needle)
                 (point))))
            '("body B" "C" "body C" "D"))))
      (org-fold-show-subtree)
      (let ((shown-b
             (mapcar
              (lambda (needle)
                (invisible-p
                 (save-excursion
                   (goto-char (point-min))
                   (search-forward needle)
                   (point))))
              '("body B" "C" "body C" "D"))))
        (goto-char (point-min))
        (org-fold-hide-sublevels 2)
        (let ((sublevels
               (mapcar
                (lambda (needle)
                  (invisible-p
                   (save-excursion
                     (goto-char (point-min))
                     (search-forward needle)
                     (point))))
                '("A" "body A" "B" "body B" "C" "body C" "D" "body D"))))
          (list hidden-b
                shown-b
                sublevels
                (buffer-substring-no-properties (point-min) (point-max))))))))"#,
    );
}

#[test]
fn org_font_lock_heading_faces_level_four_plus_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* L1\n** L2\n*** L3\n**** L4\n***** L5\n****** L6\n")
    (font-lock-ensure (point-min) (point-max))
    (goto-char (point-min))
    (let (out)
      (while (re-search-forward "^\\*+ \\(L[0-9]\\)" nil t)
        (push (list (substring-no-properties (match-string 1))
                    (get-text-property (match-beginning 1) 'face)
                    (get-text-property (line-beginning-position) 'face))
              out))
      (nreverse out))))"#,
    );
}

#[test]
fn org_local_cycle_then_edit_preserves_newline_structure_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n**** D\nbody D\n")
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (let (states)
      (dotimes (_ 5)
        (org-cycle)
        (push
         (mapcar
          (lambda (needle)
            (invisible-p
             (save-excursion
               (goto-char (point-min))
               (search-forward needle)
               (point))))
          '("body A" "body B" "C" "body C" "D" "body D"))
         states))
      (org-fold-show-all)
      (goto-char (point-min))
      (search-forward "body B")
      (end-of-line)
      (insert "\ninserted under B")
      (list (nreverse states)
            (mapcar
             (lambda (needle)
               (invisible-p
                (save-excursion
                  (goto-char (point-min))
                  (search-forward needle)
                  (point))))
             '("inserted" "D" "body D"))
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_cycle_cut_paste_subtree_reexpand_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nbody A\n** B\nbody B\n*** C\nbody C\n* E\nbody E\n")
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (org-cycle)
    (org-cycle)
    (org-fold-show-all)
    (goto-char (point-min))
    (search-forward "C")
    (beginning-of-line)
    (org-cut-subtree)
    (goto-char (point-max))
    (org-paste-subtree 2)
    (org-cycle-overview)
    (org-fold-show-all)
    (list
     (mapcar
      (lambda (needle)
        (invisible-p
         (save-excursion
           (goto-char (point-min))
           (search-forward needle)
           (point))))
      '("body A" "B" "body B" "C" "body C" "E" "body E"))
     (buffer-substring-no-properties (point-min) (point-max)))))"#,
    );
}

#[test]
fn org_cycle_hide_drawers_show_all_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (with-temp-buffer
    (org-mode)
    (insert "* A\n")
    (insert ":PROPERTIES:\n:X: y\n:END:\n")
    (insert "body\n")
    (insert "** B\n")
    (insert ":LOGBOOK:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:00] =>  1:00\n")
    (insert ":END:\n")
    (insert "body B\n")
    (goto-char (point-min))
    (org-cycle-hide-drawers 'children)
    (let ((hidden
           (mapcar
            (lambda (needle)
              (invisible-p
               (save-excursion
                 (goto-char (point-min))
                 (search-forward needle)
                 (point))))
            '(":X:" "CLOCK" "body" "B"))))
      (org-fold-show-all)
      (let ((shown
             (mapcar
              (lambda (needle)
                (invisible-p
                 (save-excursion
                   (goto-char (point-min))
                   (search-forward needle)
                   (point))))
              '(":X:" "CLOCK" "body" "B"))))
        (list hidden
              shown
              (buffer-substring-no-properties (point-min) (point-max)))))))"#,
    );
}

#[test]
fn org_deep_heading_font_lock_after_level_edits_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-cycle-level-faces nil)
          (org-level-color-stars-only nil))
      (org-mode)
      (insert "* L1\n** L2\n*** L3\n**** L4\n***** L5\nBody\n")
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (org-demote-subtree)
      (search-forward "L5")
      (beginning-of-line)
      (org-promote-subtree)
      (font-lock-ensure (point-min) (point-max))
      (let (out)
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) \\(L[0-9]\\)" nil t)
          (push (list (match-string 1)
                      (substring-no-properties (match-string 2))
                      (org-outline-level)
                      (get-text-property (match-beginning 1) 'face)
                      (get-text-property (match-beginning 2) 'face))
                out))
        (list (nreverse out)
              (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_global_cycle_deep_sibling_visibility_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* Root\n")
    (insert "** A\nA body\n*** A1\nA1 body\n**** A1a\nA1a body\n")
    (insert "** B\nB body\n*** B1\nB1 body\n**** B1a\nB1a body\n")
    (insert "* Tail\nTail body\n")
    (let ((snapshot
           (lambda ()
             (mapcar
              (lambda (needle)
                (let ((pos (save-excursion
                             (goto-char (point-min))
                             (search-forward needle)
                             (point))))
                  (list needle (not (null (org-invisible-p pos))))))
              '("Root" "A body" "A1" "A1a body" "B body" "B1" "B1a body"
                "Tail" "Tail body"))))
          states)
      (dotimes (_ 5)
        (org-cycle-global)
        (push (funcall snapshot) states))
      (org-fold-show-all)
      (push (funcall snapshot) states)
      (list (nreverse states)
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_reveal_hidden_deep_heading_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fold-show-context-detail '((default . lineage))))
      (org-mode)
      (insert "* A\nA body\n** B\nB body\n*** C\nC body\n**** D\nD body\n")
      (insert "* E\nE body\n")
      (goto-char (point-min))
      (org-fold-hide-sublevels 1)
      (goto-char (point-min))
      (search-forward "D body")
      (org-fold-reveal)
      (let ((visibility
             (mapcar
              (lambda (needle)
                (let ((pos (save-excursion
                             (goto-char (point-min))
                             (search-forward needle)
                             (point))))
                  (list needle (not (null (org-invisible-p pos))))))
              '("A body" "B" "B body" "C" "C body" "D" "D body" "E" "E body"))))
        (list visibility
              (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}
