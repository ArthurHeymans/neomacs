//! Oracle parity tests for GNU `subr.el' event predicate semantics.

use super::common::assert_oracle_parity_with_bootstrap;

#[test]
fn oracle_eventp_accepts_integers_and_non_keyword_symbols() {
    let form = r#"
(list
 (eventp ?a)
 (eventp -1)
 (eventp 'mouse-1)
 (eventp '(mouse-1 ignored))
 (eventp nil)
 (eventp t)
 (eventp :keyword)
 (eventp '(:keyword ignored))
 (eventp "mouse-1")
 (eventp '(\"mouse-1\" ignored)))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_mouse_event_predicates_follow_basic_type() {
    let form = r#"
(let* ((w (selected-window))
       (pos (list w 7 '(12 . 34) 99))
       (events (list 'mouse-1
                     'down-mouse-1
                     'drag-mouse-2
                     'double-mouse-3
                     'mouse-movement
                     (list 'C-M-drag-mouse-1 pos pos)
                     (list 'wheel-up pos)
                     'wheel-up
                     'f1
                     ?a)))
  (mapcar
   (lambda (event)
     (list (if (consp event) (car event) event)
           (mouse-event-p event)
           (mouse-movement-p event)
           (event-basic-type event)))
   events))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_mouse_movement_p_only_checks_event_car() {
    let form = r#"
(list
 (mouse-movement-p '(mouse-movement))
 (mouse-movement-p '(mouse-movement nil))
 (mouse-movement-p 'mouse-movement)
 (mouse-movement-p nil)
 (mouse-movement-p '(mouse-1))
 (mouse-movement-p '(drag-mouse-1 nil nil)))"#;
    assert_oracle_parity_with_bootstrap(form);
}
