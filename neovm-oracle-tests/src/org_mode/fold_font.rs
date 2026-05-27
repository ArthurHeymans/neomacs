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
