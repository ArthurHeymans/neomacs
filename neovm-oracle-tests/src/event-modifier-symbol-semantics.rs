//! Oracle parity tests for GNU `subr.el' event modifier decomposition.

use super::common::assert_oracle_parity_with_bootstrap;

#[test]
fn oracle_event_modifiers_and_basic_type_for_symbolic_mouse_events() {
    let form = r#"
(let ((events '(mouse-1
                down-mouse-1
                drag-mouse-1
                double-mouse-1
                triple-mouse-1
                double-drag-mouse-1
                C-M-down-mouse-2
                S-double-mouse-3
                wheel-up
                C-S-wheel-down)))
  (mapcar
   (lambda (event)
     (list event
           (event-modifiers event)
           (event-basic-type event)
           (get event 'event-symbol-elements)))
   events))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_event_modifiers_accept_full_event_lists() {
    let form = r#"
(let* ((w (selected-window))
       (pos (list w 7 '(12 . 34) 99))
       (events (list (list 'double-mouse-1 pos)
                     (list 'triple-down-mouse-2 pos)
                     (list 'drag-mouse-3 pos pos)
                     (list 'C-M-drag-mouse-1 pos pos))))
  (mapcar
   (lambda (event)
     (list (car event)
           (event-modifiers event)
           (event-basic-type event)))
   events))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_event_modifiers_ignore_string_events() {
    let form = r#"
(list (event-modifiers "mouse-1")
      (event-basic-type "mouse-1")
      (event-modifiers "")
      (event-basic-type ""))"#;
    assert_oracle_parity_with_bootstrap(form);
}
