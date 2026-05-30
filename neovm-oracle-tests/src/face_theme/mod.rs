//! Face and theme oracle divergence tests.
//!
//! Complex tests that capture deep face/text-property state to surface
//! divergences in fontification, theming, font-lock, and face rendering.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn ft_complex_org_faces_state_transitions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* TODO Alpha :work:\n")
      (insert "SCHEDULED: <2026-05-28 Wed>\n")
      (insert ":PROPERTIES:\n:Effort: 2h\n:Owner: Alice\n:END:\n")
      (insert "Body alpha.\n\n")
      (insert "** DONE Beta :home:\n")
      (insert "CLOSED: [2026-05-27 Tue 14:00]\n")
      (insert ":LOGBOOK:\nCLOCK: [2026-05-27 Tue 10:00]--[2026-05-27 Tue 14:00] =>  4:00\n:END:\n")
      (insert "Body beta.\n\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Capture faces at every element
      (let ((snap (lambda ()
                    (list
                     ;; Heading faces
                     (mapcar (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (if (search-forward needle nil t)
                                     (list needle
                                           (get-text-property (match-beginning 0) 'face)
                                           (get-text-property (match-beginning 0) 'font-lock-face)
                                           (get-text-property (line-beginning-position) 'face))
                                     (list needle 'not-found nil nil))))
                             '("Alpha" "Beta"))
                     ;; TODO/DONE faces
                     (mapcar (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (if (search-forward needle nil t)
                                     (list needle
                                           (get-text-property (match-beginning 0) 'face))
                                     (list needle 'not-found))))
                             '("TODO" "DONE"))
                     ;; Tag faces
                     (mapcar (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (if (search-forward needle nil t)
                                     (list needle
                                           (get-text-property (match-beginning 0) 'face)
                                           (get-text-property (match-beginning 0) 'font-lock-face))
                                     (list needle 'not-found nil))))
                             '(":work:" ":home:"))
                     ;; Timestamp faces
                     (mapcar (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (if (search-forward needle nil t)
                                     (list needle
                                           (get-text-property (match-beginning 0) 'face))
                                     (list needle 'not-found))))
                             '("SCHEDULED" "CLOSED" "<2026-05-28 Wed>" "[2026-05-27 Tue"))
                     ;; Property faces
                     (mapcar (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (if (search-forward needle nil t)
                                     (list needle
                                           (get-text-property (match-beginning 0) 'face))
                                     (list needle 'not-found))))
                             '(":PROPERTIES:" ":Effort:" ":Owner:" ":END:"))
                     ;; LOGBOOK faces
                     (mapcar (lambda (needle)
                               (save-excursion
                                 (goto-char (point-min))
                                 (if (search-forward needle nil t)
                                     (list needle
                                           (get-text-property (match-beginning 0) 'face))
                                     (list needle 'not-found))))
                             '(":LOGBOOK:" "CLOCK"))
                     ;; Body text faces
                     (list (save-excursion
                             (goto-char (point-min))
                             (search-forward "Body alpha.")
                             (list 'body-alpha-face
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-text-property (match-beginning 0) 'fontified)))
                           (save-excursion
                             (goto-char (point-min))
                             (search-forward "Body beta.")
                             (list 'body-beta-face
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-text-property (match-beginning 0) 'fontified))))))))
        (let ((initial-faces (funcall snap)))
          ;; Cycle visibility states and re-check faces
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-cycle nil)  ;; overview
          (let ((overview-faces (funcall snap)))
            (org-cycle nil)  ;; children
            (font-lock-ensure (point-min) (point-max))
            (let ((children-faces (funcall snap)))
              (org-cycle nil)  ;; subtree (all)
              (font-lock-ensure (point-min) (point-max))
              (let ((subtree-faces (funcall snap)))
                (list initial-faces
                      overview-faces
                      children-faces
                      subtree-faces))))))))))"##,
    );
}

#[test]
fn ft_complex_face_overlay_textprop_interaction_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    ;; Layer 1: text properties
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 1 6 'font-lock-face 'bold-italic)
    ;; Layer 2: overlays
    (let ((ov1 (make-overlay 8 18)))
      (overlay-put ov1 'face '(:foreground "red")))
    (let ((ov2 (make-overlay 14 22)))
      (overlay-put ov2 'face '(:background "yellow")))
    ;; Capture composite face at each position
    (list
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'font-lock-face)
                     (get-char-property pos 'face)
                     (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov) (overlay-get ov 'face)))
                             (overlays-at pos))))
             '(1 5 8 12 15 18 21))
     ;; Test face with overlay removal
     (progn
       (delete-overlay ov1)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos
                       (get-text-property pos 'face)
                       (get-char-property pos 'face)))
               '(8 12 15)))
     ;; Test face after overlay re-add
     (progn
       (let ((ov3 (make-overlay 3 19)))
         (overlay-put ov3 'face '(:slant italic :weight bold)))
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(3 8 15 18))))))"##,
    );
}

#[test]
fn ft_complex_font_lock_refontify_after_edits_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Project\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:Effort: 5h\n:END:\n")
      (insert "** DONE Task-1\nBody 1.\n\n")
      (insert "** TODO Task-2\nBody 2.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-text-property (match-beginning 0) 'fontified)
                                   (get-text-property (match-beginning 0) 'font-lock-face))
                             (list needle 'not-found nil nil))))
                     '("TODO" "DONE" ":PROPERTIES:" ":END:")))))
        (let ((v0 (funcall snap)))
          ;; Edit: change Task-2 to DONE
          (goto-char (point-min))
          (search-forward "TODO Task-2")
          (replace-match "DONE Task-2")
          ;; Hide all
          (org-fold-hide-all)
          ;; Edit under hidden
          (goto-char (point-min))
          (search-forward "Project")
          (end-of-line)
          (insert "\n** WAIT Task-3\nBody 3.\n")
          ;; Show all and refontify
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Global cycle
            (org-global-cycle nil)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Edit: remove property drawer
              (goto-char (point-min))
              (search-forward ":PROPERTIES:")
              (beginning-of-line)
              (delete-region (point) (progn (search-forward ":END:") (end-of-line) (1+ (point))))
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3))))))))))"##,
    );
}

#[test]
fn ft_complex_face_inheritance_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* Root :root:\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:CATEGORY: main\n:END:\n")
      (insert "** Branch-A :fe:perf:\n")
      (insert ":PROPERTIES:\n:Effort: 3h\n:Priority: high\n:END:\n")
      (insert "*** Leaf-A1\nBody A1.\n\n")
      (insert "*** Leaf-A2\nBody A2.\n\n")
      (insert "** Branch-B :be:\n")
      (insert ":PROPERTIES:\n:Effort: 5h\n:END:\n")
      (insert "*** Leaf-B1\nBody B1.\n\n")
      (insert "* Other :other:\n")
      (insert "** Sub-Other\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (mapcar
       (lambda (needle)
         (save-excursion
           (goto-char (point-min))
           (if (search-forward needle nil t)
               (list needle
                     (org-outline-level)
                     (get-text-property (line-beginning-position) 'face)
                     (get-text-property (line-beginning-position) 'font-lock-face)
                     (org-get-tags nil t)
                     (org-entry-get nil "Owner" 'inherit)
                     (org-entry-get nil "CATEGORY" 'inherit)
                     (org-entry-get nil "Priority" 'inherit)
                     (org-entry-get nil "Effort"))
               (list needle 'not-found nil nil nil nil nil nil nil)))
       '("Root" "Branch-A" "Leaf-A1" "Leaf-A2" "Branch-B" "Leaf-B1"
         "Other" "Sub-Other")))))"##,
    );
}

#[test]
fn ft_complex_face_with_display_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'display)
  (with-temp-buffer
    (insert "Line with display property")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 1 6 'display "[[bolded]]")
    (put-text-property 6 9 'face 'italic)
    (put-text-property 10 17 'face 'underline)
    (put-text-property 10 17 'display "__________")
    (put-text-property 18 23 'face '(:foreground "blue"))
    (list
     ;; Face + display at position
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'display)
                     (get-text-property pos 'fontified)
                     (text-properties-at pos)))
             '(1 5 6 9 10 17 18))
     ;; After adding an overlay
     (progn
       (let ((ov (make-overlay 1 23)))
         (overlay-put ov 'face '(:background "gray"))
         (overlay-put ov 'display "***overlay***"))
       (list (get-char-property 1 'face)
             (get-char-property 1 'display)
             (get-char-property 10 'face)
             (get-char-property 10 'display)))
     ;; After overlay removal
     (progn
       (mapc #'delete-overlay (overlays-at 1))
       (list (get-char-property 1 'face)
             (get-char-property 1 'display))))))"##,
    );
}

#[test]
fn ft_complex_face_indirect_buffer_propagation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Base heading\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:END:\n")
      (insert "Base body.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let* ((base-name (buffer-name))
             (ind-name (generate-new-buffer-name (concat base-name "-ind")))
             (ind-buf (make-indirect-buffer (current-buffer) ind-name t)))
        (unwind-protect
            (let ((base-faces
                   (with-current-buffer (current-buffer)
                     (mapcar (lambda (needle)
                               (goto-char (point-min))
                               (if (search-forward needle nil t)
                                   (list needle
                                         (get-text-property (match-beginning 0) 'face)
                                         (get-text-property (match-beginning 0) 'fontified)
                                         (get-text-property (match-beginning 0) 'font-lock-face))
                                   (list needle 'not-found nil nil)))
                             '("TODO" "DONE" ":PROPERTIES:" ":END:"))))
                  (ind-faces
                   (with-current-buffer ind-buf
                     (font-lock-ensure (point-min) (point-max))
                     (mapcar (lambda (needle)
                               (goto-char (point-min))
                               (if (search-forward needle nil t)
                                   (list needle
                                         (get-text-property (match-beginning 0) 'face)
                                         (get-text-property (match-beginning 0) 'fontified)
                                         (get-text-property (match-beginning 0) 'font-lock-face))
                                   (list needle 'not-found nil nil)))
                             '("TODO" "DONE" ":PROPERTIES:" ":END:")))))
              ;; Edit in indirect buffer
              (with-current-buffer ind-buf
                (goto-char (point-min))
                (search-forward "TODO")
                (replace-match "DONE")
                (font-lock-ensure (point-min) (point-max)))
              (let ((base-after (with-current-buffer (current-buffer)
                                  (mapcar (lambda (needle)
                                            (goto-char (point-min))
                                            (if (search-forward needle nil t)
                                                (list needle
                                                      (get-text-property (match-beginning 0) 'face))
                                                (list needle 'not-found)))
                                          '("TODO" "DONE"))))
                    (ind-after (with-current-buffer ind-buf
                                 (mapcar (lambda (needle)
                                           (goto-char (point-min))
                                           (if (search-forward needle nil t)
                                               (list needle
                                                     (get-text-property (match-beginning 0) 'face))
                                               (list needle 'not-found)))
                                         '("TODO" "DONE")))))
                (list base-faces ind-faces base-after ind-after))))
        (when (get-buffer ind-name) (kill-buffer ind-name))))))"##,
    );
}

#[test]
fn ft_complex_face_remove_add_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property manipulation test line")
    ;; Add multiple face properties
    (add-face-text-property 1 10 'bold)
    (add-face-text-property 1 10 'italic)
    (add-face-text-property 1 6 'underline)
    (put-text-property 11 23 'font-lock-face 'bold-italic)
    (let ((snap (lambda ()
                  (mapcar (lambda (pos)
                            (goto-char pos)
                            (list pos
                                  (get-text-property pos 'face)
                                  (get-text-property pos 'font-lock-face)
                                  (text-properties-at pos)))
                          '(1 5 8 11 15 20)))))
      (let ((v0 (funcall snap)))
        ;; Remove face at position 1-6
        (remove-text-properties 1 6 '(face nil))
        (let ((v1 (funcall snap)))
          ;; Re-add face
          (put-text-property 1 6 'face '(:foreground "green" :weight bold))
          (let ((v2 (funcall snap)))
            ;; Replace all properties
            (set-text-properties 11 23 (list 'face '(:foreground "red" :slant italic)))
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_complex_org_face_global_cycle_edit_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* TODO A\nBody A.\n\n")
      (insert "** DONE A1\nBody A1.\n\n")
      (insert "** TODO A2\nBody A2.\n\n")
      (insert "* NEXT B\nBody B.\n\n")
      (insert "** WAIT B1\nBody B1.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (line-beginning-position) 'face)
                                   (get-text-property (line-beginning-position) 'font-lock-face)
                                   (invisible-p (match-beginning 0))
                                   (org-outline-level))
                             (list needle 'not-found nil nil nil))))
                     '("A" "A1" "A2" "B" "B1")))))
        (let ((v0 (funcall snap)))
          ;; Global cycle: overview
          (org-global-cycle nil)
          (let ((v1 (funcall snap)))
            ;; Edit: insert A3 under hidden A
            (goto-char (point-min))
            (search-forward "A")
            (end-of-line)
            (insert "\n** TODO A3\nBody A3.\n")
            (let ((v2 (funcall snap)))
              ;; Global cycle: children
              (org-global-cycle nil)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Global cycle: all
                (org-global-cycle nil)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  ;; Edit: change A2 to DONE
                  (goto-char (point-min))
                  (search-forward "TODO A2")
                  (replace-match "DONE A2")
                  (font-lock-ensure (point-min) (point-max))
                  (let ((v5 (funcall snap)))
                    (list v0 v1 v2 v3 v4 v5))))))))))))"##,
    );
}

#[test]
fn ft_complex_face_overlay_stack_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay stack test text here")
    (put-text-property 1 23 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face '(:foreground "red"))
      (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 5 15)))
      (overlay-put ov2 'face '(:foreground "green"))
      (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 10 20)))
      (overlay-put ov3 'face '(:foreground "orange"))
      (overlay-put ov3 'priority 5))
    (let ((ov4 (make-overlay 12 23)))
      (overlay-put ov4 'face '(:foreground "purple"))
      (overlay-put ov4 'priority 15))
    (list
     ;; Face at each position with overlay stack
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-char-property pos 'face)
                     (get-text-property pos 'face)
                     (mapcar (lambda (ov) (list (overlay-get ov 'face) (overlay-get ov 'priority)))
                             (sort (overlays-at pos) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority)))))))
             '(1 5 10 12 15 18 22))
     ;; Change priority and recheck
     (progn
       (overlay-put ov1 'priority 30)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(1 5 10)))
     ;; Remove top overlay and recheck
     (progn
       (delete-overlay ov2)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(5 10 12)))
     ;; Move overlay and recheck
     (progn
       (move-overlay ov3 1 8)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(1 5 8 10))))))"##,
    );
}

#[test]
fn ft_complex_org_face_property_drawer_hide_show() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Alpha\n")
      (insert ":PROPERTIES:\n:Effort: 2h\n:Owner: Alice\n:CUSTOM_ID: alpha\n:END:\n")
      (insert ":LOGBOOK:\nCLOCK: [2026-05-28 Wed 09:00]--[2026-05-28 Wed 11:00] =>  2:00\n:END:\n")
      (insert "Body alpha.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (match-beginning 0) 'face)
                                   (invisible-p (match-beginning 0)))
                             (list needle 'not-found nil))))
                     '(":PROPERTIES:" ":Effort:" ":Owner:" ":CUSTOM_ID:"
                       ":END:" ":LOGBOOK:" "CLOCK")))))
        (let ((v0 (funcall snap)))
          ;; Hide all drawers
          (goto-char (point-min))
          (search-forward "PROPERTIES")
          (beginning-of-line)
          (org-fold-hide-drawer-all)
          (let ((v1 (funcall snap)))
            ;; Show all drawers
            (org-fold-show-all)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Hide Alpha subtree
              (goto-char (point-min))
              (search-forward "Alpha")
              (beginning-of-line)
              (org-fold-hide-subtree)
              (let ((v3 (funcall snap)))
                ;; Show subtree
                (org-fold-show-subtree)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_complex_face_sticky_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Sticky property test boundary line")
    ;; Front-sticky face
    (put-text-property 1 8 'face 'bold)
    (put-text-property 1 8 'front-sticky t)
    ;; Rear-sticky face
    (put-text-property 8 17 'face 'italic)
    (put-text-property 8 17 'rear-nonsticky nil)
    ;; Non-sticky face
    (put-text-property 17 26 'face 'underline)
    ;; Both sticky
    (put-text-property 26 35 'face '(:foreground "red"))
    (put-text-property 26 35 'front-sticky t)
    (put-text-property 26 35 'rear-nonsticky nil)
    (list
     ;; Initial properties
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'front-sticky)
                     (get-text-property pos 'rear-nonsticky)))
             '(1 8 10 17 18 26 30 35))
     ;; Insert at boundary and check propagation
     (progn
       (goto-char 8)
       (insert "INSERTED")
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face)))
               '(1 5 8 12 15 18 25 30 35)))
     ;; Insert at non-sticky boundary
     (progn
       (goto-char 30)
       (insert "X")
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face)))
               '(17 25 30 32 38))))))"##,
    );
}

#[test]
fn ft_complex_face_font_lock_with_overlays_and_edits_deep() {
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
      (font-lock-ensure (point-min) (point-max))
      ;; Add overlays
      (let ((ov1 (make-overlay 1 6)))
        (overlay-put ov1 'face '(:background "yellow")))
      (let ((ov2 (make-overlay 15 20)))
        (overlay-put ov2 'face '(:foreground "red" :weight bold)))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-char-property (match-beginning 0) 'face))
                             (list needle 'not-found nil))))
                     '("TODO" "DONE" "Body alpha" "Body beta")))))
        (let ((v0 (funcall snap)))
          ;; Hide Alpha subtree
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-fold-hide-subtree)
          ;; Edit: insert Gamma under hidden Alpha
          (end-of-line)
          (insert "\n*** TODO Gamma\nBody gamma.\n")
          ;; Show all with fontify
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Remove overlays
            (delete-overlay ov1)
            (delete-overlay ov2)
            ;; Add new overlay
            (let ((ov3 (make-overlay 5 12)))
              (overlay-put ov3 'face '(:slant italic)))
            (let ((v2 (funcall snap)))
              ;; Global cycle
              (org-global-cycle nil)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3))))))))))"##,
    );
}

#[test]
fn ft_complex_face_color_value_comparison_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   ;; Named colors
   (mapcar (lambda (color)
             (list color
                   (condition-case nil (color-values color) (error 'no-values))
                   (condition-case nil (color-defined-p color) (error 'no-defined))))
           '("red" "green" "blue" "black" "white" "#FF0000" "#00FF00" "#0000FF"))
   ;; Color functions
   (list
    (fboundp 'color-dark-p)
    (fboundp 'color-light-name-p)
    (fboundp 'color-complement)
    (fboundp 'color-gradient)
    (condition-case nil (color-dark-p "white") (error 'no-darkp))
    (condition-case nil (color-dark-p "#000000") (error 'no-darkp2))
    (condition-case nil (color-complement "red") (error 'no-compl)))
   ;; RGB parsing
   (list
    (condition-case nil (color-values "red" t) (error 'no-values-frame))
    (condition-case nil (color-name-to-rgb "black") (error 'no-rgb-black))
    (condition-case nil (color-name-to-rgb "#FFFFFF") (error 'no-rgb-white))
    (condition-case nil (color-name-to-rgb "#FF00FF") (error 'no-rgb-magenta)))))"##,
    );
}

#[test]
fn ft_complex_org_face_link_custom_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "Links: [[https://example.com][HTTPS]] [[file:test.org][FILE]]\n")
    (insert "         [[id:abc123][ID]] [[mailto:a@b.com][MAIL]]\n")
    (insert "         [[*internal][INTERNAL]] [[ftp://server/file][FTP]]\n")
    (font-lock-ensure (point-min) (point-max))
    (mapcar
     (lambda (needle)
       (save-excursion
         (goto-char (point-min))
         (if (search-forward needle nil t)
             (list needle
                   (get-text-property (match-beginning 0) 'face)
                   (get-text-property (match-beginning 0) 'mouse-face)
                   (get-text-property (match-beginning 0) 'help-echo)
                   (get-text-property (match-beginning 0) 'htmlize-link)
                   (get-text-property (match-beginning 0) 'font-lock-face)
                   (get-text-property (match-beginning 0) 'fontified)
                   (keymapp (get-text-property (match-beginning 0) 'keymap))
                   (get-text-property (match-beginning 0) 'invisible))
             (list needle 'not-found nil nil nil nil nil nil nil))))
     '("HTTPS" "FILE" "ID" "MAIL" "INTERNAL" "FTP"))))"##,
    );
}

#[test]
fn ft_complex_face_property_change_intervals_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    ;; Set faces on intervals
    (put-text-property 1 6 'face '(:foreground "red"))
    (put-text-property 6 11 'face '(:foreground "green"))
    (put-text-property 11 16 'face '(:foreground "blue"))
    (put-text-property 16 21 'face '(:foreground "orange"))
    (put-text-property 21 26 'face '(:foreground "purple"))
    (put-text-property 26 31 'face '(:foreground "brown"))
    (list
     ;; Property change positions
     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face)))
             '(1 5 6 10 11 15 16 20 21 25 26 30))
     ;; next-single-property-change
     (mapcar (lambda (start)
               (let ((next (next-single-property-change start 'face)))
                 (list start next)))
             '(1 6 11 16 21 26))
     ;; previous-single-property-change
     (mapcar (lambda (start)
               (let ((prev (previous-single-property-change start 'face)))
                 (list start prev)))
             '(5 10 15 20 25 30))
     ;; Edit: remove middle range and check intervals
     (progn
       (delete-region 11 16)
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face)))
               '(1 5 10 14 18 22 26))))))"##,
    );
}

#[test]
fn ft_complex_face_composition_property_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Composed text with face")
    (put-text-property 1 9 'face 'bold)
    (put-text-property 9 14 'face 'italic)
    (put-text-property 14 24 'face 'underline)
    (list
     ;; Initial
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'composition)
                     (get-text-property pos 'fontified)))
             '(1 5 9 12 14 20))
     ;; Add composition property
     (progn
       (compose-region 1 14 '(?C ?O ?M ?P ?O ?S ?E ?D))
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos
                       (get-text-property pos 'face)
                       (get-text-property pos 'composition)))
               '(1 5 9 12 14 20)))
     ;; Decompose and recheck
     (progn
       (decompose-region 1 14)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face)
                       (get-text-property pos 'composition)))
               '(1 5 9 12 14 20))))))"##,
    );
}

#[test]
fn ft_complex_face_after_narrow_widen_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Alpha\nBody A.\n\n")
      (insert "** DONE Beta\nBody B.\n\n")
      (insert "* TODO Gamma\nBody C.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-text-property (match-beginning 0) 'fontified))
                             (list needle 'not-found nil))))
                     '("TODO" "DONE" "Body A" "Body B" "Body C")))))
        (let ((v0 (funcall snap)))
          ;; Narrow to Alpha subtree
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-narrow-to-subtree)
          (let ((v1 (funcall snap)))
            ;; Widen
            (widen)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Narrow to Gamma
              (goto-char (point-min))
              (search-forward "Gamma")
              (beginning-of-line)
              (org-narrow-to-subtree)
              (let ((v3 (funcall snap)))
                ;; Widen
                (widen)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_complex_face_with_org_indent_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-indent)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-startup-indented t))
      (org-mode)
      (insert "* TODO Alpha\nBody alpha.\n\n")
      (insert "** DONE Beta\nBody beta.\n\n")
      (insert "*** TODO Gamma\nBody gamma.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (mapcar
       (lambda (needle)
         (save-excursion
           (goto-char (point-min))
           (if (search-forward needle nil t)
               (list needle
                     (get-text-property (match-beginning 0) 'face)
                     (get-text-property (match-beginning 0) 'font-lock-face)
                     (get-text-property (match-beginning 0) 'wrap-prefix)
                     (get-text-property (match-beginning 0) 'line-prefix)
                     (invisible-p (match-beginning 0)))
               (list needle 'not-found nil nil nil nil)))
       '("Alpha" "Beta" "Gamma"
         "Body alpha" "Body beta" "Body gamma")))))"##,
    );
}

// ==========================================================
// Very strict, complex multi-layer divergence tests
// ==========================================================

#[test]
fn ft_strict_face_remap_buffer_face_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "Buffer face mode test text here")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 8 17 'face 'italic)
    (let ((snap (lambda ()
                  (list
                   (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 8 12 17))
                   (face-remapping-alist)))))
      (let ((v0 (funcall snap)))
        ;; Apply buffer face remapping
        (condition-case nil
            (face-remap-add-relative 'default '(:height 1.2 :weight bold))
          (error nil))
        (let ((v1 (funcall snap)))
          ;; Remap italic to underline
          (condition-case nil
              (face-remap-add-relative 'italic '(:underline t))
            (error nil))
          (let ((v2 (funcall snap)))
            ;; Remove remaps
            (condition-case nil
                (face-remap-reset-base 'default)
              (error nil))
            (condition-case nil
                (face-remap-reset-base 'italic)
              (error nil))
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_strict_font_lock_add_keywords_custom_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "IMPORTANT: This is a WARNING message with NOTE highlighted.")
    ;; Add custom keywords
    (let ((kwds '(("\\<\\(IMPORTANT\\)\\>" 1 'font-lock-warning-face prepend)
                  ("\\<\\(WARNING\\)\\>" 1 '(:foreground "red" :weight bold) prepend)
                  ("\\<\\(NOTE\\)\\>" 1 '(:foreground "blue" :slant italic) prepend))))
      (font-lock-add-keywords nil kwds)
      (font-lock-fontify-buffer)
      (list
       (mapcar (lambda (needle)
                 (save-excursion
                   (goto-char (point-min))
                   (if (search-forward needle nil t)
                       (list needle
                             (get-text-property (match-beginning 0) 'face)
                             (get-text-property (match-beginning 0) 'fontified)
                             (get-text-property (match-beginning 0) 'font-lock-face))
                       (list needle 'not-found nil nil))))
               '("IMPORTANT" "WARNING" "NOTE" "message" "highlighted"))
       ;; Edit and re-fontify
       (progn
         (goto-char (point-min))
         (search-forward "WARNING")
         (replace-match "CRITICAL")
         (search-forward "NOTE")
         (replace-match "INFO")
         (font-lock-fontify-buffer)
         (mapcar (lambda (needle)
                   (save-excursion
                     (goto-char (point-min))
                     (if (search-forward needle nil t)
                         (list needle
                               (get-text-property (match-beginning 0) 'face))
                         (list needle 'not-found))))
                 '("IMPORTANT" "CRITICAL" "INFO")))))))"##,
    );
}

#[test]
fn ft_strict_face_cache_invalidation_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (newline-and-indent)
  (with-temp-buffer
    (insert "Face caching test - repeated face lookups")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 13 'face 'italic)
    (put-text-property 13 23 'face 'underline)
    (put-text-property 23 36 'face '(:foreground "red"))
    (list
     ;; Multiple reads from same position
     (mapcar (lambda (pos)
               (goto-char pos)
               (let ((f1 (get-text-property pos 'face))
                     (f2 (get-text-property pos 'face))
                     (f3 (get-text-property pos 'face)))
                 (list pos f1 (eq f1 f2) (eq f2 f3))))
             '(1 6 13 23 30))
     ;; Face at prop change boundary
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (let ((next (next-single-property-change pos 'face)))
                       (list 'next-face-at next
                             (get-text-property next 'face)))))
             '(1 5 6 8 13 18 23 28 35))
     ;; Verify text-properties-at returns all props
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos (length (text-properties-at pos))))
             '(1 6 13 23 30)))))"##,
    );
}

#[test]
fn ft_strict_org_emphasis_list_modify_render_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "*bold* /italic/ _under_ +strike+ ~code~ =verb=\n")
    (font-lock-ensure (point-min) (point-max))
    (let ((snap (lambda ()
                  (mapcar (lambda (what pos)
                            (goto-char pos)
                            (list what pos (get-text-property pos 'face)))
                          '("b" "i" "u" "s" "c" "v")
                          '(2 4 6 8 10 12)))))
      (let ((v0 (funcall snap)))
        ;; Modify org-emphasis-alist to change face
        (let ((org-emphasis-alist
               '(("*" bold)
                 ("/" italic)
                 ("_" (:foreground "blue" :underline t))
                 ("+" strike-through)
                 ("~" (:foreground "green"))
                 ("=" (:foreground "purple" :slant italic)))))
          (font-lock-flush)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Restore and re-render
            (let ((org-emphasis-alist
                   '(("*" bold)
                     ("/" italic)
                     ("_" underline)
                     ("+" strike-through)
                     ("~" org-code)
                     ("=" org-verbatim))))
              (font-lock-flush)
              (font-lock-ensure (point-min) (point-max))
              (let ((v2 (funcall snap)))
                (list v0 v1 v2)))))))))"##,
    );
}

#[test]
fn ft_strict_face_text_scale_adjust_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "Text scale adjust test line")
    (put-text-property 1 7 'face 'bold)
    (put-text-property 7 14 'face 'italic)
    (put-text-property 14 24 'face '(:foreground "blue"))
    (let ((snap (lambda ()
                  (list
                   (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 7 10 14 18 22))
                   (face-attribute 'default :height nil 'default-on)))))
      (let ((v0 (funcall snap)))
        ;; Apply text-scale
        (condition-case nil
            (text-scale-increase 2)
          (error nil))
        (let ((v1 (funcall snap)))
          ;; Reset
          (condition-case nil
              (text-scale-set 0)
            (error nil))
          (let ((v2 (funcall snap)))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_strict_face_clone_buffer_face_propagation_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Original\nBody orig.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let* ((original (current-buffer))
             (reader (lambda (buf)
                       (with-current-buffer buf
                         (mapcar (lambda (needle)
                                   (goto-char (point-min))
                                   (if (search-forward needle nil t)
                                       (list needle
                                             (get-text-property (match-beginning 0) 'face)
                                             (get-text-property (match-beginning 0) 'fontified)
                                             (get-text-property (match-beginning 0) 'font-lock-face))
                                       (list needle 'not-found nil nil)))
                                 '("TODO" "DONE" "Body orig"))))))
        (let ((v0 (funcall reader original)))
          ;; Clone indirect buffer
          (let* ((clone-name (generate-new-buffer-name "*clone*"))
                 (clone (make-indirect-buffer original clone-name t)))
            (unwind-protect
                (let ((v1 (funcall reader clone)))
                  ;; Edit in clone: change TODO to DONE
                  (with-current-buffer clone
                    (goto-char (point-min))
                    (search-forward "TODO")
                    (replace-match "DONE")
                    (font-lock-ensure (point-min) (point-max)))
                  (let ((v2 (funcall reader clone))
                        (v2o (funcall reader original)))
                    ;; Edit in original: insert new heading
                    (with-current-buffer original
                      (goto-char (point-max))
                      (insert "* NEXT New heading\nBody new.\n")
                      (font-lock-ensure (point-min) (point-max)))
                    (let ((v3 (funcall reader original))
                          (v3c (funcall reader clone)))
                      (list v0 v1 v2 v2o v3 v3c)))))
              (when (get-buffer clone-name) (kill-buffer clone-name)))))))))"##,
    );
}

#[test]
fn ft_strict_face_multiple_overlay_faces_at_point_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multi overlay face test text zone")
    ;; Base text property
    (put-text-property 1 31 'face '(:foreground "gray"))
    ;; Multiple overlays with different priorities and windows
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face '(:background "red"))
      (overlay-put ov1 'priority 50))
    (let ((ov2 (make-overlay 5 15)))
      (overlay-put ov2 'face '(:foreground "green" :weight bold))
      (overlay-put ov2 'priority 100))
    (let ((ov3 (make-overlay 10 20)))
      (overlay-put ov3 'face '(:foreground "blue"))
      (overlay-put ov3 'priority 75))
    (let ((ov4 (make-overlay 15 25)))
      (overlay-put ov4 'face '(:background "yellow" :foreground "black"))
      (overlay-put ov4 'priority 200))
    (let ((ov5 (make-overlay 20 31)))
      (overlay-put ov5 'face '(:slant italic :weight bold))
      (overlay-put ov5 'priority 25))
    (list
     ;; Face at each position
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-char-property pos 'face)
                     (get-text-property pos 'face)
                     (length (overlays-at pos))))
             '(1 5 8 10 12 15 18 20 22 25 28 30))
     ;; Remove highest priority overlay
     (progn
       (delete-overlay ov4)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(1 5 10 15 18 20 25)))
     ;; Change priority
     (progn
       (overlay-put ov1 'priority 300)
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(1 5 8 10)))
     ;; Add an overlay with nil face (should expose lower priority)
     (progn
       (let ((ov6 (make-overlay 1 31)))
         (overlay-put ov6 'face nil)
         (overlay-put ov6 'priority 999))
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-char-property pos 'face)))
               '(1 5 10 15 20 25 30))))))"##,
    );
}

#[test]
fn ft_strict_face_glyphless_char_display_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (if (fboundp 'glyphless-char-display-control)
        ;; Test glyphless display
        (list
         'glyphless-available
         (glyphless-char-display-control))
      (list 'no-glyphless-control))
    (if (fboundp 'standard-display-table)
        ;; Test display table
        (list
         'display-table-available
         (standard-display-table))
      (list 'no-display-table))
    ;; Test with actual text
    (insert "\x200B\x200C\x200D")
    (font-lock-ensure (point-min) (point-max))
    (mapcar (lambda (pos)
              (goto-char pos)
              (list pos
                    (get-text-property pos 'face)
                    (get-text-property pos 'display)
                    (get-text-property pos 'glyphless-char)))
            '(1 2 3))))"##,
    );
}

#[test]
fn ft_strict_org_face_hide_show_refile_combo_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* TODO Project\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:END:\n")
      (insert "** DONE Task-A\nBody A.\n\n")
      (insert "** TODO Task-B\nBody B.\n\n")
      (insert "** WAIT Task-C\nBody C.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (line-beginning-position) 'face)
                                   (get-text-property (match-beginning 0) 'face)
                                   (invisible-p (match-beginning 0))
                                   (org-outline-level))
                             (list needle 'not-found nil nil nil))))
                     '("Project" "Task-A" "Task-B" "Task-C")))))
        (let ((v0 (funcall snap)))
          ;; Hide all
          (org-fold-hide-all)
          (let ((v1 (funcall snap)))
            ;; Edit: insert Task-D under hidden Project
            (goto-char (point-min))
            (search-forward "Project")
            (end-of-line)
            (insert "\n** TODO Task-D\nBody D.\n")
            (let ((v2 (funcall snap)))
              ;; Show all and refontify
              (org-fold-show-all)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Change Task-B to DONE
                (goto-char (point-min))
                (search-forward "TODO Task-B")
                (replace-match "DONE Task-B")
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  ;; Global cycle
                  (org-global-cycle nil)
                  (let ((v5 (funcall snap)))
                    (org-global-cycle nil)
                    (font-lock-ensure (point-min) (point-max))
                    (let ((v6 (funcall snap)))
                      (list v0 v1 v2 v3 v4 v5 v6)))))))))))))"##,
    );
}

#[test]
fn ft_strict_face_custom_theme_load_unload_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'custom)
  (require 'cus-face)
  (list
   ;; Theme functions
   (list 'custom-available-themes
         (fboundp 'custom-available-themes)
         (condition-case nil
             (and (fboundp 'custom-available-themes)
                  (custom-available-themes))
           (error 'no-available-themes)))
   ;; Load theme
   (condition-case nil
       (progn
         (if (fboundp 'load-theme)
             (let ((before-weight (face-attribute 'default :weight nil 'default-on))
                   (before-slant (face-attribute 'default :slant nil 'default-on)))
               (list 'load-theme-ok
                     before-weight
                     before-slant))
           (list 'no-load-theme)))
     (error 'themes-error))
   ;; face-spec functions
   (list 'face-spec
         (fboundp 'face-spec-set)
         (fboundp 'face-spec-choose)
         (fboundp 'face-spec-recalc)
         (fboundp 'face-spec-match-p))
   ;; Default face attributes after all ops
   (list 'final-attrs
         (face-attribute 'default :family nil 'default-on)
         (face-attribute 'default :foundry nil 'default-on)
         (face-attribute 'default :width nil 'default-on)
         (face-attribute 'default :height nil 'default-on)
         (face-attribute 'default :weight nil 'default-on)
         (face-attribute 'default :slant nil 'default-on)
         (face-attribute 'default :underline nil 'default-on)
         (face-attribute 'default :overline nil 'default-on)
         (face-attribute 'default :strike-through nil 'default-on)
         (face-attribute 'default :box nil 'default-on)
         (face-attribute 'default :inverse-video nil 'default-on)
         (face-attribute 'default :foreground nil 'default-on)
         (face-attribute 'default :background nil 'default-on)
         (face-attribute 'default :stipple nil 'default-on)
         (face-attribute 'default :inherit nil 'default-on))))"##,
    );
}

#[test]
fn ft_strict_face_with_special_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Special text property test buffer content")
    ;; Various special properties
    (put-text-property 1 9 'face 'bold)
    (put-text-property 1 9 'read-only t)
    (put-text-property 9 15 'face 'italic)
    (put-text-property 9 15 'intangible t)
    (put-text-property 15 24 'face 'underline)
    (put-text-property 15 24 'invisible t)
    (put-text-property 24 33 'face '(:foreground "red"))
    (put-text-property 24 33 'field 'test-field)
    (put-text-property 33 40 'face '(:background "yellow"))
    (put-text-property 33 40 'category 'test-cat)
    (list
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'read-only)
                     (get-text-property pos 'intangible)
                     (get-text-property pos 'invisible)
                     (get-text-property pos 'field)
                     (get-text-property pos 'category)
                     (get-text-property pos 'fontified)))
             '(1 5 9 12 15 20 24 28 33 37))
     ;; After removing read-only
     (progn
       (let ((inhibit-read-only t))
         (remove-text-properties 1 9 '(read-only nil face nil))
         (put-text-property 1 9 'face '(:slant italic))
         (mapcar (lambda (pos)
                   (goto-char pos)
                   (list pos
                         (get-text-property pos 'face)
                         (get-text-property pos 'read-only)))
                 '(1 5))))
     ;; After removing invisible
     (progn
       (remove-text-properties 15 24 '(invisible nil))
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face) (get-text-property pos 'invisible)))
                '(15 20))))))"##,
    );
}

#[test]
fn ft_extreme_face_property_interval_split_merge_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXXX")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 8 'face 'italic)
    (put-text-property 8 13 'face 'underline)
    (let ((snap (lambda ()
                  (mapcar (lambda (pos)
                            (goto-char pos)
                            (list pos (get-text-property pos 'face)))
                          '(1 3 5 7 9 12)))))
      (let ((v0 (funcall snap)))
        ;; Insert at boundary to split interval
        (goto-char 4)
        (insert "YY")
        (let ((v1 (funcall snap)))
          ;; Delete to merge intervals
          (delete-region 8 10)
          (let ((v2 (funcall snap)))
            ;; Insert interior
            (goto-char 6)
            (insert "ZZZ")
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_extreme_face_substring_and_copy_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJ")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 7 'face 'italic)
    (put-text-property 7 11 'face 'underline)
    (list
     ;; Substring with properties
     (progn
       (let ((sub (buffer-substring 1 6)))
         (with-temp-buffer
           (insert sub)
           (list 'substring-props
                 (mapcar (lambda (pos)
                           (goto-char pos)
                           (list pos (get-text-property pos 'face)))
                         '(1 3 5))))))
     ;; substring-no-properties
     (list 'no-props (buffer-substring-no-properties 1 6))
     ;; Buffer copy
     (progn
       (let ((copy (buffer-substring 1 11)))
         (with-temp-buffer
           (insert copy)
           (mapcar (lambda (pos)
                     (goto-char pos)
                     (list pos (get-text-property pos 'face)))
                   '(1 4 7)))))
     ;; buffer-string
     (list 'buffer-string (length (buffer-string))))))"##,
    );
}

#[test]
fn ft_extreme_face_multibyte_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    ;; Unicode characters
    (insert "αβγδεζηθικλμν")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 14 'face 'underline)
    (list
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (char-after pos)
                     (get-text-property pos 'face)
                     (char-width (char-after pos))))
             '(1 3 5 7 9 11 13))
     ;; Property change positions
     (list 'prop-changes
           (next-single-property-change 1 'face)
           (next-single-property-change 5 'face)
           (previous-single-property-change 8 'face))
     ;; After edit
     (progn
       (goto-char 5)
       (insert "ΩΣ")
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face)))
               '(1 3 5 7 9 11))))))"##,
    );
}

#[test]
fn ft_extreme_face_font_lock_remove_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "This is IMPORTANT and CRITICAL with WARNING notes.")
    ;; Add keywords
    (let ((kwds1 '(("\\<\\(IMPORTANT\\)\\>" 1 font-lock-warning-face t)
                   ("\\<\\(CRITICAL\\)\\>" 1 '(:foreground "red" :weight bold) t))))
      (font-lock-add-keywords nil kwds1)
      (font-lock-fontify-buffer)
      (let ((v0 (mapcar (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (if (search-forward needle nil t)
                                (list needle (get-text-property (match-beginning 0) 'face))
                                (list needle 'not-found))))
                        '("IMPORTANT" "CRITICAL" "WARNING"))))
        ;; Remove IMPORTANT keyword
        (font-lock-remove-keywords nil
                                   '(("\\<\\(IMPORTANT\\)\\>" 1 font-lock-warning-face t)))
        (font-lock-fontify-buffer)
        (let ((v1 (mapcar (lambda (needle)
                            (save-excursion
                              (goto-char (point-min))
                              (if (search-forward needle nil t)
                                  (list needle (get-text-property (match-beginning 0) 'face))
                                  (list needle 'not-found))))
                          '("IMPORTANT" "CRITICAL" "WARNING"))))
          ;; Add different keywords
          (font-lock-add-keywords nil
                                  '(("\\<\\(WARNING\\)\\>" 1 '(:foreground "orange" :slant italic) t)))
          (font-lock-fontify-buffer)
          (let ((v2 (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (match-beginning 0) 'face))
                                    (list needle 'not-found))))
                            '("IMPORTANT" "CRITICAL" "WARNING"))))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_extreme_face_kill_yank_face_propagation_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Source\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Kill the heading with properties
      (goto-char (point-min))
      (kill-region (point) (progn (forward-line 390) (point)))
      ;; Yank into new buffer
      (with-temp-buffer
        (org-mode)
        (yank)
        (font-lock-ensure (point-min) (point-max))
        (list 'yanked-faces
              (mapcar (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (if (search-forward needle nil t)
                              (list needle
                                    (get-text-property (match-beginning 0) 'face)
                                    (get-text-property (match-beginning 0) 'fontified))
                              (list needle 'not-found nil))))
                      '("TODO" "DONE" "Body")))
        ;; Yank again
        (yank)
        (font-lock-ensure (point-min) (point-max))
        (list 'double-yank
              (mapcar (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (if (search-forward needle nil t)
                              (list needle (get-text-property (match-beginning 0) 'face))
                              (list needle 'not-found))))
                      '("TODO" "Body"))))))))"##,
    );
}

#[test]
fn ft_extreme_face_org_headline_level_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-faces)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* Level-1 heading\nBody 1.\n\n")
      (insert "** Level-2 heading\nBody 2.\n\n")
      (insert "*** Level-3 heading\nBody 3.\n\n")
      (insert "**** Level-4 heading\nBody 4.\n\n")
      (insert "***** Level-5 heading\nBody 5.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (list
       ;; Level faces
       (mapcar (lambda (needle)
                 (save-excursion
                   (goto-char (point-min))
                   (if (search-forward needle nil t)
                       (list needle
                             (org-outline-level)
                             (get-text-property (line-beginning-position) 'face)
                             (face-attribute (get-text-property (line-beginning-position) 'face) :foreground nil 'default-on))
                       (list needle 'not-found nil nil nil))))
               '("Level-1" "Level-2" "Level-3" "Level-4" "Level-5"))
       ;; org-level-N face existence check
       (mapcar (lambda (level)
                 (let ((face (intern (format "org-level-%d" level))))
                   (list level face (condition-case nil (facep face) (error 'no-face)))))
               '(1 2 3 4 5 6 7 8)))))"##,
    );
}

#[test]
fn ft_extreme_face_buffer_swap_text_property_transfer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((buf1 (generate-new-buffer "*ft-buf1*"))
        (buf2 (generate-new-buffer "*ft-buf2*")))
    (unwind-protect
        (progn
          ;; Populate buf1 with org content
          (with-current-buffer buf1
            (org-mode)
            (insert "* TODO Buf1-heading\nBody in buf1.\n\n")
            (font-lock-ensure (point-min) (point-max)))
          ;; Populate buf2 with faces
          (with-current-buffer buf2
            (insert "Buf2 content with faces")
            (put-text-property 1 6 'face 'bold)
            (put-text-property 6 14 'face 'italic)
            (put-text-property 14 25 'face 'underline))
          (let ((snap (lambda (buf)
                        (with-current-buffer buf
                          (mapcar (lambda (pos)
                                    (goto-char pos)
                                    (list pos (get-text-property pos 'face)))
                                  '(1 5 10 15))))))
            (let ((before-buf1 (funcall snap buf1))
                  (before-buf2 (funcall snap buf2)))
              ;; Buffer swap
              (let ((buf3 buf1))
                (setq buf1 buf2 buf2 buf3))
              (let ((after-buf1 (funcall snap buf1))
                    (after-buf2 (funcall snap buf2)))
                (list before-buf1 before-buf2 after-buf1 after-buf2))))))
      (kill-buffer buf1)
      (kill-buffer buf2))))"##,
    );
}

#[test]
fn ft_extreme_face_org_table_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "| Name | Val1 | Val2 |\n")
    (insert "|------+------+------|\n")
    (insert "| A | 100 | 200 |\n")
    (insert "| B | 300 | 400 |\n")
    (font-lock-ensure (point-min) (point-max))
    (mapcar
     (lambda (needle)
       (save-excursion
         (goto-char (point-min))
         (if (search-forward needle nil t)
             (list needle
                   (get-text-property (match-beginning 0) 'face)
                   (get-text-property (match-beginning 0) 'font-lock-face))
             (list needle 'not-found nil))))
     '("Name" "|------" "| A |" "| B |"))))"##,
    );
}

#[test]
fn ft_extreme_face_add_face_text_property_multiple_calls_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (insert "Multi-face-text-property-test")
    ;; Add faces one by one
    (add-face-text-property 1 30 '(:foreground "blue"))
    (add-face-text-property 1 15 '(:weight bold))
    (add-face-text-property 10 25 '(:slant italic))
    (add-face-text-property 20 30 '(:underline t))
    (add-face-text-property 1 30 '(:height 1.2))
    (list
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'font-lock-face)))
             '(1 5 10 12 15 18 20 22 25 28))
     ;; Remove some faces
     (progn
       (remove-text-properties 1 15 '(face nil))
       (add-face-text-property 1 15 '(:foreground "red" :background "yellow"))
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face)))
               '(1 5 10 15 20 25)))
     ;; Clear all face text properties
     (progn
       (remove-text-properties 1 30 '(face nil))
       ;; Re-add single property
       (add-face-text-property 1 30 '(:foreground "green" :weight bold :slant italic))
       (mapcar (lambda (pos)
                 (goto-char pos)
                 (list pos (get-text-property pos 'face)))
               '(1 10 20 25))))))"##,
    );
}

#[test]
fn ft_extreme_org_face_cycle_edit_show_hide_repeat_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* TODO Root :root:\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:CATEGORY: core\n:END:\n")
      (insert "** DONE Leaf-A :fe:\nBody A.\n\n")
      (insert "** TODO Leaf-B :be:\nBody B.\n\n")
      (insert "** WAIT Leaf-C :ops:\nBody C.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar
                     (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (line-beginning-position) 'face)
                                   (get-text-property (match-beginning 0) 'face)
                                   (invisible-p (match-beginning 0))
                                   (org-outline-level)
                                   (org-get-tags nil t))
                             (list needle 'not-found nil nil nil nil nil))))
                     '("Root" "Leaf-A" "Leaf-B" "Leaf-C")))))
        (let ((v0 (funcall snap)))
          ;; Cycle Root: overview
          (goto-char (point-min))
          (search-forward "Root :root:")
          (beginning-of-line)
          (org-cycle nil)
          (let ((v1 (funcall snap)))
            ;; Edit: insert Leaf-D under hidden Root
            (end-of-line)
            (insert "\n** TODO Leaf-D :qa:\nBody D.\n")
            (let ((v2 (funcall snap)))
              ;; Cycle Root: children
              (goto-char (point-min))
              (search-forward "Root :root:")
              (beginning-of-line)
              (org-cycle nil)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Hide all drawers
                (goto-char (point-min))
                (search-forward "PROPERTIES")
                (beginning-of-line)
                (org-fold-hide-drawer-all)
                (let ((v4 (funcall snap)))
                  ;; Show all
                  (org-fold-show-all)
                  (font-lock-ensure (point-min) (point-max))
                  (let ((v5 (funcall snap)))
                    ;; Global cycle
                    (org-global-cycle nil)
                    (let ((v6 (funcall snap)))
                      (org-global-cycle nil)
                      (font-lock-ensure (point-min) (point-max))
                      (let ((v7 (funcall snap)))
                        (list v0 v1 v2 v3 v4 v5 v6 v7)))))))))))))))"##,
    );
}

#[test]
fn ft_yotta_face_with_undo_and_redo_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Undo redo face test")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 14 'face 'underline)
    (put-text-property 14 19 'face '(:foreground "red"))
    (let ((snap (lambda ()
                  (mapcar (lambda (pos)
                            (goto-char pos)
                            (list pos (get-text-property pos 'face)))
                          '(1 3 5 7 9 12 14 17)))))
      (let ((v0 (funcall snap)))
        ;; Edit: change some text
        (goto-char 9)
        (insert " NEW")
        (let ((v1 (funcall snap)))
          ;; Undo
          (undo)
          (let ((v2 (funcall snap)))
            ;; Redo
            (condition-case nil (and (fboundp 'redo) (redo)) (error nil))
            (let ((v3 (funcall snap)))
              ;; Another undo
              (undo)
              (let ((v4 (funcall snap)))
                (list v0 v1 v2 v3 v4)))))))))"##,
    );
}

#[test]
fn ft_yotta_face_empty_and_single_char_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    ;; Empty buffer
    (let ((empty-faces (text-properties-at 1)))
      ;; Single char buffer
      (insert "X")
      (put-text-property 1 2 'face 'bold)
      (let ((single-face (get-text-property 1 'face)))
        ;; Two chars: different faces
        (goto-char 2)
        (insert "Y")
        (put-text-property 2 3 'face 'italic)
        (list
         'empty empty-faces
         'single single-face
         'two-chars (mapcar (lambda (pos) (list pos (get-text-property pos 'face))) '(1 2))
         'property-boundaries (list (next-single-property-change 1 'face)
                                     (next-single-property-change 2 'face))
         'no-boundary (next-single-property-change 3 'face))))))"##,
    );
}

#[test]
fn ft_yotta_face_very_long_text_property_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    ;; Create a long chain of alternating face properties
    (dotimes (i 20)
      (insert (make-string 5 (+ ?A i))))
    (let ((colors '("red" "green" "blue" "orange" "purple" "brown" "cyan" "magenta" "olive" "navy")))
      (let ((i 0))
        (while (< i 100)
          (let ((fi (nth (mod i (length colors)) colors)))
            (put-text-property (1+ i) (+ i 6) 'face (list :foreground fi :weight 'bold)))
          (setq i (+ i 5))))
      (list
       ;; Spot checks
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 20 30 50 75 95))
       ;; Property intervals
       (list 'intervals (length (object-intervals (current-buffer))))
       ;; Next property changes
       (mapcar (lambda (pos) (list pos (next-single-property-change pos 'face))) '(1 6 11 25 50))))))"##,
    );
}

#[test]
fn ft_yotta_face_property_copy_via_buffer_substring_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Copy-source\nBody content.\n\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Capture rendered faces in source
      (let ((source-faces
             (mapcar (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-text-property (match-beginning 0) 'fontified))
                             (list needle 'not-found nil))))
                     '("TODO" "DONE" "Copy-source" "Body content"))))
        ;; Copy text to another buffer
        (let ((buf-content (buffer-substring (point-min) (point-max))))
          (with-temp-buffer
            (org-mode)
            (insert buf-content)
            (font-lock-ensure (point-min) (point-max))
            (list
             'source-faces source-faces
             'target-faces
             (mapcar (lambda (needle)
                       (save-excursion
                         (goto-char (point-min))
                         (if (search-forward needle nil t)
                             (list needle
                                   (get-text-property (match-beginning 0) 'face)
                                   (get-text-property (match-beginning 0) 'fontified))
                             (list needle 'not-found nil))))
                     '("TODO" "DONE" "Copy-source" "Body content")))))))))"##,
    );
}

#[test]
fn ft_yotta_face_with_eieio_object_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'eieio)
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO EIEIO face test\nBody.\n\n")
    (font-lock-ensure (point-min) (point-max))
    ;; Mix EIEIO objects as text properties
    (condition-case nil
        (let ((obj (list :name "test-obj" :value 42)))
          (put-text-property 1 5 'eieio-object obj)
          (put-text-property 1 5 'face 'bold)
          (list 'with-eieio
                (mapcar (lambda (pos)
                          (goto-char pos)
                          (list pos
                                (get-text-property pos 'face)
                                (get-text-property pos 'eieio-object)
                                (get-text-property pos 'fontified)))
                        '(1 3 5))
                'eieio-props (text-properties-at 1)))
      (error (list 'eieio-error
                   (fboundp 'eieio-oref)
                   (facep 'bold)
                   (get-text-property 1 'face)
                   (text-properties-at 1)))))))"##,
    );
}

#[test]
fn ft_yotta_face_with_buffer_local_variable_affects_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'face-remap)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Buffer-local-test\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (save-excursion
                      (goto-char (point-min))
                      (search-forward "TODO")
                      (list 'todo-face (get-text-property (match-beginning 0) 'face)
                            'todo-line-face (get-text-property (line-beginning-position) 'face))))))
        (let ((v0 (funcall snap)))
          ;; Toggle org-fontify-todo-headline
          (setq-local org-fontify-todo-headline nil)
          (font-lock-flush)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Re-enable
            (setq-local org-fontify-todo-headline t)
            (font-lock-flush)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Toggle org-fontify-whole-heading-line
              (setq-local org-fontify-whole-heading-line nil)
              (font-lock-flush)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Re-enable
                (setq-local org-fontify-whole-heading-line t)
                (font-lock-flush)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_zeta_face_lazy_font_lock_deferred_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'jit-lock)
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Lazy fontify\nBody lazy.\n\n")
      (insert "** DONE More lazy\nBody more.\n\n")
      (list
       'before-fontify (mapcar (lambda (needle)
                                 (save-excursion
                                   (goto-char (point-min))
                                   (if (search-forward needle nil t)
                                       (list needle
                                             (get-text-property (match-beginning 0) 'face)
                                             (get-text-property (match-beginning 0) 'fontified)
                                             (get-text-property (match-beginning 0) 'font-lock-face))
                                       (list needle 'not-found nil nil))))
                               '("TODO" "DONE" "Body lazy" "Body more"))
       'after-fontify (progn
                        (font-lock-fontify-buffer)
                        (mapcar (lambda (needle)
                                  (save-excursion
                                    (goto-char (point-min))
                                    (if (search-forward needle nil t)
                                        (list needle (get-text-property (match-beginning 0) 'face))
                                        (list needle 'not-found))))
                                '("TODO" "DONE" "Body lazy" "Body more")))
       (list 'jit-lock-mode (if (boundp 'jit-lock-mode) jit-lock-mode 'no-jit-lock))
       (list 'font-lock-mode font-lock-mode)))))"##,
    );
}

#[test]
fn ft_zeta_face_org_set_delete_property_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Alpha\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (match-beginning 0) 'face))
                                    (list needle 'not-found))))
                            '("TODO" ":PROPERTIES:" ":Effort:" ":END:")))))
        (let ((v0 (funcall snap)))
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-set-property "Effort" "3h")
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            (org-set-property "Owner" "Alice")
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              (org-delete-property "Effort")
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_zeta_face_display_graphic_conditional_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Display-dependent face test")
    (put-text-property 1 30 'face (list :foreground (if (display-graphic-p) "#FF0000" "#00FF00")
                                         :weight (if (display-graphic-p) 'bold 'normal)))
    (list
     'display-graphic-p (display-graphic-p)
     'face-value (get-text-property 1 'face)
     (condition-case nil (face-attribute 'default :family nil t) (error 'no-frame-family))
     (condition-case nil (face-attribute 'default :foreground) (error 'no-fg))
     (condition-case nil (face-attribute 'default :background) (error 'no-bg))
     (list 'display-planes (if (fboundp 'display-planes) (display-planes) 'no))
     (list 'display-color-cells (if (fboundp 'display-color-cells) (display-color-cells) 'no))
     (list 'display-grayscale-p (if (fboundp 'display-grayscale-p) (display-grayscale-p) 'no)))))"##,
    );
}

#[test]
fn ft_zeta_face_multi_buffer_faces_consistent_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((content "* TODO Test\nBody.\n\n")
        (results nil))
    (dotimes (i 3)
      (let ((buf (generate-new-buffer (format "*ft-multi-%d*" i))))
        (with-current-buffer buf
          (let ((org-fontify-whole-heading-line t)
                (org-fontify-done-headline t))
            (org-mode)
            (insert content)
            (font-lock-ensure (point-min) (point-max))))
        (push (cons i (with-current-buffer buf
                        (mapcar (lambda (needle)
                                  (save-excursion
                                    (goto-char (point-min))
                                    (if (search-forward needle nil t)
                                        (list needle (get-text-property (match-beginning 0) 'face))
                                        (list needle 'not-found))))
                                '("TODO" "Body"))))
              results)
        (kill-buffer buf)))
    (nreverse results)))"##,
    );
}

#[test]
fn ft_zeta_face_link_appear_hide_show_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Appear [[https://example.com][click me]]\n")
      (insert "Body with /italic/ and *bold*.\n\n")
      (insert "** DONE More [[file:test.org][file here]]\n")
      (insert "Body 2.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle
                                          (get-text-property (match-beginning 0) 'face)
                                          (get-text-property (match-beginning 0) 'mouse-face)
                                          (get-text-property (match-beginning 0) 'font-lock-face))
                                    (list needle 'not-found nil nil))))
                            '("TODO" "DONE" "click me" "file here" "italic" "bold")))))
        (let ((v0 (funcall snap)))
          (org-fold-hide-all)
          (let ((v1 (funcall snap)))
            (org-fold-show-all)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              (org-fold-hide-drawer-all)
              (let ((v3 (funcall snap)))
                (org-fold-show-all)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_hunt_overlay_face_evaporate_empty_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Evaporate overlay test buffer here")
    (put-text-property 1 31 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'evaporate t))
    (let ((ov2 (make-overlay 15 25)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold))
      (overlay-put ov2 'evaporate nil))
    (list
     'before-edit (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 8 15 20 24 30))
     ;; Delete evaporating overlay region
     'after-delete (progn
                     (delete-region 1 10)
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 10 15 20)))
     ;; Insert text that should let non-evaporating overlay persist
     'after-insert (progn
                     (goto-char 10)
                     (insert " INSERTED ")
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 15 20 25))))))"##,
    );
}

#[test]
fn ft_hunt_font_lock_mode_toggle_face_persistence_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Font toggle test\nBody font test.\n\n")
      (insert "** DONE Font done\nBody done.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (match-beginning 0) 'face))
                                    (list needle 'not-found))))
                            '("TODO" "DONE" "Body font test" "Body done")))))
        (let ((v0 (funcall snap)))
          ;; Toggle font-lock-mode off
          (font-lock-mode -1)
          (let ((v1 (funcall snap)))
            ;; Toggle font-lock-mode back on
            (font-lock-mode 1)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Toggle off again
              (font-lock-mode -1)
              ;; Insert new content while off
              (goto-char (point-max))
              (insert "* WAIT New while off\nBody new.\n\n")
              ;; Turn on
              (font-lock-mode 1)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3
                      (save-excursion
                        (goto-char (point-min))
                        (if (search-forward "WAIT" nil t)
                            (get-text-property (match-beginning 0) 'face)
                            'not-found))))))))))))"##,
    );
}

#[test]
fn ft_hunt_org_face_after_priority_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Alpha\nBody.\n\n")
      (insert "* TODO Beta\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle
                                          (get-text-property (line-beginning-position) 'face)
                                          (org-get-priority (point) 'force))
                                    (list needle 'not-found nil))))
                            '("Alpha" "Beta")))))
        (let ((v0 (funcall snap)))
          ;; Set priority on Alpha
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-priority ?A)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Set priority on Beta
            (goto-char (point-min))
            (search-forward "Beta")
            (beginning-of-line)
            (org-priority ?C)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Change Alpha priority
              (goto-char (point-min))
              (search-forward "Alpha")
              (beginning-of-line)
              (org-priority ?B)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_hunt_face_after_kill_line_property_shift_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Line1 with face property here\n")
    (insert "Line2 with different face here\n")
    (insert "Line3 with another face\n")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 12 'face 'italic)
    (put-text-property 30 38 'face 'underline)
    (put-text-property 45 55 'face '(:foreground "red"))
    (let ((snap (lambda ()
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face)))
                          '(1 5 8 30 35 45 50)))))
      (let ((v0 (funcall snap)))
        ;; Kill line 1 - properties should shift
        (goto-char 1)
        (kill-line)
        (let ((v1 (funcall snap)))
          ;; Kill line 2
          (goto-char (point-min))
          (kill-line)
          (let ((v2 (funcall snap)))
            ;; Yank killed text
            (goto-char (point-max))
            (yank)
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_hunt_org_face_todo_cycle_face_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "#+TODO: TODO(t) NEXT(n) WAIT(w) | DONE(d) CANCEL(c)\n")
      (insert "* TODO Alpha\nBody.\n\n")
      (insert "* NEXT Beta\nBody.\n\n")
      (insert "* WAIT Gamma\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle
                                          (get-text-property (match-beginning 0) 'face)
                                          (get-text-property (line-beginning-position) 'face)
                                          (org-get-todo-state))
                                    (list needle 'not-found nil nil))))
                            '("TODO" "NEXT" "WAIT" "DONE")))))
        (let ((v0 (funcall snap)))
          ;; Cycle Alpha: TODO -> DONE
          (goto-char (point-min))
          (search-forward "TODO Alpha")
          (beginning-of-line)
          (org-todo 'done)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Cycle Beta: NEXT -> DONE
            (goto-char (point-min))
            (search-forward "NEXT Beta")
            (beginning-of-line)
            (org-todo 'done)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Cycle Gamma: WAIT -> TODO
              (goto-char (point-min))
              (search-forward "WAIT Gamma")
              (beginning-of-line)
              (org-todo 'todo)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_hunt_face_after_replace_string_property_update_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (let ((snap (lambda ()
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face)))
                          '(1 5 8 11 14 16 19 21 24)))))
      (let ((v0 (funcall snap)))
        ;; Replace BBBBB with XXXX (same length)
        (goto-char (point-min))
        (search-forward "BBBBB")
        (replace-match "XXXXX")
        (let ((v1 (funcall snap)))
          ;; Replace CCCCC with YY (shorter)
          (search-forward "CCCCC")
          (replace-match "YY")
          (let ((v2 (funcall snap)))
            ;; Replace DDDDD with ZZZZZZZZZ (longer)
            (search-forward "DDDDD")
            (replace-match "ZZZZZZZZZ")
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_hunt_face_multiple_textprop_stack_order_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multi text property stack test area")
    ;; Add properties in specific order
    (put-text-property 1 11 'face 'bold)
    (put-text-property 1 11 'font-lock-face 'italic)
    (put-text-property 11 22 'font-lock-face 'underline)
    (put-text-property 11 22 'face '(:foreground "blue"))
    (put-text-property 22 31 'face '(:background "gray"))
    (list
     'with-font-lock-face
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-text-property pos 'font-lock-face)))
             '(1 5 11 15 22 28))
     ;; Remove font-lock-face and check face remains
     'after-remove-lock-face
     (progn
       (remove-text-properties 1 22 '(font-lock-face nil))
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 11 15 22 28)))
     ;; Put font-lock-face back differently
     'after-re-add
     (progn
       (put-text-property 1 31 'font-lock-face '(:slant italic :weight bold))
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 15 22 28))))))"##,
    );
}

#[test]
fn ft_hunt_org_face_with_scheduled_deadline_combo_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-todo-headline t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Alpha\n")
      (insert "SCHEDULED: <2026-05-28 Wed>\n")
      (insert "Body alpha.\n\n")
      (insert "* DONE Beta\n")
      (insert "DEADLINE: <2026-06-01 Mon>\n")
      (insert "CLOSED: [2026-05-27 Tue 14:00]\n")
      (insert "Body beta.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (mapcar
       (lambda (needle)
         (save-excursion
           (goto-char (point-min))
           (if (search-forward needle nil t)
               (list needle
                     (get-text-property (match-beginning 0) 'face)
                     (get-text-property (match-beginning 0) 'font-lock-face))
               (list needle 'not-found nil))))
       '("SCHEDULED" "DEADLINE" "CLOSED"
         "<2026-05-28 Wed>" "<2026-06-01 Mon>" "[2026-05-27 Tue")))))"##,
    );
}

#[test]
fn ft_hunt_face_with_face_filters_from_overlays_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Face filter via overlays test text here now")
    (put-text-property 1 12 'face 'bold)
    (put-text-property 12 23 'face 'italic)
    (put-text-property 23 36 'face 'underline)
    ;; Overlay with face-remapping
    (let ((ov1 (make-overlay 1 36)))
      (overlay-put ov1 'face '(:filtered (:window t) (:foreground "blue")))
      (list
       'with-filtered (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-text-property pos 'face))) '(1 10 20 30))
       ;; Remove overlay
       'after-overlay-removal (progn
                                (delete-overlay ov1)
                                (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 20 30)))
       ;; Re-add simpler overlay
       'after-simple-overlay (progn
                               (let ((ov2 (make-overlay 5 30)))
                                 (overlay-put ov2 'face '(:inverse-video t)))
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 15 25 35)))))))"##,
    );
}

#[test]
fn ft_hunt_org_face_multiple_cycle_edit_cycle_edit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t)
          (org-cycle-level-faces t))
      (org-mode)
      (insert "* TODO Cycle-Test :test:\n")
      (insert ":PROPERTIES:\n:Effort: 2h\n:END:\n")
      (insert "** DONE Sub-A :fe:\nBody A.\n\n")
      (insert "** TODO Sub-B :be:\nBody B.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle
                                          (get-text-property (line-beginning-position) 'face)
                                          (invisible-p (match-beginning 0))
                                          (org-outline-level)
                                          (org-get-tags nil t))
                                    (list needle 'not-found nil nil nil nil))))
                            '("Cycle-Test" "Sub-A" "Sub-B")))))
        (let ((v0 (funcall snap)))
          ;; Cycle: overview
          (goto-char (point-min))
          (search-forward "Cycle-Test :test:")
          (beginning-of-line)
          (org-cycle nil)
          (let ((v1 (funcall snap)))
            ;; Edit: insert Sub-C
            (end-of-line)
            (insert "\n** WAIT Sub-C :ops:\nBody C.\n")
            ;; Cycle: children
            (goto-char (point-min))
            (search-forward "Cycle-Test :test:")
            (beginning-of-line)
            (org-cycle nil)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Edit: change Sub-A to TODO
              (goto-char (point-min))
              (search-forward "DONE Sub-A")
              (replace-match "TODO Sub-A")
              (font-lock-ensure (point-min) (point-max))
              ;; Cycle: overview
              (goto-char (point-min))
              (search-forward "Cycle-Test :test:")
              (beginning-of-line)
              (org-cycle nil)
              (let ((v3 (funcall snap)))
                ;; Cycle: children
                (org-cycle nil)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  ;; Global cycle
                  (org-global-cycle nil)
                  (let ((v5 (funcall snap)))
                    (org-global-cycle nil)
                    (font-lock-ensure (point-min) (point-max))
                    (let ((v6 (funcall snap)))
                      (list v0 v1 v2 v3 v4 v5 v6))))))))))))))"##,
    );
}

#[test]
fn ft_probe_face_read_only_buffer_ops_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Read only face test buffer text")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 10 20 'face 'italic)
    (put-text-property 20 31 'face 'underline)
    ;; Check faces before read-only
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25))))
      ;; Set read-only
      (put-text-property 1 31 'read-only t)
      ;; Check faces still accessible
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'read-only))) '(1 5 10 15 20 25))))
        ;; Try to modify with inhibit-read-only
        (let ((inhibit-read-only t))
          (put-text-property 10 20 'face '(:foreground "red" :weight bold))
          (list v0 v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25)))))))))"##,
    );
}

#[test]
fn ft_probe_face_text_property_at_eob_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "End of buffer face test")
    (let ((eob (point-max)))
      (put-text-property 1 eob 'face 'bold)
      ;; Check at EOB and beyond
      (list
       'at-eob (text-properties-at eob)
       'at-1-before-eob (text-properties-at (1- eob))
       'face-at-eob (get-text-property eob 'face)
       'face-at-before (get-text-property (1- eob) 'face)
       ;; Insert at EOB - check face propagation
       (progn
         (goto-char eob)
         (insert " APPENDED")
         (list 'after-append
               (mapcar (lambda (pos) (list pos (get-text-property pos 'face))) (list (1- eob) eob (1+ eob) (point-max)))))))))"##,
    );
}

#[test]
fn ft_probe_face_with_invisible_text_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Visible HIDDEN Visible")
    (put-text-property 1 9 'face 'bold)
    (put-text-property 9 15 'face 'italic)
    (put-text-property 9 15 'invisible t)
    (put-text-property 15 23 'face 'underline)
    (list
     'with-invisible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible) (invisible-p pos))) '(1 5 9 12 15 20))
     ;; Remove invisible
     'after-remove-invisible (progn
                               (remove-text-properties 9 15 '(invisible nil))
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5 9 12 15 20)))
     ;; Re-add invisible and check
     'after-re-add-invisible (progn
                               (put-text-property 9 15 'invisible t)
                               (put-text-property 9 15 'face '(:foreground "red"))
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5 9 12 15 20))))))"##,
    );
}

#[test]
fn ft_probe_face_char_property_vs_text_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Char vs text property comparison")
    (put-text-property 1 8 'face 'bold)
    (let ((ov (make-overlay 8 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'priority 100))
    (let ((ov2 (make-overlay 14 24)))
      (overlay-put ov2 'face 'underline)
      (overlay-put ov2 'priority 50))
    (put-text-property 24 34 'face '(:foreground "blue"))
    (list
     (mapcar (lambda (pos)
               (goto-char pos)
               (list pos
                     (get-text-property pos 'face)
                     (get-char-property pos 'face)
                     (get-char-property-and-overlay pos 'face)))
             '(1 5 8 10 12 14 18 24 28))
     ;; After modifying overlay
     (progn
       (overlay-put ov 'face '(:foreground "green" :weight bold))
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(8 10 12)))
     ;; After deleting overlay
     (progn
       (delete-overlay ov)
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-text-property pos 'face))) '(8 10 12 14))))))"##,
    );
}

#[test]
fn ft_probe_face_with_field_text_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Field-A Field-B Field-C")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 1 8 'field 'field-a)
    (put-text-property 9 16 'face 'italic)
    (put-text-property 9 16 'field 'field-b)
    (put-text-property 17 24 'face 'underline)
    (put-text-property 17 24 'field 'field-c)
    (list
     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'field) (get-char-property pos 'field))) '(1 5 9 12 17 20))
     ;; Field boundaries
     (list 'field-beg (field-beginning) 'field-end (field-end))
     ;; Move to next field
     (progn
       (goto-char 1)
       (list 'field-at-1 (get-text-property (point) 'field)
             'field-after-move (progn (goto-char (field-end)) (get-text-property (point) 'field)))))
    ;; After clearing field
    (progn
      (remove-text-properties 9 16 '(field nil))
      (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'field))) '(9 12))))))"##,
    );
}

#[test]
fn ft_probe_org_face_after_insert_subtree_and_refontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Root :root:\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:END:\n")
      (insert "** DONE T1\nBody T1.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (line-beginning-position) 'face) (invisible-p (match-beginning 0)))
                                    (list needle 'not-found nil))))
                            '("Root" "T1" "T2" "T3")))))
        (let ((v0 (funcall snap)))
          ;; Hide all
          (org-fold-hide-all)
          ;; Insert T2 and T3 under hidden Root
          (goto-char (point-min))
          (search-forward "Root")
          (end-of-line)
          (insert "\n** TODO T2 :fe:\nBody T2.\n** WAIT T3 :ops:\nBody T3.\n")
          ;; Show all and fontify
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Global cycle
            (org-global-cycle nil)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              (list v0 v1 v2)))))))))"##,
    );
}

#[test]
fn ft_probe_face_with_overlay_before_after_string_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with before/after strings")
    (put-text-property 1 35 'face 'bold)
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face 'italic)
      (overlay-put ov1 'before-string "[[")
      (overlay-put ov1 'after-string "]]"))
    (let ((ov2 (make-overlay 15 25)))
      (overlay-put ov2 'face 'underline)
      (overlay-put ov2 'before-string "{{")
      (overlay-put ov2 'after-string "}}"))
    (list
     (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25))
     ;; after-string face check
     (list 'ov1-before-face (overlay-get ov1 'before-string)
           'ov2-after-face (overlay-get ov2 'after-string))
     ;; Modify overlays
     (progn
       (overlay-put ov1 'face '(:foreground "red"))
       (overlay-put ov2 'face '(:background "yellow"))
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25))))))"##,
    );
}

#[test]
fn ft_probe_face_with_text_property_not_all_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJ")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 7 'face 'italic)
    (put-text-property 7 11 'face 'underline)
    ;; Add a property NOT on ALL chars
    (put-text-property 1 11 'my-prop "all")
    (remove-text-properties 4 7 '(my-prop nil))
    (list
     'face-all (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 8 10))
     'my-prop-gaps (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'my-prop))) '(1 3 5 6 8 10))
     'next-prop-change (mapcar (lambda (pos) (list pos (next-single-property-change pos 'my-prop))) '(1 5 8))
     'text-props (text-properties-at 1)
     'text-props-with-gap (text-properties-at 5)))))"##,
    );
}

#[test]
fn ft_probe_org_face_with_org_adapt_indentation_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-indent)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-adapt-indentation t))
      (org-mode)
      (insert "* TODO Indent test\nBody here.\n\n")
      (insert "** DONE Sub indent\nBody sub here.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (mapcar
       (lambda (needle)
         (save-excursion
           (goto-char (point-min))
           (if (search-forward needle nil t)
               (list needle
                     (get-text-property (match-beginning 0) 'face)
                     (get-text-property (match-beginning 0) 'wrap-prefix)
                     (get-text-property (match-beginning 0) 'line-prefix)
                     (get-text-property (match-beginning 0) 'fontified))
               (list needle 'not-found nil nil nil))))
       '("Indent test" "Sub indent" "Body here" "Body sub here")))))"##,
    );
}

#[test]
fn ft_probe_face_with_rear_nonsticky_custom_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Rear nonsticky test buffer")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 8 16 'face 'italic)
    (put-text-property 8 16 'rear-nonsticky '(face))
    (put-text-property 16 26 'face 'underline)
    (put-text-property 16 26 'front-sticky t)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'rear-nonsticky) (get-text-property pos 'front-sticky))) '(1 5 8 12 16 20 25))
     ;; Insert at rear-nonsticky boundary (face should NOT propagate backward)
     'after-insert-at-rear-nonsticky (progn
                                       (goto-char 16)
                                       (insert "XYZ")
                                       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 8 12 16 19 22 25)))
     ;; Insert at front-sticky boundary (face should propagate forward)
     'after-insert-at-front-sticky (progn
                                     (goto-char 5)
                                     (insert "UVW")
                                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 8 12 16 19 22 25))))))"##,
    );
}
