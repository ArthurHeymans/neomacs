use super::{assert_ace_jump_mode_parity, assert_ace_jump_mode_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_mode_push_mark_records_complete_current_visual_position() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-mark*"))
             (ace-jump-mode-mark-ring nil)
             (ace-jump-mode-mark-ring-max 100))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcdef")
                 (goto-char 4)
                 (ace-jump-push-mark)
                 (let ((position
                        (car ace-jump-mode-mark-ring)))
                   (list
                    (length ace-jump-mode-mark-ring)
                    (aj-position-offset position)
                    (eq
                     (aj-position-buffer position)
                     buffer)
                    (eq
                     (aj-position-window position)
                     (selected-window))
                    (eq
                     (aj-position-frame position)
                     (selected-frame))
                    (aj-position-recover-buffer
                     position)
                    (marker-position
                     (mark-marker))))))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (1 4 t t t nil 4)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_push_mark_caps_ring_at_configured_maximum() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-jump-mark-cap*"))
             (ace-jump-mode-mark-ring nil)
             (ace-jump-mode-mark-ring-max 2))
         (unwind-protect
             (save-window-excursion
               (set-window-buffer
                (selected-window)
                buffer)
               (with-current-buffer buffer
                 (insert "abcdef")
                 (mapc
                  (lambda (offset)
                    (goto-char offset)
                    (ace-jump-push-mark))
                  '(2 3 4 5))
                 (mapcar
                  #'aj-position-offset
                  ace-jump-mode-mark-ring)))
           (kill-buffer buffer)))"##;
    let expect = expect!["OK (5 4)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_discards_dead_buffers_then_rotates_live_entry() {
    let elisp_form = r##"(let* ((dead-buffer
                (generate-new-buffer
                 " *ace-jump-dead*"))
               (live-buffer
                (current-buffer))
               (area-dead
                (make-aj-visual-area
                 :buffer dead-buffer
                 :window (selected-window)
                 :frame (selected-frame)))
               (area-live
                (make-aj-visual-area
                 :buffer live-buffer
                 :window (selected-window)
                 :frame (selected-frame)))
               (dead
                (make-aj-position
                 :offset 2
                 :visual-area area-dead))
               (live
                (make-aj-position
                 :offset 3
                 :visual-area area-live))
               (ace-jump-mode-mark-ring
                (list dead live))
               (ace-jump-sync-emacs-mark-ring nil)
               jumped)
         (kill-buffer dead-buffer)
         (cl-letf (((symbol-function 'ace-jump-jump-to)
                    (lambda (position)
                      (setq jumped position))))
           (ace-jump-mode-pop-mark))
         (list
          (eq jumped live)
          (mapcar
           #'aj-position-offset
           ace-jump-mode-mark-ring)))"##;
    let expect = expect!["OK (t (3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_rotates_multiple_live_entries() {
    let elisp_form = r##"(let* ((area
                (make-aj-visual-area
                 :buffer (current-buffer)
                 :window (selected-window)
                 :frame (selected-frame)))
               (positions
                (mapcar
                 (lambda (offset)
                   (make-aj-position
                    :offset offset
                    :visual-area area))
                 '(2 3 4)))
               (ace-jump-mode-mark-ring
                (copy-sequence positions))
               (ace-jump-sync-emacs-mark-ring nil)
               jumped)
         (cl-letf (((symbol-function 'ace-jump-jump-to)
                    (lambda (position)
                      (setq jumped
                            (cons
                             (aj-position-offset position)
                             jumped)))))
           (ace-jump-mode-pop-mark)
           (ace-jump-mode-pop-mark))
         (list
          (nreverse jumped)
          (mapcar
           #'aj-position-offset
           ace-jump-mode-mark-ring)))"##;
    let expect = expect!["OK ((2 3) (4 2 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_syncs_equal_local_mark_with_mark_ring_rotation() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let* ((area
                 (make-aj-visual-area
                  :buffer (current-buffer)
                  :window (selected-window)
                  :frame (selected-frame)))
                (position
                 (make-aj-position
                  :offset 3
                  :visual-area area))
                (ace-jump-mode-mark-ring
                 (list position))
                (ace-jump-sync-emacs-mark-ring t)
                (first (copy-marker 5))
                (second (copy-marker 7))
                (mark-ring
                 (list first second))
                jumped)
           (set-marker
            (mark-marker)
            3
            (current-buffer))
           (cl-letf (((symbol-function 'ace-jump-jump-to)
                      (lambda (target)
                        (setq jumped
                              (aj-position-offset target)))))
             (ace-jump-mode-pop-mark))
           (list
            jumped
            (marker-position
             (mark-marker))
            (mapcar
             #'marker-position
             mark-ring)
            (mapcar
             #'aj-position-offset
             ace-jump-mode-mark-ring)
            (marker-buffer first))))"##;
    let expect = expect!["OK (3 5 (7 3) (3) nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_syncs_nonmatching_local_target_by_moving_first_marker() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (let* ((area
                 (make-aj-visual-area
                  :buffer (current-buffer)
                  :window (selected-window)
                  :frame (selected-frame)))
                (position
                 (make-aj-position
                  :offset 5
                  :visual-area area))
                (ace-jump-mode-mark-ring
                 (list position))
                (ace-jump-sync-emacs-mark-ring t)
                (mark-ring
                 (mapcar
                  (lambda (offset)
                    (copy-marker offset))
                  '(5 7 5 8)))
                jumped)
           (set-marker
            (mark-marker)
            2
            (current-buffer))
           (cl-letf (((symbol-function 'ace-jump-jump-to)
                      (lambda (target)
                        (setq jumped
                              (aj-position-offset target)))))
             (ace-jump-mode-pop-mark))
           (list
            jumped
            (marker-position
             (mark-marker))
            (mapcar
             #'marker-position
             mark-ring)
            (mapcar
             #'aj-position-offset
             ace-jump-mode-mark-ring))))"##;
    let expect = expect!["OK (5 2 (7 5 8 5) (5))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_syncs_cross_buffer_target_with_global_ring_rotation() {
    let elisp_form = r##"(let ((target
              (generate-new-buffer
               " *ace-jump-global-target*"))
             (other
              (generate-new-buffer
               " *ace-jump-global-other*")))
         (unwind-protect
             (let* ((area
                     (make-aj-visual-area
                      :buffer target
                      :window (selected-window)
                      :frame (selected-frame)))
                    (position
                     (make-aj-position
                      :offset 1
                      :visual-area area))
                    (ace-jump-mode-mark-ring
                     (list position))
                    (ace-jump-sync-emacs-mark-ring t)
                    (global-mark-ring
                     (list
                      (set-marker
                       (make-marker)
                       1 target)
                      (set-marker
                       (make-marker)
                       1 other)
                      (set-marker
                       (make-marker)
                       2 target)))
                    jumped)
               (cl-letf (((symbol-function 'ace-jump-jump-to)
                          (lambda (candidate)
                            (setq jumped
                                  (aj-position-buffer candidate)))))
                 (ace-jump-mode-pop-mark))
               (list
                (eq jumped target)
                (mapcar
                 (lambda (marker)
                   (cond
                    ((eq
                      (marker-buffer marker)
                      target)
                     'target)
                    ((eq
                      (marker-buffer marker)
                      other)
                     'other)
                    (t nil)))
                 global-mark-ring)
                (mapcar
                 #'aj-position-offset
                 ace-jump-mode-mark-ring)))
           (kill-buffer target)
           (kill-buffer other)))"##;
    let expect = expect!["OK (t (other target target) (1))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_empty_ring_signals_exact_history_error() {
    let elisp_form = r##"(let ((ace-jump-mode-mark-ring nil))
         (ace-jump-mode-pop-mark))"##;
    let expect = expect![[r#"ERR (error "[AceJump] No more history")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_pop_mark_all_dead_ring_signals_after_pruning() {
    let elisp_form = r##"(let* ((buffer
                (generate-new-buffer
                 " *ace-jump-all-dead*"))
               (area
                (make-aj-visual-area
                 :buffer buffer
                 :window (selected-window)
                 :frame (selected-frame)))
               (ace-jump-mode-mark-ring
                (list
                 (make-aj-position
                 :offset 1
                  :visual-area area))))
         (kill-buffer buffer)
         (list
          (condition-case error-data
              (ace-jump-mode-pop-mark)
            (error error-data))
          ace-jump-mode-mark-ring))"##;
    let expect = expect![[r#"OK ((error "[AceJump] No more history") nil)"#]];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_kill_buffer_without_server_clients_uses_kill_buffer() {
    let elisp_form = r##"(let ((server-buffer-clients nil)
             calls)
         (cl-letf (((symbol-function 'kill-buffer)
                    (lambda (buffer)
                      (setq calls
                            (cons
                             (list 'kill buffer)
                             calls))
                      'killed))
                   ((symbol-function 'server-buffer-done)
                    (lambda (&rest arguments)
                      (setq calls
                            (cons
                             (cons 'server arguments)
                             calls)))))
           (list
            (ace-jump-kill-buffer 'target)
            (nreverse calls))))"##;
    let expect = expect!["OK (killed ((kill target)))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_kill_buffer_with_server_clients_notifies_server_then_kills() {
    let elisp_form = r##"(progn
         (defvar server-buffer-clients nil)
         (let ((server-buffer-clients '(client))
               calls)
           (cl-letf (((symbol-function 'kill-buffer)
                      (lambda (buffer)
                        (setq calls
                              (cons
                               (list 'kill buffer)
                               calls))
                        'killed))
                     ((symbol-function 'server-buffer-done)
                      (lambda (&rest arguments)
                        (setq calls
                              (cons
                               (cons 'server arguments)
                               calls)))))
             (list
              (ace-jump-kill-buffer 'target)
              (nreverse calls)))))"##;
    let expect = expect!["OK (killed ((server target t) (kill target)))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
