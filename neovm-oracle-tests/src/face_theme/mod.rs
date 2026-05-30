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

#[test]
fn ft_surface_font_lock_after_narrow_widen_cycle_combo() {
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
      (insert "* TODO A\nBody A.\n\n")
      (insert "** DONE B\nBody B.\n\n")
      (insert "* TODO C\nBody C.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle
                                          (get-text-property (line-beginning-position) 'face)
                                          (invisible-p (match-beginning 0)))
                                    (list needle 'not-found nil))))
                            '("A" "B" "C")))))
        (let ((v0 (funcall snap)))
          ;; Narrow to A
          (goto-char (point-min))
          (search-forward "A")
          (beginning-of-line)
          (org-narrow-to-subtree)
          (font-lock-fontify-buffer)
          (let ((v1 (funcall snap)))
            ;; Widen, hide all, narrow to C
            (widen)
            (org-fold-hide-all)
            (goto-char (point-min))
            (search-forward "C")
            (beginning-of-line)
            (org-narrow-to-subtree)
            (font-lock-fontify-buffer)
            (let ((v2 (funcall snap)))
              ;; Widen, show all, refontify
              (widen)
              (org-fold-show-all)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Global cycle
                (org-global-cycle nil)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_surface_org_face_set_delete_set_property_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-fold)
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
                                    (list needle
                                          (get-text-property (match-beginning 0) 'face)
                                          (org-entry-get nil "Effort")
                                          (org-entry-get nil "Status")
                                          (org-entry-get nil "Owner"))
                                    (list needle 'not-found nil nil nil))))
                            '("TODO" ":PROPERTIES:" ":Effort:" ":END:")))))
        (let ((v0 (funcall snap)))
          (goto-char (point-min))
          (search-forward "Alpha")
          (beginning-of-line)
          (org-set-property "Effort" "2h")
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            (org-delete-property "Effort")
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              (org-set-property "Status" "active")
              (org-set-property "Owner" "Alice")
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (org-fold-hide-drawer-all)
                (let ((v4 (funcall snap)))
                  (org-fold-show-all)
                  (font-lock-ensure (point-min) (point-max))
                  (let ((v5 (funcall snap)))
                    (list v0 v1 v2 v3 v4 v5))))))))))))"##,
    );
}

#[test]
fn ft_surface_face_overlay_create_delete_recreate_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay create delete recreate test buffer content here")
    (put-text-property 1 50 'face '(:foreground "blue"))
    (let ((snap (lambda ()
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 20 30 40 50)))))
      (let ((v0 (funcall snap)))
        ;; Create overlay
        (let ((ov1 (make-overlay 5 25)))
          (overlay-put ov1 'face '(:background "yellow"))
          (overlay-put ov1 'priority 50))
        (let ((v1 (funcall snap)))
          ;; Delete overlay
          (mapc #'delete-overlay (overlays-at 10))
          (let ((v2 (funcall snap)))
            ;; Recreate different overlay
            (let ((ov2 (make-overlay 15 40)))
              (overlay-put ov2 'face '(:foreground "red" :weight bold))
              (overlay-put ov2 'priority 100))
            (let ((v3 (funcall snap)))
              ;; Delete partial region
              (delete-region 10 20)
              (let ((v4 (funcall snap)))
                (list v0 v1 v2 v3 v4)))))))))"##,
    );
}

#[test]
fn ft_surface_face_after_fold_cycle_edit_show_property_combo() {
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
      (insert "* TODO Root :root:wip:\n")
      (insert ":PROPERTIES:\n:Owner: Alice\n:Effort: 10h\n:END:\n")
      (insert "** DONE Leaf1 :fe:\nBody L1.\n\n")
      (insert "** TODO Leaf2 :be:\nBody L2.\n\n")
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
                            '("Root" "Leaf1" "Leaf2")))))
        (let ((v0 (funcall snap)))
          ;; Fold all
          (org-fold-hide-all)
          ;; Edit under hidden: insert Leaf3
          (goto-char (point-min))
          (search-forward "Root")
          (end-of-line)
          (insert "\n** WAIT Leaf3 :ops:\nBody L3.\n")
          ;; Cycle Root: children
          (goto-char (point-min))
          (search-forward "Root :root:")
          (beginning-of-line)
          (org-cycle nil)
          (org-cycle nil)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Change Leaf2 to DONE
            (goto-char (point-min))
            (search-forward "TODO Leaf2")
            (replace-match "DONE Leaf2")
            (font-lock-ensure (point-min) (point-max))
            ;; Cycle Root: overview
            (goto-char (point-min))
            (search-forward "Root :root:")
            (beginning-of-line)
            (org-cycle nil)
            (let ((v2 (funcall snap)))
              ;; Show all
              (org-fold-show-all)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Global cycle
                (org-global-cycle nil)
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_surface_face_text_property_with_property_list_remove_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property list manipulation test text zone")
    (add-text-properties 1 43 (list 'face 'bold 'key1 'val1 'key2 'val2 'key3 'val3))
    (add-text-properties 15 30 (list 'face 'italic 'key4 'val4))
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key1) (get-text-property pos 'key4))) '(1 10 15 20 30 40))
     ;; Remove face only
     'after-remove-face (progn
                          (remove-text-properties 1 43 '(face nil))
                          (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key1) (get-text-property pos 'key4))) '(1 10 15 20 30 40)))
     ;; Remove key1
     'after-remove-key1 (progn
                          (remove-text-properties 1 43 '(key1 nil))
                          (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'key1) (get-text-property pos 'key2))) '(1 15 30)))
     ;; Add face back differently
     'after-re-add-face (progn
                          (put-text-property 1 15 'face 'underline)
                          (put-text-property 15 43 'face '(:foreground "red"))
                          (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 15 25 40))))))"##,
    );
}

#[test]
fn ft_surface_org_face_with_global_cycle_two_levels_deep() {
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
      (insert "* TODO L1-A :tag-a:\n")
      (insert "** DONE L2-A :tag-a1:\nBody A.\n\n")
      (insert "** TODO L2-B :tag-a2:\nBody B.\n\n")
      (insert "* NEXT L1-B :tag-b:\n")
      (insert "** WAIT L2-C :tag-b1:\nBody C.\n\n")
      (insert "** DONE L2-D :tag-b2:\nBody D.\n\n")
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
                            '("L1-A" "L2-A" "L2-B" "L1-B" "L2-C" "L2-D")))))
        (let ((v0 (funcall snap)))
          ;; Global cycle: overview
          (org-global-cycle nil)
          (let ((v1 (funcall snap)))
            ;; Global cycle: children
            (org-global-cycle nil)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Global cycle: all
              (org-global-cycle nil)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                ;; Local cycle L1-A: overview
                (goto-char (point-min))
                (search-forward "L1-A :tag-a:")
                (beginning-of-line)
                (org-cycle nil)
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_delve_overlay_textprop_face_combo_delete_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:slant italic :weight bold))
    (let ((ov1 (make-overlay 3 13)))
      (overlay-put ov1 'face '(:foreground "green"))
      (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 18 28)))
      (overlay-put ov2 'face '(:background "cyan"))
      (overlay-put ov2 'priority 20))
    (let ((snap (lambda ()
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-text-property pos 'face))) '(1 4 8 12 16 20 24 28)))))
      (let ((v0 (funcall snap)))
        ;; Delete middle region (removes ov1 partially)
        (delete-region 8 14)
        (let ((v1 (funcall snap)))
          ;; Delete more (removes ov2 region)
          (delete-region 10 25)
          (let ((v2 (funcall snap)))
            (list v0 v1 v2))))))"##,
    );
}

