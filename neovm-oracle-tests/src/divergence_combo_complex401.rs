//! Complex combo batch 401 — `timer`/`event-loop` ultimate: creation/
//! cancel/repeat/idle/list-query, sit-for/input-pending, with-timeout.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx401_timer_creation_and_cancel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (let ((timer (run-with-timer 0 nil (lambda () (push :fired fired)))))
    (sit-for 0.01)
    (cancel-timer timer)
    (list (nreverse fired))))
"##,
    )
}

#[test]
fn div_cx401_repeat_timer_fires_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (let ((timer (run-with-timer 0 0.001 (lambda () (push :tick fired)))))
    (sit-for 0.02)
    (cancel-timer timer))
  (list (>= (length fired) 1) (nreverse fired)))
"##,
    )
}

#[test]
fn div_cx401_idle_timer_does_not_fire_during_busy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (let ((idle (run-with-idle-timer 0.05 nil (lambda () (push :idle fired)))))
    (sit-for 0.01)
    (let ((short fired))
      (sit-for 0.1)
      (cancel-timer idle)
      (list short (nreverse fired)))))
"##,
    )
}

#[test]
fn div_cx401_timer_list_and_idle_list_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((before-tl timer-list)
      (before-idle timer-idle-list))
  (let ((t1 (run-with-timer 100 nil (lambda () :never))))
    (let ((after-add-tl (length timer-list)))
      (cancel-timer t1)
      (let ((after-cancel-tl (length timer-list)))
        (list (>= after-add-tl (1+ (length before-tl)))
              (>= (1- after-add-tl) after-cancel-tl)))))
"##,
    )
}

#[test]
fn div_cx401_timer_predicate_and_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((timer (run-with-timer 100 nil (lambda () :never))))
  (list (timerp timer)
        (timer--time timer)
        (timer--repeat-delay timer)
        (timer--function timer))
  (cancel-timer timer))
"##,
    )
}

#[test]
fn div_cx401_sit_for_and_input_pending() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (sit-for 0)
      (sit-for 0.001)
      (fboundp 'sit-for)
      (let ((unread-command-events nil)) (input-pending-p))
      (fboundp 'input-pending-p))
"##,
    )
}

#[test]
fn div_cx401_with_timeout_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'with-timeout)
          (fboundp 'with-timeout-suspend)
          (fboundp 'with-timeout-unsuspend))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx401_sleep_for_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (sleep-for 0)
          (sleep-for 0.001)
          (sleep-for 0 100))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx401_redisplay_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (redisplay)
          (redisplay t)
          (fboundp 'force-mode-line-update)
          (fboundp 'force-window-update))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx401_timer_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (let ((timer (run-with-timer 0 nil (lambda () (push :fired fired)))))
    (with-temp-buffer
      (buffer-enable-undo)
      (insert "Timer mega test buffer content")
      (put-text-property 1 6 'face 'bold)
      (let ((m (set-marker (make-marker) 8))
            (ov (make-overlay 4 14)))
        (overlay-put ov 'face 'italic)
        (overlay-put ov 'evaporate t)
        (narrow-to-region 2 18)
        (sit-for 0.01)
        (let ((state (list (nreverse fired)
                           (timerp timer)
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (cancel-timer timer)
          (undo)
          (widen()
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    )
}
