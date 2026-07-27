use expect_test::expect;

use super::assert_annotate_depth_parity;

#[test]
fn annotate_depth_enter_annotates_before_creating_timer() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'annotate-depth--annotate)
                    (lambda () (push 'annotate calls)))
                   ((symbol-function 'annotate-depth--create-idle-timer)
                    (lambda () (push 'timer calls))))
           (annotate-depth-enter)
           (nreverse calls)))"##;
    let expect = expect!["OK (annotate timer)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_exit_stops_timer_before_clearing_overlays() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'annotate-depth--stop-timer)
                    (lambda () (push 'stop calls)))
                   ((symbol-function 'annotate-depth--clear-overlays)
                    (lambda () (push 'clear calls))))
           (annotate-depth-exit)
           (nreverse calls)))"##;
    let expect = expect!["OK (stop clear)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_mode_enable_disable_routes_lifecycle_and_lighter() {
    let elisp_form = r##"(with-temp-buffer
         (let (calls
               (annotate-depth-lighter " Deep"))
           (cl-letf (((symbol-function 'annotate-depth-enter)
                      (lambda () (push 'enter calls)))
                     ((symbol-function 'annotate-depth-exit)
                      (lambda () (push 'exit calls))))
             (annotate-depth-mode 1)
             (let ((enabled
                    (list annotate-depth-mode
                          (assq 'annotate-depth-mode minor-mode-alist))))
               (annotate-depth-mode -1)
               (list enabled
                     annotate-depth-mode
                     (nreverse calls)
                     (assq 'annotate-depth-mode minor-mode-alist))))))"##;
    let expect =
        expect!["OK ((t #1=(annotate-depth-mode annotate-depth-lighter)) nil (enter exit) #1#)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_timer_creation_passes_exact_timeout_repeat_and_callback() {
    let elisp_form = r##"(with-temp-buffer
         (let ((annotate-depth-idle-timeout 1.75)
               captured)
           (cl-letf (((symbol-function 'run-with-idle-timer)
                      (lambda (&rest args)
                        (setq captured args)
                        'fake-timer)))
             (annotate-depth--create-idle-timer)
             (list captured
                   annotate-depth--idle-timer
                   (local-variable-p 'annotate-depth--idle-timer)))))"##;
    let expect = expect!["OK ((1.75 t annotate-depth--annotate) fake-timer t)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_timer_creation_is_idempotent_and_nil_timeout_disables_it() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (let ((annotate-depth-idle-timeout nil)
                 calls)
             (cl-letf (((symbol-function 'run-with-idle-timer)
                        (lambda (&rest args) (push args calls))))
               (annotate-depth--create-idle-timer)
               (list calls annotate-depth--idle-timer))))
         (with-temp-buffer
           (let ((annotate-depth-idle-timeout 2)
                 (annotate-depth--idle-timer 'existing)
                 calls)
             (cl-letf (((symbol-function 'run-with-idle-timer)
                        (lambda (&rest args) (push args calls))))
               (annotate-depth--create-idle-timer)
               (list calls annotate-depth--idle-timer)))))"##;
    let expect = expect!["OK ((nil nil) (nil existing))"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_stop_timer_cancels_once_and_clears_buffer_local_state() {
    let elisp_form = r##"(with-temp-buffer
         (let ((annotate-depth--idle-timer 'fake-timer)
               calls)
           (cl-letf (((symbol-function 'cancel-timer)
                      (lambda (timer) (push timer calls))))
             (annotate-depth--stop-timer)
             (annotate-depth--stop-timer)
             (list (nreverse calls)
                   annotate-depth--idle-timer
                   (local-variable-p 'annotate-depth--idle-timer)))))"##;
    let expect = expect!["OK ((fake-timer) nil nil)"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_real_mode_without_timer_marks_and_then_cleans_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (insert "root\n    nested\n      deeper\n")
         (let ((annotate-depth-idle-timeout nil)
               (annotate-depth-threshold 2)
               (standard-indent 2))
           (annotate-depth-mode 1)
           (let ((enabled
                  (list annotate-depth-mode
                        (length annotate-depth--overlays)
                        (mapcar
                         (lambda (overlay)
                           (list (overlay-start overlay)
                                 (overlay-end overlay)))
                         annotate-depth--overlays)
                        annotate-depth--idle-timer)))
             (annotate-depth-mode -1)
             (list enabled
                   annotate-depth-mode
                   annotate-depth--overlays
                   annotate-depth--idle-timer
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((t 2 ((23 29) (10 16)) nil) nil nil nil "root\n    nested\n      deeper\n")"#
    ]];
    assert_annotate_depth_parity(elisp_form, expect);
}
