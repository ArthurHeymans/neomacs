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
     (let ((pos 1) (result nil) (limit (point-max)))
       (while (and pos (< pos limit))
         (setq pos (next-single-property-change pos 'face nil limit))
         (when (and pos (< pos limit))
           (push (list pos (get-text-property pos 'face)) result)))
       (nreverse result))
     'previous-face-changes
     (let ((pos 17) (result nil) (limit (point-min)))
       (while (and pos (> pos limit))
         (setq pos (previous-single-property-change pos 'face nil limit))
         (when (and pos (> pos limit))
           (push (list pos (get-text-property pos 'face)) result)))
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

#[test]
fn ft_tera_face_buffer_swapping_text_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((buf1 (generate-new-buffer "*ft-swap1*"))
        (buf2 (generate-new-buffer "*ft-swap2*")))
    (unwind-protect
        (progn
          (with-current-buffer buf1
            (insert "Buffer one text with face")
            (put-text-property 1 13 'face 'bold)
            (put-text-property 13 23 'face 'italic))
          (with-current-buffer buf2
            (insert "Buffer two text with face")
            (put-text-property 1 13 'face 'underline)
            (put-text-property 13 23 'face '(:foreground "red")))
          (list
           'buf1-faces (with-current-buffer buf1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 13 18)))
           'buf2-faces (with-current-buffer buf2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 13 18)))))
      (kill-buffer buf1)
      (kill-buffer buf2))))"##,
    );
}

#[test]
fn ft_tera_face_indirect_buffer_clone_text_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Indirect buffer face propagation test")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 11 21 'face 'italic)
    (put-text-property 21 32 'face 'underline)
    (put-text-property 32 41 'face '(:foreground "red"))
    (let* ((clone-name (generate-new-buffer-name "*ft-clone*"))
           (clone (make-indirect-buffer (current-buffer) clone-name t)))
      (unwind-protect
          (list
           'base-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 11 15 21 25 32 38))
           'clone-faces (with-current-buffer clone
                          (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 11 15 21 25 32 38)))
           ;; Edit in clone
           (progn
             (with-current-buffer clone
               (goto-char 11)
               (insert "INSERTED")
               (put-text-property 11 19 'face '(:background "yellow"))
               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 11 15 19 25))))
           ;; Check base after clone edit
           'base-after-clone-edit (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 11 15 21 30)))
        (when (get-buffer clone-name) (kill-buffer clone-name))))))"##,
    );
}

#[test]
fn ft_tera_face_with_overlay_isearch_open_overlay_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Isearch overlay face test buffer content here now")
    (put-text-property 1 47 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'isearch-open-invisible 'ignore))
    (let ((ov2 (make-overlay 20 35)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold))
      (overlay-put ov2 'isearch-open-invisible-temporary 'ignore))
    (list
     'faces-with-isearch (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'isearch-open-invisible))) '(1 5 10 15 20 25 30 40 46))
     'overlay-props (list (overlay-get ov1 'isearch-open-invisible) (overlay-get ov2 'isearch-open-invisible-temporary))
     (progn (delete-overlay ov1) (delete-overlay ov2) 'cleaned))))"##,
    );
}

#[test]
fn ft_tera_face_overlay_display_margin_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay display margin face test buffer")
    (put-text-property 1 42 'face '(:foreground "blue"))
    (let ((ov (make-overlay 5 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'display '(left-fringe right-arrow))
      (overlay-put ov 'line-prefix "> "))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'display) (get-char-property pos 'line-prefix))) '(1 5 10 15 20 25 30 40))
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_tera_face_with_compose_region_decompose_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((text "compose-test"))
      (insert text)
      (put-text-property 1 13 'face 'bold)
      (condition-case nil
          (list
           'before-compose (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'composition))) '(1 5 10 12))
           'after-compose (progn
                            (compose-region 1 13 "COMPOSED")
                            (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'composition))) '(1 5 10 12)))
           'after-decompose (progn
                              (decompose-region 1 13)
                              (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'composition))) '(1 5 10 12))))
        (error (list
                'before-compose (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'composition))) '(1 5 10 12))
                'compose-error (fboundp 'compose-region))))))))"##,
    );
}

#[test]
fn ft_tera_face_font_lock_prepend_vs_append_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "PREPEND and APPEND keyword test")
    (font-lock-add-keywords nil
                            '(("\\<\\(PREPEND\\)\\>" 1 font-lock-warning-face prepend)
                              ("\\<\\(APPEND\\)\\>" 1 '(:foreground "red" :weight bold) append)))
    (font-lock-fontify-buffer)
    (list
     'prepend-face (save-excursion (goto-char (point-min)) (search-forward "PREPEND") (get-text-property (match-beginning 0) 'face))
     'append-face (save-excursion (goto-char (point-min)) (search-forward "APPEND") (get-text-property (match-beginning 0) 'face))
     'keyword-override (save-excursion (goto-char (point-min)) (search-forward "keyword") (get-text-property (match-beginning 0) 'face))
     'test-override (save-excursion (goto-char (point-min)) (search-forward "test") (get-text-property (match-beginning 0) 'face)))))"##,
    );
}

#[test]
fn ft_edge_indirect_buffer_face_edit_clone_edit_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (require 'face-remap)
    (insert "Indirect buffer multi-edit face test content here now")
    (put-text-property 1 13 'face 'bold)
    (put-text-property 13 25 'face 'italic)
    (put-text-property 25 38 'face 'underline)
    (put-text-property 38 51 'face '(:foreground "red" :weight bold))
    (let* ((clone-name (generate-new-buffer-name "*ft-edge-clone*"))
           (clone (make-indirect-buffer (current-buffer) clone-name t))
           (snap (lambda (buf)
                   (with-current-buffer buf
                     (list
                      (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 13 19 25 30 40 48))
                      (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 13 25 38))
                      (length (object-intervals buf)))))))
      (unwind-protect
          (let ((v0 (funcall snap (current-buffer)))
                (v0c (funcall snap clone)))
            ;; Edit in base: change some faces
            (goto-char 13)
            (put-text-property 13 25 'face '(:background "yellow" :weight bold))
            (let ((v1 (funcall snap (current-buffer)))
                  (v1c (funcall snap clone)))
              ;; Edit in clone: add overlay
              (with-current-buffer clone
                (let ((ov (make-overlay 20 35)))
                  (overlay-put ov 'face '(:foreground "green"))
                  (overlay-put ov 'priority 50)))
              (let ((v2 (funcall snap (current-buffer)))
                    (v2c (funcall snap clone)))
                ;; Edit in base: delete region
                (delete-region 10 30)
                (let ((v3 (funcall snap (current-buffer)))
                      (v3c (funcall snap clone)))
                  (list v0 v0c v1 v1c v2 v2c v3 v3c))))))
      (when (get-buffer clone-name) (kill-buffer clone-name))))))"##,
    );
}

#[test]
fn ft_edge_overlay_priority_insert_delete_advance_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 41 'face '(:foreground "gray"))
    (let ((ovs nil)
          (snap (lambda ()
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 4 8 12 16 20 24 28 32 36 40)))))
      ;; Create 5 overlapping overlays with different priorities
      (push (let ((ov (make-overlay 1 10))) (overlay-put ov 'face '(:background "red")) (overlay-put ov 'priority 10) ov) ovs)
      (push (let ((ov (make-overlay 6 16))) (overlay-put ov 'face '(:foreground "green")) (overlay-put ov 'priority 20) ov) ovs)
      (push (let ((ov (make-overlay 12 22))) (overlay-put ov 'face '(:background "blue")) (overlay-put ov 'priority 30) ov) ovs)
      (push (let ((ov (make-overlay 18 28))) (overlay-put ov 'face '(:foreground "orange")) (overlay-put ov 'priority 15) ov) ovs)
      (push (let ((ov (make-overlay 24 34))) (overlay-put ov 'face '(:background "yellow")) (overlay-put ov 'priority 25) ov) ovs)
      (let ((v0 (funcall snap)))
        ;; Insert text at overlap point
        (goto-char 10)
        (insert "INSERT")
        (let ((v1 (funcall snap)))
          ;; Delete text at another overlap
          (delete-region 20 26)
          (let ((v2 (funcall snap)))
            ;; Change priority of first overlay
            (overlay-put (car (last ovs)) 'priority 100)
            (let ((v3 (funcall snap)))
              ;; Advance all overlays
              (mapc (lambda (ov) (condition-case nil (overlay-put ov 'advance-both t) (error nil))) ovs)
              (goto-char 15)
              (insert "ADV")
              (let ((v4 (funcall snap)))
                (mapc #'delete-overlay (overlays-in 1 (point-max)))
                (list v0 v1 v2 v3 v4)))))))))"##,
    );
}

#[test]
fn ft_edge_text_property_single_char_boundaries_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOP")
    ;; Single char faces
    (put-text-property 1 2 'face 'bold)
    (put-text-property 2 3 'face 'italic)
    (put-text-property 3 4 'face 'underline)
    (put-text-property 5 6 'face '(:foreground "red"))
    (put-text-property 6 7 'face '(:background "yellow"))
    (put-text-property 7 8 'face '(:foreground "blue" :weight bold))
    (put-text-property 9 10 'face 'bold)
    (put-text-property 10 11 'face 'italic)
    (put-text-property 12 13 'face 'underline)
    (put-text-property 13 14 'face '(:foreground "green"))
    (put-text-property 15 16 'face '(:background "cyan"))
    (list
     'single-char-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 5 6 7 9 10 12 13 15 16))
     'no-face-chars (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(4 8 11 14))
     'prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face)) '(1 2 3 5 10 15))
     'interval-count (length (object-intervals (current-buffer)))
     ;; Insert between single chars
     'after-insert (progn
                     (goto-char 4)
                     (insert "X")
                     (goto-char 11)
                     (insert "Y")
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 4 5 6 9 11 12 13 15 16))))))"##,
    );
}

#[test]
fn ft_edge_face_after_kill_yank_multiple_cycles_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "KILL CYCLE ONE   KILL CYCLE TWO   KILL CYCLE THREE")
    (put-text-property 1 17 'face 'bold)
    (put-text-property 17 22 'face 'italic)
    (put-text-property 22 39 'face 'underline)
    (put-text-property 39 47 'face '(:foreground "red" :weight bold))
    (let ((snap (lambda ()
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 17 20 22 30 39 45)))))
      (let ((v0 (funcall snap)))
        ;; Kill region 1
        (goto-char 10)
        (kill-region (point) (+ (point) 7))
        (let ((v1 (funcall snap)))
          ;; Yank
          (goto-char (point-max))
          (yank)
          (let ((v2 (funcall snap)))
            ;; Kill region 2
            (goto-char 30)
            (kill-region (point) (+ (point) 9))
            (let ((v3 (funcall snap)))
              ;; Yank at position 5
              (goto-char 5)
              (yank)
              (let ((v4 (funcall snap)))
                (list v0 v1 v2 v3 v4)))))))))"##,
    );
}

#[test]
fn ft_edge_face_font_lock_fontify_buffer_partial_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Fontify partial buffer test text here")
    ;; Fontify only first half
    (font-lock-fontify-region 1 18)
    (list
     'fontified-first-half (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 17))
     'not-fontified-last (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(20 25 30 35))
     ;; Fontify remainder
     'after-full-fontify (progn
                           (font-lock-fontify-region 18 36)
                           (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 5 10 15 17 20 25 30 35)))
     ;; Unfontify and re-fontify
     'after-unfontify-refontify (progn
                                  (font-lock-unfontify-region 1 18)
                                  (font-lock-fontify-region 1 36)
                                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 10 20 30 35))))))"##,
    );
}

#[test]
fn ft_edge_face_with_overlay_start_end_markers_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay start end marker face test buffer content here")
    (put-text-property 1 52 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 10 nil t nil)))
      (overlay-put ov1 'face '(:background "yellow")))
    (let ((ov2 (make-overlay 20 35 nil nil t)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold)))
    (let ((ov3 (make-overlay 40 52 t t nil)))
      (overlay-put ov3 'face '(:slant italic)))
    (list
     'front-advance-only (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 10 15 20))
     'rear-advance-only (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(15 20 25 30 35 40))
     'both-advance (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(40 45 50 52))
     ;; Insert at front-advance boundary
     'after-insert (progn
                     (goto-char 10)
                     (insert "NEW")
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 13 15 20 30 40)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_edge_face_with_very_long_property_name_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Long property name face test")
    (let ((prop-name (intern (concat "my-very-" (make-string 20 ?x) "-property")))
          (face-name (intern (concat "face-" (make-string 20 ?y)))))
      (put-text-property 1 7 'face 'bold)
      (put-text-property 1 7 prop-name "value")
      (list
       'long-prop-name prop-name
       'face-at-pos (get-text-property 1 'face)
       'long-prop-value (get-text-property 1 prop-name)
       'text-props-count (length (text-properties-at 1))
       'long-prop-name-length (length (symbol-name prop-name))))))"##,
    );
}

#[test]
fn ft_edge_face_remove_all_properties_from_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Remove all properties from this region test buffer")
    (put-text-property 1 30 'face 'bold)
    (put-text-property 1 30 'key1 'val1)
    (put-text-property 1 30 'key2 'val2)
    (put-text-property 30 50 'face 'italic)
    (put-text-property 30 50 'key3 'val3)
    (list
     'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (length (text-properties-at pos)))) '(1 15 30 40 49))
     ;; Remove ALL properties from first half
     'after-remove-all (progn
                         (set-text-properties 1 30 nil)
                         (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (length (text-properties-at pos)))) '(1 15 30 40 49)))
     ;; Add new properties
     'after-add-new (progn
                      (put-text-property 1 30 'face 'underline)
                      (put-text-property 1 30 'new-prop 'new-val)
                      (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'new-prop) (length (text-properties-at pos)))) '(1 15 30 40 49))))))"##,
    );
}

#[test]
fn ft_extreme_edge_overlay_make_overlay_empty_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Empty overlay region test buffer content here now end")
    (put-text-property 1 51 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 10 10)))
      (overlay-put ov1 'face '(:background "yellow")))
    (let ((ov2 (make-overlay 25 26)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold)))
    (list
     'empty-overlay (list 'start (overlay-start ov1) 'end (overlay-end ov1) 'face (overlay-get ov1 'face))
     'single-char-overlay (list 'start (overlay-start ov2) 'end (overlay-end ov2) 'face (overlay-get ov2 'face))
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(5 10 15 25 30 40 50))
     ;; Insert at empty overlay position
     'after-insert (progn
                     (goto-char 10)
                     (insert "FILL")
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 12 15 25 30 50)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_extreme_edge_face_conditional_face_spec_all_types_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (list
   'face-spec-choose-color
   (condition-case nil
       (face-spec-choose '(((class color) (min-colors 88) (background light)) (:foreground "black" :background "white")
                           ((class color) (min-colors 88) (background dark)) (:foreground "white" :background "black")
                           ((class color) (min-colors 8)) (:foreground "green" :background "black")
                           (t (:foreground "black" :background "white"))))
     (error 'no))
   'face-spec-choose-mono
   (condition-case nil
       (face-spec-choose '(((class mono)) (:foreground "black" :background "white")
                           ((class color)) (:foreground "blue" :background "yellow")
                           (t (:foreground "green"))))
     (error 'no))
   'face-spec-choose-type
   (condition-case nil
       (face-spec-choose '(((type x)) (:weight bold)
                           ((type w32)) (:weight extra-bold)
                           ((type ns)) (:weight heavy)
                           (t (:weight normal))))
     (error 'no))
   'display-type (display-graphic-p)
   (if (fboundp 'window-system) (window-system) 'no-ws))))"##,
    );
}

#[test]
fn ft_extreme_edge_overlays_at_point_position_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlays at point and in region test data here now")
    (put-text-property 1 52 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 5 15))) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 10 25))) (overlay-put ov2 'face '(:foreground "green")))
    (let ((ov3 (make-overlay 20 35))) (overlay-put ov3 'face '(:background "blue")))
    (let ((ov4 (make-overlay 30 45))) (overlay-put ov4 'face '(:foreground "orange" :weight bold)))
    (list
     'overlays-at (mapcar (lambda (pos) (goto-char pos) (list pos (length (overlays-at pos)))) '(1 5 10 12 15 20 22 25 30 35 40 45 50))
     'overlays-in (mapcar (lambda (start end) (list start end (length (overlays-in start end)))) '((1 15) (10 25) (20 35) (30 45) (1 52)))
     'next-overlay-change (mapcar (lambda (pos) (next-overlay-change pos)) '(1 5 10 15 20 25 30 35))
     'previous-overlay-change (mapcar (lambda (pos) (previous-overlay-change pos)) '(5 15 25 35 45 52))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_extreme_edge_face_buffer_substring_with_faces_roundtrip_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Buffer substring face roundtrip test content area")
    (put-text-property 1 17 'face 'bold)
    (put-text-property 17 24 'face 'italic)
    (put-text-property 24 32 'face 'underline)
    (put-text-property 32 44 'face '(:foreground "red"))
    (put-text-property 44 49 'face '(:background "yellow" :weight bold))
    (list
     'source-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 17 20 24 28 32 40 44 47))
     'substring-copy
     (let ((sub (buffer-substring 1 32)))
       (with-temp-buffer
         (insert sub)
         (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 17 20 24 28))))
     'buffer-string-copy
     (let ((all (buffer-string)))
       (with-temp-buffer
         (insert all)
         (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 17 24 32 40 44))))
     'no-properties-copy
     (buffer-substring-no-properties 1 49)
     'insert-buffer-substring
     (with-temp-buffer
       (insert-buffer-substring (current-buffer) 1 49)
       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 17 24 32 40 44))))))"##,
    );
}

#[test]
fn ft_extreme_edge_face_with_font_lock_multiple_modes_switch_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (insert "Mode switch font-lock test buffer content")
    (fundamental-mode)
    (font-lock-mode 1)
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30))))
      ;; Switch to text-mode
      (text-mode)
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30))))
        ;; Switch to emacs-lisp-mode
        (emacs-lisp-mode)
        (font-lock-fontify-buffer)
        (let ((v2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30))))
          (list v0 v1 v2))))))"##,
    );
}

#[test]
fn ft_extreme_edge_text_properties_after_set_text_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Set text properties comprehensive test region")
    (put-text-property 1 10 'key-old 'val-old)
    (put-text-property 1 10 'face 'bold)
    (put-text-property 10 25 'face 'italic)
    (put-text-property 25 44 'face 'underline)
    (list
     'before-set (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key-old))) '(1 5 10 15 25 35 42))
     ;; Set text properties (replaces all)
     'after-set (progn
                  (set-text-properties 1 25 (list 'face '(:foreground "red" :weight bold) 'new-key 'new-val))
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key-old) (get-text-property pos 'new-key))) '(1 5 10 15 20 25 30 42)))
     ;; Set again with different properties
     'after-set-again (progn
                        (set-text-properties 1 44 (list 'face '(:background "yellow") 'another-key 'another-val))
                        (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'new-key) (get-text-property pos 'another-key))) '(1 10 20 30 40 42))))))"##,
    );
}

#[test]
fn ft_extreme_edge_face_inherit_through_multiple_levels_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  ;; Create face inheritance chain: base -> child -> grandchild -> greatgrandchild
  (condition-case nil (copy-face 'default 'my-level1-face) (error nil))
  (condition-case nil (set-face-attribute 'my-level1-face nil :weight 'bold :foreground "blue") (error nil))
  (condition-case nil (copy-face 'my-level1-face 'my-level2-face) (error nil))
  (condition-case nil (set-face-attribute 'my-level2-face nil :slant 'italic :inherit 'my-level1-face) (error nil))
  (condition-case nil (copy-face 'my-level2-face 'my-level3-face) (error nil))
  (condition-case nil (set-face-attribute 'my-level3-face nil :underline t :inherit 'my-level2-face) (error nil))
  (condition-case nil (copy-face 'my-level3-face 'my-level4-face) (error nil))
  (condition-case nil (set-face-attribute 'my-level4-face nil :box t :inherit 'my-level3-face) (error nil))
  (list
   'level1-fg (face-attribute 'my-level1-face :foreground nil 'default-on)
   'level1-weight (face-attribute 'my-level1-face :weight nil 'default-on)
   'level2-fg (face-attribute 'my-level2-face :foreground nil 'default-on)
   'level2-weight (face-attribute 'my-level2-face :weight nil 'default-on)
   'level2-slant (face-attribute 'my-level2-face :slant nil 'default-on)
   'level3-fg (face-attribute 'my-level3-face :foreground nil 'default-on)
   'level3-weight (face-attribute 'my-level3-face :weight nil 'default-on)
   'level3-under (face-attribute 'my-level3-face :underline nil 'default-on)
   'level4-fg (face-attribute 'my-level4-face :foreground nil 'default-on)
   'level4-box (face-attribute 'my-level4-face :box nil 'default-on)
   'level4-under (face-attribute 'my-level4-face :underline nil 'default-on))))"##,
    );
}

#[test]
fn ft_extreme_edge_face_after_text_property_remove_specific_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Remove specific properties from textured region")
    (add-text-properties 1 49 (list 'face 'bold 'key1 'val1 'key2 'val2 'key3 'val3 'key4 'val4 'fontified t))
    (remove-text-properties 10 30 '(key1 nil key2 nil))
    (remove-text-properties 20 40 '(face nil key3 nil))
    (list
     'face-after-removals (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25 30 35 40 45 48))
     'key1-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'key1)) '(1 5 10 15 25 40))
     'key2-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'key2)) '(1 5 10 15 25 40))
     'key3-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'key3)) '(1 5 10 15 25 40))
     'key4-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'key4)) '(1 10 20 30 40))
     'fontified-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30 40 48)))))"##,
    );
}

#[test]
fn ft_extreme_face_overlay_interval_split_merge_face_transition() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:foreground "blue"))
    (put-text-property 31 36 'face '(:background "cyan"))
    (let ((ov1 (make-overlay 3 14))) (overlay-put ov1 'face '(:weight bold)) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 13 24))) (overlay-put ov2 'face '(:slant italic)) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 23 34))) (overlay-put ov3 'face '(:underline t)) (overlay-put ov3 'priority 15))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-char-property pos 'face))) '(1 4 8 12 16 20 24 28 32 35)))))
      (let ((v0 (funcall snap)))
        ;; Split interval by inserting at boundary
        (goto-char 11) (insert "ZZZZZ")
        (let ((v1 (funcall snap)))
          ;; Merge intervals by deleting boundary region
          (delete-region 18 28)
          (let ((v2 (funcall snap)))
            ;; Split again differently
            (goto-char 8) (insert "YYYY")
            (let ((v3 (funcall snap)))
              ;; Delete all overlays, check text-only face
              (mapc #'delete-overlay (overlays-in 1 (point-max)))
              (let ((v4 (funcall snap)))
                (list v0 v1 v2 v3 v4)))))))))"##,
    );
}

#[test]
fn ft_extreme_overlay_variable_width_zero_length_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Zero width overlay boundary test content")
    (put-text-property 1 39 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 1)))
      (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 38 39)))
      (overlay-put ov2 'face '(:background "yellow")))
    (let ((ov3 (make-overlay 20 20)))
      (overlay-put ov3 'face '(:foreground "green" :weight bold)))
    (list
     'before-insert (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 2 20 21 38 39))
     ;; Insert at beginning where zero-width overlay sits
     'after-insert-beginning (progn
                               (goto-char 1) (insert "START ")
                               (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 38)))
     ;; Insert at middle zero-width
     'after-insert-middle (progn
                            (goto-char 25) (insert "MIDDLE")
                            (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(25 28 31 35 40)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_extreme_face_text_property_at_point_min_and_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property at buffer boundaries test text")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 35 39 'face 'italic)
    (list
     'at-point-min (text-properties-at 1)
     'at-eob (text-properties-at (point-max))
     'before-point-min (text-properties-at 0)
     'after-eob (text-properties-at (+ (point-max) 1))
     'face-at-1 (get-text-property 1 'face)
     'face-at-min (get-text-property (point-min) 'face)
     'face-at-max (get-text-property (point-max) 'face)
     'face-at-0 (get-text-property 0 'face)
     'face-beyond-max (get-text-property (+ (point-max) 1) 'face)
     'next-prop-from-min (next-single-property-change (point-min) 'face)
     'prev-prop-from-max (previous-single-property-change (point-max) 'face nil (point-min))))))"##,
    );
}

#[test]
fn ft_extreme_face_add_text_properties_then_partial_remove_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Partial property removal from textured content buffer")
    (add-text-properties 1 55 (list 'face '(:foreground "blue" :weight bold)
                                     'layer1 'val1 'layer2 'val2 'layer3 'val3))
    ;; Remove layer1 from first third
    (remove-text-properties 1 18 '(layer1 nil))
    ;; Remove face from middle third
    (remove-text-properties 18 36 '(face nil))
    ;; Remove layer2 from last third
    (remove-text-properties 36 55 '(layer2 nil))
    ;; Add face back to middle but with different value
    (put-text-property 18 36 'face '(:background "yellow" :slant italic))
    (list
     'face-across (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 18 25 36 45 54))
     'layer1-across (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'layer1)) '(1 10 18 25 36 45 54))
     'layer2-across (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'layer2)) '(1 10 18 25 36 45 54))
     'layer3-across (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'layer3)) '(1 10 18 25 36 45 54))
     'text-props-at-boundaries (mapcar (lambda (pos) (goto-char pos) (length (text-properties-at pos))) '(1 18 36 54)))))"##,
    );
}

#[test]
fn ft_extreme_overlay_evaporate_in_chain_with_textprop_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Evaporating overlays in text property chain test buffer text")
    (put-text-property 1 55 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 10)))
      (overlay-put ov1 'face '(:background "red"))
      (overlay-put ov1 'evaporate t))
    (let ((ov2 (make-overlay 10 20)))
      (overlay-put ov2 'face '(:background "green"))
      (overlay-put ov2 'evaporate t))
    (let ((ov3 (make-overlay 20 30)))
      (overlay-put ov3 'face '(:background "yellow"))
      (overlay-put ov3 'evaporate nil))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 10 15 20 25 30 35 40 50)))))
      (let ((v0 (funcall snap)))
        ;; Delete region that should evaporate ov1 and ov2
        (delete-region 5 25)
        (let ((v1 (funcall snap)))
          ;; Insert new text - ov3 should persist
          (goto-char 10) (insert "PERSISTENT-TEXT")
          (let ((v2 (funcall snap)))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_extreme_face_with_keyword_matching_multiple_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "function myFunc(param1, param2) { return param1 + param2; }")
    (font-lock-add-keywords nil
                            '(("\\<\\(function\\)\\>" 1 font-lock-keyword-face t)
                              ("\\<\\(return\\)\\>" 1 font-lock-keyword-face t)
                              ("\\<\\(myFunc\\)\\>" 1 font-lock-function-name-face t)
                              ("\\<\\(param[12]\\)\\>" 1 font-lock-variable-name-face t)))
    (font-lock-fontify-buffer)
    (list
     'keyword-face (save-excursion (goto-char (point-min)) (search-forward "function") (get-text-property (match-beginning 0) 'face))
     'func-name-face (save-excursion (goto-char (point-min)) (search-forward "myFunc") (get-text-property (match-beginning 0) 'face))
     'var1-face (save-excursion (goto-char (point-min)) (search-forward "param1") (get-text-property (match-beginning 0) 'face))
     'var2-face (save-excursion (goto-char (point-min)) (search-forward "param2") (get-text-property (match-beginning 0) 'face))
     'return-face (save-excursion (goto-char (point-min)) (search-forward "return") (get-text-property (match-beginning 0) 'face))
     'syntax-chars-face (save-excursion (goto-char (point-min)) (search-forward "{") (get-text-property (match-beginning 0) 'face))
     'non-keyword-face (save-excursion (goto-char (point-min)) (search-forward "+") (get-text-property (match-beginning 0) 'face)))))"##,
    );
}

#[test]
fn ft_extreme_face_with_face_spec_set_then_reset_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (condition-case nil (copy-face 'default 'my-spec-reset-face) (error nil))
  (list
   'before-spec (face-attribute 'my-spec-reset-face :weight nil 'default-on)
   'set-spec (condition-case nil
                 (face-spec-set 'my-spec-reset-face '((t :weight bold :foreground "red")) 'face-defface-spec)
               (error 'no-set))
   'after-spec (face-attribute 'my-spec-reset-face :weight nil 'default-on)
   'reset-spec (condition-case nil
                   (face-spec-set 'my-spec-reset-face '((t :weight normal :foreground "black")) 'face-defface-spec)
                 (error 'no-reset))
   'after-reset (face-attribute 'my-spec-reset-face :weight nil 'default-on)
   'reset-to-defaults (condition-case nil
                          (face-spec-set 'my-spec-reset-face '((t)) 'face-defface-spec)
                        (error 'no-reset2))
   'after-reset-default (face-attribute 'my-spec-reset-face :weight nil 'default-on))))"##,
    );
}

#[test]
fn ft_extreme_face_font_lock_with_overlay_priority_interleaving() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Overlay priority interleaving with font-lock faces test")
    (font-lock-add-keywords nil '(("\\<\\(Overlay\\)\\>" 1 font-lock-warning-face t)
                                  ("\\<\\(priority\\)\\>" 1 '(:foreground "red") t)
                                  ("\\<\\(font-lock\\)\\>" 1 font-lock-keyword-face t)))
    (font-lock-fontify-buffer)
    (let ((ov1 (make-overlay 1 9)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'priority 100))
    (let ((ov2 (make-overlay 20 30)))
      (overlay-put ov2 'face '(:weight bold))
      (overlay-put ov2 'priority -1))
    (let ((ov3 (make-overlay 35 45)))
      (overlay-put ov3 'face '(:foreground "green"))
      (overlay-put ov3 'priority 50))
    (list
     'faces-all (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-char-property pos 'face))) '(1 5 8 15 20 25 30 35 40 45 55))
     'overlay-counts (mapcar (lambda (pos) (goto-char pos) (length (overlays-at pos))) '(1 8 25 40))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_hyper_face_cache_invalidation_after_face_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-cache-face) (error nil))
  (list
   'before-set (face-attribute 'my-cache-face :weight nil 'default-on)
   'set-attr (condition-case nil (set-face-attribute 'my-cache-face nil :weight 'heavy) (error 'no))
   'after-set (face-attribute 'my-cache-face :weight nil 'default-on)
   'set-fg (condition-case nil (set-face-foreground 'my-cache-face "DarkSlateBlue" nil) (error 'no))
   'get-fg (condition-case nil (face-foreground 'my-cache-face nil 'default-on) (error 'no))
   'clear-cache (condition-case nil (clear-face-cache) (error 'no))
   'after-clear-weight (face-attribute 'my-cache-face :weight nil 'default-on)
   'after-clear-fg (condition-case nil (face-foreground 'my-cache-face nil 'default-on) (error 'no))
   'set-again (condition-case nil (set-face-attribute 'my-cache-face nil :weight 'ultra-light :slant 'oblique) (error 'no))
   'get-weight-again (face-attribute 'my-cache-face :weight nil 'default-on)
   'get-slant-again (face-attribute 'my-cache-face :slant nil 'default-on))))"##,
    );
}

#[test]
fn ft_hyper_face_with_display_table_slot_face_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'display-table-fbound (fboundp 'display-table-slot)
   'set-display-table-slot-fbound (fboundp 'set-display-table-slot)
   (condition-case nil
       (let ((dt (make-display-table)))
         (set-display-table-slot dt 0 ?a)
         (display-table-slot dt 0))
     (error 'no-display-table-ops))
   (condition-case nil
       (let ((dt (make-display-table)))
         (set-display-table-slot dt 'selective-display
                                 (vector (make-glyph-code ?- 'highlight)))
         'set-glyph-ok)
     (error 'no-set-glyph))
   'describe-display-table (if (fboundp 'describe-display-table) 'fbound 'not-fbound)
   'standard-display-table (if (boundp 'standard-display-table)
                                standard-display-table
                              'no-standard-table)
   'buffer-display-table (if (boundp 'buffer-display-table)
                              buffer-display-table
                            'no-buffer-table))))"##,
    );
}

#[test]
fn ft_hyper_face_font_lock_fontify_syntactically_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (c-mode)
    (insert "/* block comment */ int main() { return 0; }\n")
    (insert "// line comment\nint x = 1;\n")
    (font-lock-fontify-syntactically (point-min) (point-max) nil)
    (list
     'block-comment-face (save-excursion (goto-char (point-min)) (search-forward "block") (get-text-property (match-beginning 0) 'face))
     'line-comment-face (save-excursion (goto-char (point-min)) (search-forward "line") (get-text-property (match-beginning 0) 'face))
     'int-face (save-excursion (goto-char (point-min)) (search-forward "int main") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))
     'fontified-regions (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 15 30 45)))))"##,
    );
}

#[test]
fn ft_hyper_face_with_overlay_created_from_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay created from text property face test area now")
    (put-text-property 1 17 'face 'bold)
    (put-text-property 17 36 'face 'italic)
    (put-text-property 36 53 'face 'underline)
    ;; Create overlays that mirror text property boundaries
    (let ((ov1 (make-overlay 1 17)))
      (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 17 36)))
      (overlay-put ov2 'face '(:background "green")))
    (let ((ov3 (make-overlay 36 53)))
      (overlay-put ov3 'face '(:background "yellow")))
    (list
     'text-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 17 25 36 45 52))
     'char-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 17 25 36 45 52))
     ;; Remove text props, keep overlays
     'text-props-removed (progn
                           (set-text-properties 1 53 nil)
                           (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-char-property pos 'face))) '(1 10 17 25 36 45 52)))
     ;; Remove overlays
     (progn (mapc #'delete-overlay (overlays-in 1 53))
            (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 17 36 45 52))))))"##,
    );
}

#[test]
fn ft_hyper_face_font_lock_string_literal_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(format \"hello %s world %d\" name count)\n")
    (insert "(concat \"multi-\" \"line-\" \"string\")\n")
    (insert "(defun f () \"docstring here\" nil)\n")
    (font-lock-fontify-buffer)
    (list
     'format-string-face (save-excursion (goto-char (point-min)) (search-forward "hello") (get-text-property (match-beginning 0) 'face))
     'multi-string-fontified (save-excursion (goto-char (point-min)) (search-forward "multi") (get-text-property (match-beginning 0) 'fontified))
     'docstring-face (save-excursion (goto-char (point-min)) (search-forward "docstring") (get-text-property (match-beginning 0) 'face))
     'non-string-face (save-excursion (goto-char (point-min)) (search-forward "name") (get-text-property (match-beginning 0) 'face))
     'defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face)))))"##,
    );
}

#[test]
fn ft_hyper_face_with_buffer_unibyte_multibyte_conversion_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Unibyte multibyte face test αβγ δεζ")
    (put-text-property 1 16 'face 'bold)
    (put-text-property 16 32 'face 'italic)
    (put-text-property 32 38 'face 'underline)
    (list
     'multibyte-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (multibyte-string-p (buffer-string)) (char-width (or (char-after pos) 0)))) '(1 10 16 20 32 36))
     'enable-multibyte (enable-multibyte-characters)
     'buffer-multibyte-p (multibyte-string-p (buffer-string))
     'face-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_hyper_face_font_lock_global_mode_toggle_effect_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'global-font-lock-mode-fbound (fboundp 'global-font-lock-mode)
   'font-lock-global-modes (if (boundp 'font-lock-global-modes)
                                font-lock-global-modes
                              'no-bound)
   'global-font-lock-enabled (condition-case nil
                                 (list (if (boundp 'global-font-lock-mode) global-font-lock-mode 'no-var)
                                       (if (fboundp 'font-lock-mode) 'fbound 'not-fbound)
                                       (if (fboundp 'global-font-lock-mode-enable-in-buffers)
                                           'has-enable-func
                                         'no-enable-func))
                               (error 'no-global-font-lock))))"##,
    );
}

#[test]
fn ft_hyper_face_changing_faces_on_different_frames_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((frame (selected-frame)))
    (condition-case nil (copy-face 'default 'my-multi-frame-face) (error nil))
    (list
     'set-on-frame (condition-case nil (progn (set-face-attribute 'my-multi-frame-face frame :weight 'bold) 'ok) (error 'no))
     'get-on-frame (face-attribute 'my-multi-frame-face :weight frame 'default-on)
     'set-fg-on-frame (condition-case nil (progn (set-face-foreground 'my-multi-frame-face "red" frame) 'ok) (error 'no))
     'get-fg-on-frame (condition-case nil (face-foreground 'my-multi-frame-face frame 'default-on) (error 'no))
     'set-font-on-frame (condition-case nil (progn (set-face-font 'my-multi-frame-face "Monospace-12" frame) 'ok) (error 'no))
     'get-font-on-frame (condition-case nil (face-font 'my-multi-frame-face frame) (error 'no))
     'reset (condition-case nil (progn (set-face-attribute 'my-multi-frame-face frame :weight 'unspecified :foreground 'unspecified) 'ok) (error 'no))))))"##,
    );
}

#[test]
fn ft_apex_face_text_property_position_zero_large_high_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert (make-string 500 ?A))
    (put-text-property 1 100 'face 'bold)
    (put-text-property 100 200 'face 'italic)
    (put-text-property 200 300 'face 'underline)
    (put-text-property 300 400 'face '(:foreground "red"))
    (put-text-property 400 501 'face '(:background "yellow"))
    (list
     'face-at-start (get-text-property 1 'face)
     'face-at-100 (get-text-property 100 'face)
     'face-at-200 (get-text-property 200 'face)
     'face-at-300 (get-text-property 300 'face)
     'face-at-400 (get-text-property 400 'face)
     'face-at-500 (get-text-property 500 'face)
     'next-prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face)) '(1 100 200 300 400))
     'interval-count (length (object-intervals (current-buffer)))
     'previous-prop-changes (mapcar (lambda (pos) (previous-single-property-change pos 'face nil (point-min))) '(100 200 300 400 500)))))"##,
    );
}

#[test]
fn ft_apex_face_property_boundary_at_line_breaks_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Line one with face property\n")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 28 'face 'italic)
    (insert "Line two with different face\n")
    (put-text-property 29 35 'face 'underline)
    (put-text-property 35 52 'face '(:foreground "red"))
    (insert "\nBlank line before this\n")
    (insert "Last line with face property\n")
    (put-text-property 54 64 'face '(:background "yellow"))
    (put-text-property 64 89 'face '(:foreground "blue" :weight bold))
    (list
     'faces-across-lines (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (line-number-at-pos))) '(1 5 15 28 29 40 52 53 54 64 80 88))
     'prop-cross-newline (mapcar (lambda (pos) (next-single-property-change pos 'face)) '(1 5 28 29 52 54 64))
     'line-end-faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(28 52 53 88)))))"##,
    );
}

#[test]
fn ft_apex_face_font_lock_mode_toggle_save_restore_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t)
          (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Face toggle save\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda () (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face)))))
        (let ((v0 (funcall snap)))
          ;; Toggle off
          (font-lock-mode -1)
          (let ((v1 (funcall snap)))
            ;; Toggle on
            (font-lock-mode 1)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Toggle off again
              (font-lock-mode -1)
              ;; Insert while off
              (goto-char (point-max))
              (insert "* DONE Inserted while off\nBody.\n\n")
              ;; Toggle on and check
              (font-lock-mode 1)
              (font-lock-ensure (point-min) (point-max))
              (let ((v3 (funcall snap))
                    (v3-new (save-excursion (goto-char (point-min)) (search-forward "DONE") (get-text-property (match-beginning 0) 'face))))
                (list v0 v1 v2 v3 v3-new)))))))))"##,
    );
}

#[test]
fn ft_apex_face_cache_clear_then_immediate_read_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-clear-cache-face) (error nil))
  (condition-case nil (set-face-attribute 'my-clear-cache-face nil :weight 'bold :slant 'italic :foreground "red") (error nil))
  (list
   'before-clear-weight (face-attribute 'my-clear-cache-face :weight nil 'default-on)
   'before-clear-slant (face-attribute 'my-clear-cache-face :slant nil 'default-on)
   'clear-cache (condition-case nil (clear-face-cache) (error 'no-clear))
   'after-clear-weight (face-attribute 'my-clear-cache-face :weight nil 'default-on)
   'after-clear-slant (face-attribute 'my-clear-cache-face :slant nil 'default-on)
   'after-clear-fg (condition-case nil (face-attribute 'my-clear-cache-face :foreground nil 'default-on) (error 'no))
   'after-clear-fg-frame (condition-case nil (face-attribute 'my-clear-cache-face :foreground (selected-frame) 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_apex_face_with_double_property_removal_same_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Double property removal same region test buffer content text")
    (add-text-properties 1 55 (list 'face 'bold 'prop-a 'val-a 'prop-b 'val-b 'prop-c 'val-c))
    ;; First removal: remove face and prop-a
    (remove-text-properties 10 40 '(face nil prop-a nil))
    ;; Second removal: remove prop-b from overlapping but different region
    (remove-text-properties 20 50 '(prop-b nil))
    (list
     'face-after (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25 30 35 40 45 50 54))
     'prop-a-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'prop-a)) '(1 5 10 30 45 54))
     'prop-b-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'prop-b)) '(1 5 15 20 30 45 54))
     'prop-c-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'prop-c)) '(1 10 20 30 40 50 54))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_apex_face_indirect_buffer_face_with_different_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Indirect buffer with different face props test content")
    (put-text-property 1 14 'face 'bold)
    (put-text-property 14 26 'face 'italic)
    (put-text-property 26 38 'face 'underline)
    (put-text-property 38 52 'face '(:foreground "red"))
    (let* ((clone-name (generate-new-buffer-name "*ft-apex-clone*"))
           (clone (make-indirect-buffer (current-buffer) clone-name t)))
      (unwind-protect
          (list
           'base-text-props (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 14 20 26 32 38 45 51))
           'clone-text-props (with-current-buffer clone (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 14 20 26 32 38 45 51)))
           ;; Modify in base
           (progn
             (put-text-property 1 14 'face '(:foreground "green" :weight bold))
             (remove-text-properties 26 38 '(face nil))
             (put-text-property 26 38 'face '(:background "yellow"))
             (list 'after-base-edit
                   (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 14 20 26 32 38 45 51))
                   'clone-after-base-edit (with-current-buffer clone (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 14 20 26 32 38 45 51)))))
           ;; Modify in clone
           (progn
             (with-current-buffer clone
               (put-text-property 14 26 'face '(:slant italic :background "cyan"))
               (put-text-property 38 52 'face '(:foreground "purple" :weight bold))
               (list 'after-clone-edit
                     (with-current-buffer clone (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 14 20 26 32 38 45 51)))
                     'base-after-clone-edit (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 14 20 26 32 38 45 51))))))
        (when (get-buffer clone-name) (kill-buffer clone-name))))))"##,
    );
}

#[test]
fn ft_apex_face_with_emoji_and_special_unicode_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "🌍🌎🌏 ♥ ♦ ♣ ♠ ★ ☆ ☀ ☁ ☂ ☃")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 6 'face 'italic)
    (put-text-property 6 8 'face 'underline)
    (put-text-property 8 20 'face '(:foreground "red"))
    (put-text-property 20 26 'face '(:background "yellow" :weight bold))
    (list
     'emoji-faces (mapcar (lambda (pos) (goto-char pos) (list pos (char-after pos) (get-text-property pos 'face) (char-width (char-after pos)))) '(1 2 3 4 5 6 8 10 12 14 18 22 25))
     'prop-changes (next-single-property-change 1 'face)
     'prev-prop-change (previous-single-property-change 26 'face nil 1)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_apex_face_with_font_lock_regexp_compilation_errors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'add-invalid-regexp
   (condition-case err
       (progn
         (font-lock-add-keywords nil '(("\\<\\(unmatched[group\\>" 1 font-lock-warning-face t)))
         'added-with-error)
     (error (list 'caught-error (car err))))
   'add-valid-after-invalid
   (condition-case nil
       (progn
         (font-lock-add-keywords nil '(("\\<\\(VALID\\)\\>" 1 font-lock-function-name-face t)))
         'valid-added-ok)
     (error 'valid-add-failed))
   'remove-keywords
   (condition-case nil
       (progn
         (font-lock-remove-keywords nil '(("\\<\\(unmatched[group\\>" 1 font-lock-warning-face t)))
         (font-lock-remove-keywords nil '(("\\<\\(VALID\\)\\>" 1 font-lock-function-name-face t)))
         'removed-ok)
     (error 'remove-failed)))))"##,
    );
}

#[test]
fn ft_omega_face_text_property_search_with_limit_bounds_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face 'bold)
    (put-text-property 21 25 'face 'italic)
    (put-text-property 25 29 'face 'underline)
    (put-text-property 29 33 'face '(:foreground "green"))
    (put-text-property 33 37 'face 'bold)
    (put-text-property 37 41 'face 'italic)
    (list
     'find-bold-in-range (text-property-any 5 25 'face 'bold)
     'find-bold-in-full (text-property-any 1 41 'face 'bold)
     'find-not-all-bold (text-property-not-all 1 41 'face 'bold)
     'find-not-all-italic-start (text-property-not-all 1 41 'face 'italic)
     'next-bold-after-10 (let ((pos (next-single-property-change 10 'face nil 41)))
                            (if pos (list pos (get-text-property pos 'face)) 'none))
     'prev-italic-before-30 (let ((pos (previous-single-property-change 30 'face)))
                               (if pos (list pos (get-text-property pos 'face)) 'none))
     'search-with-limit (text-property-any 10 20 'face 'underline)
     'search-no-match (text-property-any 1 41 'face 'nonexistent)))))"##,
    );
}

#[test]
fn ft_omega_face_with_overlay_hidden_via_overlay_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with overlay properties hiding faces test now")
    (put-text-property 1 50 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 15)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'before-string "[[")
      (overlay-put ov1 'after-string "]]")
      (overlay-put ov1 'display ""))
    (let ((ov2 (make-overlay 20 35)))
      (overlay-put ov2 'face '(:foreground "red"))
      (overlay-put ov2 'invisible t))
    (list
     'faces-overlay-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'invisible) (length (overlays-at pos)))) '(1 5 10 15 20 25 30 35 40 45 49))
     'overlay-get-invisible (overlay-get ov2 'invisible)
     'overlay-get-display (overlay-get ov1 'display)
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_omega_face_font_lock_fontify_block_function_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-fontify-block-fbound (fboundp 'font-lock-fontify-block)
   (condition-case nil
       (with-temp-buffer
         (emacs-lisp-mode)
         (insert "(defun my-block-test ()\n  (let ((x 1))\n    (+ x 2)))\n")
         (font-lock-fontify-buffer)
         (list
          'defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
          'let-face (save-excursion (goto-char (point-min)) (search-forward "let") (get-text-property (match-beginning 0) 'face))
          'x-face (save-excursion (goto-char (point-min)) (search-forward "x ") (get-text-property (match-beginning 0) 'face))))
     (error 'fontify-buffer-failed))
   (condition-case nil
       (with-temp-buffer
         (emacs-lisp-mode)
         (insert "(defun test2 () (+ 1 2))\n")
         (font-lock-fontify-block 1)
         (list
          'block-defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))))
     (error 'fontify-block-failed)))))"##,
    );
}

#[test]
fn ft_omega_face_with_copy_face_via_face_all_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (set-face-attribute 'bold nil :underline t :overline t :box t) (error nil))
  (condition-case nil (copy-face 'bold 'my-copy-from-bold) (error nil))
  (list
   'bold-underline-after-set (face-attribute 'bold :underline nil 'default-on)
   'copy-underline (condition-case nil (face-attribute 'my-copy-from-bold :underline nil 'default-on) (error 'no))
   'copy-overline (condition-case nil (face-attribute 'my-copy-from-bold :overline nil 'default-on) (error 'no))
   'copy-box (condition-case nil (face-attribute 'my-copy-from-bold :box nil 'default-on) (error 'no))
   'copy-weight (face-attribute 'my-copy-from-bold :weight nil 'default-on)
   ;; Reset bold
   (progn (set-face-attribute 'bold nil :underline 'unspecified :overline 'unspecified :box 'unspecified) 'reset-ok)
   'bold-after-reset (face-attribute 'bold :underline nil 'default-on))))"##,
    );
}

#[test]
fn ft_omega_face_property_find_with_predicate_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXXXYYYYYYYYYYYYZZZZZZZZZZZZ")
    (put-text-property 1 13 'face 'bold)
    (put-text-property 13 25 'face 'italic)
    (put-text-property 25 37 'face 'underline)
    (put-text-property 1 13 'value 10)
    (put-text-property 13 25 'value 20)
    (put-text-property 25 37 'value 30)
    (put-text-property 1 13 'extra 'data)
    (list
     'find-value-gt-15 (let ((found nil))
                          (while (and (not found) (< (point) (point-max)))
                            (let ((val (get-text-property (point) 'value)))
                              (when (and val (> val 15))
                                (setq found (list (point) val (get-text-property (point) 'face))))
                              (goto-char (next-single-property-change (point) 'value (point-max)))))
                          found)
     'find-face-bold (text-property-any 1 37 'face 'bold)
     'find-extra-data (text-property-any 1 37 'extra 'data)
     'find-non-existent (text-property-any 1 37 'nonexistent 'value)))))"##,
    );
}

#[test]
fn ft_omega_face_overlay_window_specific_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Window-specific overlay face test buffer content here now end")
    (put-text-property 1 56 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 15)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'window (selected-window)))
    (let ((ov2 (make-overlay 20 35)))
      (overlay-put ov2 'face '(:foreground "red"))
      (overlay-put ov2 'window nil))
    (let ((ov3 (make-overlay 40 56)))
      (overlay-put ov3 'face '(:weight bold))
      (overlay-put ov3 'window (selected-window)))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 40 45 50 55))
     'overlay-windows (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov) (overlay-get ov 'window) (overlay-get ov 'face)))
                              (list ov1 ov2 ov3))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_omega_face_color_distance_rgb_transformations_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'name-to-rgb-red (condition-case nil (color-name-to-rgb "red") (error 'no))
   'rgb-to-hex-red (condition-case nil (apply #'color-rgb-to-hex (append (color-name-to-rgb "red") '(2))) (error 'no))
   'name-to-rgb-green (condition-case nil (color-name-to-rgb "green") (error 'no))
   'name-to-rgb-blue (condition-case nil (color-name-to-rgb "blue") (error 'no))
   'rgb-to-hsl-red (condition-case nil (apply #'color-rgb-to-hsl (color-name-to-rgb "red")) (error 'no))
   'hsl-to-rgb (condition-case nil (let ((hsl (apply #'color-rgb-to-hsl (color-name-to-rgb "red"))))
                                      (apply #'color-hsl-to-rgb hsl))
                                   (error 'no))
   'color-gradient (condition-case nil (color-gradient '(1 0 0) '(0 0 1) 5) (error 'no))
   'color-dark-p-red (condition-case nil (color-dark-p "red") (error 'no))))))"##,
    );
}

#[test]
fn ft_omega_face_with_font_lock_fontify_after_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun test (x) (+ x 1))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (get-text-property (match-beginning 0) 'face))) '("defun" "test" "x" "1"))))
      ;; Simulate after-change by modifying and refontifying
      (goto-char (point-min))
      (search-forward "test")
      (replace-match "myTest")
      (goto-char (point-min))
      (search-forward "+")
      (replace-match "-")
      (font-lock-after-change-function (point-min) (point-max) 0)
      (let ((v1 (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (get-text-property (match-beginning 0) 'face))) '("defun" "myTest" "x" "1"))))
        (list v0 v1)))))"##,
    );
}

#[test]
fn ft_maxedge_face_interval_tree_rebuild_after_many_inserts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((results nil))
      (dotimes (i 20)
        (let ((start (1+ (* i 5)))
              (end (min (1+ (* (1+ i) 5)) (* 20 5 3))))
          (insert (make-string 5 (+ ?A i)))
          (put-text-property start end 'face
                             (nth (mod i 5)
                                   '(bold italic underline
                                     (:foreground "red")
                                     (:background "yellow"))))))
      (list
       'spot-checks (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 6 11 20 35 50 70 90))
       'interval-count (length (object-intervals (current-buffer)))
       'prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face)) '(1 6 11 20 35))
       'prev-changes (mapcar (lambda (pos) (previous-single-property-change pos 'face nil (point-min))) '(10 25 40 60 90))))))"##,
    );
}

#[test]
fn ft_maxedge_overlay_property_at_start_end_of_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay at buffer start and end test text here now")
    (let ((ov-start (make-overlay 1 5)))
      (overlay-put ov-start 'face '(:background "red"))
      (overlay-put ov-start 'before-string "<<<"))
    (let ((ov-end (make-overlay 48 52)))
      (overlay-put ov-end 'face '(:background "yellow"))
      (overlay-put ov-end 'after-string ">>>"))
    (let ((ov-full (make-overlay 1 52)))
      (overlay-put ov-full 'face '(:foreground "blue"))
      (overlay-put ov-full 'priority -1))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 3 5 10 25 40 48 51))
     'overlay-starts (mapcar #'overlay-start (overlays-in 1 52))
     'overlay-ends (mapcar #'overlay-end (overlays-in 1 52))
     'overlays-at-buffer-start (overlays-at 1)
     'overlays-at-buffer-end (overlays-at 51)
     (progn (mapc #'delete-overlay (overlays-in 1 52)) 'cleaned))))"##,
    );
}

#[test]
fn ft_maxedge_face_with_raise_attribute_combined_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-raise-face) (error nil))
  (list
   'set-raise-positive (condition-case nil (progn (set-face-attribute 'my-raise-face nil :raise 0.3) 'ok) (error 'no))
   'get-raise-positive (condition-case nil (face-attribute 'my-raise-face :raise nil 'default-on) (error 'no))
   'set-raise-negative (condition-case nil (progn (set-face-attribute 'my-raise-face nil :raise -0.2) 'ok) (error 'no))
   'get-raise-negative (condition-case nil (face-attribute 'my-raise-face :raise nil 'default-on) (error 'no))
   'set-raise-zero (condition-case nil (progn (set-face-attribute 'my-raise-face nil :raise 0) 'ok) (error 'no))
   'get-raise-zero (condition-case nil (face-attribute 'my-raise-face :raise nil 'default-on) (error 'no))
   'set-raise-with-other-attrs (condition-case nil (progn (set-face-attribute 'my-raise-face nil :raise 0.1 :weight 'bold :slant 'italic :underline t) 'ok) (error 'no))
   'get-all-after-set (list (face-attribute 'my-raise-face :raise nil 'default-on)
                            (face-attribute 'my-raise-face :weight nil 'default-on)
                            (face-attribute 'my-raise-face :slant nil 'default-on)
                            (condition-case nil (face-attribute 'my-raise-face :underline nil 'default-on) (error 'no))))))"##,
    );
}

#[test]
fn ft_maxedge_font_lock_fontify_block_with_nested_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun outer (a)\n")
    (insert "  (let ((inner (lambda (b) (* b b))))\n")
    (insert "    (funcall inner a)))\n")
    (font-lock-fontify-buffer)
    (mapcar
     (lambda (needle)
       (save-excursion
         (goto-char (point-min))
         (if (search-forward needle nil t)
             (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))
             (list needle 'not-found nil))))
     '("defun" "outer" "let" "lambda" "funcall" "inner"))))"##,
    );
}

#[test]
fn ft_maxedge_face_text_property_add_to_empty_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (list
     'empty-before-face (get-text-property 1 'face)
     'empty-before-fontified (get-text-property 1 'fontified)
     'empty-interval-count (length (object-intervals (current-buffer)))
     ;; Add content and face
     (progn
       (insert "New content with face")
       (put-text-property 1 20 'face 'bold)
       (list
        'after-face (get-text-property 1 'face)
        'after-fontified (get-text-property 1 'fontified)
        'after-interval-count (length (object-intervals (current-buffer)))))
     ;; Erase and re-check
     (progn
       (erase-buffer)
       (list
        'after-erase-face (get-text-property 1 'face)
        'after-erase-interval-count (length (object-intervals (current-buffer)))))
     ;; Re-insert and check
     (progn
       (insert "Re-inserted with different face properties now")
       (put-text-property 1 13 'face 'italic)
       (put-text-property 13 30 'face 'underline)
       (put-text-property 30 49 'face '(:foreground "red"))
       (list
        'reinsert-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 13 20 30 40 48))
        'reinsert-interval-count (length (object-intervals (current-buffer)))))))))"##,
    );
}

#[test]
fn ft_maxedge_face_overlay_with_multiple_faces_appended_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with multiple appended face properties test content")
    (let ((ov (make-overlay 1 57)))
      (overlay-put ov 'face '(:foreground "blue"))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold :slant 'italic))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold :slant 'italic :background "yellow"))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold :slant 'italic :background "yellow" :underline t))
      (list
       'face-at-point (get-char-property 1 'face)
       'face-at-10 (get-char-property 10 'face)
       'face-at-30 (get-char-property 30 'face)
       'face-at-50 (get-char-property 50 'face)
       'facep-face (facep (get-char-property 1 'face))
       'overlay-props-count (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_maxedge_face_buffer_local_face_remap_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "Buffer local face remap complex test content text")
    (put-text-property 1 50 'face 'bold)
    (list
     'initial-face (get-text-property 1 'face)
     'initial-remap-alist (face-remapping-alist)
     'add-relative (condition-case nil (progn (face-remap-add-relative 'bold '(:foreground "red")) 'ok) (error 'no))
     'face-after-remap (get-text-property 1 'face)
     'remap-alist-after (face-remapping-alist)
     'add-another (condition-case nil (progn (face-remap-add-relative 'bold '(:slant italic)) 'ok) (error 'no))
     'face-after-another (get-text-property 1 'face)
     'remap-alist-after-another (face-remapping-alist)
     'reset-all (condition-case nil (progn (face-remap-reset-base 'bold) 'ok) (error 'no))
     'face-after-reset (get-text-property 1 'face)
     'remap-alist-after-reset (face-remapping-alist)))))"##,
    );
}

#[test]
fn ft_maxedge_face_compare_via_attribute_extraction_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'bold-vs-italic (list 'weight (face-attribute 'bold :weight nil 'default-on) (face-attribute 'italic :weight nil 'default-on)
                         'slant (face-attribute 'bold :slant nil 'default-on) (face-attribute 'italic :slant nil 'default-on))
   'default-vs-bold (list 'weight-d (face-attribute 'default :weight nil 'default-on) 'weight-b (face-attribute 'bold :weight nil 'default-on)
                          'face-equal (condition-case nil (face-equal 'default 'bold) (error 'no)))
   'italic-vs-underline (list 'slant-i (face-attribute 'italic :slant nil 'default-on)
                               'underline-u (condition-case nil (face-attribute 'underline :underline nil 'default-on) (error 'no)))
   'all-3-compare (list (face-equal 'default 'default)
                        (condition-case nil (face-equal 'bold 'bold) (error 'no))
                        (face-differs-from-default-p 'bold)
                        (face-differs-from-default-p 'italic)
                        (condition-case nil (face-differs-from-default-p 'underline) (error 'no)))))"##,
    );
}

#[test]
fn ft_zenith_face_with_invalid_face_name_error_handling_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'facep-nil (facep nil)
   'facep-number (facep 42)
   'facep-string (condition-case nil (facep "not-a-face-symbol") (error 'error))
   'facep-list (facep '(bold italic))
   'facep-consp (facep (cons 'bold '(italic)))
   'face-attr-nil-face (condition-case err (face-attribute nil :weight) (error (list (car err) (cdr err))))
   'set-face-attr-nil-face (condition-case err (set-face-attribute nil nil :weight 'bold) (error (car err)))
   'face-font-nil-face (condition-case err (face-font nil nil) (error (car err)))
   'face-foreground-nil-face (condition-case err (face-foreground nil nil) (error (car err)))
   'make-face-valid (condition-case err (make-face 'really-new-face) (error (car err)))
   'facep-after-make (facep 'really-new-face))))"##,
    );
}

#[test]
fn ft_zenith_face_overlay_before_after_display_string_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay before after display string face test area now final end")
    (let ((ov (make-overlay 15 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "[[START]]" 'face '(:foreground "red" :weight bold)))
      (overlay-put ov 'after-string (propertize "{{END}}" 'face '(:foreground "blue" :slant italic)))
      (overlay-put ov 'display ""))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 15 20 30 40 50 57))
     'overlay-before-face (get-text-property 1 (overlay-get ov 'before-string))
     'overlay-after-face (get-text-property 1 (overlay-get ov 'after-string))
     'overlay-before-props (text-properties-at 0 (overlay-get ov 'before-string))
     'overlay-after-props (text-properties-at 0 (overlay-get ov 'after-string))
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_zenith_face_font_lock_set_defaults_then_refontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (insert "Custom font-lock defaults test buffer content")
    (list
     'before-fontify (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30))
     'set-defaults (condition-case nil
                      (progn
                        (font-lock-set-defaults)
                        (font-lock-fontify-buffer)
                        'ok)
                    (error 'no))
     'after-fontify (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30))
     'unfontify (progn
                  (font-lock-unfontify-buffer)
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 10 20 30)))
     'refontify (progn
                  (font-lock-fontify-buffer)
                  (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 10 20 30))))))"##,
    );
}

#[test]
fn ft_zenith_face_text_property_change_hook_on_face_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (defvar my-hook-count 0)
  (defun my-face-change-hook (beg end old-len)
    (setq my-hook-count (1+ my-hook-count)))
  (with-temp-buffer
    (insert "Face change hook test buffer content text here")
    (put-text-property 1 46 'face 'bold)
    (list
     'initial-hook-count my-hook-count
     'initial-face (get-text-property 1 'face)
     ;; Modify face property
     (progn
       (put-text-property 10 25 'face 'italic)
       my-hook-count)
     ;; Modify non-face property
     (progn
       (put-text-property 25 40 'my-prop 'value)
       my-hook-count)
     ;; Remove face
     (progn
       (remove-text-properties 10 25 '(face nil))
       my-hook-count)
     'final-hook-count my-hook-count
     'final-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25 30 40 45))))))"##,
    );
}

#[test]
fn ft_zenith_face_overlay_rear_advance_front_advance_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov1 (make-overlay 6 16 nil t nil)))
      (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 16 26 nil nil t)))
      (overlay-put ov2 'face '(:background "green")))
    (let ((ov3 (make-overlay 26 36 t t nil)))
      (overlay-put ov3 'face '(:background "yellow")))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 6 10 15 16 20 25 26 30 35 36)))))
      (let ((v0 (funcall snap)))
        ;; Insert at front-advance boundary
        (goto-char 16) (insert "XX")
        (let ((v1 (funcall snap)))
          ;; Insert at rear-advance boundary
          (goto-char 26) (insert "YY")
          (let ((v2 (funcall snap)))
            ;; Insert at both-advance boundary
            (goto-char 36) (insert "ZZ")
            (let ((v3 (funcall snap)))
              (mapc #'delete-overlay (overlays-in 1 (point-max)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_zenith_face_font_lock_keywords_case_fold_search_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "CASE test Case Test case TEST CaSe TeSt")
    (let ((font-lock-keywords-case-fold-search t))
      (font-lock-add-keywords nil '(("\\<\\(CASE\\)\\>" 1 font-lock-warning-face t)))
      (font-lock-fontify-buffer)
      (list
       'case-fold-t (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (get-text-property (match-beginning 0) 'face)))
                            '("CASE" "Case" "case" "CaSe" "TEST" "TeSt" "test")))
       'bounds-case-fold (if (boundp 'font-lock-keywords-case-fold-search)
                              font-lock-keywords-case-fold-search
                            'no-bound)
       'found-all (save-excursion
                    (goto-char (point-min))
                    (let ((count 0))
                      (while (re-search-forward "\\<[Cc][Aa][Ss][Ee]\\>" nil t)
                        (setq count (1+ count)))
                      count))))))"##,
    );
}

#[test]
fn ft_zenith_face_with_propertize_create_then_modify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((s (propertize "propertized-string"
                       'face 'bold
                       'my-key 'my-value
                       'fontified t)))
    (list
     'propertized-face (get-text-property 0 'face s)
     'propertized-key (get-text-property 0 'my-key s)
     'propertized-fontified (get-text-property 0 'fontified s)
     'string-length (length s)
     ;; Modify via set-text-properties
     (progn
       (set-text-properties 0 (length s) (list 'face 'italic 'new-key 'new-val) s)
       (list 'after-set-face (get-text-property 0 'face s)
             'after-set-old-key (get-text-property 0 'my-key s)
             'after-set-new-key (get-text-property 0 'new-key s)))
     ;; Add face via add-face-text-property
     (progn
       (add-face-text-property 0 (length s) '(:underline t) nil s)
       (list 'after-add-face (get-text-property 0 'face s)
             'facep-result (facep (get-text-property 0 'face s)))))))"##,
    );
}

#[test]
fn ft_zenith_face_with_last_nonzero_length_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (put-text-property 1 2 'face 'bold)
    (list
     'single-char-face (get-text-property 1 'face)
     'single-char-fontified (get-text-property 1 'fontified)
     'single-char-props (text-properties-at 1)
     'single-char-interval-count (length (object-intervals (current-buffer)))
     ;; Extend
     (progn
       (goto-char 2)
       (insert "EXTENDED")
       (list
        'after-extend-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 5 8 10))
        'after-extend-interval-count (length (object-intervals (current-buffer))))))))"##,
    );
}

#[test]
fn ft_cosmic_overlay_variable_width_insert_delete_boundary_test() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHHIIIIIJJJJJ")
    (put-text-property 1 51 'face '(:foreground "gray"))
    (let ((ov1 (make-overlay 6 15))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 16 25))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 26 35))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 30))
    (let ((ov4 (make-overlay 36 45))) (overlay-put ov4 'face '(:foreground "orange")) (overlay-put ov4 'priority 15))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 16 20 25 26 30 35 36 40 45 50)))))
      (let ((v0 (funcall snap)))
        ;; Insert 3 chars at each overlay boundary
        (goto-char 15) (insert "XXX")
        (goto-char 25) (insert "YYY")
        (goto-char 35) (insert "ZZZ")
        (let ((v1 (funcall snap)))
          ;; Delete 3 chars through overlay boundaries
          (delete-region 10 20)
          (let ((v2 (funcall snap)))
            (delete-region 30 40)
            (let ((v3 (funcall snap)))
              (mapc #'delete-overlay (overlays-in 1 (point-max)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_cosmic_face_text_property_sticky_at_insertion_point_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Sticky insertion point face test buffer content")
    (put-text-property 1 8 'face '(:foreground "red") ,rear-nonsticky nil)
    (put-text-property 8 16 'face '(:foreground "green") ,front-sticky t)
    (put-text-property 16 24 'face '(:foreground "blue") ,front-sticky t ,rear-nonsticky nil)
    (put-text-property 24 32 'face 'bold ,rear-nonsticky '(face))
    (put-text-property 32 43 'face 'italic ,front-sticky '(face))
    (list
     'initial-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'front-sticky) (get-text-property pos 'rear-nonsticky))) '(1 5 8 12 16 20 24 28 32 38 42))
     'insert-at-nonsticky (progn (goto-char 8) (insert "AAA") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 8 11 14 16)))
     'insert-at-frontsticky (progn (goto-char 32) (insert "BBB") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(26 30 35 38 42 45))))))"##,
    );
}

#[test]
fn ft_cosmic_face_font_lock_mode_on_off_multiple_cycles() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t) (org-fontify-todo-headline t))
      (org-mode)
      (insert "* TODO Cycle face test\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((snap (lambda () (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))))))
        (let ((v0 (funcall snap)))
          (font-lock-mode -1) (let ((v1 (funcall snap)))
          (font-lock-mode 1) (font-lock-ensure (point-min) (point-max)) (let ((v2 (funcall snap)))
          (font-lock-mode -1) (let ((v3 (funcall snap)))
          (font-lock-mode 1) (font-lock-ensure (point-min) (point-max)) (let ((v4 (funcall snap)))
          (font-lock-mode -1) (let ((v5 (funcall snap)))
          (font-lock-mode 1) (font-lock-ensure (point-min) (point-max)) (let ((v6 (funcall snap)))
          (list v0 v1 v2 v3 v4 v5 v6))))))))))))))"##,
    );
}

#[test]
fn ft_cosmic_face_overlay_insert_behind_front_hooks_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay insert behind front hooks priority test text data here")
    (defvar my-callback-ran nil)
    (let ((ov1 (make-overlay 5 15)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'insert-in-front-hooks (list (lambda (ov after beg end &optional len) (setq my-callback-ran (cons 'front my-callback-ran))))))
    (let ((ov2 (make-overlay 20 35)))
      (overlay-put ov2 'face '(:background "cyan"))
      (overlay-put ov2 'insert-behind-hooks (list (lambda (ov after beg end &optional len) (setq my-callback-ran (cons 'behind my-callback-ran))))))
    (list
     'before-insert my-callback-ran
     'faces-before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 40 50))
     ;; Insert at front-hook overlay boundary
     (progn (goto-char 15) (insert "F") my-callback-ran)
     ;; Insert at behind-hook overlay boundary
     (progn (goto-char 35) (insert "B") my-callback-ran)
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_cosmic_face_text_property_search_any_with_nil_value_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property search with nil values test content buffer text")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 10 20 'face nil)
    (put-text-property 20 30 'face '(:foreground "red"))
    (put-text-property 30 40 'face nil)
    (put-text-property 40 50 'face 'underline)
    (list
     'find-bold (text-property-any 1 50 'face 'bold)
     'find-nil-face (text-property-any 1 50 'face nil)
     'find-non-nil (text-property-any 1 50 'face 'underline)
     'find-with-nil-prop (let ((pos 1) (result nil))
                           (while (and pos (< pos 50))
                             (let ((face (get-text-property pos 'face)))
                               (when (null face) (push pos result)))
                             (setq pos (next-single-property-change pos 'face nil 50)))
                           (nreverse result))
     'prop-not-all (text-property-not-all 1 50 'face nil)))))"##,
    );
}

#[test]
fn ft_cosmic_face_display_table_glyph_face_interaction_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Display table glyph face interaction test buffer")
    (put-text-property 1 48 'face '(:foreground "blue"))
    (let ((dt (make-display-table)))
      (condition-case nil
          (progn
            (set-display-table-slot dt 'selective-display
                                    (vector (make-glyph-code ?- 'highlight)))
            (list
             'display-table-created 'ok
             'face-still-present (get-text-property 1 'face)
             'display-table-slots (mapcar (lambda (slot) (display-table-slot dt slot)) '(0 1))
             'describe-display-fbound (fboundp 'describe-display-table)))
        (error (list 'display-table-error
                     (fboundp 'make-display-table)
                     (fboundp 'set-display-table-slot)
                     (fboundp 'display-table-slot)
                     (facep 'highlight)
                     (get-text-property 1 'face)))))))"##,
    );
}

#[test]
fn ft_cosmic_face_font_lock_global_fontify_mode_toggle_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (insert "Global fontify mode toggle buffer test text here now end")
    (list
     'font-lock-mode-before font-lock-mode
     'turn-on (condition-case nil (progn (font-lock-mode 1) 'ok) (error 'no))
     'font-lock-mode-after-on font-lock-mode
     'fontify-buffer (condition-case nil (progn (font-lock-fontify-buffer) 'ok) (error 'no))
     'face-after-fontify (get-text-property 1 'face)
     'fontified-after (get-text-property 1 'fontified)
     'turn-off (condition-case nil (progn (font-lock-mode -1) 'ok) (error 'no))
     'font-lock-mode-after-off font-lock-mode
     'face-after-off (get-text-property 1 'face)
     'fontified-after-off (get-text-property 1 'fontified)
     'turn-on-again (condition-case nil (progn (font-lock-mode 1) (font-lock-fontify-buffer) 'ok) (error 'no))
     'face-after-again (get-text-property 1 'face))))"##,
    );
}

#[test]
fn ft_cosmic_face_overlay_make_and_move_and_delete_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHHIIIIIJJJJJ")
    (put-text-property 1 51 'face '(:foreground "blue"))
    (let ((ov (make-overlay 10 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 15 20 25 30 40 50)))))
        (let ((v0 (funcall snap)))
          ;; Move overlay forward
          (move-overlay ov 20 30)
          (let ((v1 (funcall snap)))
            ;; Move overlay backward
            (move-overlay ov 1 12)
            (let ((v2 (funcall snap)))
              ;; Move to empty
              (move-overlay ov 50 50)
              (let ((v3 (funcall snap)))
                ;; Move back
                (move-overlay ov 30 42)
                (let ((v4 (funcall snap)))
                  (delete-overlay ov)
                  (list v0 v1 v2 v3 v4))))))))))"##,
    );
}

#[test]
fn ft_nova_face_multi_overlay_same_buffer_region_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multi overlay same region face priority stacking test text")
    (put-text-property 1 55 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 10 30))) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'face '(:foreground "green" :weight bold)))
    (let ((ov3 (make-overlay 10 30))) (overlay-put ov3 'face '(:underline t)))
    (let ((ov4 (make-overlay 10 30))) (overlay-put ov4 'face '(:slant italic)))
    (list
     'same-region-overlay-count (length (overlays-at 20))
     'face-at-20 (get-char-property 20 'face)
     'all-overlay-faces (mapcar (lambda (ov) (overlay-get ov 'face)) (overlays-at 20))
     'text-prop-face (get-text-property 20 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))"##,
    );
}

#[test]
fn ft_nova_face_add_text_properties_multiple_regions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "First region with bold face and properties here")
    (insert "\nSecond region with italic face here region end\n")
    (insert "Third region with underline face goes here end\n")
    (put-text-property 1 31 'face 'bold)
    (put-text-property 1 31 'region 'first)
    (put-text-property 32 60 'face 'italic)
    (put-text-property 32 60 'region 'second)
    (put-text-property 61 93 'face 'underline)
    (put-text-property 61 93 'region 'third)
    (list
     'faces-by-region (mapcar (lambda (pos) (goto-char pos) (list pos (line-number-at-pos) (get-text-property pos 'face) (get-text-property pos 'region))) '(1 15 30 32 45 59 61 75 92))
     'prop-changes-across-newlines (mapcar (lambda (pos) (next-single-property-change pos 'face)) '(1 32 61))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_nova_face_font_lock_ensure_versus_fontify_buffer_diff() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun compare (a b) (+ a b))\n")
    (insert "(setq x 42)\n")
    ;; Don't fontify first, check state
    (list
     'before-any-fontify (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30))
     ;; Fontify with font-lock-ensure
     'after-ensure (progn
                     (font-lock-ensure (point-min) (point-max))
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30)))
     ;; Unfontify and use font-lock-fontify-buffer
     'after-fontify-buffer (progn
                             (font-lock-unfontify-buffer)
                             (font-lock-fontify-buffer)
                             (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 30)))
     'font-lock-fontify-region-function (if (boundp 'font-lock-fontify-region-function) 'bound 'not-bound)))))"##,
    );
}

#[test]
fn ft_nova_face_with_overlay_property_at_overlay_boundaries_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Boundary overlay properties face test region content text here end")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'help-echo "This is overlay text")
      (overlay-put ov 'keymap (make-sparse-keymap))
      (overlay-put ov 'local-map (make-sparse-keymap))
      (list
       'at-start (list (get-char-property 9 'face) (get-char-property 10 'face))
       'at-end (list (get-char-property 39 'face) (get-char-property 40 'face))
       'inside (get-char-property 25 'face)
       'help-echo (overlay-get ov 'help-echo)
       'has-keymap (keymapp (overlay-get ov 'keymap))
       'has-local-map (keymapp (overlay-get ov 'local-map))
       'overlay-props-count (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_nova_face_text_properties_at_various_buffer_positions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Position-based text property check for face accuracy")
    (put-text-property 1 53 'face '(:foreground "blue"))
    (put-text-property 1 10 'extra-property 'first-ten)
    (put-text-property 40 53 'extra-property 'last-thirteen)
    (list
     'pos-1 (text-properties-at 1)
     'pos-5 (text-properties-at 5)
     'pos-10 (text-properties-at 10)
     'pos-11 (text-properties-at 11)
     'pos-25 (text-properties-at 25)
     'pos-40 (text-properties-at 40)
     'pos-52 (text-properties-at 52)
     'prop-any-first-ten (text-property-any 1 53 'extra-property 'first-ten)
     'prop-any-last (text-property-any 1 53 'extra-property 'last-thirteen)
     'prop-any-none (text-property-any 1 53 'extra-property 'none))))"##,
    );
}

#[test]
fn ft_nova_face_org_font_lock_defface_present_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-faces)
  (list
   'org-level-faces (mapcar (lambda (n) (let ((face (intern (format "org-level-%d" n)))) (list n face (facep face)))) '(1 2 3 4 5 6 7 8))
   'org-todo-face (facep 'org-todo)
   'org-done-face (facep 'org-done)
   'org-priority-face (facep 'org-priority)
   'org-tag-face (facep 'org-tag)
   'org-date-face (facep 'org-date)
   'org-link-face (facep 'org-link)
   'org-block-face (facep 'org-block)
   'org-table-face (facep 'org-table)
   'org-drawer-face (facep 'org-drawer)
   'org-special-keyword-face (facep 'org-special-keyword)
   'org-document-title-face (facep 'org-document-title)
   'org-meta-line-face (facep 'org-meta-line)
   'org-checkbox-face (facep 'org-checkbox)
   'org-verbatim-face (facep 'org-verbatim)
   'org-code-face (facep 'org-code)
   'org-formula-face (facep 'org-formula))))"##,
    );
}

#[test]
fn ft_nova_face_with_property_interval_deletion_at_boundaries_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXXXYYYYYZZZZZZZZZZWWWWWWWWW")
    (put-text-property 1 13 'face 'bold)
    (put-text-property 13 18 'face 'italic)
    (put-text-property 18 28 'face 'underline)
    (put-text-property 28 37 'face '(:foreground "red"))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 13 15 18 22 28 32 36)))))
      (let ((v0 (funcall snap)))
        ;; Delete exactly at boundary (13-18 removes italic exactly)
        (delete-region 13 18)
        (let ((v1 (funcall snap)))
          ;; Delete across boundary (part of bold, part of underline)
          (delete-region 7 22)
          (let ((v2 (funcall snap)))
            ;; Delete entire remaining region
            (delete-region 1 (point-max))
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_nova_face_text_property_character_width_multi_column_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AいろはXYZ한글テストEND")
    (put-text-property 1 2 'face 'bold)
    (put-text-property 2 4 'face 'italic)
    (put-text-property 4 6 'face 'underline)
    (put-text-property 6 15 'face '(:foreground "red"))
    (list
     'faces-and-widths (mapcar (lambda (pos) (goto-char pos) (list pos (char-after pos) (get-text-property pos 'face) (char-width (char-after pos)) (string-width (string (char-after pos))))) '(1 2 3 4 5 7 9 12 15))
     'string-width-of-region (string-width (buffer-substring 1 10))
     'byte-length (length (encode-coding-string (buffer-string) 'utf-8))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_void_face_overlay_before_string_with_multiple_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before string with multiple face properties test data now end")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string
                   (concat (propertize "[[BEFORE-FG]]" 'face '(:foreground "red" :weight bold))
                           (propertize "[[BEFORE-BG]]" 'face '(:background "cyan" :slant italic))))
      (overlay-put ov 'after-string
                   (propertize "{{AFTER-FG-BG}}" 'face '(:foreground "blue" :background "white"))))
    (list
     'faces-around (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 15 25 35 45 54))
     'before-string-length (length (overlay-get ov 'before-string))
     'after-string-length (length (overlay-get ov 'after-string))
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_void_face_font_lock_comment_delimiter_face_in_modes_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'emacs-lisp-comment
   (with-temp-buffer
     (emacs-lisp-mode)
     (insert ";; This is a comment\n(defun f (x) x)\n")
     (font-lock-fontify-buffer)
     (list (save-excursion (goto-char (point-min)) (search-forward "comment") (get-text-property (match-beginning 0) 'face))
           (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))))
   'c-mode-comment
   (with-temp-buffer
     (condition-case nil
         (progn
           (c-mode)
           (insert "/* comment */ int x; // comment\n")
           (font-lock-fontify-buffer)
           (list (save-excursion (goto-char (point-min)) (search-forward "comment") (get-text-property (match-beginning 0) 'face))
                 (save-excursion (goto-char (point-min)) (search-forward "int") (get-text-property (match-beginning 0) 'face))))
       (error 'c-mode-fontify-failed))))))"##,
    );
}

#[test]
fn ft_void_face_text_property_next_change_with_object_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:foreground "blue"))
    (put-text-property 31 36 'face '(:background "cyan"))
    (list
     'next-prop-change-with-limit (next-single-property-change 1 'face nil 20)
     'next-prop-change-to-end (next-single-property-change 1 'face nil 36)
     'prev-prop-change-with-limit (previous-single-property-change 36 'face nil (point-min))
     'prop-any-first (text-property-any 1 20 'face 'bold)
     'prop-any-middle (text-property-any 10 25 'face 'underline)
     'prop-not-all-first-half (text-property-not-all 1 20 'face 'bold)
     'prop-not-all-full (text-property-not-all 1 36 'face 'bold)
     'object-intervals (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_void_face_overlay_line_prefix_wrap_prefix_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with line and wrap prefix and face combined")
    (let ((ov (make-overlay 1 52)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'line-prefix (propertize "| " 'face '(:foreground "red")))
      (overlay-put ov 'wrap-prefix (propertize "> " 'face '(:foreground "blue"))))
    (list
     'overlay-face (get-char-property 1 'face)
     'text-prop-face (get-text-property 1 'face)
     'line-prefix (overlay-get ov 'line-prefix)
     'wrap-prefix (overlay-get ov 'wrap-prefix)
     'line-prefix-face (get-text-property 0 (overlay-get ov 'line-prefix))
     'wrap-prefix-face (get-text-property 0 (overlay-get ov 'wrap-prefix))
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_void_face_property_accumulation_via_multiple_add_calls_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property accumulation via multiple add calls test")
    (add-text-properties 1 48 '(face (:foreground "blue") key1 val1))
    (add-text-properties 1 25 '(face (:underline t) key2 val2))
    (add-face-text-property 20 48 '(:weight bold))
    (add-text-properties 10 35 '(face (:slant italic) key3 val3))
    (list
     'face-at-start (list (get-text-property 1 'face) (get-text-property 1 'key1) (get-text-property 1 'key2))
     'face-at-middle (list (get-text-property 20 'face) (get-text-property 20 'key1) (get-text-property 20 'key2) (get-text-property 20 'key3))
     'face-at-end (list (get-text-property 47 'face) (get-text-property 47 'key1))
     'text-props-at-1 (length (text-properties-at 1))
     'text-props-at-20 (length (text-properties-at 20))
     'text-props-at-47 (length (text-properties-at 47)))))"##,
    );
}

#[test]
fn ft_void_face_font_lock_regexp_subgroup_highlight_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "email: user@example.com url: https://example.org path: /usr/local/bin")
    (font-lock-add-keywords nil
                            '(("\\(email\\): \\([a-z@.]+\\)" (1 font-lock-keyword-face) (2 font-lock-warning-face))
                              ("\\(url\\): \\(https?://[^ ]+\\)" (1 font-lock-keyword-face) (2 font-lock-function-name-face))
                              ("\\(path\\): \\(/[^ ]+\\)" (1 font-lock-keyword-face) (2 font-lock-string-face))))
    (font-lock-fontify-buffer)
    (mapcar
     (lambda (needle)
       (save-excursion
         (goto-char (point-min))
         (if (search-forward needle nil t)
             (list needle (get-text-property (match-beginning 0) 'face))
             (list needle 'not-found))))
     '("email" "user@example.com" "url" "https://example.org" "path" "/usr/local/bin"))))"##,
    );
}

#[test]
fn ft_void_face_overlay_category_and_face_inheritance_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay category and face inheritance test buffer content area")
    (put-text-property 1 56 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 1 20)))
      (overlay-put ov1 'category 'my-category-1)
      (overlay-put ov1 'face '(:background "yellow")))
    (let ((ov2 (make-overlay 25 45)))
      (overlay-put ov2 'category 'my-category-2)
      (overlay-put ov2 'face '(:foreground "red" :weight bold)))
    (list
     'category-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 10 15 20 25 30 40 45 50 55))
     'overlay-categories (mapcar (lambda (ov) (list (overlay-get ov 'category) (overlay-get ov 'face) (overlay-start ov) (overlay-end ov))) (list ov1 ov2))
     'overlay-props-count (mapcar (lambda (ov) (length (overlay-properties ov))) (list ov1 ov2))
     (progn (mapc #'delete-overlay (overlays-in 1 56)) 'cleaned))))"##,
    );
}

#[test]
fn ft_void_face_with_zero_length_string_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((s1 "")
        (s2 (propertize "" 'face 'bold))
        (s3 (propertize "" 'face '(:foreground "red")))
        (s4 "X"))
    (list
     'empty-string-face (get-text-property 0 'face s1)
     'empty-string-bold-face (get-text-property 0 'face s2)
     'empty-string-red-face (get-text-property 0 'face s3)
     'normal-char-face (get-text-property 0 'face s4)
     'empty-string-length (length s1)
     'bold-empty-length (length s2)
     'empty-string-props (text-properties-at 0 s2)
     'empty-string-no-props (text-properties-at 0 s1)))))"##,
    );
}

#[test]
fn ft_abyss_face_overlay_evaporate_with_insert_before_after_text_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Evaporating overlay with before after text test here now")
    (let ((ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'evaporate t)
      (overlay-put ov 'before-string "{{")
      (overlay-put ov 'after-string "}}")
      (list
       'before-delete (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 15 20 25 30 35 45))
       ;; Delete region containing evaporating overlay
       'after-delete (progn
                       (delete-region 15 30)
                       (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 15 20 25 30 35)))
       'overlay-deleted (not (overlay-buffer ov)))))))"##,
    );
}

#[test]
fn ft_abyss_face_attribute_all_atts_extraction_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-all-atts (condition-case nil
                         (face-all-attributes 'default (selected-frame))
                       (error 'no-all-atts))
   'bold-all-atts (condition-case nil
                      (face-all-attributes 'bold (selected-frame))
                    (error 'no-bold-atts))
   'all-atts-length-default (condition-case nil
                                (length (face-all-attributes 'default (selected-frame)))
                              (error 'no-length))
   'default-foreground (condition-case nil (face-foreground 'default (selected-frame) 'default-on) (error 'no))
   'default-background (condition-case nil (face-background 'default (selected-frame) 'default-on) (error 'no))
   'default-font (condition-case nil (face-font 'default (selected-frame)) (error 'no))
   'default-size (condition-case nil (face-attribute 'default :height (selected-frame) 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_abyss_overlays_in_order_by_priority_and_position_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlays sorted by priority and position in buffer text content here")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'priority 50) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 5 25))) (overlay-put ov2 'priority 50) (overlay-put ov2 'face '(:background "green")))
    (let ((ov3 (make-overlay 10 30))) (overlay-put ov3 'priority 75) (overlay-put ov3 'face '(:background "blue")))
    (let ((ov4 (make-overlay 20 40))) (overlay-put ov4 'priority 25) (overlay-put ov4 'face '(:background "yellow")))
    (let ((ov5 (make-overlay 30 50))) (overlay-put ov5 'priority 100) (overlay-put ov5 'face '(:background "orange")))
    (list
     'overlays-sorted (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov) (overlay-get ov 'priority) (overlay-get ov 'face)))
                             (sort (overlays-in 1 55)
                                   (lambda (a b)
                                     (let ((pa (or (overlay-get a 'priority) 0))
                                           (pb (or (overlay-get b 'priority) 0)))
                                       (or (> pa pb)
                                           (and (= pa pb) (< (overlay-start a) (overlay-start b))))))))
     'faces-at-10 (get-char-property 10 'face)
     'faces-at-25 (get-char-property 25 'face)
     'faces-at-35 (get-char-property 35 'face)
     'max-priority-at-10 (apply #'max (mapcar (lambda (ov) (or (overlay-get ov 'priority) 0)) (overlays-at 10)))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))"##,
    );
}

#[test]
fn ft_abyss_face_font_lock_unfontify_region_then_refontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Region unfontify\nBody region.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))))
        ;; Unfontify specific region
        (font-lock-unfontify-region 1 20)
        (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))))
          ;; Unfontify whole buffer
          (font-lock-unfontify-buffer)
          (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))))
            ;; Re-fontify
            (font-lock-fontify-buffer)
            (let ((v3 (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))))
              (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_abyss_face_buffer_string_property_survival_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Buffer string property survival test text content area")
    (put-text-property 1 49 'face '(:foreground "blue"))
    (put-text-property 1 20 'extra-mark 'first-half)
    (put-text-property 20 49 'extra-mark 'second-half)
    (list
     'local-face (get-text-property 1 'face)
     'local-extra (get-text-property 1 'extra-mark)
     'via-buffer-string
     (let ((s (buffer-string)))
       (with-temp-buffer
         (insert s)
         (list
          'copy-face (get-text-property 1 'face)
          'copy-extra (get-text-property 1 'extra-mark)
          'copy-face-25 (get-text-property 25 'face)
          'copy-extra-25 (get-text-property 25 'extra-mark))))
     'via-buffer-substring
     (let ((s (buffer-substring 1 49)))
       (with-temp-buffer
         (insert s)
         (list
          'sub-face (get-text-property 1 'face)
          'sub-extra (get-text-property 1 'extra-mark)))))))"##,
    );
}

#[test]
fn ft_abyss_face_font_lock_fontify_region_with_boundary_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Boundary fontify\nBody.\n\n")
      ;; Fontify only portion
      (font-lock-fontify-region 1 15)
      (list
       'fontified-first-half (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15))
       'not-fontified-second (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(18 20 25))
       ;; Now fontify the rest
       'after-full-fontify (progn
                             (font-lock-fontify-region 15 (point-max))
                             (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 5 10 15 18 20 25))))))))"##,
    );
}

#[test]
fn ft_abyss_face_property_change_after_text_insert_at_each_pos_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAA")
    (put-text-property 1 11 'face 'bold)
    (let ((results nil))
      (dotimes (i 5)
        (goto-char (+ i 2))
        (insert "X")
        (push (list 'pos (+ i 2) 'face (get-text-property (+ i 2) 'face)) results))
      (nreverse results))))"##,
    );
}

#[test]
fn ft_abyss_face_font_lock_mode_without_mode_hooks_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-default-function (if (boundp 'font-lock-fontify-buffer-function)
                                     'bound
                                   'not-bound)
   'font-lock-support-mode (if (boundp 'font-lock-support-mode)
                                font-lock-support-mode
                              'not-bound)
   'fast-lock-mode (if (fboundp 'fast-lock-mode) 'fbound 'not-fbound)
   'lazy-lock-mode (if (fboundp 'lazy-lock-mode) 'fbound 'not-fbound)
   'jit-lock-stealth-fontify
   (condition-case nil
       (progn (jit-lock-stealth-fontify) 'stealth-fontify-ok)
     (error 'no-stealth-fontify))
   'font-lock-after-fontify-buffer
   (condition-case nil
       (progn (font-lock-after-fontify-buffer) 'ok)
     (error 'no-after-fontify))))"##,
    );
}

#[test]
fn ft_infinity_face_text_property_interval_split_then_insert_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXYYYYYYYYYYZZZZZZZZZZWWWWWWWWWW")
    (put-text-property 1 11 'face 'bold :my-tag 'tag-x)
    (put-text-property 11 21 'face 'italic :my-tag 'tag-y)
    (put-text-property 21 31 'face 'underline :my-tag 'tag-z)
    (put-text-property 31 41 'face '(:foreground "red") :my-tag 'tag-w)
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'my-tag))) '(1 6 11 16 21 26 31 36 40)))))
      (let ((v0 (funcall snap)))
        ;; Split interval 2 by inserting
        (goto-char 16) (insert "INSERTED")
        (let ((v1 (funcall snap)))
          ;; Split interval 3 by deleting partial
          (delete-region 28 35)
          (let ((v2 (funcall snap)))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_infinity_face_overlay_end_start_boundary_exact_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWW")
    (let ((ov1 (make-overlay 1 6))) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 6 11))) (overlay-put ov2 'face '(:background "green")))
    (let ((ov3 (make-overlay 11 16))) (overlay-put ov3 'face '(:background "blue")))
    (let ((ov4 (make-overlay 16 21))) (overlay-put ov4 'face '(:background "yellow")))
    (list
     'faces-at-5 (list (get-char-property 5 'face) (length (overlays-at 5)))
     'faces-at-6 (list (get-char-property 6 'face) (length (overlays-at 6)))
     'faces-at-11 (list (get-char-property 11 'face) (length (overlays-at 11)))
     'faces-at-16 (list (get-char-property 16 'face) (length (overlays-at 16)))
     'faces-at-20 (get-char-property 20 'face)
     ;; Insert exactly at boundary
     'after-insert-at-boundary (progn (goto-char 6) (insert "BB") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 6 7 8)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_infinity_face_text_property_with_alternating_values_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert (make-string 50 ?X))
    (let ((colors '(:foreground "red" :foreground "green" :foreground "blue"
                    :foreground "orange" :foreground "purple" :foreground "cyan")))
      (let ((i 0) (pos 1))
        (while (< pos 51)
          (put-text-property pos (min (+ pos 8) 51) 'face (nth (mod i 6) colors))
          (setq pos (+ pos 8))
          (setq i (1+ i)))))
    (list
     'spot-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 9 17 25 33 41 49))
     'interval-count (length (object-intervals (current-buffer)))
     'prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 51)) '(1 9 17 25 33 41))
     'all-different (apply #'not (mapcar (lambda (pos) (equal (get-text-property pos 'face) (get-text-property (+ pos 8) 'face))) '(1 9 17 25 33))))))"##,
    );
}

#[test]
fn ft_infinity_face_set_face_underline_with_colors_and_styles_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-styles-face) (error nil))
  (list
   'red-wave (condition-case nil (progn (set-face-underline 'my-ul-styles-face '(:color "red" :style wave) nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no))
   'blue-line (condition-case nil (progn (set-face-underline 'my-ul-styles-face '(:color "blue" :style line) nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no))
   'green-double (condition-case nil (progn (set-face-underline 'my-ul-styles-face '(:color "green" :style double-line) nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no))
   'orange-dots (condition-case nil (progn (set-face-underline 'my-ul-styles-face '(:color "orange" :style dots) nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no))
   'purple-dash (condition-case nil (progn (set-face-underline 'my-ul-styles-face '(:color "purple" :style dash) nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no))
   'no-color-wave (condition-case nil (progn (set-face-underline 'my-ul-styles-face '(:style wave) nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no))
   'clear (condition-case nil (progn (set-face-underline 'my-ul-styles-face nil nil) (face-attribute 'my-ul-styles-face :underline nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_infinity_face_font_lock_with_overlays_after_edit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Overlay font-lock edit test\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((ov (make-overlay 1 10)))
        (overlay-put ov 'face '(:background "yellow")))
      (let ((snap (lambda () (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-char-property (match-beginning 0) 'face))))))
        (let ((v0 (funcall snap)))
          ;; Edit text under overlay
          (goto-char (point-min))
          (search-forward "TODO")
          (replace-match "DONE")
          (font-lock-ensure (point-min) (point-max))
          (let ((v1 (funcall snap)))
            ;; Move overlay
            (move-overlay ov 15 25)
            (font-lock-ensure (point-min) (point-max))
            (let ((v2 (funcall snap)))
              ;; Delete overlay
              (delete-overlay ov)
              (let ((v3 (funcall snap)))
                (list v0 v1 v2 v3)))))))))"##,
    );
}

#[test]
fn ft_infinity_face_set_face_box_with_various_formats_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-box-formats-face) (error nil))
  (list
   'box-simple (condition-case nil (progn (set-face-attribute 'my-box-formats-face nil :box t) (face-attribute 'my-box-formats-face :box nil 'default-on)) (error 'no))
   'box-line-width (condition-case nil (progn (set-face-attribute 'my-box-formats-face nil :box '(:line-width 3)) (face-attribute 'my-box-formats-face :box nil 'default-on)) (error 'no))
   'box-color (condition-case nil (progn (set-face-attribute 'my-box-formats-face nil :box '(:line-width 2 :color "red")) (face-attribute 'my-box-formats-face :box nil 'default-on)) (error 'no))
   'box-style (condition-case nil (progn (set-face-attribute 'my-box-formats-face nil :box '(:line-width 1 :color "blue" :style pressed-button)) (face-attribute 'my-box-formats-face :box nil 'default-on)) (error 'no))
   'box-released (condition-case nil (progn (set-face-attribute 'my-box-formats-face nil :box '(:style released-button)) (face-attribute 'my-box-formats-face :box nil 'default-on)) (error 'no))
   'box-off (condition-case nil (progn (set-face-attribute 'my-box-formats-face nil :box nil) (face-attribute 'my-box-formats-face :box nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_infinity_face_text_property_character_by_character_access_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 10 'face 'italic)
    (put-text-property 10 15 'face 'underline)
    (put-text-property 15 20 'face '(:foreground "red"))
    (put-text-property 20 27 'face '(:background "yellow"))
    (list
     'char-by-char (let ((result nil) (pos 1))
                     (while (< pos 27)
                       (push (list pos (char-after pos) (get-text-property pos 'face)) result)
                       (setq pos (1+ pos)))
                     (nreverse result))
     'prop-changes-everywhere (let ((pos 1) (changes nil))
                                (while pos
                                  (setq pos (next-single-property-change pos 'face nil 27))
                                  (when pos (push (list pos (get-text-property pos 'face)) changes)))
                                (nreverse changes))))))"##,
    );
}

#[test]
fn ft_infinity_face_font_lock_verbose_and_debug_flags_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-verbose-bound (boundp 'font-lock-verbose)
   'font-lock-verbose-value (if (boundp 'font-lock-verbose) font-lock-verbose 'no-bound)
   'font-lock-support-mode-bound (boundp 'font-lock-support-mode)
   'font-lock-support-mode-value (if (boundp 'font-lock-support-mode) font-lock-support-mode 'no-bound)
   'font-lock-maximum-decoration-bound (boundp 'font-lock-maximum-decoration)
   'font-lock-maximum-decoration-value (if (boundp 'font-lock-maximum-decoration) font-lock-maximum-decoration 'no-bound)
   'font-lock-keywords-case-fold-search-bound (boundp 'font-lock-keywords-case-fold-search)
   'font-lock-defaults-function
   (condition-case nil
       (with-temp-buffer (emacs-lisp-mode) font-lock-defaults)
     (error 'no-defaults))
   'font-lock-set-defaults
   (condition-case nil
       (with-temp-buffer (emacs-lisp-mode) (font-lock-set-defaults) 'ok)
     (error 'no)))))"##,
    );
}

#[test]
fn ft_eternal_face_overlay_make_delete_recreate_same_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay recreate same region face test buffer content text here")
    (put-text-property 1 56 'face '(:foreground "blue"))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 20 30 40 50 55)))))
      (let ((v0 (funcall snap)))
        ;; Create overlay
        (let ((ov (make-overlay 10 30))) (overlay-put ov 'face '(:background "yellow")))
        (let ((v1 (funcall snap)))
          ;; Delete it
          (mapc #'delete-overlay (overlays-at 20))
          (let ((v2 (funcall snap)))
            ;; Recreate different overlay same region
            (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'face '(:foreground "red" :weight bold)))
            (let ((v3 (funcall snap)))
              ;; Delete and recreate third time
              (mapc #'delete-overlay (overlays-at 20))
              (let ((ov3 (make-overlay 10 30))) (overlay-put ov3 'face '(:underline t :slant italic)))
              (let ((v4 (funcall snap)))
                (mapc #'delete-overlay (overlays-in 1 (point-max)))
                (list v0 v1 v2 v3 v4))))))))))"##,
    );
}

#[test]
fn ft_eternal_face_text_property_at_buffer_start_and_end_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Buffer start end properties")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 8 16 'face 'italic)
    (put-text-property 16 25 'face 'underline)
    (put-text-property 25 30 'face '(:foreground "red"))
    (list
     'start-pos-0 (text-properties-at 0)
     'start-pos-1 (get-text-property 1 'face)
     'start-pos-2 (text-properties-at 1)
     'end-pos (text-properties-at (point-max))
     'end-pos-minus-1 (get-text-property (1- (point-max)) 'face)
     'end-pos-plus-1 (text-properties-at (1+ (point-max)))
     'next-from-0 (next-single-property-change 0 'face)
     'prev-from-end (previous-single-property-change (point-max) 'face nil 1)
     'buf-size (point-max))))"##,
    );
}

#[test]
fn ft_eternal_face_font_lock_mode_disable_enable_sequence_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Mode sequence test\nBody.\n\n")
      (let ((snap (lambda () (save-excursion (goto-char (point-min)) (search-forward "TODO") (list (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))))))
        (font-lock-ensure (point-min) (point-max))
        (let ((v0 (funcall snap)))
          (font-lock-mode -1) (let ((v1 (funcall snap)))
          (font-lock-mode 1) (font-lock-ensure (point-min) (point-max)) (let ((v2 (funcall snap)))
          (font-lock-mode -1) (let ((v3 (funcall snap)))
          (font-lock-mode 1) (font-lock-ensure (point-min) (point-max)) (let ((v4 (funcall snap)))
          (font-lock-mode -1) (let ((v5 (funcall snap)))
          (font-lock-mode 1) (font-lock-ensure (point-min) (point-max)) (let ((v6 (funcall snap)))
          (list v0 v1 v2 v3 v4 v5 v6)))))))))))))))"##,
    );
}

#[test]
fn ft_eternal_face_overlay_before_after_propertize_roundtrip_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before after propertize roundtrip face test buffer content")
    (let* ((before-str (propertize "[[B]]" 'face '(:foreground "red" :weight bold)))
           (after-str (propertize "{{A}}" 'face '(:foreground "blue" :slant italic)))
           (ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string before-str)
      (overlay-put ov 'after-string after-str)
      (list
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'after-face (get-text-property 0 (overlay-get ov 'after-string))
       'overlay-face (overlay-get ov 'face)
       'before-str (overlay-get ov 'before-string)
       'after-str (overlay-get ov 'after-string)
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_eternal_face_face_spec_set_with_multiple_displays_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (list
   'face-spec-set-fbound (fboundp 'face-spec-set)
   'spec-with-multiple-displays
   (condition-case nil
       (face-spec-choose '(((type x w32 ns) (:weight bold))
                           ((type x) (:slant italic))
                           ((type w32) (:underline t))
                           (t (:weight normal))))
     (error 'no))
   'spec-with-class
   (condition-case nil
       (face-spec-choose '(((class color) (min-colors 88)) (:foreground "blue")
                           ((class color) (min-colors 8)) (:foreground "green")
                           ((class mono)) (:foreground "black")))
     (error 'no))
   'spec-with-background
   (condition-case nil
       (face-spec-choose '(((class color) (background light)) (:foreground "black" :background "white")
                           ((class color) (background dark)) (:foreground "white" :background "black")))
     (error 'no)))))"##,
    );
}

#[test]
fn ft_eternal_face_text_property_interval_object_intervals_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (list
     'one-char-no-props (length (object-intervals (current-buffer)))
     (progn
       (put-text-property 1 2 'face 'bold)
       (list 'one-char-bold (length (object-intervals (current-buffer)))
             (get-text-property 1 'face)))
     (progn
       (goto-char 2) (insert "YYYY")
       (list 'more-chars (length (object-intervals (current-buffer)))
             (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 2 3))))
     (progn
       (set-text-properties 1 (point-max) nil)
       (list 'all-cleared (length (object-intervals (current-buffer)))))
     (progn
       (put-text-property 1 3 'face 'italic)
       (put-text-property 3 (point-max) 'face 'underline)
       (list 'two-intervals (length (object-intervals (current-buffer)))
             (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 2 3 4)))))))"##,
    );
}

#[test]
fn ft_eternal_face_font_lock_fontify_with_narrowed_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Narrowed fontify\nBody narrow.\n\n")
      (insert "* DONE Outside narrow\nBody outside.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((full-face (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        ;; Narrow to first heading
        (goto-char (point-min))
        (search-forward "TODO Narrowed fontify")
        (beginning-of-line)
        (org-narrow-to-subtree)
        (let ((narrowed-face (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
          ;; Unfontify and refontify within narrow
          (font-lock-unfontify-buffer)
          (font-lock-fontify-buffer)
          (let ((refontified-narrow-face (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
            (widen)
            (list full-face narrowed-face refontified-narrow-face))))))))"##,
    );
}

#[test]
fn ft_eternal_face_property_list_manipulation_via_plist_put_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((plist (list :weight 'bold :slant 'italic :foreground "red")))
    (list
     'initial-plist plist
     'initial-plist-len (length plist)
     'get-weight (plist-get plist :weight)
     'get-slant (plist-get plist :slant)
     'get-fg (plist-get plist :foreground)
     'put-underline (let ((new (plist-put plist :underline t))) (list new (plist-get new :underline)))
     'put-override-weight (let ((new (plist-put plist :weight 'extra-bold))) (list new (plist-get new :weight)))
     'remove-fg (let ((new (plist-put plist :foreground nil))) (list new (plist-get new :foreground)))
     'add-multiple (let ((new (copy-sequence plist)))
                     (setq new (plist-put new :underline t))
                     (setq new (plist-put new :overline t))
                     (setq new (plist-put new :box t))
                     (list new (length new)))))))"##,
    );
}

#[test]
fn ft_omega2_face_text_property_interval_count_after_many_edits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWWVVVVV")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (let ((counts nil))
      (push (length (object-intervals (current-buffer))) counts)
      (goto-char 6) (insert "A") (push (length (object-intervals (current-buffer))) counts)
      (goto-char 12) (insert "B") (push (length (object-intervals (current-buffer))) counts)
      (delete-region 8 15) (push (length (object-intervals (current-buffer))) counts)
      (goto-char 18) (insert "CDE") (push (length (object-intervals (current-buffer))) counts)
      (delete-region 1 10) (push (length (object-intervals (current-buffer))) counts)
      (list (nreverse counts) (mapcar (lambda (pos) (goto-char pos) (if (< pos (point-max)) (get-text-property pos 'face) 'eob)) '(1 3 6 10 15))))))"##,
    );
}

#[test]
fn ft_omega2_overlay_with_overlay_put_multiple_face_adds_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with multiple face properties added incrementally test buffer here")
    (let ((ov (make-overlay 1 60)))
      (overlay-put ov 'face '(:foreground "blue"))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold :slant 'italic))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold :slant 'italic :underline t))
      (overlay-put ov 'face (list :foreground "blue" :weight 'bold :slant 'italic :underline t :background "yellow"))
      (list
       'final-face (overlay-get ov 'face)
       'face-at-1 (get-char-property 1 'face)
       'face-at-30 (get-char-property 30 'face)
       'face-at-59 (get-char-property 59 'face)
       'overlay-props-length (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_omega2_face_attribute_with_inherit_default_resolution() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-inherit-resolve-face) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-resolve-face nil :inherit 'bold :slant 'italic) (error nil))
  (list
   'direct-slant (face-attribute 'my-inherit-resolve-face :slant nil 'default-on)
   'inherited-weight (face-attribute 'my-inherit-resolve-face :weight nil 'default-on)
   'direct-foreground (condition-case nil (face-attribute 'my-inherit-resolve-face :foreground nil 'default-on) (error 'no))
   'inherited-slant-from-bold (face-attribute 'bold :slant nil 'default-on)
   'inherited-weight-from-bold (face-attribute 'bold :weight nil 'default-on)
   'face-equal-inherit (condition-case nil (face-equal 'my-inherit-resolve-face 'default) (error 'no))
   'face-differs-inherit (face-differs-from-default-p 'my-inherit-resolve-face))))"##,
    );
}

#[test]
fn ft_omega2_font_lock_keywords_with_overwrite_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "OVERWRITE keyword face test with font-lock")
    (font-lock-add-keywords nil '(("\\<\\(OVERWRITE\\)\\>" 1 font-lock-warning-face overwrite)
                                  ("\\<\\(keyword\\)\\>" 1 '(:foreground "red") t)
                                  ("\\<\\(face\\)\\>" 1 '(:foreground "green") t)
                                  ("\\<\\(font-lock\\)\\>" 1 '(:foreground "purple") t)))
    (font-lock-fontify-buffer)
    (mapcar (lambda (needle)
              (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))))
            '("OVERWRITE" "keyword" "face" "font-lock"))))"##,
    );
}

#[test]
fn ft_omega2_face_text_property_with_string_value_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "String value face property test buffer content here")
    (put-text-property 1 7 'face "bold")
    (put-text-property 7 14 'face "italic")
    (put-text-property 14 22 'face "underline")
    (put-text-property 22 31 'face (:foreground "red"))
    (list
     'string-face-value-1 (get-text-property 1 'face)
     'string-face-value-2 (get-text-property 7 'face)
     'string-face-value-3 (get-text-property 14 'face)
     'proper-face-value (get-text-property 22 'face)
     'facep-string1 (facep (get-text-property 1 'face))
     'facep-string2 (facep (get-text-property 7 'face))
     'facep-proper (facep (get-text-property 22 'face))))))"##,
    );
}

#[test]
fn ft_omega2_face_overlay_with_property_list_at_boundaries_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with multi props at boundaries face test area content")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "overlay boundary test")
      (overlay-put ov 'evaporate t)
      (list
       'at-start-1 (get-char-property 9 'face)
       'at-start-2 (get-char-property 10 'face)
       'inside (get-char-property 25 'face)
       'at-end-1 (get-char-property 39 'face)
       'at-end-2 (get-char-property 40 'face)
       'overlay-start (overlay-start ov)
       'overlay-end (overlay-end ov)
       'overlay-props-count (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_omega2_font_lock_keywords_multiple_groups_same_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "func(42, \"string\")")
    (font-lock-add-keywords nil
                            '(("\\<\\(func\\)\\>" 1 font-lock-function-name-face)
                              ("(\\|)" 0 font-lock-keyword-face)
                              ("\\([0-9]+\\)" 1 '(:foreground "blue"))
                              ("\"[^\"]*\"" 0 font-lock-string-face)))
    (font-lock-fontify-buffer)
    (mapcar (lambda (needle)
              (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face))))
            '("func" "(" "42" "string")))))"##,
    );
}

#[test]
fn ft_omega2_face_set_unspecified_then_recheck_attrs_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-reset-attrs-face) (error nil))
  (condition-case nil (set-face-attribute 'my-reset-attrs-face nil :weight 'heavy :slant 'oblique :foreground "red" :underline t :overline t :box '(:line-width 2) :strike-through t :inverse-video t :height 150) (error nil))
  (list
   'before-weight (face-attribute 'my-reset-attrs-face :weight nil 'default-on)
   'before-fg (face-attribute 'my-reset-attrs-face :foreground nil 'default-on)
   'set-weight-unspecified (condition-case nil (progn (set-face-attribute 'my-reset-attrs-face nil :weight 'unspecified) 'ok) (error 'no))
   'after-weight (face-attribute 'my-reset-attrs-face :weight nil 'default-on)
   'set-all-unspecified (condition-case nil (progn (set-face-attribute 'my-reset-attrs-face nil :weight 'unspecified :slant 'unspecified :foreground 'unspecified :underline 'unspecified :overline 'unspecified :box 'unspecified :strike-through 'unspecified :inverse-video 'unspecified :height 'unspecified) 'ok) (error 'no))
   'after-all (list (face-attribute 'my-reset-attrs-face :weight nil 'default-on)
                     (condition-case nil (face-attribute 'my-reset-attrs-face :foreground nil 'default-on) (error 'no))
                     (face-attribute 'my-reset-attrs-face :underline nil 'default-on))))))"##,
    );
}

#[test]
fn ft_xero_face_char_property_vs_text_property_precedence_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Char vs text property precedence test text here now end")
    (put-text-property 1 50 'face '(:foreground "blue"))
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 100))
    (let ((ov2 (make-overlay 15 25)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold))
      (overlay-put ov2 'priority 200))
    (list
     'text-prop-only (get-text-property 1 'face)
     'char-prop-overlay1 (get-char-property 12 'face)
     'char-prop-overlay2 (get-char-property 20 'face)
     'char-prop-and-overlay-12 (get-char-property-and-overlay 12 'face)
     'char-prop-and-overlay-20 (get-char-property-and-overlay 20 'face)
     'text-prop-inside-overlays (get-text-property 20 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 50)) 'cleaned))))"##,
    );
}

#[test]
fn ft_xero_face_font_lock_fontify_region_with_fontified_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun test () 42)\n")
    (list
     'before-any-fontify (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15))
     'after-region-1-to-10 (progn
                             (font-lock-fontify-region 1 10)
                             (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15)))
     'after-full (progn
                   (font-lock-fontify-region 10 (point-max))
                   (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15)))
     'faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 10 15))))))"##,
    );
}

#[test]
fn ft_xero_face_text_property_all_intervals_list_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:foreground "blue"))
    (put-text-property 31 36 'face '(:background "cyan"))
    (list
     'interval-count (length (object-intervals (current-buffer)))
     'interval-list (mapcar (lambda (ov) (list (overlay-start (car (overlays-in (overlay-start ov) (overlay-end ov)))) (overlay-end (car (overlays-in (overlay-start ov) (overlay-end ov))))))
                            (object-intervals (current-buffer)))
     'manual-interval-walk (let ((pos 1) (result nil))
                             (while pos
                               (let ((next (next-single-property-change pos 'face nil 36)))
                                 (when next (push (list pos next (get-text-property pos 'face)) result))
                                 (setq pos next)))
                             (nreverse result))))))"##,
    );
}

#[test]
fn ft_xero_face_font_lock_add_keywords_with_keep_flag_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Keep keyword flag test with font-lock keywords")
    ;; Add with keep flag
    (font-lock-add-keywords nil '(("\\<\\(keep\\)\\>" 1 font-lock-warning-face keep)
                                  ("\\<\\(flag\\)\\>" 1 '(:foreground "blue") t)))
    (font-lock-fontify-buffer)
    (list
     'keep-word-face (save-excursion (goto-char (point-min)) (search-forward "keep") (get-text-property (match-beginning 0) 'face))
     'flag-word-face (save-excursion (goto-char (point-min)) (search-forward "flag") (get-text-property (match-beginning 0) 'face))
     'remove-keep (condition-case nil
                      (progn
                        (font-lock-remove-keywords nil '(("\\<\\(keep\\)\\>" 1 font-lock-warning-face keep)))
                        'removed)
                    (error 'remove-failed))
     'remove-flag (condition-case nil
                      (progn
                        (font-lock-remove-keywords nil '(("\\<\\(flag\\)\\>" 1 '(:foreground "blue") t)))
                        'removed)
                    (error 'remove-failed))))))"##,
    );
}

#[test]
fn ft_xero_face_overlay_modification_hooks_trigger_face_preserve() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (defvar my-ov-mod-count 0)
  (defun my-ov-mod-fn (ov after beg end &optional len)
    (setq my-ov-mod-count (1+ my-ov-mod-count)))
  (with-temp-buffer
    (insert "Overlay modification hooks face preservation test text")
    (put-text-property 1 52 'face '(:foreground "blue"))
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'modification-hooks (list 'my-ov-mod-fn)))
    (list
     'before-count my-ov-mod-count
     'before-face (get-char-property 15 'face)
     ;; Modify inside overlay
     (progn (goto-char 20) (insert "X") (list 'after-mod-count my-ov-mod-count 'face-after-mod (get-char-property 20 'face)))
     ;; Delete inside overlay
     (progn (delete-region 15 25) (list 'after-delete-count my-ov-mod-count 'face-after-delete (get-char-property 15 'face)))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_xero_face_with_text_property_face_and_category_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Text property face and category combined test buffer content")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 1 10 'category 'cat-bold)
    (put-text-property 10 20 'face 'italic)
    (put-text-property 10 20 'category 'cat-italic)
    (put-text-property 20 30 'face 'underline)
    (put-text-property 20 30 'category 'cat-underline)
    (put-text-property 30 40 'face '(:foreground "red"))
    (put-text-property 30 40 'category 'cat-red)
    (put-text-property 40 55 'face '(:background "yellow"))
    (list
     'face-and-category (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'category) (get-char-property pos 'category))) '(1 5 10 15 20 25 30 35 40 50 54))
     'prop-search (text-property-any 1 55 'category 'cat-italic)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_xero_face_font_lock_after_fontify_functions_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-after-fontify-buffer-fbound (fboundp 'font-lock-after-fontify-buffer)
   'font-lock-after-fontify-region-fbound (fboundp 'font-lock-after-fontify-region)
   'font-lock-after-change-function-fbound (fboundp 'font-lock-after-change-function)
   'font-lock-unfontify-buffer-fbound (fboundp 'font-lock-unfontify-buffer)
   'font-lock-unfontify-region-fbound (fboundp 'font-lock-unfontify-region)
   'font-lock-default-fontify-buffer-fbound (fboundp 'font-lock-default-fontify-buffer)
   'font-lock-default-fontify-region-fbound (fboundp 'font-lock-default-fontify-region)
   'font-lock-default-unfontify-buffer-fbound (fboundp 'font-lock-default-unfontify-buffer)
   'font-lock-default-unfontify-region-fbound (fboundp 'font-lock-default-unfontify-region)
   'font-lock-fontify-syntactically-region-fbound (fboundp 'font-lock-fontify-syntactically-region)
   'font-lock-fontify-keywords-region-fbound (fboundp 'font-lock-fontify-keywords-region))))"##,
    );
}

#[test]
fn ft_xero_face_set_face_attribute_int_vs_float_height_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-int-float-face) (error nil))
  (list
   'set-height-int-100 (condition-case nil (progn (set-face-attribute 'my-int-float-face nil :height 100) (face-attribute 'my-int-float-face :height nil 'default-on)) (error 'no))
   'set-height-int-200 (condition-case nil (progn (set-face-attribute 'my-int-float-face nil :height 200) (face-attribute 'my-int-float-face :height nil 'default-on)) (error 'no))
   'set-height-float-1.0 (condition-case nil (progn (set-face-attribute 'my-int-float-face nil :height 1.0) (face-attribute 'my-int-float-face :height nil 'default-on)) (error 'no))
   'set-height-float-1.5 (condition-case nil (progn (set-face-attribute 'my-int-float-face nil :height 1.5) (face-attribute 'my-int-float-face :height nil 'default-on)) (error 'no))
   'set-height-float-0.8 (condition-case nil (progn (set-face-attribute 'my-int-float-face nil :height 0.8) (face-attribute 'my-int-float-face :height nil 'default-on)) (error 'no))
   'set-height-int-300 (condition-case nil (progn (set-face-attribute 'my-int-float-face nil :height 300) (face-attribute 'my-int-float-face :height nil 'default-on)) (error 'no))
   'default-height (face-attribute 'default :height nil 'default-on))))"##,
    );
}

#[test]
fn ft_zeta_face_property_map_on_face_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOP")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (list
     'property-map (let ((pos 1) (props nil))
                     (while (< pos 17)
                       (push (cons pos (get-text-property pos 'face)) props)
                       (setq pos (1+ pos)))
                     (nreverse props))
     'face-change-positions (let ((pos 1) (changes nil))
                              (while pos
                                (setq pos (next-single-property-change pos 'face nil 17))
                                (when pos (push pos changes)))
                              (nreverse changes))
     'interval-objects (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov)))
                               (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_zeta_overlay_evaporate_with_text_insert_and_delete_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Evaporate overlay insert delete chain test content text here now")
    (let ((ovs (list (let ((ov (make-overlay 5 10)))
                       (overlay-put ov 'face '(:background "red"))
                       (overlay-put ov 'evaporate t) ov)
                     (let ((ov (make-overlay 15 25)))
                       (overlay-put ov 'face '(:background "green"))
                       (overlay-put ov 'evaporate t) ov)
                     (let ((ov (make-overlay 30 45)))
                       (overlay-put ov 'face '(:background "blue"))
                       (overlay-put ov 'evaporate t) ov))))
    (list
     'before-ops (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 8 15 20 30 35 45 55))
     'after-insert (progn (goto-char 7) (insert "NEW") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 7 10 15)))
     'after-delete (progn (delete-region 25 40) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(15 20 25 30)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned)))))"##,
    );
}

#[test]
fn ft_zeta_face_make_face_with_every_attribute_set_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (make-face 'my-every-attr-face) (error nil))
  (condition-case nil
      (set-face-attribute 'my-every-attr-face nil
                          :family "Monospace"
                          :foundry "misc"
                          :width 'condensed
                          :height 120
                          :weight 'bold
                          :slant 'italic
                          :underline '(:color "red" :style wave)
                          :overline t
                          :strike-through t
                          :box '(:line-width 3 :color "blue" :style pressed-button)
                          :inverse-video t
                          :foreground "DarkGreen"
                          :background "LightYellow"
                          :stipple nil
                          :inherit 'default
                          :extend t
                          :raise 0.1
                          :distant-foreground "gray")
    (error nil))
  (list
   'family (face-attribute 'my-every-attr-face :family nil 'default-on)
   'width (face-attribute 'my-every-attr-face :width nil 'default-on)
   'height (face-attribute 'my-every-attr-face :height nil 'default-on)
   'weight (face-attribute 'my-every-attr-face :weight nil 'default-on)
   'slant (face-attribute 'my-every-attr-face :slant nil 'default-on)
   'underline (face-attribute 'my-every-attr-face :underline nil 'default-on)
   'overline (face-attribute 'my-every-attr-face :overline nil 'default-on)
   'strike (face-attribute 'my-every-attr-face :strike-through nil 'default-on)
   'box (face-attribute 'my-every-attr-face :box nil 'default-on)
   'inverse (face-attribute 'my-every-attr-face :inverse-video nil 'default-on)
   'fg (condition-case nil (face-foreground 'my-every-attr-face nil 'default-on) (error 'no))
   'bg (condition-case nil (face-background 'my-every-attr-face nil 'default-on) (error 'no))
   'extend (face-attribute 'my-every-attr-face :extend nil 'default-on)
   'raise (face-attribute 'my-every-attr-face :raise nil 'default-on)
   'all-attrs-count (length (face-all-attributes 'my-every-attr-face (selected-frame))))))"##,
    );
}

#[test]
fn ft_zeta_font_lock_unfontify_region_with_boundaries_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun a () 1)\n(defun b () 2)\n(defun c () 3)\n")
    (font-lock-ensure (point-min) (point-max))
    (let ((all-fontified-before (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30))))
      ;; Unfontify middle region only
      (font-lock-unfontify-region 12 24)
      (let ((after-unfontify (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 15 20 25 30))))
        ;; Refontify everything
        (font-lock-fontify-buffer)
        (let ((after-refontify (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 15 20 25 30))))
          (list all-fontified-before after-unfontify after-refontify)))))))"##,
    );
}

#[test]
fn ft_zeta_face_text_properties_in_nested_buffer_inserts_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Outer content with face")
    (put-text-property 1 23 'face 'bold)
    ;; Nested insert via buffer
    (insert-buffer-substring (current-buffer) 1 10)
    (put-text-property 23 32 'face 'italic)
    (list
     'faces-across (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 23 28 31))
     'interval-count (length (object-intervals (current-buffer)))
     'buffer-length (point-max))))"##,
    );
}

#[test]
fn ft_zeta_face_overlay_priority_multiple_same_value_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Same priority multiple overlays face test buffer content text")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 50))
    (let ((ov2 (make-overlay 5 25))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 50))
    (let ((ov3 (make-overlay 10 30))) (overlay-put ov3 'face '(:underline t)) (overlay-put ov3 'priority 50))
    (let ((ov4 (make-overlay 15 35))) (overlay-put ov4 'face '(:slant italic)) (overlay-put ov4 'priority 50))
    (list
     'faces-at-positions (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 40 50))
     'overlay-count (mapcar (lambda (pos) (goto-char pos) (length (overlays-at pos))) '(5 10 15 20 25 30))
     'all-priorities (mapcar (lambda (ov) (overlay-get ov 'priority)) (list ov1 ov2 ov3 ov4))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))"##,
    );
}

#[test]
fn ft_zeta_face_face_all_attributes_length_and_keys_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-all-atts-length (condition-case nil (length (face-all-attributes 'default (selected-frame))) (error 'no))
   'bold-all-atts-length (condition-case nil (length (face-all-attributes 'bold (selected-frame))) (error 'no))
   'default-atts-keys (condition-case nil
                          (let ((atts (face-all-attributes 'default (selected-frame)))
                                (keys nil) (i 0))
                            (while (< i (length atts))
                              (when (= 0 (mod i 2)) (push (nth i atts) keys))
                              (setq i (1+ i)))
                            (nreverse keys))
                        (error 'no))
   'default-has-weight (condition-case nil (plist-get (face-all-attributes 'default (selected-frame)) :weight) (error 'no))
   'default-has-family (condition-case nil (plist-get (face-all-attributes 'default (selected-frame)) :family) (error 'no))
   'default-has-fg (condition-case nil (plist-get (face-all-attributes 'default (selected-frame)) :foreground) (error 'no)))))"##,
    );
}

#[test]
fn ft_zeta_face_font_lock_fontify_entire_buffer_twice_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Twice fontify\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((first-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 20))))
        ;; Fontify again
        (font-lock-ensure (point-min) (point-max))
        (let ((second-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 20))))
          (list first-faces second-faces)))))))"##,
    );
}

#[test]
fn ft_alpha_face_font_lock_syntactic_keywords_in_string_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(format \"hello %s\" name)\n")
    (font-lock-fontify-buffer)
    (list
     'format-face (save-excursion (goto-char (point-min)) (search-forward "format") (get-text-property (match-beginning 0) 'face))
     'string-quote-face (save-excursion (goto-char (point-min)) (search-forward "\"") (get-text-property (match-beginning 0) 'face))
     'hello-face (save-excursion (goto-char (point-min)) (search-forward "hello") (get-text-property (match-beginning 0) 'face))
     'percent-face (save-excursion (goto-char (point-min)) (search-forward "%s") (get-text-property (match-beginning 0) 'face))
     'name-face (save-excursion (goto-char (point-min)) (search-forward "name") (get-text-property (match-beginning 0) 'face)))))"##,
    );
}

#[test]
fn ft_alpha_face_text_property_change_at_midpoint_between_regions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAABBBBBBBBBBCCCCCCCCCCDDDDDDDDDD")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 11 21 'face nil)
    (put-text-property 21 31 'face 'italic)
    (put-text-property 31 41 'face nil)
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 11 15 20 21 25 30 31 35 40))
     'prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 41)) '(1 11 21 31))
     'find-nil (text-property-any 1 41 'face nil)
     'find-first-bold (text-property-any 1 41 'face 'bold)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_alpha_overlay_combined_with_text_property_both_visible() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Combined overlay and text property face test content buffer")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 10 20 'font-lock-face 'italic)
    (put-text-property 20 30 'face 'underline)
    (let ((ov1 (make-overlay 5 15))) (overlay-put ov1 'face '(:background "yellow")))
    (let ((ov2 (make-overlay 15 25))) (overlay-put ov2 'face '(:foreground "red" :weight bold)))
    (list
     'text-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 10 15 20 25 30 40 50 55))
     'char-props (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 15 20 25 30))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))"##,
    );
}

#[test]
fn ft_alpha_face_font_lock_background_color_via_overlay_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Background color overlay test with font-lock faces here")
    (font-lock-add-keywords nil '(("\\<\\(Background\\)\\>" 1 font-lock-warning-face t)))
    (font-lock-fontify-buffer)
    (let ((ov (make-overlay 1 15)))
      (overlay-put ov 'face '(:background "cyan" :foreground "black")))
    (let ((ov2 (make-overlay 25 45)))
      (overlay-put ov2 'face '(:background "yellow" :foreground "black")))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 40 50))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))"##,
    );
}

#[test]
fn ft_alpha_face_all_face_ids_comparison_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-id-default (if (fboundp 'face-id) (face-id 'default) 'no-func)
   'face-id-bold (if (fboundp 'face-id) (face-id 'bold) 'no-func)
   'face-id-italic (if (fboundp 'face-id) (face-id 'italic) 'no-func)
   'face-id-bold-italic (if (fboundp 'face-id) (face-id 'bold-italic) 'no-func)
   'face-id-underline (if (fboundp 'face-id) (face-id 'underline) 'no-func)
   'face-equal-default-bold (condition-case nil (face-equal 'default 'bold) (error 'no))
   'face-equal-bold-bold (condition-case nil (face-equal 'bold 'bold) (error 'no))
   'face-differs-bold (face-differs-from-default-p 'bold)
   'face-differs-italic (face-differs-from-default-p 'italic)
   'face-differs-default (face-differs-from-default-p 'default))))"##,
    );
}

#[test]
fn ft_alpha_face_with_narrowed_buffer_text_property_persistence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Narrow test\nBody narrow.\n\n")
      (insert "* DONE Outside\nBody outside.\n\n")
      (font-lock-ensure (point-min) (point-max))
      ;; Record full faces
      (let ((full-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 30 35 40))))
        ;; Narrow
        (goto-char (point-min))
        (search-forward "TODO Narrow test")
        (beginning-of-line)
        (org-narrow-to-subtree)
        (let ((narrowed-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15))))
          ;; Widen
          (widen)
          (let ((widened-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 30 35 40))))
            (list full-faces narrowed-faces widened-faces))))))))"##,
    );
}

#[test]
fn ft_alpha_face_overlay_string_edge_of_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Edge of buffer overlay string test text now end")
    (let ((ov-start (make-overlay 1 5)))
      (overlay-put ov-start 'face '(:background "red"))
      (overlay-put ov-start 'before-string (propertize ">>>" 'face '(:foreground "blue"))))
    (let ((ov-end (make-overlay 40 45)))
      (overlay-put ov-end 'face '(:background "yellow"))
      (overlay-put ov-end 'after-string (propertize "<<<" 'face '(:foreground "red"))))
    (list
     'start-before-face (get-text-property 0 (overlay-get ov-start 'before-string))
     'end-after-face (get-text-property 0 (overlay-get ov-end 'after-string))
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 3 5 20 40 45))
     (progn (mapc #'delete-overlay (overlays-in 1 45)) 'cleaned))))"##,
    );
}

#[test]
fn ft_alpha_face_font_lock_ensure_vs_fontify_consistency_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun consistent-test () t)\n")
    (list
     'fontify-buffer-faces (progn
                             (font-lock-fontify-buffer)
                             (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 21)))
     'ensure-faces (progn
                     (font-lock-unfontify-buffer)
                     (font-lock-ensure (point-min) (point-max))
                     (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 21)))
     'consistent (progn
                   (font-lock-unfontify-buffer)
                   (font-lock-fontify-buffer)
                   (let ((fb-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 21))))
                     (font-lock-unfontify-buffer)
                     (font-lock-ensure (point-min) (point-max))
                     (let ((fe-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 21))))
                       (equal fb-faces fe-faces))))))))"##,
    );
}

#[test]
fn ft_beta_face_text_property_with_two_distinct_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Two distinct faces on same region test buffer content text here")
    (put-text-property 1 20 'face 'bold)
    (put-text-property 1 20 'font-lock-face 'italic)
    (put-text-property 20 40 'face 'underline)
    (put-text-property 20 40 'font-lock-face '(:foreground "red"))
    (put-text-property 40 55 'face '(:background "yellow"))
    (put-text-property 40 55 'font-lock-face nil)
    (list
     'faces-and-lock-face (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 10 20 30 40 50))
     'char-props (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 10 20 30 40 50))
     'text-props-at-each (mapcar (lambda (pos) (goto-char pos) (length (text-properties-at pos))) '(1 20 40)))))"##,
    );
}

#[test]
fn ft_beta_face_font_lock_add_then_remove_multiple_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Add remove multiple keywords face test buffer content text area")
    (font-lock-add-keywords nil
                            '(("\\<\\(Add\\)\\>" 1 font-lock-warning-face t)
                              ("\\<\\(remove\\)\\>" 1 '(:foreground "red") t)
                              ("\\<\\(multiple\\)\\>" 1 '(:foreground "green") t)
                              ("\\<\\(keywords\\)\\>" 1 '(:foreground "blue") t)
                              ("\\<\\(face\\)\\>" 1 '(:foreground "purple") t)))
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (get-text-property (match-beginning 0) 'face)))
                      '("Add" "remove" "multiple" "keywords" "face" "test"))))
      ;; Remove some keywords
      (font-lock-remove-keywords nil
                                  '(("\\<\\(remove\\)\\>" 1 '(:foreground "red") t)
                                    ("\\<\\(face\\)\\>" 1 '(:foreground "purple") t)))
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (get-text-property (match-beginning 0) 'face)))
                        '("Add" "remove" "multiple" "keywords" "face" "test"))))
        ;; Remove remaining
        (font-lock-remove-keywords nil
                                    '(("\\<\\(Add\\)\\>" 1 font-lock-warning-face t)
                                      ("\\<\\(multiple\\)\\>" 1 '(:foreground "green") t)
                                      ("\\<\\(keywords\\)\\>" 1 '(:foreground "blue") t)))
        (font-lock-fontify-buffer)
        (let ((v2 (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (get-text-property (match-beginning 0) 'face)))
                          '("Add" "remove" "multiple" "keywords" "face" "test"))))
          (list v0 v1 v2))))))"##,
    );
}

#[test]
fn ft_beta_overlay_move_then_resize_then_delete_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHH")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30 35 40)))))
        (let ((v0 (funcall snap)))
          ;; Move
          (move-overlay ov 20 30) (let ((v1 (funcall snap)))
          ;; Resize (change end)
          (move-overlay ov 20 25) (let ((v2 (funcall snap)))
          ;; Resize (change start)
          (move-overlay ov 15 25) (let ((v3 (funcall snap)))
          ;; Move to new position
          (move-overlay ov 30 40) (let ((v4 (funcall snap)))
          (delete-overlay ov)
          (list v0 v1 v2 v3 v4))))))))))"##,
    );
}

#[test]
fn ft_beta_face_property_interval_access_performance_pattern_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert (make-string 200 ?X))
    (put-text-property 1 50 'face 'bold)
    (put-text-property 50 100 'face 'italic)
    (put-text-property 100 150 'face 'underline)
    (put-text-property 150 201 'face '(:foreground "red"))
    (list
     'face-at-boundaries (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 49 50 51 99 100 101 149 150 151 199 200))
     'next-prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 201)) '(1 50 100 150))
     'prev-prop-changes (mapcar (lambda (pos) (previous-single-property-change pos 'face nil 1)) '(50 100 150 200))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_beta_face_set_face_foreground_with_named_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-named-colors-face) (error nil))
  (list
   'set-Red (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Red" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Green (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Green" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Blue (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Blue" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Orange (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Orange" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Purple (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Purple" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Cyan (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Cyan" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Magenta (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Magenta" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Yellow (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Yellow" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Brown (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Brown" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no))
   'set-Gray (condition-case nil (progn (set-face-foreground 'my-named-colors-face "Gray" nil) (face-foreground 'my-named-colors-face nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_beta_face_with_no_font_lock_keywords_at_all_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "No font-lock keywords at all in this buffer")
    (font-lock-fontify-buffer)
    (list
     'faces-after-fontify (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 20 30 40))
     'no-keywords-bound (if (boundp 'font-lock-keywords) font-lock-keywords 'no-bound)
     'no-keywords-case-fold (font-lock-keywords-case-fold)
     'font-lock-defaults (condition-case nil (font-lock-defaults) (error 'no))
     'font-lock-set-defaults-done (condition-case nil (progn (font-lock-set-defaults) 'done) (error 'no)))))"##,
    );
}

#[test]
fn ft_beta_face_overlay_window_property_nil_vs_current_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Window property overlay face test region text content area now end")
    (let ((ov-all (make-overlay 1 57)))
      (overlay-put ov-all 'face '(:background "yellow"))
      (overlay-put ov-all 'window nil))
    (let ((ov-cur (make-overlay 10 30)))
      (overlay-put ov-cur 'face '(:foreground "red" :weight bold))
      (overlay-put ov-cur 'window (selected-window)))
    (list
     'faces-all (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 45 56))
     'all-window (overlay-get ov-all 'window)
     'cur-window (overlay-get ov-cur 'window)
     (progn (mapc #'delete-overlay (overlays-in 1 57)) 'cleaned))))"##,
    );
}

#[test]
fn ft_beta_face_text_property_char_property_overlay_property_precedence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Precedence test: text-property char-property overlay triple stack data now")
    (put-text-property 1 60 'face '(:foreground "blue"))
    (put-text-property 1 60 'font-lock-face '(:weight bold))
    (let ((ov1 (make-overlay 10 40))) (overlay-put ov1 'face '(:background "yellow")) (overlay-put ov1 'priority 50))
    (let ((ov2 (make-overlay 20 50))) (overlay-put ov2 'face '(:foreground "red")) (overlay-put ov2 'priority 100))
    (list
     'text-prop (get-text-property 1 'face)
     'char-prop-1 (get-char-property 1 'face)
     'char-prop-25 (get-char-property 25 'face)
     'char-prop-45 (get-char-property 45 'face)
     'char-prop-and-overlay (get-char-property-and-overlay 25 'face)
     'text-prop-inside (get-text-property 25 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 60)) 'cleaned))))"##,
    );
}

#[test]
fn ft_gamma_face_text_property_single_property_no_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Single property with no face test buffer content text")
    (put-text-property 1 47 'my-property 'yes)
    (list
     'no-face-prop (get-text-property 1 'face)
     'has-my-prop (get-text-property 1 'my-property)
     'text-props-at-1 (text-properties-at 1)
     'text-props-count (length (text-properties-at 1))
     'next-prop-change (next-property-change 1)
     'single-property-intervals (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_gamma_font_lock_add_keywords_with_override_flags_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "PREPEND vs APPEND vs OVERWRITE keyword flags test buffer")
    ;; Prepend has lowest priority (won't override existing)
    (font-lock-add-keywords nil '(("\\<\\(PREPEND\\)\\>" 1 font-lock-warning-face prepend) ("\\<\\(APPEND\\)\\>" 1 '(:foreground "red") append) ("\\<\\(OVERWRITE\\)\\>" 1 '(:foreground "green" :weight bold) overwrite)))
    (font-lock-fontify-buffer)
    (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face)))) '("PREPEND" "APPEND" "OVERWRITE" "vs" "flags" "buffer"))))"##,
    );
}

#[test]
fn ft_gamma_face_with_overlay_start_end_equal_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov1 (make-overlay 5 5))) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 10 15))) (overlay-put ov2 'face '(:background "green")))
    (let ((ov3 (make-overlay 20 20))) (overlay-put ov3 'face '(:background "blue")))
    (list
     'empty-overlay-1-start (overlay-start ov1)
     'empty-overlay-1-end (overlay-end ov1)
     'empty-overlay-3-start (overlay-start ov3)
     'empty-overlay-3-end (overlay-end ov3)
     'faces-at-boundaries (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(4 5 6 10 12 15 19 20 21))
     ;; Insert at zero-width overlay
     'after-insert (progn (goto-char 5) (insert "XX") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(4 5 7 10 12 15 20 21 22)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_gamma_face_set_face_underline_p_with_various_args_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-check-face) (error nil))
  (condition-case nil (set-face-underline 'my-ul-check-face '(:color "red" :style wave) nil) (error nil))
  (list
   'underline-p-nil-frame (condition-case nil (face-underline-p 'my-ul-check-face nil t) (error 'no))
   'underline-p-cur-frame (condition-case nil (face-underline-p 'my-ul-check-face (selected-frame) t) (error 'no))
   'underline-p-inherit-t (condition-case nil (face-underline-p 'my-ul-check-face nil t) (error 'no))
   'underline-p-inherit-nil (condition-case nil (face-underline-p 'my-ul-check-face nil nil) (error 'no))
   'face-bold-p-default (condition-case nil (face-bold-p 'default nil t) (error 'no))
   'face-bold-p-bold (condition-case nil (face-bold-p 'bold nil t) (error 'no))
   'face-italic-p-default (condition-case nil (face-italic-p 'default nil t) (error 'no))
   'face-italic-p-italic (condition-case nil (face-italic-p 'italic nil t) (error 'no))
   'face-italic-p-ul (condition-case nil (face-italic-p 'my-ul-check-face nil t) (error 'no)))))"##,
    );
}

#[test]
fn ft_gamma_face_color_rgb_hex_roundtrip_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'red-roundtrip (let ((rgb (color-name-to-rgb "red")))
                    (list 'name-to-rgb rgb
                          'rgb-to-hex (apply 'color-rgb-to-hex (append rgb '(2)))))
   'blue-roundtrip (let ((rgb (color-name-to-rgb "blue")))
                     (list 'name-to-rgb rgb
                           'rgb-to-hex (apply 'color-rgb-to-hex (append rgb '(2)))))
   'green-roundtrip (let ((rgb (color-name-to-rgb "green")))
                      (list 'name-to-rgb rgb
                            'rgb-to-hex (apply 'color-rgb-to-hex (append rgb '(2)))))
   'black-roundtrip (let ((rgb (color-name-to-rgb "black")))
                      (list 'name-to-rgb rgb
                            'rgb-to-hex (apply 'color-rgb-to-hex (append rgb '(2)))))
   'white-roundtrip (let ((rgb (color-name-to-rgb "white")))
                      (list 'name-to-rgb rgb
                            'rgb-to-hex (apply 'color-rgb-to-hex (append rgb '(2)))))
   'hex-to-rgb (color-name-to-rgb "#FF8800")
   'hex-magenta (color-name-to-rgb "#FF00FF"))))"##,
    );
}

#[test]
fn ft_gamma_font_lock_default_fontify_buffer_vs_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-default-fontify-buffer-fbound (fboundp 'font-lock-default-fontify-buffer)
   'font-lock-default-fontify-region-fbound (fboundp 'font-lock-default-fontify-region)
   'font-lock-default-unfontify-buffer-fbound (fboundp 'font-lock-default-unfontify-buffer)
   'font-lock-default-unfontify-region-fbound (fboundp 'font-lock-default-unfontify-region)
   (condition-case nil
       (with-temp-buffer
         (fundamental-mode)
         (font-lock-mode 1)
         (insert "Test buffer for default fontify")
         (font-lock-default-fontify-buffer)
         (list 'fontified-1 (get-text-property 1 'fontified) 'fontified-10 (get-text-property 10 'fontified)))
     (error 'no-default-fontify))
   (condition-case nil
       (with-temp-buffer
         (fundamental-mode)
         (font-lock-mode 1)
         (insert "Test region for default fontify region")
         (font-lock-default-fontify-region 1 10 nil)
         (list 'fontified-1 (get-text-property 1 'fontified) 'fontified-15 (get-text-property 15 'fontified)))
     (error 'no-default-fontify-region)))))"##,
    );
}

#[test]
fn ft_gamma_face_text_property_front_sticky_backward_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Front sticky backward insertion test content buffer text")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 1 10 'front-sticky '(face))
    (put-text-property 10 20 'face 'italic)
    (put-text-property 10 20 'rear-nonsticky nil)
    (put-text-property 20 44 'face 'underline)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'front-sticky) (get-text-property pos 'rear-nonsticky))) '(1 5 10 12 15 20 30 40))
     ;; Insert BEFORE front-sticky boundary - face should propagate BACKWARD
     'after-insert-before (progn (goto-char 10) (insert "BACK") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(5 8 10 12 15 20 30)))
     ;; Insert at end of buffer
     'after-insert-end (progn (goto-char (point-max)) (insert "END") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(40 44 46 48))))))"##,
    );
}

#[test]
fn ft_gamma_face_with_all_face_listed_and_sorted_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'total-face-count (length (face-list))
   'sample-10-faces (seq-take (face-list) 10)
   'all-faces-facep (apply #'and (mapcar #'facep (seq-take (face-list) 20)))
   'default-in-list (member 'default (face-list))
   'bold-in-list (member 'bold (face-list))
   'italic-in-list (member 'italic (face-list))
   'sorted-list-head (seq-take (sort (copy-sequence (face-list)) #'string<) 10))
   'face-list-no-duplicates (= (length (face-list)) (length (delete-dups (copy-sequence (face-list))))))))"##,
    );
}

#[test]
fn ft_delta_face_with_two_distinct_non_face_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Two distinct non-face properties on buffer content text here")
    (put-text-property 1 25 'face 'bold)
    (put-text-property 1 25 'property-a 'value-a)
    (put-text-property 25 55 'face 'italic)
    (put-text-property 25 55 'property-b 'value-b)
    (put-text-property 25 55 'property-c 'value-c)
    (list
     'faces-and-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'property-a) (get-text-property pos 'property-b) (get-text-property pos 'property-c))) '(1 10 20 25 30 40 54))
     'text-props-at-1 (length (text-properties-at 1))
     'text-props-at-25 (length (text-properties-at 25))
     'text-props-at-54 (length (text-properties-at 54))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_delta_font_lock_keywords_only_flag_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'keywords-only-bound (boundp 'font-lock-keywords-only)
   'keywords-only-value (if (boundp 'font-lock-keywords-only) font-lock-keywords-only 'no-bound)
   (condition-case nil
       (with-temp-buffer
         (emacs-lisp-mode)
         (let ((font-lock-keywords-only t))
           (insert "(defun kw-only-test () t)\n")
           (font-lock-fontify-buffer)
           (list
            'defun-face-keywords (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
            't-face-keywords (save-excursion (goto-char (point-min)) (search-forward " t") (get-text-property (match-beginning 0) 'face)))))
     (error 'keywords-only-failed))
   (condition-case nil
       (with-temp-buffer
         (emacs-lisp-mode)
         (let ((font-lock-keywords-only nil))
           (insert "(defun kw-all-test () t)\n")
           (font-lock-fontify-buffer)
           (list
            'defun-face-all (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
            't-face-all (save-excursion (goto-char (point-min)) (search-forward " t") (get-text-property (match-beginning 0) 'face)))))
     (error 'all-failed)))))"##,
    );
}

#[test]
fn ft_delta_overlay_face_after_moving_region_partially_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30 35)))))
        (let ((v0 (funcall snap)))
          ;; Move partially (shrink from right)
          (move-overlay ov 6 15) (let ((v1 (funcall snap)))
          ;; Move partially (shrink from left)
          (move-overlay ov 10 15) (let ((v2 (funcall snap)))
          ;; Move partially (expand both)
          (move-overlay ov 5 25) (let ((v3 (funcall snap)))
          ;; Move completely to new region
          (move-overlay ov 25 35) (let ((v4 (funcall snap)))
          (delete-overlay ov)
          (list v0 v1 v2 v3 v4))))))))))"##,
    );
}

#[test]
fn ft_delta_face_add_text_properties_with_same_key_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Add text properties same key multiple times test content here")
    (add-text-properties 1 57 (list 'face 'bold 'key1 'first))
    (add-text-properties 10 40 (list 'face 'italic 'key1 'second))
    (add-text-properties 30 57 (list 'face 'underline 'key1 'third))
    (list
     'faces-across (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'key1))) '(1 5 10 20 30 40 50 56))
     'text-props-counts (mapcar (lambda (pos) (goto-char pos) (length (text-properties-at pos))) '(1 10 20 30 40 50))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_delta_face_set_face_background_with_various_colors_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-bg-colors-face) (error nil))
  (list
   'set-bg-red (condition-case nil (progn (set-face-background 'my-bg-colors-face "red" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-green (condition-case nil (progn (set-face-background 'my-bg-colors-face "green" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-blue (condition-case nil (progn (set-face-background 'my-bg-colors-face "blue" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-yellow (condition-case nil (progn (set-face-background 'my-bg-colors-face "yellow" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-white (condition-case nil (progn (set-face-background 'my-bg-colors-face "white" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-black (condition-case nil (progn (set-face-background 'my-bg-colors-face "black" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-hex (condition-case nil (progn (set-face-background 'my-bg-colors-face "#FF00FF" nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no))
   'set-bg-unspecified (condition-case nil (progn (set-face-background 'my-bg-colors-face 'unspecified nil) (face-background 'my-bg-colors-face nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_delta_face_font_lock_fontify_block_function_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun block-test (a b)\n  \"Docstring for block test.\"\n  (+ a b))\n")
    (list
     'before-fontify (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30))
     'fontify-block (condition-case nil
                        (progn
                          (font-lock-fontify-block 1)
                          (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30 40)))
                      (error 'no-block-fontify))
     'fontify-buffer (progn
                       (font-lock-fontify-buffer)
                       (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25 35 40))))))"##,
    );
}

#[test]
fn ft_delta_face_property_change_at_multiple_boundaries_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPP")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 3 5 'face 'italic)
    (put-text-property 5 7 'face 'underline)
    (put-text-property 7 9 'face '(:foreground "red"))
    (put-text-property 9 11 'face '(:background "yellow"))
    (list
     'faces-all (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 4 5 6 7 8 9 10))
     'prop-changes (let ((pos 1) (changes nil))
                     (while pos
                       (setq pos (next-single-property-change pos 'face nil 11))
                       (when pos (push (list pos (get-text-property pos 'face)) changes)))
                     (nreverse changes))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_delta_face_overlay_with_category_and_face_together_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with category and face together test content text here")
    (let ((ov1 (make-overlay 1 20)))
      (overlay-put ov1 'category 'cat-a)
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 10 30)))
      (overlay-put ov2 'category 'cat-b)
      (overlay-put ov2 'face '(:foreground "red" :weight bold))
      (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 25 45)))
      (overlay-put ov3 'category 'cat-c)
      (overlay-put ov3 'face '(:underline t :slant italic))
      (overlay-put ov3 'priority 15))
    (list
     'category-face (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 10 15 20 25 30 35 45 50 55))
     'overlay-categories (list (overlay-get ov1 'category) (overlay-get ov2 'category) (overlay-get ov3 'category))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))"##,
    );
}

#[test]
fn ft_epsilon_face_text_property_any_with_complex_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 11 21 'face '(:foreground "red"))
    (put-text-property 21 31 'face 'underline)
    (put-text-property 31 36 'face '(:background "yellow"))
    (list
     'find-bold (text-property-any 1 36 'face 'bold)
     'find-red (text-property-any 1 36 'face '(:foreground "red"))
     'find-underline (text-property-any 1 36 'face 'underline)
     'find-complex (text-property-any 1 36 'face '(:background "yellow"))
     'find-none (text-property-any 1 36 'face 'italic)
     'not-all-bold (text-property-not-all 1 36 'face 'bold)
     'single-prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 36)) '(1 11 21 31))
     'prev-prop-changes (mapcar (lambda (pos) (previous-single-property-change pos 'face nil 1)) '(11 21 31 36)))))"##,
    );
}

#[test]
fn ft_epsilon_font_lock_fontify_with_buffer_local_var_set_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (setq-local font-lock-verbose t)
    (insert "(defun verbose-test (x) (+ x 1))\n")
    (font-lock-ensure (point-min) (point-max))
    (list
     'font-lock-verbose-local font-lock-verbose
     'defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     'verbose-test-face (save-excursion (goto-char (point-min)) (search-forward "verbose-test") (get-text-property (match-beginning 0) 'face))
     'x-face (save-excursion (goto-char (point-min)) (search-forward " x") (get-text-property (match-beginning 0) 'face))
     'fontified-after (get-text-property 1 'fontified))
    (kill-local-variable 'font-lock-verbose))))"##,
    );
}

#[test]
fn ft_epsilon_face_overlay_modification_property_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (defvar my-ov-mod-list nil)
  (defun my-ov-collector (ov after beg end &optional len)
    (push (list 'modified after beg end len) my-ov-mod-list))
  (with-temp-buffer
    (insert "Overlay modification property change face test buffer text")
    (put-text-property 1 56 'face '(:foreground "blue"))
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'modification-hooks (list 'my-ov-collector)))
    (list
     'initial-mod-list (length my-ov-mod-list)
     'face-before-mod (get-char-property 20 'face)
     (progn (goto-char 25) (insert "INSERTED") (list 'after-insert-count (length my-ov-mod-list) 'face-after (get-char-property 25 'face)))
     (progn (delete-region 15 35) (list 'after-delete-count (length my-ov-mod-list) 'face-after-delete (get-char-property 15 'face)))
     'final-mod-list-len (length my-ov-mod-list)
     (progn (delete-overlay ov) (setq my-ov-mod-list nil) 'cleaned)))))"##,
    );
}

#[test]
fn ft_epsilon_face_set_attribute_foreground_unspecified_vs_nil_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-unspec-nil-face) (error nil))
  (list
   'initial-fg (condition-case nil (face-foreground 'my-unspec-nil-face nil 'default-on) (error 'no))
   'set-fg-red (condition-case nil (progn (set-face-foreground 'my-unspec-nil-face "red" nil) (face-foreground 'my-unspec-nil-face nil 'default-on)) (error 'no))
   'set-fg-unspecified (condition-case nil (progn (set-face-foreground 'my-unspec-nil-face 'unspecified nil) (face-foreground 'my-unspec-nil-face nil 'default-on)) (error 'no))
   'set-fg-nil (condition-case nil (progn (set-face-foreground 'my-unspec-nil-face nil nil) (face-foreground 'my-unspec-nil-face nil 'default-on)) (error 'no))
   'set-weight-bold (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :weight 'bold) (face-attribute 'my-unspec-nil-face :weight nil 'default-on)) (error 'no))
   'set-weight-unspecified (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :weight 'unspecified) (face-attribute 'my-unspec-nil-face :weight nil 'default-on)) (error 'no))
   'set-weight-nil (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :weight nil) (face-attribute 'my-unspec-nil-face :weight nil 'default-on)) (error 'no))
   'final-reset (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :foreground 'unspecified :weight 'unspecified) 'reset-done) (error 'no)))))"##,
    );
}

#[test]
fn ft_epsilon_face_font_lock_remove_all_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Remove ALL keywords from font-lock test buffer content text")
    (font-lock-add-keywords nil '(("\\<\\(Remove\\)\\>" 1 font-lock-warning-face t) ("\\<\\(ALL\\)\\>" 1 '(:foreground "red") t) ("\\<\\(keywords\\)\\>" 1 '(:foreground "blue") t) ("\\<\\(buffer\\)\\>" 1 '(:foreground "green") t)))
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("Remove" "ALL" "keywords" "buffer"))))
      ;; Remove ALL keywords
      (font-lock-remove-keywords nil '(("\\<\\(Remove\\)\\>" 1 font-lock-warning-face t) ("\\<\\(ALL\\)\\>" 1 '(:foreground "red") t) ("\\<\\(keywords\\)\\>" 1 '(:foreground "blue") t) ("\\<\\(buffer\\)\\>" 1 '(:foreground "green") t)))
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("Remove" "ALL" "keywords" "buffer"))))
        (list v0 v1)))))"##,
    );
}

#[test]
fn ft_epsilon_face_overlay_priority_change_and_face_recalculation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay priority recalculation face test buffer content text area")
    (let ((ov1 (make-overlay 1 30))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 15 45))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 30 55))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 30))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 15 20 30 40 50 55)))))
      (let ((v0 (funcall snap)))
        ;; Flip priorities
        (overlay-put ov1 'priority 100) (overlay-put ov2 'priority 50) (overlay-put ov3 'priority 1)
        (let ((v1 (funcall snap)))
          ;; Change faces
          (overlay-put ov1 'face '(:foreground "purple" :weight bold))
          (overlay-put ov2 'face '(:underline t :slant italic))
          (let ((v2 (funcall snap)))
            (mapc #'delete-overlay (overlays-in 1 55))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_epsilon_face_with_string_width_calculation_and_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "String width with face properties αβγδεζηθ")
    (put-text-property 1 32 'face 'bold)
    (list
     'string-width-bold (string-width (buffer-substring 1 32))
     'string-width-no-props (string-width (buffer-substring-no-properties 1 32))
     'string-bytes (string-bytes (buffer-string))
     'length (length (buffer-string))
     'face-at-1 (get-text-property 1 'face)
     'char-widths (mapcar (lambda (pos) (goto-char pos) (char-width (char-after pos))) '(1 2 3 30 31 32))
     'multibyte-p (multibyte-string-p (buffer-string))))))"##,
    );
}

#[test]
fn ft_epsilon_face_font_lock_inhibit_font_lock_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-mode-fbound (fboundp 'font-lock-mode)
   'inhibit-font-lock-bound (boundp 'inhibit-font-lock)
   'inhibit-font-lock-value (if (boundp 'inhibit-font-lock) inhibit-font-lock 'no-bound)
   'font-lock-after-change-function (if (boundp 'font-lock-after-change-function) (fboundp 'font-lock-after-change-function) 'no-bound)
   'font-lock-fontify-region-function (if (boundp 'font-lock-fontify-region-function) (fboundp 'font-lock-fontify-region-function) 'no-bound)
   'font-lock-unfontify-region-function (if (boundp 'font-lock-unfontify-region-function) (fboundp 'font-lock-unfontify-region-function) 'no-bound)
   'font-lock-fontify-buffer-function (if (boundp 'font-lock-fontify-buffer-function) (fboundp 'font-lock-fontify-buffer-function) 'no-bound)
   'font-lock-unfontify-buffer-function (if (boundp 'font-lock-unfontify-buffer-function) (fboundp 'font-lock-unfontify-buffer-function) 'no-bound))))"##,
    );
}

#[test]
fn ft_zeta3_face_with_force_face_attribute_inheritance_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-weight (face-attribute 'default :weight nil 'default-on)
   'default-fg (condition-case nil (face-attribute 'default :foreground nil 'default-on) (error 'no))
   'bold-weight (face-attribute 'bold :weight nil 'default-on)
   'bold-fg (condition-case nil (face-attribute 'bold :foreground nil 'default-on) (error 'no))
   'bold-slant (face-attribute 'bold :slant nil 'default-on)
   'italic-slant (face-attribute 'italic :slant nil 'default-on)
   'face-equal-default-default (condition-case nil (face-equal 'default 'default) (error 'no))
   'face-differs-bold (face-differs-from-default-p 'bold)
   'face-differs-italic (face-differs-from-default-p 'italic)
   'face-differs-underline (condition-case nil (face-differs-from-default-p 'underline) (error 'no))
   'face-differs-bold-italic (condition-case nil (face-differs-from-default-p 'bold-italic) (error 'no))
   'face-differs-fringe (condition-case nil (face-differs-from-default-p 'fringe) (error 'no)))))"##,
    );
}

#[test]
fn ft_zeta3_font_lock_fontify_but_no_syntactic_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (c-mode)
    (insert "int main() { return 0; }\n")
    (condition-case nil
        (progn
          (font-lock-fontify-buffer)
          (list
           'int-face (save-excursion (goto-char (point-min)) (search-forward "int") (get-text-property (match-beginning 0) 'face))
           'main-face (save-excursion (goto-char (point-min)) (search-forward "main") (get-text-property (match-beginning 0) 'face))
           'return-face (save-excursion (goto-char (point-min)) (search-forward "return") (get-text-property (match-beginning 0) 'face))
           '0-face (save-excursion (goto-char (point-min)) (search-forward "0") (get-text-property (match-beginning 0) 'face))
           'fontified-all (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15 20))))
      (error 'c-mode-fontify-failed)))))"##,
    );
}

#[test]
fn ft_zeta3_face_overlay_with_line_prefix_and_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay line prefix face combined test content data area here now")
    (let ((ov (make-overlay 1 58)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'line-prefix (propertize "| " 'face '(:foreground "red" :weight bold)))
      (overlay-put ov 'wrap-prefix (propertize "> " 'face '(:foreground "blue" :slant italic))))
    (list
     'overlay-face (overlay-get ov 'face)
     'line-prefix (overlay-get ov 'line-prefix)
     'line-prefix-face (get-text-property 0 (overlay-get ov 'line-prefix))
     'wrap-prefix (overlay-get ov 'wrap-prefix)
     'wrap-prefix-face (get-text-property 0 (overlay-get ov 'wrap-prefix))
     'current-face (get-char-property 1 'face)
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_zeta3_face_set_strike_through_various_options_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-strike-face) (error nil))
  (list
   'set-strike-t (condition-case nil (progn (set-face-attribute 'my-strike-face nil :strike-through t) (face-attribute 'my-strike-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-nil (condition-case nil (progn (set-face-attribute 'my-strike-face nil :strike-through nil) (face-attribute 'my-strike-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-color (condition-case nil (progn (set-face-attribute 'my-strike-face nil :strike-through '(:color "red")) (face-attribute 'my-strike-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-off (condition-case nil (progn (set-face-attribute 'my-strike-face nil :strike-through 'unspecified) (face-attribute 'my-strike-face :strike-through nil 'default-on)) (error 'no))
   'default-strike (condition-case nil (face-attribute 'default :strike-through nil 'default-on) (error 'no))
   'bold-strike (condition-case nil (face-attribute 'bold :strike-through nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_zeta3_face_text_property_interval_collision_insert_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZ")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 8 11 13 15)))))
      (let ((v0 (funcall snap)))
        ;; Insert at exact boundary (should NOT merge)
        (goto-char 6) (insert "QQ")
        (let ((v1 (funcall snap)))
          ;; Insert within interval (should split)
          (goto-char 10) (insert "RR")
          (let ((v2 (funcall snap)))
            ;; Delete overlapping boundary
            (delete-region 8 12)
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_zeta3_face_font_lock_mode_without_font_lock_support() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (setq-local font-lock-support-mode nil)
    (font-lock-mode 1)
    (insert "Font lock with no support mode test buffer content text")
    (font-lock-fontify-buffer)
    (list
     'font-lock-mode font-lock-mode
     'font-lock-support-mode (if (boundp 'font-lock-support-mode) font-lock-support-mode 'no-bound)
     'fontified-region (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30 40))
     'face-at-1 (get-text-property 1 'face)
     'face-at-20 (get-text-property 20 'face)
     (progn (kill-local-variable 'font-lock-support-mode) (font-lock-mode -1) 'cleaned))))"##,
    );
}

#[test]
fn ft_zeta3_face_overlay_before_string_face_vs_overlay_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before string face vs overlay face test content data text here end")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "[[BEFORE]]" 'face '(:foreground "red" :weight bold))))
    (list
     'overlay-face (overlay-get ov 'face)
     'before-string-face (get-text-property 0 (overlay-get ov 'before-string))
     'before-string (overlay-get ov 'before-string)
     'at-overlay-start (get-char-property 15 'face)
     'at-overlay-middle (get-char-property 25 'face)
     'at-overlay-before (get-char-property 14 'face)
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_zeta3_face_put_all_text_properties_and_read_them_back_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "All text properties read back test buffer content text here end")
    (add-text-properties 1 57 (list 'face 'bold 'key1 'val1 'key2 'val2 'key3 'val3 'key4 'val4))
    (add-text-properties 20 40 (list 'face 'italic 'key5 'val5 'key6 'val6))
    (list
     'face-at-1 (get-text-property 1 'face)
     'face-at-20 (get-text-property 20 'face)
     'face-at-40 (get-text-property 40 'face)
     'face-at-56 (get-text-property 56 'face)
     'all-keys-at-1 (let ((props (text-properties-at 1)) (keys nil) (i 0))
                     (while (< i (length props))
                       (push (nth i props) keys)
                       (setq i (+ i 2)))
                     (nreverse keys))
     'all-keys-at-20 (let ((props (text-properties-at 20)) (keys nil) (i 0))
                       (while (< i (length props))
                         (push (nth i props) keys)
                         (setq i (+ i 2)))
                       (nreverse keys))
     'all-keys-at-56 (let ((props (text-properties-at 56)) (keys nil) (i 0))
                       (while (< i (length props))
                         (push (nth i props) keys)
                         (setq i (+ i 2)))
                       (nreverse keys)))))"##,
    );
}

#[test]
fn ft_eta_face_text_property_value_inheritance_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAABBBBBBBBBBCCCCCCCCCCDDDDDDDDDDEEEEEEEEEE")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 1 11 'base-prop 'base)
    (put-text-property 11 21 'face 'italic)
    (put-text-property 11 21 'mid-prop 'mid)
    (put-text-property 21 31 'face 'underline)
    (put-text-property 21 31 'leaf-prop 'leaf)
    (put-text-property 31 41 'face '(:foreground "red"))
    (put-text-property 31 41 'end-prop 'end)
    (put-text-property 41 51 'face '(:background "yellow"))
    (list
     'faces-props (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'base-prop) (get-text-property pos 'mid-prop) (get-text-property pos 'leaf-prop) (get-text-property pos 'end-prop))) '(1 5 11 15 21 25 31 35 41 45 50))
     'interval-count (length (object-intervals (current-buffer)))
     'prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 51)) '(1 11 21 31 41)))))"##,
    );
}

#[test]
fn ft_eta_font_lock_prepend_append_overwrite_resolution_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "RESOLUTION order test: PREPEND vs APPEND vs OVERWRITE flags here end")
    ;; Add with prepend first (lowest priority)
    (font-lock-add-keywords nil '(("\\<\\(RESOLUTION\\)\\>" 1 '(:foreground "blue") prepend)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "RESOLUTION") (get-text-property (match-beginning 0) 'face))))
      ;; Add with append (medium)
      (font-lock-add-keywords nil '(("\\<\\(RESOLUTION\\)\\>" 1 '(:foreground "green") append)))
      (font-lock-fontify-buffer)
      (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "RESOLUTION") (get-text-property (match-beginning 0) 'face))))
        ;; Add with overwrite (highest)
        (font-lock-add-keywords nil '(("\\<\\(RESOLUTION\\)\\>" 1 '(:foreground "red" :weight bold) overwrite)))
        (font-lock-fontify-buffer)
        (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "RESOLUTION") (get-text-property (match-beginning 0) 'face))))
          ;; Check other words
          (let ((order-face (save-excursion (goto-char (point-min)) (search-forward "order") (get-text-property (match-beginning 0) 'face)))
                (flags-face (save-excursion (goto-char (point-min)) (search-forward "flags") (get-text-property (match-beginning 0) 'face))))
            (list v0 v1 v2 order-face flags-face)))))))"##,
    );
}

#[test]
fn ft_eta_face_overlay_string_with_face_and_display_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string display properties face combined test text data")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "<<<" 'face '(:foreground "red")))
      (overlay-put ov 'after-string (propertize ">>>" 'face '(:foreground "blue")))
      (overlay-put ov 'display ""))
    (list
     'overlay-face (overlay-get ov 'face)
     'before-face (get-text-property 0 (overlay-get ov 'before-string))
     'after-face (get-text-property 0 (overlay-get ov 'after-string))
     'char-prop-at-start (get-char-property 10 'face)
     'char-prop-at-end (get-char-property 29 'face)
     'char-prop-at-middle (get-char-property 20 'face)
     (progn (delete-overlay ov) 'cleaned))))"##,
    );
}

#[test]
fn ft_eta_face_set_attribute_inherit_multiple_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-multi-inherit-face) (error nil))
  (condition-case nil (set-face-attribute 'my-multi-inherit-face nil :inherit '(bold italic)) (error nil))
  (list
   'inherited-weight (face-attribute 'my-multi-inherit-face :weight nil 'default-on)
   'inherited-slant (face-attribute 'my-multi-inherit-face :slant nil 'default-on)
   'override-weight (condition-case nil (progn (set-face-attribute 'my-multi-inherit-face nil :weight 'ultra-light) (face-attribute 'my-multi-inherit-face :weight nil 'default-on)) (error 'no))
   'override-slant (condition-case nil (progn (set-face-attribute 'my-multi-inherit-face nil :slant 'oblique) (face-attribute 'my-multi-inherit-face :slant nil 'default-on)) (error 'no))
   'remove-inherit (condition-case nil (progn (set-face-attribute 'my-multi-inherit-face nil :inherit nil) 'cleared) (error 'no))
   'weight-after-clear (face-attribute 'my-multi-inherit-face :weight nil 'default-on)
   'slant-after-clear (face-attribute 'my-multi-inherit-face :slant nil 'default-on))))"##,
    );
}

#[test]
fn ft_eta_face_text_property_next_single_change_with_object_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:foreground "blue"))
    (put-text-property 31 36 'face '(:background "cyan"))
    (list
     'next-change-default (next-single-property-change 1 'face)
     'next-change-object (next-single-property-change 1 'face nil (current-buffer))
     'next-change-limit (next-single-property-change 1 'face nil 15)
     'prev-change-default (previous-single-property-change 36 'face)
     'prev-change-object (previous-single-property-change 36 'face nil (current-buffer))
     'prev-change-limit (previous-single-property-change 36 'face nil (point-min)))
     'text-prop-any-limit (text-property-any 1 20 'face 'italic)
     'text-prop-not-all-limit (text-property-not-all 1 20 'face 'bold)))))"##,
    );
}

#[test]
fn ft_eta_font_lock_flush_vs_refresh_defaults_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-flush-fbound (fboundp 'font-lock-flush)
   'font-lock-refresh-defaults-fbound (fboundp 'font-lock-refresh-defaults)
   (condition-case nil
       (progn (font-lock-flush) 'flushed)
     (error 'no-flush))
   (condition-case nil
       (progn (font-lock-refresh-defaults) 'refreshed)
     (error 'no-refresh))
   'font-lock-fontified-bound (boundp 'font-lock-fontified)
   'font-lock-fontified-value (if (boundp 'font-lock-fontified) font-lock-fontified 'no-bound)
   'font-lock-set-defaults-fbound (fboundp 'font-lock-set-defaults)
   (condition-case nil
       (with-temp-buffer (emacs-lisp-mode) (font-lock-set-defaults) 'set)
     (error 'no-set)))))"##,
    );
}

#[test]
fn ft_eta_face_overlay_category_inheritance_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay category chain face test buffer content data text here end final")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'category 'cat-root) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'category 'cat-mid) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 15))
    (let ((ov3 (make-overlay 20 40))) (overlay-put ov3 'category 'cat-leaf) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 20))
    (let ((ov4 (make-overlay 35 60))) (overlay-put ov4 'category 'cat-last) (overlay-put ov4 'face '(:background "yellow")) (overlay-put ov4 'priority 5))
    (list
     'category-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (mapcar (lambda (ov) (overlay-get ov 'category)) (overlays-at pos)))) '(1 10 15 20 25 30 35 40 50 59))
     'all-categories (mapcar (lambda (ov) (overlay-get ov 'category)) (list ov1 ov2 ov3 ov4))
     (progn (mapc #'delete-overlay (overlays-in 1 60)) 'cleaned))))"##,
    );
}

#[test]
fn ft_eta_face_color_dark_light_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'color-dark-p-black (condition-case nil (color-dark-p "#000000") (error 'no))
   'color-dark-p-white (condition-case nil (color-dark-p "#FFFFFF") (error 'no))
   'color-dark-p-red (condition-case nil (color-dark-p "#FF0000") (error 'no))
   'color-dark-p-yellow (condition-case nil (color-dark-p "#FFFF00") (error 'no))
   'color-dark-p-blue (condition-case nil (color-dark-p "#0000FF") (error 'no))
   'color-dark-p-gray (condition-case nil (color-dark-p "#808080") (error 'no))
   'color-light-name-p-white (condition-case nil (color-light-name-p "white") (error 'no))
   'color-light-name-p-black (condition-case nil (color-light-name-p "black") (error 'no))
   'color-light-name-p-gray (condition-case nil (color-light-name-p "gray") (error 'no))
   'color-light-name-p-snow (condition-case nil (color-light-name-p "snow") (error 'no))
   'color-light-name-p-midnight (condition-case nil (color-light-name-p "midnight blue") (error 'no))
   (condition-case nil (color-complement "#FF0000") (error 'no))
   (condition-case nil (color-complement "#0000FF") (error 'no)))))"##,
    );
}

#[test]
fn ft_theta_face_overlay_with_multiple_properties_and_priorities() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multiple overlay properties and priorities face test data content")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 100)
      (overlay-put ov 'help-echo "Test overlay")
      (overlay-put ov 'evaporate t)
      (overlay-put ov 'category 'my-cat)
      (list
       'face (overlay-get ov 'face)
       'priority (overlay-get ov 'priority)
       'help-echo (overlay-get ov 'help-echo)
       'evaporate (overlay-get ov 'evaporate)
       'category (overlay-get ov 'category)
       'props-count (length (overlay-properties ov))
       'all-keys (let ((props (overlay-properties ov)) (keys nil) (i 0))
                   (while (< i (length props))
                     (push (nth i props) keys)
                     (setq i (+ i 2)))
                   (nreverse keys))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_theta_face_set_attribute_distant_foreground_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-dist-fg-face) (error nil))
  (list
   'set-dist-fg-red (condition-case nil (progn (set-face-attribute 'my-dist-fg-face nil :distant-foreground "red") 'ok) (error 'no))
   'get-dist-fg (condition-case nil (face-attribute 'my-dist-fg-face :distant-foreground nil 'default-on) (error 'no))
   'set-dist-fg-blue (condition-case nil (progn (set-face-attribute 'my-dist-fg-face nil :distant-foreground "blue") 'ok) (error 'no))
   'get-dist-fg2 (condition-case nil (face-attribute 'my-dist-fg-face :distant-foreground nil 'default-on) (error 'no))
   'set-dist-fg-unspec (condition-case nil (progn (set-face-attribute 'my-dist-fg-face nil :distant-foreground 'unspecified) 'ok) (error 'no))
   'get-dist-fg-after-unspec (condition-case nil (face-attribute 'my-dist-fg-face :distant-foreground nil 'default-on) (error 'no))
   'default-dist-fg (condition-case nil (face-attribute 'default :distant-foreground nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_theta_face_font_lock_mode_check_after_buffer_erase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Erase test\nBody erase.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (list 'mode font-lock-mode 'face (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face)))))
        ;; Erase buffer
        (erase-buffer)
        (let ((v1 (list 'mode-after-erase font-lock-mode 'empty-face (get-text-property 1 'face))))
          ;; Re-insert and fontify
          (insert "* DONE After erase\nBody after.\n\n")
          (font-lock-ensure (point-min) (point-max))
          (let ((v2 (list 'mode-after-refill font-lock-mode 'face-after (save-excursion (goto-char (point-min)) (search-forward "DONE") (get-text-property (match-beginning 0) 'face)))))
            (list v0 v1 v2))))))))"##,
    );
}

#[test]
fn ft_theta_face_property_search_with_nil_value_large_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert (make-string 100 ?X))
    (put-text-property 1 50 'face 'bold)
    (put-text-property 50 101 'face nil)
    (list
     'find-bold (text-property-any 1 101 'face 'bold)
     'find-nil (text-property-any 1 101 'face nil)
     'not-all-bold (text-property-not-all 1 101 'face 'bold)
     'next-nil (next-single-property-change 1 'face nil 101)
     'prev-bold (previous-single-property-change 101 'face nil 1)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_theta_face_add_face_text_property_incrementally_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Incremental add face text property test buffer content text data")
    ;; Start with base face
    (add-face-text-property 1 55 '(:foreground "blue"))
    (add-face-text-property 1 55 '(:weight bold))
    (add-face-text-property 1 30 '(:slant italic))
    (add-face-text-property 25 55 '(:underline t))
    (add-face-text-property 40 55 '(:background "yellow"))
    (list
     'step1 (get-text-property 1 'face)
     'step2 (get-text-property 15 'face)
     'step3 (get-text-property 28 'face)
     'step4 (get-text-property 45 'face)
     'step5 (get-text-property 54 'face)
     'facep-all (mapcar (lambda (pos) (goto-char pos) (facep (get-text-property pos 'face))) '(1 15 28 45 54))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_theta_face_font_lock_fontify_syntactically_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; comment line\n(defun test () 42)\n")
    (font-lock-fontify-syntactically (point-min) (point-max) nil)
    (list
     'comment-face (save-excursion (goto-char (point-min)) (search-forward "comment") (get-text-property (match-beginning 0) 'face))
     'comment-fontified (save-excursion (goto-char (point-min)) (search-forward "comment") (get-text-property (match-beginning 0) 'fontified))
     'defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     'test-face (save-excursion (goto-char (point-min)) (search-forward "test") (get-text-property (match-beginning 0) 'face))
     '42-face (save-excursion (goto-char (point-min)) (search-forward "42") (get-text-property (match-beginning 0) 'face))
     'fontified-after-syn (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 15 20 25 30)))))"##,
    );
}

#[test]
fn ft_theta_face_overlay_properties_plist_length_stress_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay plist length stress test face buffer content text area")
    (let ((ov (make-overlay 1 10)))
      (overlay-put ov 'prop1 'val1)
      (overlay-put ov 'prop2 'val2)
      (overlay-put ov 'prop3 'val3)
      (overlay-put ov 'prop4 'val4)
      (overlay-put ov 'prop5 'val5)
      (overlay-put ov 'prop6 'val6)
      (overlay-put ov 'prop7 'val7)
      (overlay-put ov 'prop8 'val8)
      (overlay-put ov 'prop9 'val9)
      (overlay-put ov 'prop10 'val10)
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'props-count (length (overlay-properties ov))
       'face-get (overlay-get ov 'face)
       'prop1-get (overlay-get ov 'prop1)
       'prop5-get (overlay-get ov 'prop5)
       'prop10-get (overlay-get ov 'prop10)
       'all-keys (let ((props (overlay-properties ov)) (keys nil) (i 0))
                   (while (< i (length props))
                     (push (nth i props) keys)
                     (setq i (+ i 2)))
                   (sort (nreverse keys) #'string<))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_theta_face_set_face_font_via_spec_vs_string_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-font-spec-face) (error nil))
  (list
   'set-font-by-spec (condition-case nil (progn (set-face-font 'my-font-spec-face (font-spec :family "Monospace" :size 12) nil) 'set) (error 'no))
   'get-font-after-spec (condition-case nil (face-font 'my-font-spec-face nil) (error 'no))
   'set-font-by-string (condition-case nil (progn (set-face-font 'my-font-spec-face "Monospace-12" nil) 'set) (error 'no))
   'get-font-after-string (condition-case nil (face-font 'my-font-spec-face nil) (error 'no))
   'set-font-by-xlfd (condition-case nil (let* ((font (face-font 'default nil)) (xlfd (if (fontp font) (font-xlfd-name font) "none"))) (list 'xlfd xlfd)) (error 'no-xlfd))
   'reset-font (condition-case nil (progn (set-face-font 'my-font-spec-face 'unspecified nil) 'reset) (error 'no)))))"##,
    );
}

#[test]
fn ft_iota_face_invisible_property_affects_face_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Visible HIDDEN Visible HIDDEN Visible")
    (put-text-property 1 9 'face 'bold)
    (put-text-property 9 15 'face 'italic)
    (put-text-property 9 15 'invisible t)
    (put-text-property 15 23 'face 'underline)
    (put-text-property 23 29 'face '(:foreground "red") :invisible t)
    (put-text-property 29 36 'face '(:background "yellow"))
    (list
     'faces-with-invisible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible) (invisible-p pos))) '(1 5 9 12 15 20 23 26 29 33))
     'next-visible-change (next-single-property-change 1 'invisible)
     'remove-invisible (progn (remove-text-properties 9 15 '(invisible nil)) (remove-text-properties 23 29 '(invisible nil)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 9 12 15 23 26 29)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_iota_font_lock_fontify_with_char_syntax_table_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun st-test () 42)\n")
    (font-lock-fontify-buffer)
    (list
     'defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     'st-test-face (save-excursion (goto-char (point-min)) (search-forward "st-test") (get-text-property (match-beginning 0) 'face))
     'paren-face-left (save-excursion (goto-char (point-min)) (search-forward "(") (get-text-property (match-beginning 0) 'face))
     'paren-face-right (save-excursion (goto-char (point-min)) (search-forward ")") (get-text-property (match-beginning 0) 'face))
     'syntax-table-before (syntax-table)
     'fontified-after (get-text-property 1 'fontified))))"##,
    );
}

#[test]
fn ft_iota_face_overlay_with_zero_length_text_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Zero overlay text test")
    (let ((ov (make-overlay 5 5)))
      (overlay-put ov 'face '(:background "red")))
    (let ((ov2 (make-overlay 21 21)))
      (overlay-put ov2 'face '(:background "blue")))
    (list
     'before-insert (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 4 5 6 10 19 20 21 22))
     'after-insert-at-zero (progn (goto-char 5) (insert "INSERTED") (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 4 5 8 12 16 20 22 25 30)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))"##,
    );
}

#[test]
fn ft_iota_face_set_attribute_slant_italic_oblique_normal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-slant-cycle-face) (error nil))
  (list
   'set-slant-normal (condition-case nil (progn (set-face-attribute 'my-slant-cycle-face nil :slant 'normal) (face-attribute 'my-slant-cycle-face :slant nil 'default-on)) (error 'no))
   'set-slant-italic (condition-case nil (progn (set-face-attribute 'my-slant-cycle-face nil :slant 'italic) (face-attribute 'my-slant-cycle-face :slant nil 'default-on)) (error 'no))
   'set-slant-oblique (condition-case nil (progn (set-face-attribute 'my-slant-cycle-face nil :slant 'oblique) (face-attribute 'my-slant-cycle-face :slant nil 'default-on)) (error 'no))
   'set-slant-normal-again (condition-case nil (progn (set-face-attribute 'my-slant-cycle-face nil :slant 'normal) (face-attribute 'my-slant-cycle-face :slant nil 'default-on)) (error 'no))
   'set-slant-unspecified (condition-case nil (progn (set-face-attribute 'my-slant-cycle-face nil :slant 'unspecified) (face-attribute 'my-slant-cycle-face :slant nil 'default-on)) (error 'no))
   'set-slant-nil (condition-case nil (progn (set-face-attribute 'my-slant-cycle-face nil :slant nil) (face-attribute 'my-slant-cycle-face :slant nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_iota_face_text_property_next_change_fast_path_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "11111222223333344444555556666677777888889999900000")
    (dotimes (i 10)
      (put-text-property (1+ (* i 5)) (1+ (* (1+ i) 5)) 'face
                         (nth (mod i 4) '(bold italic underline (:foreground "red")))))
    (list
     'all-next-changes (let ((pos 1) (result nil))
                         (while pos
                           (setq pos (next-single-property-change pos 'face nil 51))
                           (when pos (push pos result)))
                         (nreverse result))
     'all-prev-changes (let ((pos 51) (result nil))
                         (while pos
                           (setq pos (previous-single-property-change pos 'face nil 1))
                           (when pos (push pos result)))
                         (nreverse result))
     'interval-count (length (object-intervals (current-buffer)))
     'spot-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 6 11 16 21 26 31 36 41 46 50)))))"##,
    );
}

#[test]
fn ft_iota_font_lock_set_defaults_multiple_times_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Set defaults multiple times font lock test buffer data")
    (list
     'mode-before font-lock-mode
     'set-defaults-1 (condition-case nil (progn (font-lock-set-defaults) 'ok) (error 'no))
     'set-defaults-2 (condition-case nil (progn (font-lock-set-defaults) 'ok) (error 'no))
     'set-defaults-3 (condition-case nil (progn (font-lock-set-defaults) 'ok) (error 'no))
     'fontify-after (progn (font-lock-fontify-buffer) (get-text-property 1 'fontified))
     'mode-after font-lock-mode))))"##,
    );
}

#[test]
fn ft_iota_face_overlay_with_category_face_inherit_combination() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay category face inherit combination test content text data here end")
    (let ((ov1 (make-overlay 1 20)))
      (overlay-put ov1 'category 'cat-type-a)
      (overlay-put ov1 'face '(:background "yellow" :inherit bold))
      (overlay-put ov1 'priority 100))
    (let ((ov2 (make-overlay 15 40)))
      (overlay-put ov2 'category 'cat-type-b)
      (overlay-put ov2 'face '(:foreground "red" :inherit italic :weight bold))
      (overlay-put ov2 'priority 50))
    (list
     'cat-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 10 15 20 25 30 35 40 50 60))
     'ov1-face (overlay-get ov1 'face)
     'ov2-face (overlay-get ov2 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 60)) 'cleaned))))"##,
    );
}

#[test]
fn ft_iota_face_with_face_list_dedup_and_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'total-faces (length (face-list))
   'deduped-count (length (delete-dups (copy-sequence (face-list))))
   'all-facep (apply #'and (mapcar #'facep (seq-take (face-list) 30)))
   'sample-faces (seq-take (sort (copy-sequence (face-list)) #'string<) 15)
   'has-default (member 'default (face-list))
   'has-bold (member 'bold (face-list))
   'has-italic (member 'italic (face-list))
   'has-bold-italic (member 'bold-italic (face-list))
   'has-underline (member 'underline (face-list))
   'has-fringe (member 'fringe (face-list))
   'has-scroll-bar (member 'scroll-bar (face-list))
   'has-tool-bar (if (member 'tool-bar (face-list)) 'present 'absent))))"##,
    );
}

#[test]
fn ft_kappa_face_last_char_property_survival_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "LAST")
    (put-text-property 1 5 'face 'bold)
    (list
     'before-insert (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 4 5))
     'after-insert-end (progn (goto-char 5) (insert " INSERTION") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 4 5 6 10 14)))
     'after-insert-beginning (progn (goto-char 1) (insert "START ") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 6 7 8 12 16)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_kappa_font_lock_fontify_keyword_with_special_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Special $chars *test* with ++font-lock++ and ==highlight==")
    (font-lock-add-keywords nil
                            '(("\\*\\*\\*" 0 font-lock-warning-face t)
                              ("\\+\\+[^+]\\+\\+\\+" 0 '(:foreground "red") t)))
    (font-lock-fontify-buffer)
    (mapcar (lambda (needle)
              (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face))))
            '("*test*" "++font-lock++"))))"##,
    );
}

#[test]
fn ft_kappa_face_set_attribute_with_integer_weight_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'weight-thin (face-attribute 'default :weight nil 'default-on)
   'weight-ultra-light (if (member 'ultra-light (font-weight-table)) 'available 'not-available)
   'weight-light (if (member 'light (font-weight-table)) 'available 'not-available)
   'weight-normal (if (member 'normal (font-weight-table)) 'available 'not-available)
   'weight-regular (if (member 'regular (font-weight-table)) 'available 'not-available)
   'weight-medium (if (member 'medium (font-weight-table)) 'available 'not-available)
   'weight-semi-bold (if (member 'semi-bold (font-weight-table)) 'available 'not-available)
   'weight-bold (if (member 'bold (font-weight-table)) 'available 'not-available)
   'weight-extra-bold (if (member 'extra-bold (font-weight-table)) 'available 'not-available)
   'weight-heavy (if (member 'heavy (font-weight-table)) 'available 'not-available)
   'weight-ultra-heavy (if (member 'ultra-heavy (font-weight-table)) 'available 'not-available)
   'font-weight-table-full (if (boundp 'font-weight-table) font-weight-table 'no-table))))"##,
    );
}

#[test]
fn ft_kappa_face_object_intervals_with_multiple_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXYYYZZZWWW")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 7 'face 'italic)
    (put-text-property 7 10 'face 'underline)
    (put-text-property 10 13 'face '(:foreground "red"))
    (put-text-property 1 13 'shared-prop 'shared-value)
    (list
     'interval-count (length (object-intervals (current-buffer)))
     'interval-objects (mapcar (lambda (obj) (list (overlay-start obj) (overlay-end obj))) (object-intervals (current-buffer)))
     'spot-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'shared-prop))) '(1 3 5 8 10 12)))))"##,
    );
}

#[test]
fn ft_kappa_face_font_lock_ensure_with_no_font_lock_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    ;; Don't enable font-lock-mode
    (insert "Font lock ensure without font lock mode enabled test")
    (list
     'font-lock-mode font-lock-mode
     'before-ensure-face (get-text-property 1 'face)
     'after-ensure (condition-case nil (progn (font-lock-ensure (point-min) (point-max)) 'ensured) (error 'ensure-failed))
     'after-ensure-mode font-lock-mode
     'after-ensure-face (get-text-property 1 'face)
     'after-ensure-fontified (get-text-property 1 'fontified))))"##,
    );
}

#[test]
fn ft_kappa_face_text_property_remove_in_middle_of_interval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAAAAAABBBBBBBBBBBBBBCCCCCCCCCCCCCC")
    (put-text-property 1 17 'face 'bold)
    (put-text-property 17 33 'face 'italic)
    (put-text-property 33 49 'face 'underline)
    ;; Remove face from middle of bold region
    (remove-text-properties 5 12 '(face nil))
    ;; Remove face from middle of italic region
    (remove-text-properties 22 28 '(face nil))
    (list
     'face-gaps (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 8 12 17 22 25 28 33 40 48))
     'find-bold (text-property-any 1 49 'face 'bold)
     'find-italic (text-property-any 1 49 'face 'italic)
     'find-underline (text-property-any 1 49 'face 'underline)
     'find-nil (text-property-any 1 49 'face nil)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_kappa_face_set_face_attribute_box_styles_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-box-styles-face) (error nil))
  (list
   'box-simple (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box t) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-width (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:line-width 3)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-released (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:style released-button)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-pressed (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:style pressed-button :color "red")) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-flat (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:style flat-button)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-none (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box nil) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_kappa_face_font_lock_remove_nonexistent_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'remove-nonexistent
   (condition-case nil
       (progn
         (font-lock-remove-keywords nil '(("\\<\\(nonexistent-kw\\)\\>" 1 font-lock-warning-face t)))
         'removed-silently)
     (error 'remove-error))
   'remove-valid-with-nonexistent
   (condition-case nil
       (progn
         (font-lock-add-keywords nil '(("\\<\\(VALID-KW\\)\\>" 1 '(:foreground "red") t)))
         (font-lock-remove-keywords nil
                                    '(("\\<\\(VALID-KW\\)\\>" 1 '(:foreground "red") t)
                                      ("\\<\\(nonexistent-kw2\\)\\>" 1 font-lock-warning-face t)))
         'mixed-removed)
     (error 'mixed-error)))))"##,
    );
}

#[test]
fn ft_lambda_face_display_table_set_then_clear_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Display table set and clear face test buffer content")
    (put-text-property 1 47 'face '(:foreground "blue"))
    (let ((dt (make-display-table)))
      (condition-case nil
          (progn
            (set-display-table-slot dt 'selective-display (vector (make-glyph-code ?- 'highlight)))
            (list
             'table-created 'ok
             'face-still-there (get-text-property 1 'face)
             'slot-0 (display-table-slot dt 0)
             'slot-selective (display-table-slot dt 'selective-display)))
        (error (list 'table-error (get-text-property 1 'face)))))))"##,
    );
}

#[test]
fn ft_lambda_face_font_lock_fontify_by_chunks_incremental() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun chunk1 () 1)\n(defun chunk2 () 2)\n(defun chunk3 () 3)\n")
    (list
     'fontify-chunk1 (progn (font-lock-fontify-region 1 17) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 17 18)))
     'fontify-chunk2 (progn (font-lock-fontify-region 18 34) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 18 25 34 35)))
     'fontify-chunk3 (progn (font-lock-fontify-region 35 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 35 40 47)))
     'all-faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 30 38 45))))))"##,
    );
}

#[test]
fn ft_lambda_face_text_property_sticky_after_multiple_inserts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (put-text-property 1 6 'face 'bold :front-sticky t :rear-nonsticky nil)
    (put-text-property 6 11 'face 'italic :front-sticky nil :rear-nonsticky '(face))
    (put-text-property 11 16 'face 'underline :front-sticky t)
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 8 11 13 15)))))
      (let ((v0 (funcall snap)))
        (goto-char 6) (insert "X")
        (goto-char 11) (insert "Y")
        (goto-char 16) (insert "Z")
        (goto-char 1) (insert "P")
        (let ((v1 (funcall snap)))
          (list v0 v1))))))"##,
    );
}

#[test]
fn ft_lambda_face_set_attribute_width_variants_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-width-face) (error nil))
  (list
   'set-width-ultra-condensed (condition-case nil (progn (set-face-attribute 'my-width-face nil :width 'ultra-condensed) (face-attribute 'my-width-face :width nil 'default-on)) (error 'no))
   'set-width-condensed (condition-case nil (progn (set-face-attribute 'my-width-face nil :width 'condensed) (face-attribute 'my-width-face :width nil 'default-on)) (error 'no))
   'set-width-normal (condition-case nil (progn (set-face-attribute 'my-width-face nil :width 'normal) (face-attribute 'my-width-face :width nil 'default-on)) (error 'no))
   'set-width-expanded (condition-case nil (progn (set-face-attribute 'my-width-face nil :width 'expanded) (face-attribute 'my-width-face :width nil 'default-on)) (error 'no))
   'set-width-unspec (condition-case nil (progn (set-face-attribute 'my-width-face nil :width 'unspecified) (face-attribute 'my-width-face :width nil 'default-on)) (error 'no))
   'default-width (face-attribute 'default :width nil 'default-on)
   'bold-width (face-attribute 'bold :width nil 'default-on)
   'width-table (if (boundp 'font-width-table) (memq 'condensed font-width-table) 'no-table))))"##,
    );
}

#[test]
fn ft_lambda_face_font_lock_add_keywords_globally_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'global-font-lock-mode-fbound (fboundp 'global-font-lock-mode)
   'font-lock-global-modes-bound (boundp 'font-lock-global-modes)
   'font-lock-global-modes-value (if (boundp 'font-lock-global-modes) font-lock-global-modes 'no)
   'font-lock-set-defaults-global (condition-case nil (font-lock-set-defaults) (error 'no))
   'font-lock-refresh-defaults-global (condition-case nil (font-lock-refresh-defaults) (error 'no))
   'font-lock-ensure-global (condition-case nil (progn (font-lock-flush) 'flushed) (error 'no)))))"##,
    );
}

#[test]
fn ft_lambda_face_overlay_line_properties_combined_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay line-spacing and face combined test content text data here end now")
    (let ((ov (make-overlay 1 62)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'line-spacing 5)
      (overlay-put ov 'line-height 1.5)
      (overlay-put ov 'wrap-prefix (propertize "> " 'face '(:foreground "blue"))))
    (list
     'face (overlay-get ov 'face)
     'line-spacing (overlay-get ov 'line-spacing)
     'line-height (overlay-get ov 'line-height)
     'wrap-prefix-face (get-text-property 0 (overlay-get ov 'wrap-prefix))
     'char-face (get-char-property 30 'face)
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_lambda_face_set_face_overline_various_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-overline-face) (error nil))
  (list
   'set-overline-t (condition-case nil (progn (set-face-attribute 'my-overline-face nil :overline t) (face-attribute 'my-overline-face :overline nil 'default-on)) (error 'no))
   'set-overline-nil (condition-case nil (progn (set-face-attribute 'my-overline-face nil :overline nil) (face-attribute 'my-overline-face :overline nil 'default-on)) (error 'no))
   'set-overline-color (condition-case nil (progn (set-face-attribute 'my-overline-face nil :overline '(:color "red")) (face-attribute 'my-overline-face :overline nil 'default-on)) (error 'no))
   'set-overline-unspec (condition-case nil (progn (set-face-attribute 'my-overline-face nil :overline 'unspecified) (face-attribute 'my-overline-face :overline nil 'default-on)) (error 'no))
   'default-overline (condition-case nil (face-attribute 'default :overline nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_lambda_face_property_interval_edge_boundary_stress() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 6 'face 'italic)
    (put-text-property 6 7 'face 'underline)
    (put-text-property 7 8 'face '(:foreground "red"))
    (put-text-property 8 63 'face '(:background "yellow"))
    (list
     'every-char (let ((i 1) (result nil))
                   (while (< i 10)
                     (push (list i (get-text-property i 'face)) result)
                     (setq i (1+ i)))
                   (nreverse result))
     'prop-changes-all (let ((pos 1) (changes nil))
                         (while pos
                           (setq pos (next-single-property-change pos 'face nil 63))
                           (when pos (push (list pos (get-text-property pos 'face)) changes)))
                         (nreverse changes))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_mu_face_property_text_any_with_specific_value_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "aaaabbbbccccddddeeeeffffgggghhhhiiiijjjj")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (put-text-property 21 25 'face '(:foreground "blue"))
    (put-text-property 25 29 'face '(:background "cyan"))
    (put-text-property 29 33 'face '(:weight bold))
    (put-text-property 33 37 'face '(:slant italic))
    (put-text-property 37 41 'face '(:underline t))
    (list
     'find-bold (text-property-any 1 41 'face 'bold)
     'find-italic (text-property-any 1 41 'face 'italic)
     'find-complex (text-property-any 1 41 'face '(:foreground "red"))
     'find-weight (text-property-any 1 41 'face '(:weight bold))
     'find-none (text-property-any 1 41 'face 'nonexistent-face)
     'not-all (text-property-not-all 1 41 'face 'bold)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_mu_face_font_lock_unfontify_entire_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Unfontify entire\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 20))))
        (font-lock-unfontify-buffer)
        (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 20))))
          (font-lock-fontify-buffer)
          (let ((v2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 15 20))))
            (list v0 v1 v2))))))))"##,
    );
}

#[test]
fn ft_mu_face_overlay_get_all_properties_as_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay properties as alist face test content data text here now")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "hello")
      (overlay-put ov 'evaporate t)
      (let ((props (overlay-properties ov)))
        (list
         'props-len (length props)
         'keys (let ((keys nil) (i 0))
                 (while (< i (length props))
                   (push (nth i props) keys)
                   (setq i (+ i 2)))
                 (nreverse keys))
         'face-get (overlay-get ov 'face)
         'priority-get (overlay-get ov 'priority)
         'help-echo-get (overlay-get ov 'help-echo)
         'evaporate-get (overlay-get ov 'evaporate)
         (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_mu_face_set_attribute_family_foundry_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-family-face) (error nil))
  (list
   'set-family-monospace (condition-case nil (progn (set-face-attribute 'my-family-face nil :family "Monospace") (face-attribute 'my-family-face :family nil 'default-on)) (error 'no))
   'set-family-unspec (condition-case nil (progn (set-face-attribute 'my-family-face nil :family 'unspecified) (face-attribute 'my-family-face :family nil 'default-on)) (error 'no))
   'default-family (face-attribute 'default :family nil 'default-on)
   'default-foundry (face-attribute 'default :foundry nil 'default-on)
   'bold-family (face-attribute 'bold :family nil 'default-on)
   'font-family-list-bound (boundp 'font-family-list))))"##,
    );
}

#[test]
fn ft_mu_face_font_lock_fontify_region_with_narrow_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun narrow-fontify-test (a b c) (+ a b c))\n")
    (insert "(defun second-narrow () 42)\n")
    ;; Only fontify first function region
    (font-lock-fontify-region 1 32)
    (list
     'fontified-first (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 15 25 31 32))
     'not-fontified-second (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(33 35 40 45))
     'faces-first (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 15 25 31))
     'faces-second (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(33 35 40 45))
     ;; Now fontify all
     'all-fontified (progn (font-lock-fontify-region 33 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 31 33 40 45)))))))"##,
    );
}

#[test]
fn ft_mu_face_text_property_set_on_fresh_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (list
     'empty-buffer (list 'face (get-text-property 1 'face) 'interval-count (length (object-intervals (current-buffer))))
     'after-insert (progn
                     (insert "New content")
                     (put-text-property 1 12 'face 'bold)
                     (list 'face (get-text-property 1 'face) 'interval-count (length (object-intervals (current-buffer)))))
     'after-erase (progn
                    (erase-buffer)
                    (list 'face (get-text-property 1 'face) 'interval-count (length (object-intervals (current-buffer)))))
     'after-reinsert (progn
                       (insert "Re-inserted")
                       (put-text-property 1 12 'face 'italic)
                       (list 'face (get-text-property 1 'face) 'interval-count (length (object-intervals (current-buffer)))))))))"##,
    );
}

#[test]
fn ft_mu_face_color_rgb_manipulate_and_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'rgb-red (color-name-to-rgb "red")
   'rgb-green (color-name-to-rgb "green")
   'rgb-blue (color-name-to-rgb "blue")
   'rgb-black (color-name-to-rgb "black")
   'rgb-white (color-name-to-rgb "white")
   'hex-to-rgb-FF0000 (color-name-to-rgb "#FF0000")
   'hex-to-rgb-00FF00 (color-name-to-rgb "#00FF00")
   'hex-to-rgb-0000FF (color-name-to-rgb "#0000FF")
   'hex-to-rgb-FF00FF (color-name-to-rgb "#FF00FF")
   'hex-to-rgb-FFFF00 (color-name-to-rgb "#FFFF00")
   'hex-to-rgb-00FFFF (color-name-to-rgb "#00FFFF")
   'hex-to-rgb-808080 (color-name-to-rgb "#808080")
   'valid-color-p (mapcar (lambda (c) (list c (condition-case nil (color-defined-p c) (error 'no)))) '("red" "green" "blue" "#FF0000" "#invalid")))))"##,
    );
}

#[test]
fn ft_mu_face_overlay_at_point_gets_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay at point get properties face test content data text here now end")
    (let ((ov1 (make-overlay 10 25))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 15 30))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 20 40))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 15))
    (list
     'at-12 (list (get-char-property 12 'face) (mapcar (lambda (ov) (overlay-get ov 'priority)) (overlays-at 12)))
     'at-22 (list (get-char-property 22 'face) (mapcar (lambda (ov) (overlay-get ov 'priority)) (overlays-at 22)))
     'at-28 (list (get-char-property 28 'face) (mapcar (lambda (ov) (overlay-get ov 'priority)) (overlays-at 28)))
     'overlay-start-list (mapcar #'overlay-start (list ov1 ov2 ov3))
     'overlay-end-list (mapcar #'overlay-end (list ov1 ov2 ov3))
     (progn (mapc #'delete-overlay (overlays-in 1 50)) 'cleaned))))"##,
    );
}

#[test]
fn ft_nu_face_text_property_change_during_insert_delete_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 3 5 6 8 10)))))
      (let ((v0 (funcall snap)))
        (goto-char 5) (insert "XX") (delete-region 3 6)
        (let ((v1 (funcall snap)))
          (goto-char 2) (insert "YYY") (delete-region 1 4)
          (let ((v2 (funcall snap)))
            (list v0 v1 v2 (length (object-intervals (current-buffer))))))))))"##,
    );
}

#[test]
fn ft_nu_face_font_lock_fontify_immediately_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Insert fontify\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        ;; Insert new content immediately after existing
        (goto-char (point-max))
        (insert "** DONE New insert\nBody new.\n\n")
        ;; Fontify just the new part
        (font-lock-fontify-region 20 (point-max))
        (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face)))
              (v2 (save-excursion (goto-char (point-min)) (search-forward "DONE") (get-text-property (match-beginning 0) 'face))))
          (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_nu_face_overlay_empty_move_then_fill_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 6 6)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'empty-overlay-start (overlay-start ov)
       'empty-overlay-end (overlay-end ov)
       'empty-face-get (overlay-get ov 'face)
       ;; Insert at empty overlay
       'after-insert (progn (goto-char 6) (insert "FILLED") (list 'face-at-6 (get-char-property 6 'face) 'face-at-8 (get-char-property 8 'face) 'face-at-12 (get-char-property 12 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_nu_face_set_face_font_weight_from_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-weight-table-face) (error nil))
  (list
   'set-bold (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'bold) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no))
   'set-light (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'light) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no))
   'set-heavy (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'heavy) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no))
   'set-medium (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'medium) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no))
   'set-ultra-light (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'ultra-light) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no))
   'set-ultra-bold (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'ultra-bold) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no))
   'reset (condition-case nil (progn (set-face-attribute 'my-weight-table-face nil :weight 'unspecified) (face-attribute 'my-weight-table-face :weight nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_nu_face_font_lock_unfontify_partial_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun a () 1)\n(defun b () 2)\n(defun c () 3)\n")
    (font-lock-ensure (point-min) (point-max))
    (let ((all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 15 30 45))))
      ;; Unfontify only middle region
      (font-lock-unfontify-region 15 30)
      (let ((partial-unfontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 15 20 25 30 45))))
        ;; Unfontify left and right too
        (font-lock-unfontify-region 1 15)
        (font-lock-unfontify-region 30 45)
        (let ((all-unfontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 15 30 45))))
          (list all-fontified partial-unfontified all-unfontified)))))))"##,
    );
}

#[test]
fn ft_nu_face_property_interval_overlap_resolve_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAABBBBBBBCCCCCCCDDDDDDDEEEEEEE")
    (put-text-property 1 8 'face 'bold)
    (put-text-property 5 15 'face 'italic)
    (put-text-property 12 22 'face 'underline)
    (put-text-property 20 29 'face '(:foreground "red"))
    (put-text-property 27 36 'face '(:background "yellow"))
    (list
     'overlap-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 8 12 15 20 22 27 29 35))
     'last-write-wins (list (get-text-property 1 'face) (get-text-property 8 'face) (get-text-property 15 'face) (get-text-property 22 'face) (get-text-property 29 'face))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_nu_face_set_attribute_inverse_video_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-inverse-face) (error nil))
  (list
   'set-inverse-on (condition-case nil (progn (set-face-attribute 'my-inverse-face nil :inverse-video t) (face-attribute 'my-inverse-face :inverse-video nil 'default-on)) (error 'no))
   'set-inverse-off (condition-case nil (progn (set-face-attribute 'my-inverse-face nil :inverse-video nil) (face-attribute 'my-inverse-face :inverse-video nil 'default-on)) (error 'no))
   'set-inverse-unspec (condition-case nil (progn (set-face-attribute 'my-inverse-face nil :inverse-video 'unspecified) (face-attribute 'my-inverse-face :inverse-video nil 'default-on)) (error 'no))
   'default-inverse (condition-case nil (face-attribute 'default :inverse-video nil 'default-on) (error 'no))
   'bold-inverse (condition-case nil (face-attribute 'bold :inverse-video nil 'default-on) (error 'no))
   'face-inverse-video-p-fbound (fboundp 'face-inverse-video-p))))"##,
    );
}

#[test]
fn ft_nu_face_text_property_read_after_write_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Read after write property face test buffer content text")
    (let ((result nil))
      ;; Write many face properties
      (put-text-property 1 11 'face 'bold) (push (list 1 (get-text-property 1 'face) (get-text-property 1 'face)) result)
      (put-text-property 11 21 'face 'italic) (push (list 11 (get-text-property 11 'face) (get-text-property 11 'face)) result)
      (put-text-property 21 31 'face 'underline) (push (list 21 (get-text-property 21 'face) (get-text-property 21 'face)) result)
      (put-text-property 31 41 'face '(:foreground "red")) (push (list 31 (get-text-property 31 'face) (get-text-property 31 'face)) result)
      (put-text-property 41 52 'face '(:background "yellow")) (push (list 41 (get-text-property 41 'face) (get-text-property 41 'face)) result)
      (list (nreverse result) (length (object-intervals (current-buffer))))))))"##,
    );
}

#[test]
fn ft_xi_face_font_lock_add_duplicate_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "DUPLICATE keyword test for font lock face duplicate check now")
    ;; Add same keyword twice with different faces
    (font-lock-add-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 font-lock-warning-face t)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "DUPLICATE") (get-text-property (match-beginning 0) 'face))))
      ;; Add duplicate with different face
      (font-lock-add-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "red") t)))
      (font-lock-fontify-buffer)
      (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "DUPLICATE") (get-text-property (match-beginning 0) 'face))))
        ;; Add again with overwrite
        (font-lock-add-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "green" :weight bold) overwrite)))
        (font-lock-fontify-buffer)
        (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "DUPLICATE") (get-text-property (match-beginning 0) 'face))))
          (list v0 v1 v2))))))"##,
    );
}

#[test]
fn ft_xi_face_overlay_evaporate_with_insert_before_and_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Evaporate overlay before after insert face test content text data here")
    (let ((ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'evaporate t)
      (overlay-put ov 'before-string (propertize "<<<" 'face '(:foreground "red")))
      (overlay-put ov 'after-string (propertize ">>>" 'face '(:foreground "blue"))))
    (list
     'before-evaporate (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(10 15 20 25 30 35))
     'overlay-alive (and ov (overlay-buffer ov))
     ;; Delete region to evaporate
     'after-delete (progn (delete-region 15 30) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25 30 35)))
     'overlay-dead (not (and ov (overlay-buffer ov)))))))"##,
    );
}

#[test]
fn ft_xi_face_set_face_font_registry_attribute() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-registry-face) (error nil))
  (list
   'default-registry (face-attribute 'default :registry nil 'default-on)
   'set-registry (condition-case nil (progn (set-face-attribute 'my-registry-face nil :registry "iso10646-1") (face-attribute 'my-registry-face :registry nil 'default-on)) (error 'no))
   'set-registry-unspec (condition-case nil (progn (set-face-attribute 'my-registry-face nil :registry 'unspecified) (face-attribute 'my-registry-face :registry nil 'default-on)) (error 'no))
   'font-registry-alternatives (if (boundp 'face-font-registry-alternatives) (length face-font-registry-alternatives) 'no-bound)))))"##,
    );
}

#[test]
fn ft_xi_face_font_lock_mode_toggle_and_check_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* TODO Consistency test\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        (font-lock-mode -1)
        (font-lock-mode 1)
        (font-lock-ensure (point-min) (point-max))
        (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
          (list v0 v1)))))))"##,
    );
}

#[test]
fn ft_xi_face_text_property_remove_list_of_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Remove list of properties face test buffer content text data here")
    (add-text-properties 1 57 (list 'face 'bold 'prop1 'val1 'prop2 'val2 'prop3 'val3 'prop4 'val4 'fontified t))
    (list
     'before-remove (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'prop1) (get-text-property pos 'prop2) (get-text-property pos 'prop3) (get-text-property pos 'prop4))) '(1 20 40 56))
     'after-remove-some (progn
                          (remove-list-of-text-properties 10 45 '(face prop1 prop3))
                          (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'prop1) (get-text-property pos 'prop2) (get-text-property pos 'prop3) (get-text-property pos 'prop4))) '(1 10 20 30 40 45 56)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_xi_face_overlay_variable_width_properties_after_mods() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Variable width overlay after modifications face test content data")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before-mod (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 15 20 25 30 35))
       'after-resize (progn (move-overlay ov 15 25) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25 30 35)))
       'after-move (progn (move-overlay ov 30 45) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25 30 35 40 45 50)))
       (progn (delete-overlay ov) 'cleaned)))))
"##,
    );
}

#[test]
fn ft_xi_face_font_lock_no_mode_fontify_buffer_manually() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    ;; Don't set major mode at all, just font-lock manually
    (font-lock-mode 1)
    (insert "Manual font lock without mode buffer content text area")
    (font-lock-fontify-buffer)
    (list
     'font-lock-mode font-lock-mode
     'fontified-at-1 (get-text-property 1 'fontified)
     'face-at-1 (get-text-property 1 'face)
     'fontified-at-end (get-text-property (point-max) 'fontified)
     'face-at-end (get-text-property (point-max) 'face)))))"##,
    );
}

#[test]
fn ft_xi_face_set_attribute_slant_weight_combo_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-slamt-weight-face) (error nil))
  (list
   'set-bold-italic (condition-case nil (progn (set-face-attribute 'my-slamt-weight-face nil :weight 'bold :slant 'italic) (list (face-attribute 'my-slamt-weight-face :weight nil 'default-on) (face-attribute 'my-slamt-weight-face :slant nil 'default-on))) (error 'no))
   'set-light-oblique (condition-case nil (progn (set-face-attribute 'my-slamt-weight-face nil :weight 'light :slant 'oblique) (list (face-attribute 'my-slamt-weight-face :weight nil 'default-on) (face-attribute 'my-slamt-weight-face :slant nil 'default-on))) (error 'no))
   'set-normal-normal (condition-case nil (progn (set-face-attribute 'my-slamt-weight-face nil :weight 'normal :slant 'normal) (list (face-attribute 'my-slamt-weight-face :weight nil 'default-on) (face-attribute 'my-slamt-weight-face :slant nil 'default-on))) (error 'no))
   'set-heavy-italic (condition-case nil (progn (set-face-attribute 'my-slamt-weight-face nil :weight 'heavy :slant 'italic) (list (face-attribute 'my-slamt-weight-face :weight nil 'default-on) (face-attribute 'my-slamt-weight-face :slant nil 'default-on))) (error 'no))
   'unspec-both (condition-case nil (progn (set-face-attribute 'my-slamt-weight-face nil :weight 'unspecified :slant 'unspecified) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_omicron_face_with_text_property_add_remove_add_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Add remove add cycle face property test buffer content")
    (put-text-property 1 50 'face 'bold)
    (remove-text-properties 10 30 '(face nil))
    (put-text-property 10 30 'face 'italic)
    (remove-text-properties 20 40 '(face nil))
    (put-text-property 20 40 'face 'underline)
    (list
     'faces-across (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 25 30 35 40 45 49))
     'interval-count (length (object-intervals (current-buffer)))
     'next-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 50)) '(1 10 20 30 40)))))"##,
    );
}

#[test]
fn ft_omicron_font_lock_add_keywords_with_override_behavior() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "OVERRIDE keyword test OVERRIDE behavior OVERRIDE face")
    ;; Add with prepend
    (font-lock-add-keywords nil '(("\\<\\(OVERRIDE\\)\\>" 1 '(:foreground "blue") prepend)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "OVERRIDE") (get-text-property (match-beginning 0) 'face))))
      ;; Add again with overwrite
      (font-lock-add-keywords nil '(("\\<\\(OVERRIDE\\)\\>" 1 '(:foreground "red" :weight bold) overwrite)))
      (font-lock-fontify-buffer)
      (list v0 (save-excursion (goto-char (point-min)) (search-forward "OVERRIDE") (get-text-property (match-beginning 0) 'face)))))))"##,
    );
}

#[test]
fn ft_omicron_face_overlay_before_string_face_inheritance_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before string face inheritance overlay test content data text here")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "[BEFORE]" 'face '(:foreground "red" :inherit bold))))
    (list
     'overlay-face (overlay-get ov 'face)
     'before-face (get-text-property 0 (overlay-get ov 'before-string))
     'before-char-face (get-char-property 10 'face)
     'overlay-start (overlay-start ov)
     'overlay-end (overlay-end ov)
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_omicron_face_set_attribute_with_nil_vs_unspecified_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-nil-face) (error nil))
  (list
   'set-weight-bold (condition-case nil (progn (set-face-attribute 'my-nil-face nil :weight 'bold) (face-attribute 'my-nil-face :weight nil 'default-on)) (error 'no))
   'set-weight-nil (condition-case nil (progn (set-face-attribute 'my-nil-face nil :weight nil) (face-attribute 'my-nil-face :weight nil 'default-on)) (error 'no))
   'set-weight-unspec (condition-case nil (progn (set-face-attribute 'my-nil-face nil :weight 'unspecified) (face-attribute 'my-nil-face :weight nil 'default-on)) (error 'no))
   'set-fg-red (condition-case nil (progn (set-face-foreground 'my-nil-face "red" nil) (face-foreground 'my-nil-face nil 'default-on)) (error 'no))
   'set-fg-nil (condition-case nil (progn (set-face-foreground 'my-nil-face nil nil) (face-foreground 'my-nil-face nil 'default-on)) (error 'no))
   'set-fg-unspec (condition-case nil (progn (set-face-foreground 'my-nil-face 'unspecified nil) (face-foreground 'my-nil-face nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_omicron_face_text_properties_at_boundary_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXYYYYZZZZWWWW")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (list
     'at-boundaries (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property (1+ pos) 'face))) '(1 4 5 8 9 12 13 16))
     'prop-changes-all (mapcar (lambda (pos) (next-single-property-change pos 'face nil 17)) '(1 2 5 6 9 10 13 14))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_omicron_font_lock_unfontify_region_then_fontify_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun unfontify-test () 42)\n")
    (font-lock-ensure (point-min) (point-max))
    (list
     'fontified-before (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20))
     'unfontify-region (progn (font-lock-unfontify-region 5 15) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15 20)))
     'fontify-all (progn (font-lock-fontify-region 1 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15 20)))))))"##,
    );
}

#[test]
fn ft_omicron_face_overlays_at_overlapping_regions_face_stack() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlapping overlay regions face stack test content text data buffer")
    (let ((ov1 (make-overlay 5 25))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 15 35))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 30))
    (let ((ov4 (make-overlay 3 40))) (overlay-put ov4 'face '(:foreground "gray")) (overlay-put ov4 'priority 5))
    (list
     'face-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (mapcar (lambda (ov) (overlay-get ov 'face)) (sort (overlays-at pos) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority))))))) '(1 5 8 12 18 22 28 32 38 42))
     (progn (mapc #'delete-overlay (overlays-in 1 43)) 'cleaned))))"##,
    );
}

#[test]
fn ft_omicron_face_with_face_spec_match_display_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (list
   'face-spec-match-p-fbound (fboundp 'face-spec-match-p)
   (condition-case nil
       (face-spec-match-p 'default
                           '(((class color) (min-colors 88)) (:foreground "black"))
                           (selected-frame))
     (error 'no-match))
   (condition-case nil
       (face-spec-choose '(((class color)) (:foreground "red")
                           (t (:foreground "blue"))))
     (error 'no-choose))
   'display-type (display-graphic-p)
   'display-colors (if (fboundp 'display-color-cells) (display-color-cells) 'no-cells)
   'min-colors-88 (>= (if (fboundp 'display-color-cells) (display-color-cells) 0) 88))))"##,
    );
}

#[test]
fn ft_pi_face_set_property_then_read_then_modify_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Set read modify cycle face property test buffer data")
    (put-text-property 1 15 'face 'bold)
    (let ((v0 (get-text-property 1 'face)))
      (put-text-property 1 15 'face 'italic)
      (let ((v1 (get-text-property 1 'face)))
        (put-text-property 15 30 'face 'underline)
        (let ((v2 (get-text-property 15 'face)))
          (remove-text-properties 1 30 '(face nil))
          (put-text-property 1 30 'face '(:foreground "red" :weight bold))
          (list v0 v1 v2 (get-text-property 1 'face) (get-text-property 15 'face) (get-text-property 29 'face)))))))"##,
    );
}

#[test]
fn ft_pi_font_lock_global_mode_vs_buffer_local_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'global-font-lock-mode-fbound (fboundp 'global-font-lock-mode)
   'font-lock-mode-fbound (fboundp 'font-lock-mode)
   (if (boundp 'global-font-lock-mode) (list 'global-var global-font-lock-mode) (list 'no-global-var))
   (condition-case nil
       (with-temp-buffer
         (fundamental-mode)
         (list
          'buffer-mode-before font-lock-mode
          'turn-on (progn (font-lock-mode 1) font-lock-mode)
          'turn-off (progn (font-lock-mode -1) font-lock-mode)
          'turn-on-again (progn (font-lock-mode 1) font-lock-mode)))
     (error 'no-font-lock-mode))
   'font-lock-global-modes (if (boundp 'font-lock-global-modes) font-lock-global-modes 'no-bound)
   'font-lock-defaults-function (fboundp 'font-lock-defaults))))"##,
    );
}

#[test]
fn ft_pi_face_overlay_with_line_height_and_face_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with line height and face combined test content text data here")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'line-height 2.0)
      (overlay-put ov 'line-spacing 10))
    (list
     'face (overlay-get ov 'face)
     'line-height (overlay-get ov 'line-height)
     'line-spacing (overlay-get ov 'line-spacing)
     'face-at-overlay (get-char-property 20 'face)
     'face-at-outside (get-char-property 5 'face)
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_pi_face_text_property_change_at_single_point_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (list
     'before-face (get-text-property 1 'face)
     'interval-count-0 (length (object-intervals (current-buffer)))
     'set-face (progn (put-text-property 1 2 'face 'bold) (get-text-property 1 'face))
     'interval-count-1 (length (object-intervals (current-buffer)))
     'change-face (progn (put-text-property 1 2 'face 'italic) (get-text-property 1 'face))
     'interval-count-2 (length (object-intervals (current-buffer)))
     'remove-face (progn (remove-text-properties 1 2 '(face nil)) (get-text-property 1 'face))
     'interval-count-3 (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_pi_font_lock_fontify_region_with_limit_bounds_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun limit-test (x y) (+ x y))\n")
    (list
     'fontify-first-half (progn (font-lock-fontify-region 1 15) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15)))
     'fontify-rest (progn (font-lock-fontify-region 15 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 15 20 25 30)))
     'all-faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 10 15 20 25 30))))))"##,
    );
}

#[test]
fn ft_pi_face_extend_property_various_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-extend (condition-case nil (face-attribute 'default :extend nil 'default-on) (error 'no))
   'bold-extend (condition-case nil (face-attribute 'bold :extend nil 'default-on) (error 'no))
   'italic-extend (condition-case nil (face-attribute 'italic :extend nil 'default-on) (error 'no))
   'underline-extend (condition-case nil (face-attribute 'underline :extend nil 'default-on) (error 'no))
   'fringe-extend (condition-case nil (face-attribute 'fringe :extend nil 'default-on) (error 'no))
   'region-extend (condition-case nil (face-attribute 'region :extend nil 'default-on) (error 'no))
   'highlight-extend (condition-case nil (face-attribute 'highlight :extend nil 'default-on) (error 'no))
   'mode-line-extend (condition-case nil (face-attribute 'mode-line :extend nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_pi_face_overlay_delete_from_inside_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Delete from inside overlay region face test text data buffer content here")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before-delete (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 20 30 40 45))
       ;; Delete from inside overlay (partial)
       'after-delete-partial (progn (delete-region 20 35) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25 30 35)))
       ;; Delete entire overlay region
       'after-delete-full (progn (delete-region 10 40) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_pi_face_color_defined_p_various_formats_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'color-defined-red (condition-case nil (color-defined-p "red") (error 'no))
   'color-defined-green (condition-case nil (color-defined-p "green") (error 'no))
   'color-defined-blue (condition-case nil (color-defined-p "blue") (error 'no))
   'color-defined-black (condition-case nil (color-defined-p "black") (error 'no))
   'color-defined-white (condition-case nil (color-defined-p "white") (error 'no))
   'color-defined-ff0000 (condition-case nil (color-defined-p "#FF0000") (error 'no))
   'color-defined-00ff00 (condition-case nil (color-defined-p "#00FF00") (error 'no))
   'color-defined-0000ff (condition-case nil (color-defined-p "#0000FF") (error 'no))
   'color-defined-invalid (condition-case nil (color-defined-p "#INVALID") (error 'no))
   'color-defined-notacolor (condition-case nil (color-defined-p "not-a-real-color-name-at-all") (error 'no))
   'color-values-fbound (fboundp 'color-values))))"##,
    );
}

#[test]
fn ft_rho_face_property_overlap_precise_regions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHH")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 3 11 'face 'italic)
    (put-text-property 8 16 'face 'underline)
    (put-text-property 13 21 'face '(:foreground "red"))
    (put-text-property 18 26 'face '(:background "yellow"))
    (put-text-property 23 31 'face '(:foreground "blue"))
    (put-text-property 28 36 'face '(:background "cyan"))
    (put-text-property 33 41 'face '(:slant italic))
    (list
     'overlapping-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 6 8 11 13 16 18 21 23 26 28 31 33 36 40))
     'last-write-wins-check (list (= (length (delete-dups (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 3 5))) 2)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_rho_font_lock_syntactic_keyword_regions_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (c-mode)
    (insert "int main() {\n  return 42;\n}\n")
    (font-lock-fontify-buffer)
    (mapcar
     (lambda (needle)
       (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))))
     '("int" "main" "return" "42"))))"##,
    );
}

#[test]
fn ft_rho_face_overlay_move_with_negative_start_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWW")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'initial-face (get-char-property 10 'face)
       ;; Move backward (negative direction)
       'after-move-back (progn (move-overlay ov 1 10) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 8 10 12 15 20)))
       ;; Move forward
       'after-move-fwd (progn (move-overlay ov 15 20) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 15 18 20 21)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_rho_face_text_property_inherit_in_face_plist_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Face plist with inherit property in text property test here end")
    (put-text-property 1 56 'face '(:foreground "blue" :inherit bold :weight extra-bold :slant italic))
    (list
     'face-value (get-text-property 1 'face)
     'facep (facep (get-text-property 1 'face))
     'face-listp (listp (get-text-property 1 'face))
     'face-plistp (plistp (get-text-property 1 'face))
     'extract-fg (plist-get (get-text-property 1 'face) :foreground)
     'extract-inherit (plist-get (get-text-property 1 'face) :inherit)
     'extract-weight (plist-get (get-text-property 1 'face) :weight)
     'extract-slant (plist-get (get-text-property 1 'face) :slant))))"##,
    );
}

#[test]
fn ft_rho_font_lock_unfontify_region_exact_boundary_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun a () 1)\n")
    (font-lock-ensure (point-min) (point-max))
    (list
     'all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15))
     ;; Unfontify exact region boundary
     'unfontify-1-10 (progn (font-lock-unfontify-region 1 11) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 11)))
     ;; Unfontify rest
     'unfontify-rest (progn (font-lock-unfontify-region 11 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 11)))
     ;; Re-fontify all
     'refontify-all (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 11)))))))"##,
    );
}

#[test]
fn ft_rho_face_overlay_string_length_and_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string length face test content buffer data text here")
    (let* ((before-str (propertize "[[[LONG-BEFORE-STRING]]]" 'face '(:foreground "red" :weight bold)))
           (after-str (propertize "{{{LONG-AFTER-STRING}}}" 'face '(:foreground "blue" :slant italic)))
           (ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string before-str)
      (overlay-put ov 'after-string after-str)
      (list
       'before-len (length before-str)
       'after-len (length after-str)
       'before-face (get-text-property 0 before-str)
       'after-face (get-text-property 0 after-str)
       'overlay-face (overlay-get ov 'face)
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_rho_face_set_font_attribute_with_frame_parameter() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-frame-font-face) (error nil))
  (list
   'frame-font (condition-case nil (frame-parameter nil 'font) (error 'no))
   'face-font (condition-case nil (face-font 'default nil) (error 'no))
   'set-font-by-frame-param (condition-case nil (progn (set-face-font 'my-frame-font-face (frame-parameter nil 'font) nil) 'set) (error 'no))
   'face-font-after-set (condition-case nil (face-font 'my-frame-font-face nil) (error 'no)))))"##,
    );
}

#[test]
fn ft_rho_face_text_property_line_number_line_begin_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Line one with face property\nLine two with face property\nLine three with face property")
    (put-text-property 1 27 'face 'bold)
    (put-text-property 28 52 'face 'italic)
    (put-text-property 53 78 'face 'underline)
    (list
     'faces-at-line-beginnings (mapcar (lambda (pos) (goto-char pos) (list pos (line-number-at-pos) (get-text-property pos 'face))) '(1 28 53))
     'faces-at-line-ends (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(26 27 51 52 77 78))
     'line-end-positions (save-excursion (goto-char (point-min)) (list (line-end-position 1) (line-end-position 2) (line-end-position 3)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_sigma_face_with_text_property_read_after_insert_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 3 6 8 10))))
      (goto-char 5) (insert "INSERTED")
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 3 6 10 14 17 20))))
        (list v0 v1 (length (object-intervals (current-buffer))))))))"##,
    );
}

#[test]
fn ft_sigma_font_lock_add_remove_add_same_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "CYCLE keyword test for font lock add remove cycle here end")
    (font-lock-add-keywords nil '(("\\<\\(CYCLE\\)\\>" 1 font-lock-warning-face t)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "CYCLE") (get-text-property (match-beginning 0) 'face))))
      (font-lock-remove-keywords nil '(("\\<\\(CYCLE\\)\\>" 1 font-lock-warning-face t)))
      (font-lock-fontify-buffer)
      (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "CYCLE") (get-text-property (match-beginning 0) 'face))))
        (font-lock-add-keywords nil '(("\\<\\(CYCLE\\)\\>" 1 '(:foreground "red" :weight bold) t)))
        (font-lock-fontify-buffer)
        (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "CYCLE") (get-text-property (match-beginning 0) 'face))))
          (list v0 v1 v2))))))"##,
    );
}

#[test]
fn ft_sigma_face_overlay_priority_interleave_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Priority interleave overlay face test content text data here now final end")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 15 35))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 30))
    (let ((ov3 (make-overlay 30 55))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 20))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 17 22 30 40 50 60)))))
      (let ((v0 (funcall snap)))
        (overlay-put ov1 'priority 50)
        (let ((v1 (funcall snap)))
          (overlay-put ov2 'priority 5) (overlay-put ov3 'priority 100)
          (let ((v2 (funcall snap)))
            (mapc #'delete-overlay (overlays-in 1 60))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_sigma_face_text_property_interval_with_many_splits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX")
    (put-text-property 1 53 'face 'bold)
    (list
     'one-interval (length (object-intervals (current-buffer)))
     'split-1 (progn (goto-char 10) (insert "Y") (length (object-intervals (current-buffer))))
     'split-2 (progn (goto-char 20) (insert "Z") (length (object-intervals (current-buffer))))
     'split-3 (progn (goto-char 30) (insert "W") (length (object-intervals (current-buffer))))
     'delete-merge (progn (delete-region 15 25) (length (object-intervals (current-buffer))))
     'spot-faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 10 15 25 30 40 53))))))"##,
    );
}

#[test]
fn ft_sigma_face_font_lock_fontify_region_unfontify_recheck() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun recheck-test (x) (+ x 1))\n")
    (font-lock-fontify-region 1 (point-max))
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 28))))
      (font-lock-unfontify-region 1 (point-max))
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 28))))
        (font-lock-fontify-region 1 (point-max))
        (let ((v2 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 28))))
          (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_sigma_face_with_face_and_invisible_toggle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Visible text HIDDEN here visible again")
    (put-text-property 1 13 'face 'bold)
    (put-text-property 13 23 'face 'italic :invisible t)
    (put-text-property 23 37 'face 'underline)
    (list
     'faces-invisible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'invisible) (invisible-p pos))) '(1 5 13 18 23 28 36))
     'remove-invisible (progn (remove-text-properties 13 23 '(invisible nil)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5 13 18 23 28 36)))
     're-add-invisible (progn (put-text-property 13 23 'invisible t) (put-text-property 13 23 'face '(:foreground "red")) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5 13 18 23 28 36))))))"##,
    );
}

#[test]
fn ft_sigma_face_overlay_make_buffer_copy_different_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay buffer copy different props face test content data here end now")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "test"))
    (let ((ov2 (make-overlay 35 55)))
      (overlay-put ov2 'face '(:foreground "red" :weight bold))
      (overlay-put ov2 'priority 25))
    (list
     'faces-with-overlays (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 20 30 35 40 45 55 60))
     'ov1-props (length (overlay-properties ov))
     'ov2-props (length (overlay-properties ov2))
     (progn (mapc #'delete-overlay (overlays-in 1 60)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_sigma_face_set_attribute_with_relative_height_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-rel-height-face) (error nil))
  (list
   'default-height (face-attribute 'default :height nil 'default-on)
   'set-height-1.0 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 1.0) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-height-0.5 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 0.5) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-height-2.0 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 2.0) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-height-int-120 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 120) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-height-int-200 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 200) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'face-attrib-relative-p (if (fboundp 'face-attribute-relative-p) (face-attribute-relative-p :height) 'no-func)))))"##,
    );
}

#[test]
fn ft_tau_face_text_property_value_comparison_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (list
     'compare-bold-bold (equal (get-text-property 1 'face) (get-text-property 1 'face))
     'compare-bold-italic (equal (get-text-property 1 'face) (get-text-property 6 'face))
     'compare-italic-underline (equal (get-text-property 6 'face) (get-text-property 11 'face))
     'compare-bold-red (equal (get-text-property 1 'face) (get-text-property 16 'face))
     'compare-complex-equal (equal (get-text-property 16 'face) (get-text-property 16 'face))
     'text-property-any-bold (text-property-any 1 21 'face 'bold)
     'text-property-any-underline (text-property-any 1 21 'face 'underline)
     'text-property-any-red (text-property-any 1 21 'face '(:foreground "red"))))))"##,
    );
}

#[test]
fn ft_tau_font_lock_mode_off_insert_on_recheck_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Mode off test\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        (font-lock-mode -1)
        ;; Insert while font-lock is off
        (goto-char (point-max))
        (insert "* DONE Inserted while off\nBody inserted.\n\n")
        (font-lock-mode 1)
        (font-lock-ensure (point-min) (point-max))
        (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face)))
              (v2 (save-excursion (goto-char (point-min)) (search-forward "DONE") (get-text-property (match-beginning 0) 'face))))
          (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_tau_face_overlay_creation_in_empty_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((ov (make-overlay 1 1)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'empty-overlay-start (overlay-start ov)
       'empty-overlay-end (overlay-end ov)
       'face-get (overlay-get ov 'face)
       'overlay-buffer (overlay-buffer ov)
       ;; Insert text after overlay
       'after-insert (progn (insert "Text after overlay") (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 18)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_tau_face_text_property_all_properties_list_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "All text properties list face test buffer content data here now end final")
    (add-text-properties 1 64 (list 'face 'bold 'key1 'val1 'key2 'val2 'key3 'val3 'key4 'val4 'key5 'val5 'fontified t))
    (list
     'props-count (length (text-properties-at 1))
     'all-keys (let ((props (text-properties-at 1)) (keys nil) (i 0))
                 (while (< i (length props))
                   (push (nth i props) keys)
                   (setq i (+ i 2)))
                 (nreverse keys))
     'face-at-1 (get-text-property 1 'face)
     'face-at-30 (get-text-property 30 'face)
     'face-at-63 (get-text-property 63 'face)
     'all-key-values (mapcar (lambda (k) (list k (get-text-property 1 k))) '(face key1 key2 key3 key4 key5 fontified)))))"##,
    );
}

#[test]
fn ft_tau_face_font_lock_fontify_syntactically_full_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; Comment line\n(\"string\" symbol 42)\n")
    (font-lock-fontify-syntactically (point-min) (point-max) nil)
    (list
     'comment-face (save-excursion (goto-char (point-min)) (search-forward "Comment") (get-text-property (match-beginning 0) 'face))
     'string-face (save-excursion (goto-char (point-min)) (search-forward "string") (get-text-property (match-beginning 0) 'face))
     'symbol-face (save-excursion (goto-char (point-min)) (search-forward "symbol") (get-text-property (match-beginning 0) 'face))
     'number-face (save-excursion (goto-char (point-min)) (search-forward "42") (get-text-property (match-beginning 0) 'face))
     'fontified-all (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 15 25 30 35)))))"##,
    );
}

#[test]
fn ft_tau_face_overlay_after_string_with_complex_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "After string complex face overlay test content data text here now")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'after-string
                   (propertize "{{AFTER-COMPLEX}}" 'face '(:foreground "blue" :weight bold :slant italic :underline t :background "white"))))
    (list
     'overlay-face (overlay-get ov 'face)
     'after-face (get-text-property 0 (overlay-get ov 'after-string))
     'after-face-length (length (text-properties-at 0 (overlay-get ov 'after-string)))
     'after-str-len (length (overlay-get ov 'after-string))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_tau_face_text_property_insert_at_interval_ends() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (list
     'before-insert (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 6 8 10))
     'insert-at-start (progn (goto-char 1) (insert "X") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 2 3 6 7 10 12)))
     'insert-at-boundary (progn (goto-char 6) (insert "YY") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 3 5 6 8 9 12 14)))
     'insert-at-end (progn (goto-char 11) (insert "Z") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 3 5 6 8 11 12 13))))))"##,
    );
}

#[test]
fn ft_tau_face_set_fill_column_indicator_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'fill-column-indicator-facep (facep 'fill-column-indicator)
   (condition-case nil (face-attribute 'fill-column-indicator :foreground nil 'default-on) (error 'no-fg))
   (condition-case nil (face-attribute 'fill-column-indicator :background nil 'default-on) (error 'no-bg))
   (condition-case nil (face-attribute 'fill-column-indicator :underline nil 'default-on) (error 'no-ul))
   'line-number-facep (facep 'line-number)
   'display-fill-column-indicator-bound (boundp 'display-fill-column-indicator))))"##,
    );
}

#[test]
fn ft_upsilon_face_property_every_other_char_different_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABABABABABABABABABAB")
    (let ((i 0))
      (while (< i 20)
        (put-text-property (1+ i) (+ i 2) 'face (if (evenp i) 'bold 'italic))
        (setq i (1+ i))))
    (list
     'every-other (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 4 5 6 7 8 9 10))
     'interval-count (length (object-intervals (current-buffer)))
     'next-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 21)) '(1 3 5 7 9 11 13 15 17 19)))))"##,
    );
}

#[test]
fn ft_upsilon_font_lock_add_keywords_with_eval_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "EVAL keyword test with font lock face now here end")
    (font-lock-add-keywords nil
                            (list (list "\\<\\(EVAL\\)\\>"
                                        1
                                        '(if (> (point) 15) '(:foreground "red") '(:foreground "blue"))
                                        t)))
    (font-lock-fontify-buffer)
    (list
     'first-eval (save-excursion (goto-char (point-min)) (search-forward "EVAL") (get-text-property (match-beginning 0) 'face))
     'all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30 40 50))))))"##,
    );
}

#[test]
fn ft_upsilon_face_overlay_local_map_and_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay local map face test content data text here now end final")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'keymap (make-sparse-keymap))
      (overlay-put ov 'local-map (make-sparse-keymap))
      (list
       'face (overlay-get ov 'face)
       'has-keymap (keymapp (overlay-get ov 'keymap))
       'has-local-map (keymapp (overlay-get ov 'local-map))
       'overlay-props-count (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_upsilon_face_text_property_handle_nil_values_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Handle nil values in text properties face test content text data")
    (put-text-property 1 20 'face 'bold)
    (put-text-property 20 40 'face nil)
    (put-text-property 40 53 'face '(:foreground "red"))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 20 30 40 50 52))
     'find-bold (text-property-any 1 53 'face 'bold)
     'find-nil (text-property-any 1 53 'face nil)
     'find-red (text-property-any 1 53 'face '(:foreground "red"))
     'find-italic (text-property-any 1 53 'face 'italic)
     'not-all-nil (text-property-not-all 1 53 'face nil)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_upsilon_Font_lock_defaults_after_mode_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (list
     'fundamental-defaults (condition-case nil (progn (fundamental-mode) (font-lock-set-defaults) 'ok) (error 'no))
     'text-mode-defaults (condition-case nil (progn (text-mode) (font-lock-set-defaults) 'ok) (error 'no))
     'emacs-lisp-defaults (condition-case nil (progn (emacs-lisp-mode) (font-lock-mode 1) (font-lock-set-defaults) 'ok) (error 'no))
     'after-modes (list 'font-lock-mode font-lock-mode 'defaults-bound (boundp 'font-lock-keywords))))))"##,
    );
}

#[test]
fn ft_upsilon_face_overlay_insert_behind_hooks_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (defvar my-behind-count 0)
  (defun my-behind-fn (ov after beg end &optional len) (setq my-behind-count (1+ my-behind-count)))
  (with-temp-buffer
    (insert "Insert behind hooks overlay face test content text data here")
    (let ((ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'insert-behind-hooks (list 'my-behind-fn))
      (list
       'before-insert my-behind-count
       'face-before (get-char-property 20 'face)
       (progn (goto-char 30) (insert "BEHIND") (list 'after-insert my-behind-count 'face-after (get-char-property 20 'face)))
       (progn (goto-char 15) (insert "FRONT") (list 'after-front-insert my-behind-count 'face-after (get-char-property 20 'face)))
       (progn (delete-overlay ov) (setq my-behind-count 0) 'cleaned))))))"##,
    );
}

#[test]
fn ft_upsilon_face_set_attribute_font_unspecified_comparison() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-unspec-font-face) (error nil))
  (list
   'default-font (condition-case nil (face-font 'default nil) (error 'no))
   'set-font (condition-case nil (progn (set-face-font 'my-unspec-font-face "Monospace-12" nil) (face-font 'my-unspec-font-face nil)) (error 'no))
   'set-font-unspec (condition-case nil (progn (set-face-font 'my-unspec-font-face 'unspecified nil) (face-font 'my-unspec-font-face nil)) (error 'no))
   'set-font-nil (condition-case nil (progn (set-face-font 'my-unspec-font-face nil nil) (face-font 'my-unspec-font-face nil)) (error 'no)))))"##,
    );
}

#[test]
fn ft_upsilon_face_text_properties_at_overlay_boundary_buffer_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay boundary buffer edge face test content text data")
    (let ((ov-start (make-overlay 1 10)))
      (overlay-put ov-start 'face '(:background "red")))
    (let ((ov-end (make-overlay 40 50)))
      (overlay-put ov-end 'face '(:background "blue")))
    (list
     'at-start (list (get-char-property 1 'face) (get-char-property 10 'face))
     'at-end (list (get-char-property 39 'face) (get-char-property 40 'face) (get-char-property 49 'face))
     'at-middle (get-char-property 25 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 51)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_phi_face_overlay_string_face_vs_overlay_face_precedence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string face vs overlay face precedence test content text data now")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow" :foreground "black"))
      (overlay-put ov 'before-string (propertize "[B]" 'face '(:foreground "red" :weight bold :background "white")))
      (list
       'overlay-face (overlay-get ov 'face)
       'before-str-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-str-all-props (text-properties-at 0 (overlay-get ov 'before-string))
       'at-overlay (get-char-property 15 'face)
       'at-outside (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_phi_font_lock_fontify_entire_buffer_partial_unfontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun a () 1)\n(defun b () 2)\n(defun c () 3)\n")
    (font-lock-ensure (point-min) (point-max))
    (list
     'all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30 40))
     'unfontify-a (progn (font-lock-unfontify-region 1 18) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15 18 20 30)))
     'unfontify-b (progn (font-lock-unfontify-region 18 34) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 18 25 34 40)))
     'unfontify-c (progn (font-lock-unfontify-region 34 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 18 34 40 45)))
     'refontify-all (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 18 34 45)))))))"##,
    );
}

#[test]
fn ft_phi_face_property_interval_count_after_each_prop_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXXXXXXXXXXX")
    (let ((counts nil))
      (push (list 'empty (length (object-intervals (current-buffer)))) counts)
      (put-text-property 1 21 'face 'bold)
      (push (list 'bold (length (object-intervals (current-buffer)))) counts)
      (put-text-property 5 15 'face 'italic)
      (push (list 'italic-overlap (length (object-intervals (current-buffer)))) counts)
      (remove-text-properties 5 15 '(face nil))
      (push (list 'removed-italic (length (object-intervals (current-buffer)))) counts)
      (put-text-property 10 20 'face 'underline)
      (push (list 'underline-at-end (length (object-intervals (current-buffer)))) counts)
      (nreverse counts))))"##,
    );
}

#[test]
fn ft_phi_face_overlay_priority_with_nil_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay priority with nil face test content text data here now end")
    (put-text-property 1 55 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 10 40)))
      (overlay-put ov1 'face nil)
      (overlay-put ov1 'priority 100))
    (let ((ov2 (make-overlay 15 35)))
      (overlay-put ov2 'face '(:background "yellow"))
      (overlay-put ov2 'priority 50))
    (list
     'nil-face-overlay (list (overlay-get ov1 'face) (overlay-get ov1 'priority) (overlay-start ov1) (overlay-end ov1))
     'has-face-overlay (list (overlay-get ov2 'face) (overlay-get ov2 'priority))
     'at-position (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 20 30 40 50))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_phi_face_text_property_any_and_not_all_with_limits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face 'bold)
    (put-text-property 21 25 'face 'italic)
    (put-text-property 25 29 'face 'underline)
    (put-text-property 29 33 'face '(:foreground "blue"))
    (put-text-property 33 37 'face 'bold)
    (put-text-property 37 41 'face 'italic)
    (list
     'any-bold-first-half (text-property-any 1 20 'face 'bold)
     'any-bold-second-half (text-property-any 20 41 'face 'bold)
     'not-all-bold-full (text-property-not-all 1 41 'face 'bold)
     'not-all-bold-first (text-property-not-all 1 10 'face 'bold)
     'any-underline-mid (text-property-any 5 30 'face 'underline)
     'any-red (text-property-any 1 41 'face '(:foreground "red")))
     'any-blue (text-property-any 1 41 'face '(:foreground "blue"))))))"##,
    );
}

#[test]
fn ft_phi_font_lock_fontify_two_same_buffers_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (let ((content "(defun same-test () 42)\n"))
    (with-temp-buffer
      (emacs-lisp-mode)
      (insert content)
      (font-lock-fontify-buffer)
      (let ((faces1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 23))))
        (with-temp-buffer
          (emacs-lisp-mode)
          (insert content)
          (font-lock-fontify-buffer)
          (let ((faces2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15 20 23))))
            (list faces1 faces2 (equal faces1 faces2))))))))"##,
    );
}

#[test]
fn ft_phi_face_overlay_read_only_property_with_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Read only overlay face property test content text data here")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'read-only t))
    (list
     'face (overlay-get ov 'face)
     'read-only (overlay-get ov 'read-only)
     'char-prop (get-char-property 20 'face)
     'read-only-prop (get-char-property 20 'read-only)
     'outside-overlay (get-char-property 5 'read-only)
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_phi_face_set_attribute_default_return_value_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'face-attr-fg (face-attribute 'default :foreground nil 'default-on)
   'face-attr-fg-type (type-of (face-attribute 'default :foreground nil 'default-on))
   'face-attr-bg (condition-case nil (face-attribute 'default :background nil 'default-on) (error 'no))
   'face-attr-weight (face-attribute 'default :weight nil 'default-on)
   'face-attr-slant (face-attribute 'default :slant nil 'default-on)
   'face-attr-width (face-attribute 'default :width nil 'default-on)
   'face-attr-height (face-attribute 'default :height nil 'default-on)
   'face-font-val (condition-case nil (face-font 'default nil) (error 'no))
   'face-font-type (condition-case nil (type-of (face-font 'default nil)) (error 'no)))))"##,
    );
}

#[test]
fn ft_chi_face_with_three_overlay_layers_on_same_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (put-text-property 1 26 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 5 20))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 5 20))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 5 20))) (overlay-put ov3 'face '(:slant italic)) (overlay-put ov3 'priority 30))
    (list
     'all-overlays-count (length (overlays-at 10))
     'face-at-10 (get-char-property 10 'face)
     'face-at-1 (get-char-property 1 'face)
     'face-at-25 (get-char-property 25 'face)
     'all-overlay-faces (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at 10) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority)))))
     (progn (mapc #'delete-overlay (overlays-in 1 26)) 'cleaned))))"##,
    );
}

#[test]
fn ft_chi_font_lock_fontify_region_outside_visible_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun out-of-range-test () t)\n")
    ;; Fontify region that extends beyond buffer
    (condition-case nil
        (font-lock-fontify-region 1 (+ (point-max) 10))
      (error 'no))
    (list
     'fontified-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 28))
     'faces-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 10 15 20 25))))))"##,
    );
}

#[test]
fn ft_chi_face_property_value_deep_comparison() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAA")
    (put-text-property 1 6 'face '(:foreground "red"))
    (put-text-property 1 6 'key1 '(:complex value))
    (list
     'face-equal-plist (equal (get-text-property 1 'face) '(:foreground "red"))
     'key-equal-list (equal (get-text-property 1 'key1) '(:complex value))
     'facep-check (facep (get-text-property 1 'face))
     'key-type (type-of (get-text-property 1 'key1))
     'all-props (text-properties-at 1)
     'props-count (length (text-properties-at 1)))))"##,
    );
}

#[test]
fn ft_chi_face_overlay_after_delete_face_persistence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Delete overlay face persistence test content text data here now end")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before-delete (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 15 25 35 45))
       'after-delete-ov (progn (delete-overlay ov) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 15 25 35 45)))
       'overlay-dead (not (overlay-buffer ov)))))))"##,
    );
}

#[test]
fn ft_chi_font_lock_fontify_region_before_after_fontified_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun fontified-check (x) (* x x))\n")
    (list
     'unfontified-before (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30))
     'after-fontify (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30)))
     'after-unfontify (progn (font-lock-unfontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30)))
     'after-refontify (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30)))))))"##,
    );
}

#[test]
fn ft_chi_face_text_property_at_very_narrow_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (list
     'single-char-no-face (get-text-property 1 'face)
     'single-char-no-fontified (get-text-property 1 'fontified)
     'set-face (progn (put-text-property 1 2 'face 'bold) (get-text-property 1 'face))
     'text-props-at-1 (text-properties-at 1)
     'interval-count (length (object-intervals (current-buffer)))
     ;; Extend buffer
     'after-extend (progn (goto-char 2) (insert "Y") (list (get-text-property 1 'face) (get-text-property 2 'face) (length (object-intervals (current-buffer))))))))))"##,
    );
}

#[test]
fn ft_chi_face_set_face_attribute_with_face_spec_match_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (list
   'face-spec-choose (condition-case nil (face-spec-choose '(((class color)) (:foreground "red") (t (:foreground "blue")))) (error 'no))
   (condition-case nil (face-spec-match-p 'default '(((class color)) (:foreground "red")) (selected-frame)) (error 'no))
   (condition-case nil (face-attribute 'default :foreground nil 'default-on) (error 'no))
   (condition-case nil (face-attribute 'default :weight nil 'default-on) (error 'no))
   (condition-case nil (face-attribute 'default :background nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_chi_face_overlay_get_set_face_roundtrip_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay get set face roundtrip test content data text here now done")
    (let ((ov (make-overlay 1 57)))
      (overlay-put ov 'face '(:foreground "blue"))
      (let ((v0 (overlay-get ov 'face)))
        (overlay-put ov 'face '(:background "yellow" :weight bold))
        (let ((v1 (overlay-get ov 'face)))
          (overlay-put ov 'face '(:foreground "red" :slant italic :underline t))
          (let ((v2 (overlay-get ov 'face)))
            (list v0 v1 v2 (get-char-property 30 'face) (progn (delete-overlay ov) 'cleaned)))))))))"##,
    );
}

#[test]
fn ft_remap_add_relative_multiple_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'default-weight-before (face-attribute 'default :weight nil 'default-on)
   'remap-1 (condition-case nil (progn (face-remap-add-relative 'default '(:weight bold)) 'ok) (error 'no))
   'default-weight-after-1 (face-attribute 'default :weight nil 'default-on)
   'remap-2 (condition-case nil (progn (face-remap-add-relative 'default '(:foreground "red")) 'ok) (error 'no))
   'remap-alist (face-remapping-alist)
   'remap-alist-length (length (face-remapping-alist))
   (condition-case nil (progn (face-remap-reset-base 'default) 'reset) (error 'no))
   'default-weight-after-reset (face-attribute 'default :weight nil 'default-on))))"##,
    );
}

#[test]
fn ft_remap_set_base_then_add_relative_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'set-base (condition-case nil (progn (face-remap-set-base 'bold '(:slant italic :foreground "blue")) 'ok) (error 'no))
   'bold-slant-after-base (condition-case nil (face-attribute 'bold :slant nil 'default-on) (error 'no))
   'bold-fg-after-base (condition-case nil (face-attribute 'bold :foreground nil 'default-on) (error 'no))
   'add-relative-to-base (condition-case nil (progn (face-remap-add-relative 'bold '(:weight extra-bold)) 'ok) (error 'no))
   'bold-weight-after-relative (condition-case nil (face-attribute 'bold :weight nil 'default-on) (error 'no))
   'remap-alist (face-remapping-alist)
   (condition-case nil (progn (face-remap-reset-base 'bold) 'reset) (error 'no)))))"##,
    );
}

#[test]
fn ft_remap_text_scale_cycle_up_down_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'default-height-before (face-attribute 'default :height nil 'default-on)
   'scale-up-1 (condition-case nil (progn (text-scale-increase 1) 'ok) (error 'no))
   'height-after-up-1 (face-attribute 'default :height nil 'default-on)
   'scale-up-2 (condition-case nil (progn (text-scale-increase 1) 'ok) (error 'no))
   'height-after-up-2 (face-attribute 'default :height nil 'default-on)
   'scale-down-1 (condition-case nil (progn (text-scale-decrease 1) 'ok) (error 'no))
   'height-after-down-1 (face-attribute 'default :height nil 'default-on)
   'scale-reset (condition-case nil (progn (text-scale-set 0) 'ok) (error 'no))
   'height-after-reset (face-attribute 'default :height nil 'default-on)
   'remap-alist (face-remapping-alist))))"##,
    );
}

#[test]
fn ft_remap_buffer_face_mode_toggle_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "Buffer face mode toggle face test content text data here")
    (put-text-property 1 50 'face 'bold)
    (list
     'before-mode (list (get-text-property 1 'face) (face-remapping-alist))
     'turn-on (condition-case nil (progn (buffer-face-mode 1) 'ok) (error 'no))
     'face-after-on (get-text-property 1 'face)
     'remap-after-on (face-remapping-alist)
     'turn-off (condition-case nil (progn (buffer-face-mode -1) 'ok) (error 'no))
     'face-after-off (get-text-property 1 'face)
     'remap-after-off (face-remapping-alist)))))"##,
    );
}

#[test]
fn ft_remap_variable_pitch_mode_toggle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "Variable pitch mode toggle face test content text buffer data here")
    (put-text-property 1 55 'face 'bold)
    (list
     'before-faces (list (get-text-property 1 'face) (face-attribute 'default :family nil 'default-on))
     'turn-on (condition-case nil (progn (variable-pitch-mode 1) 'ok) (error 'no))
     'faces-after-on (list (get-text-property 1 'face) (face-attribute 'default :family nil 'default-on))
     'remap-after-on (face-remapping-alist)
     'turn-off (condition-case nil (progn (variable-pitch-mode -1) 'ok) (error 'no))
     'faces-after-off (list (get-text-property 1 'face) (face-attribute 'default :family nil 'default-on))
     'remap-after-off (face-remapping-alist)))))"##,
    );
}

#[test]
fn ft_remap_add_remove_relative_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'default-weight-before (face-attribute 'default :weight nil 'default-on)
   'add-1 (condition-case nil (progn (face-remap-add-relative 'default '(:weight bold)) 'ok) (error 'no))
   'add-2 (condition-case nil (progn (face-remap-add-relative 'default '(:foreground "red")) 'ok) (error 'no))
   'add-3 (condition-case nil (progn (face-remap-add-relative 'default '(:slant italic)) 'ok) (error 'no))
   'remap-alist-after-adds (face-remapping-alist)
   'remove-1 (condition-case nil (progn (face-remap-remove-relative 'default) 'ok) (error 'no))
   'remap-alist-after-remove-1 (face-remapping-alist)
   'remove-2 (condition-case nil (progn (face-remap-remove-relative 'default) 'ok) (error 'no))
   'remap-alist-after-remove-2 (face-remapping-alist)
   'remove-3 (condition-case nil (progn (face-remap-remove-relative 'default) 'ok) (error 'no))
   'remap-alist-after-all-removed (face-remapping-alist)
   'default-weight-after-all (face-attribute 'default :weight nil 'default-on))))"##,
    );
}

#[test]
fn ft_remap_set_base_with_complex_spec_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'set-base-complex (condition-case nil (progn (face-remap-set-base 'default '(:weight bold :slant italic :underline t :height 1.2)) 'ok) (error 'no))
   'weight-after (face-attribute 'default :weight nil 'default-on)
   'slant-after (face-attribute 'default :slant nil 'default-on)
   'underline-after (condition-case nil (face-attribute 'default :underline nil 'default-on) (error 'no))
   'height-after (face-attribute 'default :height nil 'default-on)
   'remap-alist (face-remapping-alist)
   (condition-case nil (progn (face-remap-reset-base 'default) 'reset) (error 'no))
   'weight-after-reset (face-attribute 'default :weight nil 'default-on))))"##,
    );
}

#[test]
fn ft_remap_remapping_alist_consistency_after_reset() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'alist-before (face-remapping-alist)
   'add-relative (condition-case nil (progn (face-remap-add-relative 'default '(:weight bold)) 'ok) (error 'no))
   'alist-after-add (face-remapping-alist)
   'set-base (condition-case nil (progn (face-remap-set-base 'italic '(:slant oblique)) 'ok) (error 'no))
   'alist-after-base (face-remapping-alist)
   'reset-default (condition-case nil (progn (face-remap-reset-base 'default) 'ok) (error 'no))
   'alist-after-reset-default (face-remapping-alist)
   'reset-italic (condition-case nil (progn (face-remap-reset-base 'italic) 'ok) (error 'no))
   'alist-after-all-reset (face-remapping-alist)
   (equal (face-remapping-alist) nil))))"##,
    );
}

#[test]
fn ft_psi_face_remap_add_relative_to_inherited_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'bold-weight-before (face-attribute 'bold :weight nil 'default-on)
   'add-relative-to-bold (condition-case nil (progn (face-remap-add-relative 'bold '(:foreground "red" :slant italic)) 'ok) (error 'no))
   'bold-weight-after (face-attribute 'bold :weight nil 'default-on)
   'bold-fg-after (condition-case nil (face-attribute 'bold :foreground nil 'default-on) (error 'no))
   'bold-slant-after (face-attribute 'bold :slant nil 'default-on)
   'remap-alist (face-remapping-alist)
   (condition-case nil (progn (face-remap-reset-base 'bold) 'reset) (error 'no)))))"##,
    );
}

#[test]
fn ft_psi_overlay_invisible_with_face_both_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Visible text INVISIBLE text visible again here now end")
    (put-text-property 1 14 'face 'bold)
    (put-text-property 14 28 'face 'italic)
    (put-text-property 14 28 'invisible t)
    (put-text-property 28 53 'face 'underline)
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 100))
    (list
     'text+overlay-invisible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-char-property pos 'face) (invisible-p pos))) '(1 5 14 20 28 40 52))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_psi_font_lock_fontify_after_buffer_revert_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun revert-test () 42)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 26))))
      (erase-buffer)
      (insert "(defun new-test () 99)\n")
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 25))))
        (list v0 v1))))))"##,
    );
}

#[test]
fn ft_psi_face_property_search_text_property_not_all_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face 'bold)
    (list
     'not-all-bold (text-property-not-all 1 26 'face 'bold)
     'not-all-italic (text-property-not-all 1 26 'face 'italic)
     'not-all-underline (text-property-not-all 1 26 'face 'underline)
     'any-bold (text-property-any 1 26 'face 'bold)
     'any-none (text-property-any 1 26 'face 'nonexistent)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_psi_overlay_face_after_resize_and_move_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWWVVVVV")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25))
       'after-resize (progn (move-overlay ov 10 20) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 10 15 20 25)))
       'after-move (progn (move-overlay ov 1 10) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 10 15 20)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_psi_font_lock_fontify_keywords_then_add_more_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "FIRST keyword test SECOND keyword test THIRD keyword test end")
    (font-lock-add-keywords nil '(("\\<\\(FIRST\\)\\>" 1 '(:foreground "red") t)))
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("FIRST" "SECOND" "THIRD"))))
      (font-lock-add-keywords nil '(("\\<\\(SECOND\\)\\>" 1 '(:foreground "green") t) ("\\<\\(THIRD\\)\\>" 1 '(:foreground "blue") t)))
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("FIRST" "SECOND" "THIRD"))))
        (list v0 v1))))))"##,
    );
}

#[test]
fn ft_psi_face_text_property_char_property_get_and_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Char property get and overlay face test content text data buffer")
    (put-text-property 1 10 'face 'bold)
    (put-text-property 10 20 'font-lock-face 'italic)
    (let ((ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 100))
    (list
     'get-text-property (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face))) '(1 5 10 15 20 30 40 50))
     'get-char-property (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 30 40 50))
     'get-char-property-and-overlay (mapcar (lambda (pos) (goto-char pos) (get-char-property-and-overlay pos 'face)) '(1 5 10 15 20 30 40 50))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_psi_face_eieio_interaction_with_face_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'eieio)
  (list
   'facep-default (facep 'default)
   'facep-bold (facep 'bold)
   'face-attribute-default-weight (face-attribute 'default :weight nil 'default-on)
   'eieio-fbound-p (fboundp 'eieio-oref)
   (if (fboundp 'eieio-oref)
       (condition-case nil
           (progn
             (defclass test-face-class nil ((weight :initarg :weight :initform 'bold)))
             (let ((obj (test-face-class :weight 'bold)))
               (list 'class-created (eieio-oref obj 'weight) (facep 'default))))
         (error 'eieio-error))
     'no-eieio))))"##,
    );
}

#[test]
fn ft_omega3_face_text_property_any_predicate_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face ':extend t)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:foreground "green"))
    (put-text-property 21 25 'face 'underline)
    (put-text-property 25 29 'face ':weight bold)
    (put-text-property 29 33 'face '(:background "yellow"))
    (put-text-property 33 37 'face '(:foreground "blue" :weight bold))
    (put-text-property 37 41 'face '(:foreground "purple" :slant italic))
    (list
     'find-bold (text-property-any 1 41 'face 'bold)
     'find-underline (text-property-any 1 41 'face 'underline)
     'find-red (text-property-any 1 41 'face '(:foreground "red"))
     'find-yellow (text-property-any 1 41 'face '(:background "yellow"))
     'find-complex (text-property-any 1 41 'face '(:foreground "blue" :weight bold))
     'find-extend (text-property-any 1 41 'face ':extend t)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_omega3_font_lock_fontify_region_repeatedly_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun repeat-fontify (x y) (+ x y))\n")
    (let ((results nil))
      (dotimes (i 3)
        (font-lock-fontify-region 1 (point-max))
        (push (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 30)) results))
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_omega3_face_overlay_properties_after_buffer_erase_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay properties after buffer erase face test content")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before-erase (list 'ov-start (overlay-start ov) 'ov-end (overlay-end ov) 'ov-buffer (and ov (overlay-buffer ov) t))
       'after-erase (progn (erase-buffer) (list 'ov-start (overlay-start ov) 'ov-end (overlay-end ov) 'ov-dead (not (overlay-buffer ov)))))))))"##,
    );
}

#[test]
fn ft_omega3_face_text_property_next_single_change_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPP")
    (dotimes (i 16)
      (put-text-property (1+ (* i 2)) (+ (* i 2) 3) 'face (if (evenp i) 'bold 'italic)))
    (list
     'all-props (let ((pos 1) (result nil))
                  (while pos
                    (setq pos (next-single-property-change pos 'face nil 33))
                    (when pos (push (list pos (get-text-property pos 'face)) result)))
                  (nreverse result))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_omega3_face_set_attribute_with_distant_foreground_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-dist-fg-v-face) (error nil))
  (list
   'set-dist-fg-gray (condition-case nil (progn (set-face-attribute 'my-dist-fg-v-face nil :distant-foreground "gray") (face-attribute 'my-dist-fg-v-face :distant-foreground nil 'default-on)) (error 'no))
   'set-dist-fg-red (condition-case nil (progn (set-face-attribute 'my-dist-fg-v-face nil :distant-foreground "dark red") (face-attribute 'my-dist-fg-v-face :distant-foreground nil 'default-on)) (error 'no))
   'set-dist-fg-nil (condition-case nil (progn (set-face-attribute 'my-dist-fg-v-face nil :distant-foreground nil) (face-attribute 'my-dist-fg-v-face :distant-foreground nil 'default-on)) (error 'no))
   'set-dist-fg-unspec (condition-case nil (progn (set-face-attribute 'my-dist-fg-v-face nil :distant-foreground 'unspecified) (face-attribute 'my-dist-fg-v-face :distant-foreground nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_omega3_face_font_lock_fontify_block_in_narrowed_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* TODO Block in narrow\nBody.\n\n")
      (insert "* DONE Outside narrow\nBody outside.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((full-face (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        (goto-char (point-min))
        (search-forward "TODO Block in narrow")
        (beginning-of-line)
        (org-narrow-to-subtree)
        (let ((narrowed-face (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
          (widen)
          (list full-face narrowed-face)))))))"##,
    );
}

#[test]
fn ft_omega3_face_overlay_with_void_property_access_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Void property overlay access face test content text data here now end")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'face-get (overlay-get ov 'face)
       'nonexistent-prop (overlay-get ov 'this-prop-does-not-exist-at-all)
       'nil-prop-after-set (progn (overlay-put ov 'nonexistent nil) (overlay-get ov 'nonexistent))
       'overlay-props-count (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_omega3_text_property_previous_single_change_deep() {
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
    (put-text-property 26 31 'face '(:foreground "blue"))
    (list
     'prev-from-end (let ((pos 31) (result nil))
                      (while pos
                        (setq pos (previous-single-property-change pos 'face nil 1))
                        (when pos (push (list pos (get-text-property pos 'face)) result)))
                      (nreverse result))
     'prev-near-start (previous-single-property-change 6 'face nil 1)
     'prev-from-15 (previous-single-property-change 15 'face nil 1)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_cosmic3_face_font_lock_keywords_case_fold_disabled_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Case Sensitive MATCH case sensitive match CASE MATCH")
    (let ((font-lock-keywords-case-fold-search nil))
      (font-lock-add-keywords nil '(("\\<\\(CASE\\)\\>" 1 font-lock-warning-face t)))
      (font-lock-fontify-buffer)
      (list
       'case-upper (save-excursion (goto-char (point-min)) (search-forward "CASE") (get-text-property (match-beginning 0) 'face))
       'case-lower (save-excursion (goto-char (point-min)) (search-forward " case") (get-text-property (match-beginning 0) 'face))
       'case-mixed (save-excursion (goto-char (point-min)) (search-forward "Case") (get-text-property (match-beginning 0) 'face)))))))"##,
    );
}

#[test]
fn ft_cosmic3_face_overlay_properties_plist_access_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay plist access face test content data text here now end final done")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "help")
      (overlay-put ov 'evaporate t)
      (let ((plist (overlay-properties ov)))
        (list
         'plist-length (length plist)
         'plist-contains-face (plist-member plist 'face)
         'plist-get-face (plist-get plist 'face)
         'plist-get-priority (plist-get plist 'priority)
         'plist-get-help (plist-get plist 'help-echo)
         'plist-get-evap (plist-get plist 'evaporate)
         'plist-get-none (plist-get plist 'nonexistent-property)
         'last-key (car (last plist 2))
         (progn (delete-overlay ov) 'cleaned)))))))"##,
    );
}

#[test]
fn ft_cosmic3_face_spec_set_with_face_defface_spec_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (condition-case nil (copy-face 'default 'my-defface-spec-face) (error nil))
  (list
   'set-spec (condition-case nil (face-spec-set 'my-defface-spec-face '((t :weight bold :foreground "blue")) 'face-defface-spec) (error 'no-set))
   'weight-after (face-attribute 'my-defface-spec-face :weight nil 'default-on)
   'fg-after (condition-case nil (face-attribute 'my-defface-spec-face :foreground nil 'default-on) (error 'no))
   'set-spec-2 (condition-case nil (face-spec-set 'my-defface-spec-face '((t :slant italic :background "yellow")) 'face-defface-spec) (error 'no))
   'weight-after-2 (face-attribute 'my-defface-spec-face :weight nil 'default-on)
   'slant-after-2 (face-attribute 'my-defface-spec-face :slant nil 'default-on)
   'reset-spec (condition-case nil (face-spec-set 'my-defface-spec-face '((t)) 'face-defface-spec) (error 'no)))))"##,
    );
}

#[test]
fn ft_cosmic3_face_text_property_sticky_advanced_boundary_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "FrontStickyBackNonstickyMiddleRegionEndHereNowDoneFinalLast")
    (put-text-property 1 12 'face 'bold :front-sticky t :rear-nonsticky nil)
    (put-text-property 12 24 'face 'italic :front-sticky nil :rear-nonsticky '(face))
    (put-text-property 24 36 'face 'underline :front-sticky '(face) :rear-nonsticky nil)
    (put-text-property 36 52 'face '(:foreground "red") :front-sticky nil)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'front-sticky) (get-text-property pos 'rear-nonsticky))) '(1 6 12 18 24 30 36 42 48 51))
     'insert-at-12 (progn (goto-char 12) (insert "IN") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(10 12 14 16 20 26 38)))
     'insert-at-24 (progn (goto-char 24) (insert "AT") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(10 14 20 24 26 30 38)))
     'insert-at-36 (progn (goto-char 36) (insert "END") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(10 16 24 30 36 39 42 48 54))))))"##,
    );
}

#[test]
fn ft_cosmic3_face_font_lock_mode_disabled_state_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (font-lock-mode -1)
      (insert "* TODO Disabled mode\nBody.\n\n")
      (list
       'font-lock-mode font-lock-mode
       'face-at-heading (get-text-property 1 'face)
       'fontified-at-1 (get-text-property 1 'fontified)
       'face-at-body (save-excursion (goto-char (point-min)) (search-forward "Body") (get-text-property (match-beginning 0) 'face))
       ;; Enable and check
       (progn
         (font-lock-mode 1)
         (font-lock-ensure (point-min) (point-max))
         (list
          'face-after-enable (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))
          'fontified-after (get-text-property 1 'fontified))))))))"##,
    );
}

#[test]
fn ft_cosmic3_face_overlay_empty_insert_behind_at_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Empty overlay insert at buffer start face test text here now end")
    (let ((ov-start (make-overlay 1 1)))
      (overlay-put ov-start 'face '(:background "red")))
    (let ((ov-mid (make-overlay 20 20)))
      (overlay-put ov-mid 'face '(:foreground "green")))
    (let ((ov-end (make-overlay 52 52)))
      (overlay-put ov-end 'face '(:background "blue")))
    (list
     'before-insert (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35 40 45 50 52))
     'after-insert-start (progn (goto-char 1) (insert "START-") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 8 10 15 25 35 45 55 58)))
     'after-insert-mid (progn (goto-char 25) (insert "MID-") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 15 25 30 35 45 55 60)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned)))))"##,
    );
}

#[test]
fn ft_cosmic3_face_property_overlay_text_combined_layers_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property overlay text combined layers face test content data buffer")
    ;; Layer 1: text properties
    (put-text-property 1 20 'face 'bold)
    (put-text-property 20 40 'face '(:foreground "blue"))
    (put-text-property 40 55 'face 'underline)
    ;; Layer 2: font-lock-face
    (put-text-property 10 30 'font-lock-face 'italic)
    (put-text-property 30 50 'font-lock-face '(:foreground "red"))
    ;; Layer 3: overlays
    (let ((ov1 (make-overlay 5 25))) (overlay-put ov1 'face '(:background "yellow")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 20 45))) (overlay-put ov2 'face '(:weight bold)) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 35 55))) (overlay-put ov3 'face '(:slant italic)) (overlay-put ov3 'priority 15))
    (list
     'layered-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'font-lock-face) (get-char-property pos 'face))) '(1 10 20 30 40 50 55))
     'prop-counts (mapcar (lambda (pos) (goto-char pos) (list pos (length (text-properties-at pos)) (length (overlays-at pos)))) '(1 10 20 30 40 50))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_cosmic3_face_set_face_box_multiple_styles_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-box-roundtrip-face) (error nil))
  (list
   'set-none (condition-case nil (progn (set-face-attribute 'my-box-roundtrip-face nil :box t) (face-attribute 'my-box-roundtrip-face :box nil 'default-on)) (error 'no))
   'set-width-2 (condition-case nil (progn (set-face-attribute 'my-box-roundtrip-face nil :box '(:line-width 2)) (face-attribute 'my-box-roundtrip-face :box nil 'default-on)) (error 'no))
   'set-released (condition-case nil (progn (set-face-attribute 'my-box-roundtrip-face nil :box '(:style released-button)) (face-attribute 'my-box-roundtrip-face :box nil 'default-on)) (error 'no))
   'set-pressed-color (condition-case nil (progn (set-face-attribute 'my-box-roundtrip-face nil :box '(:style pressed-button :color "red" :line-width 3)) (face-attribute 'my-box-roundtrip-face :box nil 'default-on)) (error 'no))
   'set-flat (condition-case nil (progn (set-face-attribute 'my-box-roundtrip-face nil :box '(:style flat-button :line-width 1)) (face-attribute 'my-box-roundtrip-face :box nil 'default-on)) (error 'no))
   'set-off (condition-case nil (progn (set-face-attribute 'my-box-roundtrip-face nil :box nil) (face-attribute 'my-box-roundtrip-face :box nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_deepspace_face_overlay_make_empty_then_fill_face_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((ov (make-overlay 1 1)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'empty-start (overlay-start ov)
       'empty-end (overlay-end ov)
       'empty-face (overlay-get ov 'face)
       ;; Fill with text
       (progn (insert "Filled buffer with overlay face") (list 'after-fill-start (overlay-start ov) 'after-fill-end (overlay-end ov) 'face-at-1 (get-char-property 1 'face) 'face-at-10 (get-char-property 10 'face) 'face-at-30 (get-char-property 30 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_deepspace_font_lock_fontify_syntactically_vs_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; comment\n(defun syn-test () \"string\" 42)\n")
    (list
     'fontify-syn-only (progn
                         (font-lock-fontify-syntactically (point-min) (point-max) nil)
                         (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))) '("comment" "defun" "syn-test" "string" "42")))
     'fontify-kw-only (progn
                        (font-lock-fontify-keywords-region (point-min) (point-max) nil)
                        (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified)))) '("comment" "defun" "syn-test" "string" "42")))))))"##,
    );
}

#[test]
fn ft_deepspace_face_text_property_get_and_set_many_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Get and set many text property keys face test buffer content text data")
    (let ((keys '(k1 k2 k3 k4 k5 k6 k7 k8 k9 k10))
          (vals '(v1 v2 v3 v4 v5 v6 v7 v8 v9 v10)))
      (dotimes (i 10)
        (put-text-property 1 55 (nth i keys) (nth i vals)))
      (put-text-property 1 55 'face 'bold)
      (list
       'all-keys-set (mapcar (lambda (k) (list k (get-text-property 1 k))) keys)
       'face-value (get-text-property 1 'face)
       'props-count (length (text-properties-at 1))
       'props-count-at-30 (length (text-properties-at 30))))))"##,
    );
}

#[test]
fn ft_deepspace_face_overlay_string_at_exact_overlap_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Exact overlap overlay string face test content text data here now")
    (let ((ov1 (make-overlay 10 30)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'before-string (propertize "[B1]" 'face '(:foreground "red"))))
    (let ((ov2 (make-overlay 10 30)))
      (overlay-put ov2 'face '(:foreground "green"))
      (overlay-put ov2 'after-string (propertize "[A2]" 'face '(:foreground "blue"))))
    (list
     'face-at-ov (get-char-property 20 'face)
     'ov1-before-face (get-text-property 0 (overlay-get ov1 'before-string))
     'ov2-after-face (get-text-property 0 (overlay-get ov2 'after-string))
     'ov-count (length (overlays-at 20))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_deepspace_face_set_font_attribute_family_from_list_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-family-list-face) (error nil))
  (list
   'default-family (face-attribute 'default :family nil 'default-on)
   'set-monospace (condition-case nil (progn (set-face-attribute 'my-family-list-face nil :family "Monospace") (face-attribute 'my-family-list-face :family nil 'default-on)) (error 'no))
   'set-unspecified (condition-case nil (progn (set-face-attribute 'my-family-list-face nil :family 'unspecified) (face-attribute 'my-family-list-face :family nil 'default-on)) (error 'no))
   (condition-case nil (font-family-list) (error 'no-family-list))
   (if (boundp 'face-font-family-alternatives) (length face-font-family-alternatives) 'no-alternatives))))"##,
    );
}

#[test]
fn ft_deepspace_font_lock_remove_keywords_that_dont_exist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'remove-nonexistent (condition-case err (progn (font-lock-remove-keywords nil '(("\\<\\(NONEXISTENT\\)\\>" 1 font-lock-warning-face t))) 'silent-ok) (error (list 'error (car err))))
   'remove-from-empty (condition-case err (progn (font-lock-remove-keywords nil nil) 'silent-ok) (error (list 'error (car err))))
   'add-valid-remove-valid-and-non (condition-case nil (progn (font-lock-add-keywords nil '(("\\<\\(VALID\\)\\>" 1 '(:foreground "red") t))) (font-lock-remove-keywords nil '(("\\<\\(VALID\\)\\>" 1 '(:foreground "red") t) ("\\<\\(NOPE\\)\\>" 1 font-lock-warning-face t))) 'mixed-ok) (error 'mixed-error)))))"##,
    );
}

#[test]
fn ft_deepspace_face_overlay_before_after_display_property_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before after display property overlay face test content data text here")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "<<BEFORE>>" 'face '(:foreground "red" :weight bold)))
      (overlay-put ov 'after-string (propertize "{{AFTER}}" 'face '(:foreground "blue" :slant italic)))
      (overlay-put ov 'display "")
      (overlay-put ov 'help-echo "This is an overlay"))
    (list
     'overlay-face (overlay-get ov 'face)
     'before-face (get-text-property 0 (overlay-get ov 'before-string))
     'after-face (get-text-property 0 (overlay-get ov 'after-string))
     'has-display (eq (overlay-get ov 'display) "")
     'has-help (equal (overlay-get ov 'help-echo) "This is an overlay")
     'props-count (length (overlay-properties ov))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_deepspace_face_rear_nonsticky_face_propagation_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (put-text-property 1 6 'face 'bold :rear-nonsticky '(face))
    (put-text-property 6 11 'face 'italic :rear-nonsticky nil)
    (put-text-property 11 16 'face 'underline :rear-nonsticky t)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'rear-nonsticky))) '(1 5 6 8 11 13 15))
     'insert-at-rear-1 (progn (goto-char 6) (insert "X") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 6 7 8 12 14 16)))
     'insert-at-rear-2 (progn (goto-char 11) (insert "Y") (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 6 7 8 12 13 15 17))))))"##,
    );
}

#[test]
fn ft_hyperspace_face_set_attribute_multiple_in_one_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-multi-attrs-2-face) (error nil))
  (list
   'set-5-attrs (condition-case nil (progn (set-face-attribute 'my-multi-attrs-2-face nil :weight 'bold :slant 'italic :underline '(:color "red") :foreground "blue" :background "yellow") 'ok) (error 'no))
   'weight (face-attribute 'my-multi-attrs-2-face :weight nil 'default-on)
   'slant (face-attribute 'my-multi-attrs-2-face :slant nil 'default-on)
   'underline (condition-case nil (face-attribute 'my-multi-attrs-2-face :underline nil 'default-on) (error 'no))
   'fg (condition-case nil (face-foreground 'my-multi-attrs-2-face nil 'default-on) (error 'no))
   'bg (condition-case nil (face-background 'my-multi-attrs-2-face nil 'default-on) (error 'no))
   'reset (condition-case nil (progn (set-face-attribute 'my-multi-attrs-2-face nil :weight 'unspecified :slant 'unspecified :underline 'unspecified :foreground 'unspecified :background 'unspecified) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_hyperspace_font_lock_fontify_region_outside_bounds_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun out-bounds (x) x)\n")
    (list
     'fontify-before-buffer (condition-case nil (progn (font-lock-fontify-region -10 10) 'ok) (error 'clamped))
     'fontify-beyond-buffer (condition-case nil (progn (font-lock-fontify-region 1 (+ (point-max) 50)) 'ok) (error 'clamped))
     'unfontify-before (condition-case nil (progn (font-lock-unfontify-region -5 5) 'ok) (error 'clamped))
     'unfontify-beyond (condition-case nil (progn (font-lock-unfontify-region (point-max) (+ (point-max) 10)) 'ok) (error 'clamped))
     'fontified-after-all (get-text-property 1 'fontified)))))"##,
    );
}

#[test]
fn ft_hyperspace_face_overlay_get_all_props_as_alist_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay get all properties as plist alist face test content data")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'category 'my-test-cat)
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "test overlay help text")
      (let ((props (overlay-properties ov)))
        (list
         'props-length (length props)
         'face-get (plist-get props 'face)
         'category-get (plist-get props 'category)
         'priority-get (plist-get props 'priority)
         'help-get (plist-get props 'help-echo)
         'none-get (plist-get props 'nonexistent-property-key)
         'all-keys (let ((keys nil) (i 0))
                     (while (< i (length props))
                       (push (nth i props) keys)
                       (setq i (+ i 2)))
                     (nreverse keys))
         (progn (delete-overlay ov) 'cleaned)))))))"##,
    );
}

#[test]
fn ft_hyperspace_face_text_property_with_custom_plist_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Custom plist face in text property test content buffer text data here")
    (put-text-property 1 57 'face '(:foreground "#FF6600" :weight bold :slant italic :underline (:color "blue" :style wave) :height 1.2 :extend t))
    (list
     'face-value (get-text-property 1 'face)
     'is-plist (plistp (get-text-property 1 'face))
     'plist-length (/ (length (get-text-property 1 'face)) 2)
     'all-keys (let ((plist (get-text-property 1 'face)) (keys nil) (i 0))
                 (while (< i (length plist))
                   (push (nth i plist) keys)
                   (setq i (+ i 2)))
                 (nreverse keys))
     'facep-result (facep (get-text-property 1 'face))
     'listp-result (listp (get-text-property 1 'face))))))"##,
    );
}

#[test]
fn ft_hyperspace_font_lock_fontify_with_empty_font_lock_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (setq font-lock-keywords nil)
    (insert "No keywords no default font lock test buffer content text")
    (font-lock-fontify-buffer)
    (list
     'font-lock-keywords font-lock-keywords
     'fontified (get-text-property 1 'fontified)
     'face (get-text-property 1 'face)
     'font-lock-keywords-case-fold font-lock-keywords-case-fold
     'font-lock-keywords-only (if (boundp 'font-lock-keywords-only) font-lock-keywords-only 'no-bound)))))"##,
    );
}

#[test]
fn ft_hyperspace_face_overlay_face_with_priority_interleave() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Face priority interleave overlay test content text data buffer here now")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 20))
    (let ((ov2 (make-overlay 15 30))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 40))
    (let ((ov3 (make-overlay 25 45))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 10))
    (let ((ov4 (make-overlay 40 60))) (overlay-put ov4 'face '(:background "yellow")) (overlay-put ov4 'priority 60))
    (list
     'face-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at pos) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority))))))) '(1 10 18 22 28 35 42 50 58))
     (progn (mapc #'delete-overlay (overlays-in 1 60)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_hyperspace_face_add_face_text_property_incremental_build() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Incremental face text property build test content text data here end")
    (list
     'step-0 (get-text-property 1 'face)
     'step-1 (progn (add-face-text-property 1 58 '(:foreground "blue")) (get-text-property 1 'face))
     'step-2 (progn (add-face-text-property 1 58 '(:weight bold)) (get-text-property 1 'face))
     'step-3 (progn (add-face-text-property 1 30 '(:slant italic)) (get-text-property 1 'face))
     'step-4 (progn (add-face-text-property 30 58 '(:underline t)) (get-text-property 30 'face))
     'step-5 (progn (add-face-text-property 1 58 '(:background "yellow")) (get-text-property 1 'face))
     'step-6 (progn (add-face-text-property 40 58 '(:overline t)) (get-text-property 50 'face))))))"##,
    );
}

#[test]
fn ft_hyperspace_face_overlay_string_length_and_face_attrs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string length and face attributes test content text data")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string
                   (propertize "[[BEFORE-LONG-TEXT-STRING]]" 'face '(:foreground "red" :weight bold :slant italic :underline t)))
      (list
       'before-len (length (overlay-get ov 'before-string))
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-face-attrs-count (length (get-text-property 0 (overlay-get ov 'before-string)))
       'before-props-count (length (text-properties-at 0 (overlay-get ov 'before-string)))
       'overlay-face (overlay-get ov 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_voidspace_face_overlay_property_prio_change_check_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov1 (make-overlay 1 15))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 10 25))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 30))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 15 20 25 30 35)))))
      (let ((v0 (funcall snap)))
        (overlay-put ov1 'priority 50)
        (let ((v1 (funcall snap)))
          (overlay-put ov2 'priority 5)
          (let ((v2 (funcall snap)))
            (mapc #'delete-overlay (overlays-in 1 35))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_voidspace_font_lock_enable_fontify_buffer_immediate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* TODO Immediate fontify\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (list
       'face-at-heading (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))
       'fontified-at-1 (get-text-property 1 'fontified)
       'face-at-body (save-excursion (goto-char (point-min)) (search-forward "Body") (get-text-property (match-beginning 0) 'face)))
      ))))"##,
    );
}

#[test]
fn ft_voidspace_face_text_properties_after_add_face_increment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Add face incrementally test buffer content text data here now end")
    (list
     'none (get-text-property 1 'face)
     'add-bold (progn (put-text-property 1 20 'face 'bold) (get-text-property 1 'face))
     'add-italic-override (progn (put-text-property 1 30 'face 'italic) (get-text-property 15 'face))
     'add-underline-end (progn (put-text-property 40 58 'face 'underline) (get-text-property 50 'face))
     'add-red-mid (progn (put-text-property 25 40 'face '(:foreground "red")) (get-text-property 30 'face))
     'all-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 15 25 30 40 50 57)))))"##,
    );
}

#[test]
fn ft_voidspace_face_set_face_stipple_attribute_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'stipple-default (condition-case nil (face-attribute 'default :stipple nil 'default-on) (error 'no))
   'stipple-bold (condition-case nil (face-attribute 'bold :stipple nil 'default-on) (error 'no))
   'stipple-italic (condition-case nil (face-attribute 'italic :stipple nil 'default-on) (error 'no))
   (if (fboundp 'set-face-stipple)
       (condition-case nil (set-face-stipple 'default "gray" nil) (error 'no-set-stipple))
     'no-set-stipple-func)
   (if (fboundp 'face-stipple)
       (condition-case nil (face-stipple 'default nil 'default-on) (error 'no-face-stipple))
     'no-face-stipple-func))))"##,
    );
}

#[test]
fn ft_voidspace_face_overlay_move_to_start_of_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay move to start of buffer face test content text data here")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 20 30 40 54))
       'move-to-start (progn (move-overlay ov 1 15) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 15 20 30 40 54)))
       'move-to-end (progn (move-overlay ov 40 54) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 20 30 40 45 53 54)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_voidspace_font_lock_fontify_keywords_with_highlight_override() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "HIGHLIGHT keyword test with font lock keyword highlight override check")
    (font-lock-add-keywords nil
                            '(("\\<\\(HIGHLIGHT\\)\\>" 0 font-lock-warning-face t)
                              ("\\<\\(keyword\\)\\>" 0 '(:foreground "red" :weight bold) t)
                              ("\\<\\(override\\)\\>" 0 '(:foreground "blue" :slant italic) t)))
    (font-lock-fontify-buffer)
    (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (list n (get-text-property (match-beginning 0) 'face)))) '("HIGHLIGHT" "keyword" "override"))))"##,
    );
}

#[test]
fn ft_voidspace_face_overlay_before_string_empty_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before string empty overlay face test content text data here end")
    (let ((ov1 (make-overlay 10 30)))
      (overlay-put ov1 'face '(:background "yellow"))
      (overlay-put ov1 'before-string ""))
    (let ((ov2 (make-overlay 35 50)))
      (overlay-put ov2 'face '(:background "cyan"))
      (overlay-put ov2 'after-string ""))
    (list
     'ov1-before-str (overlay-get ov1 'before-string)
     'ov2-after-str (overlay-get ov2 'after-string)
     'face-at-ov1 (get-char-property 20 'face)
     'face-at-ov2 (get-char-property 40 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned))))))"##,
    );
}

#[test]
fn ft_voidspace_face_text_property_single_property_interval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Single property interval face test buffer content text data now end final")
    (put-text-property 1 63 'face '(:foreground "blue"))
    (list
     'interval-count (length (object-intervals (current-buffer)))
     'face-at-1 (get-text-property 1 'face)
     'face-at-30 (get-text-property 30 'face)
     'face-at-62 (get-text-property 62 'face)
     'next-prop-change (next-single-property-change 1 'face nil 63)
     'prev-prop-change (previous-single-property-change 63 'face nil 1)
     'multiple-reads-same (equal (get-text-property 1 'face) (get-text-property 30 'face) (get-text-property 60 'face)))))"##,
    );
}

#[test]
fn ft_quark_face_text_property_interval_object_access_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIII")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (put-text-property 21 25 'face '(:foreground "blue"))
    (put-text-property 25 29 'face '(:background "cyan"))
    (put-text-property 29 33 'face '(:slant italic))
    (let ((intervals (object-intervals (current-buffer))))
      (list
       'count (length intervals)
       'first-start (overlay-start (car intervals))
       'first-end (overlay-end (car intervals))
       'last-start (overlay-start (car (last intervals)))
       'last-end (overlay-end (car (last intervals)))
       'all-starts (mapcar #'overlay-start intervals)
       'all-ends (mapcar #'overlay-end intervals))))))"##,
    );
}

#[test]
fn ft_quark_font_lock_fontify_keywords_with_multiple_matches_in_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "MATCH MATCH MATCH multiple same keyword on one line MATCH end")
    (font-lock-add-keywords nil '(("\\<\\(MATCH\\)\\>" 1 '(:foreground "red" :weight bold) t)))
    (font-lock-fontify-buffer)
    (list
     'all-occurrences (let ((result nil))
                        (save-excursion
                          (goto-char (point-min))
                          (while (search-forward "MATCH" nil t)
                            (push (list (match-beginning 0) (get-text-property (match-beginning 0) 'face)) result)))
                        (nreverse result))
     'match-count (save-excursion (goto-char (point-min)) (how-many "MATCH"))
     'non-match-face (save-excursion (goto-char (point-min)) (search-forward "same") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_quark_face_set_face_all_properties_then_unset_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-all-unset-face) (error nil))
  (condition-case nil (set-face-attribute 'my-all-unset-face nil :weight 'bold :slant 'italic :underline '(:color "red") :overline t :strike-through t :box t :inverse-video t :foreground "red" :background "yellow" :height 150 :width 'condensed) (error nil))
  (list
   'before-weight (face-attribute 'my-all-unset-face :weight nil 'default-on)
   'before-fg (condition-case nil (face-foreground 'my-all-unset-face nil 'default-on) (error 'no))
   'unset-all (condition-case nil (progn (set-face-attribute 'my-all-unset-face nil :weight 'unspecified :slant 'unspecified :underline 'unspecified :overline 'unspecified :strike-through 'unspecified :box 'unspecified :inverse-video 'unspecified :foreground 'unspecified :background 'unspecified :height 'unspecified :width 'unspecified) 'ok) (error 'no))
   'after-weight (face-attribute 'my-all-unset-face :weight nil 'default-on)
   'after-fg (condition-case nil (face-foreground 'my-all-unset-face nil 'default-on) (error 'no))
   'after-bg (condition-case nil (face-background 'my-all-unset-face nil 'default-on) (error 'no))
   'after-height (face-attribute 'my-all-unset-face :height nil 'default-on))))"##,
    );
}

#[test]
fn ft_quark_face_overlay_category_property_inherit_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay category property inherit face test content text data buffer")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'category 'my-cat) (overlay-put ov1 'face '(:background "red" :inherit bold)))
    (let ((ov2 (make-overlay 15 35))) (overlay-put ov2 'category 'my-cat) (overlay-put ov2 'face '(:foreground "green")))
    (let ((ov3 (make-overlay 30 55))) (overlay-put ov3 'category 'other-cat) (overlay-put ov3 'face '(:background "blue" :inherit italic)))
    (list
     'cat-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 10 18 25 32 40 50 55))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_quark_font_lock_fontify_whole_buffer_piece_by_piece() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun a () 1)\n(defun b () 2)\n(defun c () 3)\n(defun d () 4)\n(defun e () 5)\n")
    (let ((chunk-size 15) (pos 1) (total (point-max)) (fontified nil))
      (while (< pos total)
        (font-lock-fontify-region pos (min (+ pos chunk-size) total))
        (setq pos (+ pos chunk-size)))
      (list
       'fontified-1 (get-text-property 1 'fontified)
       'fontified-15 (get-text-property 15 'fontified)
       'fontified-30 (get-text-property 30 'fontified)
       'fontified-45 (get-text-property 45 'fontified)
       'fontified-60 (get-text-property 60 'fontified)
       'fontified-75 (get-text-property 75 'fontified)
       'all-faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 18 25 35 42 52 60 70 78)))))))"##,
    );
}

#[test]
fn ft_quark_face_property_all_text_properties_at_specific_pos() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Properties at specific positions face test content text data here now")
    (add-text-properties 1 60 (list 'face 'bold 'a '1 'b '2 'c '3))
    (add-text-properties 20 40 (list 'face 'italic 'd '4 'e '5))
    (add-text-properties 40 60 (list 'face 'underline 'f '6))
    (list
     'at-1 (text-properties-at 1)
     'at-20 (text-properties-at 20)
     'at-30 (text-properties-at 30)
     'at-40 (text-properties-at 40)
     'at-50 (text-properties-at 50)
     'at-59 (text-properties-at 59)
     'props-count-at-1 (length (text-properties-at 1))
     'props-count-at-20 (length (text-properties-at 20))
     'props-count-at-40 (length (text-properties-at 40))))))"##,
    );
}

#[test]
fn ft_quark_face_overlay_priority_and_face_get_after_move() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHHIIIIIJJJJJ")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 100)
      (list
       'face-before (get-char-property 10 'face)
       'start-before (overlay-start ov)
       'end-before (overlay-end ov)
       'move-right (progn (move-overlay ov 20 30) (list 'face (get-char-property 25 'face) 'start (overlay-start ov) 'end (overlay-end ov)))
       'move-left (progn (move-overlay ov 1 10) (list 'face (get-char-property 5 'face) 'start (overlay-start ov) 'end (overlay-end ov)))
       'move-to-end (progn (move-overlay ov 40 55) (list 'face (get-char-property 50 'face) 'start (overlay-start ov) 'end (overlay-end ov)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_quark_face_face_everything_all_fbounds_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'facep-fbound (fboundp 'facep)
   'make-face-fbound (fboundp 'make-face)
   'copy-face-fbound (fboundp 'copy-face)
   'face-list-fbound (fboundp 'face-list)
   'face-id-fbound (fboundp 'face-id)
   'face-equal-fbound (fboundp 'face-equal)
   'face-differs-from-default-p-fbound (fboundp 'face-differs-from-default-p)
   'face-attribute-fbound (fboundp 'face-attribute)
   'set-face-attribute-fbound (fboundp 'set-face-attribute)
   'face-foreground-fbound (fboundp 'face-foreground)
   'face-background-fbound (fboundp 'face-background)
   'face-font-fbound (fboundp 'face-font)
   'set-face-foreground-fbound (fboundp 'set-face-foreground)
   'set-face-background-fbound (fboundp 'set-face-background)
   'set-face-font-fbound (fboundp 'set-face-font)
   'face-bold-p-fbound (fboundp 'face-bold-p)
   'face-italic-p-fbound (fboundp 'face-italic-p)
   'face-underline-p-fbound (fboundp 'face-underline-p)
   'set-face-underline-fbound (fboundp 'set-face-underline)
   'face-all-attributes-fbound (fboundp 'face-all-attributes)
   'face-spec-set-fbound (fboundp 'face-spec-set)
   'face-spec-choose-fbound (fboundp 'face-spec-choose)
   'face-remap-add-relative-fbound (fboundp 'face-remap-add-relative)
   'face-remap-set-base-fbound (fboundp 'face-remap-set-base)
   'face-remap-reset-base-fbound (fboundp 'face-remap-reset-base))))"##,
    );
}

#[test]
fn ft_neutrino_face_overlay_face_get_after_resize_and_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'initial (get-char-property 10 'face)
       'resize-shrink (progn (move-overlay ov 10 15) (get-char-property 12 'face))
       'resize-expand (progn (move-overlay ov 5 25) (get-char-property 15 'face))
       'change-face (progn (overlay-put ov 'face '(:foreground "red" :weight bold)) (get-char-property 15 'face))
       'change-priority (progn (overlay-put ov 'priority 100) (get-char-property 15 'face))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neutrino_font_lock_fontify_region_with_keywords_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (insert "Keywords only mode test for font lock face buffer content")
    (setq-local font-lock-keywords-only t)
    (font-lock-add-keywords nil '(("\\<\\(Keywords\\)\\>" 1 '(:foreground "red" :weight bold) t)))
    (font-lock-mode 1)
    (font-lock-fontify-buffer)
    (list
     'keywords-only (if (boundp 'font-lock-keywords-only) font-lock-keywords-only 'no-bound)
     'face-keyword (save-excursion (goto-char (point-min)) (search-forward "Keywords") (get-text-property (match-beginning 0) 'face))
     'face-only (save-excursion (goto-char (point-min)) (search-forward "only") (get-text-property (match-beginning 0) 'face))
     'fontified (get-text-property 1 'fontified)
     (progn (kill-local-variable 'font-lock-keywords-only) 'cleaned)))))"##,
    );
}

#[test]
fn ft_neutrino_face_and_display_graphic_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'display-graphic-p (display-graphic-p)
   'display-color-p (display-color-p)
   'display-planes (if (fboundp 'display-planes) (display-planes) 'no)
   'display-color-cells (if (fboundp 'display-color-cells) (display-color-cells) 'no)
   'display-mm-height (if (fboundp 'display-mm-height) (display-mm-height) 'no)
   'display-mm-width (if (fboundp 'display-mm-width) (display-mm-width) 'no)
   'display-pixel-height (display-pixel-height)
   'display-pixel-width (display-pixel-width)
   'face-attrs-conditional (condition-case nil (face-attribute 'default :family nil t) (error 'no))
   'face-fg-conditional (condition-case nil (face-foreground 'default (selected-frame) 'default-on) (error 'no))
   'face-bg-conditional (condition-case nil (face-background 'default (selected-frame) 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_neutrino_face_overlay_with_multiple_property_deletes_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (put-text-property 1 6 'face 'bold :prop1 'v1 :prop2 'v2)
    (put-text-property 6 11 'face 'italic :prop1 'v3 :prop3 'v4)
    (put-text-property 11 16 'face 'underline :prop2 'v5 :prop4 'v6)
    (put-text-property 16 21 'face '(:foreground "red") :prop1 'v7 :prop3 'v8 :prop5 'v9)
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:foreground "blue") :prop6 'v10 :prop7 'v11)
    (list
     'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (length (text-properties-at pos)))) '(1 3 6 8 11 13 16 18 21 23 26 28 30))
     'remove-prop1 (progn (remove-text-properties 1 26 '(prop1 nil prop3 nil)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'prop1) (get-text-property pos 'prop3))) '(1 6 11 16 21 26)))
     'remove-prop2 (progn (remove-text-properties 1 26 '(prop2 nil prop4 nil)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'prop2))) '(1 6 11 16 21 26)))
     'faces-after-removals (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 6 11 16 21 26 30)))))"##,
    );
}

#[test]
fn ft_neutrino_font_lock_fontify_keywords_with_backref_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "<TAG>content</TAG> <DIV>more</DIV> <SPAN>text</SPAN>")
    (font-lock-add-keywords nil
                            '(("<\\([A-Z]+\\)>\\([^<]*\\)</\\([A-Z]+\\)>"
                               (1 '(:foreground "red" :weight bold))
                               (2 '(:foreground "blue"))
                               (3 '(:foreground "red" :weight bold)))))
    (font-lock-fontify-buffer)
    (list
     'tag1-face (save-excursion (goto-char (point-min)) (search-forward "TAG>") (get-text-property (match-beginning 0) 'face))
     'content-face (save-excursion (goto-char (point-min)) (search-forward "content") (get-text-property (match-beginning 0) 'face))
     'closing-face (save-excursion (goto-char (point-min)) (search-forward "/TAG>") (get-text-property (match-beginning 0) 'face))
     'fontified-all (get-text-property 1 'fontified)))))"##,
    );
}

#[test]
fn ft_neutrino_face_set_face_font_by_spec_then_by_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-spec-name-font-face) (error nil))
  (list
   'set-by-spec (condition-case nil (progn (set-face-font 'my-spec-name-font-face (font-spec :family "Monospace" :size 12 :weight 'bold) nil) 'ok) (error 'no))
   'get-after-spec (condition-case nil (face-font 'my-spec-name-font-face nil) (error 'no))
   'set-by-name (condition-case nil (progn (set-face-font 'my-spec-name-font-face "Monospace-Bold-12" nil) 'ok) (error 'no))
   'get-after-name (condition-case nil (face-font 'my-spec-name-font-face nil) (error 'no))
   'set-by-xlfd-name (condition-case nil (let ((font (face-font 'default nil))) (if (fontp font) (progn (set-face-font 'my-spec-name-font-face (font-xlfd-name font) nil) 'ok) 'no-default-font)) (error 'no-xlfd))
   'get-after-xlfd (condition-case nil (face-font 'my-spec-name-font-face nil) (error 'no))
   'reset-font (condition-case nil (progn (set-face-font 'my-spec-name-font-face 'unspecified nil) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_neutrino_face_overlay_insert_behind_hook_face_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (defvar my-behind-hook-ran nil)
  (defun my-behind-hook-fn (ov after beg end &optional len)
    (setq my-behind-hook-ran (cons (list 'called after beg end len) my-behind-hook-ran)))
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 6 11)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'insert-behind-hooks (list 'my-behind-hook-fn))
      (list
       'before-hook my-behind-hook-ran
       'face-before (get-char-property 8 'face)
       (progn (goto-char 11) (insert "AFTER-OV") (list 'after-insert my-behind-hook-ran 'face-at-8 (get-char-property 8 'face) 'face-at-12 (get-char-property 12 'face)))
       (progn (goto-char 6) (insert "BEFORE-OV") (list 'after-another my-behind-hook-ran 'face-at-5 (get-char-property 5 'face)))
       (progn (delete-overlay ov) (setq my-behind-hook-ran nil) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neutrino_face_with_filtered_face_attribute_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'filtered-face-fbound (fboundp 'filteredp)
   'face-filtered-available (condition-case nil (and (boundp 'face-filters) face-filters) (error 'no-filters))
   (condition-case nil
       (with-temp-buffer
         (insert "Filtered face test")
         (put-text-property 1 18 'face '(:filtered (:window t) (:foreground "blue")))
         (list
          'face-value (get-text-property 1 'face)
          'facep (facep (get-text-property 1 'face))
          'contains-filtered (memq ':filtered (get-text-property 1 'face))))
     (error 'test-failed))
   (condition-case nil (face-filters) (error 'no-face-filters))))"##,
    );
}

#[test]
fn ft_graviton_face_font_lock_defaults_vs_actual_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (font-lock-mode 1)
    (font-lock-set-defaults)
    (insert "(defun defaults-test () 42)\n")
    (font-lock-fontify-buffer)
    (list
     'defaults (condition-case nil (font-lock-defaults) (error 'no))
     'keywords (if (boundp 'font-lock-keywords) font-lock-keywords 'no)
     'face-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     'face-test (save-excursion (goto-char (point-min)) (search-forward "test") (get-text-property (match-beginning 0) 'face))
     'face-42 (save-excursion (goto-char (point-min)) (search-forward "42") (get-text-property (match-beginning 0) 'face))
     'fontified (get-text-property 1 'fontified)))))"##,
    );
}

#[test]
fn ft_graviton_face_overlay_line_spacing_inherit_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay line spacing and face inherit test content text data here now end done final")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow" :inherit bold))
      (overlay-put ov 'line-spacing 8)
      (overlay-put ov 'line-height 1.8)
      (list
       'face (overlay-get ov 'face)
       'line-spacing (overlay-get ov 'line-spacing)
       'line-height (overlay-get ov 'line-height)
       'face-at-20 (get-char-property 20 'face)
       'face-at-5 (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_graviton_face_set_text_properties_on_range_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Set text properties on range face test buffer content text data")
    (put-text-property 1 20 'face 'bold)
    (put-text-property 20 40 'face 'italic)
    (put-text-property 40 55 'face 'underline)
    (list
     'initial-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 10 20 30 40 50 54))
     ;; Override middle section entirely
     'set-text-props-middle (progn (set-text-properties 15 45 (list 'face '(:foreground "red" :weight bold) 'new-prop 'value)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'new-prop))) '(1 10 15 25 35 45 50 54)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_graviton_font_lock_keywords_with_subgroup_0_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "Subgroup 0 match test for font lock face buffer content")
    (font-lock-add-keywords nil '(("\\<\\(Subgroup\\)\\>" 0 font-lock-warning-face t)))
    (font-lock-fontify-buffer)
    (list
     'match-0-face (save-excursion (goto-char (point-min)) (search-forward "Subgroup") (get-text-property (match-beginning 0) 'face))
     'other-word-face (save-excursion (goto-char (point-min)) (search-forward "match") (get-text-property (match-beginning 0) 'face)))
    (font-lock-remove-keywords nil '(("\\<\\(Subgroup\\)\\>" 0 font-lock-warning-face t)))
    'cleaned))))"##,
    );
}

#[test]
fn ft_graviton_face_overlay_before_after_face_inherit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before after face inherit overlay test content text data buffer here done")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow" :inherit italic))
      (overlay-put ov 'before-string
                   (propertize "[[BEFORE]]" 'face '(:foreground "red" :inherit bold)))
      (overlay-put ov 'after-string
                   (propertize "{{AFTER}}" 'face '(:foreground "blue" :inherit underline))))
    (list
     'overlay-face (overlay-get ov 'face)
     'before-face (get-text-property 0 (overlay-get ov 'before-string))
     'after-face (get-text-property 0 (overlay-get ov 'after-string))
     'before-props (text-properties-at 0 (overlay-get ov 'before-string))
     'after-props (text-properties-at 0 (overlay-get ov 'after-string))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_graviton_face_overlay_with_face_change_and_move_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30)))))
        (let ((v0 (funcall snap)))
          (move-overlay ov 20 25)
          (overlay-put ov 'face '(:foreground "red" :weight bold))
          (overlay-put ov 'priority 100)
          (let ((v1 (funcall snap)))
            (move-overlay ov 6 15)
            (overlay-put ov 'face '(:background "cyan" :slant italic))
            (let ((v2 (funcall snap)))
              (delete-overlay ov)
              (list v0 v1 v2))))))))"##,
    );
}

#[test]
fn ft_graviton_font_lock_fontify_with_lazy_lock_no_jit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (condition-case nil
      (progn
        (setq jit-lock-mode nil)
        (with-temp-buffer
          (emacs-lisp-mode)
          (insert "(defun no-jit-test () 42)\n")
          (font-lock-fontify-buffer)
          (list
           'jit-lock-mode jit-lock-mode
           'face-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
           'fontified (get-text-property 1 'fontified)
           'font-lock-support-mode (if (boundp 'font-lock-support-mode) font-lock-support-mode 'no)))))
    (error (list 'error 'no (fboundp 'jit-lock-mode) (fboundp 'emacs-lisp-mode))))))"##,
    );
}

#[test]
fn ft_graviton_face_text_properties_after_concat_of_propertized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let* ((s1 (propertize "BOLD" 'face 'bold))
         (s2 (propertize "ITALIC" 'face 'italic))
         (s3 (concat s1 " " s2))
         (s4 (propertize "RED" 'face '(:foreground "red")))
         (s5 (concat s3 " - " s4)))
    (list
     's1-face (get-text-property 0 'face s1)
     's1-props (text-properties-at 0 s1)
     's2-face (get-text-property 0 'face s2)
     's3-face-at-0 (get-text-property 0 'face s3)
     's3-face-at-5 (get-text-property 5 'face s3)
     's4-face (get-text-property 0 'face s4)
     's5-face-at-0 (get-text-property 0 'face s5)
     's5-face-at-8 (get-text-property 8 'face s5)
     's5-face-at-12 (get-text-property 12 'face s5)
     's5-length (length s5))))"##,
    );
}

#[test]
fn ft_boson_face_overlay_with_multiple_faces_and_priority_stacking() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multiple overlays face priority stacking test content text data here done")
    (let ((ov1 (make-overlay 5 20))) (overlay-put ov1 'face '(:background "red" :foreground "white")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 15 25))) (overlay-put ov3 'face '(:foreground "blue" :weight bold :slant italic)) (overlay-put ov3 'priority 30))
    (let ((ov4 (make-overlay 20 40))) (overlay-put ov4 'face '(:underline t)) (overlay-put ov4 'priority 15))
    (list
     'all-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (mapcar (lambda (ov) (overlay-get ov 'priority)) (overlays-at pos)))) '(1 5 10 18 22 28 35 42 55))
     'max-priority-at-18 (let ((ovs (overlays-at 18))) (apply #'max (mapcar (lambda (ov) (or (overlay-get ov 'priority) 0)) ovs)))
     (progn (mapc #'delete-overlay (overlays-in 1 55)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_boson_font_lock_fontify_with_rx_regexp_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (condition-case nil
      (progn
        (require 'rx)
        (with-temp-buffer
          (fundamental-mode)
          (font-lock-mode 1)
          (insert "RX-based matching: FOO and BAR and BAZ keywords here")
          (font-lock-add-keywords nil (list (list (rx word-start (or "FOO" "BAR" "BAZ") word-end) 0 font-lock-warning-face t)))
          (font-lock-fontify-buffer)
          (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (list n (get-text-property (match-beginning 0) 'face)))) '("FOO" "BAR" "BAZ"))))
    (error (list 'rx-error (fboundp 'rx) (fboundp 'font-lock-add-keywords))))))"##,
    );
}

#[test]
fn ft_boson_face_inherit_chain_resolve_attributes_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-inherit-A-face) (error nil))
  (condition-case nil (copy-face 'my-inherit-A-face 'my-inherit-B-face) (error nil))
  (condition-case nil (copy-face 'my-inherit-B-face 'my-inherit-C-face) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-A-face nil :weight 'bold :foreground "red") (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-B-face nil :slant 'italic :inherit 'my-inherit-A-face) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-C-face nil :underline t :inherit 'my-inherit-B-face) (error nil))
  (condition-case nil (copy-face 'my-inherit-C-face 'my-inherit-D-face) (error nil))
  (condition-case nil (set-face-attribute 'my-inherit-D-face nil :box t :inherit 'my-inherit-C-face) (error nil))
  (list
   'A-weight (face-attribute 'my-inherit-A-face :weight nil 'default-on)
   'B-weight (face-attribute 'my-inherit-B-face :weight nil 'default-on)
   'B-slant (face-attribute 'my-inherit-B-face :slant nil 'default-on)
   'C-weight (face-attribute 'my-inherit-C-face :weight nil 'default-on)
   'C-under (condition-case nil (face-attribute 'my-inherit-C-face :underline nil 'default-on) (error 'no))
   'D-weight (face-attribute 'my-inherit-D-face :weight nil 'default-on)
   'D-box (condition-case nil (face-attribute 'my-inherit-D-face :box nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_boson_face_overlay_hidden_via_invisible_face_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Visible text INVISIBLE here visible again final end now done")
    (put-text-property 1 14 'face 'bold)
    (put-text-property 14 23 'face 'italic :invisible t)
    (put-text-property 23 55 'face 'underline)
    (let ((ov (make-overlay 25 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'invisible t))
    (list
     'with-invisible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-char-property pos 'face) (invisible-p pos))) '(1 5 14 18 23 25 30 40 45 54))
     'remove-invisible-text (progn (remove-text-properties 14 23 '(invisible nil)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (invisible-p pos))) '(1 5 14 18 23 30 40 54)))
     'remove-invisible-ov (progn (overlay-put ov 'invisible nil) (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(25 30 40 45)))
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_boson_font_lock_fontify_without_font_lock_mode_enabled() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    ;; Deliberately don't enable font-lock-mode
    (insert "No font lock mode enabled buffer content text data here")
    (list
     'font-lock-mode-before font-lock-mode
     'fontify-buffer-direct (condition-case nil (progn (font-lock-fontify-buffer) 'fontified) (error 'no-fontify))
     'fontified-after (get-text-property 1 'fontified)
     'face-after (get-text-property 1 'face)
     'font-lock-mode-after font-lock-mode))))"##,
    );
}

#[test]
fn ft_boson_face_set_and_get_text_properties_at_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (dotimes (i 10)
      (put-text-property (1+ (* i 4)) (+ (* i 4) 5) 'face
                         (if (evenp i) 'bold 'italic)))
    (list
     'all-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 4 5 8 9 12 13 16 17 20 21 24 25 28 29 32 33 36 37 40))
     'prop-boundaries (mapcar (lambda (pos) (next-single-property-change pos 'face nil 41)) '(1 5 9 13 17 21 25 29 33 37))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_boson_face_overlay_create_delete_create_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay create delete create cycle face test content text data buffer content")
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 20 30 40 50 60)))))
      (let ((v0 (funcall snap)))
        (let ((ov (make-overlay 15 35))) (overlay-put ov 'face '(:background "yellow")))
        (let ((v1 (funcall snap)))
          (mapc #'delete-overlay (overlays-at 25))
          (let ((v2 (funcall snap)))
            (let ((ov2 (make-overlay 20 40))) (overlay-put ov2 'face '(:foreground "red" :weight bold)))
            (let ((v3 (funcall snap)))
              (mapc #'delete-overlay (overlays-at 30))
              (let ((v4 (funcall snap)))
                (let ((ov3 (make-overlay 10 50))) (overlay-put ov3 'face '(:underline t :slant italic)))
                (let ((v5 (funcall snap)))
                  (mapc #'delete-overlay (overlays-in 1 60))
                  (list v0 v1 v2 v3 v4 v5)))))))))))"##,
    );
}

#[test]
fn ft_boson_face_default_face_attributes_all_checked() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-family (face-attribute 'default :family nil 'default-on)
   'default-foundry (face-attribute 'default :foundry nil 'default-on)
   'default-width (face-attribute 'default :width nil 'default-on)
   'default-height (face-attribute 'default :height nil 'default-on)
   'default-weight (face-attribute 'default :weight nil 'default-on)
   'default-slant (face-attribute 'default :slant nil 'default-on)
   'default-underline (condition-case nil (face-attribute 'default :underline nil 'default-on) (error 'no))
   'default-overline (condition-case nil (face-attribute 'default :overline nil 'default-on) (error 'no))
   'default-strike (condition-case nil (face-attribute 'default :strike-through nil 'default-on) (error 'no))
   'default-box (condition-case nil (face-attribute 'default :box nil 'default-on) (error 'no))
   'default-inverse (condition-case nil (face-attribute 'default :inverse-video nil 'default-on) (error 'no))
   'default-fg (condition-case nil (face-foreground 'default nil 'default-on) (error 'no))
   'default-bg (condition-case nil (face-background 'default nil 'default-on) (error 'no))
   'default-font (condition-case nil (face-font 'default nil) (error 'no))
   'default-extend (condition-case nil (face-attribute 'default :extend nil 'default-on) (error 'no))
   'default-inherit (condition-case nil (face-attribute 'default :inherit nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_fermion_face_font_lock_ensure_full_vs_partial_fontify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun full-partial-test (a b c) (+ a b c))\n")
    (list
     'partial-first-half (progn (font-lock-fontify-region 1 20) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 25 35 38)))
     'ensure-full (progn (font-lock-ensure (point-min) (point-max)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 10 20 25 35 38)))
     'consistent (progn (font-lock-unfontify-buffer) (font-lock-fontify-buffer) (let ((fb (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 10 25 35 38)))) (font-lock-unfontify-buffer) (font-lock-ensure (point-min) (point-max)) (equal fb (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 10 25 35 38)))))))))"##,
    );
}

#[test]
fn ft_fermion_face_overlay_multiple_priority_interleave_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHH")
    (let ((ov1 (make-overlay 6 15))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 30))
    (let ((ov2 (make-overlay 10 25))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 40))
    (let ((ov3 (make-overlay 20 35))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 10))
    (let ((ov4 (make-overlay 30 40))) (overlay-put ov4 'face '(:foreground "orange")) (overlay-put ov4 'priority 50))
    (list
     'before-flip (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30 35 40 45))
     'flip-priorities (progn (overlay-put ov1 'priority 50) (overlay-put ov3 'priority 60) (overlay-put ov2 'priority 5) (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30 35 40 45)))
     (progn (mapc #'delete-overlay (overlays-in 1 45)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_fermion_face_property_search_with_limit_advanced_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAABBBCCCDDDEEEFFFGGGHHHIIIJJJKKKLLLMMMNNNOOOPPPQQQRRRSSSTTTUUUVVVWWWXXXYYYZZZ")
    (put-text-property 1 4 'face 'bold :tag 'first)
    (put-text-property 4 7 'face 'italic :tag 'second)
    (put-text-property 10 13 'face 'underline :tag 'third)
    (put-text-property 34 37 'face '(:foreground "red") :tag 'fourth)
    (put-text-property 55 58 'face '(:background "yellow") :tag 'fifth)
    (put-text-property 73 76 'face '(:foreground "blue") :tag 'sixth)
    (list
     'find-bold (text-property-any 1 76 'face 'bold)
     'find-italic (text-property-any 1 76 'face 'italic)
     'find-underline-limited (text-property-any 1 20 'face 'underline)
     'find-underline-full (text-property-any 1 76 'face 'underline)
     'find-red (text-property-any 1 76 'face '(:foreground "red"))
     'find-yellow (text-property-any 1 76 'face '(:background "yellow"))
     'find-blue (text-property-any 1 76 'face '(:foreground "blue"))
     'find-nil (text-property-any 1 76 'face nil)
     'find-none (text-property-any 1 76 'face 'nonexistent)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_fermion_face_font_lock_keywords_add_remove_cycles_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "CYCLE-A CYCLE-B CYCLE-C font lock keyword cycle test end now")
    (let ((results nil))
      (font-lock-add-keywords nil '(("\\<\\(CYCLE-A\\)\\>" 1 '(:foreground "red") t)))
      (font-lock-fontify-buffer)
      (push (list 'after-add-A (save-excursion (goto-char (point-min)) (search-forward "CYCLE-A") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-add-keywords nil '(("\\<\\(CYCLE-B\\)\\>" 1 '(:foreground "green") t)))
      (font-lock-fontify-buffer)
      (push (list 'after-add-B (save-excursion (goto-char (point-min)) (search-forward "CYCLE-B") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-add-keywords nil '(("\\<\\(CYCLE-C\\)\\>" 1 '(:foreground "blue") t)))
      (font-lock-fontify-buffer)
      (push (list 'after-add-C (save-excursion (goto-char (point-min)) (search-forward "CYCLE-C") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-remove-keywords nil '(("\\<\\(CYCLE-A\\)\\>" 1 '(:foreground "red") t)))
      (font-lock-fontify-buffer)
      (push (list 'after-remove-A (save-excursion (goto-char (point-min)) (search-forward "CYCLE-A") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-remove-keywords nil '(("\\<\\(CYCLE-B\\)\\>" 1 '(:foreground "green") t) ("\\<\\(CYCLE-C\\)\\>" 1 '(:foreground "blue") t)))
      (font-lock-fontify-buffer)
      (push (list 'after-remove-all (save-excursion (goto-char (point-min)) (search-forward "CYCLE-B") (get-text-property (match-beginning 0) 'face))) results)
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_fermion_face_overlay_string_face_inheritance_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string face inheritance chain test content text data buffer here now done")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow" :inherit bold))
      (overlay-put ov 'before-string (propertize "[[BEFORE-INHERIT]]" 'face '(:foreground "red" :inherit italic :weight extra-bold)))
      (list
       'overlay-face (overlay-get ov 'face)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-face-attrs (length (get-text-property 0 (overlay-get ov 'before-string)))
       'before-str-length (length (overlay-get ov 'before-string))
       'overlay-props (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_fermion_face_font_lock_inhibit_fontification_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun inhibit-test () 42)\n")
    (let ((font-lock-mode nil))
      (list
       'before-fontify (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20))
       'inhibit-font-lock-bound (boundp 'inhibit-font-lock)
       'inhibit-font-lock-val (if (boundp 'inhibit-font-lock) inhibit-font-lock 'no-bound)
       'fontify-anyway (progn (font-lock-fontify-buffer) (get-text-property 1 'fontified))
       'face-after (get-text-property 1 'face)))))"##,
    );
}

#[test]
fn ft_fermion_face_with_buffer_unibyte_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Unibyte multibyte face comparison test αβγδε text here")
    (put-text-property 1 20 'face 'bold)
    (put-text-property 20 32 'face 'italic)
    (put-text-property 32 48 'face 'underline)
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (char-width (or (char-after pos) 0)))) '(1 10 20 25 32 40 47))
     'multibyte-p (multibyte-string-p (buffer-string))
     'buffer-string-length (length (buffer-string))
     'narrow-to-region (progn (narrow-to-region 1 25) (multibyte-string-p (buffer-string)))
     (widen)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_fermion_face_overlay_make_multiple_delete_all_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Multiple overlay creation and deletion face test content text data here now end final done")
    (let ((ov1 (make-overlay 1 15))) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 20 35))) (overlay-put ov2 'face '(:background "green")))
    (let ((ov3 (make-overlay 40 60))) (overlay-put ov3 'face '(:background "blue")))
    (let ((ov4 (make-overlay 50 70))) (overlay-put ov4 'face '(:foreground "orange" :weight bold)))
    (list
     'before-delete (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 10 15 20 25 35 40 50 60 70))
     'delete-ov1-ov2 (progn (delete-overlay ov1) (delete-overlay ov2) (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 20 30 40 50 60 70)))
     'delete-all (progn (mapc #'delete-overlay (overlays-in 1 70)) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 10 20 40 50 60 70)))))))"##,
    );
}

#[test]
fn ft_gluon_face_display_table_and_face_combined_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Display table with face test buffer content text data here now")
    (put-text-property 1 54 'face '(:foreground "blue"))
    (condition-case nil
        (let ((dt (make-display-table)))
          (set-display-table-slot dt 0 (make-glyph-code ?a 'bold))
          (list
           'display-table-created 'ok
           'face-still-present (get-text-property 1 'face)
           'slot-0 (display-table-slot dt 0)
           'glyph-code-p (glyphp (car (display-table-slot dt 0)))))
      (error (list 'dt-error (get-text-property 1 'face) (fboundp 'make-display-table) (fboundp 'set-display-table-slot)))))))"##,
    );
}

#[test]
fn ft_gluon_face_text_property_interval_merge_after_delete() {
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
    (put-text-property 26 31 'face '(:foreground "blue"))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 10 11 15 16 20 21 25 26 30)))))
      (let ((v0 (funcall snap)))
        ;; Delete the italic region entirely
        (delete-region 6 11)
        (let ((v1 (funcall snap)))
          ;; Delete the red region partially
          (delete-region 10 18)
          (let ((v2 (funcall snap)))
            ;; Delete remaining
            (delete-region 1 (point-max))
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3))))))))"##,
    );
}

#[test]
fn ft_gluon_face_font_lock_fontify_unfontify_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t) (org-fontify-done-headline t))
      (org-mode)
      (insert "* TODO Region unfontify\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        (font-lock-unfontify-region 1 25)
        (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
          (font-lock-fontify-region 1 25)
          (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
            (list v0 v1 v2))))))))"##,
    );
}

#[test]
fn ft_gluon_face_overlay_window_specific_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay window specific face property test content text data buffer here done")
    (let ((ov-all (make-overlay 1 25))) (overlay-put ov-all 'face '(:background "yellow")))
    (let ((ov-win (make-overlay 25 50))) (overlay-put ov-win 'face '(:background "cyan")) (overlay-put ov-win 'window (selected-window)))
    (let ((ov-nil-win (make-overlay 50 70))) (overlay-put ov-nil-win 'face '(:background "magenta")) (overlay-put ov-nil-win 'window nil))
    (list
     'face-all (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 15 25 30 40 50 55 60 69))
     'ov-all-window (overlay-get ov-all 'window)
     'ov-win-window (overlay-get ov-win 'window)
     'ov-nil-window (overlay-get ov-nil-win 'window)
     (progn (mapc #'delete-overlay (overlays-in 1 70)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_gluon_face_set_face_attribute_with_eieio_interaction() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'eieio)
  (list
   'facep-bold (facep 'bold)
   'face-attribute-bold-weight (face-attribute 'bold :weight nil 'default-on)
   (condition-case nil
       (progn
         (defclass ft-face-test () ((face-weight :initarg :weight :initform 'bold) (face-used :initform nil)))
         (let ((obj (ft-face-test)))
           (list
            'obj-weight (eieio-oref obj :weight)
            'eieio-oref-fbound (fboundp 'eieio-oref)
            'face-still-bold (facep 'bold)
            'face-attr-after-eieio (face-attribute 'bold :weight nil 'default-on))))
     (error (list 'eieio-error (fboundp 'eieio-oref) (facep 'bold)))))))"##,
    );
}

#[test]
fn ft_gluon_face_overlay_with_item_property_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay with item property face test content text data buffer content")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string "> ")
      (overlay-put ov 'display '(margin left-margin) "NOTE"))
    (list
     'face (overlay-get ov 'face)
     'before-string (overlay-get ov 'before-string)
     'display (overlay-get ov 'display)
     'face-at-ov (get-char-property 25 'face)
     (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_gluon_font_lock_fontify_with_mode_specific_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (text-mode)
    (font-lock-mode 1)
    (insert "Text mode font lock keyword face test buffer content data here end")
    (font-lock-fontify-buffer)
    (list
     'font-lock-mode font-lock-mode
     'fontified (get-text-property 1 'fontified)
     'face-at-1 (get-text-property 1 'face)
     'face-at-20 (get-text-property 20 'face)
     'face-at-40 (get-text-property 40 'face)
     'mode-name major-mode))))"##,
    );
}

#[test]
fn ft_gluon_face_text_property_cursor_sensor_and_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Cursor sensor and face text property test buffer content data text")
    (put-text-property 1 20 'face 'bold)
    (put-text-property 20 40 'face 'italic)
    (put-text-property 20 40 'cursor-sensor-functions (list 'ignore))
    (put-text-property 40 58 'face 'underline)
    (list
     'face-and-sensor (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'cursor-sensor-functions))) '(1 10 20 30 40 50 57))
     'sensor-at-20 (get-text-property 20 'cursor-sensor-functions)
     'sensor-at-1 (get-text-property 1 'cursor-sensor-functions)
     'sensor-functions-fbound (fboundp 'cursor-sensor-functions))))"##,
    );
}

#[test]
fn ft_higgs_face_text_property_any_and_not_all_combined_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHHIIIIIJJJJJ")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face 'bold)
    (put-text-property 21 26 'face 'italic)
    (put-text-property 26 31 'face 'underline)
    (put-text-property 31 36 'face 'bold)
    (put-text-property 36 41 'face 'italic)
    (put-text-property 41 46 'face 'underline)
    (put-text-property 46 51 'face '(:foreground "red"))
    (list
     'any-bold (text-property-any 1 51 'face 'bold)
     'any-italic (text-property-any 1 51 'face 'italic)
     'not-all-bold (text-property-not-all 1 51 'face 'bold)
     'any-red-first (text-property-any 1 51 'face '(:foreground "red"))
     'not-all-red (text-property-not-all 1 51 'face '(:foreground "red"))
     'any-nonexistent (text-property-any 1 51 'face 'none)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_higgs_font_lock_fontify_with_font_lock_maximum_decoration_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'max-dec-bound (boundp 'font-lock-maximum-decoration)
   'max-dec-value (if (boundp 'font-lock-maximum-decoration) font-lock-maximum-decoration 'no-bound)
   (if (boundp 'font-lock-maximum-decoration)
       (condition-case nil
           (let ((font-lock-maximum-decoration t))
             (with-temp-buffer
               (emacs-lisp-mode)
               (insert "(defun max-dec-test (x) (* x x))\n")
               (font-lock-fontify-buffer)
               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 15 20 26 30))))
         (error 'max-dec-test-failed))
     'no-max-dec-bound)
   (if (boundp 'font-lock-maximum-decoration)
       (condition-case nil
           (let ((font-lock-maximum-decoration nil))
             (with-temp-buffer
               (emacs-lisp-mode)
               (insert "(defun no-max-dec-test (x) (* x x))\n")
               (font-lock-fontify-buffer)
               (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 15 20 26 30))))
         (error 'no-max-dec-test-failed))
     'no-max-dec-bound-2))))"##,
    );
}

#[test]
fn ft_higgs_face_overlay_before_after_string_with_complex_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before after string complex overlay props face test content data buffer")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow" :inherit bold))
      (overlay-put ov 'before-string (propertize "[[BUFFER]]" 'face '(:foreground "red" :weight bold :slant italic :underline t :height 1.2) 'key1 'val1 'key2 'val2))
      (overlay-put ov 'after-string (propertize "{{AFTER}}" 'face '(:foreground "blue" :background "white" :overline t :width condensed) 'key3 'val3))
      (list
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-extra-props (length (text-properties-at 0 (overlay-get ov 'before-string)))
       'after-face (get-text-property 0 (overlay-get ov 'after-string))
       'after-extra-props (length (text-properties-at 0 (overlay-get ov 'after-string)))
       'overlay-props (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_higgs_face_font_lock_default_unfontify_buffer_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-default-unfontify-buffer-fbound (fboundp 'font-lock-default-unfontify-buffer)
   'font-lock-default-unfontify-region-fbound (fboundp 'font-lock-default-unfontify-region)
   'font-lock-default-fontify-buffer-fbound (fboundp 'font-lock-default-fontify-buffer)
   'font-lock-default-fontify-region-fbound (fboundp 'font-lock-default-fontify-region)
   (condition-case nil
       (with-temp-buffer
         (fundamental-mode)
         (font-lock-mode 1)
         (insert "Default unfontify buffer face test content text here")
         (font-lock-fontify-buffer)
         (let ((v0 (get-text-property 1 'fontified)))
           (font-lock-unfontify-buffer)
           (list 'before v0 'after (get-text-property 1 'fontified))))
     (error 'no-default-unfontify))
   (condition-case nil
       (with-temp-buffer
         (fundamental-mode)
         (font-lock-mode 1)
         (insert "Default fontify buffer face test content text here")
         (font-lock-default-fontify-buffer)
         (list 'fontified-after-default (get-text-property 1 'fontified)))
     (error 'no-default-fontify)))))"##,
    );
}

#[test]
fn ft_higgs_face_overlay_put_then_get_then_put_again_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay put get put cycle face test content text data buffer here now")
    (let ((ov (make-overlay 10 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (let ((v0 (overlay-get ov 'face)))
        (overlay-put ov 'face '(:foreground "red" :weight bold))
        (let ((v1 (overlay-get ov 'face)))
          (overlay-put ov 'priority 50)
          (overlay-put ov 'face '(:foreground "blue" :slant italic :underline t))
          (let ((v2 (overlay-get ov 'face)))
            (list v0 v1 v2 (overlay-get ov 'priority) (progn (delete-overlay ov) 'cleaned)))))))))"##,
    );
}

#[test]
fn ft_higgs_face_property_text_property_any_with_complex_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEE")
    (put-text-property 1 5 'face '(:foreground "red"))
    (put-text-property 5 9 'face '(:foreground "red" :weight bold))
    (put-text-property 9 13 'face '(:foreground "red"))
    (put-text-property 13 17 'face '(:foreground "green"))
    (put-text-property 17 21 'face '(:foreground "red" :weight bold :slant italic))
    (list
     'find-plain-red (text-property-any 1 21 'face '(:foreground "red"))
     'find-bold-red (text-property-any 1 21 'face '(:foreground "red" :weight bold))
     'find-complex-red (text-property-any 1 21 'face '(:foreground "red" :weight bold :slant italic))
     'find-green (text-property-any 1 21 'face '(:foreground "green"))
     'not-all-bold-red (text-property-not-all 1 21 'face '(:foreground "red" :weight bold)))))"##,
    );
}

#[test]
fn ft_higgs_font_lock_add_keywords_prepend_append_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "PREPEND keyword APPEND keyword test prepend append order check end")
    (font-lock-add-keywords nil '(("\\<\\(PREPEND\\)\\>" 1 '(:foreground "blue") prepend) ("\\<\\(APPEND\\)\\>" 1 '(:foreground "green") append)))
    (font-lock-fontify-buffer)
    (list
     'prepend-face (save-excursion (goto-char (point-min)) (search-forward "PREPEND") (get-text-property (match-beginning 0) 'face))
     'append-face (save-excursion (goto-char (point-min)) (search-forward "APPEND") (get-text-property (match-beginning 0) 'face))
     'non-keyword-face (save-excursion (goto-char (point-min)) (search-forward "keyword") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_higgs_face_remap_add_relative_to_inherited_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'italic-weight-before (face-attribute 'italic :weight nil 'default-on)
   'add-relative-to-italic (condition-case nil (progn (face-remap-add-relative 'italic '(:weight bold :foreground "red")) 'ok) (error 'no))
   'italic-weight-after (condition-case nil (face-attribute 'italic :weight nil 'default-on) (error 'no))
   'remap-alist (face-remapping-alist)
   (condition-case nil (progn (face-remap-reset-base 'italic) 'reset) (error 'no)))))"##,
    );
}

#[test]
fn ft_tachyon_face_text_property_interval_split_and_merge_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (put-text-property 1 6 'face 'bold :tag 'first)
    (put-text-property 6 11 'face 'italic :tag 'second)
    (put-text-property 11 16 'face 'underline :tag 'third)
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'tag))) '(1 3 5 6 8 10 11 13 15)))))
      (let ((v0 (funcall snap)))
        (goto-char 6) (insert "SPLIT")
        (let ((v1 (funcall snap)))
          (delete-region 8 14)
          (let ((v2 (funcall snap)))
            (goto-char 5) (insert "MERGE")
            (let ((v3 (funcall snap)))
              (list v0 v1 v2 v3 (length (object-intervals (current-buffer)))))))))))"##,
    );
}

#[test]
fn ft_tachyon_font_lock_fontify_with_empty_keywords_list_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (setq font-lock-keywords nil)
    (insert "Empty keywords font lock test buffer content text data here end")
    (font-lock-fontify-buffer)
    (list
     'font-lock-keywords font-lock-keywords
     'fontified (get-text-property 1 'fontified)
     'face (get-text-property 1 'face)
     'fontified-at-end (get-text-property 40 'fontified)
     'font-lock-maximum-decoration (if (boundp 'font-lock-maximum-decoration) font-lock-maximum-decoration 'no))))))"##,
    );
}

#[test]
fn ft_tachyon_face_overlay_with_display_invisible_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'invisible t)
      (overlay-put ov 'display "")
      (list
       'face (overlay-get ov 'face)
       'invisible (overlay-get ov 'invisible)
       'display (overlay-get ov 'display)
       'char-prop-10 (get-char-property 10 'face)
       'char-prop-25 (get-char-property 25 'face)
       (progn (overlay-put ov 'invisible nil) (get-char-property 10 'face))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_tachyon_face_overlay_with_priority_and_face_new_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov1 (make-overlay 1 15))) (overlay-put ov1 'face '(:background "red" :foreground "white")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 8 22))) (overlay-put ov2 'face '(:foreground "green" :weight bold)) (overlay-put ov2 'priority 30))
    (let ((ov3 (make-overlay 18 30))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 20))
    (list
     'face-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 10 15 20 25 30 35))
     'all-priorities (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov) (overlay-get ov 'priority) (overlay-get ov 'face))) (list ov1 ov2 ov3))
     (progn (mapc #'delete-overlay (overlays-in 1 35)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_tachyon_face_font_lock_double_fontify_consistency_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun double-fontify (x) (+ x 1))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 28 33))))
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 28 33))))
        (list v0 v1 (equal v0 v1)))))))"##,
    );
}

#[test]
fn ft_tachyon_face_property_nil_value_interval_handling_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face nil)
    (put-text-property 11 16 'face 'italic)
    (put-text-property 16 21 'face nil)
    (put-text-property 21 26 'face 'underline)
    (list
     'faces-with-nil (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 6 8 11 13 16 18 21 23 25))
     'find-bold (text-property-any 1 26 'face 'bold)
     'find-nil (text-property-any 1 26 'face nil)
     'find-italic (text-property-any 1 26 'face 'italic)
     'find-underline (text-property-any 1 26 'face 'underline)
     'not-all-nil (text-property-not-all 1 26 'face nil)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_tachyon_face_overlay_category_face_inherit_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov1 (make-overlay 1 15))) (overlay-put ov1 'category 'cat-red) (overlay-put ov1 'face '(:backward "red" :inherit bold)))
    (let ((ov2 (make-overlay 10 25))) (overlay-put ov2 'category 'cat-green) (overlay-put ov2 'face '(:foreground "green" :slant italic)))
    (let ((ov3 (make-overlay 20 30))) (overlay-put ov3 'category 'cat-blue) (overlay-put ov3 'face '(:underline t :inherit italic)))
    (list
     'cat-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 5 10 15 20 25 30))
     'ov1-face (overlay-get ov1 'face)
     'ov2-face (overlay-get ov2 'face)
     'ov3-face (overlay-get ov3 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 30)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_tachyon_face_set_face_properties_all_combined_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-all-combined-face) (error nil))
  (condition-case nil (set-face-attribute 'my-all-combined-face nil :weight 'bold :slant 'italic :underline '(:color "red" :style wave) :overline t :strike-through t :box '(:line-width 2 :color "blue") :inverse-video t :foreground "dark green" :background "light yellow" :height 140 :width 'semi-condensed :extend t :line-spacing 5) (error nil))
  (list
   'weight (face-attribute 'my-all-combined-face :weight nil 'default-on)
   'slant (face-attribute 'my-all-combined-face :slant nil 'default-on)
   'underline (face-attribute 'my-all-combined-face :underline nil 'default-on)
   'overline (face-attribute 'my-all-combined-face :overline nil 'default-on)
   'strike (face-attribute 'my-all-combined-face :strike-through nil 'default-on)
   'box (face-attribute 'my-all-combined-face :box nil 'default-on)
   'inverse (face-attribute 'my-all-combined-face :inverse-video nil 'default-on)
   'fg (condition-case nil (face-foreground 'my-all-combined-face nil 'default-on) (error 'no))
   'bg (condition-case nil (face-background 'my-all-combined-face nil 'default-on) (error 'no))
   'height (face-attribute 'my-all-combined-face :height nil 'default-on)
   'width (face-attribute 'my-all-combined-face :width nil 'default-on)
   'extend (face-attribute 'my-all-combined-face :extend nil 'default-on)
   'line-spacing (face-attribute 'my-all-combined-face :line-spacing nil 'default-on)
   (condition-case nil (progn (set-face-attribute 'my-all-combined-face nil :weight 'unspecified :slant 'unspecified :underline 'unspecified :overline 'unspecified :strike-through 'unspecified :box 'unspecified :inverse-video 'unspecified :foreground 'unspecified :background 'unspecified :height 'unspecified :width 'unspecified :extend 'unspecified :line-spacing 'unspecified) 'reset-done) (error 'no)))))"##,
    );
}

#[test]
fn ft_wave_face_text_property_set_on_empty_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (list
     'empty-face (get-text-property 1 'face)
     'empty-intervals (length (object-intervals (current-buffer)))
     (progn (insert "Now with text") (put-text-property 1 13 'face 'bold) (list 'after-insert-face (get-text-property 1 'face) 'intervals (length (object-intervals (current-buffer)))))
     (progn (erase-buffer) (list 'after-erase-face (get-text-property 1 'face) 'intervals (length (object-intervals (current-buffer)))))
     (progn (insert "Again") (put-text-property 1 6 'face 'italic) (list 'final-face (get-text-property 1 'face) 'intervals (length (object-intervals (current-buffer))))))))"##,
    );
}

#[test]
fn ft_wave_font_lock_fontify_two_buffers_independently_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (let ((buf1 (generate-new-buffer "*ft-wave-1*"))
        (buf2 (generate-new-buffer "*ft-wave-2*")))
    (unwind-protect
        (progn
          (with-current-buffer buf1 (emacs-lisp-mode) (insert "(defun buf1-test () 111)\n") (font-lock-fontify-buffer))
          (with-current-buffer buf2 (emacs-lisp-mode) (insert "(defun buf2-test () 222)\n") (font-lock-fontify-buffer))
          (list
           'buf1-faces (with-current-buffer buf1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 15 20 22)))
           'buf2-faces (with-current-buffer buf2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 15 20 22))))
           'buf1-fontified (with-current-buffer buf1 (get-text-property 1 'fontified))
           'buf2-fontified (with-current-buffer buf2 (get-text-property 1 'fontified))))
      (kill-buffer buf1) (kill-buffer buf2))))"##,
    );
}

#[test]
fn ft_wave_face_overlay_start_end_equal_face_test() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Start end equal overlay face test content text data buffer")
    (let ((ov (make-overlay 10 10)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'empty-start (overlay-start ov)
       'empty-end (overlay-end ov)
       'face-get (overlay-get ov 'face)
       'face-at-9 (get-char-property 9 'face)
       'face-at-10 (get-char-property 10 'face)
       'face-at-11 (get-char-property 11 'face)
       (progn (goto-char 10) (insert "FILL") (list 'after-fill-face-10 (get-char-property 10 'face) 'face-12 (get-char-property 12 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_wave_face_font_lock_keywords_priority_resolution_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "PRIORITY RESOLUTION keyword test for font lock face buffer content")
    (font-lock-add-keywords nil '(("\\<\\(PRIORITY\\)\\>" 1 '(:foreground "red") t) ("\\<\\(PRIORITY\\)\\>" 1 '(:foreground "green") t) ("\\<\\(PRIORITY\\)\\>" 1 '(:foreground "blue") overwrite)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "PRIORITY") (get-text-property (match-beginning 0) 'face))))
      (font-lock-remove-keywords nil '(("\\<\\(PRIORITY\\)\\>" 1 '(:foreground "red") t) ("\\<\\(PRIORITY\\)\\>" 1 '(:foreground "green") t) ("\\<\\(PRIORITY\\)\\>" 1 '(:foreground "blue") overwrite)))
      v0))))"##,
    );
}

#[test]
fn ft_wave_face_remap_reset_base_multiple_faces_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'alist-before (face-remapping-alist)
   'add-default (condition-case nil (progn (face-remap-add-relative 'default '(:weight bold)) 'ok) (error 'no))
   'add-bold (condition-case nil (progn (face-remap-add-relative 'bold '(:foreground "red")) 'ok) (error 'no))
   'add-italic (condition-case nil (progn (face-remap-add-relative 'italic '(:slant oblique)) 'ok) (error 'no))
   'alist-after-adds (face-remapping-alist)
   'reset-default (condition-case nil (progn (face-remap-reset-base 'default) 'ok) (error 'no))
   'reset-bold (condition-case nil (progn (face-remap-reset-base 'bold) 'ok) (error 'no))
   'reset-italic (condition-case nil (progn (face-remap-reset-base 'italic) 'ok) (error 'no))
   'alist-after-all (face-remapping-alist))))"##,
    );
}

#[test]
fn ft_wave_face_overlay_face_at_edge_of_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Edge of buffer overlay face test content text data buffer here now end")
    (let ((ov-start (make-overlay 1 10))) (overlay-put ov-start 'face '(:background "red")))
    (let ((ov-end (make-overlay 50 60))) (overlay-put ov-end 'face '(:background "blue")))
    (let ((ov-full (make-overlay 1 60))) (overlay-put ov-full 'face '(:foreground "green")) (overlay-put ov-full 'priority -100))
    (list
     'face-at-1 (get-char-property 1 'face)
     'face-at-5 (get-char-property 5 'face)
     'face-at-10 (get-char-property 10 'face)
     'face-at-50 (get-char-property 50 'face)
     'face-at-55 (get-char-property 55 'face)
     'face-at-60 (get-char-property 60 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 60)) 'cleaned))))))"##,
    );
}

#[test]
fn ft_wave_face_property_set_all_clear_then_set_new_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Set all clear set new face property test buffer content text here done")
    (add-text-properties 1 60 (list 'face 'bold 'a 1 'b 2 'c 3))
    (list
     'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (length (text-properties-at pos)))) '(1 20 40 59))
     'clear-all (progn (set-text-properties 1 60 nil) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (length (text-properties-at pos)))) '(1 20 40 59)))
     'set-new (progn (put-text-property 1 30 'face 'italic) (put-text-property 30 60 'face 'underline) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 15 30 45 59)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_wave_face_overlay_copy_and_paste_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Copy and paste overlay face test content text data buffer content here now end done final")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "Copy source overlay")
      (list
       'source-face (overlay-get ov 'face)
       'source-priority (overlay-get ov 'priority)
       'source-help (overlay-get ov 'help-echo)
       'source-string (buffer-substring 15 35)
       (progn (delete-overlay ov) 'cleaned)
       ;; Recreate similar overlay
       'recreate (progn
                   (let ((ov2 (make-overlay 40 55)))
                     (overlay-put ov2 'face '(:background "yellow"))
                     (overlay-put ov2 'priority 50)
                     (list 'new-face (overlay-get ov2 'face) 'new-priority (overlay-get ov2 'priority) 'char-prop-at-45 (get-char-property 45 'face) (progn (delete-overlay ov2) 'new-cleaned)))))))))"##,
    );
}

#[test]
fn ft_ray_face_overlay_priority_zero_vs_negative_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 0))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority -1))
    (let ((ov3 (make-overlay 15 35))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 1))
    (let ((ov4 (make-overlay 5 25))) (overlay-put ov4 'face '(:foreground "cyan")) (overlay-put ov4 'priority -10))
    (list
     'face-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (length (overlays-at pos)))) '(1 5 10 15 20 25 30 35))
     'negative-priorities (mapcar (lambda (ov) (list (overlay-start ov) (overlay-get ov 'priority))) (list ov1 ov2 ov3 ov4))
     (progn (mapc #'delete-overlay (overlays-in 1 35)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_ray_font_lock_fontify_only_syntactic_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; comment text\n\"string literal\"\n(defun syn-only () 42)\n")
    (font-lock-fontify-syntactically (point-min) (point-max) nil)
    (mapcar
     (lambda (needle)
       (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face) (get-text-property (match-beginning 0) 'fontified))))
     '("comment" "string" "defun" "syn-only" "42"))))"##,
    );
}

#[test]
fn ft_ray_face_overlay_get_face_after_evaporate_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Evaporate overlay face after deletion test content text data buffer")
    (let ((ov (make-overlay 15 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'evaporate t)
      (list
       'face-before (overlay-get ov 'face)
       'evaporate-before (overlay-get ov 'evaporate)
       'overlay-alive (and ov (overlay-buffer ov) t)
       (progn (delete-region 15 30) (list 'overlay-dead (not (and ov (overlay-buffer ov))) 'face-at-14 (get-char-property 14 'face) 'face-at-16 (get-char-property 16 'face)))))))"##,
    );
}

#[test]
fn ft_ray_face_text_property_interval_boundary_cross_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABCDEFGHIJKLMNOPQRSTUVWXYZ")
    (put-text-property 2 4 'face 'bold)
    (put-text-property 4 6 'face 'italic)
    (put-text-property 6 8 'face 'underline)
    (put-text-property 8 10 'face '(:foreground "red"))
    (put-text-property 10 12 'face '(:backward "yellow"))
    (list
     'every-char-face (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 4 5 6 7 8 9 10 11 12 13))
     'prop-boundaries (mapcar (lambda (pos) (next-single-property-change pos 'face nil 13)) '(1 2 4 6 8 10 12))
     'no-face-chars (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 13))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_ray_font_lock_fontify_region_vs_ensure_equivalence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun equiv-test (a) (+ a 1))\n")
    (list
     'fontify-region (progn (font-lock-fontify-region 1 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 29)))
     'unfontify (progn (font-lock-unfontify-buffer) 'unfontified)
     'fontify-ensure (progn (font-lock-ensure (point-min) (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 29)))
     'equivalent (progn (font-lock-unfontify-buffer) (let ((v1 (progn (font-lock-fontify-region 1 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 29))))) (font-lock-unfontify-buffer) (let ((v2 (progn (font-lock-ensure (point-min) (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 29))))) (equal v1 v2))))))))"##,
    );
}

#[test]
fn ft_ray_face_overlay_string_property_persistence_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string property persistence face test content text buffer data here done")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "BEFORE" 'face '(:foreground "red") 'custom-prop 'custom-value))
      (overlay-put ov 'after-string (propertize "AFTER" 'face '(:foreground "blue") 'other-prop 'other-value))
      (list
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-custom (get-text-property 0 'custom-prop (overlay-get ov 'before-string))
       'after-face (get-text-property 0 (overlay-get ov 'after-string))
       'after-custom (get-text-property 0 'other-prop (overlay-get ov 'after-string))
       'before-props (text-properties-at 0 (overlay-get ov 'before-string))
       'after-props (text-properties-at 0 (overlay-get ov 'after-string))
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_ray_face_set_attribute_with_relative_float_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-rel-height-face) (error nil))
  (list
   'set-1.5 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 1.5) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-2.0 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 2.0) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-0.5 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 0.5) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-int-180 (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 180) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-unspecified (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height 'unspecified) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no))
   'set-nil (condition-case nil (progn (set-face-attribute 'my-rel-height-face nil :height nil) (face-attribute 'my-rel-height-face :height nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_ray_face_property_list_access_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Property list access cycle face test buffer content text data here end now final done all")
    (add-text-properties 1 76 (list 'face 'bold 'p1 'v1 'p2 'v2 'p3 'v3 'p4 'v4 'p5 'v5))
    (add-text-properties 25 50 (list 'face 'italic 'p6 'v6 'p7 'v7))
    (let ((props (text-properties-at 1)))
      (list
       'props-length (length props)
       'all-keys (let ((keys nil) (i 0))
                   (while (< i (length props))
                     (push (nth i props) keys)
                     (setq i (+ i 2)))
                   (nreverse keys))
       'all-values (let ((vals nil) (i 0))
                     (while (< i (length props))
                       (push (nth (1+ i) props) vals)
                       (setq i (+ i 2)))
                     (nreverse vals))
       'check-face (get-text-property 1 'face)
       'check-p3 (get-text-property 1 'p3)
       'check-p6 (get-text-property 1 'p6)
       'check-face-at-30 (get-text-property 30 'face)
       'check-p6-at-30 (get-text-property 30 'p6))))))"##,
    );
}

#[test]
fn ft_beam_face_overlay_with_both_start_end_empty_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Both start end empty overlay test content text data buffer here now")
    (let ((ov1 (make-overlay 1 1))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'before-string "[START]"))
    (let ((ov2 (make-overlay 57 57))) (overlay-put ov2 'face '(:background "blue")) (overlay-put ov2 'after-string "[END]"))
    (list
     'start-face (overlay-get ov1 'face)
     'end-face (overlay-get ov2 'face)
     'start-before (overlay-get ov1 'before-string)
     'end-after (overlay-get ov2 'after-string)
     'faces-at-boundaries (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 10 30 57))
     (progn (mapc #'delete-overlay (overlays-in 1 57)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_beam_font_lock_fontify_after_keyword_removal_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "REMOVE-ONE REMOVE-TWO REMOVE-ALL font lock keyword removal test end")
    (font-lock-add-keywords nil '(("\\<\\(REMOVE-ONE\\)\\>" 1 '(:foreground "red") t) ("\\<\\(REMOVE-TWO\\)\\>" 1 '(:foreground "green") t) ("\\<\\(REMOVE-ALL\\)\\>" 1 '(:foreground "blue") t)))
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("REMOVE-ONE" "REMOVE-TWO" "REMOVE-ALL"))))
      (font-lock-remove-keywords nil '(("\\<\\(REMOVE-ONE\\)\\>" 1 '(:foreground "red") t) ("\\<\\(REMOVE-TWO\\)\\>" 1 '(:foreground "green") t) ("\\<\\(REMOVE-ALL\\)\\>" 1 '(:foreground "blue") t)))
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("REMOVE-ONE" "REMOVE-TWO" "REMOVE-ALL"))))
        (list v0 v1))))))"##,
    );
}

#[test]
fn ft_beam_face_text_property_with_complex_nested_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Complex nested face property test buffer content text data here now")
    (put-text-property 1 60 'face (list :foreground "blue" :weight (list 'quote 'bold) :slant (list 'quote 'italic)))
    (list
     'face-value (get-text-property 1 'face)
     'facep (facep (get-text-property 1 'face))
     'plistp (plistp (get-text-property 1 'face))
     'extract-fg (plist-get (get-text-property 1 'face) :foreground)
     'extract-weight (plist-get (get-text-property 1 'face) :weight)
     'extract-slant (plist-get (get-text-property 1 'face) :slant)
     'length (length (get-text-property 1 'face))))))"##,
    );
}

#[test]
fn ft_beam_face_overlay_insert_behind_front_combo_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov-front (make-overlay 6 15)))
      (overlay-put ov-front 'face '(:background "yellow"))
      (overlay-put ov-front 'insert-in-front-hooks (list 'ignore)))
    (let ((ov-behind (make-overlay 20 30)))
      (overlay-put ov-behind 'face '(:foreground "red" :weight bold))
      (overlay-put ov-behind 'insert-behind-hooks (list 'ignore)))
    (list
     'faces-before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30 35))
     'after-insert-front (progn (goto-char 15) (insert "FRONT-INSERT") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 10 15 20 28 35)))
     'after-insert-behind (progn (goto-char 30) (insert "BEHIND-INSERT") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 10 15 20 28 35 40)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned)))))"##,
    );
}

#[test]
fn ft_beam_face_font_lock_fontify_batch_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (let ((noninteractive t))
      (emacs-lisp-mode)
      (insert "(defun batch-test (x) x)\n")
      (font-lock-fontify-buffer)
      (list
       'noninteractive noninteractive
       'face-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
       'fontified (get-text-property 1 'fontified)
       'font-lock-verbose (if (boundp 'font-lock-verbose) font-lock-verbose 'no-bound))))))"##,
    );
}

#[test]
fn ft_beam_face_text_property_fontished_flag_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Fontified flag check face property test buffer content text data here end")
    (list
     'before-fontify (get-text-property 1 'fontified)
     'set-face-prop (progn (put-text-property 1 20 'face 'bold) (get-text-property 1 'fontified))
     'set-fontified-prop (progn (put-text-property 1 20 'fontified t) (get-text-property 1 'fontified))
     'remove-fontified (progn (remove-text-properties 1 20 '(fontified nil)) (get-text-property 1 'fontified))
     'set-fontified-back (progn (put-text-property 1 20 'fontified t) (get-text-property 1 'fontified))
     'face-still-there (get-text-property 1 'face)))))"##,
    );
}

#[test]
fn ft_beam_face_overlay_priority_within_range_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHH")
    (let ((ov (make-overlay 5 40)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 20 30 40 45))
       'change-priority-100 (progn (overlay-put ov 'priority 100) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 20 30 40 45)))
       'change-priority-0 (progn (overlay-put ov 'priority 0) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 20 30 40 45)))
       'change-face (progn (overlay-put ov 'face '(:foreground "red" :weight bold)) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 20 30 40 45)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_beam_face_propertize_multiple_props_and_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (let ((s (propertize "PROPERTIZED" 'face 'bold 'key1 'value1 'key2 'value2 'fontified t)))
    (list
     'face (get-text-property 0 'face s)
     'key1 (get-text-property 0 'key1 s)
     'key2 (get-text-property 0 'key2 s)
     'fontified (get-text-property 0 'fontified s)
     'length (length s)
     'all-props (text-properties-at 0 s)
     'props-count (length (text-properties-at 0 s))
     ;; Modify and re-read
     (progn (set-text-properties 0 (length s) (list 'face 'italic 'key3 'newval) s) (list 'modified-face (get-text-property 0 'face s) 'old-key1 (get-text-property 0 'key1 s) 'new-key3 (get-text-property 0 'key3 s)))))))"##,
    );
}

#[test]
fn ft_pulse_face_set_face_underline_complex_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-patterns-face) (error nil))
  (list
   'wave-red (condition-case nil (progn (set-face-underline 'my-ul-patterns-face '(:color "red" :style wave) nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   'line-blue (condition-case nil (progn (set-face-underline 'my-ul-patterns-face '(:color "blue" :style line) nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   'double-green (condition-case nil (progn (set-face-underline 'my-ul-patterns-face '(:color "green" :style double-line) nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   'dots-orange (condition-case nil (progn (set-face-underline 'my-ul-patterns-face '(:color "orange" :style dots) nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   'dash-purple (condition-case nil (progn (set-face-underline 'my-ul-patterns-face '(:color "purple" :style dash) nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   'clear (condition-case nil (progn (set-face-underline 'my-ul-patterns-face nil nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   't-simple (condition-case nil (progn (set-face-underline 'my-ul-patterns-face t nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no))
   'unspec (condition-case nil (progn (set-face-underline 'my-ul-patterns-face 'unspacified nil) (face-attribute 'my-ul-patterns-face :underline nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_pulse_font_lock_fontify_buffer_unfontify_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun unfontify-cycle (x) (* x x))\n")
    (let ((results nil))
      (font-lock-fontify-buffer)
      (push (list 'fontify-1 (get-text-property 1 'fontified) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 25))) results)
      (font-lock-unfontify-buffer)
      (push (list 'unfontify (get-text-property 1 'fontified)) results)
      (font-lock-fontify-buffer)
      (push (list 'fontify-2 (get-text-property 1 'fontified) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 25))) results)
      (font-lock-unfontify-buffer)
      (push (list 'unfontify-2 (get-text-property 1 'fontified)) results)
      (font-lock-fontify-buffer)
      (push (list 'fontify-3 (get-text-property 1 'fontified)) results)
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_pulse_face_text_property_get_all_props_and_rebuild() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Get all props and rebuild face test content text data buffer content")
    (add-text-properties 1 58 (list 'face 'bold 'a 1 'b 2 'c 3 'fontified t))
    (let ((props (text-properties-at 1)) (keys nil))
      (dotimes (i (/ (length props) 2))
        (push (nth (* i 2) props) keys)
        (push (nth (1+ (* i 2)) props) keys))
      (setq keys (nreverse keys))
      (list
       'extracted-keys (let ((k nil) (i 0))
                         (while (< i (length props))
                           (push (nth i props) k) (setq i (+ i 2)))
                         (nreverse k))
       'face-value (get-text-property 1 'face)
       'props-count (length props)
       ;; Clear and rebuild with extracted props
       (progn (set-text-properties 1 58 nil) (add-text-properties 1 58 keys) (list 'rebuilt-face (get-text-property 1 'face) 'rebuilt-fontified (get-text-property 1 'fontified))))))))"##,
    );
}

#[test]
fn ft_pulse_face_overlay_both_empty_end_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((ov-start (make-overlay 1 1))) (overlay-put ov-start 'face '(:background "red")) (overlay-put ov-start 'before-string "[B]"))
    (let ((ov-end (make-overlay 1 1))) (overlay-put ov-end 'face '(:background "blue")) (overlay-put ov-end 'after-string "[A]"))
    (let ((ov-mid (make-overlay 1 1))) (overlay-put ov-mid 'face '(:weigh bold :slant italic)) (overlay-put ov-mid 'display ""))
    (list
     'all-empty-faces (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov) (overlay-get ov 'face))) (list ov-start ov-end ov-mid))
     'char-prop (get-char-property 1 'face)
     (progn (insert "FILLED") (list 'after-fill (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 3 6))))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned))))))"##,
    );
}

#[test]
fn ft_pulse_font_lock_keywords_fontify_case_sensitive_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "CASE case Case CAse font lock face case sensitivity test")
    (let ((results nil))
      (let ((font-lock-keywords-case-fold-search t))
        (font-lock-add-keywords nil '(("\\<\\(CASE\\)\\>" 1 font-lock-warning-face t)))
        (font-lock-fontify-buffer)
        (push (list 'case-fold-t (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("CASE" "case" "Case")))
              results)
        (font-lock-remove-keywords nil '(("\\<\\(CASE\\)\\>" 1 font-lock-warning-face t))))
      (let ((font-lock-keywords-case-fold-search nil))
        (font-lock-add-keywords nil '(("\\<\\(CASE\\)\\>" 1 font-lock-warning-face t)))
        (font-lock-fontify-buffer)
        (push (list 'case-fold-nil (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face))) '("CASE" "case" "Case")))
              results)
        (font-lock-remove-keywords nil '(("\\<\\(CASE\\)\\>" 1 font-lock-warning-face t))))
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_pulse_face_overlay_face_get_after_overlay_buffer_deletion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Face after overlay buffer deletion test content text data here now done")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before (list 'face (overlay-get ov 'face) 'buffer (and (overlay-buffer ov) t))
       (progn (delete-overlay ov) (list 'after-delete (and (overlay-buffer ov) t) 'face-at-15 (get-char-property 15 'face) 'face-at-25 (get-char-property 25 'face))))))))"##,
    );
}

#[test]
fn ft_pulse_face_with_text_property_category_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Text property category face test buffer content text data here now end")
    (put-text-property 1 20 'face 'bold)
    (put-text-property 1 20 'category 'my-text-cat-bold)
    (put-text-property 20 40 'face 'italic)
    (put-text-property 20 40 'category 'my-text-cat-italic)
    (put-text-property 40 60 'face 'underline)
    (list
     'face-cat (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'category))) '(1 10 20 30 40 50 59))
     'get-char-property (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'category)) '(1 20 40 59))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_pulse_face_overlay_properties_count_after_multiple_adds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAA")
    (let ((ov (make-overlay 1 11)))
      (list
       'empty-props (length (overlay-properties ov))
       'add-face (progn (overlay-put ov 'face '(:background "yellow")) (length (overlay-properties ov)))
       'add-priority (progn (overlay-put ov 'priority 50) (length (overlay-properties ov)))
       'add-help (progn (overlay-put ov 'help-echo "test") (length (overlay-properties ov)))
       'add-evap (progn (overlay-put ov 'evaporate t) (length (overlay-properties ov)))
       'add-category (progn (overlay-put ov 'category 'my-cat) (length (overlay-properties ov)))
       'all-keys (let ((props (overlay-properties ov)) (keys nil) (i 0))
                   (while (< i (length props)) (push (nth i props) keys) (setq i (+ i 2)))
                   (nreverse keys))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_colour_face_set_face_underline_style_only_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-style-face) (error nil))
  (list
   'set-wave (condition-case nil (progn (set-face-underline 'my-ul-style-face '(:style wave) nil) (face-attribute 'my-ul-style-face :underline nil 'default-on)) (error 'no))
   'set-line (condition-case nil (progn (set-face-underline 'my-ul-style-face '(:style line) nil) (face-attribute 'my-ul-style-face :underline nil 'default-on)) (error 'no))
   'set-double (condition-case nil (progn (set-face-underline 'my-ul-style-face '(:style double-line) nil) (face-attribute 'my-ul-style-face :underline nil 'default-on)) (error 'no))
   'set-dots (condition-case nil (progn (set-face-underline 'my-ul-style-face '(:style dots) nil) (face-attribute 'my-ul-style-face :underline nil 'default-on)) (error 'no))
   'set-dash (condition-case nil (progn (set-face-underline 'my-ul-style-face '(:style dash) nil) (face-attribute 'my-ul-style-face :underline nil 'default-on)) (error 'no))
   'set-off (condition-case nil (progn (set-face-underline 'my-ul-style-face nil nil) (face-attribute 'my-ul-style-face :underline nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_colour_font_lock_fontify_unfontify_toggle_repeat_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun toggle-repeat (x) (* x x))\n")
    (let ((results nil))
      (dotimes (i 4)
        (if (evenp i)
            (font-lock-fontify-buffer)
          (font-lock-unfontify-buffer))
        (push (list i (get-text-property 1 'fontified) (get-text-property 7 'face)) results))
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_colour_face_text_property_boundary_zero_width_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 27 'face '(:foreground "red"))
    (list
     'at-boundary-5 (list (get-text-property 4 'face) (get-text-property 5 'face))
     'at-boundary-9 (list (get-text-property 8 'face) (get-text-property 9 'face))
     'at-boundary-13 (list (get-text-property 12 'face) (get-text-property 13 'face))
     'next-prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 27)) '(1 5 9 13))
     'prev-prop-changes (mapcar (lambda (pos) (previous-single-property-change pos 'face nil 1)) '(5 9 13 27))
     'text-prop-any-check (text-property-any 1 27 'face 'bold)
     'not-all-check (text-property-not-all 1 27 'face 'bold)))))"##,
    );
}

#[test]
fn ft_colour_face_overlay_window_specific_properties_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov-nil (make-overlay 1 15))) (overlay-put ov-nil 'face '(:background "red")) (overlay-put ov-nil 'window nil))
    (let ((ov-cur (make-overlay 10 25))) (overlay-put ov-cur 'face '(:background "green")) (overlay-put ov-cur 'window (selected-window)))
    (let ((ov-nil2 (make-overlay 20 35))) (overlay-put ov-nil2 'face '(:background "blue")) (overlay-put ov-nil2 'window nil))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35))
     'windows (mapcar (lambda (ov) (list (overlay-start ov) (overlay-get ov 'window))) (list ov-nil ov-cur ov-nil2))
     (progn (mapc #'delete-overlay (overlays-in 1 35)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_colour_font_lock_fontify_block_vs_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun block-buffer-test (x) (+ x 1))\n")
    (list
     'fontify-block (condition-case nil (progn (font-lock-fontify-block 1) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified) (get-text-property pos 'face))) '(1 10 20 30))) (error 'no-block-fontify))
     'unfontify (progn (font-lock-unfontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30)))
     'fontify-buffer (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 30)))))))"##,
    );
}

#[test]
fn ft_colour_face_property_change_at_midpoint_interval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXYYYYYYYYYYZZZZZZZZZZ")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 11 21 'face 'italic)
    (put-text-property 21 31 'face 'underline)
    ;; Change face at midpoint of italic region
    (put-text-property 15 18 'face '(:foreground "red"))
    ;; Change face at start of underline region
    (put-text-property 21 25 'face '(:background "yellow"))
    (list
     'faces-across (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 11 14 15 16 18 20 21 24 25 26 30))
     'interval-count (length (object-intervals (current-buffer)))
     'find-bold (text-property-any 1 31 'face 'bold)
     'find-italic (text-property-any 1 31 'face 'italic)
     'find-red (text-property-any 1 31 'face '(:foreground "red")))
     'find-yellow (text-property-any 1 31 'face '(:background "yellow"))))))"##,
    );
}

#[test]
fn ft_colour_face_overlay_face_vs_text_prop_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (put-text-property 1 31 'face '(:foreground "blue"))
    (let ((ov1 (make-overlay 5 15))) (overlay-put ov1 'face '(:background "yellow")) (overlay-put ov1 'priority 100))
    (let ((ov2 (make-overlay 15 25))) (overlay-put ov2 'face '(:foreground "red" :weight bold)) (overlay-put ov2 'priority 200))
    (list
     'text-prop-only (get-text-property 10 'face)
     'overlay-priority-100 (get-char-property 10 'face)
     'overlay-priority-200 (get-char-property 20 'face)
     'text-prop-at-20 (get-text-property 20 'face)
     'char-prop-at-20 (get-char-property 20 'face)
     'char-prop-and-overlay-at-20 (get-char-property-and-overlay 20 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 31)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_colour_face_set_all_attrs_to_unspec_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-full-unspec-face) (error nil))
  (condition-case nil (set-face-attribute 'my-full-unspec-face nil :weight 'bold :slant 'italic :foreground "red" :background "yellow" :underline t :overline t :strike-through t :box t :inverse-video t :height 150 :width 'condensed :extend t) (error nil))
  (list
   'weight-before (face-attribute 'my-full-unspec-face :weight nil 'default-on)
   'fg-before (condition-case nil (face-foreground 'my-full-unspec-face nil 'default-on) (error 'no))
   'unset-all (condition-case nil (progn (set-face-attribute 'my-full-unspec-face nil :weight 'unspecified :slant 'unspecified :foreground 'unspecified :background 'unspecified :underline 'unspecified :overline 'unspecified :strike-through 'unspecified :box 'unspecified :inverse-video 'unspecified :height 'unspecified :width 'unspecified :extend 'unspecified) 'unset-done) (error 'no))
   'weight-after (face-attribute 'my-full-unspec-face :weight nil 'default-on)
   'fg-after (condition-case nil (face-foreground 'my-full-unspec-face nil 'default-on) (error 'no))
   'bg-after (condition-case nil (face-background 'my-full-unspec-face nil 'default-on) (error 'no))
   'height-after (face-attribute 'my-full-unspec-face :height nil 'default-on)
   'width-after (face-attribute 'my-full-unspec-face :width nil 'default-on))))))"##,
    );
}

#[test]
fn ft_photon_face_font_lock_multi_line_string_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(format \"hello\nworld\n%s\" name)\n")
    (font-lock-fontify-buffer)
    (list
     'format-face (save-excursion (goto-char (point-min)) (search-forward "format") (get-text-property (match-beginning 0) 'face))
     'hello-face (save-excursion (goto-char (point-min)) (search-forward "hello") (get-text-property (match-beginning 0) 'face))
     'world-face (save-excursion (goto-char (point-min)) (search-forward "world") (get-text-property (match-beginning 0) 'face))
     'name-face (save-excursion (goto-char (point-min)) (search-forward "name") (get-text-property (match-beginning 0) 'face))
     'fontified-all (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 18 25 30 34))))))"##,
    );
}

#[test]
fn ft_photon_face_overlay_priority_interleave_complex_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHHIIIIIJJJJJ")
    (let ((ov1 (make-overlay 6 15))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 20))
    (let ((ov2 (make-overlay 10 25))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 40))
    (let ((ov3 (make-overlay 20 35))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 30))
    (let ((ov4 (make-overlay 30 45))) (overlay-put ov4 'face '(:foreground "orange" :weight bold)) (overlay-put ov4 'priority 10))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 8 12 18 22 28 32 38 45 50)))))
      (let ((v0 (funcall snap)))
        ;; Rearrange priorities
        (overlay-put ov1 'priority 50) (overlay-put ov3 'priority 60) (overlay-put ov2 'priority 1)
        (let ((v1 (funcall snap)))
          (mapc #'delete-overlay (overlays-in 1 50))
          (list v0 v1)))))))"##,
    );
}

#[test]
fn ft_photon_face_text_property_fontified_flag_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Fontified flag cleanup face property test buffer content text data")
    (put-text-property 1 54 'face 'bold)
    (put-text-property 1 54 'fontified t)
    (list
     'fontified-before (get-text-property 1 'fontified)
     'face-before (get-text-property 1 'face)
     'remove-fontified (progn (remove-text-properties 1 54 '(fontified nil)) (get-text-property 1 'fontified))
     'face-after-remove (get-text-property 1 'face)
     're-add-fontified (progn (put-text-property 1 54 'fontified t) (get-text-property 1 'fontified))
     'face-after-re-add (get-text-property 1 'face)
     'props-count-at-1 (length (text-properties-at 1))
     'props-count-at-30 (length (text-properties-at 30))))))"##,
    );
}

#[test]
fn ft_photon_face_overlay_line_spacing_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Line spacing overlay face test content text data buffer here now done final")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow" :foreground "black"))
      (overlay-put ov 'line-spacing 15)
      (overlay-put ov 'line-height 2.0)
      (list
       'face (overlay-get ov 'face)
       'line-spacing (overlay-get ov 'line-spacing)
       'line-height (overlay-get ov 'line-height)
       'face-at-20 (get-char-property 20 'face)
       'face-at-5 (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_photon_font_lock_keywords_overwrite_vs_keep_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "OVERWRITE keep KEEP test font lock face buffer content")
    (font-lock-add-keywords nil '(("\\<\\(OVERWRITE\\)\\>" 1 font-lock-warning-face overwrite) ("\\<\\(KEEP\\)\\>" 1 '(:foreground "red" :weight bold) keep)))
    (font-lock-fontify-buffer)
    (list
     'overwrite-face (save-excursion (goto-char (point-min)) (search-forward "OVERWRITE") (get-text-property (match-beginning 0) 'face))
     'keep-face (save-excursion (goto-char (point-min)) (search-forward "KEEP") (get-text-property (match-beginning 0) 'face))
     'non-keyword (save-excursion (goto-char (point-min)) (search-forward "test") (get-text-property (match-beginning 0) 'face)))
    (font-lock-remove-keywords nil '(("\\<\\(OVERWRITE\\)\\>" 1 font-lock-warning-face overwrite) ("\\<\\(KEEP\\)\\>" 1 '(:foreground "red" :weight bold) keep)))
    'cleaned-keywords)))"##,
    );
}

#[test]
fn ft_photon_face_set_face_font_attribute_with_large_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-large-spec-face) (error nil))
  (list
   'set-font-spec (condition-case nil (progn (set-face-font 'my-large-spec-face (font-spec :family "Monospace" :size 14 :weight 'bold :slant 'italic :width 'normal) nil) 'ok) (error 'no))
   'get-font (condition-case nil (face-font 'my-large-spec-face nil) (error 'no))
   'set-font-string (condition-case nil (progn (set-face-font 'my-large-spec-face "Monospace-Bold-Italic-14" nil) 'ok) (error 'no))
   'get-font-2 (condition-case nil (face-font 'my-large-spec-face nil) (error 'no))
   'reset-font (condition-case nil (progn (set-face-font 'my-large-spec-face 'unspecified nil) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_photon_face_text_property_get_set_and_clear_at_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (put-text-property 1 2 'face 'bold)
    (list
     'single-char-face (get-text-property 1 'face)
     'single-char-fontified (get-text-property 1 'fontified)
     'text-props (text-properties-at 1)
     (progn (goto-char 2) (insert "EXTENDED-TEXT") (list 'face-1 (get-text-property 1 'face) 'face-2 (get-text-property 2 'face) 'face-5 (get-text-property 5 'face) 'face-14 (get-text-property 14 'face)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_photon_face_default_properties_of_font_lock_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'font-lock-warning-face-underline (condition-case nil (face-attribute 'font-lock-warning-face :underline nil 'default-on) (error 'no))
   'font-lock-keyword-face-weight (face-attribute 'font-lock-keyword-face :weight nil 'default-on)
   'font-lock-function-name-face-weight (face-attribute 'font-lock-function-name-face :weight nil 'default-on)
   'font-lock-variable-name-face-slant (face-attribute 'font-lock-variable-name-face :slant nil 'default-on)
   'font-lock-type-face-weight (face-attribute 'font-lock-type-face :weight nil 'default-on)
   'font-lock-constant-face-weight (face-attribute 'font-lock-constant-face :weight nil 'default-on)
   'font-lock-string-face-slant (condition-case nil (face-attribute 'font-lock-string-face :slant nil 'default-on) (error 'no))
   'font-lock-comment-face-slant (condition-case nil (face-attribute 'font-lock-comment-face :slant nil 'default-on) (error 'no))
   'font-lock-doc-face-slant (condition-case nil (face-attribute 'font-lock-doc-face :slant nil 'default-on) (error 'no))
   'font-lock-builtin-face-weight (condition-case nil (face-attribute 'font-lock-builtin-face :weight nil 'default-on) (error 'no))
   'font-lock-preprocessor-face-weight (condition-case nil (face-attribute 'font-lock-preprocessor-face :weight nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_gluon2_face_overlay_with_negative_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov1 (make-overlay 1 15))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority -5))
    (let ((ov2 (make-overlay 10 25))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority -10))
    (let ((ov3 (make-overlay 20 30))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority -1))
    (list
     'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30))
     'negative-priority-order (mapcar (lambda (ov) (list (overlay-start ov) (overlay-get ov 'priority))) (sort (list ov1 ov2 ov3) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority)))))
     (progn (mapc #'delete-overlay (overlays-in 1 30)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_gluon2_font_lock_fontify_after_add_and_remove_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "ADD-REMOVE keyword cycle test font lock face buffer content data text")
    (let ((results nil))
      (font-lock-add-keywords nil '(("\\<\\(ADD-REMOVE\\)\\>" 1 '(:foreground "red" :weight bold) t)))
      (font-lock-fontify-buffer)
      (push (list 'after-add (save-excursion (goto-char (point-min)) (search-forward "ADD-REMOVE") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-remove-keywords nil '(("\\<\\(ADD-REMOVE\\)\\>" 1 '(:foreground "red" :weight bold) t)))
      (font-lock-fontify-buffer)
      (push (list 'after-remove (save-excursion (goto-char (point-min)) (search-forward "ADD-REMOVE") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-add-keywords nil '(("\\<\\(ADD-REMOVE\\)\\>" 1 '(:foreground "green")) t))
      (font-lock-fontify-buffer)
      (push (list 'after-re-add (save-excursion (goto-char (point-min)) (search-forward "ADD-REMOVE") (get-text-property (match-beginning 0) 'face))) results)
      (font-lock-remove-keywords nil '(("\\<\\(ADD-REMOVE\\)\\>" 1 '(:foreground "green")) t))
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_gluon2_face_text_property_value_plist_decompose_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Plist decompose face property test buffer content text data here")
    (put-text-property 1 56 'face '(:foreground "blue" :weight bold :slant italic :underline t :overline t :strike-through nil :box (:line-width 2) :inherit bold :extend t))
    (let ((plist (get-text-property 1 'face)))
      (list
       'plist-length (length plist)
       'all-keys (let ((keys nil) (i 0))
                   (while (< i (length plist)) (push (nth i plist) keys) (setq i (+ i 2)))
                   (nreverse keys))
       'all-values (let ((vals nil) (i 0))
                     (while (< i (length plist)) (push (nth (1+ i) plist) vals) (setq i (+ i 2)))
                     (nreverse vals))
       'fg-get (plist-get plist :foreground)
       'weight-get (plist-get plist :weight)
       'slant-get (plist-get plist :slant)
       'underline-get (plist-get plist :underline)
       'box-get (plist-get plist :box)
       'inherit-get (plist-get plist :inherit)
       'extend-get (plist-get plist :extend)))))"##,
    );
}

#[test]
fn ft_gluon2_face_overlay_properties_list_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAA")
    (let ((ov (make-overlay 1 11)))
      (list
       'empty-props (overlay-properties ov)
       'add-face (progn (overlay-put ov 'face '(:background "yellow")) (overlay-properties ov))
       'add-prio (progn (overlay-put ov 'priority 50) (overlay-properties ov))
       'add-help (progn (overlay-put ov 'help-echo "help") (overlay-properties ov))
       'add-evap (progn (overlay-put ov 'evaporate t) (overlay-properties ov))
       'add-cat (progn (overlay-put ov 'category 'my-cat) (overlay-properties ov))
       'remove-face (progn (overlay-put ov 'face nil) (overlay-properties ov))
       'face-gone (overlay-get ov 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_gluon2_font_lock_fontify_after_mode_disable_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (require 'org)
  (with-temp-buffer
    (let ((org-fontify-whole-heading-line t))
      (org-mode)
      (insert "* TODO Mode disable fontify test\nBody.\n\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
        (font-lock-mode -1)
        (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
          (font-lock-mode 1)
          (font-lock-ensure (point-min) (point-max))
          (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "TODO") (get-text-property (match-beginning 0) 'face))))
            (list v0 v1 v2))))))))"##,
    );
}

#[test]
fn ft_gluon2_face_property_next_change_with_object_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (put-text-property 21 41 'face '(:foreground "blue"))
    (list
     'next-with-limit (next-single-property-change 1 'face nil 20)
     'next-no-limit (next-single-property-change 1 'face)
     'next-from-10-limit (next-single-property-change 10 'face nil 15)
     'next-from-10-no-limit (next-single-property-change 10 'face)
     'prev-with-limit (previous-single-property-change 41 'face nil 1)
     'text-prop-any-limited (text-property-any 1 15 'face 'bold)
     'text-prop-not-all (text-property-not-all 1 41 'face '(:foreground "blue")))))"##,
    );
}

#[test]
fn ft_gluon2_face_overlay_priority_change_affects_rendering() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 100))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 50))
    (let ((snap (lambda () (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 5 10 15 20 25 30 35)))))
      (let ((v0 (funcall snap)))
        (overlay-put ov2 'priority 200)
        (let ((v1 (funcall snap)))
          (overlay-put ov1 'face '(:foreground "blue" :weight bold))
          (overlay-put ov2 'face '(:foreground "orange" :slant italic))
          (let ((v2 (funcall snap)))
            (mapc #'delete-overlay (overlays-in 1 35))
            (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_gluon2_face_set_face_attribute_with_default_family() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-default-family-face) (error nil))
  (list
   'set-family-to-default (condition-case nil (progn (set-face-attribute 'my-default-family-face nil :family (face-attribute 'default :family nil 'default-on)) 'ok) (error 'no))
   'get-family (face-attribute 'my-default-family-face :family nil 'default-on)
   'set-family-unspec (condition-case nil (progn (set-face-attribute 'my-default-family-face nil :family 'unspecified) (face-attribute 'my-default-family-face :family nil 'default-on)) (error 'no)))
   'default-family-is (face-attribute 'default :family nil 'default-on)
   'default-foundry-is (face-attribute 'default :foundry nil 'default-on)
   'default-registry-is (face-attribute 'default :registry nil 'default-on))))"##,
    );
}

#[test]
fn ft_wave2_face_overlay_properties_with_duplicate_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "red"))
      (overlay-put ov 'face '(:background "green"))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'face-value (overlay-get ov 'face)
       'props-count (length (overlay-properties ov))
       'props-get-face (plist-get (overlay-properties ov) 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_wave2_font_lock_unfontify_buffer_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun unfontify-buf-test () 42)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (and (get-text-property 1 'fontified) (get-text-property 7 'face))))
      (font-lock-unfontify-buffer)
      (let ((v1 (get-text-property 1 'fontified)))
        (font-lock-fontify-buffer)
        (let ((v2 (and (get-text-property 1 'fontified) (get-text-property 7 'face))))
          (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_wave2_face_property_interval_boundary_precise_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOP")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 3 6 'face 'italic)
    (put-text-property 6 10 'face 'underline)
    (put-text-property 10 15 'face '(:foreground "red"))
    (put-text-property 15 17 'face '(:background "yellow"))
    (list
     'every-char (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16))
     'prop-boundaries (mapcar (lambda (pos) (next-single-property-change pos 'face nil 17)) '(1 3 6 10 15))
     'text-prop-any-italic (text-property-any 1 17 'face 'italic)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_wave2_face_overlay_priorities_sort_order_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 16))) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 1 16))) (overlay-put ov2 'priority 30))
    (let ((ov3 (make-overlay 1 16))) (overlay-put ov3 'priority 20))
    (let ((sorted (sort (overlays-at 5) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority))))))
      (list
       'priorities (mapcar (lambda (ov) (overlay-get ov 'priority)) sorted)
       'count (length (overlays-at 5))
       (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned))))))"##,
    );
}

#[test]
fn ft_wave2_font_lock_set_defaults_for_fundamental_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (font-lock-set-defaults)
    (insert "Fundamental mode font lock defaults test buffer content text data")
    (font-lock-fontify-buffer)
    (list
     'font-lock-keywords font-lock-keywords
     'font-lock-keywords-case-fold font-lock-keywords-case-fold
     'fontified (get-text-property 1 'fontified)
     'face (get-text-property 1 'face)
     'mode major-mode))))"##,
    );
}

#[test]
fn ft_wave2_face_text_property_interval_object_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEE")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (let ((intervals (object-intervals (current-buffer))))
      (list
       'interval-count (length intervals)
       'first-obj-type (type-of (car intervals))
       'all-starts (mapcar #'overlay-start intervals)
       'all-ends (mapcar #'overlay-end intervals)
       'faces-at-starts (mapcar (lambda (start) (get-text-property start 'face)) (mapcar #'overlay-start intervals))))))"##,
    );
}

#[test]
fn ft_wave2_face_overlay_face_with_window_and_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 1 26)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'window (selected-window))
      (list
       'face (overlay-get ov 'face)
       'window-eq-selected (eq (overlay-get ov 'window) (selected-window))
       'buffer-eq-current (eq (overlay-buffer ov) (current-buffer))
       'face-at-10 (get-char-property 10 'face)
       'face-at-25 (get-char-property 25 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_wave2_face_set_face_font_by_family_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-family-only-face) (error nil))
  (list
   'set-monospace (condition-case nil (progn (set-face-font 'my-family-only-face (font-spec :family "Monospace") nil) 'ok) (error 'no))
   'get-font (condition-case nil (face-font 'my-family-only-face nil) (error 'no))
   'set-size-12 (condition-case nil (progn (set-face-font 'my-family-only-face (font-spec :family "Monospace" :size 12) nil) 'ok) (error 'no))
   'get-font-2 (condition-case nil (face-font 'my-family-only-face nil) (error 'no))
   'reset (condition-case nil (progn (set-face-font 'my-family-only-face 'unspecified nil) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_particle_face_font_lock_set_defaults_multiple_modes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (list
     'fundamental-defaults (progn (fundamental-mode) (font-lock-set-defaults) (list 'mode major-mode 'keywords (if (boundp 'font-lock-keywords) (type-of font-lock-keywords) 'no))))
     'text-mode-defaults (progn (text-mode) (font-lock-set-defaults) (list 'mode major-mode 'keywords (if (boundp 'font-lock-keywords) (type-of font-lock-keywords) 'no))))
     'emacs-lisp-defaults (progn (emacs-lisp-mode) (font-lock-set-defaults) (list 'mode major-mode 'keywords (if (boundp 'font-lock-keywords) (type-of font-lock-keywords) 'no))))))))"##,
    );
}

#[test]
fn ft_particle_face_overlay_with_display_string_and_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay display string and face combined test content text data buffer here")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'display "")
      (overlay-put ov 'before-string "[[")
      (overlay-put ov 'after-string "]]")
      (list
       'face (overlay-get ov 'face)
       'display (overlay-get ov 'display)
       'before (overlay-get ov 'before-string)
       'after (overlay-get ov 'after-string)
       'char-prop (get-char-property 25 'face)
       (progn (delete-overlay ov) 'cleaned)))))"##,
    );
}

#[test]
fn ft_particle_face_text_property_read_multiple_same_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Read multiple times same property face test buffer content text")
    (put-text-property 1 51 'face '(:foreground "red" :weight bold))
    (list
     'read-1 (get-text-property 1 'face)
     'read-2 (get-text-property 1 'face)
     'read-3 (get-text-property 1 'face)
     'equal-1-2 (equal (get-text-property 1 'face) (get-text-property 1 'face))
     'text-props-at (text-properties-at 1)
     'read-10 (get-text-property 10 'face)
     'read-50 (get-text-property 50 'face)
     'all-equal (and (equal (get-text-property 1 'face) (get-text-property 25 'face)) (equal (get-text-property 25 'face) (get-text-property 50 'face)))))))"##,
    );
}

#[test]
fn ft_particle_face_overlay_category_with_face_both_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'category 'my-special-cat)
      (overlay-put ov 'face '(:background "yellow" :inherit bold))
      (overlay-put ov 'priority 50)
      (list
       'face (overlay-get ov 'face)
       'category (overlay-get ov 'category)
       'priority (overlay-get ov 'priority)
       'char-prop-at-10 (get-char-property 10 'face)
       'char-prop-at-10-cat (get-char-property 10 'category)
       'char-prop-at-5 (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_particle_font_lock_fontify_no_keywords_just_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (setq font-lock-keywords nil)
    (insert ";; comment only\n\"string only\"\n(defun syntax-only-test () 42)\n")
    (font-lock-fontify-buffer)
    (list
     'keywords font-lock-keywords
     'comment-face (save-excursion (goto-char (point-min)) (search-forward "comment") (get-text-property (match-beginning 0) 'face))
     'string-face (save-excursion (goto-char (point-min)) (search-forward "string") (get-text-property (match-beginning 0) 'face))
     'defun-face (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     '42-face (save-excursion (goto-char (point-min)) (search-forward "42") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_particle_face_set_attribute_width_ultra_condensed_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-width-cycle-face) (error nil))
  (list
   'default-width (face-attribute 'default :width nil 'default-on)
   'set-ultra-condensed (condition-case nil (progn (set-face-attribute 'my-width-cycle-face nil :width 'ultra-condensed) (face-attribute 'my-width-cycle-face :width nil 'default-on)) (error 'no))
   'set-condensed (condition-case nil (progn (set-face-attribute 'my-width-cycle-face nil :width 'condensed) (face-attribute 'my-width-cycle-face :width nil 'default-on)) (error 'no))
   'set-normal (condition-case nil (progn (set-face-attribute 'my-width-cycle-face nil :width 'normal) (face-attribute 'my-width-cycle-face :width nil 'default-on)) (error 'no))
   'set-expanded (condition-case nil (progn (set-face-attribute 'my-width-cycle-face nil :width 'expanded) (face-attribute 'my-width-cycle-face :width nil 'default-on)) (error 'no))
   'set-extra-expanded (condition-case nil (progn (set-face-attribute 'my-width-cycle-face nil :width 'extra-expanded) (face-attribute 'my-width-cycle-face :width nil 'default-on)) (error 'no))
   'set-unspec (condition-case nil (progn (set-face-attribute 'my-width-cycle-face nil :width 'unspecified) (face-attribute 'my-width-cycle-face :width nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_particle_face_overlay_priority_stacking_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 16))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 5))
    (let ((ov2 (make-overlay 1 16))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 15))
    (let ((ov3 (make-overlay 1 16))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 10))
    (let ((sorted (sort (overlays-at 5) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0))))))
      (list
       'sorted-priorities (mapcar (lambda (ov) (overlay-get ov 'priority)) sorted)
       'sorted-faces (mapcar (lambda (ov) (overlay-get ov 'face)) sorted)
       'effective-face (get-char-property 5 'face)
       (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned))))))"##,
    );
}

#[test]
fn ft_particle_face_text_properties_all_at_point_min_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Text properties at buffer boundaries face test content data")
    (put-text-property 1 52 'face 'bold :key 'value)
    (list
     'at-point-min (text-properties-at 1)
     'at-point-max (text-properties-at (point-max))
     'face-at-1 (get-text-property 1 'face)
     'face-at-max (get-text-property (point-max) 'face)
     'key-at-1 (get-text-property 1 'key)
     'key-at-max (get-text-property (point-max) 'key)
     'next-prop-from-min (next-single-property-change 1 'face)
     'prev-prop-from-max (previous-single-property-change (point-max) 'face nil 1)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_quanta_face_text_property_manual_iteration_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPP")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 3 5 'face 'italic)
    (put-text-property 5 7 'face 'underline)
    (put-text-property 7 9 'face '(:foreground "red"))
    (put-text-property 9 11 'face '(:background "yellow"))
    (let ((result nil) (pos 1))
      (while (< pos 11)
        (let ((face (get-text-property pos 'face)))
          (push (list pos face) result))
        (setq pos (1+ pos)))
      (nreverse result))))"##,
    );
}

#[test]
fn ft_quanta_font_lock_fontify_two_different_modes_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'emacs-lisp-faces (with-temp-buffer (emacs-lisp-mode) (insert "(defun f (x) x)") (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 10 14)))
   'text-mode-faces (with-temp-buffer (text-mode) (insert "Text mode faces test") (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 10 15)))
   'c-mode-faces (condition-case nil (with-temp-buffer (c-mode) (insert "int x = 42;") (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 4 7 10))) (error 'c-mode-error)))))"##,
    );
}

#[test]
fn ft_quanta_face_overlay_empty_make_and_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((ov (make-overlay 1 1)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'empty-start (overlay-start ov)
       'empty-end (overlay-end ov)
       'empty-face (overlay-get ov 'face)
       'empty-priority (overlay-get ov 'priority)
       'fill (progn (insert "ABCDEFGHIJ") (list 'start (overlay-start ov) 'end (overlay-end ov) 'face-at-1 (get-char-property 1 'face) 'face-at-5 (get-char-property 5 'face) 'face-at-10 (get-char-property 10 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_quanta_face_set_attribute_weight_symbol_table_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'font-weight-table (if (boundp 'font-weight-table) (length font-weight-table) 'no-table)
   'weight-thin (if (member 'thin (or (and (boundp 'font-weight-table) font-weight-table) '())) 'available 'not)
   'weight-ultra-light (if (member 'ultra-light (or (and (boundp 'font-weight-table) font-weight-table) '())) 'available 'not)
   'weight-light (if (member 'light (or (and (boundp 'font-weight-table) font-weight-table) '())) 'available 'not)
   'weight-normal (if (member 'normal (or (and (boundp 'font-weight-table) font-weight-table) '())) 'available 'not)
   'weight-bold (if (member 'bold (or (and (boundp 'font-weight-table) font-weight-table) '())) 'available 'not)
   'weight-heavy (if (member 'heavy (or (and (boundp 'font-weight-table) font-weight-table) '())) 'available 'not)
   'default-weight (face-attribute 'default :weight nil 'default-on)
   'default-slant (face-attribute 'default :slant nil 'default-on)
   'default-width (face-attribute 'default :width nil 'default-on))))"##,
    );
}

#[test]
fn ft_quanta_face_overlay_same_region_multi_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 16))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 50))
    (let ((ov2 (make-overlay 1 16))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 30))
    (let ((ov3 (make-overlay 1 16))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 10))
    (list
     'overlays-count (length (overlays-at 5))
     'effective-face (get-char-property 5 'face)
     'all-faces (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at 5) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority)))))
     (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_quanta_font_lock_fontify_no_keywords_at_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (setq-local font-lock-keywords nil)
    (setq-local font-lock-keywords-only nil)
    (insert "No keywords at all tested in fundamental mode buffer content text")
    (font-lock-fontify-buffer)
    (list
     'fontified (get-text-property 1 'fontified)
     'face (get-text-property 1 'face)
     'font-lock-keywords font-lock-keywords
     'font-lock-keywords-only (if (boundp 'font-lock-keywords-only) font-lock-keywords-only 'no-bound)
     (progn (kill-local-variable 'font-lock-keywords) (kill-local-variable 'font-lock-keywords-only) 'cleaned)))))"##,
    );
}

#[test]
fn ft_quanta_face_property_read_at_exact_midpoint() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (list
     'at-5-border (list (get-text-property 5 'face) (get-text-property 6 'face))
     'at-10-border (list (get-text-property 10 'face) (get-text-property 11 'face))
     'next-change-5 (next-single-property-change 5 'face nil 16)
     'next-change-6 (next-single-property-change 6 'face nil 16)
     'prev-change-6 (previous-single-property-change 6 'face nil 1)
     'prev-change-11 (previous-single-property-change 11 'face nil 1)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_quanta_face_overlay_face_get_after_properties_clear() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "test")
      (let ((v0 (list 'face (overlay-get ov 'face) 'priority (overlay-get ov 'priority) 'help (overlay-get ov 'help-echo) 'props-count (length (overlay-properties ov)))))
        (overlay-put ov 'face nil)
        (overlay-put ov 'priority nil)
        (overlay-put ov 'help-echo nil)
        (let ((v1 (list 'face-after-clear (overlay-get ov 'face) 'priority-after (overlay-get ov 'priority) 'help-after (overlay-get ov 'help-echo) 'props-after (length (overlay-properties ov)))))
          (list v0 v1 (progn (delete-overlay ov) 'cleaned))))))))"##,
    );
}

#[test]
fn ft_atom_face_overlay_all_properties_to_alist_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "overlay help")
      (overlay-put ov 'evaporate t)
      (overlay-put ov 'category 'my-category)
      (let ((props (overlay-properties ov)))
        (list
         'props-len (length props)
         'face (plist-get props 'face)
         'priority (plist-get props 'priority)
         'help-echo (plist-get props 'help-echo)
         'evaporate (plist-get props 'evaporate)
         'category (plist-get props 'category)
         'nonexistent (plist-get props 'nonexistent-property)
         (progn (delete-overlay ov) 'cleaned)))))))"##,
    );
}

#[test]
fn ft_atom_font_lock_fontify_different_font_lock_modes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'emacs-lisp (with-temp-buffer (emacs-lisp-mode) (font-lock-mode 1) (insert "(defun test () 1)") (font-lock-fontify-buffer) (list 'mode major-mode 'fontified (get-text-property 1 'fontified) 'face-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))))
   'text (with-temp-buffer (text-mode) (font-lock-mode 1) (insert "text mode test") (font-lock-fontify-buffer) (list 'mode major-mode 'fontified (get-text-property 1 'fontified) 'face (get-text-property 1 'face)))
   'fundamental (with-temp-buffer (fundamental-mode) (font-lock-mode 1) (insert "fundamental test") (font-lock-fontify-buffer) (list 'mode major-mode 'fontified (get-text-property 1 'fontified))))))"##,
    );
}

#[test]
fn ft_atom_face_property_interval_extract_all_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (put-text-property 21 25 'face '(:foreground "blue"))
    (put-text-property 25 29 'face '(:background "cyan"))
    (put-text-property 29 33 'face '(:slant italic))
    (put-text-property 33 37 'face '(:weight bold))
    (put-text-property 37 41 'face '(:underline t))
    (let ((intervals (object-intervals (current-buffer))))
      (list
       'count (length intervals)
       'all-starts (mapcar #'overlay-start intervals)
       'all-ends (mapcar #'overlay-end intervals)
       'all-faces (mapcar (lambda (ov) (get-text-property (overlay-start ov) 'face)) intervals))))))"##,
    );
}

#[test]
fn ft_atom_face_overlay_evaporate_with_face_after_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 10 25)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'evaporate t)
      (list
       'face-before (overlay-get ov 'face)
       'ov-alive (and (overlay-buffer ov) t)
       'delete-region (progn (delete-region 10 25) 'deleted)
       'ov-dead (not (and (overlay-buffer ov)))
       'face-at-9 (get-char-property 9 'face)
       'face-at-10 (get-char-property 10 'face)
       'face-at-25 (get-char-property 25 'face))))))"##,
    );
}

#[test]
fn ft_atom_font_lock_keywords_with_group_highlight_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "group1=value1 group2=value2 group3=value3 font lock keyword group test end")
    (font-lock-add-keywords nil
                            '(("\\(group[0-9]\\)\\(=\\)\\([a-z0-9]+\\)"
                               (1 font-lock-function-name-face)
                               (2 font-lock-keyword-face)
                               (3 font-lock-warning-face))))
    (font-lock-fontify-buffer)
    (list
     'g1-face (save-excursion (goto-char (point-min)) (search-forward "group1") (get-text-property (match-beginning 0) 'face))
     'eq-face (save-excursion (goto-char (point-min)) (search-forward "=") (get-text-property (match-beginning 0) 'face))
     'val1-face (save-excursion (goto-char (point-min)) (search-forward "value1") (get-text-property (match-beginning 0) 'face))
     'g2-face (save-excursion (goto-char (point-min)) (search-forward "group2") (get-text-property (match-beginning 0) 'face))
     'fontified (get-text-property 1 'fontified)))))"##,
    );
}

#[test]
fn ft_atom_face_overlay_priority_zero_vs_positive_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov0 (make-overlay 1 21))) (overlay-put ov0 'face '(:background "red")) (overlay-put ov0 'priority 0))
    (let ((ov-1 (make-overlay 1 21))) (overlay-put ov-1 'face '(:background "green")) (overlay-put ov-1 'priority 1))
    (let ((ov-2 (make-overlay 1 21))) (overlay-put ov-2 'face '(:background "blue")) (overlay-put ov-2 'priority 2))
    (list
     'effective (get-char-property 5 'face)
     'overlays-sorted (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at 5) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority)))))
     (progn (mapc #'delete-overlay (overlays-in 1 21)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_atom_face_set_face_strike_color_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-strike-color-face) (error nil))
  (list
   'default-strike (condition-case nil (face-attribute 'default :strike-through nil 'default-on) (error 'no))
   'set-strike-t (condition-case nil (progn (set-face-attribute 'my-strike-color-face nil :strike-through t) (face-attribute 'my-strike-color-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-red (condition-case nil (progn (set-face-attribute 'my-strike-color-face nil :strike-through '(:color "red")) (face-attribute 'my-strike-color-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-green (condition-case nil (progn (set-face-attribute 'my-strike-color-face nil :strike-through '(:color "green")) (face-attribute 'my-strike-color-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-off (condition-case nil (progn (set-face-attribute 'my-strike-color-face nil :strike-through nil) (face-attribute 'my-strike-color-face :strike-through nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_atom_face_text_property_search_any_in_full_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (put-text-property 21 25 'face '(:foreground "blue"))
    (put-text-property 25 29 'face '(:background "cyan"))
    (put-text-property 29 33 'face '(:slant italic))
    (put-text-property 33 37 'face '(:weight bold))
    (put-text-property 37 41 'face '(:underline t))
    (list
     'find-bold (text-property-any 1 41 'face 'bold)
     'find-red (text-property-any 1 41 'face '(:foreground "red"))
     'find-yellow (text-property-any 1 41 'face '(:background "yellow"))
     'find-weight-bold (text-property-any 1 41 'face '(:weight bold))
     'find-underline-t (text-property-any 1 41 'face '(:underline t))
     'find-none (text-property-any 1 41 'face 'nonexistent)
     'not-all-bold (text-property-not-all 1 41 'face 'bold)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_meson_face_font_lock_add_then_remove_blank_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "BLANK keyword handling font lock test buffer content text data")
    (font-lock-add-keywords nil '(("\\<\\(BLANK\\)\\>" 1 '(:foreground "red" :weight bold) t)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "BLANK") (get-text-property (match-beginning 0) 'face))))
      (font-lock-remove-keywords nil '())
      (font-lock-fontify-buffer)
      (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "BLANK") (get-text-property (match-beginning 0) 'face))))
        (font-lock-remove-keywords nil nil)
        (font-lock-fontify-buffer)
        (let ((v2 (save-excursion (goto-char (point-min)) (search-forward "BLANK") (get-text-property (match-beginning 0) 'face))))
          (font-lock-remove-keywords nil '(("\\<\\(BLANK\\)\\>" 1 '(:foreground "red" :weight bold) t)))
          (font-lock-fontify-buffer)
          (let ((v3 (save-excursion (goto-char (point-min)) (search-forward "BLANK") (get-text-property (match-beginning 0) 'face))))
            (list v0 v1 v2 v3)))))))"##,
    );
}

#[test]
fn ft_meson_face_overlay_both_advance_front_and_rear() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGG")
    (let ((ov (make-overlay 5 15 t t nil)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before-insert (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 4 5 10 15 16 20 25 30))
       'insert-front (progn (goto-char 5) (insert "FRONT") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 4 5 9 14 19 23 28 33)))
       'insert-rear (progn (goto-char 15) (insert "REAR") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 4 5 9 14 19 23 28 33 37)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_meson_face_text_property_interval_split_precise_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 6 8 10))
     'insert-at-3 (progn (goto-char 3) (insert "X") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 4 6 7 9 11)))
     'insert-at-6 (progn (goto-char 6) (insert "Y") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 4 6 7 8 10 12)))
     'insert-at-1 (progn (goto-char 1) (insert "Z") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 5 6 8 9 11 13)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_meson_font_lock_fontify_buffer_after_partial_unfontify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun partial-unfontify-test (x) (* x x))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 30 38))))
      (font-lock-unfontify-region 10 25)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 7 15 22 30 38))))
        (font-lock-fontify-buffer)
        (let ((v2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 30 38))))
          (list v0 v1 v2)))))))"##,
    );
}

#[test]
fn ft_meson_face_set_face_overline_with_various_attrs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-overline-test-face) (error nil))
  (list
   'set-over-t (condition-case nil (progn (set-face-attribute 'my-overline-test-face nil :overline t) (face-attribute 'my-overline-test-face :overline nil 'default-on)) (error 'no))
   'set-over-color (condition-case nil (progn (set-face-attribute 'my-overline-test-face nil :overline '(:color "red")) (face-attribute 'my-overline-test-face :overline nil 'default-on)) (error 'no))
   'set-over-color-style (condition-case nil (progn (set-face-attribute 'my-overline-test-face nil :overline '(:color "blue" :style wave)) (face-attribute 'my-overline-test-face :overline nil 'default-on)) (error 'no))
   'set-over-nil (condition-case nil (progn (set-face-attribute 'my-overline-test-face nil :overline nil) (face-attribute 'my-overline-test-face :overline nil 'default-on)) (error 'no))
   'default-over (condition-case nil (face-attribute 'default :overline nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_meson_face_overlay_before_string_get_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Before string overlay face get test content text data buffer")
    (let ((ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "[[BEFORE]]" 'face '(:foreground "red" :weight bold :slant italic) 'custom-prop 'custom-val))
      (list
       'overlay-face (overlay-get ov 'face)
       'before-string (overlay-get ov 'before-string)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-props-count (length (text-properties-at 0 (overlay-get ov 'before-string)))
       'before-custom-prop (get-text-property 0 'custom-prop (overlay-get ov 'before-string))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_meson_face_property_search_in_narrowed_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 9 'face 'italic)
    (put-text-property 9 13 'face 'underline)
    (put-text-property 13 17 'face '(:foreground "red"))
    (put-text-property 17 41 'face '(:background "yellow"))
    (list
     'search-full (list (text-property-any 1 41 'face 'bold) (text-property-any 1 41 'face 'italic))
     'narrow-to-1-20 (progn (narrow-to-region 1 20) (list (text-property-any 1 20 'face 'bold) (text-property-any 1 20 'face '(:foreground "red"))))
     (widen)
     'search-after-widen (list (text-property-any 1 41 'face '(:background "yellow"))))))))"##,
    );
}

#[test]
fn ft_meson_face_text_property_interval_count_after_edits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold :prop1 'val1)
    (put-text-property 6 11 'face 'italic :prop2 'val2)
    (let ((counts nil))
      (push (length (object-intervals (current-buffer))) counts)
      (goto-char 6) (insert "X") (push (length (object-intervals (current-buffer))) counts)
      (goto-char 3) (insert "YY") (push (length (object-intervals (current-buffer))) counts)
      (delete-region 1 8) (push (length (object-intervals (current-buffer))) counts)
      (nreverse counts))))"##,
    );
}

#[test]
fn ft_lepton_face_overlay_with_string_propertize_face_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Overlay string propertize face roundtrip test content data buffer")
    (let* ((s (propertize "[[BEFORE-STRING-FACE-TEST]]" 'face '(:foreground "red" :weight bold :slant italic :underline t)))
           (ov (make-overlay 15 35)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string s)
      (list
       'overlay-face (overlay-get ov 'face)
       'before-string (overlay-get ov 'before-string)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-fg (plist-get (get-text-property 0 (overlay-get ov 'before-string)) :foreground)
       'before-weight (plist-get (get-text-property 0 (overlay-get ov 'before-string)) :weight)
       'before-slant (plist-get (get-text-property 0 (overlay-get ov 'before-string)) :slant)
       'before-under (plist-get (get-text-property 0 (overlay-get ov 'before-string)) :underline)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_lepton_font_lock_fontify_buffer_double_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun double-check-test (x) (+ x 1))\n")
    (let ((r1 nil) (r2 nil))
      (font-lock-fontify-buffer)
      (setq r1 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 26 32)))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (setq r2 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 26 32)))
      (list r1 r2 (equal r1 r2))))))"##,
    );
}

#[test]
fn ft_lepton_face_set_face_attribute_height_absolute_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-abs-height-face) (error nil))
  (list
   'set-height-80 (condition-case nil (progn (set-face-attribute 'my-abs-height-face nil :height 80) (face-attribute 'my-abs-height-face :height nil 'default-on)) (error 'no))
   'set-height-120 (condition-case nil (progn (set-face-attribute 'my-abs-height-face nil :height 120) (face-attribute 'my-abs-height-face :height nil 'default-on)) (error 'no))
   'set-height-200 (condition-case nil (progn (set-face-attribute 'my-abs-height-face nil :height 200) (face-attribute 'my-abs-height-face :height nil 'default-on)) (error 'no))
   'set-height-0.5 (condition-case nil (progn (set-face-attribute 'my-abs-height-face nil :height 0.5) (face-attribute 'my-abs-height-face :height nil 'default-on)) (error 'no))
   'set-height-2.0 (condition-case nil (progn (set-face-attribute 'my-abs-height-face nil :height 2.0) (face-attribute 'my-abs-height-face :height nil 'default-on)) (error 'no))
   'set-height-unspec (condition-case nil (progn (set-face-attribute 'my-abs-height-face nil :height 'unspecified) (face-attribute 'my-abs-height-face :height nil 'default-on)) (error 'no))
   'default-height (face-attribute 'default :height nil 'default-on))))"##,
    );
}

#[test]
fn ft_lepton_face_property_get_intangible_text_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Intangible text property face test buffer content data")
    (put-text-property 1 15 'face 'bold)
    (put-text-property 15 25 'intangible t)
    (put-text-property 15 25 'face 'italic)
    (put-text-property 25 45 'face 'underline)
    (list
     'face-and-intangible (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'intangible))) '(1 10 15 20 25 30 40 44))
     'prop-change-intangible (next-single-property-change 1 'intangible nil 45)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_lepton_face_overlay_multiple_overlays_same_region_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 16))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 1 16))) (overlay-put ov2 'face '(:foreground "green")) (overlay-put ov2 'priority 10))
    (let ((ov3 (make-overlay 1 16))) (overlay-put ov3 'face '(:slant italic)) (overlay-put ov3 'priority 20))
    (list
     'same-prio-count (length (overlays-at 5))
     'effective-face (get-char-property 5 'face)
     'all-overlay-faces (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at 5) (lambda (a b) (> (overlay-get a 'priority) (overlay-get b 'priority)))))
     (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_lepton_font_lock_global_font_lock_available_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'global-font-lock-mode-fbound (fboundp 'global-font-lock-mode)
   'global-font-lock-mode-bound (boundp 'global-font-lock-mode)
   'global-font-lock-mode-value (if (boundp 'global-font-lock-mode) global-font-lock-mode 'no-bound)
   'font-lock-global-modes-bound (boundp 'font-lock-global-modes)
   'font-lock-global-modes-value (if (boundp 'font-lock-global-modes) font-lock-global-modes 'no-bound)
   'font-lock-support-mode-bound (boundp 'font-lock-support-mode)
   'font-lock-support-mode-value (if (boundp 'font-lock-support-mode) font-lock-support-mode 'no-bound)
   'jit-lock-mode-fbound (fboundp 'jit-lock-mode))))"##,
    );
}

#[test]
fn ft_lepton_face_text_property_prev_single_property_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (put-text-property 26 31 'face '(:foreground "blue"))
    (put-text-property 31 36 'face '(:background "cyan"))
    (list
     'prev-from-36 (previous-single-property-change 36 'face nil 1)
     'prev-from-25 (previous-single-property-change 25 'face nil 1)
     'prev-from-15 (previous-single-property-change 15 'face nil 1)
     'prev-from-5 (previous-single-property-change 5 'face nil 1)
     'all-prev (let ((pos 36) (result nil))
                 (while pos
                   (setq pos (previous-single-property-change pos 'face nil 1))
                   (when pos (push pos result)))
                 (nreverse result)))))"##,
    );
}

#[test]
fn ft_lepton_face_overlay_face_remap_and_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (put-text-property 1 26 'face '(:foreground "blue"))
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before-remap (get-char-property 10 'face)
       'add-remap-to-bold (condition-case nil (progn (face-remap-add-relative 'bold '(:foreground "red")) 'ok) (error 'no))
       'after-remap (get-char-property 10 'face)
       'remap-alist (face-remapping-alist)
       (condition-case nil (progn (face-remap-reset-base 'bold) 'reset) (error 'no))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_baryon_face_overlay_string_with_multiple_props_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 10 25)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "<<<" 'face '(:foreground "red" :weight bold) 'key1 'val1))
      (overlay-put ov 'after-string (propertize ">>>" 'face '(:foreground "blue" :slant italic) 'key2 'val2))
      (overlay-put ov 'display "")
      (list
       'ov-face (overlay-get ov 'face)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'after-face (get-text-property 0 (overlay-get ov 'after-string))
       'before-props-count (length (text-properties-at 0 (overlay-get ov 'before-string)))
       'after-props-count (length (text-properties-at 0 (overlay-get ov 'after-string)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_baryon_font_lock_add_keywords_prepend_overwrite_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "PREPEND overwrite test PREPEND keyword font lock face buffer")
    (font-lock-add-keywords nil '(("\\<\\(PREPEND\\)\\>" 1 '(:foreground "blue") prepend)))
    (font-lock-add-keywords nil '(("\\<\\(PREPEND\\)\\>" 1 '(:foreground "red" :weight bold) overwrite)))
    (font-lock-fontify-buffer)
    (list
     'prepend-face (save-excursion (goto-char (point-min)) (search-forward "PREPEND") (get-text-property (match-beginning 0) 'face))
     'prepend-face-next (save-excursion (search-forward "PREPEND") (get-text-property (match-beginning 0) 'face)))))"##,
    );
}

#[test]
fn ft_baryon_face_set_face_attribute_with_face_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'cus-face)
  (condition-case nil
      (progn
        (defface my-defface-test-face
          '((t :weight bold :foreground "dark blue" :underline t))
          "Test defface")
        (list
         'facep (facep 'my-defface-test-face)
         'weight (face-attribute 'my-defface-test-face :weight nil 'default-on)
         'fg (condition-case nil (face-foreground 'my-defface-test-face nil 'default-on) (error 'no))
         'underline (condition-case nil (face-attribute 'my-defface-test-face :underline nil 'default-on) (error 'no))
         (condition-case nil (progn (set-face-attribute 'my-defface-test-face nil :weight 'unspecified :foreground 'unspecified :underline 'unspecified) 'reset) (error 'no))))
    (error (list 'defface-error (fboundp 'defface))))))"##,
    );
}

#[test]
fn ft_baryon_face_overlay_empty_buffer_make_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (let ((ov (make-overlay 1 1)))
      (overlay-put ov 'face '(:background "red"))
      (list
       'empty-ov-start (overlay-start ov)
       'empty-ov-end (overlay-end ov)
       'empty-ov-face (overlay-get ov 'face)
       'fill (progn (insert "FILLED-CONTENT") (list 'ov-start (overlay-start ov) 'ov-end (overlay-end ov) 'face-at-1 (get-char-property 1 'face) 'face-at-7 (get-char-property 7 'face) 'face-at-14 (get-char-property 14 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_baryon_font_lock_fontify_all_at_once_vs_incremental() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (let ((content "(defun incr-test (a b) (+ a b))\n"))
    (list
     'all-at-once (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 15 20 25 31)))
     'incremental (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-region 1 15) (font-lock-fontify-region 15 (point-max)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 7 15 20 25 31)))
     'consistent (equal (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 31))) (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-region 1 15) (font-lock-fontify-region 15 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 31))))))))"##,
    );
}

#[test]
fn ft_baryon_face_text_property_next_property_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPP")
    (dotimes (i 16)
      (put-text-property (1+ (* i 2)) (+ (* i 2) 3) 'face (if (evenp i) 'bold 'italic)))
    (list
     'interval-count (length (object-intervals (current-buffer)))
     'next-from-1 (next-property-change 1)
     'next-single-from-1 (next-single-property-change 1 'face)
     'next-property-at-3 (next-property-change 3)
     'next-single-at-3 (next-single-property-change 3 'face)
     'next-single-at-30 (next-single-property-change 30 'face)
     'next-single-with-limit (next-single-property-change 1 'face nil 10)
     'next-nonexistent (next-single-property-change 1 'nonexistent-prop))))))"##,
    );
}

#[test]
fn ft_baryon_face_overlay_priority_change_repeated_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov1 (make-overlay 1 21))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 10))
    (let ((ov2 (make-overlay 1 21))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 20))
    (let ((snap (lambda () (get-char-property 5 'face))))
      (let ((v0 (funcall snap)))
        (overlay-put ov1 'priority 30) (let ((v1 (funcall snap)))
        (overlay-put ov2 'priority 40) (let ((v2 (funcall snap)))
        (overlay-put ov1 'priority 5) (let ((v3 (funcall snap)))
        (list v0 v1 v2 v3 (progn (mapc #'delete-overlay (overlays-in 1 21)) 'cleaned))))))))))"##,
    );
}

#[test]
fn ft_baryon_face_color_complement_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'complement-red (condition-case nil (color-complement "#FF0000") (error 'no))
   'complement-green (condition-case nil (color-complement "#00FF00") (error 'no))
   'complement-blue (condition-case nil (color-complement "#0000FF") (error 'no))
   'complement-white (condition-case nil (color-complement "#FFFFFF") (error 'no))
   'complement-black (condition-case nil (color-complement "#000000") (error 'no))
   'complement-named (condition-case nil (color-complement "red") (error 'no))
   'color-gradient (condition-case nil (color-gradient '(0 0 0) '(1 1 1) 5) (error 'no))
   'color-hsl-to-rgb (condition-case nil (color-hsl-to-rgb 0 1 0.5) (error 'no))
   'color-rgb-to-hsl (condition-case nil (color-rgb-to-hsl 1 0 0) (error 'no)))))"##,
    );
}

#[test]
fn ft_hadron_face_overlay_face_set_unset_set_again() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (let ((v0 (overlay-get ov 'face)))
        (overlay-put ov 'face nil)
        (let ((v1 (overlay-get ov 'face)))
          (overlay-put ov 'face '(:foreground "red" :weight bold))
          (list v0 v1 (overlay-get ov 'face) (get-char-property 5 'face)
                (progn (delete-overlay ov) 'cleaned))))))))"##,
    );
}

#[test]
fn ft_hadron_font_lock_fontify_with_syntactic_keywords_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; syntax comment\n\"syntax string\"\n(defun syntax-test () 42)\n")
    (font-lock-fontify-syntactically (point-min) (point-max) nil)
    (font-lock-fontify-keywords-region (point-min) (point-max) nil)
    (mapcar (lambda (needle) (save-excursion (goto-char (point-min)) (search-forward needle) (list needle (get-text-property (match-beginning 0) 'face)))) '("comment" "string" "defun" "syntax-test" "42"))))"##,
    );
}

#[test]
fn ft_hadron_face_text_property_read_after_different_setters() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Read after different setters face property test buffer content data")
    (list
     'put-text (progn (put-text-property 1 10 'face 'bold) (get-text-property 1 'face))
     'add-face (progn (add-face-text-property 10 20 '(:foreground "red")) (get-text-property 15 'face))
     'add-text-props (progn (add-text-properties 20 30 (list 'face 'italic 'key 'val)) (get-text-property 25 'face))
     'set-text-props (progn (set-text-properties 30 40 (list 'face 'underline 'key2 'val2)) (get-text-property 35 'face))
     'all-faces (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 5 12 17 22 28 32 38 50)))))"##,
    );
}

#[test]
fn ft_hadron_face_overlay_window_specific_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 16))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 30) (overlay-put ov1 'window (selected-window)))
    (let ((ov2 (make-overlay 1 16))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 10) (overlay-put ov2 'window nil))
    (let ((ov3 (make-overlay 1 16))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 20))
    (list
     'effective-face (get-char-property 5 'face)
     'sorted-overlays (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face) (overlay-get ov 'window))) (sort (overlays-at 5) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0)))))
     (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_hadron_font_lock_fontify_region_with_start_end_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun region-params-test (x) (+ x 1))\n")
    (list
     'fontify-1-10 (progn (font-lock-fontify-region 1 10) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15)))
     'fontify-10-20 (progn (font-lock-fontify-region 10 20) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 5 10 15 20)))
     'fontify-rest (progn (font-lock-fontify-region 20 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 15 20 25 30 35)))
     'faces-after (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 25 30 35))))))"##,
    );
}

#[test]
fn ft_hadron_face_set_face_font_attribute_xlfd_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-xlfd-font-face) (error nil))
  (list
   'default-font (condition-case nil (face-font 'default nil) (error 'no))
   'default-font-xlfd (condition-case nil (let ((f (face-font 'default nil))) (if (fontp f) (font-xlfd-name f) 'not-a-font)) (error 'no))
   'set-font-by-spec (condition-case nil (progn (set-face-font 'my-xlfd-font-face (font-spec :family "Monospace" :size 12 :weight 'bold) nil) 'ok) (error 'no))
   'get-font-after (condition-case nil (face-font 'my-xlfd-font-face nil) (error 'no))
   'get-font-xlfd (condition-case nil (let ((f (face-font 'my-xlfd-font-face nil))) (if (fontp f) (font-xlfd-name f) 'not-font)) (error 'no))
   'reset-font (condition-case nil (progn (set-face-font 'my-xlfd-font-face 'unspecified nil) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_hadron_face_property_change_increment_counter_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((count 0))
      (put-text-property 1 6 'face 'bold) (setq count (1+ count))
      (put-text-property 6 11 'face 'italic) (setq count (1+ count))
      (put-text-property 11 16 'face 'underline) (setq count (1+ count))
      (put-text-property 16 21 'face '(:foreground "red")) (setq count (1+ count))
      (list
       'prop-changes-count count
       'faces-after (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 10 11 15 16 20))
       'interval-count (length (object-intervals (current-buffer)))))))"##,
    );
}

#[test]
fn ft_hadron_face_overlay_face_with_evaporate_and_before_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov (make-overlay 10 25)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'evaporate t)
      (overlay-put ov 'before-string (propertize ">>" 'face '(:foreground "red")))
      (list
       'ov-face (overlay-get ov 'face)
       'ov-evap (overlay-get ov 'evaporate)
       'before-str (overlay-get ov 'before-string)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'at-before-del (get-char-property 15 'face)
       (progn (delete-region 10 25) (list 'ov-gone (not (and (overlay-buffer ov))) 'face-at-10 (get-char-property 10 'face))))))))"##,
    );
}

#[test]
fn ft_nucleon_face_font_lock_add_duplicate_and_remove_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "DUPLICATE keyword DUPLICATE test font lock face buffer content here")
    (font-lock-add-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "red") t)))
    (font-lock-add-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "blue") t)))
    (font-lock-add-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "green" :weight bold) overwrite)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "DUPLICATE") (get-text-property (match-beginning 0) 'face))))
      (font-lock-remove-keywords nil '(("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "red") t) ("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "blue") t) ("\\<\\(DUPLICATE\\)\\>" 1 '(:foreground "green" :weight bold) overwrite)))
      (font-lock-fontify-buffer)
      (list v0 (save-excursion (goto-char (point-min)) (search-forward "DUPLICATE") (get-text-property (match-beginning 0) 'face)))))))"##,
    );
}

#[test]
fn ft_nucleon_face_overlay_face_after_property_overwrite() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "red"))
      (overlay-put ov 'face '(:background "green"))
      (overlay-put ov 'face '(:background "blue" :foreground "white"))
      (overlay-put ov 'priority 10)
      (overlay-put ov 'priority 50)
      (list
       'final-face (overlay-get ov 'face)
       'final-priority (overlay-get ov 'priority)
       'props-count (length (overlay-properties ov))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_nucleon_face_text_property_previous_property_change_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPP")
    (dotimes (i 16)
      (put-text-property (1+ (* i 2)) (+ (* i 2) 3) 'face (if (evenp i) 'bold 'italic)))
    (list
     'prev-from-31 (previous-property-change 31)
     'prev-single-from-31 (previous-single-property-change 31 'face)
     'prev-from-15 (previous-property-change 15)
     'prev-single-from-15 (previous-single-property-change 15 'face)
     'prev-single-from-5 (previous-single-property-change 5 'face)
     'prev-single-with-limit (previous-single-property-change 31 'face nil 10)
     'prev-nil-prop (previous-single-property-change 31 'nonexistent-prop)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_nucleon_font_lock_fontify_with_jit_lock_disabled() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (condition-case nil
      (let ((jit-lock-mode nil))
        (with-temp-buffer
          (emacs-lisp-mode)
          (insert "(defun jit-test () 42)\n")
          (font-lock-fontify-buffer)
          (list
           'jit-lock-mode jit-lock-mode
           'face-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
           'fontified (get-text-property 1 'fontified))))
    (error (list 'jit-error (fboundp 'jit-lock-mode) (fboundp 'font-lock-fontify-buffer))))))"##,
    );
}

#[test]
fn ft_nucleon_face_set_face_underline_then_check_color() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-color-face) (error nil))
  (list
   'set-ul-red (condition-case nil (progn (set-face-underline 'my-ul-color-face '(:color "red") nil) (condition-case nil (face-attribute 'my-ul-color-face :underline nil 'default-on) (error 'no2))) (error 'no1))
   'set-ul-blue (condition-case nil (progn (set-face-underline 'my-ul-color-face '(:color "blue" :style wave) nil) (condition-case nil (face-attribute 'my-ul-color-face :underline nil 'default-on) (error 'no2))) (error 'no1))
   'set-ul-green (condition-case nil (progn (set-face-underline 'my-ul-color-face '(:color "green" :style double-line) nil) (condition-case nil (face-attribute 'my-ul-color-face :underline nil 'default-on) (error 'no2))) (error 'no1))
   'clear-ul (condition-case nil (progn (set-face-underline 'my-ul-color-face nil nil) (condition-case nil (face-attribute 'my-ul-color-face :underline nil 'default-on) (error 'no2))) (error 'no1)))))"##,
    );
}

#[test]
fn ft_nucleon_face_overlay_priority_negative_vs_positive_vs_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov-neg (make-overlay 1 31))) (overlay-put ov-neg 'face '(:background "red")) (overlay-put ov-neg 'priority -10))
    (let ((ov-zero (make-overlay 1 31))) (overlay-put ov-zero 'face '(:background "green")) (overlay-put ov-zero 'priority 0))
    (let ((ov-pos (make-overlay 1 31))) (overlay-put ov-pos 'face '(:background "blue")) (overlay-put ov-pos 'priority 10))
    (list
     'effective-face (get-char-property 5 'face)
     'sorted-priorities (mapcar (lambda (ov) (list (overlay-start ov) (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at 5) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0)))))
     (progn (mapc #'delete-overlay (overlays-in 1 31)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_nucleon_face_font_lock_fontify_buffer_full_vs_partial_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (let ((content "(defun eq-test (a b) (+ a b))\n"))
    (list
     'full-fontify (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 20 26 31)))
     'partial-fontify (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-region 1 15) (font-lock-fontify-region 15 (point-max)) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 20 26 31)))
     'consistent (equal (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 26 31))) (with-temp-buffer (emacs-lisp-mode) (insert content) (font-lock-fontify-region 1 15) (font-lock-fontify-region 15 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 26 31))))))))"##,
    );
}

#[test]
fn ft_nucleon_face_text_property_find_with_complex_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPPQQRRSSTTUUVVWWXXYYZZ")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 19 21 'face '(:foreground "red"))
    (put-text-property 37 39 'face 'underline)
    (put-text-property 49 51 'face '(:background "yellow" :weight bold))
    (list
     'find-bold (text-property-any 1 51 'face 'bold)
     'find-red (text-property-any 1 51 'face '(:foreground "red"))
     'find-underline (text-property-any 1 51 'face 'underline)
     'find-complex (text-property-any 1 51 'face '(:background "yellow" :weight bold))
     'not-all (text-property-not-all 1 51 'face 'bold)
     'find-nil (text-property-any 1 51 'face nil)
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_electron_face_font_lock_unfontify_partial_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun part-unfontify (x) x)\n")
    (font-lock-fontify-buffer)
    (list
     'all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 25))
     'unfontify-middle (progn (font-lock-unfontify-region 10 20) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 10 15 20 25)))
     'unfontify-all (progn (font-lock-unfontify-region 1 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 25)))
     'refontify (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 25)))))))"##,
    );
}

#[test]
fn ft_electron_face_overlay_face_with_category_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov1 (make-overlay 1 20))) (overlay-put ov1 'category 'cat-a) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 30))
    (let ((ov2 (make-overlay 10 30))) (overlay-put ov2 'category 'cat-b) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 20))
    (let ((ov3 (make-overlay 25 36))) (overlay-put ov3 'category 'cat-c) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 10))
    (list
     'face-cat-stack (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face) (get-char-property pos 'category))) '(1 10 15 20 25 30 35))
     'all-cats (mapcar (lambda (ov) (overlay-get ov 'category)) (list ov1 ov2 ov3))
     (progn (mapc #'delete-overlay (overlays-in 1 36)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_electron_face_set_face_box_with_multiple_style_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-box-styles-face) (error nil))
  (list
   'box-t (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box t) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-width-3 (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:line-width 3)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-released (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:style released-button :line-width 2)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-pressed (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:style pressed-button :color "red" :line-width 3)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-flat (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box '(:style flat-button :line-width 1)) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no))
   'box-none (condition-case nil (progn (set-face-attribute 'my-box-styles-face nil :box nil) (face-attribute 'my-box-styles-face :box nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_electron_face_text_property_search_any_char_by_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    (put-text-property 1 5 'face 'bold)
    (put-text-property 5 10 'face 'italic)
    (put-text-property 10 15 'face 'underline)
    (put-text-property 15 20 'face '(:foreground "red"))
    (put-text-property 20 27 'face '(:background "yellow"))
    (list
     'char-faces (let ((i 1) (result nil))
                   (while (< i 27)
                     (push (list i (get-text-property i 'face)) result)
                     (setq i (1+ i)))
                   (nreverse result))
     'prop-boundary-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_electron_font_lock_fontify_syntactic_by_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; comment-only\n\"string-only\"\n(defun syn-region-test () 42)\n")
    (font-lock-fontify-syntactically (point-min) (point-max) nil)
    (list
     'comment-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 5 10 13))
     'string-region (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(15 20 25 28))
     'after-keywords (progn (font-lock-fontify-keywords-region (point-min) (point-max) nil) (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(28 35 40 46 49)))))))"##,
    );
}

#[test]
fn ft_electron_face_overlay_before_string_empty_vs_nonempty() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov-empty (make-overlay 1 6))) (overlay-put ov-empty 'face '(:background "red")) (overlay-put ov-empty 'before-string ""))
    (let ((ov-nonempty (make-overlay 6 11))) (overlay-put ov-nonempty 'face '(:background "green")) (overlay-put ov-nonempty 'before-string (propertize "B" 'face '(:foreground "blue"))))
    (let ((ov-nil (make-overlay 11 16))) (overlay-put ov-nil 'face '(:background "yellow")))
    (list
     'empty-before (overlay-get ov-empty 'before-string)
     'nonempty-before (overlay-get ov-nonempty 'before-string)
     'nil-before (overlay-get ov-nil 'before-string)
     'face-at-3 (get-char-property 3 'face)
     'face-at-8 (get-char-property 8 'face)
     'face-at-13 (get-char-property 13 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_electron_face_set_attribute_stipple_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-stipple (condition-case nil (face-attribute 'default :stipple nil 'default-on) (error 'no))
   'bold-stipple (condition-case nil (face-attribute 'bold :stipple nil 'default-on) (error 'no))
   'italic-stipple (condition-case nil (face-attribute 'italic :stipple nil 'default-on) (error 'no))
   'set-face-stipple-fbound (fboundp 'set-face-stipple)
   'face-stipple-fbound (fboundp 'face-stipple))))"##,
    );
}

#[test]
fn ft_electron_face_face_documentation_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-doc (condition-case nil (face-documentation 'default) (error 'no))
   'bold-doc (condition-case nil (face-documentation 'bold) (error 'no))
   'fringe-doc (condition-case nil (face-documentation 'fringe) (error 'no))
   'face-documentation-fbound (fboundp 'face-documentation)
   'set-face-documentation-fbound (fboundp 'set-face-documentation))))"##,
    );
}

#[test]
fn ft_final2_face_text_property_char_by_char_interval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJKLMNOP")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 3 6 'face 'italic)
    (put-text-property 6 10 'face 'underline)
    (put-text-property 10 15 'face '(:foreground "red"))
    (put-text-property 15 17 'face '(:background "yellow"))
    (list
     'char-faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 4 5 6 8 10 12 14 15 16))
     'interval-starts (mapcar (lambda (ov) (overlay-start ov)) (object-intervals (current-buffer)))
     'interval-ends (mapcar (lambda (ov) (overlay-end ov)) (object-intervals (current-buffer)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_final2_font_lock_fontify_buffer_and_unfontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun fontify-unfontify-final (x) (+ x 1))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 30 38))))
      (font-lock-unfontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'fontified))) '(1 7 15 22 30 38))))
        (font-lock-fontify-buffer)
        (let ((v2 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 30 38))))
          (list v0 v1 v2 (equal v0 v2))))))))"##,
    );
}

#[test]
fn ft_final2_face_overlay_start_end_equal_propagation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 1))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'before-string "[S]"))
    (let ((ov2 (make-overlay 16 16))) (overlay-put ov2 'face '(:background "blue")) (overlay-put ov2 'after-string "[E]"))
    (list
     'start-face (overlay-get ov1 'face)
     'start-before (overlay-get ov1 'before-string)
     'end-face (overlay-get ov2 'face)
     'end-after (overlay-get ov2 'after-string)
     'face-at-1 (get-char-property 1 'face)
     'face-at-16 (get-char-property 16 'face)
     'after-insert (progn (goto-char 16) (insert "END") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 16 19)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned)))))"##,
    );
}

#[test]
fn ft_final2_face_set_attribute_weight_with_symbol_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-weight-sym-face) (error nil))
  (list
   'set-normal (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'normal) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no))
   'set-bold (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'bold) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no))
   'set-light (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'light) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no))
   'set-heavy (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'heavy) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no))
   'set-extra-bold (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'extra-bold) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no))
   'set-ultra-light (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'ultra-light) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no))
   'set-unspecified (condition-case nil (progn (set-face-attribute 'my-weight-sym-face nil :weight 'unspecified) (face-attribute 'my-weight-sym-face :weight nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_final2_face_font_lock_global_mode_toggle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'global-mode-fbound (fboundp 'global-font-lock-mode)
   'turn-on-global (condition-case nil (progn (global-font-lock-mode 1) 'on) (error 'no))
   'global-mode-var (if (boundp 'global-font-lock-mode) global-font-lock-mode 'no-bound)
   'turn-off-global (condition-case nil (progn (global-font-lock-mode -1) 'off) (error 'no))
   'global-mode-var-after (if (boundp 'global-font-lock-mode) global-font-lock-mode 'no-bound)
   (condition-case nil (progn (global-font-lock-mode 1) 'restored) (error 'no)))))"##,
    );
}

#[test]
fn ft_final2_face_overlay_face_remove_then_readd_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before (get-char-property 5 'face)
       'remove-face (progn (overlay-put ov 'face nil) (get-char-property 5 'face))
       'readd-face (progn (overlay-put ov 'face '(:foreground "red" :weight bold)) (get-char-property 5 'face))
       'remove-again (progn (overlay-put ov 'face nil) (get-char-property 5 'face))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_final2_face_text_property_all_at_point_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "All text properties at point face test content data buffer")
    (add-text-properties 1 51 (list 'face 'bold 'a 1 'b 2 'c 3 'd 4 'e 5 'f 6 'g 7 'h 8 'i 9 'j 10))
    (list
     'at-1 (text-properties-at 1)
     'at-25 (text-properties-at 25)
     'at-50 (text-properties-at 50)
     'props-count-1 (length (text-properties-at 1))
     'props-count-25 (length (text-properties-at 25))
     'face-at-1 (get-text-property 1 'face)
     'face-at-25 (get-text-property 25 'face)
     'face-at-50 (get-text-property 50 'face))))))"##,
    );
}

#[test]
fn ft_final2_face_font_lock_default_settings_consistency_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'font-lock-verbose-bound (boundp 'font-lock-verbose)
   'font-lock-max-dec-bound (boundp 'font-lock-maximum-decoration)
   'font-lock-support-mode-bound (boundp 'font-lock-support-mode)
   'font-lock-global-modes-bound (boundp 'font-lock-global-modes)
   'font-lock-keywords-case-fold-bound (boundp 'font-lock-keywords-case-fold-search)
   'font-lock-verbose-val (if (boundp 'font-lock-verbose) font-lock-verbose 'no)
   'font-lock-max-dec-val (if (boundp 'font-lock-maximum-decoration) font-lock-maximum-decoration 'no)
   'font-lock-support-val (if (boundp 'font-lock-support-mode) font-lock-support-mode 'no)
   'font-lock-global-modes-val (if (boundp 'font-lock-global-modes) font-lock-global-modes 'no)
   'font-lock-case-fold-val (if (boundp 'font-lock-keywords-case-fold-search) font-lock-keywords-case-fold-search 'no))))"##,
    );
}

#[test]
fn ft_ultimate_face_overlay_multiple_categories_same_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov1 (make-overlay 1 16))) (overlay-put ov1 'category 'type-a) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 1 16))) (overlay-put ov2 'category 'type-b) (overlay-put ov2 'face '(:foreground "green")))
    (let ((ov3 (make-overlay 1 16))) (overlay-put ov3 'category 'type-c) (overlay-put ov3 'face '(:underline t)))
    (list
     'all-categories (mapcar (lambda (ov) (overlay-get ov 'category)) (overlays-at 5))
     'effective-face (get-char-property 5 'face)
     'effective-category (get-char-property 5 'category)
     (progn (mapc #'delete-overlay (overlays-in 1 16)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_ultimate_font_lock_fontify_unfontify_boundary_stress() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun boundary-stress (a b c d e f g h) (+ a b c d e f g h))\n")
    (font-lock-fontify-region 1 10)
    (let ((v0 (get-text-property 1 'fontified)))
      (font-lock-unfontify-region 5 15)
      (let ((v1 (get-text-property 1 'fontified)))
        (font-lock-fontify-region 1 20)
        (let ((v2 (get-text-property 1 'fontified)))
          (list v0 v1 v2 (get-text-property 5 'fontified) (get-text-property 7 'face) (get-text-property 50 'fontified))))))))"##,
    );
}

#[test]
fn ft_ultimate_face_set_face_attribute_foreground_background_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-fg-bg-face) (error nil))
  (list
   'default-fg (condition-case nil (face-foreground 'default nil 'default-on) (error 'no))
   'default-bg (condition-case nil (face-background 'default nil 'default-on) (error 'no))
   'set-fg-red (condition-case nil (progn (set-face-foreground 'my-fg-bg-face "red" nil) (face-foreground 'my-fg-bg-face nil 'default-on)) (error 'no))
   'set-bg-yellow (condition-case nil (progn (set-face-background 'my-fg-bg-face "yellow" nil) (face-background 'my-fg-bg-face nil 'default-on)) (error 'no))
   'set-fg-blue (condition-case nil (progn (set-face-foreground 'my-fg-bg-face "blue" nil) (face-foreground 'my-fg-bg-face nil 'default-on)) (error 'no))
   'set-bg-white (condition-case nil (progn (set-face-background 'my-fg-bg-face "white" nil) (face-background 'my-fg-bg-face nil 'default-on)) (error 'no))
   'set-fg-unspec (condition-case nil (progn (set-face-foreground 'my-fg-bg-face 'unspecified nil) (face-foreground 'my-fg-bg-face nil 'default-on)) (error 'no))
   'set-bg-unspec (condition-case nil (progn (set-face-background 'my-fg-bg-face 'unspecified nil) (face-background 'my-fg-bg-face nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_ultimate_face_overlay_face_when_moved_outside_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25))
       'move-to-start (progn (move-overlay ov 1 5) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 6 10 15 20)))
       'move-to-end (progn (move-overlay ov 30 35) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 15 20 25 30 35)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_ultimate_face_text_property_prop_change_boundaries_precise() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWWVVVVV")
    (put-text-property 1 6 'face 'bold :tag 'x)
    (put-text-property 6 11 'face 'italic :tag 'y)
    (put-text-property 11 16 'face 'underline :tag 'z)
    (put-text-property 16 21 'face '(:foreground "red") :tag 'w)
    (put-text-property 21 26 'face '(:background "yellow") :tag 'v)
    (list
     'all-tags (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'tag))) '(1 3 5 6 8 10 11 13 15 16 18 20 21 23 25))
     'prop-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 26)) '(1 6 11 16 21))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_ultimate_font_lock_add_keywords_then_refresh_fontify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "REFRESH keyword font lock face test buffer content text end")
    (font-lock-add-keywords nil '(("\\<\\(REFRESH\\)\\>" 1 '(:foreground "red" :weight bold) t)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "REFRESH") (get-text-property (match-beginning 0) 'face))))
      (font-lock-refresh-defaults)
      (font-lock-fontify-buffer)
      (let ((v1 (save-excursion (goto-char (point-min)) (search-forward "REFRESH") (get-text-property (match-beginning 0) 'face))))
        (font-lock-remove-keywords nil '(("\\<\\(REFRESH\\)\\>" 1 '(:foreground "red" :weight bold) t)))
        (list v0 v1))))))"##,
    );
}

#[test]
fn ft_ultimate_face_overlay_face_with_max_priority_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'prio-0 (progn (overlay-put ov 'priority 0) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       'prio-100 (progn (overlay-put ov 'priority 100) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       'prio-1000 (progn (overlay-put ov 'priority 1000) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       'prio--1 (progn (overlay-put ov 'priority -1) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_ultimate_face_font_lock_fontify_with_c_major_mode_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (condition-case nil
      (with-temp-buffer
        (c-mode)
        (insert "int main(int argc, char **argv) { return 0; }\n")
        (font-lock-fontify-buffer)
        (list
         'int-face (save-excursion (goto-char (point-min)) (search-forward "int") (get-text-property (match-beginning 0) 'face))
         'main-face (save-excursion (goto-char (point-min)) (search-forward "main") (get-text-property (match-beginning 0) 'face))
         'return-face (save-excursion (goto-char (point-min)) (search-forward "return") (get-text-property (match-beginning 0) 'face))
         'fontified (get-text-property 1 'fontified)))
    (error (list 'c-mode-error (fboundp 'c-mode) (fboundp 'font-lock-fontify-buffer))))))"##,
    );
}

#[test]
fn ft_omega_final_face_overlay_string_with_face_and_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 10 30)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "<<" 'face '(:foreground "red" :weight bold)))
      (overlay-put ov 'after-string (propertize ">>" 'face '(:foreground "blue" :slant italic)))
      (overlay-put ov 'display "")
      (list
       'ov-face (overlay-get ov 'face)
       'before-str (overlay-get ov 'before-string)
       'after-str (overlay-get ov 'after-string)
       'display-val (overlay-get ov 'display)
       'char-prop-at-20 (get-char-property 20 'face)
       'char-prop-at-5 (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_omega_final_font_lock_keywords_regexp_group_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "test@example.com user@domain.org name@company.net end here")
    (font-lock-add-keywords nil
                            '(("\\([a-z]+\\)@\\([a-z]+\\)\\.\\([a-z]+\\)"
                               (1 font-lock-function-name-face)
                               (2 font-lock-warning-face)
                               (3 font-lock-keyword-face))))
    (font-lock-fontify-buffer)
    (list
     'user1-face (save-excursion (goto-char (point-min)) (search-forward "test") (get-text-property (match-beginning 0) 'face))
     'dom1-face (save-excursion (goto-char (point-min)) (search-forward "example") (get-text-property (match-beginning 0) 'face))
     'tld1-face (save-excursion (goto-char (point-min)) (search-forward "com") (get-text-property (match-beginning 0) 'face))
     'user2-face (save-excursion (goto-char (point-min)) (search-forward "user") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_omega_final_face_overlay_face_priority_order_verify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ovs (list (let ((ov (make-overlay 1 16))) (overlay-put ov 'face '(:background "red")) (overlay-put ov 'priority 30) ov)
                     (let ((ov (make-overlay 1 16))) (overlay-put ov 'face '(:background "green")) (overlay-put ov 'priority 10) ov)
                     (let ((ov (make-overlay 1 16))) (overlay-put ov 'face '(:background "blue")) (overlay-put ov 'priority 20) ov))))
    (let ((sorted (sort (copy-sequence ovs) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0))))))
      (list
       'sorted-priorities (mapcar (lambda (ov) (overlay-get ov 'priority)) sorted)
       'highest-face (overlay-get (car sorted) 'face)
       'effective-face (get-char-property 5 'face)
       (progn (mapc #'delete-overlay ovs) 'cleaned))))))"##,
    );
}

#[test]
fn ft_omega_final_face_set_attribute_line_spacing_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ls-face) (error nil))
  (list
   'default-ls (condition-case nil (face-attribute 'default :line-spacing nil 'default-on) (error 'no))
   'set-ls-5 (condition-case nil (progn (set-face-attribute 'my-ls-face nil :line-spacing 5) (face-attribute 'my-ls-face :line-spacing nil 'default-on)) (error 'no))
   'set-ls-10 (condition-case nil (progn (set-face-attribute 'my-ls-face nil :line-spacing 10) (face-attribute 'my-ls-face :line-spacing nil 'default-on)) (error 'no))
   'set-ls-1.5 (condition-case nil (progn (set-face-attribute 'my-ls-face nil :line-spacing 1.5) (face-attribute 'my-ls-face :line-spacing nil 'default-on)) (error 'no))
   'set-ls-nil (condition-case nil (progn (set-face-attribute 'my-ls-face nil :line-spacing nil) (face-attribute 'my-ls-face :line-spacing nil 'default-on)) (error 'no))
   'set-ls-unspec (condition-case nil (progn (set-face-attribute 'my-ls-face nil :line-spacing 'unspecified) (face-attribute 'my-ls-face :line-spacing nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_omega_final_face_text_property_object_intervals_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (let ((counts nil))
      (push (length (object-intervals (current-buffer))) counts)
      (goto-char 2) (insert "A") (push (length (object-intervals (current-buffer))) counts)
      (put-text-property 1 3 'face 'bold) (push (length (object-intervals (current-buffer))) counts)
      (goto-char 2) (insert "B") (push (length (object-intervals (current-buffer))) counts)
      (delete-region 1 3) (push (length (object-intervals (current-buffer))) counts)
      (nreverse counts))))"##,
    );
}

#[test]
fn ft_omega_final_font_lock_add_remove_then_readd_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "ADD-REMOVE-ADD keyword test font lock face buffer content text end now final done")
    (let ((f (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face)))))
      (font-lock-add-keywords nil '(("\\<\\(ADD-REMOVE-ADD\\)\\>" 1 '(:foreground "red") t)))
      (font-lock-fontify-buffer)
      (let ((v0 (funcall f "ADD-REMOVE-ADD")))
        (font-lock-remove-keywords nil '(("\\<\\(ADD-REMOVE-ADD\\)\\>" 1 '(:foreground "red") t)))
        (font-lock-fontify-buffer)
        (let ((v1 (funcall f "ADD-REMOVE-ADD")))
          (font-lock-add-keywords nil '(("\\<\\(ADD-REMOVE-ADD\\)\\>" 1 '(:foreground "blue" :weight bold) t)))
          (font-lock-fontify-buffer)
          (list v0 v1 (funcall f "ADD-REMOVE-ADD"))))))))"##,
    );
}

#[test]
fn ft_omega_final_face_overlay_properties_get_then_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (let ((props (overlay-properties ov)))
        (list
         'props-len-before (length props)
         'add-help (progn (overlay-put ov 'help-echo "added later") (length (overlay-properties ov)))
         'remove-help (progn (overlay-put ov 'help-echo nil) (length (overlay-properties ov)))
         'face-still-there (overlay-get ov 'face)
         'priority-still-there (overlay-get ov 'priority)
         (progn (delete-overlay ov) 'cleaned))))))))"##,
    );
}

#[test]
fn ft_omega_final_face_remapping_alist_length_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'face-remap)
  (list
   'alist-before (length (face-remapping-alist))
   'add-default (progn (face-remap-add-relative 'default '(:weight bold)) (length (face-remapping-alist)))
   'add-bold (progn (face-remap-add-relative 'bold '(:foreground "red")) (length (face-remapping-alist)))
   'add-italic (progn (face-remap-add-relative 'italic '(:slant oblique)) (length (face-remapping-alist)))
   'reset-default (progn (face-remap-reset-base 'default) (length (face-remapping-alist)))
   'reset-bold (progn (face-remap-reset-base 'bold) (length (face-remapping-alist)))
   'reset-italic (progn (face-remap-reset-base 'italic) (length (face-remapping-alist)))
   'final-empty (null (face-remapping-alist)))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_overlay_without_face_prop_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (put-text-property 1 26 'face '(:foreground "blue"))
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'priority 100)
      (list
       'no-face-overlay (overlay-get ov 'face)
       'priority (overlay-get ov 'priority)
       'char-prop-at-10 (get-char-property 10 'face)
       'char-prop-at-3 (get-char-property 3 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_cosmic_final_font_lock_keywords_keep_flag_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "KEEP-FLAG keyword test font lock face buffer text content here end now done")
    (font-lock-add-keywords nil '(("\\<\\(KEEP-FLAG\\)\\>" 1 '(:foreground "red" :weight bold) keep)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "KEEP-FLAG") (get-text-property (match-beginning 0) 'face))))
      (font-lock-remove-keywords nil '(("\\<\\(KEEP-FLAG\\)\\>" 1 '(:foreground "red" :weight bold) keep)))
      (font-lock-fontify-buffer)
      (list v0 (save-excursion (goto-char (point-min)) (search-forward "KEEP-FLAG") (get-text-property (match-beginning 0) 'face)))))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_set_font_attribute_family_foundry_registry() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-family-info-face) (error nil))
  (list
   'default-family (face-attribute 'default :family nil 'default-on)
   'default-foundry (face-attribute 'default :foundry nil 'default-on)
   'default-registry (face-attribute 'default :registry nil 'default-on)
   'set-family (condition-case nil (progn (set-face-attribute 'my-family-info-face nil :family "Monospace") (face-attribute 'my-family-info-face :family nil 'default-on)) (error 'no))
   'set-foundry (condition-case nil (progn (set-face-attribute 'my-family-info-face nil :foundry "adobe") (face-attribute 'my-family-info-face :foundry nil 'default-on)) (error 'no))
   'set-registry (condition-case nil (progn (set-face-attribute 'my-family-info-face nil :registry "iso8859-1") (face-attribute 'my-family-info-face :registry nil 'default-on)) (error 'no))
   'reset-all (condition-case nil (progn (set-face-attribute 'my-family-info-face nil :family 'unspecified :foundry 'unspecified :registry 'unspecified) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_overlay_insert_in_front_hooks_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'insert-in-front-hooks (list 'ignore))
      (list
       'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25))
       'insert-in-front (progn (goto-char 6) (insert "F") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 7 10 15 20 25)))
       'insert-another (progn (goto-char 10) (insert "X") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 7 10 11 15 21 27)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_property_text_property_any_intervals() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AABBCCDDEEFFGGHHIIJJKKLLMMNNOOPP")
    (dotimes (i 16)
      (put-text-property (1+ (* i 2)) (+ (* i 2) 3) 'face (if (evenp i) 'bold 'italic)))
    (let ((intervals (object-intervals (current-buffer))))
      (list
       'interval-count (length intervals)
       'first-face (get-text-property (overlay-start (car intervals)) 'face)
       'second-face (get-text-property (overlay-start (cadr intervals)) 'face)
       'last-face (get-text-property (overlay-start (car (last intervals))) 'face))))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_font_lock_unfontify_then_refontify_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun unfontify-refontify-consistent-test () 42)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 25 40 45))))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 25 40 45))))
        (list (equal v0 v1) v0 v1))))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_set_face_overline_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-over-line-face) (error nil))
  (list
   'default-overline (condition-case nil (face-attribute 'default :overline nil 'default-on) (error 'no))
   'set-overline-t (condition-case nil (progn (set-face-attribute 'my-over-line-face nil :overline t) (face-attribute 'my-over-line-face :overline nil 'default-on)) (error 'no))
   'set-overline-color (condition-case nil (progn (set-face-attribute 'my-over-line-face nil :overline '(:color "red")) (face-attribute 'my-over-line-face :overline nil 'default-on)) (error 'no))
   'set-overline-color-style (condition-case nil (progn (set-face-attribute 'my-over-line-face nil :overline '(:color "blue" :style wave)) (face-attribute 'my-over-line-face :overline nil 'default-on)) (error 'no))
   'set-overline-off (condition-case nil (progn (set-face-attribute 'my-over-line-face nil :overline nil) (face-attribute 'my-over-line-face :overline nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_cosmic_final_face_text_property_next_single_change_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWW")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (list
     'next-1 (next-single-ic-property-change 1 'face nil 21)
     'next-6 (next-single-property-change 6 'face nil 21)
     'next-11 (next-single-property-change 11 'face nil 21)
     'next-16 (next-single-property-change 16 'face nil 21)
     'next-20 (next-single-property-change 20 'face nil 21)
     'last-at-16 (get-text-property 16 'face)
     'last-at-20 (get-text-property 20 'face))))))"##,
    );
}

#[test]
fn ft_final_boson_face_overlay_face_after_delete_region_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(1 6 10 15 20 25 30))
       'delete-inside (progn (delete-region 10 15) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 6 10 15 20 25)))
       'delete-outside (progn (delete-region 1 6) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(1 5 10 15 20)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_final_boson_font_lock_unfontify_region_with_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun un-font-region-bound (x) (+ x 1))\n")
    (font-lock-fontify-buffer)
    (list
     'all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 30 35 40))
     'unfontify-1-15 (progn (font-lock-unfontify-region 1 15) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 30)))
     'unfontify-15-end (progn (font-lock-unfontify-region 15 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 30)))
     'refontify-all (progn (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 30)))))))"##,
    );
}

#[test]
fn ft_final_boson_face_overlay_string_props_deep_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "<<<" 'face '(:foreground "red" :weight bold) 'tag 'before-tag))
      (overlay-put ov 'after-string (propertize ">>>" 'face '(:foreground "blue" :slant italic) 'tag 'after-tag))
      (list
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-tag (get-text-property 0 'tag (overlay-get ov 'before-string))
       'after-face (get-text-property 0 (overlay-get ov 'after-string))
       'after-tag (get-text-property 0 'tag (overlay-get ov 'after-string))
       'before-props-count (length (text-properties-at 0 (overlay-get ov 'before-string)))
       'after-props-count (length (text-properties-at 0 (overlay-get ov 'after-string)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_final_boson_face_text_property_empty_props_after_erase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Properties before erase face test content data buffer text")
    (put-text-property 1 51 'face 'bold :prop1 'val1)
    (list
     'before (list 'face (get-text-property 1 'face) 'prop1 (get-text-property 1 'prop1) 'intervals (length (object-intervals (current-buffer))))
     'after-erase (progn (erase-buffer) (list 'face (get-text-property 1 'face) 'prop1 (get-text-property 1 'prop1) 'intervals (length (object-intervals (current-buffer)))))
     'after-refill (progn (insert "Refilled") (put-text-property 1 9 'face 'italic) (list 'face (get-text-property 1 'face) 'intervals (length (object-intervals (current-buffer)))))))))"##,
    );
}

#[test]
fn ft_final_boson_face_set_attribute_color_hex_vs_name_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-hex-name-face) (error nil))
  (list
   'set-hex-red (condition-case nil (progn (set-face-foreground 'my-hex-name-face "#FF0000" nil) (face-foreground 'my-hex-name-face nil 'default-on)) (error 'no))
   'set-name-red (condition-case nil (progn (set-face-foreground 'my-hex-name-face "red" nil) (face-foreground 'my-hex-name-face nil 'default-on)) (error 'no))
   'set-hex-blue (condition-case nil (progn (set-face-foreground 'my-hex-name-face "#0000FF" nil) (face-foreground 'my-hex-name-face nil 'default-on)) (error 'no))
   'set-name-blue (condition-case nil (progn (set-face-foreground 'my-hex-name-face "blue" nil) (face-foreground 'my-hex-name-face nil 'default-on)) (error 'no))
   'set-hex-green (condition-case nil (progn (set-face-foreground 'my-hex-name-face "#00FF00" nil) (face-foreground 'my-hex-name-face nil 'default-on)) (error 'no))
   'set-name-green (condition-case nil (progn (set-face-foreground 'my-hex-name-face "green" nil) (face-foreground 'my-hex-name-face nil 'default-on)) (error 'no))
   'reset (condition-case nil (progn (set-face-foreground 'my-hex-name-face 'unspecified nil) 'ok) (error 'no)))))"##,
    );
}

#[test]
fn ft_final_boson_font_lock_set_defaults_and_fontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (font-lock-mode 1)
    (font-lock-set-defaults)
    (insert "(defun set-defaults-fontify-test () 42)\n")
    (font-lock-fontify-buffer)
    (list
     'face-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     'face-test (save-excursion (goto-char (point-min)) (search-forward "test") (get-text-property (match-beginning 0) 'face))
     'fontified (get-text-property 1 'fontified)
     'font-lock-defaults (condition-case nil (font-lock-defaults) (error 'no))
     'font-lock-keywords (if (boundp 'font-lock-keywords) (type-of font-lock-keywords) 'no-bound)))))"##,
    );
}

#[test]
fn ft_final_boson_face_overlay_priority_degree_check_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov-p (make-overlay 1 21))) (overlay-put ov-p 'face '(:background "red")) (overlay-put ov-p 'priority 5))
    (let ((ov-m (make-overlay 1 21))) (overlay-put ov-m 'face '(:background "green")) (overlay-put ov-m 'priority 10))
    (let ((ov-h (make-overlay 1 21))) (overlay-put ov-h 'face '(:background "blue")) (overlay-put ov-h 'priority 20))
    (list
     'lowest-face (overlay-get ov-p 'face)
     'mid-face (overlay-get ov-m 'face)
     'highest-face (overlay-get ov-h 'face)
     'effective (get-char-property 5 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 21)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_final_boson_face_text_property_charset_differences() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Hello 世界 مرحبا 🌍 Γειά")
    (put-text-property 1 7 'face 'bold)
    (put-text-property 7 10 'face 'italic)
    (put-text-property 10 15 'face 'underline)
    (put-text-property 15 17 'face '(:foreground "red"))
    (put-text-property 17 21 'face '(:background "yellow"))
    (list
     'faces-by-pos (mapcar (lambda (pos) (goto-char pos) (list pos (char-after pos) (get-text-property pos 'face) (char-width (or (char-after pos) 0)))) '(1 3 5 7 8 10 12 15 17 19))
     'string-width (string-width (buffer-string))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_final_fermion_face_overlay_no_priority_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'no-priority (overlay-get ov 'priority)
       'face-get (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_final_fermion_font_lock_fontify_buffer_unfontify_refontify_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun cycle-test () 42)\n")
    (let ((results nil))
      (font-lock-fontify-buffer) (push 'f1 results)
      (font-lock-unfontify-buffer) (push 'u1 results)
      (font-lock-fontify-buffer) (push 'f2 results)
      (font-lock-unfontify-buffer) (push 'u2 results)
      (font-lock-fontify-buffer) (push 'f3 results)
      (list (nreverse results) (get-text-property 1 'fontified) (get-text-property 7 'face))))))"##,
    );
}

#[test]
fn ft_final_fermion_face_set_attribute_with_relative_values_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-rel-values-face) (error nil))
  (list
   'height-1.0 (condition-case nil (progn (set-face-attribute 'my-rel-values-face nil :height 1.0) (face-attribute 'my-rel-values-face :height nil 'default-on)) (error 'no))
   'height-2.0 (condition-case nil (progn (set-face-attribute 'my-rel-values-face nil :height 2.0) (face-attribute 'my-rel-values-face :height nil 'default-on)) (error 'no))
   'height-100 (condition-case nil (progn (set-face-attribute 'my-rel-values-face nil :height 100) (face-attribute 'my-rel-values-face :height nil 'default-on)) (error 'no))
   'height-200 (condition-case nil (progn (set-face-attribute 'my-rel-values-face nil :height 200) (face-attribute 'my-rel-values-face :height nil 'default-on)) (error 'no))
   'height-0.5 (condition-case nil (progn (set-face-attribute 'my-rel-values-face nil :height 0.5) (face-attribute 'my-rel-values-face :height nil 'default-on)) (error 'no))
   'height-unspec (condition-case nil (progn (set-face-attribute 'my-rel-values-face nil :height 'unspecified) (face-attribute 'my-rel-values-face :height nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_final_fermion_face_overlay_face_after_move_and_resize_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (let ((snap (lambda () (get-char-property 10 'face))))
        (let ((v0 (funcall snap)))
          (move-overlay ov 20 30) (let ((v1 (funcall snap)))
          (move-overlay ov 1 10) (let ((v2 (funcall snap)))
          (delete-overlay ov)
          (list v0 v1 v2))))))))"##,
    );
}

#[test]
fn ft_final_fermion_face_text_property_interval_list_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWW")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (let ((intervals (object-intervals (current-buffer)))
          (result nil))
      (dolist (ov intervals)
        (push (list (overlay-start ov) (overlay-end ov) (get-text-property (overlay-start ov) 'face)) result))
      (nreverse result))))"##,
    );
}

#[test]
fn ft_final_fermion_font_lock_add_keywords_prepend_append_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "ORDER keyword ORDER test font lock face buffer content ORDER end")
    (font-lock-add-keywords nil '(("\\<\\(ORDER\\)\\>" 1 '(:foreground "blue") prepend)))
    (font-lock-add-keywords nil '(("\\<\\(ORDER\\)\\>" 1 '(:foreground "red") append)))
    (font-lock-add-keywords nil '(("\\<\\(ORDER\\)\\>" 1 '(:foreground "green" :weight bold) overwrite)))
    (font-lock-fontify-buffer)
    (list
     'order-face (save-excursion (goto-char (point-min)) (search-forward "ORDER") (get-text-property (match-beginning 0) 'face))
     'other-face (save-excursion (goto-char (point-min)) (search-forward "keyword") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_final_fermion_face_overlay_evaporate_auto_remove_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'evaporate t)
      (list
       'before-delete (list 'ov-alive (and (overlay-buffer ov) t) 'face (overlay-get ov 'face))
       'delete-region (progn (delete-region 6 15) (list 'ov-dead (not (and (overlay-buffer ov))) 'face-at-5 (get-char-property 5 'face) 'face-at-10 (get-char-property 10 'face))))))))"##,
    );
}

#[test]
fn ft_final_fermion_face_color_hex_roundtrip_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'name-to-rgb-red (color-name-to-rgb "red")
   'name-to-rgb-green (color-name-to-rgb "green")
   'name-to-rgb-blue (color-name-to-rgb "blue")
   'hex-ff0000 (color-name-to-rgb "#FF0000")
   'hex-00ff00 (color-name-to-rgb "#00FF00")
   'hex-0000ff (color-name-to-rgb "#0000FF")
   'hex-ff00ff (color-name-to-rgb "#FF00FF")
   'rgb-to-hex (apply 'color-rgb-to-hex (append (color-name-to-rgb "red") '(2)))
   'hex-roundtrip (equal (color-name-to-rgb "red") (color-name-to-rgb "#FF0000")))))"##,
    );
}

#[test]
fn ft_final_quark_face_overlay_face_string_at_buffer_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov-start (make-overlay 1 5))) (overlay-put ov-start 'face '(:background "red")) (overlay-put ov-start 'before-string "[S]"))
    (let ((ov-end (make-overlay 35 36))) (overlay-put ov-end 'face '(:background "blue")) (overlay-put ov-end 'after-string "[E]"))
    (list
     'start-face (overlay-get ov-start 'face)
     'end-face (overlay-get ov-end 'face)
     'face-at-1 (get-char-property 1 'face)
     'face-at-5 (get-char-property 5 'face)
     'face-at-35 (get-char-property 35 'face)
     'face-at-36 (get-char-property 36 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 36)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_final_quark_font_lock_add_keywords_with_null_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "NULL-MATCH keyword test font lock face buffer content text")
    (font-lock-add-keywords nil '(("\\<\\(NULL-MATCH\\)\\>" 1 font-lock-warning-face t)))
    (font-lock-fontify-buffer)
    (list
     'match-face (save-excursion (goto-char (point-min)) (search-forward "NULL-MATCH") (get-text-property (match-beginning 0) 'face))
     'non-match-face (save-excursion (goto-char (point-min)) (search-forward "keyword") (get-text-property (match-beginning 0) 'face))
     'fontified (get-text-property 1 'fontified)))))"##,
    );
}

#[test]
fn ft_final_quark_face_set_attribute_inherit_chain_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-inherit-chain-face) (error nil))
  (list
   'set-inherit-bold (condition-case nil (progn (set-face-attribute 'my-inherit-chain-face nil :inherit 'bold) (face-attribute 'my-inherit-chain-face :inherit nil 'default-on)) (error 'no))
   'weight-from-bold (face-attribute 'my-inherit-chain-face :weight nil 'default-on)
   'set-inherit-italic (condition-case nil (progn (set-face-attribute 'my-inherit-chain-face nil :inherit 'italic) (face-attribute 'my-inherit-chain-face :inherit nil 'default-on)) (error 'no))
   'slant-from-italic (face-attribute 'my-inherit-chain-face :slant nil 'default-on)
   'set-inherit-list (condition-case nil (progn (set-face-attribute 'my-inherit-chain-face nil :inherit '(bold italic)) (face-attribute 'my-inherit-chain-face :inherit nil 'default-on)) (error 'no))
   'inherit-list-get (face-attribute 'my-inherit-chain-face :inherit nil 'default-on)
   'reset (condition-case nil (progn (set-face-attribute 'my-inherit-chain-face nil :inherit 'unspecified) (face-attribute 'my-inherit-chain-face :inherit nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_final_quark_face_overlay_face_priority_sort_verify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ovs (list (let ((ov (make-overlay 1 21))) (overlay-put ov 'face '(:background "red")) (overlay-put ov 'priority 5) ov)
                     (let ((ov (make-overlay 1 21))) (overlay-put ov 'face '(:background "green")) (overlay-put ov 'priority 25) ov)
                     (let ((ov (make-overlay 1 21))) (overlay-put ov 'face '(:background "blue")) (overlay-put ov 'priority 15) ov))))
    (let ((sorted (sort (copy-sequence ovs) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0))))))
      (list
       'sorted-prios (mapcar (lambda (ov) (overlay-get ov 'priority)) sorted)
       'highest-prio (overlay-get (car sorted) 'priority)
       'highest-face (overlay-get (car sorted) 'face)
       'effective-face (get-char-property 5 'face)
       (progn (mapc #'delete-overlay ovs) 'cleaned))))))"##,
    );
}

#[test]
fn ft_final_quark_face_property_read_write_read_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Read write read cycle face property test buffer content data")
    (list
     'read-1 (get-text-property 1 'face)
     'write-bold (progn (put-text-property 1 20 'face 'bold) (get-text-property 1 'face))
     'write-italic (progn (put-text-property 1 20 'face 'italic) (get-text-property 1 'face))
     'write-underline (progn (put-text-property 1 20 'face 'underline) (get-text-property 1 'face))
     'write-plist (progn (put-text-property 1 20 'face '(:foreground "red" :weight bold)) (get-text-property 1 'face))
     'remove (progn (remove-text-properties 1 20 '(face nil)) (get-text-property 1 'face))
     'final (get-text-property 1 'face)))))"##,
    );
}

#[test]
fn ft_final_quark_font_lock_fontify_with_custom_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "CUSTOM-KW-1 CUSTOM-KW-2 CUSTOM-KW-3 font lock face test buffer end")
    (font-lock-add-keywords nil
                            '(("\\<\\(CUSTOM-KW-1\\)\\>" 1 '(:foreground "red") t)
                              ("\\<\\(CUSTOM-KW-2\\)\\>" 1 '(:foreground "green") t)
                              ("\\<\\(CUSTOM-KW-3\\)\\>" 1 '(:foreground "blue" :weight bold) t)))
    (font-lock-fontify-buffer)
    (mapcar (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (list n (get-text-property (match-beginning 0) 'face)))) '("CUSTOM-KW-1" "CUSTOM-KW-2" "CUSTOM-KW-3"))))"##,
    );
}

#[test]
fn ft_final_quark_face_color_defined_p_various_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'color-red (condition-case nil (color-defined-p "red") (error 'no))
   'color-green (condition-case nil (color-defined-p "green") (error 'no))
   'color-blue (condition-case nil (color-defined-p "blue") (error 'no))
   'color-ff0000 (condition-case nil (color-defined-p "#FF0000") (error 'no))
   'color-00ff00 (condition-case nil (color-defined-p "#00FF00") (error 'no))
   'color-0000ff (condition-case nil (color-defined-p "#0000FF") (error 'no))
   'color-invalid (condition-case nil (color-defined-p "#ZYXWVU") (error 'no))
   'color-not-a-color (condition-case nil (color-defined-p "not-a-color-name-at-all") (error 'no))))))"##,
    );
}

#[test]
fn ft_final_quark_face_overlay_face_with_zero_width_regions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov-z1 (make-overlay 5 5))) (overlay-put ov-z1 'face '(:background "red")))
    (let ((ov-z2 (make-overlay 15 15))) (overlay-put ov-z2 'face '(:background "green")))
    (let ((ov-z3 (make-overlay 25 25))) (overlay-put ov-z3 'face '(:background "blue")))
    (list
     'zero-width-faces (mapcar (lambda (ov) (list (overlay-start ov) (overlay-end ov) (overlay-get ov 'face))) (list ov-z1 ov-z2 ov-z3))
     'face-at-5 (get-char-property 5 'face)
     'face-at-15 (get-char-property 15 'face)
     'face-at-25 (get-char-property 25 'face)
     'after-insert (progn (goto-char 5) (insert "X") (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(4 5 6 10 15 25)))
     (progn (mapc #'delete-overlay (overlays-in 1 (point-max))) 'cleaned)))))"##,
    );
}

#[test]
fn ft_last_hope_face_overlay_face_with_nil_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'category nil)
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'nil-category (overlay-get ov 'category)
       'face (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_last_hope_font_lock_keywords_with_override_flag_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "OVERRIDE-FLAG keyword test font lock face buffer content text end")
    (font-lock-add-keywords nil '(("\\<\\(OVERRIDE-FLAG\\)\\>" 1 '(:foreground "blue") t)))
    (font-lock-add-keywords nil '(("\\<\\(OVERRIDE-FLAG\\)\\>" 1 '(:foreground "red" :weight bold) overwrite)))
    (font-lock-fontify-buffer)
    (list
     'override-face (save-excursion (goto-char (point-min)) (search-forward "OVERRIDE-FLAG") (get-text-property (match-beginning 0) 'face))
     'non-override (save-excursion (goto-char (point-min)) (search-forward "keyword") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_last_hope_face_set_face_underline_multiple_styles_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-ul-cycle-face) (error nil))
  (list
   'set-wave-red (condition-case nil (progn (set-face-underline 'my-ul-cycle-face '(:color "red" :style wave) nil) (face-attribute 'my-ul-cycle-face :underline nil 'default-on)) (error 'no))
   'set-line-blue (condition-case nil (progn (set-face-underline 'my-ul-cycle-face '(:color "blue" :style line) nil) (face-attribute 'my-ul-cycle-face :underline nil 'default-on)) (error 'no))
   'set-double-green (condition-case nil (progn (set-face-underline 'my-ul-cycle-face '(:color "green" :style double-line) nil) (face-attribute 'my-ul-cycle-face :underline nil 'default-on)) (error 'no))
   'set-tsimple (condition-case nil (progn (set-face-underline 'my-ul-cycle-face t nil) (face-attribute 'my-ul-cycle-face :underline nil 'default-on)) (error 'no))
   'set-off (condition-case nil (progn (set-face-underline 'my-ul-cycle-face nil nil) (face-attribute 'my-ul-cycle-face :underline nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_last_hope_face_overlay_priority_zero_and_negative_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov-pos (make-overlay 1 21))) (overlay-put ov-pos 'face '(:background "red")) (overlay-put ov-pos 'priority 5))
    (let ((ov-zero (make-overlay 1 21))) (overlay-put ov-zero 'face '(:background "green")) (overlay-put ov-zero 'priority 0))
    (let ((ov-neg (make-overlay 1 21))) (overlay-put ov-neg 'face '(:background "blue")) (overlay-put ov-neg 'priority -5))
    (list
     'effective (get-char-property 5 'face)
     'all-prios (mapcar (lambda (ov) (list (overlay-get ov 'priority) (overlay-get ov 'face))) (sort (overlays-at 5) (lambda (a b) (> (or (overlay-get a 'priority) 0) (or (overlay-get b 'priority) 0)))))
     (progn (mapc #'delete-overlay (overlays-in 1 21)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_last_hope_face_text_property_insert_boundary_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 6 8 10))
     'insert-at-1 (progn (goto-char 1) (insert "Z") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 4 6 7 9 11)))
     'insert-at-6 (progn (goto-char 6) (insert "Y") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 4 6 7 8 10 12)))
     'insert-at-11 (progn (goto-char 11) (insert "X") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 4 6 7 8 10 12 13 14))))))"##,
    );
}

#[test]
fn ft_last_hope_font_lock_unfontify_buffer_and_fontify_again() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun unfontify-again-test (x) (* x x))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 30 36))))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 30 36))))
        (list (equal v0 v1) v0 v1))))))"##,
    );
}

#[test]
fn ft_last_hope_face_property_interval_object_basic_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXYYYZZZWWW")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 7 'face 'italic)
    (put-text-property 7 10 'face 'underline)
    (put-text-property 10 13 'face '(:foreground "red"))
    (let ((intervals (object-intervals (current-buffer))))
      (list
       'count (length intervals)
       'first-start (overlay-start (car intervals))
       'first-end (overlay-end (car intervals))
       'face-at-first (get-text-property (overlay-start (car intervals)) 'face)
       'last-start (overlay-start (car (last intervals)))
       'last-end (overlay-end (car (last intervals)))
       'face-at-last (get-text-property (overlay-start (car (last intervals))) 'face))))))"##,
    );
}

#[test]
fn ft_last_hope_face_overlay_face_after_deleting_overlay_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov (make-overlay 10 25)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before (mapcar (lambda (pos) (goto-char pos) (list pos (get-char-property pos 'face))) '(5 10 15 20 25 30))
       'delete-partial (progn (delete-region 15 20) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25 28)))
       'delete-all (progn (delete-region 10 25) (mapcar (lambda (pos) (goto-char pos) (get-char-property pos 'face)) '(5 10 15 20 25)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neverend_face_overlay_all_properties_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "help")
      (overlay-put ov 'category 'my-cat)
      (let ((props (overlay-properties ov)))
        (list
         'props-count (length props)
         'keys (let ((ks nil) (i 0)) (while (< i (length props)) (push (nth i props) ks) (setq i (+ i 2))) (nreverse ks))
         'face (plist-get props 'face)
         'priority (plist-get props 'priority)
         'help (plist-get props 'help-echo)
         'category (plist-get props 'category)
         (progn (delete-overlay ov) 'cleaned)))))))"##,
    );
}

#[test]
fn ft_neverend_font_lock_unfontify_region_multi_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun multi-boundary-test (a b c d) (+ a b c d))\n")
    (font-lock-fontify-buffer)
    (list
     'all-fontified (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30 40 45))
     'unfontify-chunk1 (progn (font-lock-unfontify-region 1 15) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 15 20 30)))
     'unfontify-chunk2 (progn (font-lock-unfontify-region 15 30) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 15 20 30)))
     'unfontify-chunk3 (progn (font-lock-unfontify-region 30 (point-max)) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 15 30 40)))
     'refontify-all (progn (font-lock-fontify-buffer) (get-text-property 1 'fontified))))))"##,
    );
}

#[test]
fn ft_neverend_face_overlay_face_propagate_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow" :foreground "black" :weight bold))
      (overlay-put ov 'before-string (propertize "B4" 'face '(:foreground "red")))
      (overlay-put ov 'after-string (propertize "AF" 'face '(:foreground "blue")))
      (list
       'ov-face (overlay-get ov 'face)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'after-face (get-text-property 0 (overlay-get ov 'after-string))
       'char-prop-inside (get-char-property 10 'face)
       'char-prop-outside (get-char-property 3 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neverend_face_text_property_get_all_chars_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJ")
    (put-text-property 1 3 'face 'bold)
    (put-text-property 3 6 'face 'italic)
    (put-text-property 6 9 'face 'underline)
    (put-text-property 9 11 'face '(:foreground "red"))
    (list
     'all-chars (mapcar (lambda (pos) (goto-char pos) (list pos (char-after pos) (get-text-property pos 'face))) '(1 2 3 4 5 6 7 8 9 10))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_neverend_font_lock_add_keywords_remove_all_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "REMOVE-ALL-KW-1 REMOVE-ALL-KW-2 font lock face buffer test end")
    (font-lock-add-keywords nil
                            '(("\\<\\(REMOVE-ALL-KW-1\\)\\>" 1 '(:foreground "red") t)
                              ("\\<\\(REMOVE-ALL-KW-2\\)\\>" 1 '(:foreground "green") t)))
    (font-lock-fontify-buffer)
    (let ((f (lambda (n) (save-excursion (goto-char (point-min)) (search-forward n) (get-text-property (match-beginning 0) 'face)))))
      (let ((v0 (list (funcall f "REMOVE-ALL-KW-1") (funcall f "REMOVE-ALL-KW-2"))))
        (font-lock-remove-keywords nil
                                    '(("\\<\\(REMOVE-ALL-KW-1\\)\\>" 1 '(:foreground "red") t)
                                      ("\\<\\(REMOVE-ALL-KW-2\\)\\>" 1 '(:foreground "green") t)))
        (font-lock-fontify-buffer)
        (list v0 (list (funcall f "REMOVE-ALL-KW-1") (funcall f "REMOVE-ALL-KW-2"))))))))"##,
    );
}

#[test]
fn ft_neverend_face_set_attribute_inverse_vid_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-inverse-vid-face) (error nil))
  (list
   'default-inverse (condition-case nil (face-attribute 'default :inverse-video nil 'default-on) (error 'no))
   'set-inverse-t (condition-case nil (progn (set-face-attribute 'my-inverse-vid-face nil :inverse-video t) (face-attribute 'my-inverse-vid-face :inverse-video nil 'default-on)) (error 'no))
   'set-inverse-nil (condition-case nil (progn (set-face-attribute 'my-inverse-vid-face nil :inverse-video nil) (face-attribute 'my-inverse-vid-face :inverse-video nil 'default-on)) (error 'no))
   'set-inverse-unspec (condition-case nil (progn (set-face-attribute 'my-inverse-vid-face nil :inverse-video 'unspecified) (face-attribute 'my-inverse-vid-face :inverse-video nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_neverend_face_overlay_make_nil_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face nil)
      (list
       'nil-face (overlay-get ov 'face)
       'char-prop-with-nil-overlay (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neverend_face_text_property_interval_object_stats() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZWWWWWVVVVV")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (put-text-property 16 21 'face '(:foreground "red"))
    (put-text-property 21 26 'face '(:background "yellow"))
    (let ((intervals (object-intervals (current-buffer))))
      (list
       'count (length intervals)
       'starts (mapcar #'overlay-start intervals)
       'ends (mapcar #'overlay-end intervals)
       'faces (mapcar (lambda (ov) (get-text-property (overlay-start ov) 'face)) intervals))))))"##,
    );
}

#[test]
fn ft_depth_face_overlay_category_nil_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'category 'my-cat)
      (overlay-put ov 'face nil)
      (list
       'cat (overlay-get ov 'category)
       'face (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_depth_font_lock_keywords_then_unfontify_then_refontify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun kw-unfont-refont (x) x)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 28))))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (list v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 28)) (equal v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 28))))))))"##,
    );
}

#[test]
fn ft_depth_face_set_attribute_all_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-family (face-attribute 'default :family nil 'default-on)
   'default-weight (face-attribute 'default :weight nil 'default-on)
   'default-slant (face-attribute 'default :slant nil 'default-on)
   'default-width (face-attribute 'default :width nil 'default-on)
   'default-height (face-attribute 'default :height nil 'default-on)
   'default-underline (condition-case nil (face-attribute 'default :underline nil 'default-on) (error 'no))
   'default-overline (condition-case nil (face-attribute 'default :overline nil 'default-on) (error 'no))
   'default-strike (condition-case nil (face-attribute 'default :strike-through nil 'default-on) (error 'no))
   'default-box (condition-case nil (face-attribute 'default :box nil 'default-on) (error 'no))
   'default-inverse (condition-case nil (face-attribute 'default :inverse-video nil 'default-on) (error 'no))
   'default-fg (condition-case nil (face-foreground 'default nil 'default-on) (error 'no))
   'default-bg (condition-case nil (face-background 'default nil 'default-on) (error 'no))
   'default-font (condition-case nil (face-font 'default nil) (error 'no))
   'default-inherit (condition-case nil (face-attribute 'default :inherit nil 'default-on) (error 'no))
   'default-extend (condition-case nil (face-attribute 'default :extend nil 'default-on) (error 'no))
   'default-raise (condition-case nil (face-attribute 'default :raise nil 'default-on) (error 'no)))))"##,
    );
}

#[test]
fn ft_depth_face_text_property_at_boundary_stress_test() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AB")
    (put-text-property 1 2 'face 'bold)
    (put-text-property 2 3 'face 'italic)
    (list
     'char-1-face (get-text-property 1 'face)
     'char-1-boundary (next-single-property-change 1 'face nil 3)
     'char-2-face (get-text-property 2 'face)
     'char-2-boundary (previous-single-property-change 2 'face nil 1)
     'interval-count (length (object-intervals (current-buffer)))
     'expand-right (progn (goto-char 3) (insert "CDEF") (list 'face-at-1 (get-text-property 1 'face) 'face-at-2 (get-text-property 2 'face) 'face-at-3 (get-text-property 3 'face) 'face-at-5 (get-text-property 5 'face))))))"##,
    );
}

#[test]
fn ft_depth_font_lock_fontify_unfontify_region_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun region-loop-test (x) (+ x 1))\n")
    (let ((results nil))
      (dotimes (i 3)
        (font-lock-fontify-buffer)
        (push (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30)) results)
        (font-lock-unfontify-buffer)
        (push (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 10 20 30)) results))
      (nreverse results)))))"##,
    );
}

#[test]
fn ft_depth_face_overlay_with_multiple_properties_cleared() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (overlay-put ov 'help-echo "help")
      (list
       'before (length (overlay-properties ov))
       'clear-face (progn (overlay-put ov 'face nil) (length (overlay-properties ov)))
       'clear-prio (progn (overlay-put ov 'priority nil) (length (overlay-properties ov)))
       'clear-help (progn (overlay-put ov 'help-echo nil) (length (overlay-properties ov)))
       'all-cleared (overlay-properties ov)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_depth_face_face_attribute_return_value_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'weight-type (type-of (face-attribute 'default :weight nil 'default-on))
   'slant-type (type-of (face-attribute 'default :slant nil 'default-on))
   'width-type (type-of (face-attribute 'default :width nil 'default-on))
   'height-type (type-of (face-attribute 'default :height nil 'default-on))
   'family-type (type-of (face-attribute 'default :family nil 'default-on))
   'foundry-type (type-of (face-attribute 'default :foundry nil 'default-on))
   'underline-type (condition-case nil (type-of (face-attribute 'default :underline nil 'default-on)) (error 'no-type))
   'box-type (condition-case nil (type-of (face-attribute 'default :box nil 'default-on)) (error 'no-type))
   'font-type (condition-case nil (type-of (face-font 'default nil)) (error 'no-type))
   'fg-type (condition-case nil (type-of (face-foreground 'default nil 'default-on)) (error 'no-type))
   'bg-type (condition-case nil (type-of (face-background 'default nil 'default-on)) (error 'no-type)))))"##,
    );
}

#[test]
fn ft_depth_face_overlay_face_remove_then_readd_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 1 26)))
      (list
       'set-red (progn (overlay-put ov 'face '(:background "red")) (list (overlay-get ov 'face) (get-char-property 5 'face)))
       'remove (progn (overlay-put ov 'face nil) (list (overlay-get ov 'face) (get-char-property 5 'face)))
       'set-green (progn (overlay-put ov 'face '(:background "green")) (list (overlay-get ov 'face) (get-char-property 5 'face)))
       'remove-2 (progn (overlay-put ov 'face nil) (list (overlay-get ov 'face) (get-char-property 5 'face)))
       'set-blue (progn (overlay-put ov 'face '(:background "blue")) (list (overlay-get ov 'face) (get-char-property 5 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_relentless_face_overlay_category_empty_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'category 'test-category)
      (overlay-put ov 'face nil)
      (list
       'category (overlay-get ov 'category)
       'face (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_relentless_font_lock_fontify_region_then_fontify_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun region-then-buffer-test (x) (+ x 1))\n")
    (font-lock-fontify-region 1 15)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 30))))
      (font-lock-fontify-buffer)
      (list v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'fontified)) '(1 7 15 22 30)))))))"##,
    );
}

#[test]
fn ft_relentless_face_overlay_string_props_deep_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFF")
    (let ((ov (make-overlay 10 25)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string (propertize "[B]" 'face '(:foreground "red" :weight bold)))
      (list
       'ov-face (overlay-get ov 'face)
       'before-str (overlay-get ov 'before-string)
       'before-face (get-text-property 0 (overlay-get ov 'before-string))
       'before-face-attrs (length (get-text-property 0 (overlay-get ov 'before-string)))
       'char-prop (get-char-property 15 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_relentless_face_text_property_empty_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (list
     'empty-buffer-face (get-text-property 1 'face)
     'empty-buffer-fontified (get-text-property 1 'fontified)
     'empty-buffer-props (text-properties-at 1)
     'fill (progn (insert "FILLED-TEXT") (put-text-property 1 12 'face 'bold) (list 'face (get-text-property 1 'face) 'props (text-properties-at 1)))
     'clear (progn (set-text-properties 1 12 nil) (list 'face (get-text-property 1 'face) 'props (text-properties-at 1)))))))"##,
    );
}

#[test]
fn ft_relentless_font_lock_unfontify_and_fontify_again_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun un-and-refont (a b) (+ a b))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 28 34))))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (list (equal v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 28 34))))))))"##,
    );
}

#[test]
fn ft_relentless_face_set_face_box_attr_style_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-box-rt-face) (error nil))
  (list
   'set-t (condition-case nil (progn (set-face-attribute 'my-box-rt-face nil :box t) (face-attribute 'my-box-rt-face :box nil 'default-on)) (error 'no))
   'set-style (condition-case nil (progn (set-face-attribute 'my-box-rt-face nil :box '(:line-width 2 :color "red" :style pressed-button)) (face-attribute 'my-box-rt-face :box nil 'default-on)) (error 'no))
   'set-flat (condition-case nil (progn (set-face-attribute 'my-box-rt-face nil :box '(:style flat-button :line-width 1)) (face-attribute 'my-box-rt-face :box nil 'default-on)) (error 'no))
   'set-off (condition-case nil (progn (set-face-attribute 'my-box-rt-face nil :box nil) (face-attribute 'my-box-rt-face :box nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_relentless_face_overlay_face_at_region_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'at-5 (get-char-property 5 'face)
       'at-6 (get-char-property 6 'face)
       'at-15 (get-char-property 15 'face)
       'at-16 (get-char-property 16 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_relentless_face_point_min_max_face_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "X")
    (put-text-property 1 2 'face 'bold)
    (list
     'at-1 (get-text-property 1 'face)
     'at-2 (get-text-property 2 'face)
     'at-0 (text-properties-at 0)
     'after-insert (progn (goto-char 2) (insert "YYYY") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 2 3 5)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_void2_face_overlay_empty_with_no_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (list
       'no-props (overlay-properties ov)
       'no-face (overlay-get ov 'face)
       'no-priority (overlay-get ov 'priority)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_void2_font_lock_fontify_then_unfontify_then_fontify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun fu-test (x) (* x x))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (list 'f1 pos (get-text-property pos 'face))) '(1 7 15 22 28))))
      (font-lock-unfontify-buffer)
      (let ((v1 (mapcar (lambda (pos) (goto-char pos) (list 'u pos (get-text-property pos 'fontified))) '(1 7 15 22 28))))
        (font-lock-fontify-buffer)
        (list v0 v1 (mapcar (lambda (pos) (goto-char pos) (list 'f2 pos (get-text-property pos 'face))) '(1 7 15 22 28))))))))"##,
    );
}

#[test]
fn ft_void2_face_overlay_before_string_nil_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'before-string ">")
      (list
       'ov-face (overlay-get ov 'face)
       'before-str (overlay-get ov 'before-string)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_void2_face_set_face_attribute_unspec_vs_nil_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-unspec-nil-face) (error nil))
  (list
   'set-weight-bold (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :weight 'bold) (face-attribute 'my-unspec-nil-face :weight nil 'default-on)) (error 'no))
   'set-weight-nil (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :weight nil) (face-attribute 'my-unspec-nil-face :weight nil 'default-on)) (error 'no))
   'set-weight-unspec (condition-case nil (progn (set-face-attribute 'my-unspec-nil-face nil :weight 'unspecified) (face-attribute 'my-unspec-nil-face :weight nil 'default-on)) (error 'no))
   'set-fg-red (condition-case nil (progn (set-face-foreground 'my-unspec-nil-face "red" nil) (face-foreground 'my-unspec-nil-face nil 'default-on)) (error 'no))
   'set-fg-nil (condition-case nil (progn (set-face-foreground 'my-unspec-nil-face nil nil) (face-foreground 'my-unspec-nil-face nil 'default-on)) (error 'no))
   'set-fg-unspec (condition-case nil (progn (set-face-foreground 'my-unspec-nil-face 'unspecified nil) (face-foreground 'my-unspec-nil-face nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_void2_font_lock_fontify_empty_buffer_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (font-lock-fontify-buffer)
    (let ((v0 (list 'empty-face (get-text-property 1 'face) 'empty-fontified (get-text-property 1 'fontified))))
      (insert "(defun empty-test () 42)\n")
      (font-lock-fontify-buffer)
      (list v0 (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face) (get-text-property pos 'fontified))) '(1 7 15 22 27)))))))"##,
    );
}

#[test]
fn ft_void2_face_text_property_interval_count_after_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXXXXXXXX")
    (put-text-property 1 13 'face 'bold)
    (let ((counts nil))
      (push (list 'one-interval (length (object-intervals (current-buffer)))) counts)
      (goto-char 5) (insert "Y") (push (list 'after-split (length (object-intervals (current-buffer)))) counts)
      (goto-char 8) (insert "Z") (push (list 'after-second-split (length (object-intervals (current-buffer)))) counts)
      (nreverse counts))))"##,
    );
}

#[test]
fn ft_void2_face_overlay_priority_one_field_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'priority 42)
      (list
       'priority-only (overlay-get ov 'priority)
       'no-face (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_void2_face_color_rgb_hex_name_roundtrip_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'color)
  (list
   'red-named (color-name-to-rgb "red")
   'red-hex (color-name-to-rgb "#FF0000")
   'equal-red (equal (color-name-to-rgb "red") (color-name-to-rgb "#FF0000"))
   'green-named (color-name-to-rgb "green")
   'green-hex (color-name-to-rgb "#00FF00")
   'equal-green (equal (color-name-to-rgb "green") (color-name-to-rgb "#00FF00"))
   'blue-named (color-name-to-rgb "blue")
   'blue-hex (color-name-to-rgb "#0000FF")
   'equal-blue (equal (color-name-to-rgb "blue") (color-name-to-rgb "#0000FF")))))"##,
    );
}

#[test]
fn ft_spacetime_face_overlay_category_with_face_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 1 26)))
      (overlay-put ov 'category 'my-special-cat)
      (overlay-put ov 'face '(:background "yellow" :inherit bold))
      (overlay-put ov 'priority 50)
      (list
       'category (overlay-get ov 'category)
       'face (overlay-get ov 'face)
       'priority (overlay-get ov 'priority)
       'char-prop (get-char-property 10 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_spacetime_font_lock_fontify_after_kill_local_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun after-kill-locals-test () 42)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (get-text-property 7 'face)))
      (kill-local-variable 'font-lock-keywords)
      (font-lock-fontify-buffer)
      (list v0 (get-text-property 7 'face))))))"##,
    );
}

#[test]
fn ft_spacetime_face_set_face_slant_all_values_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-slant-all-face) (error nil))
  (list
   'set-normal (condition-case nil (progn (set-face-attribute 'my-slant-all-face nil :slant 'normal) (face-attribute 'my-slant-all-face :slant nil 'default-on)) (error 'no))
   'set-italic (condition-case nil (progn (set-face-attribute 'my-slant-all-face nil :slant 'italic) (face-attribute 'my-slant-all-face :slant nil 'default-on)) (error 'no))
   'set-oblique (condition-case nil (progn (set-face-attribute 'my-slant-all-face nil :slant 'oblique) (face-attribute 'my-slant-all-face :slant nil 'default-on)) (error 'no))
   'set-unspec (condition-case nil (progn (set-face-attribute 'my-slant-all-face nil :slant 'unspecified) (face-attribute 'my-slant-all-face :slant nil 'default-on)) (error 'no))
   'default-slant (face-attribute 'default :slant nil 'default-on))))"##,
    );
}

#[test]
fn ft_spacetime_face_overlay_presence_after_buffer_erase_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before-erase (list 'start (overlay-start ov) 'end (overlay-end ov) 'alive (and (overlay-buffer ov) t))
       'after-erase (progn (erase-buffer) (list 'start (overlay-start ov) 'end (overlay-end ov) 'alive (and (overlay-buffer ov) t)))
       'after-refill (progn (insert "REFILLED-CONTENT-TEXT") (list 'start (overlay-start ov) 'end (overlay-end ov) 'face-at-1 (get-char-property 1 'face) 'face-at-10 (get-char-property 10 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_spacetime_face_text_property_insert_and_split_interval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBB")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (list
     'initial (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 6 8 10))
     'split-at-3 (progn (goto-char 3) (insert "SPLIT") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 8 10 12 15)))
     'split-at-6 (progn (goto-char 6) (insert "HERE") (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 3 5 8 10 12 14 16 19)))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_spacetime_font_lock_fontify_multiple_modes_comparison() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (list
   'emacs-lisp-mode-fontified (with-temp-buffer (emacs-lisp-mode) (insert "(defun f (x) x)") (font-lock-fontify-buffer) (get-text-property 1 'fontified))
   'text-mode-fontified (with-temp-buffer (text-mode) (insert "text mode test") (font-lock-fontify-buffer) (get-text-property 1 'fontified))
   'fundamental-mode-fontified (with-temp-buffer (fundamental-mode) (font-lock-mode 1) (insert "fundamental test") (font-lock-fontify-buffer) (get-text-property 1 'fontified)))))"##,
    );
}

#[test]
fn ft_spacetime_face_overlay_face_with_negative_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'prio-negative (progn (overlay-put ov 'priority -10) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       'prio-zero (progn (overlay-put ov 'priority 0) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       'prio-positive (progn (overlay-put ov 'priority 10) (list 'prio (overlay-get ov 'priority) 'face (get-char-property 5 'face)))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_spacetime_face_text_property_interval_ends_precise() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZ")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (list
     'at-borders (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 10 11 15))
     'next-changes (mapcar (lambda (pos) (next-single-property-change pos 'face nil 16)) '(1 6 11))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_infinite_face_overlay_properties_roundtrip_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'help-echo "help text")
      (overlay-put ov 'priority 42)
      (let ((p (overlay-properties ov)))
        (list
         'face-get (plist-get p 'face)
         'help-get (plist-get p 'help-echo)
         'priority-get (plist-get p 'priority)
         'none-get (plist-get p 'nonexistent)
         'props-len (length p)
         (progn (delete-overlay ov) 'cleaned)))))))"##,
    );
}

#[test]
fn ft_infinite_font_lock_fontify_unfontify_empty_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (list
     'empty-before (list 'face (get-text-property 1 'face) 'fontified (get-text-property 1 'fontified))
     'fontify-empty (progn (font-lock-fontify-buffer) (list 'face (get-text-property 1 'face) 'fontified (get-text-property 1 'fontified)))
     'unfontify-empty (progn (font-lock-unfontify-buffer) (list 'fontified (get-text-property 1 'fontified)))
     'insert-then-fontify (progn (insert "(defun ef (x) x)") (font-lock-fontify-buffer) (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 11 14 15 17)))))))"##,
    );
}

#[test]
fn ft_infinite_face_overlay_same_region_diff_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov1 (make-overlay 1 21))) (overlay-put ov1 'face '(:background "red")))
    (let ((ov2 (make-overlay 1 21))) (overlay-put ov2 'face '(:foreground "green")))
    (let ((ov3 (make-overlay 1 21))) (overlay-put ov3 'face '(:underline t)))
    (list
     'how-many (length (overlays-at 5))
     'effective-face (get-char-property 5 'face)
     'all-faces (mapcar (lambda (ov) (overlay-get ov 'face)) (overlays-at 5))
     (progn (mapc #'delete-overlay (overlays-in 1 21)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_infinite_face_text_property_interval_after_delete_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZ")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face 'italic)
    (put-text-property 11 16 'face 'underline)
    (list
     'init-intervals (length (object-intervals (current-buffer)))
     'delete-middle (progn (delete-region 6 11) (list 'intervals (length (object-intervals (current-buffer))) 'faces (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 7 10 11)))))
     'delete-all (progn (delete-region 1 (point-max)) (list 'intervals (length (object-intervals (current-buffer))))))))"##,
    );
}

#[test]
fn ft_infinite_font_lock_fontify_region_only_first_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun first-line () 1)\n(defun second-line () 2)\n")
    (font-lock-fontify-region 1 25)
    (list
     'fontified-first (get-text-property 1 'fontified)
     'fontified-second (get-text-property 27 'fontified)
     'face-first-defun (save-excursion (goto-char (point-min)) (search-forward "defun") (get-text-property (match-beginning 0) 'face))
     'face-second-defun (save-excursion (goto-char (point-min)) (search-forward "second-line") (get-text-property (match-beginning 0) 'face))
     'fontify-rest (progn (font-lock-fontify-region 25 (point-max)) (get-text-property 27 'fontified))))))"##,
    );
}

#[test]
fn ft_infinite_face_set_face_attribute_width_table_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'font-width-table-bound (boundp 'font-width-table)
   'font-width-table-value (if (boundp 'font-width-table) (length font-width-table) 'no-bound)
   'ultra-condensed-available (if (boundp 'font-width-table) (member 'ultra-condensed font-width-table) 'no-table)
   'condensed-available (if (boundp 'font-width-table) (member 'condensed font-width-table) 'no-table)
   'normal-available (if (boundp 'font-width-table) (member 'normal font-width-table) 'no-table)
   'expanded-available (if (boundp 'font-width-table) (member 'expanded font-width-table) 'no-table)
   'default-width (face-attribute 'default :width nil 'default-on))))"##,
    );
}

#[test]
fn ft_infinite_face_overlay_face_after_property_overwrite_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "red"))
      (list
       'first-face (overlay-get ov 'face)
       'overwrite-face (progn (overlay-put ov 'face '(:background "green")) (overlay-get ov 'face))
       'overwrite-again (progn (overlay-put ov 'face '(:background "blue" :foreground "white")) (overlay-get ov 'face))
       'char-prop-after-all (get-char-property 10 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_infinite_face_text_property_null_values_handling() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (put-text-property 1 6 'face 'bold)
    (put-text-property 6 11 'face nil)
    (put-text-property 11 16 'face nil)
    (put-text-property 16 21 'face '(:foreground "red"))
    (list
     'faces-with-nil (mapcar (lambda (pos) (goto-char pos) (list pos (get-text-property pos 'face))) '(1 5 6 8 11 13 16 18 20))
     'find-bold (text-property-any 1 21 'face 'bold)
     'find-nil (text-property-any 1 21 'face nil)
     'find-red (text-property-any 1 21 'face '(:foreground "red")))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_eternity_face_overlay_face_and_priority_together() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'face (overlay-get ov 'face)
       'priority (overlay-get ov 'priority)
       'char-prop (get-char-property 5 'face)
       'char-prop-outside nil
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_eternity_font_lock_fontify_complete_then_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun complete-partial (x) (+ x 1))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (get-text-property 1 'fontified)))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-region 1 15)
      (list v0 (get-text-property 1 'fontified) (get-text-property 20 'fontified))))))"##,
    );
}

#[test]
fn ft_eternity_face_overlay_make_destroy_make_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((snap (lambda () (get-char-property 10 'face))))
      (let ((v0 (funcall snap)))
        (let ((ov (make-overlay 6 20))) (overlay-put ov 'face '(:background "yellow")) (let ((v1 (funcall snap)))
        (delete-overlay ov) (let ((v2 (funcall snap)))
        (let ((ov2 (make-overlay 6 20))) (overlay-put ov2 'face '(:foreground "red" :weight bold)) (let ((v3 (funcall snap)))
        (delete-overlay ov2) (let ((v4 (funcall snap)))
        (list v0 v1 v2 v3 v4))))))))))))"##,
    );
}

#[test]
fn ft_eternity_face_text_property_prop_change_interval_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAABBBCCCDDDEEEFFFGGGHHHIIIJJJ")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 7 'face 'italic)
    (put-text-property 7 10 'face 'underline)
    (put-text-property 10 13 'face '(:foreground "red"))
    (put-text-property 13 16 'face '(:background "yellow"))
    (put-text-property 16 19 'face '(:foreground "blue"))
    (put-text-property 19 22 'face '(:background "cyan"))
    (put-text-property 22 25 'face '(:slant italic))
    (put-text-property 25 28 'face '(:weight bold))
    (put-text-property 28 31 'face '(:underline t))
    (list
     'all-prop-changes (let ((pos 1) (result nil))
                         (while pos
                           (setq pos (next-single-property-change pos 'face nil 31))
                           (when pos (push (list pos (get-text-property pos 'face)) result)))
                         (nreverse result))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_eternity_font_lock_fontify_then_fontify_again_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun fontify-twice (x) (+ x x))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 28 34))))
      (font-lock-fontify-buffer)
      (list v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 28 34)) (equal v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 20 28 34))))))))"##,
    );
}

#[test]
fn ft_eternity_face_set_attribute_all_basic_attrs_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-all-basic-face) (error nil))
  (condition-case nil (set-face-attribute 'my-all-basic-face nil :weight 'bold :slant 'italic :underline t :overline t :strike-through t :box t :inverse-video t) (error nil))
  (list
   'weight (face-attribute 'my-all-basic-face :weight nil 'default-on)
   'slant (face-attribute 'my-all-basic-face :slant nil 'default-on)
   'underline (condition-case nil (face-attribute 'my-all-basic-face :underline nil 'default-on) (error 'no))
   'overline (condition-case nil (face-attribute 'my-all-basic-face :overline nil 'default-on) (error 'no))
   'strike (condition-case nil (face-attribute 'my-all-basic-face :strike-through nil 'default-on) (error 'no))
   'box (condition-case nil (face-attribute 'my-all-basic-face :box nil 'default-on) (error 'no))
   'inverse (condition-case nil (face-attribute 'my-all-basic-face :inverse-video nil 'default-on) (error 'no))
   (condition-case nil (progn (set-face-attribute 'my-all-basic-face nil :weight 'unspecified :slant 'unspecified :underline 'unspecified :overline 'unspecified :strike-through 'unspecified :box 'unspecified :inverse-video 'unspecified) 'reset) (error 'no)))))"##,
    );
}

#[test]
fn ft_eternity_face_overlay_priority_get_set_verify() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'priority 42)
      (list
       'get-prio (overlay-get ov 'priority)
       'set-prio-10 (progn (overlay-put ov 'priority 10) (overlay-get ov 'priority))
       'set-prio-nil (progn (overlay-put ov 'priority nil) (overlay-get ov 'priority))
       'set-prio-100 (progn (overlay-put ov 'priority 100) (overlay-get ov 'priority))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_eternity_face_text_properties_all_at_interval_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "XXXXXYYYYYZZZZZ")
    (put-text-property 1 6 'face 'bold :x-prop 'x-val)
    (put-text-property 6 11 'face 'italic :y-prop 'y-val)
    (put-text-property 11 16 'face 'underline :z-prop 'z-val)
    (list
     'at-1 (text-properties-at 1)
     'at-5 (text-properties-at 5)
     'at-6 (text-properties-at 6)
     'at-10 (text-properties-at 10)
     'at-11 (text-properties-at 11)
     'at-15 (text-properties-at 15)
     'props-count-1 (length (text-properties-at 1))
     'props-count-6 (length (text-properties-at 6))
     'props-count-11 (length (text-properties-at 11)))))"##,
    );
}

#[test]
fn ft_neverstop_face_overlay_face_and_nil_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority nil)
      (list
       'nil-prio (overlay-get ov 'priority)
       'face (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neverstop_font_lock_unfontify_buffer_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun insert-test () 42)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (get-text-property 1 'fontified)))
      (goto-char (point-max))
      (insert "\n(defun inserted-test () 99)\n")
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (list v0 (get-text-property 1 'fontified) (get-text-property 30 'fontified))))))"##,
    );
}

#[test]
fn ft_neverstop_face_overlay_with_category_and_face_both() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'category 'my-cat)
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'cat (overlay-get ov 'category)
       'face (overlay-get ov 'face)
       'char-prop (get-char-property 5 'face)
       'char-prop-cat (get-char-property 5 'category)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neverstop_face_set_face_attribute_width_cycle_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-w-cycle-face) (error nil))
  (list
   'set-condensed (condition-case nil (progn (set-face-attribute 'my-w-cycle-face nil :width 'condensed) (face-attribute 'my-w-cycle-face :width nil 'default-on)) (error 'no))
   'set-normal (condition-case nil (progn (set-face-attribute 'my-w-cycle-face nil :width 'normal) (face-attribute 'my-w-cycle-face :width nil 'default-on)) (error 'no))
   'set-expanded (condition-case nil (progn (set-face-attribute 'my-w-cycle-face nil :width 'expanded) (face-attribute 'my-w-cycle-face :width nil 'default-on)) (error 'no))
   'set-ultra-condensed (condition-case nil (progn (set-face-attribute 'my-w-cycle-face nil :width 'ultra-condensed) (face-attribute 'my-w-cycle-face :width nil 'default-on)) (error 'no))
   'set-unspec (condition-case nil (progn (set-face-attribute 'my-w-cycle-face nil :width 'unspecified) (face-attribute 'my-w-cycle-face :width nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_neverstop_face_text_properties_put_then_get_then_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Put get remove face property test buffer content data")
    (list
     'put (progn (put-text-property 1 15 'face 'bold) (get-text-property 1 'face))
     'get (get-text-property 1 'face)
     'get-again (get-text-property 1 'face)
     'remove (progn (remove-text-properties 1 15 '(face nil)) (get-text-property 1 'face))
     'put-new (progn (put-text-property 1 15 'face 'italic) (get-text-property 1 'face))
     'remove-new (progn (remove-text-properties 1 15 '(face nil)) (get-text-property 1 'face)))))"##,
    );
}

#[test]
fn ft_neverstop_font_lock_fontify_buffer_with_keywords_append() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "APPEND-KW keywords test font lock face buffer content text end now")
    (font-lock-add-keywords nil '(("\\<\\(APPEND-KW\\)\\>" 1 '(:foreground "red" :weight bold) append)))
    (font-lock-fontify-buffer)
    (list
     'append-face (save-excursion (goto-char (point-min)) (search-forward "APPEND-KW") (get-text-property (match-beginning 0) 'face))
     'other-face (save-excursion (goto-char (point-min)) (search-forward "keywords") (get-text-property (match-beginning 0) 'face))))))"##,
    );
}

#[test]
fn ft_neverstop_face_overlay_face_clear_then_reapply() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'before (get-char-property 5 'face)
       'clear (progn (overlay-put ov 'face nil) (get-char-property 5 'face))
       'reapply (progn (overlay-put ov 'face '(:foreground "red" :weight bold)) (get-char-property 5 'face))
       'clear-again (progn (overlay-put ov 'face nil) (get-char-property 5 'face))
       'reapply-again (progn (overlay-put ov 'face '(:background "blue")) (get-char-property 5 'face))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_neverstop_face_color_value_get_frame_specific() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'fg-nil-frame (condition-case nil (face-foreground 'default nil 'default-on) (error 'no))
   'fg-frame (condition-case nil (face-foreground 'default (selected-frame) 'default-on) (error 'no))
   'bg-nil-frame (condition-case nil (face-background 'default nil 'default-on) (error 'no))
   'bg-frame (condition-case nil (face-background 'default (selected-frame) 'default-on) (error 'no))
   'fg-no-frame (condition-case nil (face-foreground 'default) (error 'no))
   'font-nil-frame (condition-case nil (face-font 'default nil) (error 'no))
   'font-frame (condition-case nil (face-font 'default (selected-frame)) (error 'no))
   'font-no-frame (condition-case nil (face-font 'default) (error 'no)))))"##,
    );
}

#[test]
fn ft_nofinal_face_overlay_face_property_access_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov (make-overlay 1 21)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'face (overlay-get ov 'face)
       'priority (overlay-get ov 'priority)
       'non-existent (overlay-get ov 'non-existent-key)
       'all-keys (let ((p (overlay-properties ov)) (ks nil) (i 0))
                   (while (< i (length p)) (push (nth i p) ks) (setq i (+ i 2)))
                   (nreverse ks))
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_nofinal_font_lock_unfontify_then_fontify_region_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun un-fon-region (x) x)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (get-text-property 7 'face)))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-region 1 (point-max))
      (list v0 (get-text-property 7 'face) (get-text-property 1 'fontified))))))"##,
    );
}

#[test]
fn ft_nofinal_face_text_property_put_on_same_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (put-text-property 1 16 'face 'bold)
    (put-text-property 1 16 'face 'italic)
    (put-text-property 1 16 'face 'underline)
    (put-text-property 1 16 'face '(:foreground "red"))
    (list
     'final-face (get-text-property 1 'face)
     'final-face-at-5 (get-text-property 5 'face)
     'final-face-at-15 (get-text-property 15 'face)
     'all-equal (equal (get-text-property 1 'face) (get-text-property 8 'face) (get-text-property 15 'face))))))"##,
    );
}

#[test]
fn ft_nofinal_face_overlay_make_then_move_then_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGG")
    (let ((ov (make-overlay 6 15)))
      (overlay-put ov 'face '(:background "yellow"))
      (list
       'init (get-char-property 10 'face)
       'move-right (progn (move-overlay ov 20 30) (get-char-property 25 'face))
       'move-left (progn (move-overlay ov 1 10) (get-char-property 5 'face))
       'delete (progn (delete-overlay ov) (get-char-property 5 'face)))))))"##,
    );
}

#[test]
fn ft_nofinal_font_lock_add_remove_keywords_verify_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (fundamental-mode)
    (font-lock-mode 1)
    (insert "VERIFY-KW keyword test VERIFY-KW font lock face buffer end")
    (font-lock-add-keywords nil '(("\\<\\(VERIFY-KW\\)\\>" 1 '(:foreground "red" :weight bold) t)))
    (font-lock-fontify-buffer)
    (let ((v0 (save-excursion (goto-char (point-min)) (search-forward "VERIFY-KW") (get-text-property (match-beginning 0) 'face))))
      (font-lock-remove-keywords nil '(("\\<\\(VERIFY-KW\\)\\>" 1 '(:foreground "red" :weight bold) t)))
      (font-lock-fontify-buffer)
      (list v0 (save-excursion (goto-char (point-min)) (search-forward "VERIFY-KW") (get-text-property (match-beginning 0) 'face)))))))"##,
    );
}

#[test]
fn ft_nofinal_face_set_face_font_by_name_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-font-name-face) (error nil))
  (list
   'default-font (condition-case nil (face-font 'default nil) (error 'no))
   'set-monospace-12 (condition-case nil (progn (set-face-font 'my-font-name-face "Monospace-12" nil) (face-font 'my-font-name-face nil)) (error 'no))
   'reset-unspec (condition-case nil (progn (set-face-font 'my-font-name-face 'unspecified nil) (face-font 'my-font-name-face nil)) (error 'no))
   'set-monospace-14 (condition-case nil (progn (set-face-font 'my-font-name-face "Monospace-14" nil) (face-font 'my-font-name-face nil)) (error 'no))
   'reset-again (condition-case nil (progn (set-face-font 'my-font-name-face 'unspecified nil) (face-font 'my-font-name-face nil)) (error 'no)))))"##,
    );
}

#[test]
fn ft_nofinal_face_text_property_get_at_specific_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAAAAAAABBBBBBBBBBCCCCCCCCCC")
    (put-text-property 1 11 'face 'bold)
    (put-text-property 11 21 'face 'italic)
    (put-text-property 21 31 'face 'underline)
    (list
     'at-1 (get-text-property 1 'face)
     'at-5 (get-text-property 5 'face)
     'at-10 (get-text-property 10 'face)
     'at-11 (get-text-property 11 'face)
     'at-15 (get-text-property 15 'face)
     'at-20 (get-text-property 20 'face)
     'at-21 (get-text-property 21 'face)
     'at-25 (get-text-property 25 'face)
     'at-30 (get-text-property 30 'face))))))"##,
    );
}

#[test]
fn ft_nofinal_face_overlay_all_face_attrs_get_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (list
   'default-weight (face-attribute 'default :weight nil 'default-on)
   'bold-weight (face-attribute 'bold :weight nil 'default-on)
   'italic-slant (face-attribute 'italic :slant nil 'default-on)
   'bold-italic-weight (face-attribute 'bold-italic :weight nil 'default-on)
   'bold-italic-slant (face-attribute 'bold-italic :slant nil 'default-on)
   'underline-underline (condition-case nil (face-attribute 'underline :underline nil 'default-on) (error 'no))
   'fringe-facep (facep 'fringe)
   'region-facep (facep 'region)
   'highlight-facep (facep 'highlight)
   'secondary-selection-facep (condition-case nil (facep 'secondary-selection) (error 'no))
   'mode-line-facep (facep 'mode-line)
   'mode-line-inactive-facep (condition-case nil (facep 'mode-line-inactive) (error 'no))
   'cursor-facep (facep 'cursor)
   'scroll-bar-facep (facep 'scroll-bar)
   'tool-bar-facep (facep 'tool-bar)
   'menu-facep (facep 'menu))))"##,
    );
}

#[test]
fn ft_ever_face_overlay_empty_region_no_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCC")
    (let ((ov (make-overlay 1 16)))
      (list
       'no-props (overlay-properties ov)
       'no-face (overlay-get ov 'face)
       'no-prio (overlay-get ov 'priority)
       (progn (delete-overlay ov) 'cleaned))))))"##,
    );
}

#[test]
fn ft_ever_font_lock_fontify_after_buffer_recreation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun recreate-test (x) x)\n")
    (font-lock-fontify-buffer)
    (let ((v0 (get-text-property 7 'face)))
      (erase-buffer)
      (insert "(defun new-test (y) (+ y 1))\n")
      (font-lock-fontify-buffer)
      (list v0 (get-text-property 7 'face) (get-text-property 16 'face))))))"##,
    );
}

#[test]
fn ft_ever_face_text_property_face_plist_decompose() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "Face plist decompose test buffer content text data here now")
    (put-text-property 1 54 'face '(:foreground "blue" :weight bold :slant italic :underline t :overline nil :strike-through t :box (:line-width 2) :inherit bold :extend t :height 1.2))
    (let ((plist (get-text-property 1 'face)))
      (list
       'plist-len (length plist)
       'keys (let ((ks nil) (i 0)) (while (< i (length plist)) (push (nth i plist) ks) (setq i (+ i 2))) (nreverse ks))
       'fg (plist-get plist :foreground)
       'weight (plist-get plist :weight)
       'slant (plist-get plist :slant)
       'underline (plist-get plist :underline)
       'overline (plist-get plist :overline)
       'strike (plist-get plist :strike-through)
       'box (plist-get plist :box)
       'inherit (plist-get plist :inherit)
       'extend (plist-get plist :extend)
       'height (plist-get plist :height)))))"##,
    );
}

#[test]
fn ft_ever_face_overlay_reorder_by_priority_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDD")
    (let ((ov1 (make-overlay 1 21))) (overlay-put ov1 'face '(:background "red")) (overlay-put ov1 'priority 5))
    (let ((ov2 (make-overlay 1 21))) (overlay-put ov2 'face '(:background "green")) (overlay-put ov2 'priority 15))
    (let ((ov3 (make-overlay 1 21))) (overlay-put ov3 'face '(:background "blue")) (overlay-put ov3 'priority 10))
    (list
     'before-reorder (get-char-property 5 'face)
     'reorder (progn (overlay-put ov1 'priority 20) (overlay-put ov3 'priority 25) (get-char-property 5 'face))
     'effective-after (get-char-property 5 'face)
     (progn (mapc #'delete-overlay (overlays-in 1 21)) 'cleaned)))))"##,
    );
}

#[test]
fn ft_ever_font_lock_fontify_unfontify_verify_face_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun verify-face-test (x) (* x 2))\n")
    (font-lock-fontify-buffer)
    (let ((v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 30 36))))
      (font-lock-unfontify-buffer)
      (font-lock-fontify-buffer)
      (list (equal v0 (mapcar (lambda (pos) (goto-char pos) (get-text-property pos 'face)) '(1 7 15 22 30 36))))))))"##,
    );
}

#[test]
fn ft_ever_face_set_face_strike_through_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (condition-case nil (copy-face 'default 'my-st-thru-face) (error nil))
  (list
   'default-strike (condition-case nil (face-attribute 'default :strike-through nil 'default-on) (error 'no))
   'set-strike-t (condition-case nil (progn (set-face-attribute 'my-st-thru-face nil :strike-through t) (face-attribute 'my-st-thru-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-color (condition-case nil (progn (set-face-attribute 'my-st-thru-face nil :strike-through '(:color "red")) (face-attribute 'my-st-thru-face :strike-through nil 'default-on)) (error 'no))
   'set-strike-color-flat (condition-case nil (progn (set-face-attribute 'my-st-thru-face nil :strike-through '(:color "blue" :style line)) (face-attribute 'my-st-thru-face :strike-through nil 'default-on)) (error 'no))
   'set-off (condition-case nil (progn (set-face-attribute 'my-st-thru-face nil :strike-through nil) (face-attribute 'my-st-thru-face :strike-through nil 'default-on)) (error 'no)))))"##,
    );
}

#[test]
fn ft_ever_face_text_property_char_at_each_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "ABCDEFGHIJ")
    (put-text-property 1 4 'face 'bold)
    (put-text-property 4 7 'face 'italic)
    (put-text-property 7 11 'face 'underline)
    (list
     'all-chars (mapcar (lambda (pos) (goto-char pos) (list pos (char-after pos) (get-text-property pos 'face))) '(1 2 3 4 5 6 7 8 9 10))
     'interval-count (length (object-intervals (current-buffer))))))"##,
    );
}

#[test]
fn ft_ever_face_overlay_property_after_delete_and_recreate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (with-temp-buffer
    (insert "AAAAABBBBBCCCCCDDDDDEEEEE")
    (let ((ov (make-overlay 6 20)))
      (overlay-put ov 'face '(:background "yellow"))
      (overlay-put ov 'priority 50)
      (list
       'before (list 'face (overlay-get ov 'face) 'prio (overlay-get ov 'priority))
       (progn (delete-overlay ov) 'deleted)
       (let ((ov2 (make-overlay 6 20)))
         (overlay-put ov2 'face '(:foreground "red"))
         (list 'new-face (overlay-get ov2 'face) 'new-prio (overlay-get ov2 'priority) 'char-prop (get-char-property 10 'face)
               (progn (delete-overlay ov2) 'final-cleaned))))))))"##,
    );
}
