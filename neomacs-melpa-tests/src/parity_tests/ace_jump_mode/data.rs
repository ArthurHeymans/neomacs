use super::{assert_ace_jump_mode_parity, assert_ace_jump_mode_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_mode_position_constructor_accessors_predicate_and_copy_match() {
    let elisp_form = r##"(let* ((area (make-aj-visual-area
                           :buffer 'buffer
                           :window 'window
                           :frame 'frame
                           :recover-buffer 'recover))
              (position (make-aj-position
                         :offset 17
                         :visual-area area))
              (copy (copy-aj-position position)))
         (list
          (aj-position-p position)
          (aj-position-offset position)
          (eq (aj-position-visual-area position) area)
          (aj-position-buffer position)
          (aj-position-window position)
          (aj-position-frame position)
          (aj-position-recover-buffer position)
          (equal copy position)
          (eq copy position)
          (aj-position-p '(cl-struct-aj-position 1 nil))
          (aj-position-p nil)))"##;
    let expect = expect!["OK (t 17 t buffer window frame recover t nil nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_position_and_visual_area_setters_are_independent_after_copy() {
    let elisp_form = r##"(let* ((area (make-aj-visual-area
                           :buffer 'a
                           :window 'b
                           :frame 'c
                           :recover-buffer 'd))
              (position (make-aj-position
                         :offset 1
                         :visual-area area))
              (copy (copy-aj-position position))
              (area-copy (copy-aj-visual-area area)))
         (setf (aj-position-offset copy) 9)
         (setf (aj-position-visual-area copy) area-copy)
         (setf (aj-visual-area-buffer area-copy) 'changed)
         (list
          (aj-position-offset position)
          (aj-position-offset copy)
          (aj-position-buffer position)
          (aj-position-buffer copy)
          (eq (aj-position-visual-area position)
              (aj-position-visual-area copy))))"##;
    let expect = expect!["OK (1 9 a changed nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_visual_area_constructor_accessors_predicate_and_copy_match() {
    let elisp_form = r##"(let* ((area (make-aj-visual-area
                           :buffer 'buffer
                           :window 'window
                           :frame 'frame
                           :recover-buffer 'recover))
              (copy (copy-aj-visual-area area)))
         (list
          (aj-visual-area-p area)
          (aj-visual-area-buffer area)
          (aj-visual-area-window area)
          (aj-visual-area-frame area)
          (aj-visual-area-recover-buffer area)
          (equal copy area)
          (eq copy area)
          (aj-visual-area-p
           '(cl-struct-aj-visual-area nil nil nil nil))
          (aj-visual-area-p nil)))"##;
    let expect = expect!["OK (t buffer window frame recover t nil nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_queue_starts_empty_and_round_trips_one_item() {
    let elisp_form = r##"(let ((queue (make-aj-queue)))
         (let ((initial
                (list
                 (aj-queue-p queue)
                 (aj-queue-head queue)
                 (aj-queue-tail queue))))
           (aj-queue-push 'one queue)
           (list
            initial
            (aj-queue-head queue)
            (eq (aj-queue-head queue)
                (aj-queue-tail queue))
            (aj-queue-pop queue)
            (aj-queue-head queue)
            (aj-queue-tail queue))))"##;
    let expect = expect!["OK ((t nil nil) (one) t one nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_queue_preserves_fifo_order_across_interleaved_operations() {
    let elisp_form = r##"(let ((queue (make-aj-queue)))
         (aj-queue-push 'a queue)
         (aj-queue-push 'b queue)
         (let ((first (aj-queue-pop queue)))
           (aj-queue-push 'c queue)
           (list
            first
            (aj-queue-pop queue)
            (aj-queue-pop queue)
            (aj-queue-head queue)
            (aj-queue-tail queue))))"##;
    let expect = expect!["OK (a b c nil nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_queue_tail_tracks_the_last_cons_cell() {
    let elisp_form = r##"(let ((queue (make-aj-queue)))
         (mapc
          (lambda (item)
            (aj-queue-push item queue))
          '(a b c))
         (list
          (aj-queue-head queue)
          (aj-queue-tail queue)
          (eq (nthcdr 2 (aj-queue-head queue))
              (aj-queue-tail queue))
          (progn
            (aj-queue-pop queue)
            (eq (cdr (aj-queue-head queue))
                (aj-queue-tail queue)))))"##;
    let expect = expect!["OK ((a b . #1=(c)) #1# t t)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_queue_copy_has_independent_slots_but_shared_list_payload() {
    let elisp_form = r##"(let ((queue (make-aj-queue)))
         (aj-queue-push 'a queue)
         (aj-queue-push 'b queue)
         (let ((copy (copy-aj-queue queue)))
           (setf (aj-queue-head copy)
                 (cdr
                  (aj-queue-head copy)))
           (list
            (aj-queue-p copy)
            (equal copy queue)
            (eq
             (aj-queue-tail copy)
             (aj-queue-tail queue))
            (aj-queue-head queue)
            (aj-queue-head copy)
            (aj-queue-tail queue)
            (aj-queue-tail copy))))"##;
    let expect = expect!["OK (t nil t (a . #1=(b)) #1# #1# #1#)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_queue_pop_from_empty_signals_exact_error() {
    let elisp_form = r##"(aj-queue-pop (make-aj-queue))"##;
    let expect = expect![[r#"ERR (error "[AceJump] Interal Error: Empty queue")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_struct_constructor_keyword_validation_matches() {
    let elisp_form = r##"(make-aj-position :unknown 1)"##;
    let expect =
        expect![[r#"ERR (error "Keyword argument :unknown not one of (:offset :visual-area)")"#]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}
