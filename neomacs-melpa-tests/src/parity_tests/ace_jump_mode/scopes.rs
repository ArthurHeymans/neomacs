use super::{assert_ace_jump_mode_parity, assert_ace_jump_mode_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_mode_window_scope_uses_current_selection() {
    let elisp_form = r##"(let* ((ace-jump-mode-scope 'window)
              (areas (ace-jump-list-visual-area))
              (area (car areas)))
         (list
          (length areas)
          (eq (aj-visual-area-buffer area)
              (current-buffer))
          (eq (aj-visual-area-window area)
              (selected-window))
          (eq (aj-visual-area-frame area)
              (selected-frame))
          (aj-visual-area-recover-buffer area)))"##;
    let expect = expect!["OK (1 t t t nil)"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_global_scope_flattens_all_frame_windows_in_order() {
    let elisp_form = r##"(let ((ace-jump-mode-scope 'global))
         (cl-letf (((symbol-function 'frame-list)
                    (lambda () '(f1 f2)))
                   ((symbol-function 'window-list)
                    (lambda (frame)
                      (if (eq frame 'f1)
                          '(w11 w12)
                        '(w21))))
                   ((symbol-function 'window-buffer)
                    (lambda (window)
                      (intern
                       (format "b-%s" window)))))
           (mapcar
            (lambda (area)
              (list
               (aj-visual-area-buffer area)
               (aj-visual-area-window area)
               (aj-visual-area-frame area)
               (aj-visual-area-recover-buffer area)))
            (ace-jump-list-visual-area))))"##;
    let expect = expect!["OK ((b-w11 w11 f1 nil) (b-w12 w12 f1 nil) (b-w21 w21 f2 nil))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_visible_scope_accepts_only_exact_t_visibility() {
    let elisp_form = r##"(let ((ace-jump-mode-scope 'visible))
         (cl-letf (((symbol-function 'frame-list)
                    (lambda () '(shown iconified hidden)))
                   ((symbol-function 'frame-visible-p)
                    (lambda (frame)
                      (cond
                       ((eq frame 'shown) t)
                       ((eq frame 'iconified) 'icon)
                       (t nil))))
                   ((symbol-function 'window-list)
                    (lambda (frame)
                      (list
                       (intern
                        (format "w-%s" frame)))))
                   ((symbol-function 'window-buffer)
                    (lambda (window)
                      (intern
                       (format "b-%s" window)))))
           (mapcar
            (lambda (area)
              (list
               (aj-visual-area-buffer area)
               (aj-visual-area-window area)
               (aj-visual-area-frame area)))
            (ace-jump-list-visual-area))))"##;
    let expect = expect!["OK ((b-w-shown w-shown shown))"];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_frame_scope_uses_selected_frame_windows() {
    let elisp_form = r##"(let ((ace-jump-mode-scope 'frame))
         (cl-letf (((symbol-function 'selected-frame)
                    (lambda () 'selected))
                   ((symbol-function 'window-list)
                    (lambda (frame)
                      (list
                       frame
                       'second-window)))
                   ((symbol-function 'window-buffer)
                    (lambda (window)
                      (list 'buffer-for window))))
           (mapcar
            (lambda (area)
              (list
               (aj-visual-area-buffer area)
               (aj-visual-area-window area)
               (aj-visual-area-frame area)))
            (ace-jump-list-visual-area))))"##;
    let expect = expect![
        "OK (((buffer-for selected) selected selected) ((buffer-for second-window) second-window selected))"
    ];
    assert_ace_jump_mode_parity(elisp_form, expect);
}

#[test]
fn ace_jump_mode_invalid_scope_signals_exact_configuration_error() {
    let elisp_form = r##"(let ((ace-jump-mode-scope 'invalid))
         (ace-jump-list-visual-area))"##;
    let expect = expect![[
        r#"ERR (error "[AceJump] Invalid ace-jump-mode-scope, please check your configuration")"#
    ]];
    assert_ace_jump_mode_signal_parity(elisp_form, expect);
}
