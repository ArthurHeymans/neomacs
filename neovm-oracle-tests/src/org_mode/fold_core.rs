use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_fold_core_region_copy_narrow_edit_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (require 'org-fold-core)
  (with-temp-buffer
    (let ((org-cycle-include-plain-lists 'integrate)
          (org-cycle-hide-drawer-startup t)
          (org-cycle-hide-block-startup t)
          (org-fold-show-context-detail
           '((default . lineage)
             (isearch . lineage)
             (bookmark-jump . lineage)))))
      (org-mode)
      (insert "#+STARTUP: content hideblocks\n")
      (insert "* Alpha\n")
      (insert ":PROPERTIES:\n:VISIBILITY: folded\n:Owner: Ada\n:END:\n")
      (insert "alpha body one\n\n")
      (insert "- [ ] parent\n")
      (insert "  - [X] child\n")
      (insert "  - [ ] hidden child\n\n")
      (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
      (insert "** Beta\n")
      (insert ":LOGBOOK:\nCLOCK: [2026-05-27 Wed 08:00]--[2026-05-27 Wed 09:15] =>  1:15\n:END:\n")
      (insert "beta body\n")
      (insert "*** Gamma\n")
      (insert "gamma body\n")
      (insert "**** Delta\n")
      (insert "delta body\n")
      (insert "** Epsilon\n")
      (insert "epsilon body\n")
      (insert "* Zeta\nzeta body\n")
      (let ((needles
             '("Alpha" ":Owner:" "alpha body one" "parent" "child"
               "(+ 1 2)" "Beta" "CLOCK:" "beta body" "Gamma"
               "gamma body" "Delta" "delta body" "Epsilon"
               "epsilon body" "Zeta" "zeta body"))
            states)
        (let ((fold-regions
               (lambda ()
                 (sort
                  (mapcar (lambda (region)
                            (list (nth 0 region)
                                  (nth 1 region)
                                  (nth 2 region)
                                  (buffer-substring-no-properties
                                   (max (point-min) (nth 0 region))
                                   (min (point-max) (nth 1 region)))))
                          (org-fold-core-get-regions
                           :specs '(org-fold-outline
                                    org-fold-drawer
                                    org-fold-block)
                           :from (point-min)
                           :to (point-max)
                           :relative t))
                  (lambda (a b)
                    (if (= (car a) (car b))
                        (string< (symbol-name (nth 2 a))
                                 (symbol-name (nth 2 b)))
                      (< (car a) (car b)))))))
              (visibility
               (lambda ()
                 (mapcar
                  (lambda (needle)
                    (save-excursion
                      (goto-char (point-min))
                      (search-forward needle)
                      (list needle
                            (line-number-at-pos)
                            (current-column)
                            (invisible-p (point))
                            (get-text-property (point) 'invisible)
                            (org-fold-get-region-at-point
                             '(headline drawer block)
                             (point)))))
                  needles)))
              (snapshot
               (lambda (label)
                 (let ((visible-copy
                        (progn
                          (org-copy-visible (point-min) (point-max))
                          (current-kill 0 t))))
                   (list label
                         org-cycle-global-status
                         org-cycle-subtree-status
                         (funcall visibility)
                         (funcall fold-regions)
                         (split-string visible-copy "\n" t)
                         (count-lines (point-min) (point-max))
                         (buffer-substring-no-properties
                          (point-min) (point-max)))))))
          (org-cycle-set-startup-visibility)
          (org-fold-hide-drawer-all)
          (org-fold-hide-block-all)
          (push (funcall snapshot 'startup) states)
          (goto-char (point-min))
          (search-forward "parent")
          (beginning-of-line)
          (dotimes (_ 3)
            (org-cycle)
            (push (funcall snapshot 'plain-list-cycle) states))
          (goto-char (point-min))
          (search-forward "Beta")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (push (funcall snapshot 'beta-hidden) states)
          (save-restriction
            (org-narrow-to-subtree)
            (goto-char (point-min))
            (search-forward "Gamma")
            (beginning-of-line)
            (org-fold-show-subtree)
            (org-fold-hide-drawer-all)
            (push (list 'narrowed
                        (point-min)
                        (point-max)
                        (funcall visibility)
                        (funcall fold-regions)
                        (buffer-substring-no-properties
                         (point-min) (point-max)))
                  states))
          (push (funcall snapshot 'after-widen) states)
          (goto-char (point-min))
          (search-forward "delta body")
          (org-fold-show-context 'isearch)
          (push (funcall snapshot 'delta-revealed) states)
          (goto-char (point-min))
          (search-forward "Delta")
          (beginning-of-line)
          (org-fold-hide-subtree)
          (end-of-line)
          (insert "\n**** Delta sibling after hidden\nsibling body\n")
          (push (funcall snapshot 'after-hidden-insert) states)
          (org-fold-show-all '(headlines drawers blocks))
          (push (funcall snapshot 'final-show-all) states)
          (list (nreverse states)
                (split-string
                 (buffer-substring-no-properties
                  (point-min) (point-max))
                 "\n" t))))))"##,
    );
}

#[test]
fn org_fold_context_narrow_subtree_drawer_block_recovery_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fold-show-context-detail
           '((default . lineage)
             (isearch . lineage)
             (occur . ancestors)
             (bookmark-jump . ancestors)
             (agenda . local)
             (mark-goto . lineage)
             (org-goto . ancestors))))
      (org-mode)
      (insert "* Root\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "Root paragraph.\n")
      (insert "#+begin_quote\nroot quote\n#+end_quote\n")
      (insert "** Alpha\n")
      (insert ":LOGBOOK:\nCLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 09:30] =>  0:30\n:END:\n")
      (insert "Alpha body.\n")
      (insert "*** Alpha child\n")
      (insert "Alpha child body.\n")
      (insert "**** Alpha fourth\n")
      (insert "Alpha fourth body.\n")
      (insert "***** Alpha fifth\n")
      (insert "Alpha fifth body.\n")
      (insert "****** Alpha sixth\n")
      (insert "Alpha sixth body.\n")
      (insert "** Beta\n")
      (insert "Beta body.\n")
      (insert "*** Beta child\n")
      (insert "Beta child body.\n")
      (insert "**** Beta fourth\n")
      (insert "Beta fourth body.\n")
      (insert "* Sibling\n")
      (insert "Sibling body.\n")
        (let ((probe
               (lambda (needle)
                 (save-excursion
                   (goto-char (point-min))
                   (if (search-forward needle nil t)
                       (list needle
                             (invisible-p (point))
                             (get-text-property (point) 'invisible))
                     (list needle 'not-found nil)))))
            states)
        (let ((snapshot
               (lambda (label)
                 (push (list label
                             (mapcar probe
                                     '("Root" "Root paragraph" "root quote"
                                       "Owner" "Alpha" "Alpha body"
                                       "Alpha child" "Alpha child body"
                                       "Alpha fourth" "Alpha fourth body"
                                       "Alpha fifth" "Alpha fifth body"
                                       "Alpha sixth" "Alpha sixth body"
                                       "Beta" "Beta body" "Beta child"
                                       "Beta child body" "Beta fourth"
                                       "Beta fourth body" "Sibling"
                                       "Sibling body")))
                       states))))
          (org-fold-hide-sublevels 1)
          (funcall snapshot 'overview)
          (goto-char (point-min))
          (search-forward "Alpha sixth body.")
          (org-fold-show-context 'isearch)
          (funcall snapshot 'isearch-sixth)
          (org-fold-hide-sublevels 1)
          (goto-char (point-min))
          (search-forward "Beta fourth body.")
          (org-fold-show-context 'default)
          (funcall snapshot 'default-beta-fourth)
          (org-fold-hide-sublevels 1)
          (goto-char (point-min))
          (search-forward "Alpha child body.")
          (org-fold-show-context 'agenda)
          (funcall snapshot 'agenda-alpha-child)
          (org-fold-hide-sublevels 1)
          (goto-char (point-min))
          (search-forward "root quote")
          (org-fold-show-context 'mark-goto)
          (funcall snapshot 'mark-goto-quote)
          (org-fold-show-all)
          (org-fold-hide-drawer-all)
          (org-fold-hide-block-all)
          (funcall snapshot 'drawers-blocks-hidden)
          (save-restriction
            (org-narrow-to-subtree)
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (org-fold-hide-subtree)
            (funcall snapshot 'narrowed-alpha-hidden)
            (org-fold-show-subtree)
            (funcall snapshot 'narrowed-alpha-shown)
            (org-fold-hide-sublevels 2)
            (funcall snapshot 'narrowed-sublevels-2))
          (funcall snapshot 'after-widen)
           (list (nreverse states)
                 (buffer-substring-no-properties
                  (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_fold_core_region_spec_visibility_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
    (insert "Alpha body.\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
    (insert "** Beta\n")
    (insert ":LOGBOOK:\nclock line\n:END:\n")
    (insert "Beta body.\n")
    (insert "*** Gamma\n")
    (insert "Gamma body.\n")
    (insert "* Delta\n")
    (insert "Delta body.\n")
    (let ((probe (lambda (needle)
                   (save-excursion
                     (goto-char (point-min))
                     (search-forward needle)
                     (list needle
                           (invisible-p (point))
                           (get-text-property (point) 'invisible))))))
      (org-fold-hide-drawer-all)
      (org-fold-hide-block-all)
      (let ((after-db (mapcar probe '("Owner" "(+ 1 2)" "clock line" "Alpha body" "Beta body" "Gamma body"))))
        (goto-char (point-min))
        (search-forward "Beta")
        (beginning-of-line)
        (org-fold-hide-subtree)
        (let ((after-hide (mapcar probe '("Beta body" "Gamma body" "Delta body"))))
          (org-fold-show-subtree)
          (let ((after-show (mapcar probe '("Beta body" "Gamma body" "Delta body"))))
            (org-fold-show-all '(drawers blocks))
            (let ((after-db-show (mapcar probe '("Owner" "(+ 1 2)" "clock line"))))
              (list after-db after-hide after-show after-db-show
                    (buffer-substring-no-properties
                     (point-min) (point-max))))))))))"##,
    );
}
