//! Strict core subsystem combo oracle probes.
//!
//! These tests intentionally combine several GNU Emacs subsystems in each
//! form: buffer lifecycle and local hooks, abnormal hooks, advice ordering,
//! window/frame state, markers, overlays, text properties, and undo.  Most
//! tests are parity locks.  The final `divergence_surface_*` tests are normal
//! oracle parity assertions that are expected to fail until the recorded
//! GNU/Neomacs differences are fixed.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_core_buffer_kill_query_and_kill_hook_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((b (generate-new-buffer " *probe-kill*")))
    (with-current-buffer b
      (setq-local kill-buffer-query-functions
                  (list (lambda () (push 'q1 log) t)
                        (lambda () (push 'q2 log) t)))
      (setq-local kill-buffer-hook
                  (list (lambda () (push (list 'kill (buffer-name)) log)))))
    (list (kill-buffer b) log (buffer-live-p b))))
"##,
    );
}

#[test]
fn div_core_buffer_change_hooks_and_modified_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (add-hook 'before-change-functions
              (lambda (b e) (push (list 'before b e (buffer-modified-p)) log))
              nil t)
    (add-hook 'after-change-functions
              (lambda (b e l) (push (list 'after b e l (buffer-modified-p)) log))
              nil t)
    (insert "abc")
    (goto-char 2)
    (delete-char 1)
    (list (buffer-string) (buffer-modified-p) (nreverse log))))
"##,
    );
}

#[test]
fn div_core_normal_and_abnormal_hook_order_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (defvar probe--normal-hook nil)
  (defvar probe--abnormal-hook nil)
  (setq probe--normal-hook nil
        probe--abnormal-hook nil)
  (add-hook 'probe--normal-hook (lambda () (push 'global log)))
  (add-hook 'probe--abnormal-hook (lambda (x) (push (list 'a x) log) nil))
  (add-hook 'probe--abnormal-hook (lambda (x) (push (list 'b x) log) 'done))
  (list
   (with-temp-buffer
     (add-hook 'probe--normal-hook (lambda () (push 'local log)) nil t)
     (run-hooks 'probe--normal-hook)
     log)
   (run-hook-with-args-until-success 'probe--abnormal-hook 42)
   log))
"##,
    );
}

#[test]
fn div_core_advice_depth_filter_return_and_member_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (defun probe--adv-depth (x) (push (list 'orig x) log) (+ x 10))
  (let ((before (lambda (x) (push (list 'before x) log)))
        (around1 (lambda (fn x)
                   (push 'around1-in log)
                   (prog1 (funcall fn (+ x 1))
                     (push 'around1-out log))))
        (around2 (lambda (fn x)
                   (push 'around2-in log)
                   (prog1 (funcall fn (* x 2))
                     (push 'around2-out log))))
        (filter-ret (lambda (r) (push (list 'filter-ret r) log) (* r 3))))
    (advice-add 'probe--adv-depth :before before)
    (advice-add 'probe--adv-depth :around around1 '((depth . 10)))
    (advice-add 'probe--adv-depth :around around2 '((depth . -10)))
    (advice-add 'probe--adv-depth :filter-return filter-ret)
    (unwind-protect
        (list (probe--adv-depth 4)
              (nreverse log)
              (not (null (advice-member-p around1 'probe--adv-depth))))
      (advice-remove 'probe--adv-depth before)
      (advice-remove 'probe--adv-depth around1)
      (advice-remove 'probe--adv-depth around2)
      (advice-remove 'probe--adv-depth filter-ret)
      (fmakunbound 'probe--adv-depth))))
"##,
    );
}

#[test]
fn div_core_advice_preserves_command_interactive_shape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defun probe--adv-command (x) (interactive "p") x)
  (let ((a (lambda (&rest _) nil)))
    (advice-add 'probe--adv-command :before a)
    (unwind-protect
        (list (commandp 'probe--adv-command)
              (interactive-form 'probe--adv-command)
              (help-function-arglist 'probe--adv-command))
      (advice-remove 'probe--adv-command a)
      (fmakunbound 'probe--adv-command))))
"##,
    );
}

#[test]
fn div_core_window_buffer_configuration_and_parameters_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-win-a*"))
      (b2 (get-buffer-create " *probe-win-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b1)
        (set-window-parameter (selected-window) 'probe 42)
        (set-window-dedicated-p (selected-window) 'side)
        (erase-buffer)
        (insert "abcdef")
        (goto-char 3)
        (let ((dedicated (window-dedicated-p))
              (param (window-parameter nil 'probe))
              (cfg (current-window-configuration))
              (w2 (split-window nil nil 'right)))
          (set-window-buffer w2 b2)
          (select-window w2)
          (erase-buffer)
          (insert "12345")
          (goto-char 4)
          (set-window-configuration cfg)
          (list dedicated param
                (count-windows)
                (buffer-name (window-buffer (selected-window)))
                (point)
                (window-point (selected-window)))))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_display_buffer_window_parameters_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-display*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (with-current-buffer b (erase-buffer) (insert "x"))
        (let* ((display-buffer-alist
                '(("\\*probe-display\\*" display-buffer-below-selected
                   (window-height . 4)
                   (window-parameters . ((probe . yes))))))
               (w (display-buffer b)))
          (list (window-live-p w)
                (count-windows)
                (window-parameter w 'probe)
                (buffer-name (window-buffer w))
                (window-total-height w))))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_core_buffer_swap_text_moves_markers_and_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a (generate-new-buffer " *probe-swap-a*"))
      (b (generate-new-buffer " *probe-swap-b*")))
  (unwind-protect
      (let (ma mb oa ob)
        (with-current-buffer a
          (insert "alpha")
          (setq ma (copy-marker 3))
          (setq oa (make-overlay 2 5))
          (overlay-put oa 'tag 'oa))
        (with-current-buffer b
          (insert "BETA")
          (setq mb (copy-marker 2))
          (setq ob (make-overlay 1 4))
          (overlay-put ob 'tag 'ob))
        (with-current-buffer a (buffer-swap-text b))
        (list (with-current-buffer a (buffer-string))
              (with-current-buffer b (buffer-string))
              (marker-position ma) (eq (marker-buffer ma) a)
              (marker-position mb) (eq (marker-buffer mb) b)
              (with-current-buffer a
                (mapcar (lambda (o) (overlay-get o 'tag))
                        (overlays-in (point-min) (point-max))))
              (with-current-buffer b
                (mapcar (lambda (o) (overlay-get o 'tag))
                        (overlays-in (point-min) (point-max))))))
    (kill-buffer a)
    (kill-buffer b)))
"##,
    );
}

#[test]
fn div_core_indirect_buffers_narrowing_and_local_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (insert "abcdef")
    (let ((ind (make-indirect-buffer (current-buffer) " *probe-indirect*" t)))
      (unwind-protect
          (with-current-buffer ind
            (narrow-to-region 2 5)
            (add-hook 'kill-buffer-hook
                      (lambda ()
                        (push (list 'kill (buffer-name) (buffer-base-buffer)) log))
                      nil t)
            (list (bufferp (buffer-base-buffer))
                  (buffer-string)
                  (point-min) (point-max)
                  (length (buffer-local-value 'kill-buffer-hook ind))
                  (kill-buffer ind)
                  (mapcar (lambda (x)
                            (list (car x) (nth 1 x) (buffer-live-p (nth 2 x))))
                          log)))
        (when (buffer-live-p ind) (kill-buffer ind))))))
"##,
    );
}

#[test]
fn div_core_marker_insertion_and_retarget_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (generate-new-buffer " *probe-marker-a*"))
      (b2 (generate-new-buffer " *probe-marker-b*")))
  (unwind-protect
      (let ((m (make-marker)))
        (with-current-buffer b1
          (insert "abcd")
          (set-marker m 3)
          (set-marker-insertion-type m t)
          (goto-char 3)
          (insert "X"))
        (let ((p1 (marker-position m))
              (it (marker-insertion-type m)))
          (set-marker m 1 b2)
          (list p1 it (marker-position m) (eq (marker-buffer m) b2))))
    (kill-buffer b1)
    (kill-buffer b2)))
"##,
    );
}

#[test]
fn div_core_textprop_overlay_delete_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (add-text-properties 2 5 '(face bold category probe-cat rear-nonsticky t))
  (put 'probe-cat 'face 'italic)
  (let ((ov (make-overlay 3 6)))
    (overlay-put ov 'priority 10)
    (overlay-put ov 'before-string "<")
    (overlay-put ov 'evaporate t)
    (delete-region 3 6)
    (list (buffer-string)
          (get-text-property 2 'face)
          (overlays-in (point-min) (point-max))
          (overlay-buffer ov)
          (overlay-start ov)
          (overlay-end ov))))
"##,
    );
}

#[test]
fn div_core_text_property_stickiness_insert_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ab")
  (add-text-properties 1 2 '(face bold rear-nonsticky (face)))
  (add-text-properties 2 3 '(face italic front-sticky (face)))
  (goto-char 2)
  (insert "X")
  (list (buffer-string)
        (text-properties-at 1)
        (text-properties-at 2)
        (text-properties-at 3)))
"##,
    );
}

#[test]
fn div_core_overlay_front_rear_advance_insert_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcd")
  (let ((o1 (make-overlay 2 3 nil nil nil))
        (o2 (make-overlay 2 3 nil t t)))
    (goto-char 2) (insert "L")
    (goto-char 4) (insert "R")
    (list (buffer-string)
          (list (overlay-start o1) (overlay-end o1))
          (list (overlay-start o2) (overlay-end o2)))))
"##,
    );
}

#[test]
fn div_core_undo_boundaries_and_change_hooks_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (buffer-enable-undo)
    (add-hook 'before-change-functions
              (lambda (&rest args) (push (cons 'before args) log))
              nil t)
    (insert "abc")
    (undo-boundary)
    (insert "def")
    (let ((u1 buffer-undo-list))
      (undo 1)
      (list (buffer-string) (consp u1) (nreverse log)))))
"##,
    );
}

#[test]
fn div_core_window_delete_restores_selected_window_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-fsw-a*"))
      (b2 (get-buffer-create " *probe-fsw-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b1)
        (let ((w2 (split-window nil nil 'right)))
          (set-window-buffer w2 b2)
          (select-window w2)
          (delete-window w2)
          (list (count-windows)
                (eq (selected-window) (frame-selected-window))
                (buffer-name (window-buffer (selected-window)))
                (window-live-p w2))))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_minibuffer_window_frame_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mw (minibuffer-window)))
  (list (window-live-p mw)
        (window-minibuffer-p mw)
        (eq (window-frame mw) (selected-frame))
        (buffer-name (window-buffer mw))
        (active-minibuffer-window)))
"##,
    );
}

#[test]
fn div_core_batch_frame_font_and_display_metrics_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (frame-parameter nil 'font)
      (frame-parameter nil 'font-backend)
      (display-pixel-width)
      (display-pixel-height)
      (display-mm-width)
      (display-mm-height))
"##,
    );
}

#[test]
fn div_core_frame_fullscreen_alpha_sequence_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(unwind-protect
    (progn
      (modify-frame-parameters
       nil '((fullscreen . fullboth) (alpha . 80) (alpha-background . 70)))
      (let ((a (list (frame-parameter nil 'fullscreen)
                     (frame-parameter nil 'alpha)
                     (frame-parameter nil 'alpha-background))))
        (modify-frame-parameters
         nil '((fullscreen . nil) (alpha . nil) (alpha-background . nil)))
        (list a
              (frame-parameter nil 'fullscreen)
              (frame-parameter nil 'alpha)
              (frame-parameter nil 'alpha-background))))
  (modify-frame-parameters
   nil '((fullscreen . nil) (alpha . nil) (alpha-background . nil))))
"##,
    );
}

#[test]
fn div_core_inhibit_modification_hooks_boundary_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (add-hook 'before-change-functions
              (lambda (&rest args) (push (cons 'before args) log))
              nil t)
    (add-hook 'after-change-functions
              (lambda (&rest args) (push (cons 'after args) log))
              nil t)
    (let ((inhibit-modification-hooks t))
      (insert "abc"))
    (insert "d")
    (list (buffer-string) (nreverse log))))
"##,
    );
}

#[test]
fn div_core_overlay_modification_hooks_order_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (insert "abcd")
    (let ((ov (make-overlay 2 4)))
      (overlay-put
       ov 'modification-hooks
       (list (lambda (o after beg end &optional len)
               (push (list 'mod after beg end len) log))))
      (overlay-put
       ov 'insert-in-front-hooks
       (list (lambda (o after beg end &optional len)
               (push (list 'front after beg end len) log))))
      (overlay-put
       ov 'insert-behind-hooks
       (list (lambda (o after beg end &optional len)
               (push (list 'behind after beg end len) log))))
      (goto-char 2) (insert "X")
      (goto-char (overlay-end ov)) (insert "Y")
      (delete-region 3 5)
      (list (buffer-string)
            (nreverse log)
            (list (overlay-start ov) (overlay-end ov)))))))
"##,
    );
}

#[test]
fn div_core_temp_buffer_hooks_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((temp-buffer-setup-hook
         (list (lambda () (push (list 'setup (buffer-name)) log))))
        (temp-buffer-show-hook
         (list (lambda () (push (list 'show (buffer-name)) log)))))
    (with-output-to-temp-buffer "*Probe Help*"
      (princ "hello"))
    (list (get-buffer "*Probe Help*") (nreverse log))))
"##,
    );
}

#[test]
fn div_core_save_window_excursion_restores_buffer_point_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-swe-a*"))
      (b2 (get-buffer-create " *probe-swe-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b1)
        (erase-buffer) (insert "abcdef") (goto-char 4)
        (let ((before (list (buffer-name) (point) (count-windows))))
          (save-window-excursion
            (split-window nil nil 'right)
            (switch-to-buffer b2)
            (erase-buffer) (insert "123") (goto-char 2))
          (list before (buffer-name) (point) (count-windows))))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_save_current_buffer_kill_current_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-scb-a*"))
      (b2 (get-buffer-create " *probe-scb-b*")))
  (unwind-protect
      (progn
        (switch-to-buffer b1)
        (let ((before (current-buffer)))
          (condition-case err
              (save-current-buffer
                (set-buffer b2)
                (kill-buffer b2)
                (buffer-name (current-buffer)))
            (error (list 'err (car err))))
          (list (eq (current-buffer) before)
                (buffer-live-p b2)
                (buffer-name))))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_advice_removed_by_fset_redefinition_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((log nil))
  (defun probe--redef (x) (list 'old x))
  (let ((a (lambda (fn x) (push (list 'around x) log) (funcall fn x))))
    (advice-add 'probe--redef :around a)
    (let ((before (probe--redef 1)))
      (fset 'probe--redef (lambda (x) (list 'new x)))
      (unwind-protect
          (list before (probe--redef 2) log (advice-member-p a 'probe--redef))
        (advice-remove 'probe--redef a)
        (fmakunbound 'probe--redef)))))
"##,
    );
}

#[test]
fn div_core_frame_parameter_delete_default_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(unwind-protect
    (progn
      (modify-frame-parameters nil '((probe-x . 1)))
      (let ((a (list (frame-parameter nil 'probe-x)
                     (assq 'probe-x (frame-parameters)))))
        (modify-frame-parameters nil '((probe-x . nil)))
        (list a
              (frame-parameter nil 'probe-x)
              (assq 'probe-x (frame-parameters)))))
  (modify-frame-parameters nil '((probe-x . nil))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_frame_unsplittable_parameter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (nil (unsplittable))
    // Neomacs:   OK (t (unsplittable . t))
    assert_oracle_parity(
        r##"
(unwind-protect
    (progn
      (modify-frame-parameters nil '((unsplittable . t)))
      (list (frame-parameter nil 'unsplittable)
            (assq 'unsplittable (frame-parameters))))
  (modify-frame-parameters nil '((unsplittable . nil))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_frame_visibility_nil_parameter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (t (visibility . t))
    // Neomacs:   OK (nil (visibility))
    assert_oracle_parity(
        r##"
(unwind-protect
    (progn
      (modify-frame-parameters nil '((visibility . nil)))
      (list (frame-parameter nil 'visibility)
            (assq 'visibility (frame-parameters))))
  (modify-frame-parameters nil '((visibility . t))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_batch_frame_size_and_position_mutation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (80 25 nil nil)
    // Neomacs:   OK (81 27 10 20)
    assert_oracle_parity(
        r##"
(progn
  (set-frame-size nil 81 26)
  (set-frame-position nil 10 20)
  (list (frame-width) (frame-height)
        (frame-parameter nil 'left)
        (frame-parameter nil 'top)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_frame_width_height_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (80 25 80 25 7 8)
    // Neomacs:   OK (90 30 90 30 7 8)
    assert_oracle_parity(
        r##"
(unwind-protect
    (progn
      (modify-frame-parameters
       nil '((width . 90) (height . 30) (left . 7) (top . 8)))
      (list (frame-width) (frame-height)
            (frame-parameter nil 'width)
            (frame-parameter nil 'height)
            (frame-parameter nil 'left)
            (frame-parameter nil 'top)))
  (modify-frame-parameters
   nil '((width . nil) (height . nil) (left . nil) (top . nil))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_resize_split_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((10 14 (0 1 80 11) (0 11 80 25)) 8 16 (0 1 80 9) (0 9 80 25) 2)
    // Neomacs:   OK ((10 14 (0 0 80 10) (0 10 80 24)) 8 16 (0 0 80 8) (0 8 80 24) 2)
    assert_oracle_parity(
        r##"
(progn
  (delete-other-windows)
  (let* ((root (selected-window))
         (w2 (split-window root 10 'below)))
    (let ((before (list (window-total-height root)
                        (window-total-height w2)
                        (window-edges root)
                        (window-edges w2))))
      (condition-case err
          (window-resize w2 2 nil nil nil)
        (error (push (cons 'resize-error (cons (car err) (cdr err)))
                     before)))
      (prog1 (list before
                   (window-total-height root)
                   (window-total-height w2)
                   (window-edges root)
                   (window-edges w2)
                   (count-windows))
        (delete-other-windows)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_start_end_scroll_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((15 30 561) 22 30 561)
    // Neomacs:   OK ((15 30 176) 50 50 211)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (dotimes (i 80) (insert (format "line%02d\n" i)))
  (switch-to-buffer (current-buffer))
  (let ((w (selected-window)))
    (set-window-start w 15)
    (set-window-point w 30)
    (let ((before (list (window-start w) (window-point w) (window-end w t))))
      (condition-case err
          (scroll-up 3)
        (error
         (setq before
               (cons (cons 'scroll-error (cons (car err) (cdr err)))
                     before))))
      (list before (window-start w) (window-point w) (window-end w t)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_margins_body_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (5 (2 . 3) (0 0 nil nil) 75)
    // Neomacs:   OK (5 (2 . 3) (0 0 nil nil) 80)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdefghijklmnopqrstuvwxyz")
  (switch-to-buffer (current-buffer))
  (let ((w (selected-window)))
    (set-window-hscroll w 5)
    (set-window-margins w 2 3)
    (set-window-fringes w 4 5 nil)
    (list (window-hscroll w)
          (window-margins w)
          (window-fringes w)
          (window-body-width w))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_switch_buffer_update_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (" *probe-switch-b*" ((update "*scratch*") (update " *probe-switch-a*") (update " *probe-switch-b*")))
    // Neomacs:   OK (" *probe-switch-b*" nil)
    assert_oracle_parity(
        r##"
(let ((log nil)
      (b1 (get-buffer-create " *probe-switch-a*"))
      (b2 (get-buffer-create " *probe-switch-b*")))
  (unwind-protect
      (let ((buffer-list-update-hook
             (list (lambda () (push (list 'update (buffer-name)) log)))))
        (switch-to-buffer b1)
        (switch-to-buffer b2)
        (list (buffer-name) (nreverse log)))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_parameter_configuration_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (after (probe . after))
    // Neomacs:   OK (before (probe . before))
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-wparam*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b)
        (set-window-parameter nil 'probe 'before)
        (let ((cfg (current-window-configuration)))
          (set-window-parameter nil 'probe 'after)
          (set-window-configuration cfg)
          (list (window-parameter nil 'probe)
                (assq 'probe (window-parameters)))))
    (when (buffer-live-p b) (kill-buffer b))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_kill_all_local_variables_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (local t 70 nil ((change local 33)))
    // Neomacs:   OK (local t 70 nil nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (defvar probe--perm-local 'global)
  (put 'probe--perm-local 'permanent-local t)
  (with-temp-buffer
    (setq-local probe--perm-local 'local)
    (setq-local fill-column 33)
    (add-hook 'change-major-mode-hook
              (lambda ()
                (push (list 'change probe--perm-local fill-column) log))
              nil t)
    (kill-all-local-variables)
    (list probe--perm-local
          (local-variable-p 'probe--perm-local)
          fill-column
          (local-variable-p 'fill-column)
          log)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_derived_mode_change_hook_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (probe-child-mode (change parent-body child-body parent-hook child-hook) probe-parent-mode nil)
    // Neomacs:   OK (probe-child-mode (parent-body child-body parent-hook child-hook) probe-parent-mode nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (define-derived-mode probe-parent-mode fundamental-mode "ProbeParent"
    (push 'parent-body log))
  (define-derived-mode probe-child-mode probe-parent-mode "ProbeChild"
    (push 'child-body log))
  (add-hook 'change-major-mode-hook (lambda () (push 'change log)))
  (add-hook 'probe-parent-mode-hook (lambda () (push 'parent-hook log)))
  (add-hook 'probe-child-mode-hook (lambda () (push 'child-hook log)))
  (with-temp-buffer
    (probe-child-mode)
    (list major-mode
          (nreverse log)
          (derived-mode-p 'probe-parent-mode)
          (derived-mode-p 'fundamental-mode))))
"##,
    );
}

#[test]
fn div_core_after_change_major_mode_hook_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: after-change-major-mode-hook runs with the new major mode
    // already installed.
    assert_oracle_parity(
        r##"
(let ((log nil))
  (add-hook 'after-change-major-mode-hook
            (lambda () (push (list 'after major-mode) log)))
  (with-temp-buffer
    (text-mode)
    (list major-mode (nreverse log))))
"##,
    );
}

#[test]
fn div_core_field_boundary_motion_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: field-beginning/field-end/field-string and constrain-to-field
    // across three text-property fields.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "aa bb cc")
  (put-text-property 1 3 'field 'left)
  (put-text-property 4 6 'field 'mid)
  (put-text-property 7 9 'field 'right)
  (list (field-beginning 5) (field-end 5) (field-string 5)
        (constrain-to-field 8 5)
        (constrain-to-field 2 5)))
"##,
    );
}

#[test]
fn div_core_default_value_buffer_local_symbol_plist_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: set-default after per-buffer setq-local, plus symbol plist.
    assert_oracle_parity(
        r##"
(progn
  (defvar probe--bufvar 'global)
  (put 'probe--bufvar 'safe-local-variable #'symbolp)
  (let ((b1 (generate-new-buffer " *probe-var-a*"))
        (b2 (generate-new-buffer " *probe-var-b*")))
    (unwind-protect
        (progn
          (with-current-buffer b1 (setq-local probe--bufvar 'a))
          (with-current-buffer b2 (setq-local probe--bufvar 'b))
          (set-default 'probe--bufvar 'new-global)
          (list (default-value 'probe--bufvar)
                (buffer-local-value 'probe--bufvar b1)
                (buffer-local-value 'probe--bufvar b2)
                (get 'probe--bufvar 'safe-local-variable)))
      (kill-buffer b1)
      (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_text_mode_change_major_mode_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (text-mode (change (after text-mode)))
    // Neomacs:   OK (text-mode ((after text-mode)))
    assert_oracle_parity(
        r##"
(let ((log nil))
  (add-hook 'change-major-mode-hook (lambda () (push 'change log)))
  (add-hook 'after-change-major-mode-hook
            (lambda () (push (list 'after major-mode) log)))
  (with-temp-buffer
    (text-mode)
    (list major-mode (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_set_auto_mode_change_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (text-mode (change text-hook))
    // Neomacs:   OK (text-mode (text-hook))
    assert_oracle_parity(
        r##"
(let ((log nil)
      (auto-mode-alist '(("\\.probe\\'" . text-mode))))
  (add-hook 'change-major-mode-hook (lambda () (push 'change log)))
  (add-hook 'text-mode-hook (lambda () (push 'text-hook log)))
  (with-temp-buffer
    (setq buffer-file-name "x.probe")
    (set-auto-mode)
    (list major-mode (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_read_only_before_change_hook_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24: a rejected read-only insertion still
    // double-fires before-change-functions in Neomacs.
    // GNU Emacs: OK (buffer-read-only ok "abcY" ((before 4 4)))
    // Neomacs:   OK (buffer-read-only ok "abcY" ((before 4 4) (before 4 4)))
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (insert "abc")
    (setq buffer-read-only t)
    (add-hook 'before-change-functions
              (lambda (&rest args) (push (cons 'before args) log))
              nil t)
    (let ((err1 (condition-case err (progn (insert "X") 'ok) (error (car err))))
          (err2 (let ((inhibit-read-only t))
                  (goto-char (point-max))
                  (insert "Y")
                  'ok)))
      (list err1 err2 (buffer-string) (nreverse log)))))
"##,
    );
}

#[test]
fn div_core_file_temp_attributes_and_insert_file_contents_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: temp file creation, file attributes, and insert-file-contents
    // without retaining volatile absolute temp names in the asserted result.
    assert_oracle_parity(
        r##"
(let* ((dir (make-temp-file "neo-probe" t))
       (f1 (expand-file-name "a.txt" dir))
       (f2 (make-temp-file "neo-probe" nil ".txt" "abc\ndef")))
  (unwind-protect
      (progn
        (write-region "hello" nil f1 nil 'silent)
        (list
         (file-exists-p f1)
         (file-readable-p f1)
         (file-directory-p dir)
         (nth 7 (file-attributes f1 'integer))
         (file-name-nondirectory (file-truename f1))
         (with-temp-buffer
           (let ((ret (insert-file-contents f2)))
             (list (string-match-p "\\`neo-probe.*\\.txt\\'" (file-name-nondirectory (car ret)))
                   (cadr ret)
                   (buffer-string)
                   buffer-file-name
                   (buffer-modified-p))))))
    (delete-directory dir t)
    (delete-file f2)))
"##,
    );
}

#[test]
fn div_core_call_process_environment_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: call-process, shell-command-switch, process-environment, and
    // getenv binding all agree in batch mode.
    assert_oracle_parity(
        r##"
(let ((process-environment (cons "NEO_PROBE_VAR=xyz" process-environment)))
  (list
   (with-temp-buffer
     (let ((status (call-process shell-file-name nil t nil
                                 shell-command-switch "printf 'a\\nb'")))
       (list status (buffer-string))))
   (with-temp-buffer
     (let ((status (call-process shell-file-name nil t nil
                                 shell-command-switch "printf $NEO_PROBE_VAR")))
       (list status (buffer-string) (getenv "NEO_PROBE_VAR"))))))
"##,
    );
}

#[test]
fn div_core_register_point_text_and_rectangle_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: point registers, text registers, and rectangle registers.
    assert_oracle_parity(
        r##"
(list
 (with-temp-buffer
   (insert "abcdef")
   (goto-char 4)
   (point-to-register ?a)
   (copy-to-register ?b 2 5)
   (list (markerp (get-register ?a))
         (marker-position (get-register ?a))
         (get-register ?b)))
 (with-temp-buffer
   (insert "aa11\nbb22\ncc33\n")
   (copy-rectangle-to-register ?r 3 13)
   (erase-buffer)
   (insert-register ?r)
   (list (get-register ?r) (buffer-string))))
"##,
    );
}

#[test]
fn div_core_timer_absolute_and_idle_shape_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: absolute timer and idle timer shape are stable when the
    // scheduled time is explicit.
    assert_oracle_parity(
        r##"
(list
 (let ((tm (run-at-time '(0 1 2 345000) nil (lambda () 'never))))
   (unwind-protect
       (list (timerp tm) (timer--time tm) (timer--repeat-delay tm))
     (cancel-timer tm)))
 (let ((tm (run-with-idle-timer 1000 nil (lambda () 'never))))
   (unwind-protect
       (list (timerp tm) (memq tm timer-idle-list) (timer--idle-delay tm))
     (cancel-timer tm))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_relative_timer_microseconds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: relative run-at-time retains a nonzero microsecond component.
    // Neomacs:   relative run-at-time reports zero microseconds.
    assert_oracle_parity(
        r##"
(let ((tm (run-at-time 1000 nil (lambda () 'never))))
  (unwind-protect
      (let ((tt (timer--time tm)))
        (list (timerp tm)
              (integerp (nth 0 tt))
              (integerp (nth 1 tt))
              (integerp (nth 2 tt))
              (not (zerop (nth 3 tt)))
              (timer--repeat-delay tm)))
    (cancel-timer tm)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_repeating_timer_microseconds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: repeating run-at-time keeps a nonzero microsecond component.
    // Neomacs:   repeating run-at-time reports zero microseconds.
    assert_oracle_parity(
        r##"
(let ((tm (run-at-time 10 5 (lambda () 'never))))
  (unwind-protect
      (let ((tt (timer--time tm)))
        (list (timerp tm)
              (integerp (nth 0 tt))
              (integerp (nth 1 tt))
              (integerp (nth 2 tt))
              (not (zerop (nth 3 tt)))
              (timer--repeat-delay tm)))
    (cancel-timer tm)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_message_repetition_coalescing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (nil "same [3 times]\n")
    // Neomacs:   OK (nil "same\nsame\nsame\n")
    assert_oracle_parity(
        r##"
(let ((message-log-max t))
  (with-current-buffer (get-buffer-create "*Messages*")
    (let ((inhibit-read-only t))
      (erase-buffer)))
  (message "same")
  (message "same")
  (message "same")
  (list (current-message)
        (with-current-buffer "*Messages*" (buffer-string))))
"##,
    );
}

#[test]
fn div_core_variable_watcher_set_and_buffer_local_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: variable watchers receive global set/setq-default events,
    // buffer-local set events, and makunbound from kill-local-variable.
    assert_oracle_parity(
        r##"
(list
 (let ((log nil))
   (defvar probe--watch 0)
   (let ((watcher (lambda (sym new op where)
                    (push (list sym new op (bufferp where)) log))))
     (add-variable-watcher 'probe--watch watcher)
     (unwind-protect
         (progn
           (setq probe--watch 1)
           (set 'probe--watch 2)
           (setq-default probe--watch 3)
           (list probe--watch (default-value 'probe--watch) (nreverse log)))
       (remove-variable-watcher 'probe--watch watcher))))
 (let ((log nil))
   (defvar probe--watch-local 0)
   (let ((watcher (lambda (sym new op where)
                    (push (list sym new op
                                (and (bufferp where) (buffer-name where)))
                          log))))
     (add-variable-watcher 'probe--watch-local watcher)
     (unwind-protect
         (with-temp-buffer
           (rename-buffer " *probe-watch-local*" t)
           (setq-local probe--watch-local 10)
           (setq probe--watch-local 11)
           (kill-local-variable 'probe--watch-local)
           (list probe--watch-local
                 (default-value 'probe--watch-local)
                 (nreverse log)))
       (remove-variable-watcher 'probe--watch-local watcher)))))
"##,
    );
}

#[test]
fn div_core_make_variable_buffer_local_default_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: automatically buffer-local variables keep per-buffer values
    // after a default value mutation.
    assert_oracle_parity(
        r##"
(progn
  (defvar probe--auto-local 'global)
  (make-variable-buffer-local 'probe--auto-local)
  (let ((b1 (generate-new-buffer " *probe-auto-a*"))
        (b2 (generate-new-buffer " *probe-auto-b*")))
    (unwind-protect
        (progn
          (with-current-buffer b1 (setq probe--auto-local 'a))
          (with-current-buffer b2 (setq probe--auto-local 'b))
          (setq-default probe--auto-local 'new-default)
          (list (default-value 'probe--auto-local)
                (buffer-local-value 'probe--auto-local b1)
                (buffer-local-value 'probe--auto-local b2)
                (local-variable-if-set-p 'probe--auto-local b1)
                (local-variable-if-set-p 'probe--auto-local b2)))
      (kill-buffer b1)
      (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_window_state_get_put_keymap_and_face_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Parity lock: window-state-get/put, remapping keymaps, event descriptions,
    // and batch face attributes in one broader basic-subsystem probe.
    assert_oracle_parity(
        r##"
(list
 (let ((b1 (get-buffer-create " *probe-state-a*"))
       (b2 (get-buffer-create " *probe-state-b*")))
   (unwind-protect
       (progn
         (delete-other-windows)
         (switch-to-buffer b1)
         (let ((w2 (split-window nil nil 'right)))
           (set-window-buffer w2 b2)
           (set-window-parameter w2 'probe 'yes)
           (let ((state (window-state-get nil t)))
             (delete-other-windows)
             (window-state-put state nil 'safe)
             (list (count-windows)
                   (mapcar (lambda (w) (buffer-name (window-buffer w)))
                           (window-list nil 'nomini))
                   (mapcar (lambda (w) (window-parameter w 'probe))
                           (window-list nil 'nomini))))))
     (when (buffer-live-p b1) (kill-buffer b1))
     (when (buffer-live-p b2) (kill-buffer b2))))
 (let ((map (make-sparse-keymap)))
   (define-key map [remap old-cmd] 'new-cmd)
   (define-key map (kbd "C-c n") 'new-cmd)
   (list (command-remapping 'old-cmd nil (list map))
         (where-is-internal 'new-cmd map t)
         (lookup-key map [remap old-cmd])))
 (list (key-description [C-a])
       (key-description [C-S-a])
       (single-key-description ?\C-a)
       (kbd "C-c <f5>")
       (event-modifiers 'C-S-a)
       (event-basic-type 'C-S-a))
 (list (facep 'default)
       (face-attribute 'default :foreground nil 'default)
       (face-attribute 'default :background nil 'default)
       (face-attribute 'bold :weight nil 'default)
       (face-all-attributes 'bold nil)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_permanent_local_mode_change_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (local t nil nil ((change local probe)))
    // Neomacs:   OK (local t nil nil nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (defvar probe--perm2 'global)
  (put 'probe--perm2 'permanent-local t)
  (with-temp-buffer
    (setq-local probe--perm2 'local)
    (setq-local transient-mark-mode 'probe)
    (add-hook 'change-major-mode-hook
              (lambda () (push (list 'change probe--perm2 transient-mark-mode)
                               log))
              nil t)
    (fundamental-mode)
    (text-mode)
    (list probe--perm2
          (local-variable-p 'probe--perm2)
          transient-mark-mode
          (local-variable-p 'transient-mark-mode)
          (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_prev_buffers_after_previous_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("*scratch*" nil (" *probe-prev-c*"))
    // Neomacs:   OK ("*scratch*" (("*scratch*" 1 1)) (" *probe-prev-c*"))
    assert_oracle_parity(
        r##"
(let ((b1 (get-buffer-create " *probe-prev-a*"))
      (b2 (get-buffer-create " *probe-prev-b*"))
      (b3 (get-buffer-create " *probe-prev-c*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer b1)
        (switch-to-buffer b2)
        (switch-to-buffer b3)
        (previous-buffer)
        (let ((w (selected-window)))
          (list (buffer-name (window-buffer w))
                (mapcar (lambda (e)
                          (list (buffer-name (nth 0 e))
                                (marker-position (nth 1 e))
                                (marker-position (nth 2 e))))
                        (window-prev-buffers w))
                (mapcar #'buffer-name (window-next-buffers w)))))
    (mapc (lambda (b) (when (buffer-live-p b) (kill-buffer b)))
          (list b1 b2 b3))))
"##,
    );
}