#[test]
fn ft_delve_org_face_hide_all_edit_unhide_double_cycle() {
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
      (insert "* TODO A\nBody A.\n\n")
      (insert "** DONE B\nBody B.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (line-beginning-position) 'face) (invisible-p (match-beginning 0)))
                                    (list needle 'not-found nil))))
                            '("A" "B" "C")))))
        (let ((v0 (funcall snap)))
          (org-fold-hide-all)
          (goto-char (point-min))
          (search-forward "A")
          (end-of-line)
          (insert "\n** TODO C\nBody C.\n")
          (org-fold-show-all)
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            (org-global-cycle nil)
            (let ((v2 (funcall snap)))
              (org-global-cycle nil)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_delve_face_interval_tree_property_walk_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "0123456789ABCDEF")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 3 6 'face 'italic)
    (put-text-property 6 10 'face 'underline)
    (put-text-property 10 15 'face '(:foreground "red"))
    (put-text-property 15 17 'face '(:background "yellow"))
    (list
     'next-face-changes
     (let ((pos 1) (result nil))
       (while pos
         (setq pos (next-single-property-change pos 'face nil (point-max)))
         (when pos (push (list pos (get-text-property pos 'face)) result)))
       (nreverse result))
     'previous-face-changes
     (let ((pos 17) (result nil))
       (while pos
         (setq pos (previous-single-property-change pos 'face nil (point-min)))
         (when pos (push (list pos (get-text-property pos 'face)) result)))
       (nreverse result))
     'text-property-any
     (text-property-any 1 10 'face 'bold)
     'text-property-not-all
     (text-property-not-all 1 17 'face 'underline)
     'interval-count
     (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_delve_face_put_text_property_over_existing_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Override existing face properties test")
    (put-text-property 1 38 'face 'bold)
    (put-text-property 1 10 'face 'italic)
    (put-text-property 5 15 'face 'underline)
    (put-text-property 20 30 'face '(:foreground "red"))
    (put-text-property 25 38 'face '(:background "yellow" :weight bold))
    (list
     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 8 10 12 15 20 25 30 35))
     ;; Remove last override and check
     (progn
       (remove-text-properties 25 38 '(face nil))
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(20 25 30 35)))
     ;; Add better override
     (progn
       (put-text-property 1 38 'face '(:slant italic))
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 20 30))))))"##,
    );
}

#[test]
fn ft_delve_org_face_with_archive_tag_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t)
          (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Active task\nBody active.\n\n")
      (insert "* DONE old task :ARCHIVE:\n")
      (insert ":PROPERTIES:\n:ARCHIVE_TIME: 2026-05-28\n:END:\n")
      (insert "Body archive.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (mapcar
       (lambda (needle)
         (save-excursion
           (goto-char (point-min))
           (if (search-forward needle nil t)
               (list needle
                     (get-text-property (match-beginning 0) 'face)
                     (get-text-property (line-beginning-position) 'face))
               (list needle 'not-found nil))))
       '("Active" "old task" ":ARCHIVE:" "ARCHIVE_TIME")))))"##,
    );
}

#[test]
fn ft_delve_face_after_buffer_erase_and_refill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO First\nBody first.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (mapcar (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (if (search-forward needle nil t)
                                (list needle (get-text-property (match-beginning 0) 'face))
                                (list needle 'not-found))))
                        '("TODO" "DONE" "Body first"))))
        ;; Erase buffer
        (erase-buffer)
        (let ((v1 (mapcar (lambda (needle)
                            (save-excursion
                              (goto-char (point-min))
                              (if (search-forward needle nil t)
                                  (list needle (get-text-property (match-beginning 0) 'face))
                                  (list needle 'not-found))))
                          '("TODO" "Body first"))))
          ;; Refill with new content
          (insert "* DONE Second\nBody second.\n\n")
          (font-lock-ensure (point-min) (point-max))
          (let ((v2 (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (match-beginning 0) 'face))
                                    (list needle 'not-found))))
                            '("TODO" "DONE" "Body first" "Body second"))))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_delve_face_with_same_property_different_values_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Same property multiple values")
    (put-text-property 1 7 'face 'bold)
    (put-text-property 7 19 'face 'italic)
    (put-text-property 19 31 'face '(:foreground "blue"))
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 7 10 15 19 25 30))
     'text-property-any-face (mapcar (lambda (face-sym) (list face-sym (text-property-any 1 31 'face face-sym)))
                                     '(bold italic underline (:foreground "blue")))
     'char-property-at (mapcar (lambda (pos) (list pos (get-char-property pos 'face))) '(1 7 19))
     ;; Remove one value and check text-property-any
     'after-remove-italic (progn
                            (remove-text-properties 7 19 '(face nil))
                            (put-text-property 7 19 'face 'underline)
                            (mapcar (lambda (face-sym) (list face-sym (text-property-any 1 31 'face face-sym)))
                                    '(bold italic underline))))))"##,
    );
}

#[test]
fn ft_delve_org_face_double_global_cycle_with_edits() {
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
      (insert "* TODO P :proj:\n")
      (insert "** DONE S1 :fe:\nBody S1.\n\n")
      (insert "** TODO S2 :be:\nBody S2.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda ()
                    (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (line-beginning-position) 'face) (invisible-p (match-beginning 0)))
                                    (list needle 'not-found nil))))
                            '("P" "S1" "S2")))))
        (let ((v0 (funcall snap)))
          (org-global-cycle nil)
          (let ((v1 (funcall snap)))
            (org-global-cycle nil)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              (goto-char (point-min))
              (search-forward "P")
              (end-of-line)
              (insert "\n** WAIT S3 :ops:\nBody S3.\n")
              (org-global-cycle nil)
              (let ((v3 (funcall snap)))
                (org-global-cycle nil)
                (font-lock-ensure (point-min) (point-max))
                (let ((v4 (funcall snap)))
                  (list v0 v1 v2 v3 v4)))))))))))"##,
    );
}

#[test]
fn ft_pure_face_copy_face_and_compare_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'bold 'my-test-face)
    (error nil))
  (condition-case nil
      (copy-face 'italic 'my-test-face-2)
    (error nil))
  (list
   'my-test-face-exists (facep 'my-test-face)
   'my-test-face-2-exists (facep 'my-test-face-2)
   'face-equal-test (condition-case nil
                         (face-equal 'my-test-face 'bold)
                       (error 'no-face-equal))
   'face-bold-p (condition-case nil (face-bold-p 'my-test-face nil t) (error 'no-bold-p))
   'face-italic-p (condition-case nil (face-italic-p 'my-test-face-2 nil t) (error 'no-italic-p))
   'face-id-default (if (fboundp 'face-id) (face-id 'default) 'no-face-id)
   'face-id-bold (if (fboundp 'face-id) (face-id 'bold) 'no-face-id)
   'face-differs-default (face-differs-from-default-p 'bold)
   'face-differs-italic (face-differs-from-default-p 'italic)
   (condition-case nil
       (progn (set-face-attribute 'my-test-face nil :underline t :weight 'bold) 'set-ok)
     (error 'set-error))
   'face-underline-after-set (condition-case nil (face-underline-p 'my-test-face nil t) (error 'no-ulp))))"##,
    );
}

#[test]
fn ft_pure_face_all_atts_get_set_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-face-all-atts)
    (error nil))
  (list
   'family (face-attribute 'default :family nil 'default-on)
   'foundry (face-attribute 'default :foundry nil 'default-on)
   'width (face-attribute 'default :width nil 'default-on)
   'height (face-attribute 'default :height nil 'default-on)
   'weight (face-attribute 'default :weight nil 'default-on)
   'slant (face-attribute 'default :slant nil 'default-on)
   'underline (face-attribute 'default :underline nil 'default-on)
   'overline (face-attribute 'default :overline nil 'default-on)
   'strike (face-attribute 'default :strike-through nil 'default-on)
   'box (face-attribute 'default :box nil 'default-on)
   'inverse (face-attribute 'default :inverse-video nil 'default-on)
   'fg (face-attribute 'default :foreground nil 'default-on)
   'bg (face-attribute 'default :background nil 'default-on)
   'stipple (face-attribute 'default :stipple nil 'default-on)
   'inherit (face-attribute 'default :inherit nil 'default-on)
   'font (condition-case nil (face-attribute 'default :font nil 'default-on) (error 'no-font))
   'distant-fg (condition-case nil (face-attribute 'default :distant-foreground nil 'default-on) (error 'no-distant))
   (if (facep 'my-face-all-atts)
       (list (face-attribute 'my-face-all-atts :weight nil 'default-on)
             (face-attribute 'my-face-all-atts :slant nil 'default-on))
     'no-copy)))"##,
    );
}

#[test]
fn ft_pure_font_family_list_and_info_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'font-family-list-fbound (fboundp 'font-family-list)
   'first-10-families (condition-case nil
                          (let ((families (font-family-list)))
                            (list 'count (length families) 'sample (seq-take families 5)))
                        (error 'no-font-family-list))
   'find-font (condition-case nil
                  (let ((f (find-font (font-spec :family "Monospace"))))
                    (if f 'found 'not-found))
                (error 'no-find-font))
   'font-info (condition-case nil
                  (font-info (face-attribute 'default :font nil 'default-on))
                (error 'no-font-info))
   'fontp-default-font (condition-case nil
                           (fontp (face-attribute 'default :font nil 'default-on))
                         (error 'no-fontp))
   'font-slant-table (condition-case nil (font-slant-table) (error 'no-slant-table))
   'font-width-table (condition-case nil (font-width-table) (error 'no-width-table))
   'font-weight-table (condition-case nil (font-weight-table) (error 'no-weight-table)))))"##,
    );
}

#[test]
fn ft_pure_custom_theme_set_get_enable_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'custom)
  (list
   'custom-theme-set-faces-fbound (fboundp 'custom-theme-set-faces)
   'custom-theme-face-value-fbound (fboundp 'custom-theme-face-value)
   'enable-theme-fbound (fboundp 'enable-theme)
   'disable-theme-fbound (fboundp 'disable-theme)
   'load-theme-fbound (fboundp 'load-theme)
   'theme-list (if (fboundp 'custom-available-themes)
                   (custom-available-themes)
                 'no-theme-list)
   'custom-known-themes (if (boundp 'custom-known-themes) custom-known-themes 'no-known-themes)
   'face-attr-default-weight (face-attribute 'default :weight nil 'default-on)
   (condition-case nil
       (custom-theme-face-value 'default 'default)
     (error 'no-face-value))
   (condition-case nil
       (custom-theme-set-faces 'user '(default ((t (:weight bold)))))
     (error 'no-set-faces)))))"##,
    );
}

#[test]
fn ft_pure_font_lock_defaults_and_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-defaults-fbound (fboundp 'font-lock-defaults)
   'font-lock-set-defaults-fbound (fboundp 'font-lock-set-defaults)
   'font-lock-add-keywords-fbound (fboundp 'font-lock-add-keywords)
   'font-lock-remove-keywords-fbound (fboundp 'font-lock-remove-keywords)
   'font-lock-update-fbound (fboundp 'font-lock-update)
   'font-lock-flush-fbound (fboundp 'font-lock-flush)
   'font-lock-ensure-fbound (fboundp 'font-lock-ensure)
   (condition-case nil
       (progn
         (font-lock-add-keywords nil '(("\\<\\(TODO\\)\\>" 1 font-lock-warning-face t)))
         (font-lock-remove-keywords nil '(("\\<\\(TODO\\)\\>" 1 font-lock-warning-face t)))
         'add-remove-ok)
     (error 'add-remove-failed))
   'font-lock-keywords-case-fold (font-lock-keywords-case-fold)
   'font-lock-syntactic-face-function (if (fboundp 'font-lock-syntactic-face-function)
                                          (font-lock-syntactic-face-function)
                                        'no-func))))"##,
    );
}

#[test]
fn ft_pure_jit_lock_functions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'jit-lock)
  (list
   'jit-lock-mode-fbound (fboundp 'jit-lock-mode)
   'jit-lock-register-fbound (fboundp 'jit-lock-register)
   'jit-lock-unregister-fbound (fboundp 'jit-lock-unregister)
   'jit-lock-function-fbound (fboundp 'jit-lock-function)
   (if (boundp 'jit-lock-mode) jit-lock-mode 'no-jit-mode)
   (if (boundp 'jit-lock-chunk-size) jit-lock-chunk-size 'no-chunk-size)
   (if (boundp 'jit-lock-stealth-time) jit-lock-stealth-time 'no-stealth)
   (if (boundp 'jit-lock-stealth-nice) jit-lock-stealth-nice 'no-nice)
   'jit-lock-functions (if (boundp 'jit-lock-functions)
                           (length jit-lock-functions)
                         'no-functions))))"##,
    );
}

#[test]
fn ft_pure_face_remap_advanced_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'face-remap-add-relative (fboundp 'face-remap-add-relative)
   'face-remap-remove-relative (fboundp 'face-remap-remove-relative)
   'face-remap-set-base (fboundp 'face-remap-set-base)
   'face-remap-reset-base (fboundp 'face-remap-reset-base)
   'text-scale-set (fboundp 'text-scale-set)
   'text-scale-increase (fboundp 'text-scale-increase)
   'text-scale-decrease (fboundp 'text-scale-decrease)
   'buffer-face-mode (fboundp 'buffer-face-mode)
   'variable-pitch-mode (fboundp 'variable-pitch-mode)
   (condition-case nil
       (progn
         (face-remap-add-relative 'default '(:weight bold))
         (face-remap-reset-base 'default)
         'remap-ok)
     (error 'remap-failed))
   'face-remapping-alist (face-remapping-alist)
   'default-text-scale (if (boundp 'text-scale-mode-amount) text-scale-mode-amount 'no-amount))))"##,
    );
}

#[test]
fn ft_pure_color_api_functions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'color-name-to-rgb (fboundp 'color-name-to-rgb)
   'color-rgb-to-hex (fboundp 'color-rgb-to-hex)
   'color-values (fboundp 'color-values)
   'color-defined-p (fboundp 'color-defined-p)
   'color-dark-p (fboundp 'color-dark-p)
   'color-light-name-p (fboundp 'color-light-name-p)
   'color-complement (fboundp 'color-complement)
   'color-gradient (fboundp 'color-gradient)
   'color-hsl-to-rgb (fboundp 'color-hsl-to-rgb)
   'color-rgb-to-hsl (fboundp 'color-rgb-to-hsl)
   (condition-case nil (color-name-to-rgb "red") (error 'no-rgb))
   (condition-case nil (color-values "black") (error 'no-values))
   (condition-case nil (color-values "black" t) (error 'no-values-frame))
   (condition-case nil (color-dark-p "#000000") (error 'no-darkp))
   (condition-case nil (color-complement "red") (error 'no-complement))
   (condition-case nil (color-gradient '(0 0 0) '(1 1 1) 3) (error 'no-gradient)))))"##,
    );
}

#[test]
fn ft_pure_face_list_and_basic_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-list-fbound (fboundp 'face-list)
   'first-10-faces (condition-case nil
                       (seq-take (face-list) 10)
                     (error 'no-face-list))
   'facep-default (facep 'default)
   'facep-bold (facep 'bold)
   'facep-italic (facep 'italic)
   'facep-bold-italic (facep 'bold-italic)
   'facep-underline (facep 'underline)
   'facep-non-existent (facep 'this-face-does-not-exist-really)
   'make-face (condition-case nil
                  (progn (make-face 'my-dynamic-face) (facep 'my-dynamic-face))
                (error 'no-make-face))
   'internal-lisp-face-p (if (fboundp 'internal-lisp-face-p)
                             (list (internal-lisp-face-p 'default)
                                   (internal-lisp-face-p 'bold)
                                   (internal-lisp-face-p 'my-dynamic-face))
                           'no-internal-lisp-face-p)
   'face-nontrivial-faces (condition-case nil
                              (length (face-list))
                            (error 'no-face-list))))"##,
    );
}

#[test]
fn ft_pure_frame_face_parameters_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'frame-parameter-face (if (fboundp 'frame-parameter)
                             (frame-parameter nil 'font)
                           'no-frame-parameter)
   'face-attribute-frame (condition-case nil
                             (face-attribute 'default :family nil t)
                           (error 'no-frame-face))
   'face-attribute-frame-fg (condition-case nil
                                (face-attribute 'default :foreground nil t)
                              (error 'no-frame-fg))
   'face-attribute-frame-bg (condition-case nil
                                (face-attribute 'default :background nil t)
                              (error 'no-frame-bg))
   'font-attribute-from-frame (condition-case nil
                                  (face-attribute 'default :font nil t)
                                (error 'no-frame-font))
   'display-graphic-p (display-graphic-p)
   'display-color-p (display-color-p)
   'display-grayscale-p (if (fboundp 'display-grayscale-p) (display-grayscale-p) 'no)
   'display-planes (if (fboundp 'display-planes) (display-planes) 'no)
   'display-color-cells (if (fboundp 'display-color-cells) (display-color-cells) 'no)
   'display-mm-height (if (fboundp 'display-mm-height) (display-mm-height) 'no)
   'display-mm-width (if (fboundp 'display-mm-width) (display-mm-width) 'no)
   'display-pixel-height (display-pixel-height)
   'display-pixel-width (display-pixel-width))))"##,
    );
}

#[test]
fn ft_pure_set_face_attribute_nil_frame_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'set-face-attribute-nil-frame
   (condition-case nil
       (progn
         (set-face-attribute 'default nil :weight 'normal :slant 'normal)
         'ok)
     (error "set-face-attribute with nil frame failed"))
   'set-face-underline-nil-frame
   (condition-case nil
       (progn
         (set-face-underline 'default nil nil)
         'ok)
     (error "set-face-underline with nil frame failed"))
   'face-underline-p-nil-frame
   (condition-case nil
       (face-underline-p 'default nil t)
     (error "face-underline-p with nil frame failed"))
   'face-bold-p-nil-frame
   (condition-case nil
       (face-bold-p 'default nil t)
     (error "face-bold-p with nil frame failed"))
   'face-italic-p-nil-frame
   (condition-case nil
       (face-italic-p 'default nil t)
     (error "face-italic-p with nil frame failed"))
   'face-font-nil-frame
   (condition-case nil
       (face-font 'default nil)
     (error "face-font with nil frame failed"))
   'face-foreground-nil-frame
   (condition-case nil
       (face-foreground 'default nil 'default-on)
     (error "face-foreground with nil frame failed"))
   'face-background-nil-frame
   (condition-case nil
       (face-background 'default nil 'default-on)
     (error "face-background with nil frame failed"))))"##,
    );
}

#[test]
fn ft_pure_set_face_attribute_with_frame_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((frame (selected-frame)))
    (list
     'set-face-attr-with-frame
     (condition-case nil
         (progn
           (set-face-attribute 'default frame :weight 'normal)
           'ok)
       (error "set-face-attribute with frame failed"))
     'face-attr-weight
     (condition-case nil
         (face-attribute 'default :weight frame 'default-on)
       (error 'no))
     'face-bold-p-with-frame
     (condition-case nil
         (face-bold-p 'default frame t)
       (error 'no))
     'face-font-with-frame
     (condition-case nil
         (face-font 'default frame)
       (error 'no))
     'face-foreground-with-frame
     (condition-case nil
         (face-foreground 'default frame)
       (error 'no))
     (if (fboundp 'internal-set-lisp-face-attribute)
         (condition-case nil
             (progn (internal-set-lisp-face-attribute 'default :weight 'bold frame) 'internal-set-ok)
           (error 'internal-set-failed))
       'no-internal-set))))"##,
    );
}

#[test]
fn ft_pure_face_font_spec_create_query_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'font-spec-fbound (fboundp 'font-spec)
   'font-xlfd-name-fbound (fboundp 'font-xlfd-name)
   'query-font-fbound (fboundp 'query-font)
   'list-fonts-fbound (fboundp 'list-fonts)
   'find-font-fbound (fboundp 'find-font)
   (condition-case nil
       (let ((spec (font-spec :family "Monospace" :size 12 :weight 'bold)))
         (list 'font-spec-created (fontp spec)
               'font-get-family (font-get spec :family)
               'font-get-size (font-get spec :size)
               'font-get-weight (font-get spec :weight)))
     (error 'no-font-spec))
   (condition-case nil
       (let ((fonts (list-fonts (font-spec :family "Monospace"))))
         (list 'list-fonts-count (length fonts) 'first-font-p (fontp (car fonts))))
     (error 'no-list-fonts))
   (condition-case nil
       (font-xlfd-name (font-spec :family "Monospace"))
     (error 'no-xlfd-name))
   (condition-case nil
       (let ((f (query-font (font-spec :family "Monospace"))))
         (if f 'found 'not-found))
     (error 'no-query-font)))))"##,
    );
}

#[test]
fn ft_pure_face_set_foreground_background_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-color-face)
    (error nil))
  (list
   'set-fg
   (condition-case nil
       (progn (set-face-foreground 'my-color-face "red" nil) 'ok)
     (error 'no-set-fg))
   'set-bg
   (condition-case nil
       (progn (set-face-background 'my-color-face "yellow" nil) 'ok)
     (error 'no-set-bg))
   'get-fg
   (condition-case nil
       (face-foreground 'my-color-face nil 'default-on)
     (error 'no-get-fg))
   'get-bg
   (condition-case nil
       (face-background 'my-color-face nil 'default-on)
     (error 'no-get-bg))
   'set-fg-named
   (condition-case nil
       (progn (set-face-foreground 'my-color-face "blue" nil) 'ok)
     (error 'no-set-fg2))
   'set-bg-named
   (condition-case nil
       (progn (set-face-background 'my-color-face "white" nil) 'ok)
     (error 'no-set-bg2))
   'get-fg-after
   (condition-case nil
       (face-foreground 'my-color-face nil 'default-on)
     (error 'no-get-fg2))
   'get-bg-after
   (condition-case nil
       (face-background 'my-color-face nil 'default-on)
     (error 'no-get-bg2))))"##,
    );
}

#[test]
fn ft_pure_set_face_font_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-font-face)
    (error nil))
  (list
   'set-face-font-string
   (condition-case nil
       (progn (set-face-font 'my-font-face "Monospace-12" nil) 'ok)
     (error 'no-set-face-font-string))
   'set-face-font-spec
   (condition-case nil
       (progn (set-face-font 'my-font-face (font-spec :family "Monospace" :size 12) nil) 'ok)
     (error 'no-set-face-font-spec))
   'get-face-font
   (condition-case nil
       (face-font 'my-font-face nil)
     (error 'no-get-face-font))
   'set-face-font-bold
   (condition-case nil
       (progn (set-face-font 'my-font-face "Monospace-Bold-12" nil) 'ok)
     (error 'no-set-face-font-bold))
   'get-face-font-after (condition-case nil (face-font 'my-font-face nil) (error 'no-get-face-font2)))))"##,
    );
}

#[test]
fn ft_pure_set_face_underline_over_line_strike_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-decoration-face)
    (error nil))
  (list
   'set-underline-on
   (condition-case nil
       (progn (set-face-underline 'my-decoration-face t nil) 'ok)
     (error 'no-set-underline))
   'check-underline
   (condition-case nil
       (face-underline-p 'my-decoration-face nil t)
     (error 'no-check-underline))
   'set-underline-off
   (condition-case nil
       (progn (set-face-underline 'my-decoration-face nil nil) 'ok)
     (error 'no-set-underline-off))
   'check-underline-off
   (condition-case nil
       (face-underline-p 'my-decoration-face nil t)
     (error 'no-check-underline2))
   'set-underline-color
   (condition-case nil
       (progn (set-face-underline 'my-decoration-face '(:color "red" :style wave) nil) 'ok)
     (error 'no-set-underline-color))
   'get-underline-color
   (condition-case nil
       (face-attribute 'my-decoration-face :underline nil 'default-on)
     (error 'no-get-underline))
   'set-overline
   (condition-case nil
       (progn (set-face-attribute 'my-decoration-face nil :overline t) 'ok)
     (error 'no-set-overline))
   'set-strike-through
   (condition-case nil
       (progn (set-face-attribute 'my-decoration-face nil :strike-through t) 'ok)
     (error 'no-set-strike))
   'get-overline (condition-case nil (face-attribute 'my-decoration-face :overline nil 'default-on) (error 'no))
   'get-strike (condition-case nil (face-attribute 'my-decoration-face :strike-through nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_pure_face_inverse_video_and_box_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-box-face)
    (error nil))
  (list
   'set-inverse-video
   (condition-case nil
       (progn (set-face-attribute 'my-box-face nil :inverse-video t) 'ok)
     (error 'no))
   'get-inverse-video
   (condition-case nil
       (face-attribute 'my-box-face :inverse-video nil 'default-on)
     (error 'no))
   'set-inverse-video-off
   (condition-case nil
       (progn (set-face-attribute 'my-box-face nil :inverse-video nil) 'ok)
     (error 'no))
   'set-box
   (condition-case nil
       (progn (set-face-attribute 'my-box-face nil :box '(:line-width 2 :color "red")) 'ok)
     (error 'no))
   'get-box
   (condition-case nil
       (face-attribute 'my-box-face :box nil 'default-on)
     (error 'no))
   'set-box-simple
   (condition-case nil
       (progn (set-face-attribute 'my-box-face nil :box t) 'ok)
     (error 'no))
   'get-box-simple
   (condition-case nil
       (face-attribute 'my-box-face :box nil 'default-on)
     (error 'no))
   'set-box-off
   (condition-case nil
       (progn (set-face-attribute 'my-box-face nil :box nil) 'ok)
     (error 'no)))))"##,
    );
}

#[test]
fn ft_pure_face_width_height_slant_weight_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-prop-face)
    (error nil))
  (list
   'set-weight-bold
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :weight 'bold) 'ok)
     (error 'no))
   'get-weight (face-attribute 'my-prop-face :weight nil 'default-on)
   'set-weight-light
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :weight 'light) 'ok)
     (error 'no))
   'get-weight-light (face-attribute 'my-prop-face :weight nil 'default-on)
   'set-slant-italic
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :slant 'italic) 'ok)
     (error 'no))
   'get-slant-italic (face-attribute 'my-prop-face :slant nil 'default-on)
   'set-slant-oblique
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :slant 'oblique) 'ok)
     (error 'no))
   'get-slant-oblique (face-attribute 'my-prop-face :slant nil 'default-on)
   'set-width-condensed
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :width 'condensed) 'ok)
     (error 'no))
   'get-width (face-attribute 'my-prop-face :width nil 'default-on)
   'set-height
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :height 120) 'ok)
     (error 'no))
   'get-height (face-attribute 'my-prop-face :height nil 'default-on)
   'set-height-float
   (condition-case nil
       (progn (set-face-attribute 'my-prop-face nil :height 1.5) 'ok)
     (error 'no))
   'get-height-float (face-attribute 'my-prop-face :height nil 'default-on)))))"##,
    );
}

#[test]
fn ft_pure_face_stipple_and_inherit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil
      (copy-face 'default 'my-inherit-face)
    (error nil))
  (list
   'set-inherit
   (condition-case nil
       (progn (set-face-attribute 'my-inherit-face nil :inherit 'bold) 'ok)
     (error 'no))
   'get-inherit (condition-case nil (face-attribute 'my-inherit-face :inherit nil 'default-on) (error 'no))
   'set-inherit-multiple
   (condition-case nil
       (progn (set-face-attribute 'my-inherit-face nil :inherit '(bold italic)) 'ok)
     (error 'no))
   'get-inherit-multi (condition-case nil (face-attribute 'my-inherit-face :inherit nil 'default-on) (error 'no))
   'set-inherit-none
   (condition-case nil
       (progn (set-face-attribute 'my-inherit-face nil :inherit nil) 'ok)
     (error 'no))
   'stipple (condition-case nil (face-attribute 'default :stipple nil 'default-on) (error 'no))
   'distant-fg (condition-case nil (face-attribute 'default :distant-foreground nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_pure_merge_faces_and_face_filter_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'merge-faces-fbound (fboundp 'merge-faces)
   (condition-case nil
       (merge-faces 'bold 'italic)
     (error 'no-merge-faces))
   (condition-case nil
       (face-spec-set 'default '((t :weight bold)) 'face-defface-spec)
     (error 'no-face-spec-set))
   'face-spec-choose (condition-case nil
                         (face-spec-choose '((t :weight bold)))
                       (error 'no-face-spec-choose))
   'face-attribute-relative-p (if (fboundp 'face-attribute-relative-p)
                                   (face-attribute-relative-p :height)
                                 'no-func)
   'face-spec-match-p (if (fboundp 'face-spec-match-p)
                           (face-spec-match-p 'default '((t :weight bold)))
                         'no-func)
   'face-filters (if (fboundp 'face-filters) (face-filters) 'no-func))))"##,
    );
}

#[test]
fn ft_pure_fontset_and_charset_operations_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'set-fontset-font-fbound (fboundp 'set-fontset-font)
   'fontset-plain-fbound (fboundp 'fontset-plain-name)
   'new-fontset-fbound (fboundp 'new-fontset)
   'set-face-font-fbound (fboundp 'set-face-font)
   'internal-char-font-fbound (fboundp 'internal-char-font)
   (condition-case nil
       (let ((fs (create-fontset-from-fontset-spec
                  (fontset-plain-name "fontset-default") nil 'noerror)))
         (if fs 'created 'not-created))
     (error 'no-fontset))
   (condition-case nil
       (internal-char-font nil ?A)
     (error 'no-internal-char-font))
   (if (fboundp 'fontset-info)
       (condition-case nil
           (fontset-info "fontset-default")
         (error 'no-fontset-info))
     'no-fontset-info)
   (if (fboundp 'fontset-list)
       (condition-case nil
           (length (fontset-list))
         (error 'no-fontset-list))
     'no-fontset-list)
   (if (boundp 'font-encoding-alist)
       (length font-encoding-alist)
     'no-encoding-alist))))"##,
    );
}

#[test]
fn ft_pure_variable_font_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'describe-font-fbound (fboundp 'describe-font)
   'font-face-attributes-fbound (fboundp 'font-face-attributes)
   'clear-face-cache-fbound (fboundp 'clear-face-cache)
   (condition-case nil
       (clear-face-cache)
     (error 'no-clear-cache))
   (if (fboundp 'frame-parameter)
       (condition-case nil
           (frame-parameter nil 'font-backend)
         (error 'no-font-backend))
     'no-frame-param)
   (condition-case nil
       (list-fonts (font-spec))
     (error 'no-list-all-fonts))
   (condition-case nil
       (let ((font (face-font 'default nil)))
         (if (fontp font)
             (list 'font-family (font-get font :family)
                   'font-size (font-get font :size)
                   'font-weight (font-get font :weight)
                   'font-slant (font-get font :slant)
                   'font-width (font-get font :width)
                   'font-adstyle (font-get font :adstyle)
                   'font-registry (font-get font :registry))
           'not-a-font))
     (error 'no-font-get))
   (cond ((fboundp 'font-xlfd-name)
          (condition-case nil
              (font-xlfd-name (face-font 'default nil))
            (error 'no-xlfd-name)))
         (t 'no-xlfd-func)))))"##,
    );
}

#[test]
fn ft_pure_face_readable_x_resources_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-set-after-frame-default-fbound (fboundp 'face-set-after-frame-default)
   'x-get-resource-fbound (fboundp 'x-get-resource)
   (condition-case nil
       (face-set-after-frame-default (selected-frame))
     (error 'no-face-set-after-default))
   (condition-case nil
       (x-get-resource "font" "Font")
     (error 'no-x-resource))
   (condition-case nil
       (let ((res (x-get-resource "face.attributeForeground" "Face.AttributeForeground")))
         (if res (format "got: %s" res) 'no-res))
     (error 'no-x-resource))
   (if (fboundp 'display-backing-store)
       (display-backing-store)
     'no-backing-store)
   (if (fboundp 'display-save-under)
       (display-save-under)
     'no-save-under)
   (if (fboundp 'display-visual-class)
       (display-visual-class)
     'no-visual-class)
   (if (fboundp 'x-display-color-p)
       (x-display-color-p)
     'no-x-display-color)
   (if (fboundp 'x-display-grayscale-p)
       (x-display-grayscale-p)
     'no-x-display-grayscale))))"##,
    );
}

#[test]
fn ft_pure_face_documentation_and_error_handling_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-documentation-fbound (fboundp 'face-documentation)
   (condition-case nil
       (face-documentation 'default)
     (error 'no-doc))
   (condition-case nil
       (face-documentation 'bold)
     (error 'no-doc-bold))
   'error-invalid-face
   (condition-case err
       (face-attribute 'this-is-not-a-valid-face-at-all-nope :weight)
     (error (list 'caught-error (car err))))
   'error-invalid-face-fg
   (condition-case err
       (face-foreground 'not-a-face)
     (error (list 'caught-error (car err))))
   'error-invalid-face-bg
   (condition-case err
       (face-background 'not-a-face)
     (error (list 'caught-error (car err))))
   'error-invalid-face-font
   (condition-case err
       (face-font 'not-a-face nil)
     (error (list 'caught-error (car err))))
   'error-set-attribute-invalid-face
   (condition-case err
       (set-face-attribute 'not-a-face nil :weight 'bold)
     (error (list 'caught-error (car err))))))"##,
    );
}

#[test]
fn ft_pure_face_convenience_functions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'internal-find-face-fbound (fboundp 'internal-find-face)
   'face-equal-fbound (fboundp 'face-equal)
   'face-differs-from-default-p-fbound (fboundp 'face-differs-from-default-p)
   'face-nontrivial-faces-fbound (fboundp 'face-nontrivial-faces)
   (condition-case nil
       (face-equal 'default 'default)
     (error 'no-face-equal))
   (condition-case nil
       (face-equal 'default 'bold)
     (error 'no-face-equal2))
   (condition-case nil
       (face-differs-from-default-p 'bold)
     (error 'no-face-differs))
   (condition-case nil
       (face-differs-from-default-p 'default)
     (error 'no-face-differs2))
   (condition-case nil
       (internal-find-face 'default)
     (error 'no-internal-find-face))
   (if (fboundp 'face-nontrivial-faces)
       (condition-case nil
           (length (face-nontrivial-faces))
         (error 'no-length))
     'no-func)
   (if (fboundp 'face-nontrivial-faces)
       (condition-case nil
           (member 'bold (face-nontrivial-faces))
         (error 'no-member))
     'no-func2))))"##,
    );
}

#[test]
fn ft_pure_face_buffer_face_mode_interactions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "Buffer face mode test")
    (put-text-property 1 23 'face 'bold)
    (list
     'initial-face (get-text-property 1 'face)
     'buffer-face-mode-fbound (fboundp 'buffer-face-mode)
     'variable-pitch-mode-fbound (fboundp 'variable-pitch-mode)
     'turn-on-buffer-face
     (condition-case nil
         (progn
           (buffer-face-mode 1)
           'ok)
       (error 'no-buffer-face-mode))
     'turn-on-variable-pitch
     (condition-case nil
         (progn
           (variable-pitch-mode 1)
           'ok)
       (error 'no-variable-pitch-mode))
     'turn-off-buffer-face
     (condition-case nil
         (progn
           (buffer-face-mode -1)
           'ok)
       (error 'no-buffer-face-mode-off))
     'turn-off-variable-pitch
     (condition-case nil
         (progn
           (variable-pitch-mode -1)
           'ok)
       (error 'no-variable-pitch-off))
     'face-remap-set-base
     (condition-case nil
         (progn
           (face-remap-set-base 'default '(:height 1.5))
           'ok)
       (error 'no-remap-set-base))
     'face-remap-reset
     (condition-case nil
         (progn
           (face-remap-reset-base 'default)
           'ok)
       (error 'no-remap-reset))))))"##,
    );
}

#[test]
fn ft_pure_face_with_derived_mode_font_lock_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-defaults-bound (boundp 'font-lock-defaults)
   (if (fboundp 'font-lock-defaults)
       (condition-case nil
           (font-lock-defaults t)
         (error 'no-defaults))
     'no-font-lock-defaults-func)
   (if (fboundp 'font-lock-choose-keywords)
       (condition-case nil
           (font-lock-choose-keywords
            '(("\\<FOO\\>" . font-lock-warning-face))
            'emacs-lisp-mode)
         (error 'no-choose-keywords))
     'no-choose-func)
   (if (boundp 'font-lock-support-mode)
       font-lock-support-mode
     'no-support-mode)
   (if (boundp 'font-lock-maximum-decoration)
       font-lock-maximum-decoration
     'no-max-dec)
   (if (boundp 'font-lock-verbose)
       font-lock-verbose
     'no-verbose)
   (if (fboundp 'font-lock-value-in-major-mode)
       (condition-case nil
           (font-lock-value-in-major-mode)
         (error 'no-value-in-mode))
     'no-func)
   (if (fboundp 'font-lock-refresh-defaults)
       (condition-case nil
           (font-lock-refresh-defaults)
         (error 'no-refresh))
     'no-refresh-func))))"##,
    );
}

#[test]
fn ft_pure_face_invisible_intangible_display_text_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Invisible intangible display face test here now")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 1 11 'invisible t)
    (put-text-property 11 23 'face 'italic)
    (put-text-property 11 23 'intangible t)
    (put-text-property 23 40 'face 'underline)
    (put-text-property 23 40 'display "[[replaced]]")
    (list
     'invisible-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible) (invisible-p pos))) '(1 5 11))
     'intangible-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'intangible))) '(11 15 20))
     'display-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'display))) '(23 30 38))
     'text-properties-full (mapcar (lambda (pos) (goto-char pos) (text-properties-at pos)) '(1 11 23))
     ;; Remove invisible and recheck
     'after-remove-invisible (progn
                               (remove-text-properties 1 11 '(invisible nil))
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5 11)))
     ;; Remove intangible
     'after-remove-intangible (progn
                                (remove-text-properties 11 23 '(intangible nil))
                                (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'intangible))) '(11 15 20))))))"##,
    );
}

#[test]
fn ft_pure_face_conditional_face_with_eval_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (list
   'custom-declare-face-fbound (fboundp 'custom-declare-face)
   'face-spec-set-fbound (fboundp 'face-spec-set)
   'face-spec-choose-fbound (fboundp 'face-spec-choose)
   (condition-case nil
       (face-spec-choose '((t :weight bold)
                           (((class color) (min-colors 88)) :foreground "red")
                           (((class mono)) :foreground "black")))
     (error 'no-spec-choose))
   (condition-case nil
       (face-all-attributes 'default (selected-frame))
     (error 'no-all-atts))
   (condition-case nil
       (let ((atts (face-all-attributes 'default (selected-frame))))
         (if atts (list 'count (length atts) 'has-weight (plist-get atts :weight)) 'no-atts))
     (error 'no-all-atts2))
   (condition-case nil
       (face-spec-set 'default
                      '((t :weight bold :slant italic))
                      'face-defface-spec)
     (error 'no-face-spec-set))
   (condition-case nil
       (face-spec-set 'default
                      '((t :weight normal :slant normal))
                      'face-defface-spec)
     (error 'no-face-spec-reset)))))"##,
    );
}

#[test]
fn ft_pure_font_lock_fontify_buffer_region_syntactically_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (insert "/* comment */ int x = 42; // line comment")
    (c-mode)
    (font-lock-ensure (point-min) (point-max))
    (list
     'faces-after-fontify
     (mapcar (lambda (needle)
               (save-excursion
                 (goto-char (point-min))
                 (if (search-forward needle nil t)
                     (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))
                     (list needle 'not-found nil))))
             '("comment" "int" "x" "42" "line"))
     'font-lock-fontify-region
     (condition-case nil
         (progn (font-lock-fontify-region (point-min) (point-max) t) 'ok)
       (error 'no-fontify-region))
     'font-lock-fontify-syntactically
     (condition-case nil
         (progn (font-lock-fontify-syntactically (point-min) (point-max) nil) 'ok)
       (error 'no-fontify-syn))
     'font-lock-fontify-keywords
     (condition-case nil
         (progn (font-lock-fontify-keywords-region (point-min) (point-max) nil) 'ok)
       (error 'no-fontify-keywords))
     'font-lock-unfontify
     (condition-case nil
         (progn (font-lock-unfontify-region (point-min) (point-max)) 'ok)
       (error 'no-unfontify))
     'after-unfontify
     (mapcar (lambda (needle)
               (save-excursion
                 (goto-char (point-min))
                 (if (search-forward needle nil t)
                     (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))
                     (list needle 'not-found nil))))
             '("comment" "int" "x"))))))"##,
    );
}

#[test]
fn ft_pure_font_lock_syntactic_keyword_table_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-syntactic-keywords-fbound (fboundp 'font-lock-syntactic-keywords)
   'font-lock-syntax-table-fbound (fboundp 'font-lock-syntax-table)
   'font-lock-keywords-alist-fbound (fboundp 'font-lock-keywords-alist)
   'font-lock-remove-keywords-fbound (fboundp 'font-lock-remove-keywords)
   'font-lock-add-keywords-multiple
   (condition-case nil
       (progn
         (font-lock-add-keywords nil
                                 '(("\\<\\(A1\\)\\>" 1 font-lock-warning-face t)
                                   ("\\<\\(A2\\)\\>" 1 '(:foreground "red") t)
                                   ("\\<\\(A3\\)\\>" 1 '(:weight bold) t)))
         'add-multiple-ok)
     (error 'add-multiple-failed))
   'font-lock-remove-keywords-one
   (condition-case nil
       (progn
         (font-lock-remove-keywords nil
                                    '(("\\<\\(A1\\)\\>" 1 font-lock-warning-face t)))
         'remove-one-ok)
     (error 'remove-one-failed))
   'font-lock-remove-keywords-rest
   (condition-case nil
       (progn
         (font-lock-remove-keywords nil
                                    '(("\\<\\(A2\\)\\>" 1 '(:foreground "red") t)
                                      ("\\<\\(A3\\)\\>" 1 '(:weight bold) t)))
         'remove-rest-ok)
     (error 'remove-rest-failed))
   (if (boundp 'font-lock-keywords-only)
       font-lock-keywords-only
     'no-keywords-only)
   (if (boundp 'font-lock-beginning-of-syntax-function)
       font-lock-beginning-of-syntax-function
     'no-bos-func))))"##,
    );
}

#[test]
fn ft_pure_face_set_face_attribute_interactive_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'set-face-attribute-fbound (fboundp 'set-face-attribute)
   'modify-face-fbound (fboundp 'modify-face)
   'customize-face-fbound (fboundp 'customize-face)
   (condition-case nil
       (copy-face 'default 'my-interactive-face)
     (error nil))
   'modify-face
   (condition-case nil
       (progn
         (modify-face 'my-interactive-face "blue" "yellow" nil nil "Monospace" nil nil)
         'modify-ok)
     (error 'modify-failed))
   'face-foreground-after-modify (condition-case nil (face-foreground 'my-interactive-face nil 'default-on) (error 'no))
   'face-background-after-modify (condition-case nil (face-background 'my-interactive-face nil 'default-on) (error 'no))
   'set-face-attribute-multiple
   (condition-case nil
       (progn
         (set-face-attribute 'my-interactive-face nil
                             :foreground "green"
                             :background "black"
                             :weight 'bold
                             :slant 'italic)
         'set-multi-ok)
     (error 'set-multi-failed))
   'face-fg-after-multi (condition-case nil (face-foreground 'my-interactive-face nil 'default-on) (error 'no))
   'face-bg-after-multi (condition-case nil (face-background 'my-interactive-face nil 'default-on) (error 'no))
   'reset-to-default
   (condition-case nil
       (progn
         (set-face-attribute 'my-interactive-face nil
                             :foreground 'unspecified :background 'unspecified
                             :weight 'unspecified :slant 'unspecified)
         'reset-ok)
     (error 'reset-failed)))))"##,
    );
}

#[test]
fn ft_pure_face_aliases_and_obsolete_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-alias-fbound (fboundp 'face-alias)
   'define-obsolete-face-alias-fbound (fboundp 'define-obsolete-face-alias)
   (condition-case nil
       (face-alias 'default)
     (error 'no-face-alias))
   (condition-case nil
       (face-alias 'modeline)
     (error 'no-modeline-alias))
   (condition-case nil
       (face-alias 'mode-line)
     (error 'no-mode-line-alias))
   'check-region-face
   (if (fboundp 'region)
       (condition-case nil
           (facep 'region)
         (error 'no-region-facep))
     'no-region-func)
   'check-secondary-selection
   (if (fboundp 'secondary-selection)
       (condition-case nil
           (facep 'secondary-selection)
         (error 'no-sec-sel-facep))
     'no-sec-sel)
   'check-highlight
   (condition-case nil
       (facep 'highlight)
     (error 'no-highlight))
   'check-trailing-whitespace
   (condition-case nil
       (facep 'trailing-whitespace)
     (error 'no-trailing-ws))
   'check-fringe
   (condition-case nil
       (facep 'fringe)
     (error 'no-fringe))
   'check-cursor
   (condition-case nil
       (facep 'cursor)
     (error 'no-cursor))
   'check-scroll-bar
   (condition-case nil
       (facep 'scroll-bar)
     (error 'no-scroll-bar))
   'check-tool-bar
   (condition-case nil
       (facep 'tool-bar)
     (error 'no-tool-bar))
   'check-menu
   (condition-case nil
       (facep 'menu)
     (error 'no-menu))
   'check-border
   (condition-case nil
       (facep 'border)
     (error 'no-border))
   'check-mouse
   (condition-case nil
       (facep 'mouse)
     (error 'no-mouse))
   'check-fixed-pitch
   (condition-case nil
       (facep 'fixed-pitch)
     (error 'no-fixed-pitch))
   'check-variable-pitch
   (condition-case nil
       (facep 'variable-pitch)
     (error 'no-var-pitch))
   'check-shadow
   (condition-case nil
       (facep 'shadow)
     (error 'no-shadow))
   'check-link
   (condition-case nil
       (facep 'link)
     (error 'no-link))
   'check-link-visited
   (condition-case nil
       (facep 'link-visited)
     (error 'no-link-visited))
   'check-error
   (condition-case nil
       (facep 'error)
     (error 'no-error))
   'check-warning
   (condition-case nil
       (facep 'warning)
     (error 'no-warning))
   'check-success
   (condition-case nil
       (facep 'success)
     (error 'no-success))
   'check-match
   (condition-case nil
       (facep 'match)
     (error 'no-match))
   'check-isearch
   (condition-case nil
       (facep 'isearch)
     (error 'no-isearch))
   'check-lazy-highlight
   (condition-case nil
       (facep 'lazy-highlight)
     (error 'no-lazy))))"##,
    );
}

#[test]
fn ft_pure_face_font_lock_keywords_many_overlapping_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "KEYWORD1 and KEYWORD2 and KEYWORD3 plus KEYWORD1 again")
    (font-lock-add-keywords nil
                            '(("\\<\\(KEYWORD1\\)\\>" 1 font-lock-warning-face t)
                              ("\\<\\(KEYWORD2\\)\\>" 1 '(:foreground "red" :weight bold) t)
                              ("\\<\\(KEYWORD3\\)\\>" 1 '(:foreground "blue" :underline t) t)))
    (font-lock-fontify-buffer)
    (list
     'all-keywords (mapcar (lambda (needle)
                              (save-excursion
                                (goto-char (point-min))
                                (if (search-forward needle nil t)
                                    (list needle (get-text-property (match-beginning 0) 'face))
                                    (list needle 'not-found))))
                            '("KEYWORD1" "KEYWORD2" "KEYWORD3"))
     'duplicate-keyword (save-excursion
                          (goto-char (point-min))
                          (let ((result nil))
                            (while (search-forward "KEYWORD1" nil t)
                              (push (list (point) (get-text-property (match-beginning 0) 'face)) result))
                            (nreverse result)))
     'non-keyword (save-excursion
                    (goto-char (point-min))
                    (search-forward "and")
                    (list 'and-face (get-text-property (match-beginning 0) 'face)
                          'plus-face (progn (search-forward "plus") (get-text-property (match-beginning 0) 'face)))))))"##,
    );
}

#[test]
fn ft_pure_face_after_buffer_substring_with_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "BUFFER-SUBSTRING FACE TEST CONTENT")
    (put-text-property 1 16 'face 'bold)
    (put-text-property 16 22 'face 'italic)
    (put-text-property 22 36 'face '(:foreground "red" :weight bold))
    (list
     'substring-with-props (let ((sub (buffer-substring 1 16)))
                              (with-temp-buffer
                                (insert sub)
                                (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15))))
     'substring-no-props (buffer-substring-no-properties 1 16)
     'buffer-string-length (length (buffer-string))
     'insert-buffer-substring (with-temp-buffer
                                (insert-buffer-substring (current-buffer) 1 36)
                                (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 16 22 30)))
     'copy-to-buffer (let ((buf (generate-new-buffer "*ft-copy*")))
                       (unwind-protect
                           (progn
                             (copy-to-buffer buf 1 36)
                             (with-current-buffer buf
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 16 22 30))))
                         (kill-buffer buf))))))"##,
    );
}

#[test]
fn ft_pure_set_face_attribute_underline_style_variants_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-face) (error nil))
  (list
   'underline-line (condition-case nil (progn (set-face-underline 'my-ul-face '(:style line) nil) 'ok) (error 'no))
   'underline-line-get (condition-case nil (face-attribute 'my-ul-face :underline nil 'default-on) (error 'no))
   'underline-wave (condition-case nil (progn (set-face-underline 'my-ul-face '(:style wave) nil) 'ok) (error 'no))
   'underline-wave-get (condition-case nil (face-attribute 'my-ul-face :underline nil 'default-on) (error 'no))
   'underline-double (condition-case nil (progn (set-face-underline 'my-ul-face '(:style double-line) nil) 'ok) (error 'no))
   'underline-double-get (condition-case nil (face-attribute 'my-ul-face :underline nil 'default-on) (error 'no))
   'underline-dot (condition-case nil (progn (set-face-underline 'my-ul-face '(:style dots) nil) 'ok) (error 'no))
   'underline-dot-get (condition-case nil (face-attribute 'my-ul-face :underline nil 'default-on) (error 'no))
   'underline-dash (condition-case nil (progn (set-face-underline 'my-ul-face '(:style dash) nil) 'ok) (error 'no))
   'underline-dash-get (condition-case nil (face-attribute 'my-ul-face :underline nil 'default-on) (error 'no))
   'underline-color (condition-case nil (progn (set-face-underline 'my-ul-face '(:color "red" :style wave) nil) 'ok) (error 'no))
   'underline-color-get (condition-case nil (face-attribute 'my-ul-face :underline nil 'default-on) (error 'no))
   'underline-off (condition-case nil (progn (set-face-underline 'my-ul-face nil nil) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_pure_face_merged_via_add_face_text_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Add face text property merge test")
    ;; Add face incrementally
    (add-face-text-property 1 34 '(:underline t))
    (add-face-text-property 1 34 '(:slant italic))
    (add-face-text-property 1 15 '(:weight bold))
    (add-face-text-property 15 34 '(:foreground "red"))
    (list
     'merged-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25 30))
     'facep-merged (mapcar (lambda (pos) (goto-char pos) (list pos (facep (get-text-property pos 'face)))) '(1 15))
     ;; Remove underlines
     'after-remove-underline (progn
                               (remove-text-properties 1 34 '(face nil))
                               (add-face-text-property 1 34 '(:weight bold :slant italic :foreground "blue"))
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 15)))
     ;; Add more with append
     'with-append (progn
                    (add-face-text-property 1 34 '(:underline t) t)
                    (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 15 30))))))"##,
    );
}

#[test]
fn ft_pure_face_make_face_and_set_all_attrs_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (make-face 'my-full-face) (error nil))
  (list
   'face-exists-after-make (facep 'my-full-face)
   'set-family (condition-case nil (progn (set-face-attribute 'my-full-face nil :family "Monospace") 'ok) (error 'no))
   'set-foundry (condition-case nil (progn (set-face-attribute 'my-full-face nil :foundry "misc") 'ok) (error 'no))
   'set-width (condition-case nil (progn (set-face-attribute 'my-full-face nil :width 'normal) 'ok) (error 'no))
   'set-height (condition-case nil (progn (set-face-attribute 'my-full-face nil :height 100) 'ok) (error 'no))
   'set-weight (condition-case nil (progn (set-face-attribute 'my-full-face nil :weight 'bold) 'ok) (error 'no))
   'set-slant (condition-case nil (progn (set-face-attribute 'my-full-face nil :slant 'italic) 'ok) (error 'no))
   'set-underline (condition-case nil (progn (set-face-attribute 'my-full-face nil :underline t) 'ok) (error 'no))
   'set-overline (condition-case nil (progn (set-face-attribute 'my-full-face nil :overline t) 'ok) (error 'no))
   'set-strike (condition-case nil (progn (set-face-attribute 'my-full-face nil :strike-through t) 'ok) (error 'no))
   'set-box (condition-case nil (progn (set-face-attribute 'my-full-face nil :box t) 'ok) (error 'no))
   'set-inverse (condition-case nil (progn (set-face-attribute 'my-full-face nil :inverse-video t) 'ok) (error 'no))
   'set-fg (condition-case nil (progn (set-face-attribute 'my-full-face nil :foreground "red") 'ok) (error 'no))
   'set-bg (condition-case nil (progn (set-face-attribute 'my-full-face nil :background "yellow") 'ok) (error 'no))
   'get-family (face-attribute 'my-full-face :family nil 'default-on)
   'get-weight (face-attribute 'my-full-face :weight nil 'default-on)
   'get-slant (face-attribute 'my-full-face :slant nil 'default-on)
   'get-underline (face-attribute 'my-full-face :underline nil 'default-on)
   'get-box (face-attribute 'my-full-face :box nil 'default-on)
   'get-fg (face-attribute 'my-full-face :foreground nil 'default-on)
   'get-bg (face-attribute 'my-full-face :background nil 'default-on)
   'unset-all (condition-case nil (progn (set-face-attribute 'my-full-face nil
                                                              :family 'unspecified :foundry 'unspecified
                                                              :width 'unspecified :height 'unspecified
                                                              :weight 'unspecified :slant 'unspecified
                                                              :underline 'unspecified :overline 'unspecified
                                                              :strike-through 'unspecified :box 'unspecified
                                                              :inverse-video 'unspecified :foreground 'unspecified
                                                              :background 'unspecified) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_deep_face_with_overlay_window_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay window-specific face test data here")
    (put-text-property 1 42 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 15)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'window (selected-window)))
    (let ((ov2 (make-overlay 20 35)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold))
      (overlay-put ov2 'window nil))
    (list
     'faces-with-window-overlays (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 15 20 25 35 40))
     'overlay-get-window (list (overlay-get ov1 'window) (overlay-get ov2 'window))
     'overlay-buffer (list (overlay-buffer ov1) (overlay-buffer ov2))
     (progn (delete-overlay ov1) (delete-overlay ov2) 'cleaned))))"##,
    );
}

#[test]
fn ft_deep_face_font_lock_with_c_mode_syntax_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (c-mode)
    (insert "int main() {\n  int x = 42;\n  printf(\"hello %d\\n\", x);\n  return 0;\n}\n")
    (font-lock-ensure (point-min) (point-max))
    (mapcar
     (lambda (needle)
       (save-excursion
         (goto-char (point-min))
         (if (search-forward needle nil t)
             (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified) (get-text-property (match-beginning 0) 'font-lock-face))
             (list needle 'not-found nil nil))))
     '("int" "main" "x" "42" "printf" "return"))))"##,
    );
}

#[test]
fn ft_deep_face_font_lock_with_emacs_lisp_mode_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun my-func (x)\n  \"Docstring.\"\n  (let ((y (+ x 1)))\n    y))\n")
    (font-lock-ensure (point-min) (point-max))
    (mapcar
     (lambda (needle)
       (save-excursion
         (goto-char (point-min))
         (if (search-forward needle nil t)
             (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))
             (list needle 'not-found nil))))
     '("defun" "my-func" "Docstring" "let" "+"))))"##,
    );
}

#[test]
fn ft_deep_face_font_lock_with_python_mode_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (condition-case nil
        (let ((python-indent-guess-indent-offset nil))
          (python-mode)
          (insert "def my_func(x):\n    \"\"\"Docstring.\"\"\"\n    y = x + 1\n    return y\n")
          (font-lock-ensure (point-min) (point-max))
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (if (search-forward needle nil t)
                   (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))
                   (list needle 'not-found nil))))
           '("def" "my_func" "Docstring" "return")))
      (error (list 'python-error (fboundp 'python-mode))))))"##,
    );
}

#[test]
fn ft_deep_font_lock_ruby_mode_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (condition-case nil
        (progn
          (ruby-mode)
          (insert "def my_method(x)\n  x + 1\nend\n")
          (font-lock-ensure (point-min) (point-max))
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (if (search-forward needle nil t)
                   (list needle (get-text-property (match-beginning 0) 'face))
                   (list needle 'not-found))))
           '("def" "my_method" "end")))
      (error (list 'ruby-error (fboundp 'ruby-mode))))))"##,
    );
}

#[test]
fn ft_deep_font_lock_cpp_mode_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (condition-case nil
        (progn
          (c++-mode)
          (insert "#include <stdio.h>\nint main() { return 0; }\n")
          (font-lock-ensure (point-min) (point-max))
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (if (search-forward needle nil t)
                   (list needle (get-text-property (match-beginning 0) 'face))
                   (list needle 'not-found))))
           '("include" "int" "main" "return")))
      (error (list 'cpp-error (fboundp 'c++-mode))))))"##,
    );
}

#[test]
fn ft_deep_face_font_lock_with_js_mode_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (condition-case nil
        (let ((js-indent-level 2))
          (js-mode)
          (insert "function hello(name) {\n  return 'hi ' + name;\n}\n")
          (font-lock-ensure (point-min) (point-max))
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (if (search-forward needle nil t)
                   (list needle (get-text-property (match-beginning 0) 'face))
                   (list needle 'not-found))))
           '("function" "hello" "return")))
      (error (list 'js-error (fboundp 'js-mode))))))"##,
    );
}

#[test]
fn ft_deep_face_font_lock_markdown_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (condition-case nil
        (progn
          (markdown-mode)
          (insert "# Heading\n**Bold** text *italic* text\n")
          (font-lock-ensure (point-min) (point-max))
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (if (search-forward needle nil t)
                   (list needle (get-text-property (match-beginning 0) 'face))
                   (list needle 'not-found))))
           '("Heading" "Bold" "italic")))
      (error (list 'md-error (fboundp 'markdown-mode))))))"##,
    );
}

#[test]
fn ft_deep_face_font_lock_rust_mode_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (condition-case nil
        (progn
          (rust-mode)
          (insert "fn main() { let x = 42; println!(\"{}\", x); }\n")
          (font-lock-ensure (point-min) (point-max))
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (if (search-forward needle nil t)
                   (list needle (get-text-property (match-beginning 0) 'face))
                   (list needle 'not-found))))
           '("fn" "main" "let" "println")))
      (error (list 'rust-error (fboundp 'rust-mode))))))"##,
    );
}

#[test]
fn ft_deep_face_multiple_windows_same_buffer_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multi-window same buffer face test text")
    (put-text-property 1 13 'face 'bold)
    (put-text-property 13 25 'face 'italic)
    (put-text-property 25 40 'face 'underline)
    (save-selected-window
      (list
       'face-in-original (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 20 30))
       'buffer-name (buffer-name))))))"##,
    );
}

#[test]
fn ft_deep_face_with_minibuffer_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Minibuffer-like text properties test")
    (put-text-property 1 12 'face 'minibuffer-prompt)
    (put-text-property 12 35 'face '(:foreground "gray"))
    (list
     'minibuffer-prompt-face (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'field))) '(1 5 10))
     'normal-text (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(15 20 30))
     'minibuffer-prompt-facep (facep 'minibuffer-prompt)
     'completions-common-part-facep (condition-case nil (facep 'completions-common-part) (error 'no-face))
     'completions-first-difference-facep (condition-case nil (facep 'completions-first-difference) (error 'no-face))))))"##,
    );
}

#[test]
fn ft_hard_face_defface_custom_spec_set_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (list
   'defface-fbound (fboundp 'defface)
   'custom-declare-face-fbound (fboundp 'custom-declare-face)
   (condition-case nil
       (progn
         (defface my-custom-test-face '((t :weight bold :foreground "blue")) "Test face")
         (facep 'my-custom-test-face))
     (error 'no-defface))
   'face-spec-after (condition-case nil (face-spec 'my-custom-test-face) (error 'no-spec))
   'face-attr-weight (condition-case nil (face-attribute 'my-custom-test-face :weight nil 'default-on) (error 'no))
   'face-attr-fg (condition-case nil (face-attribute 'my-custom-test-face :foreground nil 'default-on) (error 'no))
   (condition-case nil
       (progn (set-face-attribute 'my-custom-test-face nil :foreground 'unspecified :weight 'unspecified) 'reset-ok)
     (error 'no-reset))))"##,
    );
}

#[test]
fn ft_hard_face_spec_set_with_display_conditions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-spec-set-fbound (fboundp 'face-spec-set)
   (condition-case nil
       (progn
         (face-spec-set 'default
                        '(((class color) (min-colors 88) (background light))
                          (:foreground "black" :background "white"))
                        'face-defface-spec)
         'set-with-condition-ok)
     (error 'no-condition-set))
   'face-spec-choose-with-display
   (condition-case nil
       (face-spec-choose '(((class color) :foreground "red")
                           ((class mono) :foreground "black")
                           (t :foreground "blue")))
     (error 'no-choose))
   'face-spec-match-p
   (condition-case nil
       (face-spec-match-p 'default
                           '(((class color) (min-colors 88))
                             (:foreground "black"))
                           (selected-frame))
     (error 'no-match-p))
   'display-graphic-check (display-graphic-p)
   'display-color-check (display-color-p)
   (if (fboundp 'display-color-cells) (display-color-cells) 'no-cells))))"##,
    );
}

#[test]
fn ft_hard_face_with_custom_theme_load_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'custom)
  (list
   'load-theme-fbound (fboundp 'load-theme)
   'enable-theme-fbound (fboundp 'enable-theme)
   'disable-theme-fbound (fboundp 'disable-theme)
   'custom-enabled-themes-fbound (boundp 'custom-enabled-themes)
   (if (boundp 'custom-enabled-themes)
       (length custom-enabled-themes)
     'no-enabled-themes)
   'face-attr-before (face-attribute 'default :weight nil 'default-on)
   (condition-case nil
       (progn
         (custom-theme-set-faces 'user
                                 '(default ((t (:weight bold :slant italic)))))
         'set-user-theme-ok)
     (error 'no-set-user-theme))
   'face-attr-after (face-attribute 'default :weight nil 'default-on))))"##,
    );
}

#[test]
fn ft_hard_face_set_face_attribute_foreground_background_rgb_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-rgb-face) (error nil))
  (list
   'set-fg-hex (condition-case nil (progn (set-face-foreground 'my-rgb-face "#FF0000" nil) 'ok) (error 'no))
   'get-fg-hex (condition-case nil (face-foreground 'my-rgb-face nil 'default-on) (error 'no))
   'set-fg-name (condition-case nil (progn (set-face-foreground 'my-rgb-face "DarkGreen" nil) 'ok) (error 'no))
   'get-fg-name (condition-case nil (face-foreground 'my-rgb-face nil 'default-on) (error 'no))
   'set-bg-hex (condition-case nil (progn (set-face-background 'my-rgb-face "#FFFF00" nil) 'ok) (error 'no))
   'get-bg-hex (condition-case nil (face-background 'my-rgb-face nil 'default-on) (error 'no))
   'set-fg-rgb (condition-case nil (progn (set-face-foreground 'my-rgb-face "#00FF00" nil) 'ok) (error 'no))
   'set-bg-rgb (condition-case nil (progn (set-face-background 'my-rgb-face "#0000FF" nil) 'ok) (error 'no))
   'get-fg-rgb (condition-case nil (face-foreground 'my-rgb-face nil 'default-on) (error 'no))
   'get-bg-rgb (condition-case nil (face-background 'my-rgb-face nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_hard_face_multiple_displays_face_attr_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'display-list-fbound (fboundp 'display-list)
   (condition-case nil
       (let ((displays (display-list)))
         (list 'count (length displays) 'type (type-of displays)))
     (error 'no-display-list))
   'framep-selected (framep (selected-frame))
   'face-attr-on-terminal (condition-case nil (face-attribute 'default :family nil t) (error 'no-term))
   'face-attr-defaults (condition-case nil (face-attribute 'default :family) (error 'no-defs))
   'x-display-list (if (fboundp 'x-display-list) (x-display-list) 'no-x-display)
   'default-font-on-frame (condition-case nil (face-font 'default (selected-frame)) (error 'no-font))
   'face-attr-multiple-frames (condition-case nil (list (face-attribute 'default :family nil 'default-on)
                                                         (face-attribute 'default :family nil t)
                                                         (face-attribute 'default :family))
                                                        (error 'no)))))"##,
    );
}

#[test]
fn ft_hard_face_add_text_properties_vs_put_text_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Add vs put text property test text here")
    (put-text-property 1 5 'face 'bold)
    (add-text-properties 5 15 '(face italic key1 val1 key2 val2))
    (add-face-text-property 15 25 '(:foreground "red"))
    (add-face-text-property 15 25 '(:weight bold))
    (put-text-property 25 35 'face 'underline)
    (put-text-property 25 35 'face '(:foreground "blue"))
    (list
     'put-only (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3))
     'add-text-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key1) (get-text-property pos 'key2))) '(5 10))
     'add-face-text-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(15 20))
     'put-override (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(25 30))
     'all-text-properties (mapcar (lambda (pos) (goto-char pos) (text-properties-at pos)) '(3 10 20 30)))))"##,
    );
}

#[test]
fn ft_hard_face_with_nested_invisible_and_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Nested invisible display face buffer content")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 10 20 'invisible t)
    (put-text-property 10 20 'face 'italic)
    (put-text-property 20 30 'display "REPLACED")
    (put-text-property 20 30 'face 'underline)
    (put-text-property 30 43 'face '(:foreground "red" :weight bold))
    (put-text-property 30 43 'invisible t)
    (put-text-property 30 43 'display "HIDDEN")
    (list
     'face-visible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5))
     'face-invisible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible))) '(10 15))
     'face-displayed (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'display))) '(20 25))
     'face-both (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible) (get-text-property pos 'display))) '(30 35 40))
     ;; Check char-property (considers overlays and text properties)
     'char-prop-visible (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 20 30)))))"##,
    );
}

#[test]
fn ft_brute_face_wrap_line_prefix_with_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Line with wrap prefix and line prefix face")
    (put-text-property 1 45 'wrap-prefix ">>> ")
    (put-text-property 1 45 'line-prefix "| ")
    (put-text-property 1 45 'face '(:foreground "blue"))
    (list
     'wrap-prefix (get-text-property 1 'wrap-prefix)
     'line-prefix (get-text-property 1 'line-prefix)
     'face (get-text-property 1 'face)
     'all-text-props (text-properties-at 1)
     'wrap-prefix-char-width (length (get-text-property 1 'wrap-prefix))
     'line-prefix-char-width (length (get-text-property 1 'line-prefix)))))"##,
    );
}

#[test]
fn ft_brute_face_overlay_with_category_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay category face test buffer content")
    (put-text-property 1 41 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 15)))
      (overlay-put ov1 'category 'my-cat)
      (overlay-put ov1 'face '(:background "yellow")))
    (let ((ov2 (make-overlay 20 35)))
      (overlay-put ov2 'category 'my-cat2)
      (overlay-put ov2 'face '(:foreground "red" :weight bold)))
    (list
     'faces-with-category (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 10 15 20 25 35 40))
     'overlay-category (list (overlay-get ov1 'category) (overlay-get ov2 'category))
     'overlay-properties (list (overlay-properties ov1) (overlay-properties ov2))
     (progn (delete-overlay ov1) (delete-overlay ov2) 'cleaned))))"##,
    );
}

#[test]
fn ft_brute_face_with_modification_hooks_prop_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (defvar my-mod-hook-counter 0)
  (defun my-mod-hook (ov after beg end &optional len)
    (setq my-mod-hook-counter (1+ my-mod-hook-counter)))
  (with-temp-buffer
    (insert "Modification hooks face test")
    (put-text-property 1 28 'face 'bold)
    (put-text-property 1 28 'modification-hooks '(my-mod-hook))
    (list
     'initial-face (get-text-property 1 'face)
     'initial-mod-hooks (get-text-property 1 'modification-hooks)
     'hook-counter-before my-mod-hook-counter
     ;; Modify text
     (progn
       (goto-char 10)
       (insert "INSERTED")
       'after-insert)
     'hook-counter-after my-mod-hook-counter
     'face-after-mod (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 8 12 15 28))))))"##,
    );
}

#[test]
fn ft_brute_face_set_face_attribute_extend_raise_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-extend-face) (error nil))
  (list
   'set-extend (condition-case nil (progn (set-face-attribute 'my-extend-face nil :extend t) 'ok) (error 'no))
   'get-extend (condition-case nil (face-attribute 'my-extend-face :extend nil 'default-on) (error 'no))
   'set-extend-off (condition-case nil (progn (set-face-attribute 'my-extend-face nil :extend nil) 'ok) (error 'no))
   'get-extend-off (condition-case nil (face-attribute 'my-extend-face :extend nil 'default-on) (error 'no))
   'get-extend-default (condition-case nil (face-attribute 'default :extend nil 'default-on) (error 'no))
   'set-raise (condition-case nil (progn (set-face-attribute 'my-extend-face nil :raise 0.2) 'ok) (error 'no-raise))
   'get-raise (condition-case nil (face-attribute 'my-extend-face :raise nil 'default-on) (error 'no))
   'set-raise-negative (condition-case nil (progn (set-face-attribute 'my-extend-face nil :raise -0.1) 'ok) (error 'no))
   'get-raise-neg (condition-case nil (face-attribute 'my-extend-face :raise nil 'default-on) (error 'no))))))"##,
    );
}

#[test]
fn ft_brute_face_font_lock_multi_line_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "START\nmulti-line content here\nEND")
    (font-lock-add-keywords nil
                            '(("\\`START\\'" (0 font-lock-warning-face t)
                               ("\\`END\\'" nil nil (0 font-lock-function-name-face t)))))
    (font-lock-fontify-buffer)
    (list
     'start-face (save-excursion (goto-char (point-min)) (search-forward "START") (get-text-property (match-beginning 0) 'face))
     'multi-line-face (save-excursion (goto-char (point-min)) (search-forward "multi-line") (get-text-property (match-beginning 0) 'face))
     'end-face (save-excursion (goto-char (point-min)) (search-forward "END") (get-text-property (match-beginning 0) 'face)))))"##,
    );
}

#[test]
fn ft_brute_face_font_lock_maximum_decoration_variants_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'max-decoration-bound (boundp 'font-lock-maximum-decoration)
   'max-dec-value (if (boundp 'font-lock-maximum-decoration)
                       font-lock-maximum-decoration
                     'no-bound)
   'font-lock-support-mode (if (boundp 'font-lock-support-mode)
                                font-lock-support-mode
                              'no-support-mode)
   (if (boundp 'font-lock-maximum-decoration)
       (condition-case nil
           (progn
             (setq font-lock-maximum-decoration t)
             (with-temp-buffer
               (c-mode)
               (insert "int x = 42;\n")
               (font-lock-ensure (point-min) (point-max))
               (list 'max-t
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward "int")
                       (get-text-property (match-beginning 0) 'face))
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward "42")
                       (get-text-property (match-beginning 0) 'face))
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward "x")
                       (get-text-property (match-beginning 0) 'face))))
         (error 'max-t-failed))
     'no-max-bound)
   (if (boundp 'font-lock-maximum-decoration)
       (condition-case nil
           (progn
             (setq font-lock-maximum-decoration nil)
             (with-temp-buffer
               (c-mode)
               (insert "int x = 42;\n")
               (font-lock-ensure (point-min) (point-max))
               (list 'max-nil
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward "int")
                       (get-text-property (match-beginning 0) 'face))
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward "42")
                       (get-text-property (match-beginning 0) 'face))))
         (error 'max-nil-failed))
     'no-max-bound2))))"##,
    );
}

#[test]
fn ft_brute_face_overlay_before_after_string_with_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before after string face test")
    (put-text-property 1 30 'face 'bold)
    (let ((ov1 (make-overlay 5 15)))
      (overlay-put ov1 'before-string #("[[BEFORE]]" 0 10 (face (:foreground "red" :weight bold))))
      (overlay-put ov1 'face 'italic))
    (list
     'faces-around-overlay (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20))
     'overlay-before-props (text-properties-at 0 (overlay-get ov1 'before-string))
     (progn (delete-overlay ov1) 'cleaned))))"##,
    );
}

#[test]
fn ft_brute_face_with_line_spacing_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-spacing-face) (error nil))
  (list
   'set-line-spacing (condition-case nil (progn (set-face-attribute 'my-spacing-face nil :line-spacing 10) 'ok) (error 'no))
   'get-line-spacing (condition-case nil (face-attribute 'my-spacing-face :line-spacing nil 'default-on) (error 'no))
   'set-line-spacing-float (condition-case nil (progn (set-face-attribute 'my-spacing-face nil :line-spacing 1.5) 'ok) (error 'no))
   'get-line-spacing-float (condition-case nil (face-attribute 'my-spacing-face :line-spacing nil 'default-on) (error 'no))
   'set-line-spacing-nil (condition-case nil (progn (set-face-attribute 'my-spacing-face nil :line-spacing nil) 'ok) (error 'no))
   'get-line-spacing-default (condition-case nil (face-attribute 'default :line-spacing nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_final_face_font_info_detailed_attrs_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'font-info-fbound (fboundp 'font-info)
   (condition-case nil
       (let ((info (font-info (face-font 'default nil))))
         (list 'font-info-name (aref info 0)
               'font-info-style (aref info 1)
               'font-info-size (aref info 2)))
     (error 'no-font-info))
   (condition-case nil
       (let ((fonts (list-fonts (font-spec :size 12))))
         (list 'fonts-count (length fonts)
               'first-font (if fonts (font-xlfd-name (car fonts)) 'none)))
     (error 'no-list-fonts))
   (condition-case nil
       (font-get (font-spec :family "Monospace") :family)
     (error 'no-font-get))
   (condition-case nil
       (font-put (font-spec :family "Monospace") :size 14)
     (error 'no-font-put)))))"##,
    );
}

#[test]
fn ft_final_face_all_faces_facep_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-list-count (condition-case nil (length (face-list)) (error 'no-face-list))
   'keyboard-faces (mapcar (lambda (f) (list f (facep f)))
                           '(default bold italic bold-italic underline
                             highlight region secondary-selection
                             shadow link link-visited
                             error warning success
                             match isearch lazy-highlight
                             fringe cursor scroll-bar tool-bar menu border
                             mouse fixed-pitch variable-pitch
                             trailing-whitespace escape-glyph
                             nobreak-space homoglyph
                             minibuffer-prompt mode-line mode-line-inactive mode-line-highlight
                             header-line tab-line tab-bar
                             tooltip vertical-border
                             window-divider window-divider-first-pixel window-divider-last-pixel
                             internal-border child-frame-border
                             line-number line-number-current-line
                             fill-column-indicator
                             completions-common-part completions-first-difference))
   'font-lock-faces (mapcar (lambda (f) (list f (facep f)))
                            '(font-lock-warning-face font-lock-function-name-face
                              font-lock-variable-name-face font-lock-keyword-face
                              font-lock-comment-face font-lock-string-face
                              font-lock-constant-face font-lock-type-face
                              font-lock-builtin-face font-lock-preprocessor-face
                              font-lock-negation-char-face font-lock-doc-face
                              font-lock-regexp-grouping-backslash
                              font-lock-regexp-grouping-construct))))"##,
    );
}

#[test]
fn ft_final_face_set_multiple_attrs_simultaneous_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-multi-attrs-face) (error nil))
  (condition-case nil
      (progn
        (set-face-attribute 'my-multi-attrs-face nil
                            :foreground "red"
                            :background "yellow"
                            :weight 'bold
                            :slant 'italic
                            :underline t
                            :overline t
                            :strike-through t
                            :box '(:line-width 2)
                            :inverse-video t
                            :height 150)
        'set-all-ok)
    (error 'set-all-failed))
  (list
   'fg (condition-case nil (face-attribute 'my-multi-attrs-face :foreground nil 'default-on) (error 'no))
   'bg (condition-case nil (face-attribute 'my-multi-attrs-face :background nil 'default-on) (error 'no))
   'weight (condition-case nil (face-attribute 'my-multi-attrs-face :weight nil 'default-on) (error 'no))
   'slant (condition-case nil (face-attribute 'my-multi-attrs-face :slant nil 'default-on) (error 'no))
   'underline (condition-case nil (face-attribute 'my-multi-attrs-face :underline nil 'default-on) (error 'no))
   'overline (condition-case nil (face-attribute 'my-multi-attrs-face :overline nil 'default-on) (error 'no))
   'strike (condition-case nil (face-attribute 'my-multi-attrs-face :strike-through nil 'default-on) (error 'no))
   'box (condition-case nil (face-attribute 'my-multi-attrs-face :box nil 'default-on) (error 'no))
   'inverse (condition-case nil (face-attribute 'my-multi-attrs-face :inverse-video nil 'default-on) (error 'no))
   'height (condition-case nil (face-attribute 'my-multi-attrs-face :height nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_final_face_unspecified_vs_nil_attrs_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-unspec-face) (error nil))
  (list
   'set-fg-red (condition-case nil (progn (set-face-foreground 'my-unspec-face "red" nil) 'ok) (error 'no))
   'get-fg-red (condition-case nil (face-foreground 'my-unspec-face nil 'default-on) (error 'no))
   'set-fg-unspecified (condition-case nil (progn (set-face-foreground 'my-unspec-face 'unspecified nil) 'ok) (error 'no))
   'get-fg-unspecified (condition-case nil (face-foreground 'my-unspec-face nil 'default-on) (error 'no))
   'set-fg-nil (condition-case nil (progn (set-face-foreground 'my-unspec-face nil nil) 'ok) (error 'no))
   'get-fg-nil (condition-case nil (face-foreground 'my-unspec-face nil 'default-on) (error 'no))
   'set-weight-bold (condition-case nil (progn (set-face-attribute 'my-unspec-face nil :weight 'bold) 'ok) (error 'no))
   'get-weight-bold (condition-case nil (face-attribute 'my-unspec-face :weight nil 'default-on) (error 'no))
   'set-weight-unspecified (condition-case nil (progn (set-face-attribute 'my-unspec-face nil :weight 'unspecified) 'ok) (error 'no))
   'get-weight-unspecified (condition-case nil (face-attribute 'my-unspec-face :weight nil 'default-on) (error 'no))
   'set-weight-nil (condition-case nil (progn (set-face-attribute 'my-unspec-face nil :weight nil) 'ok) (error 'no))
   'get-weight-nil (condition-case nil (face-attribute 'my-unspec-face :weight nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_final_face_face_id_and_face_equal_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-id-fbound (fboundp 'face-id)
   'face-equal-fbound (fboundp 'face-equal)
   'face-differs-from-default-p-fbound (fboundp 'face-differs-from-default-p)
   (condition-case nil
       (face-id 'default)
     (error 'no-face-id-default))
   (condition-case nil
       (face-id 'bold)
     (error 'no-face-id-bold))
   (condition-case nil
       (face-id 'italic)
     (error 'no-face-id-italic))
   (condition-case nil
       (face-equal 'default 'default)
     (error 'no-face-equal))
   (condition-case nil
       (face-equal 'default 'bold)
     (error 'no-face-equal2))
   (condition-case nil
       (not (face-equal 'bold 'italic))
     (error 'no-face-equal3))
   (condition-case nil
       (face-differs-from-default-p 'bold)
     (error 'no-face-differs))
   (condition-case nil
       (face-differs-from-default-p 'italic)
     (error 'no-face-differs2))
   (condition-case nil
       (face-differs-from-default-p 'default)
     (error 'no-face-differs3)))))"##,
    );
}

#[test]
fn ft_final_face_overlay_before_after_string_face_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay before/after string with face inherit test")
    (put-text-property 1 49 'face 'bold)
    (let ((ov1 (make-overlay 10 20)))
      (overlay-put ov1 'face 'italic)
      (overlay-put ov1 'before-string
                   (propertize "[[BEFORE]]" 'face '(:foreground "red" :inherit bold))))
    (let ((ov2 (make-overlay 25 40)))
      (overlay-put ov2 'face 'underline)
      (overlay-put ov2 'after-string
                   (propertize "{{AFTER}}" 'face '(:background "yellow" :inherit italic))))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 15 20 25 30 40 45))
     'ov1-before-props (progn (text-properties-at 0 (overlay-get ov1 'before-string)))
     'ov2-after-props (progn (text-properties-at 0 (overlay-get ov2 'after-string)))
     (progn (delete-overlay ov1) (delete-overlay ov2) 'cleaned))))"##,
    );
}

#[test]
fn ft_omega_face_color_distance_and_comparison_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'color-name-to-rgb-ok (condition-case nil (color-name-to-rgb "red") (error 'no))
   'color-name-to-rgb-blue (condition-case nil (color-name-to-rgb "blue") (error 'no))
   'color-name-to-rgb-hex (condition-case nil (color-name-to-rgb "#FF00FF") (error 'no))
   'rgb-to-hex (condition-case nil (color-rgb-to-hex 1.0 0.0 0.0 1) (error 'no))
   'rgb-to-hex-green (condition-case nil (color-rgb-to-hex 0.0 1.0 0.0 1) (error 'no))
   'color-values-red (condition-case nil (color-values "red") (error 'no))
   'color-values-nil-frame (condition-case nil (color-values "blue" nil) (error 'no))
   'color-values-frame (condition-case nil (color-values "green" (selected-frame)) (error 'no))
   'color-dark-p-black (condition-case nil (color-dark-p "#000000") (error 'no))
   'color-dark-p-white (condition-case nil (color-dark-p "#FFFFFF") (error 'no))
   'color-light-name-p-white (condition-case nil (color-light-name-p "white") (error 'no))
   'color-light-name-p-black (condition-case nil (color-light-name-p "black") (error 'no))
   'color-complement-red (condition-case nil (color-complement "red") (error 'no))
   'color-gradient (condition-case nil (color-gradient '(1 0 0) '(0 0 1) 3) (error 'no)))))"##,
    );
}

#[test]
fn ft_omega_face_text_property_search_operations_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:slant italic :weight bold))
    (list
     'text-property-any (text-property-any 1 31 'face 'bold)
     'text-property-any-italic (text-property-any 1 31 'face 'italic)
     'text-property-any-not (text-property-any 1 31 'face 'nonexistent)
     'text-property-not-all (text-property-not-all 1 31 'face 'bold)
     'text-property-not-all-start (text-property-not-all 1 31 'face 'underline)
     'next-property-change (next-property-change 1 (current-buffer))
     'next-single-property-change (next-single-property-change 1 'face)
     'previous-single-property-change (previous-single-property-change 31 'face)
     'next-single-property-change-6 (next-single-property-change 6 'face)
     'object-intervals-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_omega_face_set_fontset_font_for_charset_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'set-fontset-font-fbound (fboundp 'set-fontset-font)
   'fontset-plain-name (condition-case nil (fontset-plain-name "fontset-default") (error 'no))
   'fontset-info (condition-case nil (fontset-info "fontset-default") (error 'no))
   'create-fontset-from-fontset-spec
   (condition-case nil
       (create-fontset-from-fontset-spec
        (font-xlfd-name (font-spec :family "Monospace" :registry "iso8859-1"))
        nil 'noerror)
     (error 'no-create-fontset))
   'fontset-list (if (fboundp 'fontset-list) (length (fontset-list)) 'no-list)
   (condition-case nil
       (let ((r (set-fontset-font "fontset-default" 'latin
                                   (font-spec :family "Monospace") nil 'prepend)))
         (if r 'set-ok 'set-failed))
     (error 'no-set-fontset-font))
   'query-font (condition-case nil
                   (let ((f (query-font (font-spec :family "Monospace"))))
                     (if f 'found 'not-found))
                 (error 'no-query-font)))))"##,
    );
}

#[test]
fn ft_omega_face_face_attr_merge_with_nil_frame_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-attr-nil-frame-weight (condition-case nil (face-attribute 'default :weight nil 'default-on) (error 'no))
   'face-attr-nil-frame-fg (condition-case nil (face-attribute 'default :foreground nil 'default-on) (error 'no))
   'face-attr-frame-t-weight (condition-case nil (face-attribute 'default :weight (selected-frame) 'default-on) (error 'no))
   'face-attr-frame-t-fg (condition-case nil (face-attribute 'default :foreground (selected-frame) 'default-on) (error 'no))
   'face-attr-no-frame-no-inherit (condition-case nil (face-attribute 'default :weight) (error 'no))
   'face-font-nil-frame (condition-case nil (face-font 'default nil) (error 'no))
   'face-font-frame (condition-case nil (face-font 'default (selected-frame)) (error 'no))
   'face-font-no-frame (condition-case nil (face-font 'default) (error 'no)))))"##,
    );
}

#[test]
fn ft_omega_face_face_number_limit_and_overflow_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'facep-before (condition-case nil (facep 'default) (error 'no))
   'make-many-faces
   (let ((results nil))
     (dotimes (i 5)
       (let* ((name (intern (format "my-temp-face-%d" i)))
              (created (condition-case nil
                           (progn (make-face name) t)
                         (error nil))))
         (push (list name created (condition-case nil (facep name) (error nil))) results)))
     (nreverse results))
   'face-list-count-after (length (face-list)))))"##,
    );
}

#[test]
fn ft_omega_face_inherit_chain_and_resolution_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-inherit-base) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-base nil :weight 'bold :foreground "red") (error nil))
  (condition-case nil (copy-face 'my-inherit-base 'my-inherit-child) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-child nil :slant 'italic) (error nil))
  (condition-case nil (copy-face 'my-inherit-child 'my-inherit-grandchild) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-grandchild nil :underline t) (error nil))
  (list
   'base-weight (condition-case nil (face-attribute 'my-inherit-base :weight nil 'default-on) (error 'no))
   'base-fg (condition-case nil (face-attribute 'my-inherit-base :foreground nil 'default-on) (error 'no))
   'child-weight (condition-case nil (face-attribute 'my-inherit-child :weight nil 'default-on) (error 'no))
   'child-slant (condition-case nil (face-attribute 'my-inherit-child :slant nil 'default-on) (error 'no))
   'child-fg (condition-case nil (face-attribute 'my-inherit-child :foreground nil 'default-on) (error 'no))
   'grand-weight (condition-case nil (face-attribute 'my-inherit-grandchild :weight nil 'default-on) (error 'no))
   'grand-slant (condition-case nil (face-attribute 'my-inherit-grandchild :slant nil 'default-on) (error 'no))
   'grand-under (condition-case nil (face-attribute 'my-inherit-grandchild :underline nil 'default-on) (error 'no))
   'grand-fg (condition-case nil (face-attribute 'my-inherit-grandchild :foreground nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_max_face_empty_and_space_only_buffer_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (list
     'empty-buffer-face (get-text-property 1 'face)
     'empty-buffer-fontified (get-text-property 1 'fontified)
     'empty-text-props (text-properties-at 1)
     ;; Insert whitespace only
     (progn (insert "   ") (list 'spaces-face (get-text-property 1 'face) 'spaces-props (text-properties-at 1)))
     ;; Put face on whitespace
     (progn (put-text-property 1 4 'face 'bold) (list 'spaces-bold-face (get-text-property 1 'face)))
     ;; Insert newlines
     (progn (insert "\n\n") (list 'newline-face (get-text-property 4 'face) 'newline-props (text-properties-at 4))))))"##,
    );
}

#[test]
fn ft_max_face_cursor_intangible_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Cursor intangible face test buffer")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 8 19 'cursor-intangible t)
    (put-text-property 8 19 'face 'italic)
    (put-text-property 19 30 'face 'underline)
    (put-text-property 19 30 'cursor-intangible nil)
    (list
     'cursor-intangible-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'cursor-intangible) (get-char-property pos 'cursor-intangible))) '(1 5 10 15 20 25))
     'cursor-sensor-functions (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'cursor-sensor-functions)) '(1 10 20)))))"##,
    );
}

#[test]
fn ft_max_face_point_entered_exited_text_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Point entered exited face test buffer")
    (put-text-property 1 7 'face 'bold)
    (put-text-property 7 15 'point-entered (lambda (&rest _) (message "entered")))
    (put-text-property 7 15 'face 'italic)
    (put-text-property 15 24 'point-left (lambda (&rest _) (message "left")))
    (put-text-property 15 24 'face 'underline)
    (put-text-property 24 36 'face '(:foreground "red"))
    (list
     'faces-with-point-hooks (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'point-entered) (get-text-property pos 'point-left))) '(1 5 10 15 20 25 30))
     'all-props-at-entered (text-properties-at 10)
     'all-props-at-left (text-properties-at 20))))"##,
    );
}

#[test]
fn ft_max_face_font_lock_fontify_after_string_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Fontify after string test here")
    (font-lock-add-keywords nil '(("\\<\\(Fontify\\)\\>" 1 font-lock-warning-face t)))
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (if (search-forward needle nil t)
                              (list needle (get-text-property (match-beginning 0) 'face))
                              (list needle 'not-found))))
                      '("Fontify" "after" "string" "test"))))
      ;; Edit and re-fontify
      (goto-char (point-min))
      (search-forward "after")
      (replace-match "AFTER")
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (needle)
                          (save-excursion
                            (goto-char (point-min))
                            (if (search-forward needle nil t)
                                (list needle (get-text-property (match-beginning 0) 'face))
                                (list needle 'not-found))))
                        '("Fontify" "AFTER" "string" "test"))))
        (list v0 v1)))))"##,
    );
}

#[test]
fn ft_max_face_text_property_stickiness_advanced_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Stickiness advanced test buffer here")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 1 11 'front-sticky '(face))
    (put-text-property 1 11 'rear-nonsticky nil)
    (put-text-property 11 21 'face 'italic)
    (put-text-property 11 21 'front-sticky nil)
    (put-text-property 11 21 'rear-nonsticky '(face))
    (put-text-property 21 34 'face 'underline)
    (put-text-property 21 34 'front-sticky t)
    (put-text-property 21 34 'rear-nonsticky t)
    (list
     'initial-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'front-sticky) (get-text-property pos 'rear-nonsticky))) '(1 5 11 15 21 25 30))
     ;; Insert at front-sticky boundary
     'after-insert-front-sticky (progn
                                  (goto-char 11)
                                  (insert "X")
                                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 12 15 22))))
     ;; Insert at rear-nonsticky boundary
     'after-insert-rear-nonsticky (progn
                                    (goto-char 21)
                                    (insert "Z")
                                    (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(12 15 21 23 26 35))))))"##,
    );
}

#[test]
fn ft_max_face_with_number_conversion_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-attribute-relative-p-fbound (fboundp 'face-attribute-relative-p)
   (condition-case nil
       (face-attribute-relative-p :height)
     (error 'no-rel-p))
   (condition-case nil
       (face-attribute-relative-p :width)
     (error 'no-rel-p2))
   'face-font-family-alternatives (if (boundp 'face-font-family-alternatives)
                                       (length face-font-family-family-alternatives)
                                     'no-alternatives)
   'face-font-registry-alternatives (if (boundp 'face-font-registry-alternatives)
                                         (length face-font-registry-alternatives)
                                       'no-alternatives)
   'scalable-fonts-allowed-p (condition-case nil
                                 (and (boundp 'scalable-fonts-allowed) scalable-fonts-allowed)
                               (error 'no))
   'bitmap-fonts-allowed (if (boundp 'bitmap-fonts-allowed)
                              bitmap-fonts-allowed
                            'no-bound))))"##,
    );
}

#[test]
fn ft_ultra_face_buffer_invisibility_spec_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Visible text HIDDEN text visible")
    (put-text-property 1 14 'face 'bold)
    (put-text-property 14 20 'face 'italic)
    (put-text-property 20 28 'face 'underline)
    (add-to-invisibility-spec '(my-spec . t))
    (put-text-property 14 20 'invisible 'my-spec)
    (list
     'invisible-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible) (invisible-p pos))) '(1 10 14 17 20 25))
     'buffer-invisibility-spec buffer-invisibility-spec
     'remove-invisible (progn
                         (remove-from-invisibility-spec '(my-spec . t))
                         (put-text-property 14 20 'invisible nil)
                         (list 'after-remove (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 10 14 17 20 25))))))))"##,
    );
}

#[test]
fn ft_ultra_face_with_overlay_arrow_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay arrow face test buffer text")
    (put-text-property 1 36 'face '(:foreground "blue"))
    (let ((ov (make-overlay 10 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'arrow-position 15)
      (overlay-put ov 'arrow-string "=>"))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 15 20 25 35))
     'arrow-pos (overlay-get ov 'arrow-position)
     'arrow-string (overlay-get ov 'arrow-string)
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_ultra_face_overlay_priority_sort_stable_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay priority sort stable test text zone here now")
    (let ((ov1 (make-overlay 1 15)))
      (overlay-put ov1 'face '(:background "red"))
      (overlay-put ov1 'priority 50))
    (let ((ov2 (make-overlay 5 25)))
      (overlay-put ov2 'face '(:foreground "green"))
      (overlay-put ov2 'priority 50))
    (let ((ov3 (make-overlay 20 40)))
      (overlay-put ov3 'face '(:foreground "blue"))
      (overlay-put ov3 'priority 100))
    (let ((ov4 (make-overlay 30 52)))
      (overlay-put ov4 'face '(:background "yellow"))
      (overlay-put ov4 'priority 25))
    (list
     'same-priority-sort (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 40 45 50))
     'overlays-by-priority (sort (overlays-in 1 52) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0))))
     'overlay-count (length (overlays-in 1 52))
     (progn (mapc #'delete-overlay (overlays-in 1 52)) 'cleaned))))"##,
    );
}

#[test]
fn ft_ultra_face_intervals_with_many_property_changes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (dotimes (i 20) (insert (make-string 3 (+ ?A i))))
    (let ((colors '(:foreground "red" :foreground "green" :foreground "blue"
                    :foreground "orange" :foreground "purple")))
      (let ((i 0))
        (while (< i 60)
          (put-text-property (1+ i) (min (+ i 4) 61) 'face (nth (mod (/ i 3) 5) colors))
          (setq i (+ i 3)))))
    (list
     'face-at-positions (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 4 7 10 15 20 30 40 50 59))
     'interval-count (length (object-intervals (current-buffer)))
     'next-prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face)) '(1 5 10 20 30 50))))))"##,
    );
}

#[test]
fn ft_ultra_face_font_lock_fontified_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Fontified property tracking test")
    (list
     'before-fontify (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 10 20 30))
     'after-fontify (progn
                      (font-lock-fontify-buffer)
                      (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 10 20 30)))
     'after-unfontify (progn
                        (font-lock-unfontify-buffer)
                        (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 10 20 30)))
     'font-lock-fontified-bound (boundp 'font-lock-fontified)
     (if (boundp 'font-lock-fontified) font-lock-fontified 'no-bound)
     (if (boundp 'font-lock-fontify-region-function)
         'has-region-func
       'no-region-func)
     (if (boundp 'font-lock-unfontify-region-function)
         'has-unfontify-func
       'no-unfontify-func)))))"##,
    );
}

#[test]
fn ft_ultra_face_set_face_attribute_with_face_spec_plist_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-spec-face) (error nil))
  (list
   'set-with-plist (condition-case nil (progn (set-face-attribute 'my-spec-face nil :foreground "red" :weight 'bold :inherit 'italic) 'ok) (error 'no))
   'get-fg (condition-case nil (face-attribute 'my-spec-face :foreground nil 'default-on) (error 'no))
   'get-weight (condition-case nil (face-attribute 'my-spec-face :weight nil 'default-on) (error 'no))
   'get-inherit (condition-case nil (face-attribute 'my-spec-face :inherit nil 'default-on) (error 'no))
   'set-more (condition-case nil (progn (set-face-attribute 'my-spec-face nil :height 140 :slant 'italic :underline '(:color "red")) 'ok) (error 'no))
   'get-height (condition-case nil (face-attribute 'my-spec-face :height nil 'default-on) (error 'no))
   'get-slant (condition-case nil (face-attribute 'my-spec-face :slant nil 'default-on) (error 'no))
   'get-underline (condition-case nil (face-attribute 'my-spec-face :underline nil 'default-on) (error 'no))
   'reset (condition-case nil (progn (set-face-attribute 'my-spec-face nil :foreground 'unspecified :weight 'unspecified :inherit 'unspecified :height 'unspecified :slant 'unspecified :underline 'unspecified) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_giga_face_frame_parameter_font_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'frame-parameter-font (condition-case nil (frame-parameter nil 'font) (error 'no))
   'frame-parameter-font-backend (condition-case nil (frame-parameter nil 'font-backend) (error 'no))
   'frame-parameter-foreground-color (condition-case nil (frame-parameter nil 'foreground-color) (error 'no))
   'frame-parameter-background-color (condition-case nil (frame-parameter nil 'background-color) (error 'no))
   'frame-parameter-cursor-color (condition-case nil (frame-parameter nil 'cursor-color) (error 'no))
   'frame-parameter-border-color (condition-case nil (frame-parameter nil 'border-color) (error 'no))
   'frame-parameter-mouse-color (condition-case nil (frame-parameter nil 'mouse-color) (error 'no))
   'face-attr-font (condition-case nil (face-font 'default nil) (error 'no))
   'face-attr-font-fg (condition-case nil (face-foreground 'default nil 'default-on) (error 'no))
   'face-attr-font-bg (condition-case nil (face-background 'default nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_giga_face_with_face_resources_and_defaults_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-new-frame-defaults-fbound (fboundp 'face-new-frame-defaults)
   (condition-case nil
       (face-set-after-frame-default (selected-frame) nil)
     (error 'no-set-after))
   (if (fboundp 'face-new-frame-defaults)
       (condition-case nil
           (face-new-frame-defaults)
         (error 'no-defaults))
     'no-func)
   'custom-face-attributes (if (boundp 'custom-face-attributes)
                                (length custom-face-attributes)
                              'no-bound)
   'custom-face-format (if (boundp 'custom-face-format)
                             custom-face-format
                           'no-bound)
   (if (fboundp 'x-resolve-font-name)
       (condition-case nil
           (x-resolve-font-name "Monospace")
         (error 'no-x-resolve))
     'no-x-resolve-func)
   (if (fboundp 'face-spec-recalc)
       (condition-case nil
           (face-spec-recalc 'default (selected-frame))
         (error 'no-recalc))
     'no-recalc-func))))"##,
    );
}

#[test]
fn ft_giga_face_text_property_at_point_and_char_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Point and char property comparison test")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 8 14 'face 'italic)
    (put-text-property 8 14 'font-lock-face 'underline)
    (put-text-property 14 22 'face '(:foreground "red"))
    (put-text-property 22 31 'face '(:background "yellow"))
    (put-text-property 22 31 'font-lock-face nil)
    (let ((ov (make-overlay 10 20)))
      (overlay-put ov 'face '(:weight bold :slant italic))
      (overlay-put ov 'priority 100))
    (list
     'get-text-property (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 9 12 15 20 25 30))
     'get-char-property (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'font-lock-face))) '(1 5 9 12 15 20 25 30))
     'get-char-property-and-overlay (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property-and-overlay pos 'face))) '(1 5 9 12 15 20 25 30))
     'char-after (mapcar (lambda (pos) (goto-char pos) (char-after pos)) '(1 9 15 22))
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_giga_face_with_overlay_insert_in_front_behind_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay insert in front behind test")
    (put-text-property 1 38 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'insert-in-front-hooks '(ignore))
      (overlay-put ov1 'insert-behind-hooks '(ignore)))
    (list
     'overlay-with-hooks (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'face))) '(1 5 10 15 20 30 37))
     'overlay-hooks (list (overlay-get ov1 'insert-in-front-hooks) (overlay-get ov1 'insert-behind-hooks))
     (progn (delete-overlay ov1) 'cleaned))))"##,
    );
}

#[test]
fn ft_giga_face_multi_byte_string_with_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "αβγ 日本語 التصميم 🌍🌎🌏 конец")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 19 'face 'underline)
    (put-text-property 19 29 'face '(:foreground "red"))
    (put-text-property 29 33 'face '(:background "yellow"))
    (list
     'faces-multibyte (mapcar (lambda (pos)
                                (goto-char pos)
                                (list pos (char-after pos) (get-text-property pos 'face) (char-width (char-after pos))))
                              '(1 2 3 5 7 12 22 30 33))
     'prop-changes-multibyte (next-single-property-change 1 'face)
     'previous-prop-change (previous-single-property-change 33 'face)
     'buf-string-length (length (buffer-string))))))"##,
    );
}

#[test]
fn ft_giga_face_with_property_list_remove_list_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Remove list of properties face test zone area")
    (add-text-properties 1 45 (list 'face 'bold 'key1 'val1 'key2 'val2 'key3 'val3))
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key1) (get-text-property pos 'key2) (get-text-property pos 'key3))) '(1 10 20 30 40))
     'remove-face-key (progn
                        (remove-list-of-text-properties 1 45 '(face key1))
                        (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key1) (get-text-property pos 'key2) (get-text-property pos 'key3))) '(1 10 20 30 40)))
     'remove-rest (progn
                    (remove-list-of-text-properties 1 45 '(key2 key3))
                    (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'key2) (get-text-property pos 'key3))) '(1 10 20 30 40)))
     're-add (progn
               (put-text-property 1 20 'face 'italic)
               (put-text-property 20 45 'face 'underline)
               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 20 30 40))))))"##,
    );
}
