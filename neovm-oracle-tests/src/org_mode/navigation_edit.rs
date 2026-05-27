use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_element_navigation_positions_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* A\n")
    (insert "Paragraph one.\n\n")
    (insert "- item one\n- item two\n\n")
    (insert "#+begin_quote\nquoted\n#+end_quote\n")
    (insert "** B\nBody B\n")
    (insert "* C\nBody C\n")
    (let ((snap (lambda (label)
                  (let ((e (org-element-at-point)))
                    (list label
                          (point)
                          (org-element-type e)
                          (org-element-property :begin e)
                          (org-element-property :end e)
                          (thing-at-point 'line t)))))
          states)
      (goto-char (point-min))
      (push (funcall snap 'start) states)
      (org-forward-element)
      (push (funcall snap 'forward-headline) states)
      (search-forward "Paragraph")
      (push (funcall snap 'paragraph) states)
      (org-forward-element)
      (push (funcall snap 'forward-list) states)
      (org-down-element)
      (push (funcall snap 'down-item) states)
      (org-up-element)
      (push (funcall snap 'up-list) states)
      (org-forward-element)
      (push (funcall snap 'forward-quote) states)
      (org-backward-element)
      (push (funcall snap 'backward-list) states)
      (goto-char (point-min))
      (search-forward "** B")
      (beginning-of-line)
      (push (list 'end-subtree
                  (org-end-of-subtree t nil)
                  (save-excursion
                    (org-end-of-subtree t t)
                    (point))
                  (line-number-at-pos)))
            states)
      (nreverse states))))"##,
    );
}

#[test]
fn org_drag_transpose_element_buffer_integrity_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* H\n")
    (insert "First paragraph.\n\n")
    (insert "#+begin_quote\nQuote block.\n#+end_quote\n\n")
    (insert "- item one\n- item two\n\n")
    (insert "Final paragraph.\n")
    (goto-char (point-min))
    (search-forward "begin_quote")
    (beginning-of-line)
    (org-drag-element-forward)
    (let ((after-forward
           (buffer-substring-no-properties (point-min) (point-max))))
      (search-forward "Final paragraph")
      (beginning-of-line)
      (org-drag-element-backward)
      (let ((after-backward
             (buffer-substring-no-properties (point-min) (point-max))))
        (search-forward "item two")
        (beginning-of-line)
        (org-transpose-element)
        (list after-forward
              after-backward
              (buffer-substring-no-properties (point-min) (point-max))
              (org-element-map (org-element-parse-buffer)
                  '(paragraph quote-block plain-list item)
                (lambda (e)
                  (list (org-element-type e)
                        (org-element-property :begin e)
                        (org-element-property :end e))))))))"##,
    );
}

#[test]
fn org_mark_narrow_unindent_navigation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "  * A\n")
    (insert "    Body A\n")
    (insert "    ** B\n")
    (insert "      Body B\n")
    (insert "      - item\n")
    (insert "  * C\n")
    (insert "    Body C\n")
    (org-unindent-buffer)
    (let ((after-unindent
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "** B")
      (beginning-of-line)
      (let ((sibling-prev (save-excursion
                            (condition-case err
                                (progn (org-goto-sibling 'previous)
                                       (thing-at-point 'line t))
                              (error (cons (car err) (cdr err))))))
            (first-child (save-excursion
                           (org-goto-first-child)
                           (thing-at-point 'line t))))
        (search-forward "item")
        (org-mark-element)
        (let ((mark-span (list (point) (mark))))
          (org-narrow-to-element)
          (let ((narrow-text
                 (buffer-substring-no-properties (point-min) (point-max)))
                (narrow-limits (list (point-min) (point-max))))
            (widen)
            (list after-unindent
                  sibling-prev
                  first-child
                  mark-span
                  narrow-limits
                  narrow-text
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_copy_visible_clone_subtree_navigation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((kill-ring nil)
          (kill-ring-yank-pointer nil)
          (org-yank-folded-subtrees nil))
      (org-mode)
      (insert "* Project\n")
      (insert "** TODO Task\n")
      (insert "SCHEDULED: <2026-05-27 Wed>\n")
      (insert "Body task\n")
      (insert "*** Child\nChild body\n")
      (insert "** Keep\nKeep body\n")
      (insert "* Tail\nTail body\n")
      (goto-char (point-min))
      (org-fold-hide-sublevels 1)
      (org-copy-visible (point-min) (point-max))
      (let ((visible-copy (current-kill 0 t)))
        (org-fold-show-all)
        (goto-char (point-min))
        (search-forward "Task")
        (beginning-of-line)
        (org-copy-subtree 1)
        (goto-char (point-max))
        (org-paste-subtree 2)
        (let ((after-paste
               (buffer-substring-no-properties (point-min) (point-max))))
          (goto-char (point-min))
          (search-forward "Task")
          (beginning-of-line)
          (org-clone-subtree-with-time-shift 2 "+1w")
          (let ((nav nil))
            (goto-char (point-min))
            (while (re-search-forward "^\\*+ " nil t)
              (push (list (org-outline-level)
                          (org-get-heading t t t t)
                          (org-entry-get nil "SCHEDULED"))
                    nav))
            (goto-char (point-min))
            (search-forward "Child")
            (beginning-of-line)
            (list visible-copy
                  after-paste
                  (nreverse nav)
                  (org-up-heading-safe)
                  (org-get-heading t t t t)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_navigation_hidden_narrow_deep_faces_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-cycle)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-cycle-level-faces t)
          (org-level-color-stars-only nil))
      (org-mode)
      (insert "* Project\n")
      (insert "Intro paragraph.\n")
      (insert "** Alpha\n")
      (insert "Alpha body\n")
      (insert "*** Alpha child\n")
      (insert "Alpha child body\n")
      (insert "** COMMENT Folded comment\n")
      (insert "Comment body\n")
      (insert "*** Hidden comment child\n")
      (insert "Hidden body\n")
      (insert "** Beta archived :ARCHIVE:\n")
      (insert "Beta body\n")
      (insert "*** Beta child\n")
      (insert "Beta child body\n")
      (insert "** Gamma\n")
      (insert ":PROPERTIES:\n:Owner: me\n:END:\n")
      (insert "Gamma body\n")
      (insert "*** Gamma child\n")
      (insert "Gamma child body\n")
      (insert "**** Deep L4\n")
      (insert "Deep body\n")
      (insert "***** Deep L5\n")
      (insert "Deeper body\n")
      (insert "* Tail\nTail body\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((heading-state
             (lambda (label)
               (list label
                     (- (point) (point-min))
                     (line-number-at-pos)
                     (org-outline-level)
                     (org-get-heading t t t t)
                     (invisible-p (line-beginning-position))
                     (buffer-substring-no-properties
                      (line-beginning-position) (line-end-position)))))
            (goto-heading
             (lambda (needle)
               (goto-char (point-min))
               (search-forward needle)
               (beginning-of-line)))
            (hidden-state
             (lambda (label needles)
               (cons label
                     (mapcar
                      (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle
                                (- (point) (point-min))
                                (invisible-p (point)))))
                      needles)))))
            states)
        (funcall goto-heading "* Project")
        (org-fold-hide-subtree)
        (push (funcall hidden-state
                       'project-hidden
                       '("Intro" "Alpha body" "Folded comment"
                         "Beta body" "Gamma" "Tail"))
              states)
        (org-fold-show-all)
        (funcall goto-heading "** Alpha")
        (org-fold-hide-subtree)
        (push (list 'alpha-boundaries
                    (save-excursion
                      (funcall goto-heading "** Alpha")
                      (list (- (org-end-of-subtree nil nil) (point-min))
                            (line-number-at-pos)))
                    (save-excursion
                      (funcall goto-heading "** Alpha")
                      (list (- (org-end-of-subtree t nil) (point-min))
                            (line-number-at-pos)))
                    (save-excursion
                      (funcall goto-heading "** Alpha")
                      (list (- (org-end-of-subtree t t) (point-min))
                            (line-number-at-pos))))
              states)
        (push (funcall hidden-state
                       'alpha-hidden
                       '("Alpha body" "Alpha child" "Folded comment"
                         "Beta archived" "Gamma"))
              states)
        (funcall goto-heading "** Alpha")
        (org-forward-heading-same-level 1 nil)
        (push (funcall heading-state 'same-level-visible-1) states)
        (funcall goto-heading "** Alpha")
        (org-forward-heading-same-level 2 t)
        (push (funcall heading-state 'same-level-invisible-2) states)
        (org-fold-hide-sublevels 2)
        (funcall goto-heading "* Project")
        (org-next-visible-heading 1)
        (push (funcall heading-state 'next-visible-after-project) states)
        (org-next-visible-heading 1)
        (push (funcall heading-state 'next-visible-second) states)
        (org-previous-visible-heading 1)
        (push (funcall heading-state 'previous-visible-back) states)
        (org-fold-show-all)
        (funcall goto-heading "*** Gamma child")
        (org-narrow-to-subtree)
        (let ((narrow-limits (list (- (point-min) 1) (- (point-max) 1))))
          (goto-char (point-min))
          (search-forward "Deep L5")
          (beginning-of-line)
          (let ((up1 (progn
                       (org-up-heading-safe)
                       (funcall heading-state 'up-from-l5)))
                (up2 (progn
                       (org-up-heading-safe)
                       (funcall heading-state 'up-from-l4)))
                (up3 (progn
                       (org-up-heading-safe)
                       (funcall heading-state 'up-from-child))))
            (push (list 'narrowed-up narrow-limits up1 up2 up3)
                  states)))
        (widen)
        (font-lock-ensure (point-min) (point-max))
        (push (let (faces)
                (dolist (needle '("Gamma child" "Deep L4" "Deep L5"))
                  (goto-char (point-min))
                  (search-forward needle)
                  (push (list needle
                              (get-text-property
                               (line-beginning-position) 'face)
                              (get-text-property
                               (match-beginning 0) 'face)
                              (get-text-property
                               (match-beginning 0) 'font-lock-fontified))
                        faces))
                (cons 'deep-heading-faces (nreverse faces)))
              states)
        (push (buffer-substring-no-properties (point-min) (point-max))
              states)
        (nreverse states))))"##,
    );
}
