use super::assert_ace_jump_zap_parity;
use expect_test::expect;

#[test]
fn ace_jump_zap_reset_clears_all_internal_state_and_returns_nil() {
    let elisp_form = r##"(let ((ajz/zapping 'active)
             (ajz/saved-point 42)
             (ajz/to-char t))
         (list
          (ajz/reset)
          ajz/zapping
          ajz/saved-point
          ajz/to-char))"##;
    let expect = expect!["OK (nil nil nil nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_start_pushes_mark_only_during_zapping() {
    let elisp_form = r##"(mapcar
         (lambda (zapping)
           (let ((ajz/zapping zapping)
                 calls)
             (cl-letf (((symbol-function 'push-mark)
                        (lambda (&rest arguments)
                          (setq calls
                                (cons arguments calls))
                          'pushed)))
               (list
                zapping
                (ajz/maybe-zap-start)
                (nreverse calls)))))
         '(nil t active))"##;
    let expect = expect!["OK ((nil nil nil) (t pushed (nil)) (active pushed (nil)))"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_end_when_inactive_only_resets_stale_state() {
    let elisp_form = r##"(let ((ajz/zapping nil)
             (ajz/saved-point 12)
             (ajz/to-char t)
             events)
         (cl-letf (((symbol-function 'ajz/forward-query)
                    (lambda ()
                      (setq events
                            (cons 'query events))))
                   ((symbol-function 'forward-char)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'forward arguments)
                             events))))
                   ((symbol-function 'call-interactively)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'interactive arguments)
                             events))))
                   ((symbol-function 'kill-region)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'kill arguments)
                             events))))
                   ((symbol-function 'deactivate-mark)
                    (lambda ()
                      (setq events
                            (cons 'deactivate events)))))
           (ajz/maybe-zap-end))
         (list
          (nreverse events)
          ajz/zapping
          ajz/saved-point
          ajz/to-char))"##;
    let expect = expect!["OK (nil nil nil nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_end_delete_region_covers_direction_and_to_char_boundaries() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (let ((ajz/zapping t)
                 (ajz/to-char
                  (nth 1 spec))
                 (ajz/saved-point 5)
                 (ajz/zap-function
                  'delete-region)
                 events)
             (cl-letf (((symbol-function 'ajz/forward-query)
                        (lambda ()
                          (car spec)))
                       ((symbol-function 'forward-char)
                        (lambda (&rest arguments)
                          (setq events
                                (cons
                                 (cons 'forward arguments)
                                 events))))
                       ((symbol-function 'call-interactively)
                        (lambda (&rest arguments)
                          (setq events
                                (cons
                                 (cons
                                  'interactive
                                  arguments)
                                 events))))
                       ((symbol-function 'deactivate-mark)
                        (lambda ()
                          (setq events
                                (cons 'deactivate events)))))
               (ajz/maybe-zap-end))
             (list
              spec
              (nreverse events)
              ajz/zapping
              ajz/saved-point
              ajz/to-char)))
         '((t nil)
           (t t)
           (nil nil)
           (nil t)))"##;
    let expect = expect![
        "OK (((t nil) ((interactive delete-region) deactivate) nil nil nil) ((t t) ((forward) (interactive delete-region) deactivate) nil nil nil) ((nil nil) ((forward) (interactive delete-region) deactivate) nil nil nil) ((nil t) ((interactive delete-region) deactivate) nil nil nil))"
    ];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_end_kill_region_passes_point_and_mark_in_exact_order() {
    let elisp_form = r##"(let ((ajz/zapping t)
             (ajz/to-char nil)
             (ajz/saved-point 4)
             (ajz/zap-function 'kill-region)
             events)
         (cl-letf (((symbol-function 'ajz/forward-query)
                    (lambda () t))
                   ((symbol-function 'point)
                    (lambda () 11))
                   ((symbol-function 'mark)
                    (lambda (&optional force)
                      (setq events
                            (cons
                             (list 'mark force)
                             events))
                      3))
                   ((symbol-function 'kill-region)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'kill arguments)
                             events))
                      'killed))
                   ((symbol-function 'deactivate-mark)
                    (lambda ()
                      (setq events
                            (cons 'deactivate events)))))
           (ajz/maybe-zap-end))
         (list
          (nreverse events)
          ajz/zapping
          ajz/saved-point
          ajz/to-char))"##;
    let expect = expect!["OK (((mark nil) (kill 11 3) deactivate) nil nil nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_end_unknown_zap_function_still_deactivates_and_resets() {
    let elisp_form = r##"(let ((ajz/zapping t)
             (ajz/to-char t)
             (ajz/saved-point 4)
             (ajz/zap-function 'unknown)
             events)
         (cl-letf (((symbol-function 'ajz/forward-query)
                    (lambda () t))
                   ((symbol-function 'forward-char)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'forward arguments)
                             events))))
                   ((symbol-function 'deactivate-mark)
                    (lambda ()
                      (setq events
                            (cons 'deactivate events)))))
           (ajz/maybe-zap-end))
         (list
          (nreverse events)
          ajz/zapping
          ajz/saved-point
          ajz/to-char))"##;
    let expect = expect!["OK (((forward) deactivate) nil nil nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_end_forward_error_preserves_active_state_and_skips_cleanup() {
    let elisp_form = r##"(let ((ajz/zapping t)
             (ajz/to-char t)
             (ajz/saved-point 4)
             (ajz/zap-function 'delete-region)
             events)
         (cl-letf (((symbol-function 'ajz/forward-query)
                    (lambda () t))
                   ((symbol-function 'forward-char)
                    (lambda (&rest _arguments)
                      (error "cannot move")))
                   ((symbol-function 'call-interactively)
                    (lambda (&rest arguments)
                      (setq events
                            (cons
                             (cons 'interactive arguments)
                             events))))
                   ((symbol-function 'deactivate-mark)
                    (lambda ()
                      (setq events
                            (cons 'deactivate events)))))
           (list
            (condition-case error-data
                (ajz/maybe-zap-end)
              (error error-data))
            ajz/zapping
            ajz/to-char
            ajz/saved-point
            (nreverse events))))"##;
    let expect = expect![[r#"OK ((error "cannot move") t t 4 nil)"#]];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_keyboard_reset_clears_state_before_finishing_ace_jump() {
    let elisp_form = r##"(let ((ajz/zapping t)
             (ajz/to-char t)
             (ajz/saved-point 4)
             events)
         (cl-letf (((symbol-function 'ace-jump-done)
                    (lambda ()
                      (setq events
                            (cons
                             (list
                              'done
                              ajz/zapping
                              ajz/saved-point
                              ajz/to-char)
                             events))
                      'finished)))
           (list
            (ajz/keyboard-reset)
            (nreverse events)
            ajz/zapping
            ajz/saved-point
            ajz/to-char)))"##;
    let expect = expect!["OK (finished ((done nil nil nil)) nil nil nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_registered_before_hook_dispatches_real_start_handler() {
    let elisp_form = r##"(let ((ajz/zapping t)
             calls)
         (cl-letf (((symbol-function 'push-mark)
                    (lambda (&rest arguments)
                      (setq calls
                            (cons arguments calls)))))
           (run-hooks
            'ace-jump-mode-before-jump-hook))
         (nreverse calls))"##;
    let expect = expect!["OK (nil)"];
    assert_ace_jump_zap_parity(elisp_form, expect);
}
