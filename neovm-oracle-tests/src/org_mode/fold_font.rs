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

#[test]
fn org_mixed_cycle_deep_siblings_no_line_merge_combo() {
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
    (insert "***** A1a-i\nA1a-i body\n")
    (insert "** B\nB body\n*** B1\nB1 body\n**** B1a\nB1a body\n")
    (insert "***** B1a-i\nB1a-i body\n")
    (insert "** C\nC body\n* Tail\nTail body\n")
    (let ((snapshot
           (lambda (label)
             (list label
                   (mapcar
                    (lambda (needle)
                      (let ((pos (save-excursion
                                   (goto-char (point-min))
                                   (search-forward needle)
                                   (point))))
                        (list needle (not (null (org-invisible-p pos))))))
                    '("A body" "A1" "A1a" "A1a-i body"
                      "B body" "B1" "B1a" "B1a-i body"
                      "C body" "Tail" "Tail body"))
                   (split-string
                    (buffer-substring-no-properties (point-min) (point-max))
                    "\n" t)))))
          states)
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (dotimes (_ 4) (org-cycle) (push (funcall snapshot 'local-b) states))
      (goto-char (point-min))
      (search-forward "A1a")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (push (funcall snapshot 'hide-a1a) states)
      (org-fold-show-subtree)
      (push (funcall snapshot 'show-a1a) states)
      (dotimes (_ 5) (org-cycle-global) (push (funcall snapshot 'global) states))
      (org-fold-show-all)
      (push (funcall snapshot 'all) states)
      (list (nreverse states)
            (count-matches "^\\*+ " (point-min) (point-max))
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_fold_region_boundaries_after_hidden_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\nA body\n** B\nB body\n*** C\nC body\n**** D\nD body\n")
    (insert "** E\nE body\n* F\nF body\n")
    (let ((probe
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (search-forward needle)
               (let ((region (org-fold-get-region-at-point '(headline drawer))))
                 (list needle
                       (not (null (org-fold-folded-p (point) 'headline)))
                       (and region
                            (buffer-substring-no-properties
                             (car region) (min (cdr region) (point-max))))
                       (org-fold-next-visibility-change (point) nil t)
                       (org-fold-previous-visibility-change (point) nil t)))))))
          before after)
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-cycle)
      (setq before (mapcar probe '("B body" "C" "D body" "E body" "F body")))
      (org-fold-show-subtree)
      (org-end-of-subtree)
      (insert "** Inserted\nInserted body\n*** Inserted child\nChild body\n")
      (goto-char (point-min))
      (search-forward "Inserted child")
      (beginning-of-line)
      (org-demote-subtree)
      (org-fold-hide-sublevels 2)
      (setq after (mapcar probe
                          '("B body" "D body" "Inserted" "Child body"
                            "E body" "F body")))
      (org-fold-show-all)
      (list before
            after
            (mapcar probe '("B body" "D body" "Inserted" "Child body"))
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_repeated_fold_hidden_boundary_complex_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-hide-block-startup nil))
      (org-mode)
      (insert "* Root\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "Root paragraph.\n")
      (insert "** TODO Alpha\n")
      (insert "Alpha body\n")
      (insert "#+begin_quote\nquoted alpha\n#+end_quote\n")
      (insert "*** TODO Alpha child\n")
      (insert "- [ ] child task\n")
      (insert "**** TODO Alpha L4\n")
      (insert "L4 body\n")
      (insert "***** TODO Alpha L5\n")
      (insert "L5 body\n")
      (insert "** WAIT Beta\n")
      (insert "Beta body\n")
      (insert ":LOGBOOK:\nCLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 09:15] =>  0:15\n:END:\n")
      (insert "*** TODO Beta child\n")
      (insert "Beta child body\n")
      (insert "** TODO Gamma\n")
      (insert "Gamma body\n")
      (insert "* Tail\nTail body\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snapshot
             (lambda (label)
               (list label
                     (mapcar
                      (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle
                                (line-number-at-pos)
                                (not (null (org-invisible-p (point)))))))
                      '("Root paragraph" "Alpha body" "quoted alpha"
                        "Alpha child" "Alpha L4" "Alpha L5" "Beta body"
                        "Beta child" "Gamma body" "Tail body"))
                     (count-matches "^\\*+ " (point-min) (point-max))
                     (mapcar
                      (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle
                                (get-text-property
                                 (line-beginning-position) 'face)
                                (get-text-property
                                 (match-beginning 0) 'face)
                                (get-text-property
                                 (match-beginning 0)
                                 'font-lock-fontified))))
                      '("Alpha L4" "Alpha L5")))))
            states)
        (goto-char (point-min))
        (search-forward "Alpha")
        (beginning-of-line)
        (dotimes (_ 3)
          (org-cycle)
          (push (funcall snapshot 'alpha-cycle) states))
        (org-fold-show-subtree)
        (search-forward "Alpha L5")
        (beginning-of-line)
        (org-fold-hide-subtree)
        (end-of-line)
        (insert "\nInserted under hidden L5\n")
        (push (funcall snapshot 'after-hidden-insert) states)
        (org-fold-show-subtree)
        (goto-char (point-min))
        (search-forward "Beta")
        (beginning-of-line)
        (dotimes (_ 2)
          (org-cycle)
          (push (funcall snapshot 'beta-cycle) states))
        (org-fold-show-subtree)
        (search-forward "Beta child body")
        (end-of-line)
        (insert "\n*** TODO Beta inserted\nInserted beta body\n")
        (goto-char (point-min))
        (search-forward "Gamma")
        (beginning-of-line)
        (org-fold-hide-subtree)
        (org-fold-show-all)
        (font-lock-ensure (point-min) (point-max))
        (push (funcall snapshot 'after-show-all) states)
        (let ((lines (split-string
                      (buffer-substring-no-properties
                       (point-min) (point-max))
                      "\n" t))
              (bad-heading-lines nil))
          (dolist (line lines)
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line bad-heading-lines)))
          (list (nreverse states)
                (nreverse bad-heading-lines)
                (mapcar
                 (lambda (needle)
                   (save-excursion
                     (goto-char (point-min))
                     (search-forward needle)
                     (list needle
                           (line-number-at-pos)
                           (buffer-substring-no-properties
                            (line-beginning-position)
                            (line-end-position)))))
                 '("** TODO Alpha" "***** TODO Alpha L5"
                   "Inserted under hidden L5" "** WAIT Beta"
                   "*** TODO Beta inserted" "** TODO Gamma" "* Tail"))
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_font_lock_deep_headings_after_cycle_and_edits_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-cycle)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO [#A] L1 :work:\n")
      (insert "** NEXT L2 :tag:\n")
      (insert "*** WAIT L3\n")
      (insert "**** TODO L4 :deep:\n")
      (insert "***** DONE L5\n")
      (insert "****** TODO L6\n")
      (insert "******* TODO L7\n")
      (insert "******** TODO L8\n")
      (insert "body with /italic/ and =code=\n")
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (org-demote-subtree)
      (goto-char (point-min))
      (search-forward "L7")
      (beginning-of-line)
      (org-promote-subtree)
      (dotimes (_ 3) (org-cycle-global))
      (font-lock-ensure (point-min) (point-max))
      (let (out)
        (goto-char (point-min))
        (while (re-search-forward
                "^\\(\\*+\\) \\([A-Z]+\\)?\\(?: \\(\\[#[A-Z]\\]\\)\\)? \\([^:\n]+\\)\\(?: \\(:[[:alnum:]_@#%:]+:\\)\\)?"
                nil t)
          (push (list (match-string 1)
                      (match-string 2)
                      (match-string 3)
                      (substring-no-properties (match-string 4))
                      (match-string 5)
                      (org-outline-level)
                      (get-text-property (match-beginning 1) 'face)
                      (and (match-beginning 2)
                           (get-text-property (match-beginning 2) 'face))
                      (get-text-property (match-beginning 4) 'face)
                      (get-text-property (line-beginning-position)
                                         'font-lock-fontified))
                out))
        (list (nreverse out)
              (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_cycle_startup_visibility_archived_drawers_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-archive-tag "ARCHIVE")
          (org-cycle-hide-drawer-startup t)
          (org-cycle-hide-block-startup t))
      (org-mode)
      (insert "#+STARTUP: content\n")
      (insert "* Active\n")
      (insert ":PROPERTIES:\n:VISIBILITY: children\n:END:\n")
      (insert "active body\n")
      (insert "** Child\nchild body\n*** Grand\nbody grand\n")
      (insert "* Archived :ARCHIVE:\narchived body\n** Hidden child\nhidden body\n")
      (insert "* Blocks\n")
      (insert "#+begin_quote\nquoted body\n#+end_quote\n")
      (org-cycle-set-startup-visibility)
      (let ((snapshot
             (lambda ()
               (mapcar
                (lambda (needle)
                  (list needle
                        (invisible-p
                         (save-excursion
                           (goto-char (point-min))
                           (search-forward needle)
                           (point)))))
                '("Active" ":VISIBILITY:" "active body" "Child" "child body"
                  "Grand" "body grand" "Archived" "archived body"
                  "Hidden child" "hidden body" "Blocks" "quoted body")))))
        (let ((startup (funcall snapshot)))
          (goto-char (point-min))
          (search-forward "Active")
          (beginning-of-line)
          (org-cycle)
          (org-cycle)
          (let ((active-after-local (funcall snapshot)))
            (org-cycle-global)
            (org-cycle-global)
            (let ((after-global (funcall snapshot)))
              (org-fold-show-all)
              (list startup
                    active-after-local
                    after-global
                    (funcall snapshot)
                   (buffer-substring-no-properties
                    (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_face_v17_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Sibling, edit
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE New top\nNew body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "SIBLING" "New")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_face_v16_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Root subtree, edit
      (goto-char (point-min))
      (search-forward "Root")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n* DONE New top\nNew body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "New" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_face_v15_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* DONE Root\n")
      (insert "** TODO Alpha\n")
      (insert "*** DONE Beta\n")
      (insert "**** TODO Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Gamma subtree, edit
      (goto-char (point-min))
      (search-forward "Gamma")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n**** DONE Inserted under Gamma\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_face_v14_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Root subtree, edit
      (goto-char (point-min))
      (search-forward "Root")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n* DONE Inserted after Root\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_face_v13_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha, edit
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n*** DONE Inserted under Alpha\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_face_v12_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* DONE Root\n")
      (insert "** TODO Alpha\n")
      (insert "*** DONE Beta\n")
      (insert "**** TODO Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Sibling subtree, edit
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE Inserted after Sibling\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "SIBLING" "Inserted")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_all_font_v11_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Gamma, edit
      (goto-char (point-min))
      (search-forward "Gamma")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n**** DONE Inserted under Gamma\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_edit_global_show_font_face_v10_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Beta subtree, edit
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n**** DONE Inserted under Beta\nInserted body.\n")
      ;; 4 global cycles
      (dotimes (_ 4) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_todo_toggle_cycle_show_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-log-done nil))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Toggle Alpha: DONE->TODO
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-todo "TODO")
      ;; Toggle Beta: TODO->DONE
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-todo "DONE")
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face)
                                 (get-text-property (point) 'face))
                           (list needle 'not-found nil nil nil nil))))))
        (let ((headings (mapcar probe '("Root" "Alpha" "Beta" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_subtree_edit_cycle_show_font_face_v9_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* DONE Root\n")
      (insert "** TODO Alpha\n")
      (insert "*** DONE Beta\n")
      (insert "**** TODO Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Gamma subtree, edit
      (goto-char (point-min))
      (search-forward "Gamma")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n**** TODO Inserted under Gamma\nInserted body.\n")
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_hide_subtree_edit_cycle_show_font_face_v8_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Sibling, edit
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE Inserted after Sibling\nInserted body.\n")
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "SIBLING" "Inserted")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_hide_sublevels_reveal_context_font_level_v4_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "***** DONE Delta\n")
      (insert "****** TODO Epsilon\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      ;; Reveal Delta with isearch context
      (goto-char (point-min))
      (search-forward "Delta body")
      (org-fold-show-context 'isearch)
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Delta" "Epsilon" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_hide_subtree_edit_show_font_face_level_v7_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* DONE Root\n")
      (insert "** TODO Alpha\n")
      (insert "*** DONE Beta\n")
      (insert "**** TODO Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Beta subtree
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n**** TODO Inserted under Beta\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_font_level_visibility_v6_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha, edit
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n*** TODO Inserted under Alpha\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_narrow_edit_widen_cycle_font_face_level_v4_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* Root\nRoot body.\n")
      (insert "** Alpha\nAlpha body.\n")
      (insert "*** Beta\nBeta body.\n")
      (insert "**** Gamma\nGamma body.\n")
      (insert "** Sibling\nSibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Narrow to Alpha, edit, widen
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (save-restriction
        (org-narrow-to-subtree)
        (goto-char (point-max))
        (insert "*** Inserted in narrow\nInserted body.\n"))
      ;; Global cycle
      (goto-char (point-min))
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_hide_sublevels_reveal_font_face_level_v3_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "***** DONE Delta\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      ;; Reveal Gamma with isearch context
      (goto-char (point-min))
      (search-forward "Gamma body")
      (org-fold-show-context 'isearch)
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Delta" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_subtree_edit_show_global_font_level_v5_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n*** TODO Inserted under Alpha\nInserted body.\n")
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Inserted" "SIBLING")))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list headings
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_export_html_after_cycle_font_state_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (require 'ox-html)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "#+TITLE: Fold Export\n\n")
      (insert "* TODO Root :root:\n")
      (insert "Root body with *bold*.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "** NEXT Gamma\n")
      (insert "Gamma body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Cycle
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Export
      (let* ((org-export-with-toc nil)
             (html (org-export-as 'html nil nil t nil)))
        ;; Check HTML structure
        (let ((count-re (lambda (re)
                          (let ((c 0) (s 0))
                            (while (string-match re html s)
                              (setq s (match-end 0) c (1+ c)))
                            c))))
          (list (funcall count-re "<h[1-3]")
                (funcall count-re "<b>bold</b>")
                (funcall count-re "TODO")
                (funcall count-re "DONE")
                (funcall count-re "NEXT")
                (not (null (string-match-p "Root" html)))
                (not (null (string-match-p "Alpha" html)))
                (replace-regexp-in-string
                 "outline-container-org[[:alnum:]]+"
                 "outline-container-orgHASH"
                 (replace-regexp-in-string
                  "sec:org[[:alnum:]-]+" "sec:org-id"
                  (replace-regexp-in-string
                   "org[[:alnum:]-]\\{8,\\}" "orgHASH" html))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_narrow_cycle_edit_widen_v3_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* Root\nRoot body.\n")
      (insert "** Alpha\nAlpha body.\n")
      (insert "*** Beta\nBeta body.\n")
      (insert "**** Gamma\nGamma body.\n")
      (insert "** Sibling\nSibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Narrow to Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (save-restriction
        (org-narrow-to-subtree)
        ;; Cycle in narrow
        (org-cycle)
        ;; Edit in narrow
        (goto-char (point-max))
        (insert "*** Inserted in narrow\nInserted body.\n")
        ;; Show all in narrow
        (org-fold-show-all)
        (font-lock-ensure (point-min) (point-max))
        (let ((probe (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (line-number-at-pos)
                                   (invisible-p (point))
                                   (org-outline-level))
                             (list needle 'not-found nil nil))))))
          (let ((headings (mapcar probe
                                  '("Alpha" "Beta" "Gamma" "Inserted" "Root" "SIBLING"))))
            ;; Widen
            (list headings
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_narrow_subtree_cycle_edit_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* Root\n")
      (insert "** Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** Beta\n")
      (insert "Beta body.\n")
      (insert "**** Gamma\n")
      (insert "Gamma body.\n")
      (insert "** Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Narrow to Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (save-restriction
        (org-narrow-to-subtree)
        ;; Cycle in narrow
        (org-cycle)
        (org-cycle)
        ;; Edit in narrow
        (goto-char (point-max))
        (insert "*** Inserted in narrow\nInserted body.\n")
        ;; Show all in narrow
        (org-fold-show-all)
        (font-lock-ensure (point-min) (point-max))
        (let ((narrowed-headings
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
                '("Alpha" "Beta" "Gamma" "Inserted" "Root" "SIBLING"))))
          ;; Widen
          (list narrowed-headings
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_demote_subtree_show_all_v3_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\n")
      (insert "** B\n")
      (insert "*** C\n")
      (insert "* D\n")
      (insert "** E\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Demote B subtree
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-demote-subtree)
      ;; Promote E subtree
      (goto-char (point-min))
      (search-forward "E")
      (beginning-of-line)
      (org-promote-subtree)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (re-search-forward
                       (concat "^\\*+ " needle) nil t)
                      (list needle
                            (length (match-string 1))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("A" "B" "C" "D" "E")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_multiple_hidden_edits_global_show_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "** NEXT Gamma\n")
      (insert "Gamma body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n*** TODO Inserted A\nInserted A body.\n")
      ;; Hide Gamma subtree
      (goto-char (point-min))
      (search-forward "Gamma")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE Inserted G\nInserted G body.\n")
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((probe (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (if (search-forward needle nil t)
                           (list needle
                                 (line-number-at-pos)
                                 (invisible-p (point))
                                 (org-outline-level)
                                 (get-text-property (line-beginning-position) 'face))
                           (list needle 'not-found nil nil nil))))))
        (let ((headings (mapcar probe
                                '("Root" "Alpha" "Beta" "Gamma" "Inserted A" "Inserted G")))
              (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_promote_cycle_show_all_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\nbody A\n")
      (insert "** B\nbody B\n")
      (insert "*** C\nbody C\n")
      (insert "**** D\nbody D\n")
      (insert "* E\nbody E\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Promote D subtree
      (goto-char (point-min))
      (search-forward "D")
      (beginning-of-line)
      (org-promote-subtree)
      ;; Demote B subtree
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-demote-subtree)
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (length (match-string 1))
                            (org-outline-level)
                            (invisible-p (point))
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("A" "B" "C" "D" "E")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_demote_promote_cycle_global_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\nbody A\n")
      (insert "** B\nbody B\n")
      (insert "*** C\nbody C\n")
      (insert "** D\nbody D\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Demote B subtree
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-demote-subtree)
      ;; Promote D subtree
      (goto-char (point-min))
      (search-forward "D")
      (beginning-of-line)
      (org-promote-subtree)
      ;; 5 global cycles
      (dotimes (_ 5) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (length (match-string 1))
                            (org-outline-level)
                            (invisible-p (point))
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("A" "B" "C" "D")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_cut_paste_cycle_global_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\nbody A\n")
      (insert "** B\nbody B\n")
      (insert "*** C\nbody C\n")
      (insert "* D\nbody D\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Cut C, paste under D
      (goto-char (point-min))
      (search-forward "C")
      (beginning-of-line)
      (org-cut-subtree)
      (goto-char (point-min))
      (search-forward "D")
      (beginning-of-line)
      (org-paste-subtree 2)
      ;; 5 global cycles
      (dotimes (_ 5) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("A" "B" "C" "D")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_cut_paste_promote_demote_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\n")
      (insert "** B\n")
      (insert "*** C\n")
      (insert "* D\n")
      (insert "** E\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Cut C, paste under D, promote
      (goto-char (point-min))
      (search-forward "C")
      (beginning-of-line)
      (org-cut-subtree)
      (goto-char (point-min))
      (search-forward "D")
      (beginning-of-line)
      (org-paste-subtree 2)
      (goto-char (point-min))
      (search-forward "C")
      (beginning-of-line)
      (org-promote-subtree)
      ;; Demote B
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-demote-subtree)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (length (match-string 1))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("A" "B" "C" "D" "E")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_multiple_todo_toggle_show_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-log-done nil))
      (org-mode)
      (insert "* TODO A\n")
      (insert "** DONE B\n")
      (insert "*** TODO C\n")
      (insert "**** WAIT D\n")
      (insert "** NEXT E\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Toggle B: DONE->TODO
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-todo "TODO")
      ;; Toggle C: TODO->DONE
      (goto-char (point-min))
      (search-forward "C")
      (beginning-of-line)
      (org-todo "DONE")
      ;; Toggle D: WAIT->CANCELED
      (goto-char (point-min))
      (search-forward "D")
      (beginning-of-line)
      (org-todo "CANCELED")
      ;; Toggle E: NEXT->TODO
      (goto-char (point-min))
      (search-forward "E")
      (beginning-of-line)
      (org-todo "TODO")
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face)
                            (get-text-property (point) 'face))
                      (list needle 'not-found nil nil))))
              '("A" "B" "C" "D" "E")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_after_todo_toggle_cycle_show_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-log-done nil))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Toggle Alpha to TODO
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-todo "TODO")
      ;; Toggle Beta to DONE
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-todo "DONE")
      ;; Toggle Sibling to CANCELED
      (goto-char (point-min))
      (search-forward "SIBLING")
      (beginning-of-line)
      (org-todo "CANCELED")
      ;; Global cycle then show all
      (dotimes (_ 3) (org-cycle-global))
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face)
                            (get-text-property (point) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (replace-regexp-in-string
               "CLOSED: \\[.*\\]" "CLOSED: [stamp]"
               (buffer-substring-no-properties
                (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_level_after_cut_paste_subtree_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\nbody A\n")
      (insert "** B\nbody B\n")
      (insert "*** C\nbody C\n")
      (insert "* D\nbody D\n")
      (insert "** E\nbody E\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Cut C subtree
      (goto-char (point-min))
      (search-forward "C")
      (beginning-of-line)
      (org-cut-subtree)
      ;; Paste under D
      (goto-char (point-min))
      (search-forward "D")
      (beginning-of-line)
      (org-paste-subtree 2)
      ;; Cycle overview then show all
      (org-cycle-overview)
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("A" "B" "C" "D" "E")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_global_show_font_deep_v4_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Sibling subtree, edit
      (goto-char (point-min))
      (search-forward "SIBLING")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE Inserted S\nInserted S body.\n")
      ;; 3 global cycles
      (dotimes (_ 3) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "SIBLING" "Inserted S")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_sublevels_show_context_font_level_visibility_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* Root\n")
      (insert "** Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** Beta\n")
      (insert "Beta body.\n")
      (insert "**** Gamma\n")
      (insert "Gamma body.\n")
      (insert "***** Delta\n")
      (insert "Delta body.\n")
      (insert "** Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      ;; Capture hidden state
      (let ((after-hide
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Delta" "Sibling"))))
        ;; Show subtree at Alpha
        (goto-char (point-min))
        (search-forward "Alpha")
        (beginning-of-line)
        (org-fold-show-subtree)
        (let ((after-show
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (if (search-forward needle nil t)
                        (list needle
                              (invisible-p (point))
                              (org-outline-level))
                        (list needle 'not-found nil nil))))
                '("Root" "Alpha" "Beta" "Gamma" "Delta" "Sibling"))))
          ;; Merged check
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list after-hide
                  after-show
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_multiple_subtree_hide_cycle_show_font_level_v5_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n*** TODO Inserted A\nInserted A body.\n")
      ;; Hide Sibling subtree
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE Inserted S\nInserted S body.\n")
      ;; 5 global cycles
      (dotimes (_ 5) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted A"
                "Sibling" "Inserted S")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_font_level_visibility_after_multiple_cycles_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; 5 global cycles
      (dotimes (_ 5) (org-cycle-global))
      ;; Local cycle on Alpha 3 times
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (dotimes (_ 3) (org-cycle))
      ;; 5 more global cycles
      (dotimes (_ 5) (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "SIBLING")))
            (merged nil)
            (level-ok t))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) " nil t)
          (let ((stars (length (match-string 1)))
                (level (org-outline-level)))
            (unless (= stars level)
              (setq level-ok nil))))
        (list headings
              (nreverse merged)
              level-ok
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_overview_local_global_show_font_face_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Overview
      (org-cycle-overview)
      ;; Local cycle on Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-cycle)
      (org-cycle)
      ;; Global contents
      (org-cycle-contents)
      ;; Local cycle on Beta
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-cycle)
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("Root" "Root body" "Alpha" "Alpha body" "Beta" "Beta body"
                "Gamma" "Gamma body" "Sibling" "Sibling body")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_all_font_face_level_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Beta subtree
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n**** TODO Inserted under Beta\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face)
                            (get-text-property (point) 'face))
                      (list needle 'not-found nil nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted under Beta" "Sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_hide_sublevels_reveal_cycle_font_level_deep_v3_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (require 'org-cycle)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "***** DONE Delta\n")
      (insert "****** TODO Epsilon\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      ;; Reveal Delta with isearch context
      (goto-char (point-min))
      (search-forward "Delta body")
      (org-fold-show-context 'isearch)
      ;; Capture state
      (let ((after-reveal
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Delta" "Epsilon" "Sibling"))))
        ;; Cycle Gamma
        (goto-char (point-min))
        (search-forward "Gamma")
        (beginning-of-line)
        (org-cycle)
        (org-cycle)
        (let ((after-gamma-cycle
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (if (search-forward needle nil t)
                        (list needle
                              (invisible-p (point))
                              (org-outline-level))
                        (list needle 'not-found nil nil))))
                '("Root" "Alpha" "Beta" "Gamma" "Delta" "Epsilon" "Sibling"))))
          ;; Merged check
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list after-reveal
                  after-gamma-cycle
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_repeated_global_cycle_show_all_font_level_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO L1\n")
      (insert "** DONE L2\n")
      (insert "*** TODO L3\n")
      (insert "**** WAIT L4\n")
      (insert "***** DONE L5\n")
      (insert "** NEXT L2b\n")
      (font-lock-ensure (point-min) (point-max))
      ;; 10 repeated global cycles
      (dotimes (_ 10)
        (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("L1" "L2" "L3" "L4" "L5" "L2b")))
            (merged nil)
            (level-ok t))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) " nil t)
          (let ((stars (length (match-string 1)))
                (level (org-outline-level)))
            (unless (= stars level)
              (setq level-ok nil))))
        (list headings
              (nreverse merged)
              level-ok
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_narrow_subtree_show_all_widen_font_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* Root\n")
      (insert "** Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** Beta\n")
      (insert "Beta body.\n")
      (insert "**** Gamma\n")
      (insert "Gamma body.\n")
      (insert "** Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Narrow to Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (save-restriction
        (org-narrow-to-subtree)
        ;; Hide subtree
        (org-fold-hide-subtree)
        ;; Edit while hidden
        (end-of-line)
        (insert "\n*** Inserted under Alpha\nInserted body.\n")
        ;; Show all within narrow
        (org-fold-show-all)
        (font-lock-ensure (point-min) (point-max))
        (let ((narrowed-state
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
                '("Alpha" "Beta" "Gamma" "Inserted" "Root" "Sibling"))))
          ;; Widen
          (list narrowed-state
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_cycle_global_local_mixed_font_level_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Global overview
      (org-cycle-overview)
      ;; Local cycle on Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-cycle)
      (org-cycle)
      ;; Global contents
      (org-cycle-contents)
      ;; Local cycle on Beta
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-cycle)
      (org-cycle)
      ;; Global all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("Root" "Root body" "Alpha" "Alpha body" "Beta" "Beta body"
                "Gamma" "Gamma body" "Sibling" "Sibling body")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_show_all_font_face_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "***** DONE Delta\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n*** TODO Inserted under Alpha\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face)
                            (get-text-property (point) 'face))
                      (list needle 'not-found nil nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Delta"
                "Inserted under Alpha" "Sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_level_visibility_after_demote_promote_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* L1\n")
      (insert "** L2\n")
      (insert "*** L3\n")
      (insert "**** L4\n")
      (insert "***** L5\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Demote L3 subtree
      (goto-char (point-min))
      (search-forward "L3")
      (beginning-of-line)
      (org-demote-subtree)
      ;; Promote L4 subtree
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (org-promote-subtree)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (re-search-forward
                       (concat "^\\*+ " needle) nil t)
                      (list needle
                            (length (match-string 1))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("L1" "L2" "L3" "L4" "L5")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_level_visibility_after_todo_toggle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-log-done 'time))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Toggle Alpha to TODO
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-todo "TODO")
      ;; Toggle Beta to DONE
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-todo "DONE")
      ;; Toggle Sibling to CANCELED
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-todo "CANCELED")
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face)
                            (get-text-property (point) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "SIBLING")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (replace-regexp-in-string
               "CLOSED: \\[.*\\]" "CLOSED: [stamp]"
               (buffer-substring-no-properties
                (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_show_all_after_multiple_hidden_edits_font_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n*** TODO Inserted under Alpha\nInserted Alpha body.\n")
      ;; Hide Sibling subtree
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n** DONE New sibling\nNew sibling body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Inserted under Alpha"
                "Sibling" "New sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_reveal_font_level_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Beta subtree
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n**** TODO Inserted under Beta\nInserted body.\n")
      ;; Show subtree
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-fold-show-subtree)
      ;; Capture state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted under Beta" "Sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_indirect_buffer_decouple_font_level_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (require 'org-fold-core)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      ;; Create indirect buffer
      (let ((clone (condition-case nil
                       (clone-indirect-buffer nil nil)
                     (error nil))))
        (if (not clone)
            (list 'clone-failed
                  (buffer-substring-no-properties
                   (point-min) (point-max)))
            (condition-case err
                (with-current-buffer clone
                  (org-fold-core-decouple-indirect-buffer-folds)
                  (goto-char (point-min))
                  (search-forward "Alpha")
                  (beginning-of-line)
                  (org-fold-show-subtree)
                  (let ((clone-state
                         (mapcar
                          (lambda (needle)
                            (save-excursion
                              (goto-char (point-min))
                              (if (search-forward needle nil t)
                                  (list needle
                                        (invisible-p (point))
                                        (org-outline-level))
                                  (list needle 'not-found nil nil))))
                          '("Root" "Alpha" "Beta" "Gamma" "Sibling"))))
                    (kill-buffer clone)
                    (list 'ok clone-state)))
              (error (list 'divergence (car err) (cdr err))))))))))"##,
    );
}

#[test]
fn org_fold_deep_level_cycle_hidden_edit_show_all_font_v3_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO L1\n")
      (insert "** DONE L2\n")
      (insert "*** TODO L3\n")
      (insert "**** WAIT L4\n")
      (insert "***** DONE L5\n")
      (insert "****** TODO L6\n")
      (insert "******* WAIT L7\n")
      (insert "******** DONE L8\n")
      (insert "********* TODO L9\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Aggressive cycle L4
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (dotimes (_ 6)
        (org-cycle))
      ;; Hide L5 subtree, edit
      (goto-char (point-min))
      (search-forward "L5")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n***** TODO Inserted under L5\nInserted L5 body.\n")
      ;; Hide L7 subtree, edit
      (goto-char (point-min))
      (search-forward "L7")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n******* TODO Inserted under L7\nInserted L7 body.\n")
      ;; Global cycles
      (goto-char (point-min))
      (dotimes (_ 6)
        (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("L1" "L2" "L3" "L4" "L5" "L6" "L7" "L8" "L9"
                "Inserted under L5" "Inserted under L7")))
            (merged nil)
            (level-ok t))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) " nil t)
          (let ((stars (length (match-string 1)))
                (level (org-outline-level)))
            (unless (= stars level)
              (setq level-ok nil))))
        (list headings
              (nreverse merged)
              level-ok
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_narrow_widen_cycle_font_level_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* Root\nRoot body.\n")
      (insert "** Alpha\nAlpha body.\n")
      (insert "*** Beta\nBeta body.\n")
      (insert "**** Gamma\nGamma body.\n")
      (insert "** Sibling\nSibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Narrow to Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (save-restriction
        (org-narrow-to-subtree)
        ;; Cycle within narrow
        (org-cycle)
        (org-cycle)
        (let ((narrowed-state
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (if (search-forward needle nil t)
                        (list needle
                              (invisible-p (point))
                              (org-outline-level)
                              (get-text-property (line-beginning-position) 'face))
                        (list needle 'not-found nil nil nil))))
                '("Alpha" "Alpha body" "Beta" "Beta body" "Gamma" "Gamma body" "Root" "Sibling"))))
        ;; Edit within narrow
        (goto-char (point-max))
        (insert "\n*** Inserted in narrow\nInserted body.\n")
        ;; Show all within narrow
        (org-fold-show-all)
        (let ((narrowed-show
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (if (search-forward needle nil t)
                        (list needle
                              (invisible-p (point))
                              (org-outline-level))
                        (list needle 'not-found nil nil))))
                '("Alpha" "Beta" "Gamma" "Inserted" "Root" "Sibling"))))
          ;; Widen and check
          (list narrowed-state
                narrowed-show
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_overview_content_all_local_cycle_font_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Global overview
      (org-cycle-overview)
      (let ((overview-state
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Sibling"))))
        ;; Local cycle on Alpha
        (goto-char (point-min))
        (search-forward "Alpha")
        (beginning-of-line)
        (org-cycle)
        (org-cycle)
        (let ((alpha-cycled
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (if (search-forward needle nil t)
                        (list needle
                              (invisible-p (point))
                              (org-outline-level))
                        (list needle 'not-found nil nil))))
                '("Root" "Alpha" "Beta" "Gamma" "Sibling"))))
          ;; Global contents
          (org-cycle-contents)
          (let ((contents-state
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (if (search-forward needle nil t)
                          (list needle
                                (invisible-p (point))
                                (org-outline-level))
                          (list needle 'not-found nil nil))))
                  '("Root" "Alpha" "Beta" "Gamma" "Sibling"))))
            ;; Show all
            (org-fold-show-all)
            (font-lock-ensure (point-min) (point-max))
            (let ((merged nil))
              (dolist (line (split-string
                             (buffer-substring-no-properties
                              (point-min) (point-max))
                             "\n" t))
                (when (string-match-p "^\\*+ .*\\*+ " line)
                  (push line merged)))
              (list overview-state
                    alpha-cycled
                    contents-state
                    (nreverse merged)
                    (buffer-substring-no-properties
                     (point-min) (point-max))))))))))"##,
    );
}

#[test]
fn org_fold_cut_paste_subtree_reexpand_font_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* A\nbody A\n")
      (insert "** B\nbody B\n")
      (insert "*** C\nbody C\n")
      (insert "* E\nbody E\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Cycle B
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-cycle)
      (org-cycle)
      ;; Show all
      (org-fold-show-all)
      ;; Cut C subtree
      (goto-char (point-min))
      (search-forward "C")
      (beginning-of-line)
      (org-cut-subtree)
      ;; Paste under E
      (goto-char (point-min))
      (search-forward "E")
      (beginning-of-line)
      (org-paste-subtree 2)
      ;; Cycle overview
      (org-cycle-overview)
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("A" "body A" "B" "body B" "C" "body C" "E" "body E")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_startup_visibility_property_cycle_font_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert ":PROPERTIES:\n:VISIBILITY: children\n:END:\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Apply startup visibility
      (org-set-startup-visibility)
      ;; Capture state
      (let ((after-startup
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil))))
              '("Root" "Root body" "Alpha" "Alpha body" "Beta" "Beta body"
                "Gamma" "Gamma body" "Sibling" "Sibling body"))))
        ;; Cycle global
        (org-cycle-global)
        (let ((after-global
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (if (search-forward needle nil t)
                        (list needle
                              (invisible-p (point))
                              (org-outline-level))
                        (list needle 'not-found nil nil))))
                '("Root" "Alpha" "Beta" "Gamma" "Sibling"))))
          ;; Show all
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list after-startup
                  after-global
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))))"##,
    );
}

#[test]
fn org_fold_hide_sublevels_show_context_cycle_font_deep_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (require 'org-cycle)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "***** DONE Delta\n")
      (insert "****** TODO Epsilon\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      ;; Reveal Delta with isearch
      (goto-char (point-min))
      (search-forward "Delta body")
      (org-fold-show-context 'isearch)
      ;; Now cycle Gamma
      (goto-char (point-min))
      (search-forward "Gamma")
      (beginning-of-line)
      (org-cycle)
      (org-cycle)
      ;; Capture state
      (let ((state
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Delta" "Epsilon" "Sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list state
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_sublevels_reveal_context_font_state_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* Root\nRoot body.\n")
      (insert "** Alpha\nAlpha body.\n")
      (insert "*** Beta\nBeta body.\n")
      (insert "**** Gamma\nGamma body.\n")
      (insert "***** Delta\nDelta body.\n")
      (insert "****** Epsilon\nEpsilon body.\n")
      (insert "** Sibling\nSibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      (let ((after-hide
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (search-forward needle)
                  (list needle
                        (invisible-p (point))
                        (org-outline-level)
                        (get-text-property (line-beginning-position) 'face))))
              '("Root" "Alpha" "Beta" "Gamma" "Delta" "Epsilon" "Sibling"))))
        ;; Reveal Epsilon with isearch context
        (goto-char (point-min))
        (search-forward "Epsilon body")
        (org-fold-show-context 'isearch)
        (let ((after-reveal
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (search-forward needle)
                    (list needle
                          (invisible-p (point))
                          (org-outline-level))))
                '("Root" "Alpha" "Beta" "Gamma" "Delta" "Epsilon" "Sibling"))))
          ;; Merged check
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list after-hide
                  after-reveal
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_subtree_hide_edit_show_all_font_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Beta subtree
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n**** TODO Inserted under Beta\nInserted body.\n")
      ;; Hide Sibling subtree
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n** DONE Inserted after Sibling\nInserted sibling body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Inserted under Beta"
                "Sibling" "Inserted after Sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_multiple_hidden_edits_global_cycle_font_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "***** DONE Delta\n")
      (insert "Delta body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n*** TODO Inserted under Alpha\nInserted Alpha body.\n")
      ;; Hide Sibling subtree
      (goto-char (point-min))
      (search-forward "Sibling")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Edit while hidden
      (end-of-line)
      (insert "\n** DONE Inserted after Sibling\nInserted Sibling body.\n")
      ;; Global cycles
      (goto-char (point-min))
      (dotimes (_ 6)
        (org-cycle-global))
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("Root" "Alpha" "Beta" "Gamma" "Delta" "Inserted under Alpha"
                "Sibling" "Inserted after Sibling")))
            (merged nil)
            (level-ok t))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) " nil t)
          (let ((stars (length (match-string 1)))
                (level (org-outline-level)))
            (unless (= stars level)
              (setq level-ok nil))))
        (list headings
              (nreverse merged)
              level-ok
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_visibility_after_promote_demote_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* L1\n")
      (insert "** L2\n")
      (insert "*** L3\n")
      (insert "**** L4\n")
      (insert "***** L5\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Promote L4 to L3
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (org-promote-subtree)
      ;; Demote L2 to L3
      (goto-char (point-min))
      (search-forward "L2")
      (beginning-of-line)
      (org-demote-subtree)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (length (match-string 1))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face)
                            (get-text-property (point) 'face))
                      (list needle 'not-found nil nil nil))))
              '("L1" "L2" "L3" "L4" "L5")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list headings
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_deep_heading_cycle_hidden_edit_font_regression_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO L1\n")
      (insert "** DONE L2\n")
      (insert "*** TODO L3\n")
      (insert "**** WAIT L4\n")
      (insert "***** DONE L5\n")
      (insert "****** TODO L6\n")
      (insert "******* WAIT L7\n")
      (insert "******** DONE L8\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Cycle L4 multiple times
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (dotimes (_ 5)
        (org-cycle))
      ;; Hide L5 subtree, edit while hidden
      (goto-char (point-min))
      (search-forward "L5")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (end-of-line)
      (insert "\n***** TODO Inserted under L5\nInserted body.\n")
      ;; Show all
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let ((headings
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (if (search-forward needle nil t)
                      (list needle
                            (line-number-at-pos)
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))
                      (list needle 'not-found nil nil nil))))
              '("L1" "L2" "L3" "L4" "L5" "L6" "L7" "L8" "Inserted")))
            (merged nil)
            (level-ok t))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) " nil t)
          (let ((stars (length (match-string 1)))
                (level (org-outline-level)))
            (unless (= stars level)
              (setq level-ok nil))))
        (list headings
              (nreverse merged)
              level-ok
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_global_cycle_overview_content_all_font_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Capture global cycle states
      (let ((heading-state
             (lambda ()
               (font-lock-ensure (point-min) (point-max))
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (search-forward needle)
                    (list needle
                          (invisible-p (point))
                          (org-outline-level)
                          (get-text-property (line-beginning-position) 'face)
                          (get-text-property (point) 'face))))
                '("Root" "Root body" "Alpha" "Alpha body" "Beta" "Beta body" "Gamma" "Gamma body" "Sibling" "Sibling body"))))
            states)
        ;; Cycle through all global states
        (dotimes (_ 6)
          (org-cycle-global)
          (push (list org-cycle-global-status (funcall heading-state)) states))
        ;; Show all
        (org-fold-show-all)
        (font-lock-ensure (point-min) (point-max))
        (let ((final-state (funcall heading-state))
              (merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list (nreverse states)
                final-state
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_narrow_subtree_cycle_edit_widen_font_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* Root\n")
      (insert "Root body.\n")
      (insert "** Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** Beta\n")
      (insert "Beta body.\n")
      (insert "**** Gamma\n")
      (insert "Gamma body.\n")
      (insert "** Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Narrow to Alpha subtree
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (save-restriction
        (org-narrow-to-subtree)
        ;; Cycle within narrow
        (org-cycle)
        (org-cycle)
        (let ((narrowed-cycle
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
                '("Alpha" "Alpha body" "Beta" "Beta body" "Gamma" "Gamma body" "Root" "Sibling"))))
          ;; Edit within narrow
          (goto-char (point-max))
          (insert "\n*** Inserted in narrow\nInserted body.\n")
          (let ((after-edit
                 (buffer-substring-no-properties
                  (point-min) (point-max))))
            ;; Show all within narrow
            (org-fold-show-all)
            (let ((narrowed-show
                   (mapcar
                    (lambda (needle)
                      (save-excursion
                        (goto-char (point-min))
                        (if (search-forward needle nil t)
                            (list needle
                                  (invisible-p (point))
                                  (org-outline-level))
                            (list needle 'not-found nil nil))))
                    '("Alpha" "Beta" "Gamma" "Inserted" "Root" "Sibling"))))
              ;; Widen
              (list narrowed-cycle
                    after-edit
                    narrowed-show)))))
      ;; After widen
      (font-lock-ensure (point-min) (point-max))
      (let ((after-widen
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
              '("Root" "Alpha" "Beta" "Gamma" "Inserted" "Sibling")))
            (merged nil))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list after-widen
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_level_visibility_deep_state_v2_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Project :root:\n")
      (insert "Project body.\n")
      (insert "** DONE Alpha :work:\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "***** DONE Delta\n")
      (insert "Delta body.\n")
      (insert "****** TODO Epsilon\n")
      (insert "Epsilon body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Deep state capture function
      (let ((heading-state
             (lambda ()
               (font-lock-ensure (point-min) (point-max))
               (let (out)
                 (goto-char (point-min))
                 (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
                   (let ((beg (line-beginning-position)))
                     (push (list (match-string 2)
                                 (length (match-string 1))
                                 (org-outline-level)
                                 (invisible-p beg)
                                 (get-text-property beg 'face)
                                 (get-text-property (match-beginning 2) 'face))
                           out)))
                 (nreverse out)))))
        ;; Hide to level 1
        (org-fold-hide-sublevels 1)
        (let ((after-hide-1 (funcall heading-state)))
          ;; Show Beta subtree
          (goto-char (point-min))
          (search-forward "Beta")
          (beginning-of-line)
          (org-fold-show-subtree)
          (let ((after-show-beta (funcall heading-state)))
            ;; Hide Gamma subtree
            (goto-char (point-min))
            (search-forward "Gamma")
            (beginning-of-line)
            (org-fold-hide-subtree)
            (let ((after-hide-gamma (funcall heading-state)))
              ;; Edit while hidden
              (end-of-line)
              (insert "\n**** Inserted under hidden Gamma\nInserted body.\n")
              ;; Show all
              (org-fold-show-all)
              (font-lock-ensure (point-min) (point-max))
              (let ((after-show-all (funcall heading-state))
                    (merged nil))
                (dolist (line (split-string
                               (buffer-substring-no-properties
                                (point-min) (point-max))
                               "\n" t))
                  (when (string-match-p "^\\*+ .*\\*+ " line)
                    (push line merged)))
                (list after-hide-1
                      after-show-beta
                      after-hide-gamma
                      after-show-all
                      (nreverse merged)
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_fold_local_cycle_global_cycle_show_all_font_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** DONE Alpha\n")
      (insert "Alpha body.\n")
      (insert "*** TODO Beta\n")
      (insert "Beta body.\n")
      (insert "**** WAIT Gamma\n")
      (insert "Gamma body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Local cycle on Alpha
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (org-cycle)
      (org-cycle)
      (org-cycle)
      (let ((after-local
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (search-forward needle)
                  (list needle
                        (invisible-p (point))
                        (org-outline-level)
                        (get-text-property (line-beginning-position) 'face))))
              '("Root" "Root body" "Alpha" "Alpha body" "Beta" "Beta body" "Gamma" "Gamma body" "Sibling" "Sibling body"))))
        ;; Global cycle
        (goto-char (point-min))
        (org-cycle-global)
        (org-cycle-global)
        (let ((after-global
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (search-forward needle)
                    (list needle
                          (invisible-p (point))
                          (org-outline-level))))
                '("Root" "Alpha" "Beta" "Gamma" "Sibling"))))
          ;; Show all
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((after-show
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (search-forward needle)
                      (list needle
                            (invisible-p (point))
                            (org-outline-level)
                            (get-text-property (line-beginning-position) 'face))))
                  '("Root" "Alpha" "Beta" "Gamma" "Sibling")))
                (merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list after-local
                  after-global
                  after-show
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))))"##,
    );
}

#[test]
fn org_fold_hide_show_subtree_font_face_level_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* L1\nL1 body\n")
      (insert "** L2\nL2 body\n")
      (insert "*** L3\nL3 body\n")
      (insert "**** L4\nL4 body\n")
      (insert "***** L5\nL5 body\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide L2 subtree
      (goto-char (point-min))
      (search-forward "L2")
      (beginning-of-line)
      (org-fold-hide-subtree)
      ;; Check hidden state
      (let ((hidden-state
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (search-forward needle)
                  (list needle
                        (line-number-at-pos)
                        (invisible-p (point))
                        (org-outline-level)
                        (get-text-property (line-beginning-position) 'face))))
              '("L1" "L2" "L3" "L4" "L5"))))
        ;; Show subtree
        (goto-char (point-min))
        (search-forward "L2")
        (beginning-of-line)
        (org-fold-show-subtree)
        ;; Check shown state
        (let ((shown-state
               (mapcar
                (lambda (needle)
                  (save-excursion
                    (goto-char (point-min))
                    (search-forward needle)
                    (list needle
                          (invisible-p (point))
                          (org-outline-level)
                          (get-text-property (line-beginning-position) 'face))))
                '("L1" "L2" "L3" "L4" "L5"))))
          ;; Merged check
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list hidden-state
                  shown-state
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_repeated_global_cycle_font_level_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Alpha\n")
      (insert "*** TODO Beta\n")
      (insert "**** WAIT Gamma\n")
      (insert "***** DONE Delta\n")
      (insert "****** TODO Epsilon\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Repeated global cycles
      (dotimes (_ 8)
        (org-cycle-global))
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let (headings merged)
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
          (let ((beg (line-beginning-position)))
            (push (list (match-string 2)
                        (length (match-string 1))
                        (org-outline-level)
                        (invisible-p beg)
                        (get-text-property beg 'face)
                        (get-text-property (match-beginning 2) 'face))
                  headings)))
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (let ((level-consistent t))
          (goto-char (point-min))
          (while (re-search-forward "^\\(\\*+\\) " nil t)
            (let ((stars (length (match-string 1)))
                  (level (org-outline-level)))
              (unless (= stars level)
                (setq level-consistent nil))))
          (list (nreverse headings)
                (nreverse merged)
                level-consistent
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_cycle_cut_paste_subtree_expand_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert "Alpha body.\n")
    (insert "** Beta\n")
    (insert "Beta body.\n")
    (insert "*** Gamma\n")
    (insert "Gamma body.\n")
    (insert "* Delta\n")
    (insert "Delta body.\n")
    (font-lock-ensure (point-min) (point-max))
    ;; Cycle Beta
    (goto-char (point-min))
    (search-forward "Beta")
    (beginning-of-line)
    (org-cycle)
    (org-cycle)
    ;; Show all
    (org-fold-show-all)
    ;; Cut Gamma subtree
    (goto-char (point-min))
    (search-forward "Gamma")
    (beginning-of-line)
    (org-cut-subtree)
    ;; Paste under Delta
    (goto-char (point-min))
    (search-forward "Delta")
    (beginning-of-line)
    (org-paste-subtree 2)
    ;; Cycle overview
    (org-cycle-overview)
    (org-fold-show-all)
    (font-lock-ensure (point-min) (point-max))
    ;; Check state
    (let ((headings nil)
          (merged nil))
      (goto-char (point-min))
      (while (re-search-forward "^\\(\\*+\\) \\(.*\\)$" nil t)
        (let ((beg (line-beginning-position)))
          (push (list (match-string 2)
                      (length (match-string 1))
                      (org-outline-level)
                      (invisible-p beg)
                      (get-text-property beg 'face))
                headings)))
      (dolist (line (split-string
                     (buffer-substring-no-properties
                      (point-min) (point-max))
                     "\n" t))
        (when (string-match-p "^\\*+ .*\\*+ " line)
          (push line merged)))
      (list (nreverse headings)
            (nreverse merged)
            (buffer-substring-no-properties
             (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_font_lock_deep_heading_face_after_demote_promote_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* L1\n** L2\n*** L3\n**** L4\n***** L5\n****** L6\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Demote L4 subtree
      (goto-char (point-min))
      (search-forward "L4")
      (beginning-of-line)
      (org-demote-subtree)
      ;; Promote L5 subtree
      (goto-char (point-min))
      (search-forward "L5")
      (beginning-of-line)
      (org-promote-subtree)
      (font-lock-ensure (point-min) (point-max))
      ;; Capture face state
      (let (headings)
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) \\(L[0-9]+\\)" nil t)
          (let ((beg (line-beginning-position)))
            (push (list (match-string 2)
                        (length (match-string 1))
                        (org-outline-level)
                        (get-text-property beg 'face)
                        (get-text-property (match-beginning 2) 'face))
                  headings)))
        ;; Check merged
        (let ((merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list (nreverse headings)
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_subtree_show_all_font_level_state_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "** DONE Child\n")
      (insert "*** TODO Grand\n")
      (insert "**** WAIT Fourth\n")
      (insert "***** DONE Fifth\n")
      (insert "** NEXT Sibling\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide subtree, then show-all
      (goto-char (point-min))
      (search-forward "Child")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (org-fold-show-all)
      (font-lock-ensure (point-min) (point-max))
      ;; Check state
      (let (headings merged)
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
          (let ((beg (line-beginning-position)))
            (push (list (match-string 2)
                        (length (match-string 1))
                        (org-outline-level)
                        (invisible-p beg)
                        (get-text-property beg 'face)
                        (get-text-property (match-beginning 2) 'face))
                  headings)))
        ;; Check merged
        (dolist (line (split-string
                       (buffer-substring-no-properties
                        (point-min) (point-max))
                       "\n" t))
          (when (string-match-p "^\\*+ .*\\*+ " line)
            (push line merged)))
        (list (nreverse headings)
              (nreverse merged)
              (buffer-substring-no-properties
               (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_hide_sublevels_show_context_font_level_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fold-show-context-detail '((default . lineage)
                                          (isearch . lineage))))
      (org-mode)
      (insert "* L1\nL1 body\n")
      (insert "** L2\nL2 body\n")
      (insert "*** L3\nL3 body\n")
      (insert "**** L4\nL4 body\n")
      (insert "***** L5\nL5 body\n")
      (insert "****** L6\nL6 body\n")
      (insert "** L2b\nL2b body\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Hide to level 1
      (org-fold-hide-sublevels 1)
      (let ((after-hide (mapcar
                         (lambda (needle)
                           (save-excursion
                             (goto-char (point-min))
                             (search-forward needle)
                             (list needle
                                   (line-number-at-pos)
                                   (invisible-p (point))
                                   (org-outline-level)
                                   (get-text-property (line-beginning-position) 'face))))
                         '("L1" "L2" "L3" "L4" "L5" "L6" "L2b"))))
        ;; Reveal L5 with isearch
        (goto-char (point-min))
        (search-forward "L5 body")
        (org-fold-show-context 'isearch)
        (let ((after-reveal (mapcar
                             (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (search-forward needle)
                                 (list needle
                                       (invisible-p (point))
                                       (org-outline-level))))
                             '("L1" "L2" "L3" "L4" "L5" "L6" "L2b"))))
          ;; Check merged headings
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (list after-hide
                  after-reveal
                  (nreverse merged)
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}

#[test]
fn org_fold_cycle_hidden_edit_global_font_state_deep_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Root :root:\n")
      (insert "Root body.\n")
      (insert "** DONE Child :work:\n")
      (insert "Child body.\n")
      (insert "*** TODO Grand\n")
      (insert "Grand body.\n")
      (insert "**** WAIT Fourth\n")
      (insert "Fourth body.\n")
      (insert "***** DONE Fifth\n")
      (insert "Fifth body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Per-heading state capture
      (let ((heading-state
             (lambda ()
               (font-lock-ensure (point-min) (point-max))
               (let (out)
                 (goto-char (point-min))
                 (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
                   (let ((beg (line-beginning-position)))
                     (push (list (match-string 2)
                                 (length (match-string 1))
                                 (org-outline-level)
                                 (invisible-p beg)
                                 (get-text-property beg 'face)
                                 (get-text-property (match-beginning 2) 'face))
                           out)))
                 (nreverse out)))))
        ;; Cycle on Grand
        (goto-char (point-min))
        (search-forward "Grand")
        (beginning-of-line)
        (let ((after-cycle-1 (funcall heading-state)))
          (org-cycle)
          (let ((after-cycle-2 (funcall heading-state)))
            (org-cycle)
            (let ((after-cycle-3 (funcall heading-state)))
              ;; Hide Fifth subtree, edit
              (goto-char (point-min))
              (search-forward "Fifth")
              (beginning-of-line)
              (org-fold-hide-subtree)
              (end-of-line)
              (insert "\n***** TODO Inserted under hidden Fifth\nInserted body.\n")
              ;; Global cycles
              (goto-char (point-min))
              (dotimes (_ 5) (org-cycle-global))
              (org-fold-show-all)
              (font-lock-ensure (point-min) (point-max))
              ;; Check for merged headings
              (let ((merged nil))
                (dolist (line (split-string
                               (buffer-substring-no-properties
                                (point-min) (point-max))
                               "\n" t))
                  (when (string-match-p "^\\*+ .*\\*+ " line)
                    (push line merged)))
                (list after-cycle-1
                      after-cycle-2
                      after-cycle-3
                      (nreverse merged)
                      (search-forward "Inserted under hidden Fifth" nil t)
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))))"##,
    );
}

#[test]
fn org_font_lock_todo_keyword_face_level_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-todo-keywords '((sequence "TODO" "NEXT" "WAIT" "|" "DONE" "CANCELED")))
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-fontify-whole-heading-line t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* TODO Alpha\n")
      (insert "** NEXT Beta\n")
      (insert "*** WAIT Gamma\n")
      (insert "**** DONE Delta\n")
      (insert "***** CANCELED Epsilon\n")
      (insert "****** TODO Zeta\n")
      (insert "******* NEXT Eta\n")
      (insert "******** DONE Theta\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Capture face state at each heading
      (let (faces)
        (goto-char (point-min))
        (while (re-search-forward "^\\(\\*+\\) +\\([A-Z]+\\)? ?\\(.*\\)$" nil t)
          (let ((beg (line-beginning-position))
                (stars (length (match-string 1)))
                (todo (match-string 2))
                (text (substring-no-properties (match-string 3))))
            (push (list text
                        stars
                        todo
                        (org-outline-level)
                        (get-text-property beg 'face)
                        (get-text-property (match-beginning 1) 'face)
                        (and (match-beginning 2)
                             (get-text-property (match-beginning 2) 'face))
                        (get-text-property (match-beginning 3) 'face))
                  faces)))
        ;; Check for merged heading lines
        (let ((merged nil))
          (dolist (line (split-string
                         (buffer-substring-no-properties
                          (point-min) (point-max))
                         "\n" t))
            (when (string-match-p "^\\*+ .*\\*+ " line)
              (push line merged)))
          (list (nreverse faces)
                (nreverse merged)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_font_face_level_visibility_deep_state_capture_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-global-at-bob t)
          (org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-hide-leading-stars nil))
      (org-mode)
      (insert "* TODO Root :root:\n")
      (insert "Root body.\n")
      (insert "** DONE Child :work:\n")
      (insert "Child body.\n")
      (insert "*** TODO Grand\n")
      (insert "Grand body.\n")
      (insert "**** WAIT Fourth\n")
      (insert "Fourth body.\n")
      (insert "***** DONE Fifth\n")
      (insert "Fifth body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((deep-state
             (lambda ()
               (font-lock-ensure (point-min) (point-max))
               (let (out)
                 (goto-char (point-min))
                 (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
                   (let ((beg (line-beginning-position))
                         (stars (length (match-string 1)))
                         (heading (substring-no-properties (match-string 2))))
                     (push (list heading
                                 stars
                                 (org-outline-level)
                                 (invisible-p beg)
                                 (get-text-property beg 'face)
                                 (get-text-property (match-beginning 2) 'face)
                                 (get-text-property beg 'invisible)
                                 (org-fold-folded-p beg 'headline))
                           out)))
                 (nreverse out)))))
        ;; Initial state
        (let ((initial (funcall deep-state)))
          ;; Hide all to level 1
          (org-fold-hide-sublevels 1)
          (let ((after-hide-1 (funcall deep-state)))
            ;; Show children of Root
            (goto-char (point-min))
            (search-forward "Root")
            (beginning-of-line)
            (org-cycle)
            (let ((after-root-cycle (funcall deep-state)))
              ;; Show subtree of Child
              (goto-char (point-min))
              (search-forward "Child")
              (beginning-of-line)
              (org-fold-show-subtree)
              (let ((after-child-show (funcall deep-state)))
                ;; Hide subtree of Fourth
                (goto-char (point-min))
                (search-forward "Fourth")
                (beginning-of-line)
                (org-fold-hide-subtree)
                (let ((after-fourth-hide (funcall deep-state)))
                  ;; Global cycle
                  (goto-char (point-min))
                  (dotimes (_ 3) (org-cycle-global))
                  (let ((after-global (funcall deep-state)))
                    ;; Show all
                    (org-fold-show-all)
                    (let ((after-show-all (funcall deep-state)))
                      ;; Check for merged headings
                      (let ((merged nil))
                        (dolist (line (split-string
                                       (buffer-substring-no-properties
                                        (point-min) (point-max))
                                       "\n" t))
                          (when (string-match-p "^\\*+ .*\\*+ " line)
                            (push line merged)))
                        (list initial
                              after-hide-1
                              after-root-cycle
                              after-child-show
                              after-fourth-hide
                              after-global
                              after-show-all
                              (nreverse merged)
                              (buffer-substring-no-properties
                               (point-min) (point-max)))))))))))))))"##,
    );
}

#[test]
fn org_repeated_deep_cycle_edit_fontify_no_merge_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-cycle-separator-lines 1))
      (org-mode)
      (insert "* Root\n")
      (insert "** Project A\nBody A\n")
      (insert "*** Area A1\nBody A1\n")
      (insert "**** TODO Level 4 target :deep:\nBody L4\n")
      (insert "***** NEXT Level 5 child\nBody L5\n")
      (insert "****** WAIT Level 6 child\nBody L6\n")
      (insert "******* DONE Level 7 child\nBody L7\n")
      (insert "******** TODO Level 8 child\nBody L8\n")
      (insert "** Project B\nBody B\n")
      (insert "*** Area B1\nBody B1\n")
      (insert "**** TODO B Level 4 :other:\nBody B4\n")
      (let ((snapshot
             (lambda (label)
               (list label
                     (mapcar
                      (lambda (needle)
                        (let ((pos (save-excursion
                                     (goto-char (point-min))
                                     (search-forward needle)
                                     (point))))
                          (list needle
                                (not (null (org-invisible-p pos)))
                                (line-number-at-pos pos))))
                      '("Level 4 target" "Body L4" "Level 5 child" "Body L5"
                        "Level 6 child" "Body L6" "Level 7 child" "Body L7"
                        "Level 8 child" "Body L8" "Project B" "Body B4"))
                     (split-string
                      (buffer-substring-no-properties (point-min) (point-max))
                      "\n" t)))))
        (let (states faces)
          (goto-char (point-min))
          (search-forward "Level 4 target")
          (beginning-of-line)
          (dotimes (_ 7)
            (org-cycle)
            (push (funcall snapshot 'local-l4) states))
          (goto-char (point-min))
          (search-forward "Level 6 child")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (push (funcall snapshot 'hide-l6) states)
          (end-of-line)
          (insert "\nInserted while L6 subtree hidden")
          (push (funcall snapshot 'hidden-edit) states)
          (org-fold-show-subtree)
          (push (funcall snapshot 'show-l6) states)
          (dotimes (_ 4)
            (org-cycle-global)
            (push (funcall snapshot 'global) states))
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (goto-char (point-min))
          (while (re-search-forward
                  "^\\(\\*\\{4,8\\}\\) \\([A-Z]+\\)? ?\\([^:\n]+\\)\\(?: \\(:[[:alnum:]_@#%:]+:\\)\\)?"
                  nil t)
            (push (list (match-string 1)
                        (match-string 2)
                        (substring-no-properties (match-string 3))
                        (match-string 4)
                        (org-outline-level)
                        (get-text-property (match-beginning 1) 'face)
                        (and (match-beginning 2)
                             (get-text-property (match-beginning 2) 'face))
                        (get-text-property (match-beginning 3) 'face)
                        (get-text-property (line-beginning-position)
                                           'font-lock-fontified))
                  faces))
          (list (nreverse states)
                (nreverse faces)
                (count-matches "^\\*+ " (point-min) (point-max))
                (count-matches "^Inserted while L6 subtree hidden$"
                               (point-min) (point-max))
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_core_mixed_regions_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* A\n")
    (insert ":LOGBOOK:\nclock line\n:END:\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
    (insert "** B\nbody B\n*** C\nbody C\n")
    (insert "* D\nbody D\n")
    (let ((offset-region
           (lambda (region)
             (and region
                  (cons (- (car region) (point-min))
                        (- (cdr region) (point-min))))))
          (probe
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (search-forward needle)
               (list needle
                     (invisible-p (point))
                     (get-text-property (point) 'invisible)
                     (funcall offset-region
                              (org-fold-get-region-at-point 'drawer (point)))
                     (funcall offset-region
                              (org-fold-get-region-at-point 'block (point)))
                     (funcall offset-region
                              (org-fold-get-region-at-point 'outline
                                                            (point))))))))
      (org-fold-hide-drawer-all)
      (org-fold-hide-block-all)
      (goto-char (point-min))
      (search-forward "B")
      (beginning-of-line)
      (org-fold-hide-subtree)
      (let ((hidden (mapcar probe
                            '("clock line" "(+ 1 2)" "body B" "C"
                              "body C" "D" "body D"))))
        (org-fold-show-subtree)
        (org-fold-show-all '(blocks drawers))
        (let ((shown (mapcar probe
                             '("clock line" "(+ 1 2)" "body B" "C"
                               "body C" "D" "body D"))))
          (list hidden
                shown
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_fold_reveal_context_after_hidden_search_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* Root\nroot body\n")
    (insert "** Alpha\nalpha body\n")
    (insert "*** Beta\nbeta body\n")
    (insert "**** Gamma\nneedle body\n")
    (insert "** Sibling\nsibling body\n")
    (let ((visible
           (lambda ()
             (mapcar
              (lambda (needle)
                (list needle
                      (invisible-p
                       (save-excursion
                         (goto-char (point-min))
                         (search-forward needle)
                         (point)))))
              '("Root" "root body" "Alpha" "alpha body" "Beta"
                "beta body" "Gamma" "needle body" "Sibling"
                "sibling body")))))
      (org-fold-hide-sublevels 1)
      (let ((overview (funcall visible)))
        (goto-char (point-min))
        (search-forward "needle body")
        (org-fold-show-context 'isearch)
        (let ((after-context (funcall visible)))
          (org-fold-hide-sublevels 1)
          (goto-char (point-min))
          (search-forward "needle body")
          (org-fold-reveal '(4))
          (let ((after-reveal (funcall visible)))
            (org-fold-show-all)
            (list overview
                  after-context
                  after-reveal
                  (funcall visible)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_get_level_face_options_matrix_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (dotimes (level 10)
      (insert (make-string (1+ level) ?*) " L" (number-to-string (1+ level)) "\n"))
    (let (out)
      (dolist (settings
               '((nil nil nil nil)
                 (t nil nil nil)
                 (nil t nil nil)
                 (nil t t nil)
                 (nil nil nil t)
                 (t t t t)))
        (let ((org-odd-levels-only (nth 0 settings))
              (org-cycle-level-faces (nth 1 settings))
              (org-hide-leading-stars (nth 2 settings))
              (org-level-color-stars-only (nth 3 settings)))
          (goto-char (point-min))
          (while (re-search-forward "^\\(\\*+\\) \\(L[0-9]+\\)" nil t)
            (push (list settings
                        (match-string 1)
                        (substring-no-properties (match-string 2))
                        (org-outline-level)
                        (org-get-level-face 1)
                        (org-get-level-face 2)
                        (org-get-level-face 3))
                  out))))
      (nreverse out))))"##,
    );
}

#[test]
fn org_fontify_like_org_mode_deep_markup_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let* ((org-link-descriptive t)
         (input (concat "* TODO L1 :tag:\n"
                        "**** WAIT L4 [[https://example.org][Example]]\n"
                        "***** DONE L5 /italic/ =code= *bold*\n"
                        "[[file:plain.txt]] <<target>> {{{macro(arg)}}}\n"))
         (fontified (org-fontify-like-in-org-mode input t))
         (probe (lambda (needle)
                  (let ((pos (string-match (regexp-quote needle) fontified)))
                    (and pos
                         (list needle
                               pos
                               (substring-no-properties
                                fontified pos (+ pos (length needle)))
                               (get-text-property pos 'face fontified)
                               (get-text-property pos 'mouse-face fontified)
                               (get-text-property pos 'help-echo fontified)
                               (get-text-property pos 'htmlize-link fontified)
                               (get-text-property pos 'org-emphasis fontified)
                               (get-text-property pos 'font-lock-multiline fontified)
                               (get-text-property pos 'font-lock-fontified fontified)
                               (keymapp (get-text-property pos 'keymap fontified))))))))
    (list (substring-no-properties fontified)
          (mapcar probe
                  '("TODO" "L1" "WAIT" "L4" "Example" "DONE" "L5"
                    "italic" "code" "bold" "target" "{{{macro(arg)}}}")))))"##,
    );
}

#[test]
fn org_indent_deep_cycle_prefix_properties_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-indent)
  (with-temp-buffer
    (let ((org-startup-indented t)
          (org-hide-leading-stars t)
          (org-odd-levels-only nil))
      (org-mode)
      (org-indent-mode 1)
      (insert "* L1\nbody 1\n")
      (insert "** L2\nbody 2\n")
      (insert "*** L3\nbody 3\n")
      (insert "**** L4\nbody 4\n")
      (insert "***** L5\nbody 5\n")
      (insert "****** L6\nbody 6\n")
      (dotimes (_ 3) (org-cycle-global))
      (font-lock-ensure (point-min) (point-max))
      (let ((probe
             (lambda (needle)
               (save-excursion
                 (goto-char (point-min))
                 (search-forward needle)
                 (let ((pos (line-beginning-position)))
                   (list needle
                         (org-outline-level)
                         (get-text-property pos 'line-prefix)
                         (get-text-property pos 'wrap-prefix)
                         (get-text-property pos 'face)
                         (get-text-property (point) 'invisible)))))))
        (list (mapcar probe
                      '("L1" "body 1" "L2" "body 2" "L3" "body 3"
                        "L4" "body 4" "L5" "body 5" "L6" "body 6"))
              (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_cycle_plain_list_drawer_block_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-include-plain-lists 'integrate)
          (org-cycle-hide-drawer-startup t)
          (org-cycle-hide-block-startup t))
      (org-mode)
      (insert "* Project\n")
      (insert ":PROPERTIES:\n:CATEGORY: fold\n:END:\n")
      (insert "Intro paragraph\n")
      (insert "- [ ] Item one\n")
      (insert "  - [X] Child one\n")
      (insert "    text child one\n")
      (insert "  - [ ] Child two\n")
      (insert "- [ ] Item two\n")
      (insert "#+begin_src emacs-lisp\n(message \"hidden\")\n#+end_src\n")
      (insert "** Deep\nDeep body\n*** Deeper\nDeeper body\n")
      (insert "* Next\nNext body\n")
      (let ((snapshot
             (lambda (label)
               (list label
                     (mapcar
                      (lambda (needle)
                        (list needle
                              (invisible-p
                               (save-excursion
                                 (goto-char (point-min))
                                 (search-forward needle)
                                 (point)))))
                      '("Project" ":CATEGORY:" "Intro paragraph" "Item one"
                        "Child one" "text child one" "Child two" "Item two"
                        "(message" "Deep" "Deep body" "Deeper"
                        "Deeper body" "Next" "Next body"))
                     (count-matches "^\\*+ " (point-min) (point-max))
                     (split-string
                      (buffer-substring-no-properties
                       (point-min) (point-max))
                      "\n" t)))))
            states)
        (org-cycle-set-startup-visibility)
        (push (funcall snapshot 'startup) states)
        (goto-char (point-min))
        (search-forward "Item one")
        (beginning-of-line)
        (dotimes (_ 3)
          (org-cycle)
          (push (funcall snapshot 'list-cycle) states))
        (goto-char (point-min))
        (search-forward "Project")
        (beginning-of-line)
        (dotimes (_ 3)
          (org-cycle)
          (push (funcall snapshot 'headline-cycle) states))
        (org-fold-hide-drawer-all)
        (org-fold-hide-block-all)
        (push (funcall snapshot 'drawer-block-hidden) states)
        (goto-char (point-min))
        (search-forward "Deeper body")
        (org-fold-show-context 'isearch)
        (push (funcall snapshot 'context) states)
        (dotimes (_ 4)
          (org-cycle-global)
          (push (funcall snapshot 'global) states))
        (org-fold-show-all)
        (push (funcall snapshot 'all) states)
        (list (nreverse states)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_deep_visibility_property_cycle_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-max-level 5)
          (org-cycle-global-at-bob t)
          (org-cycle-hide-drawer-startup t)
          (org-fontify-whole-heading-line t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "#+STARTUP: overview\n")
      (insert "* Root\n")
      (insert ":PROPERTIES:\n:VISIBILITY: children\n:END:\n")
      (insert "root body\n")
      (insert "** Alpha\nalpha body\n")
      (insert "*** A1\nA1 body\n")
      (insert "**** A1a\nA1a body\n")
      (insert "***** A1a-i\nA1a-i body\n")
      (insert "****** A1a-i-x\nA1a-i-x body\n")
      (insert "** Beta\nbeta body\n")
      (insert "*** B1\nB1 body\n")
      (insert "**** B1a\nB1a body\n")
      (insert "***** B1a-i\nB1a-i body\n")
      (insert "* Tail\nTail body\n")
      (let ((snapshot
             (lambda (label)
               (font-lock-ensure (point-min) (point-max))
               (list label
                     org-cycle-global-status
                     org-cycle-subtree-status
                     (mapcar
                      (lambda (needle)
                        (let ((pos (save-excursion
                                     (goto-char (point-min))
                                     (search-forward needle)
                                     (point))))
                          (list needle
                                (invisible-p pos)
                                (get-text-property
                                 (line-beginning-position) 'face))))
                      '("Root" ":VISIBILITY:" "root body" "Alpha"
                        "alpha body" "A1" "A1 body" "A1a" "A1a body"
                        "A1a-i" "A1a-i body" "A1a-i-x"
                        "A1a-i-x body" "Beta" "beta body" "B1"
                        "B1 body" "B1a-i body" "Tail" "Tail body"))
                     (count-matches "^\\*+ " (point-min) (point-max))
                     (split-string
                      (buffer-substring-no-properties
                       (point-min) (point-max))
                      "\n" t)))))
            states)
        (org-cycle-set-startup-visibility)
        (push (funcall snapshot 'startup) states)
        (goto-char (point-min))
        (search-forward "A1a")
        (beginning-of-line)
        (dotimes (_ 4)
          (org-cycle)
          (push (funcall snapshot 'local-a1a) states))
        (goto-char (point-min))
        (search-forward "Beta")
        (beginning-of-line)
        (org-fold-hide-subtree)
        (push (funcall snapshot 'hide-beta) states)
        (search-forward "B1a-i body")
        (org-fold-show-context 'default)
        (push (funcall snapshot 'context-beta) states)
        (goto-char (point-min))
        (dotimes (_ 4)
          (org-cycle)
          (push (funcall snapshot 'global-bob) states))
        (org-fold-show-all)
        (goto-char (point-min))
        (search-forward "A1a-i-x body")
        (end-of-line)
        (insert "\npost recovery edit")
        (push (funcall snapshot 'after-edit) states)
        (list (nreverse states)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_repeated_deep_fold_expand_edit_no_heading_merge_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-global-at-bob t)
          (org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-hide-leading-stars nil)
          (org-cycle-separator-lines 0))
      (org-mode)
      (insert "* Alpha\nalpha body one\n")
      (insert "** Alpha child\nalpha child body\n")
      (insert "*** Alpha grand\nalpha grand body\n")
      (insert "**** Alpha fourth\nalpha fourth body\n")
      (insert "***** Alpha fifth\nalpha fifth body\n")
      (insert "****** Alpha sixth\nalpha sixth body\n")
      (insert "******* Alpha seventh\nalpha seventh body\n")
      (insert "******** Alpha eighth\nalpha eighth body\n")
      (insert "** Alpha sibling\nalpha sibling body\n")
      (insert "* Beta\nbeta body\n")
      (insert "** Beta child\nbeta child body\n")
      (insert "*** Beta grand\nbeta grand body\n")
      (insert "**** Beta fourth\nbeta fourth body\n")
      (insert "***** Beta fifth\nbeta fifth body\n")
      (insert "* Gamma\ngamma body\n")
      (let ((headings
             '("Alpha" "Alpha child" "Alpha grand" "Alpha fourth"
               "Alpha fifth" "Alpha sixth" "Alpha seventh"
               "Alpha eighth" "Alpha sibling" "Beta" "Beta child"
               "Beta grand" "Beta fourth" "Beta fifth" "Gamma"))
            (bodies
             '("alpha body one" "alpha fourth body" "alpha eighth body"
               "alpha sibling body" "beta fourth body" "gamma body"))
            states)
        (let ((snapshot
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
                            (beginning-of-line)
                            (let ((pos (point)))
                              (list needle
                                    (line-number-at-pos pos)
                                    (org-at-heading-p)
                                    (and (org-at-heading-p)
                                         (org-outline-level))
                                    (invisible-p pos)
                                    (org-fold-folded-p
                                     (line-end-position) 'headline)
                                    (get-text-property pos 'face)
                                    (get-text-property
                                     (match-beginning 0) 'face))))))
                        headings)
                       (mapcar
                        (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (search-forward needle)
                            (list needle
                                  (line-number-at-pos)
                                  (invisible-p (point))
                                  (get-text-property (point) 'face))))
                        bodies)
                       (save-excursion
                         (goto-char (point-min))
                         (let (out)
                           (while (re-search-forward "^\\(\\*+\\) \\(.*\\)$" nil t)
                             (push (list (match-string 1)
                                         (match-string 2)
                                         (line-number-at-pos))
                                   out))
                           (nreverse out)))
                       (count-matches "^\\*+ " (point-min) (point-max))
                       (count-lines (point-min) (point-max))
                       (buffer-substring-no-properties
                        (point-min) (point-max)))))))
          (push (funcall snapshot 'initial) states)
          (goto-char (point-min))
          (search-forward "Alpha fourth")
          (beginning-of-line)
          (dotimes (_ 5)
            (org-cycle)
            (push (funcall snapshot 'local-fourth) states))
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (push (funcall snapshot 'alpha-hidden) states)
          (org-fold-show-subtree)
          (push (funcall snapshot 'alpha-shown) states)
          (goto-char (point-min))
          (search-forward "Alpha eighth")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (push (funcall snapshot 'eighth-hidden) states)
          (org-end-of-subtree t t)
          (insert "******** Alpha eighth inserted sibling\ninserted body\n")
          (push (funcall snapshot 'after-hidden-insert) states)
          (org-fold-show-all)
          (push (funcall snapshot 'after-show-all) states)
          (goto-char (point-min))
          (dotimes (_ 6)
            (org-cycle-global)
            (push (funcall snapshot 'global-cycle) states))
          (goto-char (point-min))
          (search-forward "Beta fifth")
          (beginning-of-line)
          (dotimes (_ 4)
            (org-cycle)
            (push (funcall snapshot 'local-beta-fifth) states))
          (org-fold-show-all)
          (push (funcall snapshot 'final-show-all) states)
          (list (nreverse states)
                (split-string
                 (buffer-substring-no-properties (point-min) (point-max))
                 "\n" t))))))"##,
    );
}

#[test]
fn org_fold_move_reveal_deep_font_faces_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-cycle-hide-drawer-startup t)
          (org-cycle-hide-block-startup t)
          (org-cycle-global-at-bob t)
          (org-cycle-separator-lines 1))
      (org-mode)
      (insert "#+STARTUP: content\n")
      (insert "* TODO Project :work:\n")
      (insert ":PROPERTIES:\n:VISIBILITY: children\n:Owner: Ada\n:END:\n")
      (insert "project body\n")
      (insert "** NEXT Alpha\nalpha body\n")
      (insert "*** WAIT Alpha child\nchild body\n")
      (insert "**** TODO Alpha level four :deep:\nlevel four body\n")
      (insert "***** DONE Alpha level five\nlevel five body\n")
      (insert "****** TODO Alpha level six\nlevel six body\n")
      (insert "#+begin_src emacs-lisp\n(message \"alpha\")\n#+end_src\n")
      (insert "** TODO Beta\nbeta body\n")
      (insert "*** TODO Beta child\nbeta child body\n")
      (insert "**** TODO Beta level four\nbeta level four body\n")
      (insert "* Tail\ntail body\n")
      (let ((needles
             '("Project" ":Owner:" "project body" "Alpha" "alpha body"
               "Alpha child" "child body" "Alpha level four"
               "level four body" "Alpha level five" "level five body"
               "Alpha level six" "level six body" "(message \"alpha\")"
               "Beta" "beta body" "Beta child" "beta child body"
               "Beta level four" "beta level four body" "Tail"
               "tail body"))
            states)
        (let ((snapshot
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
                            (list needle
                                  (line-number-at-pos)
                                  (invisible-p (point))
                                  (get-text-property
                                   (line-beginning-position) 'face)
                                  (get-text-property
                                   (match-beginning 0) 'face))))
                        needles)
                       (save-excursion
                         (goto-char (point-min))
                         (let (out)
                           (while (re-search-forward
                                   "^\\(\\*+\\) \\([A-Z]+\\)? ?\\([^:\n]+\\)\\(?: \\(:[[:alnum:]_@#%:]+:\\)\\)?"
                                   nil t)
                             (push (list (match-string 1)
                                         (match-string 2)
                                         (substring-no-properties
                                          (match-string 3))
                                         (match-string 4)
                                         (org-outline-level)
                                         (get-text-property
                                          (line-beginning-position) 'face)
                                         (get-text-property
                                          (match-beginning 1) 'face)
                                         (and (match-beginning 2)
                                              (get-text-property
                                               (match-beginning 2) 'face))
                                         (get-text-property
                                          (match-beginning 3) 'face))
                                   out))
                           (nreverse out)))
                       (count-matches "^\\*+ " (point-min) (point-max))
                       (count-lines (point-min) (point-max))
                       (split-string
                        (buffer-substring-no-properties
                         (point-min) (point-max))
                        "\n" t)))))
          (org-cycle-set-startup-visibility)
          (push (funcall snapshot 'startup) states)
          (goto-char (point-min))
          (search-forward "Alpha level four")
          (beginning-of-line)
          (dotimes (_ 4)
            (org-cycle)
            (push (funcall snapshot 'local-alpha-four) states))
          (org-fold-show-subtree)
          (search-forward "Alpha level six")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (org-end-of-subtree t t)
          (insert "****** TODO Alpha inserted after hidden\ninserted alpha body\n")
          (push (funcall snapshot 'hidden-insert) states)
          (org-fold-show-all)
          (push (funcall snapshot 'show-all-after-insert) states)
          (goto-char (point-min))
          (search-forward "Beta child")
          (beginning-of-line)
          (org-cut-subtree)
          (goto-char (point-min))
          (search-forward "Alpha inserted")
          (beginning-of-line)
          (org-paste-subtree 5)
          (push (funcall snapshot 'after-move-beta-child) states)
          (goto-char (point-min))
          (search-forward "Beta level four")
          (beginning-of-line)
          (org-demote-subtree)
          (search-forward "Beta level four")
          (beginning-of-line)
          (org-promote-subtree)
          (push (funcall snapshot 'after-level-roundtrip) states)
          (org-fold-hide-sublevels 2)
          (goto-char (point-min))
          (search-forward "beta level four body")
          (org-fold-show-context 'isearch)
          (push (funcall snapshot 'after-context-reveal) states)
          (goto-char (point-min))
          (dotimes (_ 5)
            (org-cycle-global)
            (push (funcall snapshot 'global-cycle) states))
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((bad-lines nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line bad-lines)))
            (push (funcall snapshot 'final) states)
            (list (nreverse states)
                  (nreverse bad-lines)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_mixed_deep_objects_reveal_font_roundtrip_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-global-at-bob t)
          (org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t)
          (org-hide-emphasis-markers t)
          (org-link-descriptive t)
          (org-cycle-hide-drawer-startup t)
          (org-cycle-hide-block-startup t)
          (org-cycle-separator-lines 0))
      (org-mode)
      (insert "#+STARTUP: overview hideblocks\n")
      (insert "* TODO Root [#A] :root:\n")
      (insert "SCHEDULED: <2026-05-27 Wed 09:00>\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:VISIBILITY: children\n:END:\n")
      (insert "Root paragraph with *bold* and [[https://example.test][link]].\n")
      (insert "** NEXT Alpha :work:\n")
      (insert "- [ ] task one\n  - nested item\n")
      (insert "| Name | Qty |\n|------+-----|\n| A    |   1 |\n")
      (insert "*** WAIT Alpha child :deep:\n")
      (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
      (insert "**** TODO Alpha fourth [#B]\n")
      (insert "Alpha fourth paragraph with footnote[fn:one].\n")
      (insert "***** DONE Alpha fifth\n")
      (insert "Fifth body before folds.\n")
      (insert "** TODO Beta :work:\n")
      (insert ":LOGBOOK:\nCLOCK: [2026-05-27 Wed 10:00]--[2026-05-27 Wed 10:30] =>  0:30\n:END:\n")
      (insert "*** TODO Beta child\n")
      (insert "**** TODO Beta fourth\n")
      (insert "Beta fourth body.\n")
      (insert "* Tail\nTail body.\n")
      (insert "[fn:one] Footnote body.\n")
      (let ((needles
             '("Root" "SCHEDULED:" ":Owner:" "Root paragraph"
               "Alpha" "- [ ] task one" "nested item" "| A"
               "Alpha child" "(+ 1 2)" "Alpha fourth"
               "footnote[fn:one]" "Alpha fifth" "Fifth body"
               "Beta" ":LOGBOOK:" "Beta child" "Beta fourth"
               "Beta fourth body" "Tail" "Footnote body"))
            states)
        (let ((snapshot
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
                            (let ((pos (match-beginning 0)))
                              (list needle
                                    (line-number-at-pos pos)
                                    (invisible-p pos)
                                    (get-text-property pos 'face)
                                    (get-text-property
                                     (line-beginning-position) 'face)
                                    (get-text-property pos 'invisible)))))
                        needles)
                       (save-excursion
                         (goto-char (point-min))
                         (let (out)
                           (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
                             (push (list (match-string 1)
                                         (match-string 2)
                                         (org-outline-level)
                                         (get-text-property
                                          (line-beginning-position) 'face))
                                   out))
                           (nreverse out)))
                       (count-matches "^\\*+ " (point-min) (point-max))
                       (count-lines (point-min) (point-max))
                       (buffer-substring-no-properties
                        (point-min) (point-max)))))))
          (push (funcall snapshot 'initial) states)
          (org-cycle-set-startup-visibility)
          (push (funcall snapshot 'startup) states)
          (org-fold-hide-block-all)
          (org-fold-hide-drawer-all)
          (push (funcall snapshot 'hide-blocks-drawers) states)
          (org-fold-hide-sublevels 3)
          (push (funcall snapshot 'sublevels-3) states)
          (goto-char (point-min))
          (search-forward "(+ 1 2)")
          (org-fold-show-context 'local)
          (push (funcall snapshot 'reveal-src-local) states)
          (goto-char (point-min))
          (search-forward "Alpha fourth")
          (beginning-of-line)
          (dotimes (_ 4)
            (org-cycle)
            (push (funcall snapshot 'cycle-alpha-fourth) states))
          (goto-char (point-min))
          (search-forward "Alpha fifth")
          (beginning-of-line)
          (org-demote-subtree)
          (search-forward "Alpha fifth")
          (beginning-of-line)
          (org-promote-subtree)
          (push (funcall snapshot 'level-roundtrip) states)
          (org-fold-show-all)
          (goto-char (point-min))
          (search-forward "Fifth body before folds.")
          (end-of-line)
          (insert "\nFifth body after reveal edit.")
          (goto-char (point-min))
          (search-forward "Beta child")
          (beginning-of-line)
          (org-cut-subtree)
          (goto-char (point-min))
          (search-forward "Alpha fifth")
          (beginning-of-line)
          (org-paste-subtree 5)
          (push (funcall snapshot 'after-edit-move) states)
          (goto-char (point-min))
          (dotimes (_ 5)
            (org-cycle-global)
            (push (funcall snapshot 'global-cycle) states))
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((merged nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (push (funcall snapshot 'final) states)
            (list (nreverse states)
                  (nreverse merged)
                  (mapcar (lambda (needle)
                            (not (null
                                  (string-match-p
                                   needle
                                   (buffer-substring-no-properties
                                    (point-min) (point-max))))))
                          '("Fifth body after reveal edit."
                            "***** TODO Beta child"
                            "****** TODO Beta fourth"
                            "[fn:one] Footnote body."))
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_repeated_deep_heading_merge_font_regression_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (require 'org-fold-core)
  (with-temp-buffer
    (let ((org-cycle-global-at-bob t)
          (org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-hide-emphasis-markers t)
          (org-cycle-separator-lines 0)
          (org-startup-folded 'showeverything))
      (org-mode)
      (insert "* TODO Project :root:\n")
      (insert "Project intro with /emphasis/ and [[https://example.test][link]].\n")
      (insert "** TODO Area A :a:\n")
      (insert "Area A body.\n")
      (insert "*** TODO Thread A1\n")
      (insert "Thread A1 body.\n")
      (insert "**** TODO Fourth A1\n")
      (insert "Fourth A1 body before cycles.\n")
      (insert "***** WAIT Fifth A1\n")
      (insert "Fifth A1 body.\n")
      (insert "****** DONE Sixth A1\n")
      (insert "Sixth A1 body.\n")
      (insert "**** TODO Fourth A2\n")
      (insert "Fourth A2 body.\n")
      (insert "** TODO Area B :b:\n")
      (insert "*** NEXT Thread B1\n")
      (insert "**** TODO Fourth B1\n")
      (insert "Fourth B1 body.\n")
      (insert "***** TODO Fifth B1\n")
      (insert "Fifth B1 body.\n")
      (insert "* Tail\nTail body.\n")
      (let ((needles
             '("Project" "Area A" "Thread A1" "Fourth A1"
               "Fourth A1 body" "Fifth A1" "Fifth A1 body"
               "Sixth A1" "Sixth A1 body" "Fourth A2"
               "Area B" "Thread B1" "Fourth B1" "Fifth B1"
               "Tail"))
            states)
        (let ((headings
               (lambda ()
                 (font-lock-ensure (point-min) (point-max))
                 (let (out)
                   (goto-char (point-min))
                   (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
                     (let ((beg (line-beginning-position)))
                       (push (list (match-string 1)
                                   (match-string 2)
                                   (org-outline-level)
                                   (get-text-property beg 'face)
                                   (get-text-property
                                    (match-beginning 2) 'face)
                                   (get-text-property beg 'invisible)
                                   (org-fold-folded-p beg 'headline)
                                   (org-fold-get-region-at-point
                                    '(outline headline)
                                    (match-beginning 2)))
                             out)))
                   (nreverse out))))
              (visibility
               (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (search-forward needle)
                      (let ((pos (match-beginning 0)))
                        (list needle
                              (line-number-at-pos pos)
                              (current-column)
                              (invisible-p pos)
                              (get-text-property pos 'invisible)
                              (get-text-property pos 'face)))))
                  needles)))
              (fold-regions
               (lambda ()
                 (sort
                  (mapcar (lambda (region)
                            (list (nth 0 region)
                                  (nth 1 region)
                                  (nth 2 region)))
                          (org-fold-core-get-regions
                           :specs '(org-fold-outline)
                           :from (point-min)
                           :to (point-max)
                           :relative t))
                  (lambda (a b)
                    (if (= (car a) (car b))
                        (< (nth 1 a) (nth 1 b))
                      (< (car a) (car b)))))))
              (snapshot
               (lambda (label)
                 (list label
                       org-cycle-global-status
                       org-cycle-subtree-status
                       (funcall visibility)
                       (funcall headings)
                       (funcall fold-regions)
                       (count-matches "^\\*+ " (point-min) (point-max))
                       (count-lines (point-min) (point-max))
                       (buffer-substring-no-properties
                        (point-min) (point-max)))))))
          (push (funcall snapshot 'initial) states)
          (goto-char (point-min))
          (search-forward "Fourth A1")
          (beginning-of-line)
          (dotimes (_ 6)
            (org-cycle)
            (push (funcall snapshot 'cycle-fourth-a1) states))
          (goto-char (point-min))
          (search-forward "Fifth A1")
          (beginning-of-line)
          (dotimes (_ 4)
            (org-cycle)
            (push (funcall snapshot 'cycle-fifth-a1) states))
          (goto-char (point-min))
          (search-forward "Area A")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (goto-char (point-min))
          (search-forward "Fourth A1 body before cycles.")
          (end-of-line)
          (insert "\nFourth A1 inserted while ancestor hidden.")
          (push (funcall snapshot 'hidden-ancestor-edit) states)
          (org-fold-show-subtree)
          (goto-char (point-min))
          (search-forward "Sixth A1")
          (beginning-of-line)
          (org-demote-subtree)
          (search-forward "Sixth A1")
          (beginning-of-line)
          (org-promote-subtree)
          (push (funcall snapshot 'sixth-level-roundtrip) states)
          (goto-char (point-min))
          (search-forward "Fourth B1")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (org-fold-show-context 'default)
          (push (funcall snapshot 'context-fourth-b1) states)
          (goto-char (point-min))
          (dotimes (_ 7)
            (org-cycle-global)
            (push (funcall snapshot 'global-cycle) states))
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((merged nil)
                (bad-levels nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (goto-char (point-min))
            (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
              (let ((stars (length (match-string 1)))
                    (level (org-outline-level)))
                (unless (= stars level)
                  (push (list (match-string 0) stars level) bad-levels))))
            (push (funcall snapshot 'final) states)
            (list (nreverse states)
                  (nreverse merged)
                  (nreverse bad-levels)
                  (mapcar
                   (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward needle nil t)))
                   '("Fourth A1 inserted while ancestor hidden."
                      "****** DONE Sixth A1"
                      "***** TODO Fifth B1"))
                   (buffer-substring-no-properties
                    (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_deep_level_aggressive_cycle_hidden_edit_font_regression_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-global-at-bob t)
          (org-cycle-level-faces t)
          (org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-separator-lines 0))
      (org-mode)
      (insert "* TODO Root\n")
      (insert "Root body.\n")
      (insert "** TODO L2\n")
      (insert "L2 body.\n")
      (insert "*** TODO L3\n")
      (insert "L3 body.\n")
      (insert "**** TODO L4\n")
      (insert "L4 body.\n")
      (insert "***** TODO L5\n")
      (insert "L5 body.\n")
      (insert "****** DONE L6\n")
      (insert "L6 body.\n")
      (insert "******* WAIT L7\n")
      (insert "L7 body.\n")
      (insert "******** TODO L8\n")
      (insert "L8 body.\n")
      (insert "********* DONE L9\n")
      (insert "L9 body.\n")
      (insert "** NEXT Sibling\n")
      (insert "Sibling body.\n")
      (insert "*** TODO Sibling child\n")
      (insert "Sibling child body.\n")
      (insert "* Tail\n")
      (insert "Tail body.\n")
      (let ((headings
             '("Root" "L2" "L3" "L4" "L5" "L6" "L7" "L8" "L9"
               "Sibling" "Sibling child" "Tail"))
            (bodies
             '("Root body" "L2 body" "L3 body" "L4 body" "L5 body"
               "L6 body" "L7 body" "L8 body" "L9 body"
               "Sibling body" "Sibling child body" "Tail body"))
            states)
        (let ((snapshot
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
                            (let ((pos (match-beginning 0)))
                              (list needle
                                    (line-number-at-pos pos)
                                    (invisible-p pos)
                                    (get-text-property
                                     (line-beginning-position) 'face)
                                    (get-text-property pos 'face)))))
                        headings)
                       (mapcar
                        (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (search-forward needle)
                            (let ((pos (match-beginning 0)))
                              (list needle
                                    (line-number-at-pos pos)
                                    (invisible-p pos)))))
                        bodies)
                       (count-matches "^\\*+ " (point-min) (point-max))
                       (count-lines (point-min) (point-max))
                       (split-string
                        (buffer-substring-no-properties
                         (point-min) (point-max))
                        "\n" t)))))
          (push (funcall snapshot 'initial) states)
          ;; Aggressive cycle on L4
          (goto-char (point-min))
          (search-forward "L4")
          (beginning-of-line)
          (dotimes (_ 6)
            (org-cycle)
            (push (funcall snapshot 'cycle-l4) states))
          ;; Aggressive cycle on L7
          (goto-char (point-min))
          (search-forward "L7")
          (beginning-of-line)
          (dotimes (_ 5)
            (org-cycle)
            (push (funcall snapshot 'cycle-l7) states))
          ;; Hide L5 subtree, edit while hidden
          (goto-char (point-min))
          (search-forward "L5")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (push (funcall snapshot 'hide-l5) states)
          (end-of-line)
          (insert "\n***** TODO Inserted under hidden L5\nInserted L5 body.\n")
          (push (funcall snapshot 'hidden-insert) states)
          ;; Show L5 subtree back
          (goto-char (point-min))
          (search-forward "L5")
          (beginning-of-line)
          (org-fold-show-subtree)
          (push (funcall snapshot 'show-l5) states)
          ;; Hide L8 subtree, edit while hidden
          (goto-char (point-min))
          (search-forward "L8")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (end-of-line)
          (insert "\n******** TODO Inserted under hidden L8\nInserted L8 body.\n")
          (push (funcall snapshot 'hide-l8-insert) states)
          ;; Global cycles
          (goto-char (point-min))
          (dotimes (_ 6)
            (org-cycle-global)
            (push (funcall snapshot 'global-cycle) states))
          ;; Show all and check
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((merged nil)
                (bad-levels nil))
            (dolist (line (split-string
                           (buffer-substring-no-properties
                            (point-min) (point-max))
                           "\n" t))
              (when (string-match-p "^\\*+ .*\\*+ " line)
                (push line merged)))
            (goto-char (point-min))
            (while (re-search-forward "^\\(\\*+\\) +\\(.*\\)$" nil t)
              (let ((stars (length (match-string 1)))
                    (level (org-outline-level)))
                (unless (= stars level)
                  (push (list (match-string 0) stars level)
                        bad-levels))))
            (push (funcall snapshot 'final) states)
            (list (nreverse states)
                  (nreverse merged)
                  (nreverse bad-levels)
                  (mapcar
                   (lambda (needle)
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward needle nil t)))
                   '("***** TODO Inserted under hidden L5"
                     "Inserted L5 body."
                     "******** TODO Inserted under hidden L8"
                     "Inserted L8 body."
                     "********* DONE L9"))
                  (buffer-substring-no-properties
                   (point-min) (point-max)))))))))"##,
    );
}
