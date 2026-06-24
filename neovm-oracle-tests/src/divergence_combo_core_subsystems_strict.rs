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

#[test]
fn div_core_divergence_surface_buffer_rename_update_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("probe-rename-new" ("*scratch*" "probe-rename-new"))
    // Neomacs:   OK ("probe-rename-new" ("*scratch*"))
    assert_oracle_parity(
        r##"
(let ((log nil)
      (buffer-list-update-hook nil))
  (add-hook 'buffer-list-update-hook
            (lambda () (push (buffer-name) log)))
  (let ((b (generate-new-buffer "probe-rename")))
    (unwind-protect
        (with-current-buffer b
          (rename-buffer "probe-rename-new" t)
          (list (buffer-name) (nreverse log)))
      (when (buffer-live-p b) (kill-buffer b)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_set_buffer_major_mode_change_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (fundamental-mode "Fundamental" (text-mode))
    // Neomacs:   OK (fundamental-mode "Fundamental" nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (with-temp-buffer
    (setq major-mode 'text-mode
          mode-name "Text")
    (add-hook 'change-major-mode-hook
              (lambda () (push major-mode log))
              nil t)
    (set-buffer-major-mode (current-buffer))
    (list major-mode mode-name (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_global_major_mode_hook_order_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (fundamental-mode ((change fundamental-mode) (text-hook text-mode) (after text-mode) (change text-mode) (after fundamental-mode)))
    // Neomacs:   OK (fundamental-mode ((text-hook text-mode) (after text-mode) (after fundamental-mode)))
    assert_oracle_parity(
        r##"
(let ((log nil)
      (change-major-mode-hook nil)
      (after-change-major-mode-hook nil)
      (text-mode-hook nil))
  (add-hook 'change-major-mode-hook
            (lambda () (push (list 'change major-mode) log)))
  (add-hook 'after-change-major-mode-hook
            (lambda () (push (list 'after major-mode) log)))
  (add-hook 'text-mode-hook
            (lambda () (push (list 'text-hook major-mode) log)))
  (with-temp-buffer
    (text-mode)
    (fundamental-mode)
    (list major-mode (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_overlay_category_evaporate_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("aef" nil nil nil #("<" 0 1 (face bold)) ">")
    // Neomacs:   OK ("aef" #<killed buffer> 2 2 #("<" 0 1 (face bold)) ">")
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((o (make-overlay 2 5)))
    (overlay-put o 'before-string (propertize "<" 'face 'bold))
    (overlay-put o 'after-string ">")
    (overlay-put o 'category 'probe-cat)
    (put 'probe-cat 'evaporate t)
    (delete-region 2 5)
    (list (buffer-string)
          (overlay-buffer o)
          (overlay-start o)
          (overlay-end o)
          (overlay-get o 'before-string)
          (overlay-get o 'after-string))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_bury_buffer_update_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (" *probe-bury*" ("*scratch*" " *probe-bury*" " *probe-bury*"))
    // Neomacs:   OK (" *probe-bury*" ("*scratch*" " *probe-bury*"))
    assert_oracle_parity(
        r##"
(let ((log nil)
      (buffer-list-update-hook nil)
      (b (get-buffer-create " *probe-bury*")))
  (unwind-protect
      (progn
        (add-hook 'buffer-list-update-hook
                  (lambda () (push (buffer-name) log)))
        (switch-to-buffer b)
        (bury-buffer b)
        (list (buffer-name (current-buffer)) (nreverse log)))
    (when (buffer-live-p b) (kill-buffer b))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_configuration_hook_batch_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (1 nil)
    // Neomacs:   OK (1 ((config 2)))
    assert_oracle_parity(
        r##"
(let ((log nil)
      (b1 (get-buffer-create " *probe-wh3-a*"))
      (b2 (get-buffer-create " *probe-wh3-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (setq window-configuration-change-hook nil
              window-buffer-change-functions nil
              window-size-change-functions nil
              window-selection-change-functions nil)
        (add-hook 'window-configuration-change-hook
                  (lambda () (push (list 'config (count-windows)) log)))
        (add-hook 'window-buffer-change-functions
                  (lambda (w)
                    (push (list 'buf (buffer-name (window-buffer w))) log)))
        (add-hook 'window-size-change-functions
                  (lambda (f) (push (list 'size (framep f) (count-windows)) log)))
        (add-hook 'window-selection-change-functions
                  (lambda (f) (push (list 'select (framep f) (buffer-name)) log)))
        (switch-to-buffer b1)
        (let ((w2 (split-window nil nil 'right)))
          (set-window-buffer w2 b2)
          (select-window w2)
          (delete-window w2))
        (list (count-windows) (nreverse log)))
    (when (buffer-live-p b1) (kill-buffer b1))
    (when (buffer-live-p b2) (kill-buffer b2))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_overlay_category_modification_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("abXdef" 2 5 ((cat nil 3 3 nil 2 5) (cat t 3 5 0 2 7) (cat nil 4 6 nil 2 7) (cat t 4 4 2 2 5)))
    // Neomacs:   OK ("abXdef" 2 5 nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((log nil))
    (put 'probe-cat3 'modification-hooks
         (list (lambda (o after beg end &optional len)
                 (push (list 'cat after beg end len
                             (overlay-start o) (overlay-end o))
                       log))))
    (let ((o (make-overlay 2 5)))
      (overlay-put o 'category 'probe-cat3)
      (goto-char 3)
      (insert "XX")
      (delete-region 4 6)
      (list (buffer-string)
            (overlay-start o)
            (overlay-end o)
            (nreverse log)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_overlay_category_insert_in_front_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("abXXcdef" 3 3 ((front nil 3 3 nil 3 3) (front t 3 5 0 3 3)))
    // Neomacs:   OK ("abXXcdef" 3 3 nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((log nil))
    (put 'probe-cat4 'insert-in-front-hooks
         (list (lambda (o after beg end &optional len)
                 (push (list 'front after beg end len
                             (overlay-start o) (overlay-end o))
                       log))))
    (let ((o (make-overlay 3 3 nil t nil)))
      (overlay-put o 'category 'probe-cat4)
      (goto-char 3)
      (insert "XX")
      (list (buffer-string)
            (overlay-start o)
            (overlay-end o)
            (nreverse log)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_overlay_category_insert_behind_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("abXXcdef" 3 5 ((behind nil 3 3 nil 3 3) (behind t 3 5 0 3 5)))
    // Neomacs:   OK ("abXXcdef" 3 5 nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((log nil))
    (put 'probe-cat5 'insert-behind-hooks
         (list (lambda (o after beg end &optional len)
                 (push (list 'behind after beg end len
                             (overlay-start o) (overlay-end o))
                       log))))
    (let ((o (make-overlay 3 3 nil nil t)))
      (overlay-put o 'category 'probe-cat5)
      (goto-char 3)
      (insert "XX")
      (list (buffer-string)
            (overlay-start o)
            (overlay-end o)
            (nreverse log)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_frame_parameters_buffer_list_modeline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((no-accept-focus) (modeline . t) nil ("*scratch*") nil)
    // Neomacs:   OK (nil nil (font-parameter) nil nil)
    assert_oracle_parity(
        r##"
(let ((params (frame-parameters (selected-frame))))
  (list (assq 'no-accept-focus params)
        (assq 'modeline params)
        (assq 'font-parameter params)
        (mapcar (lambda (b) (buffer-name b))
                (cdr (assq 'buffer-list params)))
        (cdr (assq 'buried-buffer-list params))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_text_category_modification_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (#("abXdef" 1 2 (category probe-text-cat3) 3 4 (category probe-text-cat3)) ((4 6)))
    // Neomacs:   OK (#("abXdef" 1 2 (category probe-text-cat3) 3 4 (category probe-text-cat3)) nil)
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (let ((log nil))
    (put 'probe-text-cat3 'modification-hooks
         (list (lambda (&rest args) (push args log))))
    (put-text-property 2 5 'category 'probe-text-cat3)
    (goto-char 3)
    (insert "XX")
    (delete-region 4 6)
    (list (buffer-string) (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_scroll_error_and_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((err beginning-of-buffer 13 13 201 50) (ok 201 145 201))
    // Neomacs:   OK ((err end-of-buffer 1 1 93 50) (ok 189 189 201))
    assert_oracle_parity(
        r##"
(list
 (let ((b (get-buffer-create " *probe-win-scroll*")))
   (unwind-protect
       (progn
         (with-current-buffer b
           (erase-buffer)
           (dotimes (i 50) (insert (format "l%02d\n" i))))
         (delete-other-windows)
         (switch-to-buffer b)
         (goto-char (point-min))
         (condition-case err
             (progn
               (scroll-up 3)
               (let ((s1 (window-start))
                     (p1 (point)))
                 (scroll-down 1)
                 (list 'ok s1 p1 (window-start) (point))))
           (error (list 'err
                        (car err)
                        (point)
                        (window-start)
                        (window-end nil t)
                        (count-lines (point-min) (point-max))))))
     (when (buffer-live-p b) (kill-buffer b))))
 (let ((b (get-buffer-create " *probe-win-scroll2*")))
   (unwind-protect
       (progn
         (with-current-buffer b
           (erase-buffer)
           (dotimes (i 50) (insert (format "l%02d\n" i))))
         (delete-other-windows)
         (switch-to-buffer b)
         (goto-char (point-max))
         (condition-case err
             (progn
               (scroll-down 3)
               (list 'ok (point) (window-start) (window-end nil t)))
           (error (list 'err (car err) (point) (window-start) (window-end nil t)))))
     (when (buffer-live-p b) (kill-buffer b)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_recenter_window_end_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (80 57 201)
    // Neomacs:   OK (80 57 149)
    assert_oracle_parity(
        r##"
(let ((b (get-buffer-create " *probe-recenter*")))
  (unwind-protect
      (progn
        (with-current-buffer b
          (erase-buffer)
          (dotimes (i 50) (insert (format "l%02d\n" i))))
        (delete-other-windows)
        (switch-to-buffer b)
        (goto-char 80)
        (recenter 5)
        (list (point) (window-start) (window-end nil t)))
    (when (buffer-live-p b) (kill-buffer b))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_set_window_configuration_killed_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (2 (" *probe-ks-a*" "*scratch*"))
    // Neomacs:   OK (2 (" *probe-ks-a*" nil))   ; leaves a window whose buffer is nil
    //            and emits a "Selecting deleted buffer" redisplay error.
    assert_oracle_parity(
        r##"
(let ((a (get-buffer-create " *probe-ks-a*"))
      (b (get-buffer-create " *probe-ks-b*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer a)
        (let ((w2 (split-window-below)))
          (set-window-buffer w2 b)
          (let ((cfg (current-window-configuration)))
            (delete-other-windows)
            (kill-buffer b)
            (set-window-configuration cfg)
            (list (count-windows)
                  (mapcar (lambda (w) (buffer-name (window-buffer w)))
                          (window-list nil 'nomini))))))
    (when (buffer-live-p a) (kill-buffer a))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_compare_window_configurations_split_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK nil   ; split + delete leaves a configuration GNU treats as different
    // Neomacs:   OK t     ; Neomacs treats it as identical to the pre-split configuration
    assert_oracle_parity(
        r##"
(let ((a (get-buffer-create " *probe-cfg-cmp2-a*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer a)
        (let ((cfg1 (current-window-configuration))
              (w2 (split-window-below)))
          (delete-window w2)
          (compare-window-configurations cfg1 (current-window-configuration))))
    (when (buffer-live-p a) (kill-buffer a))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_window_configuration_register_killed_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (2 (" *probe-winreg-a*" "*scratch*"))
    // Neomacs:   OK (2 (" *probe-winreg-a*" nil))   ; register restore leaves a nil-buffer window
    //            and emits a "Selecting deleted buffer" redisplay error.
    assert_oracle_parity(
        r##"
(let ((a (get-buffer-create " *probe-winreg-a*"))
      (b (get-buffer-create " *probe-winreg-b*"))
      (register-alist nil))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer a)
        (let ((w2 (split-window-below)))
          (set-window-buffer w2 b)
          (window-configuration-to-register ?w)
          (delete-other-windows)
          (kill-buffer b)
          (jump-to-register ?w)
          (list (count-windows)
                (mapcar (lambda (w) (buffer-name (window-buffer w)))
                        (window-list nil 'nomini)))))
    (when (buffer-live-p a) (kill-buffer a))
    (when (buffer-live-p b) (kill-buffer b))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_frame_buffer_list_after_bury() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((" *probe-frame-buf-a*" "*scratch*") (" *probe-frame-buf-a*" "*scratch*") (" *probe-frame-buf-b*"))
    // Neomacs:   OK ((" *probe-frame-buf-a*") (" *probe-frame-buf-a*") (" *probe-frame-buf-b*"))
    assert_oracle_parity(
        r##"
(let ((a (get-buffer-create " *probe-frame-buf-a*"))
      (b (get-buffer-create " *probe-frame-buf-b*")))
  (unwind-protect
      (progn
        (switch-to-buffer a)
        (switch-to-buffer b)
        (bury-buffer b)
        (let ((params (frame-parameters)))
          (list (mapcar #'buffer-name (frame-parameter nil 'buffer-list))
                (mapcar #'buffer-name (cdr (assq 'buffer-list params)))
                (mapcar #'buffer-name (cdr (assq 'buried-buffer-list params))))))
    (mapc (lambda (x) (when (buffer-live-p x) (kill-buffer x)))
          (list a b))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_next_previous_buffer_after_bury() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("*Messages*" "*scratch*" ("*Messages*") ("*Messages*"))
    // Neomacs:   OK ("*Messages*" "*scratch*" ("*Messages*" "*scratch*") ("*Messages*"))
    assert_oracle_parity(
        r##"
(let ((a (get-buffer-create " *probe-nextbuf-a*"))
      (b (get-buffer-create " *probe-nextbuf-b*"))
      (c (get-buffer-create " *probe-nextbuf-c*")))
  (unwind-protect
      (progn
        (delete-other-windows)
        (switch-to-buffer a)
        (switch-to-buffer b)
        (switch-to-buffer c)
        (bury-buffer b)
        (next-buffer)
        (let ((after-next (buffer-name)))
          (previous-buffer)
          (list after-next
                (buffer-name)
                (mapcar (lambda (e) (buffer-name (car e)))
                        (window-prev-buffers))
                (mapcar #'buffer-name (window-next-buffers)))))
    (mapc (lambda (x) (when (buffer-live-p x) (kill-buffer x)))
          (list a b c))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_kill_buffer_live_process_hangup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (signal nil #<killed buffer> nil (("hangup\n" signal nil)))
    // Neomacs:   OK (run (run open listen connect stop) #<killed buffer> nil nil)
    assert_oracle_parity(
        r##"
(let ((buf (get-buffer-create " *probe-proc-killbuf2*"))
      (log nil))
  (let ((proc (make-process
               :name "probe-proc-killbuf2"
               :buffer buf
               :command '("/bin/sh" "-c" "read line")
               :connection-type 'pipe
               :sentinel (lambda (p e)
                           (push (list e
                                       (process-status p)
                                       (buffer-live-p (process-buffer p)))
                                 log)))))
    (set-process-query-on-exit-flag proc nil)
    (let (result)
      (unwind-protect
          (progn
            (kill-buffer buf)
            (let ((i 0))
              (while (and (process-live-p proc) (< i 20))
                (accept-process-output proc 0.05)
                (setq i (1+ i))))
            (setq result
                  (list (process-status proc)
                        (process-live-p proc)
                        (process-buffer proc)
                        (marker-buffer (process-mark proc))
                        (nreverse log))))
        (when (process-live-p proc) (delete-process proc)))
      result)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_delete_process_missing_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (signal nil (("killed\n" signal)))
    // Neomacs:   OK (signal nil nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((proc (make-process
               :name "probe-del-sent-clean"
               :command '("/bin/sh" "-c" "read line")
               :connection-type 'pipe
               :sentinel (lambda (p e)
                           (push (list e (process-status p)) log)))))
    (set-process-query-on-exit-flag proc nil)
    (delete-process proc)
    (let ((i 0))
      (while (and (null log) (< i 20))
        (accept-process-output nil 0.05)
        (setq i (1+ i))))
    (list (process-status proc)
          (process-live-p proc)
          (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_interrupt_process_missing_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (signal 2 (("interrupt\n" signal 2)))
    // Neomacs:   OK (signal 2 nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((proc (make-process
               :name "probe-interrupt-sent"
               :command '("/bin/sh" "-c" "trap 'exit 42' INT; read line")
               :connection-type 'pipe
               :sentinel (lambda (p e)
                           (push (list e
                                       (process-status p)
                                       (process-exit-status p))
                                 log)))))
    (set-process-query-on-exit-flag proc nil)
    (interrupt-process proc)
    (let ((i 0))
      (while (and (process-live-p proc) (< i 20))
        (accept-process-output proc 0.05)
        (setq i (1+ i))))
    (let ((j 0))
      (while (and (null log) (< j 20))
        (accept-process-output proc 0.05)
        (setq j (1+ j))))
    (prog1 (list (process-status proc)
                 (process-exit-status proc)
                 (nreverse log))
      (when (process-live-p proc) (delete-process proc)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_kill_process_missing_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (signal 9 (("killed\n" signal 9)))
    // Neomacs:   OK (signal 9 nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((proc (make-process
               :name "probe-kill-sent"
               :command '("/bin/sh" "-c" "read line")
               :connection-type 'pipe
               :sentinel (lambda (p e)
                           (push (list e
                                       (process-status p)
                                       (process-exit-status p))
                                 log)))))
    (set-process-query-on-exit-flag proc nil)
    (kill-process proc)
    (let ((i 0))
      (while (and (process-live-p proc) (< i 20))
        (accept-process-output proc 0.05)
        (setq i (1+ i))))
    (let ((j 0))
      (while (and (null log) (< j 20))
        (accept-process-output proc 0.05)
        (setq j (1+ j))))
    (prog1 (list (process-status proc)
                 (process-exit-status proc)
                 (nreverse log))
      (when (process-live-p proc) (delete-process proc)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_quit_process_sentinel_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (signal 3 (("quit (core dumped)\n" signal 3)))
    // Neomacs:   OK (signal 3 (("quit\n" signal 3)))
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((proc (make-process
               :name "probe-quit-sent"
               :command '("/bin/sh" "-c" "read line")
               :connection-type 'pipe
               :sentinel (lambda (p e)
                           (push (list e
                                       (process-status p)
                                       (process-exit-status p))
                                 log)))))
    (set-process-query-on-exit-flag proc nil)
    (quit-process proc)
    (let ((i 0))
      (while (and (process-live-p proc) (< i 20))
        (accept-process-output proc 0.05)
        (setq i (1+ i))))
    (let ((j 0))
      (while (and (null log) (< j 20))
        (accept-process-output proc 0.05)
        (setq j (1+ j))))
    (prog1 (list (process-status proc)
                 (process-exit-status proc)
                 (nreverse log))
      (when (process-live-p proc) (delete-process proc)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_stop_continue_delete_process_sentinels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (signal 9 (("run" run 0) ("killed\n" signal 9)))
    // Neomacs:   OK (signal 9 nil)
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((proc (make-process
               :name "probe-stop-cont-sent2"
               :command '("/bin/sh" "-c" "read line")
               :connection-type 'pipe
               :sentinel (lambda (p e)
                           (push (list e
                                       (process-status p)
                                       (process-exit-status p))
                                 log)))))
    (set-process-query-on-exit-flag proc nil)
    (stop-process proc)
    (let ((i 0))
      (while (and (< i 10) (< (length log) 1))
        (accept-process-output proc 0.05)
        (setq i (1+ i))))
    (continue-process proc)
    (let ((i 0))
      (while (and (< i 10) (< (length log) 2))
        (accept-process-output proc 0.05)
        (setq i (1+ i))))
    (delete-process proc)
    (let ((i 0))
      (while (and (< i 20) (< (length log) 3))
        (accept-process-output proc 0.05)
        (setq i (1+ i))))
    (list (process-status proc)
          (process-exit-status proc)
          (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_execute_kbd_macro_command_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("abc" nil nil "" [] [])
    // Neomacs:   OK ("abc" nil nil "c" [99] [97 98 99])
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((executing-kbd-macro nil)
        (last-kbd-macro nil))
    (execute-kbd-macro (kbd "a b c"))
    (list (buffer-string)
          last-kbd-macro
          executing-kbd-macro
          (this-command-keys)
          (this-command-keys-vector)
          (recent-keys))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_call_last_kbd_macro_from_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("xy" "xy" nil "")
    // Neomacs:   ERR (error "No keyboard macro has been defined")
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((executing-kbd-macro nil)
        (last-kbd-macro nil))
    (setq last-kbd-macro (kbd "x y"))
    (call-last-kbd-macro nil)
    (list (buffer-string)
          last-kbd-macro
          executing-kbd-macro
          (this-command-keys))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_help_window_return_message_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK "Type C-x 1 to delete the help window, C-M-v to scroll help.\n"
    // Neomacs:   OK #("Type C-x 1 to delete the help window, ESC C-v to scroll help.\n" ...)
    assert_oracle_parity(
        r##"
(let ((message-log-max t)
      (help-window-select nil))
  (with-current-buffer (get-buffer-create "*Messages*")
    (let ((inhibit-read-only t))
      (erase-buffer)))
  (with-help-window "*probe-help-msg*"
    (princ "help"))
  (prog1 (with-current-buffer "*Messages*" (buffer-string))
    (when (get-buffer "*probe-help-msg*")
      (kill-buffer "*probe-help-msg*"))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_help_window_selected_message_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK "Type q to delete help window, C-v to scroll help.\n"
    // Neomacs:   OK #("Type q to delete help window, C-v to scroll help.\n" ...)
    assert_oracle_parity(
        r##"
(let ((message-log-max t)
      (help-window-select t))
  (with-current-buffer (get-buffer-create "*Messages*")
    (let ((inhibit-read-only t))
      (erase-buffer)))
  (with-help-window "*probe-help-msg2*"
    (princ "help"))
  (prog1 (with-current-buffer "*Messages*" (buffer-string))
    (when (get-buffer "*probe-help-msg2*")
      (kill-buffer "*probe-help-msg2*"))
    (delete-other-windows)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_substitute_command_keys_meta_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (... #("Press ‘C-h’ then ‘C-M-v’" ...) [134217750] "C-M-v")
    // Neomacs:   OK (... #("Press ‘C-h’ then ‘ESC C-v’" ...) [27 22] "ESC C-v")
    assert_oracle_parity(
        r##"
(list (substitute-command-keys "\\[keyboard-quit]")
      (substitute-command-keys "Press `\\[help-command]' then `\\[scroll-other-window]'")
      (where-is-internal 'scroll-other-window nil t)
      (key-description (where-is-internal 'scroll-other-window nil t)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_help_key_description_escape_meta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (#("C-M-v" ...) #("C-M-v" ...) #("C-x 1" ...) #("q" ...))
    // Neomacs:   OK (#("C-M-v" ...) #("ESC C-v" ...) #("C-x 1" ...) #("q" ...))
    assert_oracle_parity(
        r##"
(list (help-key-description (kbd "C-M-v") nil)
      (help-key-description (kbd "ESC C-v") nil)
      (help-key-description (kbd "C-x 1") nil)
      (help-key-description (kbd "q") nil))
"##,
    );
}

#[test]
fn div_core_divergence_surface_escape_meta_key_description_canonicalization() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("M-a" "M-a" "C-M-a" "C-M-a" "M-a" "C-M-a" (meta) 97)
    // Neomacs:   OK ("M-a" "ESC a" "C-M-a" "ESC C-a" "M-a" "C-M-a" (meta) 97)
    assert_oracle_parity(
        r##"
(list (key-description (kbd "M-a"))
      (key-description (kbd "ESC a"))
      (key-description (kbd "C-M-a"))
      (key-description (kbd "ESC C-a"))
      (single-key-description ?\M-a)
      (single-key-description ?\C-\M-a)
      (event-modifiers ?\M-a)
      (event-basic-type ?\M-a))
"##,
    );
}

#[test]
fn div_core_divergence_surface_meta_command_lookup_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ([134217825] [27 97] [134217729] [27 1] [134217848] "M-x")
    // Neomacs:   OK ([134217825] [27 97] [134217729] [27 1] [27 120] "ESC x")
    assert_oracle_parity(
        r##"
(list (read-kbd-macro "M-a" nil)
      (read-kbd-macro "ESC a" nil)
      (read-kbd-macro "C-M-a" nil)
      (read-kbd-macro "ESC C-a" nil)
      (where-is-internal 'execute-extended-command nil t)
      (key-description (where-is-internal 'execute-extended-command nil t)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_manual_escape_vector_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("M-x" "M-x" "ESC ESC" "M-ESC" "M-[ A" "M-x" (meta) 120)
    // Neomacs:   OK ("ESC x" "M-x" "ESC ESC" "M-ESC" "ESC [ A" "M-x" (meta) 120)
    assert_oracle_parity(
        r##"
(list (key-description [27 120])
      (key-description [134217848])
      (key-description [27 27])
      (key-description [134217755])
      (key-description [27 91 65])
      (single-key-description 134217848)
      (event-modifiers 134217848)
      (event-basic-type 134217848))
"##,
    );
}

#[test]
fn div_core_divergence_surface_sparse_keymap_meta_where_is() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (mx mx [134217848] "M-x" [134217849] "M-y")
    // Neomacs:   OK (mx mx [27 120] "ESC x" [27 121] "ESC y")
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap)))
  (define-key map (kbd "M-x") 'mx)
  (define-key map (kbd "ESC y") 'escy)
  (list (lookup-key map (kbd "M-x"))
        (lookup-key map (kbd "ESC x"))
        (where-is-internal 'mx map t)
        (key-description (where-is-internal 'mx map t))
        (where-is-internal 'escy map t)
        (key-description (where-is-internal 'escy map t))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_substitute_command_keys_escape_meta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (#("M-x" ...) #("M-ESC ESC" ...) [134217755 27] "M-ESC ESC")
    // Neomacs:   OK (#("ESC x" ...) #("ESC ESC ESC" ...) [27 27 27] "ESC ESC ESC")
    assert_oracle_parity(
        r##"
(list (substitute-command-keys "\\[execute-extended-command]")
      (substitute-command-keys "\\[keyboard-escape-quit]")
      (where-is-internal 'keyboard-escape-quit nil t)
      (key-description (where-is-internal 'keyboard-escape-quit nil t)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_file_name_handler_operation_args_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: logs expand-file-name before handled insert-file-contents,
    // write-region, and directory-files, and passes full GNU operation args.
    // Neomacs:   skips several expand-file-name handler dispatches and passes
    // truncated insert-file-contents/write-region/directory-files arg lists.
    assert_oracle_parity(
        r##"
(let ((log nil)
      (file-name-handler-alist nil))
  (letrec ((handler
            (lambda (op &rest args)
              (push (list op args) log)
              (cond
               ((eq op 'file-exists-p) 'exists)
               ((eq op 'insert-file-contents)
                (insert "HANDLED")
                (list (car args) 7))
               ((eq op 'write-region)
                (push (list 'write-data (nth 0 args) (nth 1 args) (nth 2 args))
                      log)
                nil)
               ((eq op 'directory-files)
                '("." ".." "alpha"))
               (t
                (let ((inhibit-file-name-handlers
                       (cons handler inhibit-file-name-handlers))
                      (inhibit-file-name-operation op))
                  (apply op args)))))))
    (setq file-name-handler-alist `(("\\`/probe:" . ,handler)))
    (list (file-exists-p "/probe:/x")
          (with-temp-buffer
            (insert-file-contents "/probe:/x")
            (buffer-string))
          (with-temp-buffer
            (insert "abc")
            (write-region 1 3 "/probe:/out" nil 'silent))
          (directory-files "/probe:/dir" nil "a" t)
          (nreverse log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_thread_join_error_delivery() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (nil nil (arith-error "bad"))
    // Neomacs:   OK ((join-error arith-error ("bad")) nil (arith-error "bad"))
    assert_oracle_parity(
        r##"
(let ((th (make-thread
           (lambda () (signal 'arith-error '("bad")))
           "probe-thread-error")))
  (let ((join-result
         (condition-case err
             (thread-join th)
           (error (list 'join-error (car err) (cdr err))))))
    (list join-result
          (thread-live-p th)
          (thread-last-error th))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_network_client_open_delete_sentinels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (listen closed ((server-sentinel "open from 127.0.0.1\n" open)
    //                               (client-sentinel "deleted\n" closed)))
    // Neomacs:   OK (listen closed ((client-sentinel "open\n" open)
    //                               (server-sentinel "open from 127.0.0.1\n" open)))
    assert_oracle_parity(
        r##"
(let ((log nil)
      server
      client)
  (unwind-protect
      (progn
        (setq server
              (make-network-process
               :name "probe-net-sentinel-server"
               :server t
               :host 'local
               :service t
               :noquery t
               :sentinel (lambda (p e)
                           (push (list 'server-sentinel
                                       e
                                       (process-status p))
                                 log))))
        (setq client
              (make-network-process
               :name "probe-net-sentinel-client"
               :host 'local
               :service (process-contact server :service)
               :noquery t
               :sentinel (lambda (p e)
                           (push (list 'client-sentinel
                                       e
                                       (process-status p))
                                 log))))
        (let ((i 0))
          (while (and (< i 10) (null log))
            (accept-process-output nil 0.05)
            (setq i (1+ i))))
        (delete-process client)
        (let ((i 0))
          (while (and (< i 20) (< (length log) 2))
            (accept-process-output nil 0.05)
            (setq i (1+ i))))
        (list (process-status server)
              (process-status client)
              (nreverse log)))
    (when (processp client) (delete-process client))
    (when (processp server) (delete-process server))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_load_history_defvar_recording() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: loaded file entry records (provide . feature),
    //            (defun . function), and the defvar symbol.
    // Neomacs:   loaded file entry records provide/defun but omits defvar.
    assert_oracle_parity(
        r##"
(let* ((contents
        ";;; -*- lexical-binding: t -*-
(provide 'probe-load-history-feature)
(defun probe-load-history-fn () 42)
(defvar probe-load-history-var 9)
")
       (file (make-temp-file "neo-load-history" nil ".el" contents))
       (load-history nil))
  (unwind-protect
      (progn
        (load file nil t nil t)
        (let ((entry (cdr (assoc file load-history))))
          (list (featurep 'probe-load-history-feature)
                (probe-load-history-fn)
                probe-load-history-var
                entry
                (memq 'probe-load-history-var entry))))
    (when (get-file-buffer file)
      (kill-buffer (get-file-buffer file)))
    (delete-file file)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_case_table_search_and_conversion_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (120 121 "xAx" "xay" 0 0 2 121 121)
    // Neomacs:   OK (88 121 "XAY" "xay" 0 0 4 121 121)
    assert_oracle_parity(
        r##"
(let ((table (copy-case-table (standard-case-table))))
  (set-case-syntax-pair ?x ?y table)
  (with-temp-buffer
    (set-case-table table)
    (insert "x y X Y")
    (let ((case-fold-search t))
      (list (upcase ?x)
            (downcase ?y)
            (upcase "xay")
            (downcase "XAY")
            (string-match-p "x" "y")
            (string-match-p "y" "x")
            (progn
              (goto-char 1)
              (search-forward "y" nil t))
            (aref (current-case-table) ?x)
            (aref (current-case-table) ?y)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_visited_file_modtime_set_clear_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (((1 2 3 4000) nil) 0 t nil)
    // Neomacs:   OK (((0 1 0 0) nil) (0 1 0 0) nil nil)
    assert_oracle_parity(
        r##"
(let ((file (make-temp-file "neo-visit-explicit" nil ".txt" "abc")))
  (unwind-protect
      (let ((buf (find-file-noselect file)))
        (with-current-buffer buf
          (set-visited-file-modtime '(1 2 3 4000))
          (let ((explicit (list (visited-file-modtime)
                                (verify-visited-file-modtime buf))))
            (clear-visited-file-modtime)
            (list explicit
                  (visited-file-modtime)
                  (verify-visited-file-modtime buf)
                  (buffer-modified-p)))))
    (when (get-file-buffer file)
      (kill-buffer (get-file-buffer file)))
    (delete-file file)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_set_visited_file_name_update_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (0 0 nil ((update 0 0))) ; normalized temp-name matches
    // Neomacs:   OK (0 0 nil nil)
    assert_oracle_parity(
        r##"
(let ((log nil)
      (file-a (make-temp-file "neo-set-vfn-a" nil ".txt" "a"))
      (file-b (make-temp-file "neo-set-vfn-b" nil ".txt" "b")))
  (unwind-protect
      (let ((buf (find-file-noselect file-a)))
        (with-current-buffer buf
          (let ((buffer-list-update-hook
                 (list (lambda ()
                         (push (list 'update
                                     (file-name-nondirectory
                                      (or buffer-file-name ""))
                                     (buffer-name))
                               log)))))
            (set-visited-file-name file-b nil t)
            (list (string-match-p
                   "\\`neo-set-vfn-b.*\\.txt\\'"
                   (file-name-nondirectory buffer-file-name))
                  (string-match-p "\\`neo-set-vfn-b.*\\.txt\\'" (buffer-name))
                  (buffer-modified-p)
                  (mapcar (lambda (entry)
                            (list (car entry)
                                  (string-match-p
                                   "\\`neo-set-vfn-b.*\\.txt\\'"
                                   (cadr entry))
                                  (string-match-p
                                   "\\`neo-set-vfn-b.*\\.txt\\'"
                                   (caddr entry))))
                          (nreverse log))))))
    (when (get-file-buffer file-a)
      (kill-buffer (get-file-buffer file-a)))
    (when (get-file-buffer file-b)
      (kill-buffer (get-file-buffer file-b)))
    (delete-file file-a)
    (delete-file file-b)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_keymap_parent_accessible_keymaps_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (parent-cmd [3 112] ([] [3] [3]))
    // Neomacs:   OK (parent-cmd [3 112] ([] [3]))
    assert_oracle_parity(
        r##"
(let ((parent (make-sparse-keymap))
      (child (make-sparse-keymap)))
  (define-key parent (kbd "C-c p") 'parent-cmd)
  (define-key child (kbd "C-c c") 'child-cmd)
  (set-keymap-parent child parent)
  (list (lookup-key child (kbd "C-c p"))
        (where-is-internal 'parent-cmd child t)
        (mapcar #'car (accessible-keymaps child))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_keymap_remap_where_is_parent_child_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (child-new nil nil child-new)
    // Neomacs:   OK (child-new [remap old] [remap old] child-new)
    assert_oracle_parity(
        r##"
(let ((parent (make-sparse-keymap))
      (child (make-sparse-keymap)))
  (define-key parent [remap old] 'parent-new)
  (define-key child [remap old] 'child-new)
  (set-keymap-parent child parent)
  (list (command-remapping 'old nil (list child))
        (where-is-internal 'parent-new child t)
        (where-is-internal 'child-new child t)
        (lookup-key child [remap old])))
"##,
    );
}

#[test]
fn div_core_divergence_surface_mutex_lock_blocks_other_thread() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((t (start)) done nil (got start) "probe-mutex-block")
    // Neomacs:   OK ((nil (got start)) done nil (got start) "probe-mutex-block")
    assert_oracle_parity(
        r##"
(let ((m (make-mutex "probe-mutex-block"))
      (log nil))
  (mutex-lock m)
  (let ((th (make-thread
             (lambda ()
               (push 'start log)
               (mutex-lock m)
               (push 'got log)
               'done)
             "mutex-wait")))
    (let ((i 0))
      (while (and (< i 20) (null log))
        (sleep-for 0.01)
        (setq i (1+ i))))
    (let ((before (list (thread-live-p th) log)))
      (mutex-unlock m)
      (let ((res (thread-join th)))
        (list before
              res
              (thread-live-p th)
              log
              (mutex-name m))))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_thread_dynamic_binding_isolation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (global "*scratch*" local)
    // Neomacs:   OK (main "*scratch*" local)
    assert_oracle_parity(
        r##"
(progn
  (defvar probe-thread-dyn 'global)
  (let ((buf (get-buffer-create " *probe-thread-buf*"))
        (probe-thread-dyn 'main))
    (unwind-protect
        (progn
          (with-current-buffer buf
            (setq-local probe-thread-dyn 'local))
          (let ((th
                 (make-thread
                  (lambda ()
                    (list probe-thread-dyn
                          (buffer-name)
                          (with-current-buffer buf probe-thread-dyn)))
                  "dyn-thread")))
            (thread-join th)))
      (when (buffer-live-p buf)
        (kill-buffer buf)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_all_threads_includes_live_worker() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((nil "probe-list-thread") (nil) (run))
    // Neomacs:   OK ((nil) (nil) (run))
    assert_oracle_parity(
        r##"
(let ((log nil))
  (let ((th (make-thread
             (lambda ()
               (push 'run log)
               (sleep-for 0.1)
               'done)
             "probe-list-thread")))
    (let ((names-while-live (mapcar #'thread-name (all-threads))))
      (thread-join th)
      (list names-while-live
            (mapcar #'thread-name (all-threads))
            log))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_load_history_defcustom_recording() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ((require defface probe-lh-custom defun provide) (probe-lh-custom ...))
    // Neomacs:   OK ((require defface defun provide) nil)
    assert_oracle_parity(
        r##"
(let* ((contents
        ";;; -*- lexical-binding: t -*-
(require 'custom)
(defgroup probe-lh nil \"\" :group 'emacs)
(defface probe-lh-face '((t (:weight bold))) \"\")
(defcustom probe-lh-custom 3 \"\" :type 'integer :group 'probe-lh)
(defalias 'probe-lh-alias 'ignore)
(provide 'probe-lh)
")
       (file (make-temp-file "neo-loadhist-custom" nil ".el" contents))
       (load-history nil))
  (unwind-protect
      (progn
        (load file nil t nil t)
        (let ((entry (cdr (assoc file load-history))))
          (list (mapcar (lambda (x)
                          (if (consp x) (car x) x))
                        entry)
                (memq 'probe-lh-custom entry)
                (assq 'defface entry)
                entry)))
    (delete-file file)))
"##,
    );
}

#[test]
fn div_core_divergence_surface_case_table_word_casing_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK ("[ello World" "[abc Def" "[abc" "[hello]" "{foo} Bar" "{foo Bar}" t)
    // Neomacs:   OK ("[Ello World" "[Abc Def" "]Abc" "[Hello]" "{Foo} Bar" "{Foo Bar}" nil)
    assert_oracle_parity(
        r##"
(let ((bracket-table (copy-case-table (standard-case-table)))
      (brace-table (copy-case-table (standard-case-table))))
  (set-case-syntax-pair ?\[ ?\] bracket-table)
  (set-case-syntax-pair ?\{ ?\} brace-table)
  (list
   (with-temp-buffer
     (set-case-table bracket-table)
     (capitalize "[ello world"))
   (with-temp-buffer
     (set-case-table bracket-table)
     (upcase-initials "[abc def"))
   (with-temp-buffer
     (set-case-table bracket-table)
     (capitalize "]abc"))
   (with-temp-buffer
     (set-case-table bracket-table)
     (insert "[hello]")
     (capitalize-region (point-min) (point-max))
     (buffer-string))
   (with-temp-buffer
     (set-case-table brace-table)
     (insert "{foo} bar")
     (goto-char (point-min))
     (capitalize-word 2)
     (buffer-string))
   (with-temp-buffer
     (set-case-table brace-table)
     (insert "{foo bar}")
     (upcase-initials-region (point-min) (point-max))
     (buffer-string))
   (with-temp-buffer
     (set-case-table brace-table)
     (char-equal ?\{ ?\}))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_process_attributes_running_child_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (t "/bin/sh -c sleep\\ 0.2" "")
    // Neomacs:   OK (nil "/bin/sh -c sleep 0.2" "pipe:[...]")
    assert_oracle_parity(
        r##"
(let ((proc (make-process
             :name "probe-attrs-child"
             :command '("/bin/sh" "-c" "sleep 0.2")
             :connection-type 'pipe)))
  (unwind-protect
      (let* ((attrs (process-attributes (process-id proc)))
             (args (cdr (assq 'args attrs)))
             (ttname (cdr (assq 'ttname attrs))))
        (list (process-running-child-p proc)
              args
              (if (and (stringp ttname)
                       (string-match-p "\\`pipe:" ttname))
                  "pipe:[...]"
                ttname)))
    (when (process-live-p proc)
      (delete-process proc))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_unibyte_search_raw_byte_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (nil nil 4 (3 3 3 1 169)) ; raw-byte search-forward finds
    //            the 0xA9 byte at position 2 (point 3) inside the é sequence.
    // Neomacs:   OK (nil nil 4 (5 3 3 1 169)) ; search-forward skips the byte
    //            embedded in é and only matches the standalone trailing byte.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 195 169 65 169))
  (let ((pat (unibyte-string 169)))
    (list enable-multibyte-characters
          (multibyte-string-p (buffer-string))
          (string-bytes (buffer-string))
          (list (progn
                  (goto-char (point-min))
                  (search-forward pat nil t))
                (progn
                  (goto-char (point-min))
                  (re-search-forward pat nil t))
                (progn
                  (goto-char (point-min))
                  (skip-chars-forward (unibyte-string 195 169))
                  (point))
                (string-match pat (buffer-string))
                (char-after 4)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_unibyte_multibyte_search_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (3 nil nil nil) ; a multibyte char never matches in a
    //            unibyte buffer, so search-forward of (string ?é) returns nil.
    // Neomacs:   OK (3 3 nil nil)   ; search-forward incorrectly matches the
    //            multibyte char against the raw é byte sequence.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 195 169 65 169))
  (let ((raw-pat (unibyte-string 195 169)))
    (list (progn
            (goto-char (point-min))
            (search-forward raw-pat nil t))
          (progn
            (goto-char (point-min))
            (search-forward (string ?é) nil t))
          (progn
            (goto-char (point-min))
            (re-search-forward (string ?é) nil t))
          (string-match (string ?é) (buffer-string)))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_signal_process_signal_name_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (0 signal 15 (("terminated\n" signal 15)))
    // Neomacs:   OK ((err "Undefined signal name TERM") run 0 nil)
    // GNU accepts signal-name symbols (TERM) for signal-process; Neomacs only
    // accepts integer signal numbers and errors on the symbol, leaving the
    // process running with no sentinel event.
    assert_oracle_parity(
        r##"
(let ((proc (make-process
             :name "probe-signal-name"
             :command '("/bin/sh" "-c" "sleep 5")
             :connection-type 'pipe)))
  (set-process-query-on-exit-flag proc nil)
  (let ((log nil))
    (set-process-sentinel
     proc
     (lambda (p e)
       (push (list e (process-status p) (process-exit-status p)) log)))
    (let ((ret (condition-case err
                   (signal-process proc 'TERM)
                 (error (list 'err (cadr err))))))
      (let ((i 0))
        (while (and (< i 40) (process-live-p proc))
          (accept-process-output proc 0.05)
          (setq i (1+ i))))
      (let ((j 0))
        (while (and (< j 20) (null log))
          (accept-process-output proc 0.05)
          (setq j (1+ j))))
      (prog1 (list ret
                   (process-status proc)
                   (process-exit-status proc)
                   (nreverse log))
        (when (process-live-p proc)
          (delete-process proc))))))
"##,
    );
}

#[test]
fn div_core_divergence_surface_insert_and_inherit_full_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-24:
    // GNU Emacs: OK (#("AXB" 0 3 (face bold rear-nonsticky nil))
    //                (face bold rear-nonsticky nil) (face bold rear-nonsticky nil))
    // Neomacs:   OK (#("AXB" 0 1 (face bold rear-nonsticky nil) 1 2 (face bold)
    //                2 3 (face bold rear-nonsticky nil)) (face bold)
    //                (face bold rear-nonsticky nil))
    // GNU inherits the full property plist (including rear-nonsticky nil) for
    // the inserted char and coalesces the spans; Neomacs inherits only `face`
    // and leaves a fragmented interval with a partial plist.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert (propertize "AB" 'face 'bold 'rear-nonsticky nil))
  (goto-char 2)
  (insert-and-inherit "X")
  (list (buffer-string)
        (text-properties-at 2)
        (text-properties-at 3)))
"##,
    );
}
