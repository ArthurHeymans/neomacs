use expect_test::expect;

use super::{assert_ace_jump_buffer_parity, assert_ace_jump_buffer_signal_parity};

#[test]
fn ace_jump_buffer_select_uses_current_window_by_default() {
    let elisp_form = r##"(let ((buffer
                    (generate-new-buffer
                     "*buffer-selection*"))
                   calls)
               (unwind-protect
                   (with-current-buffer buffer
                     (let ((ajb/other-window nil)
                           (ajb/in-one-window nil))
                       (cl-letf
                           (((symbol-function 'bs-select)
                             (lambda ()
                               (push 'current calls)
                               'current-result))
                            ((symbol-function
                              'bs-select-other-window)
                             (lambda ()
                               (push 'other calls)))
                            ((symbol-function
                              'bs-select-in-one-window)
                             (lambda ()
                               (push 'one calls))))
                         (list
                          (ajb/select-buffer)
                          (nreverse calls)))))
                 (kill-buffer buffer)))"##;
    let expect = expect!["OK (current-result (current))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_prefers_other_window_when_both_flags_are_true() {
    let elisp_form = r##"(let ((buffer
                    (generate-new-buffer
                     "*buffer-selection*"))
                   calls)
               (unwind-protect
                   (with-current-buffer buffer
                     (let ((ajb/other-window t)
                           (ajb/in-one-window t))
                       (cl-letf
                           (((symbol-function 'bs-select)
                             (lambda ()
                               (push 'current calls)))
                            ((symbol-function
                              'bs-select-other-window)
                             (lambda ()
                               (push 'other calls)
                               'other-result))
                            ((symbol-function
                              'bs-select-in-one-window)
                             (lambda ()
                               (push 'one calls))))
                         (list
                          (ajb/select-buffer)
                          (nreverse calls)))))
                 (kill-buffer buffer)))"##;
    let expect = expect!["OK (other-result (other))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_uses_one_window_when_only_that_flag_is_true() {
    let elisp_form = r##"(let ((buffer
                    (generate-new-buffer
                     "*buffer-selection*"))
                   calls)
               (unwind-protect
                   (with-current-buffer buffer
                     (let ((ajb/other-window nil)
                           (ajb/in-one-window t))
                       (cl-letf
                           (((symbol-function 'bs-select)
                             (lambda ()
                               (push 'current calls)))
                            ((symbol-function
                              'bs-select-other-window)
                             (lambda ()
                               (push 'other calls)))
                            ((symbol-function
                              'bs-select-in-one-window)
                             (lambda ()
                               (push 'one calls)
                               'one-result)))
                         (list
                          (ajb/select-buffer)
                          (nreverse calls)))))
                 (kill-buffer buffer)))"##;
    let expect = expect!["OK (one-result (one))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_is_a_noop_when_the_buffer_name_does_not_match() {
    let elisp_form = r##"(with-temp-buffer
               (rename-buffer "ordinary-ajb-buffer" t)
               (let (calls)
                 (cl-letf
                     (((symbol-function 'bs-select)
                       (lambda ()
                         (push 'current calls)))
                      ((symbol-function
                        'bs-select-other-window)
                       (lambda ()
                         (push 'other calls)))
                      ((symbol-function
                        'bs-select-in-one-window)
                       (lambda ()
                         (push 'one calls))))
                   (list
                    (ajb/select-buffer)
                    calls))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_uses_the_current_buffer_name_as_a_regexp() {
    let elisp_form = r##"(with-temp-buffer
               (rename-buffer "buffer-selection" t)
               (let (calls)
                 (cl-letf
                     (((symbol-function 'bs-select)
                       (lambda ()
                         (push 'selected calls)
                         'selected-result)))
                   (list
                    (ajb/select-buffer)
                    (nreverse calls)))))"##;
    let expect = expect!["OK (selected-result (selected))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_propagates_an_invalid_buffer_name_regexp() {
    let elisp_form = r##"(with-temp-buffer
               (rename-buffer "[" t)
               (ajb/select-buffer))"##;
    let expect = expect![[r#"ERR (invalid-regexp "Unmatched [ or [^")"#]];
    assert_ace_jump_buffer_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_kill_menu_calls_bs_kill_then_kills_the_selection_buffer() {
    let elisp_form = r##"(let ((buffer
                    (generate-new-buffer
                     "*buffer-selection*"))
                   calls)
               (cl-letf
                   (((symbol-function 'bs-kill)
                     (lambda ()
                       (push
                        (list
                         'bs-kill
                         (buffer-live-p buffer))
                        calls)
                       'bs-result)))
                 (list
                  (ajb/kill-bs-menu)
                  (buffer-live-p buffer)
                  (nreverse calls))))"##;
    let expect = expect!["OK (t nil ((bs-kill t)))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_kill_menu_tolerates_an_absent_selection_buffer() {
    let elisp_form = r##"(progn
               (when (get-buffer "*buffer-selection*")
                 (kill-buffer "*buffer-selection*"))
               (let (calls)
                 (cl-letf
                     (((symbol-function 'bs-kill)
                       (lambda ()
                         (push 'bs-kill calls)
                         'bs-result)))
                   (list
                    (ajb/kill-bs-menu)
                    (get-buffer "*buffer-selection*")
                    (nreverse calls)))))"##;
    let expect = expect!["OK (nil nil (bs-kill))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_exit_kills_the_menu_and_throws_done_with_nil() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function 'ajb/kill-bs-menu)
                     (lambda ()
                       (push 'kill calls)
                       'kill-result)))
                 (list
                  (catch 'done
                    (ajb/exit ?x)
                    'not-thrown)
                  (nreverse calls))))"##;
    let expect = expect!["OK (nil (kill))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_goto_nil_result_kills_without_action_or_selection() {
    let elisp_form = r##"(with-temp-buffer
               (insert "first\nsecond\n")
               (goto-char (point-min))
               (let (calls)
                 (cl-letf
                     (((symbol-function 'selected-window)
                       (lambda () 'selected-window))
                      ((symbol-function 'window-end)
                       (lambda (window update)
                         (push
                          (list 'window-end window update)
                          calls)
                         12))
                      ((symbol-function 'avy--line)
                       (lambda (prefix start end)
                         (push
                          (list
                           'avy
                           prefix
                           start
                           end
                           avy-all-windows)
                          calls)
                         nil))
                      ((symbol-function 'avy-action-goto)
                       (lambda (result)
                         (push (list 'goto result) calls)))
                      ((symbol-function 'ajb/select-buffer)
                       (lambda ()
                         (push 'select calls)))
                      ((symbol-function 'ajb/kill-bs-menu)
                       (lambda ()
                         (push 'kill calls)
                         'kill-result)))
                   (list
                    (ajb/goto-line-and-buffer)
                    (nreverse calls)))))"##;
    let expect = expect!["OK (kill-result ((window-end selected-window t) (avy nil 1 12 t) kill))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_goto_string_result_kills_without_action_or_selection() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function 'avy--line)
                     (lambda (&rest _arguments)
                       "invalid-key"))
                    ((symbol-function 'avy-action-goto)
                     (lambda (result)
                       (push (list 'goto result) calls)))
                    ((symbol-function 'ajb/select-buffer)
                     (lambda ()
                       (push 'select calls)))
                    ((symbol-function 'ajb/kill-bs-menu)
                     (lambda ()
                       (push 'kill calls)
                       'kill-result)))
                 (list
                  (ajb/goto-line-and-buffer)
                  (nreverse calls))))"##;
    let expect = expect!["OK (kill-result (kill))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_goto_true_result_keeps_menu_without_action_or_selection() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function 'avy--line)
                     (lambda (&rest _arguments)
                       t))
                    ((symbol-function 'avy-action-goto)
                     (lambda (result)
                       (push (list 'goto result) calls)))
                    ((symbol-function 'ajb/select-buffer)
                     (lambda ()
                       (push 'select calls)))
                    ((symbol-function 'ajb/kill-bs-menu)
                     (lambda ()
                       (push 'kill calls))))
                 (list
                  (ajb/goto-line-and-buffer)
                  calls)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_goto_line_uses_outer_window_scope_before_body_rebinds_nil() {
    let elisp_form = r##"(with-temp-buffer
               (insert "first\nsecond\nthird\n")
               (goto-char 8)
               (let ((avy-all-windows 'outer)
                     calls)
                 (cl-letf
                     (((symbol-function 'selected-window)
                       (lambda () 'selected-window))
                      ((symbol-function 'window-end)
                       (lambda (window update)
                         (push
                          (list 'window-end window update)
                          calls)
                         19))
                      ((symbol-function 'avy--line)
                       (lambda (prefix start end)
                         (push
                          (list
                           'avy
                           prefix
                           start
                           end
                           avy-all-windows)
                          calls)
                         15))
                      ((symbol-function 'avy-action-goto)
                       (lambda (result)
                         (push
                          (list
                           'goto
                           result
                           avy-all-windows)
                          calls)
                         'goto-result))
                      ((symbol-function 'ajb/select-buffer)
                       (lambda ()
                         (push
                          (list
                           'select
                           avy-all-windows)
                          calls)
                         'select-result))
                      ((symbol-function 'ajb/kill-bs-menu)
                       (lambda ()
                         (push 'kill calls))))
                   (list
                    (ajb/goto-line-and-buffer)
                    avy-all-windows
                    (nreverse calls)))))"##;
    let expect = expect![
        "OK (select-result outer ((window-end selected-window t) (avy nil 7 19 outer) (goto 15 nil) (select nil)))"
    ];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
