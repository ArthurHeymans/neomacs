use super::assert_ace_mc_parity;
use expect_test::expect;

#[test]
fn ace_mc_jump_start_is_a_noop_when_not_marking() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (goto-char 4)
         (let ((ace-mc-marking nil)
               (ace-mc-saved-point 'saved)
               (ace-mc-keyboard-reset 'reset))
           (list
            (ace-mc-maybe-jump-start)
            ace-mc-saved-point
            ace-mc-keyboard-reset
            (point))))"##;
    let expect = expect!["OK (nil saved reset 4)"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_start_saves_point_and_clears_keyboard_reset_while_marking() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (goto-char 4)
         (let ((ace-mc-marking t)
               (ace-mc-saved-point 'saved)
               (ace-mc-keyboard-reset 'reset))
           (list
            (ace-mc-maybe-jump-start)
            ace-mc-saved-point
            ace-mc-keyboard-reset
            (point))))"##;
    let expect = expect!["OK (nil 4 nil 4)"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_resets_immediately_when_not_marking() {
    let elisp_form = r##"(let ((events nil)
             (ace-mc-marking nil))
         (cl-letf
             (((symbol-function 'ace-mc-reset)
               (lambda ()
                 (push 'reset events))))
           (list
            (ace-mc-maybe-jump-end)
            (nreverse events))))"##;
    let expect = expect!["OK (#1=(reset) #1#)"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_creates_a_cursor_at_a_new_point_then_restores_origin() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 8)
         (let ((events nil)
               (ace-mc-marking t)
               (ace-mc-saved-point 3)
               (ace-mc-loop-marking nil)
               (ace-mc-keyboard-reset nil))
           (cl-letf
               (((symbol-function 'overlays-at)
                 (lambda (point)
                   (push (list 'overlays point) events)
                   nil))
                ((symbol-function 'mc/create-fake-cursor-at-point)
                 (lambda ()
                   (push (list 'create (point)) events)
                   'created))
                ((symbol-function 'mc/maybe-multiple-cursors-mode)
                 (lambda ()
                   (push 'maybe-mode events))))
             (list
              (ace-mc-maybe-jump-end)
              (point)
              ace-mc-marking
              (nreverse events)))))"##;
    let expect = expect!["OK (nil 3 nil ((overlays 8) (create 8) maybe-mode))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_does_not_create_a_cursor_without_movement() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdefghij")
         (goto-char 5)
         (let ((events nil)
               (ace-mc-marking t)
               (ace-mc-saved-point 5)
               (ace-mc-loop-marking nil)
               (ace-mc-keyboard-reset nil))
           (cl-letf
               (((symbol-function 'overlays-at)
                 (lambda (_point) nil))
                ((symbol-function 'mc/create-fake-cursor-at-point)
                 (lambda ()
                   (push 'unexpected-create events)))
                ((symbol-function 'mc/maybe-multiple-cursors-mode)
                 (lambda ()
                   (push 'maybe-mode events))))
             (list
              (ace-mc-maybe-jump-end)
              (point)
              ace-mc-marking
              (nreverse events)))))"##;
    let expect = expect!["OK (nil 5 nil (maybe-mode))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_removes_only_the_first_fake_cursor_at_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert "abcdef")
         (goto-char 4)
         (let ((events nil)
               (ace-mc-marking t)
               (ace-mc-saved-point nil)
               (ace-mc-loop-marking nil)
               (ace-mc-keyboard-reset nil))
           (cl-letf
               (((symbol-function 'overlays-at)
                 (lambda (point)
                   (push (list 'overlays point) events)
                   '(ordinary fake-one fake-two)))
                ((symbol-function 'mc/fake-cursor-p)
                 (lambda (overlay)
                   (memq overlay
                         '(fake-one fake-two))))
                ((symbol-function 'mc/remove-fake-cursor)
                 (lambda (overlay)
                   (push (list 'remove overlay) events)))
                ((symbol-function 'mc/create-fake-cursor-at-point)
                 (lambda ()
                   (push 'unexpected-create events)))
                ((symbol-function 'mc/maybe-multiple-cursors-mode)
                 (lambda ()
                   (push 'maybe-mode events))))
             (list
              (ace-mc-maybe-jump-end)
              (point)
              ace-mc-marking
              (nreverse events)))))"##;
    let expect = expect!["OK (nil 4 nil ((overlays 4) (remove fake-one) maybe-mode))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_loops_when_candidate_list_is_unbound() {
    let elisp_form = r##"(let ((was-bound
              (boundp 'candidate-list))
             (saved
              (and (boundp 'candidate-list)
                   candidate-list))
             (events nil)
             (ace-mc-marking t)
             (ace-mc-saved-point nil)
             (ace-mc-loop-marking t)
             (ace-mc-keyboard-reset nil)
             (ace-mc-query-char ?q))
         (unwind-protect
             (progn
               (makunbound 'candidate-list)
               (cl-letf
                   (((symbol-function 'overlays-at)
                     (lambda (_point) nil))
                    ((symbol-function 'mc/create-fake-cursor-at-point)
                     (lambda () nil))
                    ((symbol-function 'mc/maybe-multiple-cursors-mode)
                     (lambda ()
                       (push 'maybe-mode events)))
                    ((symbol-function 'ace-mc-add-char)
                     (lambda (query)
                       (push (list 'add query) events))))
                 (list
                  (ace-mc-maybe-jump-end)
                  ace-mc-marking
                  (nreverse events))))
           (if was-bound
               (set 'candidate-list saved)
             (makunbound 'candidate-list))))"##;
    let expect = expect!["OK (#1=((add 113)) t (maybe-mode . #1#))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_loops_only_for_multiple_candidates() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((events nil)
                 (ace-mc-marking t)
                 (ace-mc-saved-point nil)
                 (ace-mc-loop-marking t)
                 (ace-mc-keyboard-reset nil)
                 (ace-mc-query-char ?x))
             (cl-progv
                 '(candidate-list)
                 (list fixture)
               (cl-letf
                   (((symbol-function 'overlays-at)
                     (lambda (_point) nil))
                    ((symbol-function 'mc/create-fake-cursor-at-point)
                     (lambda () nil))
                    ((symbol-function 'mc/maybe-multiple-cursors-mode)
                     (lambda ()
                       (push 'maybe-mode events)))
                    ((symbol-function 'ace-mc-add-char)
                     (lambda (query)
                       (push (list 'add query) events))))
                 (list
                  fixture
                  (ace-mc-maybe-jump-end)
                  ace-mc-marking
                  (nreverse events))))))
         '(nil (only) (first second)))"##;
    let expect = expect![[
        "OK ((nil nil nil (maybe-mode)) ((only) nil nil (maybe-mode)) ((first second) #1=((add 120)) t (maybe-mode . #1#)))"
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_jump_end_keyboard_reset_prevents_another_loop() {
    let elisp_form = r##"(let ((events nil)
             (candidate-list '(first second))
             (ace-mc-marking t)
             (ace-mc-saved-point nil)
             (ace-mc-loop-marking t)
             (ace-mc-keyboard-reset t)
             (ace-mc-query-char ?x))
         (cl-letf
             (((symbol-function 'overlays-at)
               (lambda (_point) nil))
              ((symbol-function 'mc/create-fake-cursor-at-point)
               (lambda () nil))
              ((symbol-function 'mc/maybe-multiple-cursors-mode)
               (lambda ()
                 (push 'maybe-mode events)))
              ((symbol-function 'ace-mc-add-char)
               (lambda (query)
                 (push (list 'unexpected-add query)
                       events))))
           (list
            (ace-mc-maybe-jump-end)
            ace-mc-marking
            (nreverse events))))"##;
    let expect = expect!["OK (nil nil (maybe-mode))"];
    assert_ace_mc_parity(elisp_form, expect);
}
