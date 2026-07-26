use super::assert_ace_jump_mode_parity;
use expect_test::expect;

#[test]
fn ace_jump_mode_move_to_end_partitions_stably_for_mixed_matches() {
    let elisp_form = r##"(list
         (ace-jump-move-to-end-if
          '(1 2 3 4 5 6)
          (lambda (value)
            (= 0 (% value 2))))
         (ace-jump-move-to-end-if
          '(a b a c a)
          (lambda (value)
            (eq value 'a))))"##;
    let expect = expect!["OK ((1 3 5 2 4 6) (b c a a a))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_to_end_handles_empty_none_and_all_matches() {
    let elisp_form = r##"(list
         (ace-jump-move-to-end-if
          nil
          (lambda (_value) t))
         (ace-jump-move-to-end-if
          '(a b c)
          (lambda (_value) nil))
         (ace-jump-move-to-end-if
          '(a b c)
          (lambda (_value) t)))"##;
    let expect = expect!["OK (nil (a b c) (a b c))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_to_end_calls_predicate_once_per_element_in_order() {
    let elisp_form = r##"(let (observed)
         (let ((result
                (ace-jump-move-to-end-if
                 '(3 1 4 1 5)
                 (lambda (value)
                   (setq observed
                         (cons value observed))
                   (> value 2)))))
           (list
            result
            (nreverse observed))))"##;
    let expect = expect!["OK ((1 1 3 4 5) (3 1 4 1 5))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_first_to_end_moves_only_first_matching_element() {
    let elisp_form = r##"(list
         (ace-jump-move-first-to-end-if
          '(a b a c a)
          (lambda (value)
            (eq value 'a)))
         (ace-jump-move-first-to-end-if
          '(1 2 4 6 3)
          (lambda (value)
            (= 0 (% value 2)))))"##;
    let expect = expect!["OK ((b a c a a) (1 4 6 3 2))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_move_first_to_end_stops_calling_user_predicate_after_match() {
    let elisp_form = r##"(let (observed)
         (let ((result
                (ace-jump-move-first-to-end-if
                 '(1 2 3 4 5)
                 (lambda (value)
                   (setq observed
                         (cons value observed))
                   (= value 3)))))
           (list
            result
            (nreverse observed))))"##;
    let expect = expect!["OK ((1 2 4 5 3) (1 2 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_enable_and_disable_mark_sync_toggle_exact_advice_state() {
    let elisp_form = r##"(unwind-protect
         (progn
           (ace-jump-mode-disable-mark-sync)
           (let ((disabled
                  (list
                   ace-jump-sync-emacs-mark-ring
                   (ad-advice-enabled
                    (ad-find-advice
                     'pop-mark
                     'before
                     'ace-jump-pop-mark-advice))
                   (ad-advice-enabled
                    (ad-find-advice
                     'pop-global-mark
                     'before
                     'ace-jump-pop-global-mark-advice)))))
             (ace-jump-mode-enable-mark-sync)
             (list
              disabled
              ace-jump-sync-emacs-mark-ring
              (ad-advice-enabled
               (ad-find-advice
                'pop-mark
                'before
                'ace-jump-pop-mark-advice))
              (ad-advice-enabled
               (ad-find-advice
                'pop-global-mark
                'before
                'ace-jump-pop-global-mark-advice)))))
       (ace-jump-mode-disable-mark-sync))"##;
    let expect = expect!["OK ((nil nil nil) t t t)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_mark_sync_toggles_are_idempotent() {
    let elisp_form = r##"(unwind-protect
         (progn
           (ace-jump-mode-enable-mark-sync)
           (ace-jump-mode-enable-mark-sync)
           (let ((enabled
                  (list
                   ace-jump-sync-emacs-mark-ring
                   (ad-advice-enabled
                    (ad-find-advice
                     'pop-mark
                     'before
                     'ace-jump-pop-mark-advice)))))
             (ace-jump-mode-disable-mark-sync)
             (ace-jump-mode-disable-mark-sync)
             (list
              enabled
              ace-jump-sync-emacs-mark-ring
              (ad-advice-enabled
               (ad-find-advice
                'pop-mark
                'before
                'ace-jump-pop-mark-advice)))))
       (ace-jump-mode-disable-mark-sync))"##;
    let expect = expect!["OK ((t t) nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_enabled_pop_mark_advice_rotates_matching_ace_position() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefgh")
         (let* ((area
                 (make-aj-visual-area
                  :buffer (current-buffer)
                  :window (selected-window)
                  :frame (selected-frame)))
                (ace-jump-mode-mark-ring
                 (mapcar
                  (lambda (offset)
                    (make-aj-position
                     :offset offset
                     :visual-area area))
                  '(3 6)))
                (mark-ring
                 (list
                  (copy-marker 5))))
           (goto-char 2)
           (set-marker
            (mark-marker)
            3
            (current-buffer))
           (unwind-protect
               (progn
                 (ace-jump-mode-enable-mark-sync)
                 (pop-mark)
                 (list
                  (point)
                  (marker-position
                   (mark-marker))
                  (mapcar
                   #'aj-position-offset
                   ace-jump-mode-mark-ring)))
             (ace-jump-mode-disable-mark-sync))))"##;
    let expect = expect!["OK (2 5 (6 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_disabled_pop_mark_advice_leaves_ace_ring_order_unchanged() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefgh")
         (let* ((area
                 (make-aj-visual-area
                  :buffer (current-buffer)
                  :window (selected-window)
                  :frame (selected-frame)))
                (ace-jump-mode-mark-ring
                 (mapcar
                  (lambda (offset)
                    (make-aj-position
                     :offset offset
                     :visual-area area))
                  '(3 6)))
                (mark-ring
                 (list
                  (copy-marker 5))))
           (goto-char 2)
           (set-marker
            (mark-marker)
            3
            (current-buffer))
           (ace-jump-mode-disable-mark-sync)
           (pop-mark)
           (list
            (point)
            (marker-position
             (mark-marker))
            (mapcar
             #'aj-position-offset
             ace-jump-mode-mark-ring))))"##;
    let expect = expect!["OK (2 5 (3 6))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_enabled_pop_global_mark_advice_skips_dead_marker_and_stably_moves_matching_buffers()
 {
    let elisp_form = r##"(let ((target
              (generate-new-buffer
               " *ace-jump-global-advice-target*"))
             (other
              (generate-new-buffer
               " *ace-jump-global-advice-other*")))
         (unwind-protect
             (save-window-excursion
               (with-current-buffer target
                 (insert "target"))
               (with-current-buffer other
                 (insert "other"))
               (let* ((target-area
                       (make-aj-visual-area
                        :buffer target
                        :window (selected-window)
                        :frame (selected-frame)))
                      (other-area
                       (make-aj-visual-area
                        :buffer other
                        :window (selected-window)
                        :frame (selected-frame)))
                      (ace-jump-mode-mark-ring
                       (list
                        (make-aj-position
                         :offset 1
                         :visual-area target-area)
                        (make-aj-position
                         :offset 2
                         :visual-area other-area)
                        (make-aj-position
                         :offset 3
                         :visual-area target-area)))
                      (global-mark-ring
                       (list
                        (make-marker)
                        (set-marker
                         (make-marker)
                         1
                         target)
                        (set-marker
                         (make-marker)
                         2
                         other))))
                 (unwind-protect
                     (progn
                       (ace-jump-mode-enable-mark-sync)
                       (pop-global-mark)
                       (mapcar
                        (lambda (position)
                          (list
                           (if
                               (eq
                                (aj-position-buffer
                                 position)
                                target)
                               'target
                             'other)
                           (aj-position-offset
                            position)))
                        ace-jump-mode-mark-ring))
                   (ace-jump-mode-disable-mark-sync))))
           (kill-buffer target)
           (kill-buffer other)))"##;
    let expect = expect!["OK ((other 2) (target 1) (target 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_disabled_pop_global_mark_advice_leaves_ace_ring_order_unchanged() {
    let elisp_form = r##"(let ((target
              (generate-new-buffer
               " *ace-jump-global-advice-target*"))
             (other
              (generate-new-buffer
               " *ace-jump-global-advice-other*")))
         (unwind-protect
             (save-window-excursion
               (with-current-buffer target
                 (insert "target"))
               (with-current-buffer other
                 (insert "other"))
               (let* ((target-area
                       (make-aj-visual-area
                        :buffer target
                        :window (selected-window)
                        :frame (selected-frame)))
                      (other-area
                       (make-aj-visual-area
                        :buffer other
                        :window (selected-window)
                        :frame (selected-frame)))
                      (ace-jump-mode-mark-ring
                       (list
                        (make-aj-position
                         :offset 1
                         :visual-area target-area)
                        (make-aj-position
                         :offset 2
                         :visual-area other-area)
                        (make-aj-position
                         :offset 3
                         :visual-area target-area)))
                      (global-mark-ring
                       (list
                        (make-marker)
                        (set-marker
                         (make-marker)
                         1
                         target)
                        (set-marker
                         (make-marker)
                         2
                         other))))
                 (ace-jump-mode-disable-mark-sync)
                 (pop-global-mark)
                 (mapcar
                  (lambda (position)
                    (list
                     (if
                         (eq
                          (aj-position-buffer position)
                          target)
                         'target
                       'other)
                     (aj-position-offset position)))
                  ace-jump-mode-mark-ring)))
           (kill-buffer target)
           (kill-buffer other)))"##;
    let expect = expect!["OK ((target 1) (other 2) (target 3))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}
