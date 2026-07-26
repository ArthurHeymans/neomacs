use expect_test::expect;

use super::{assert_zero_b_layout_parity, assert_zero_b_layout_signal_parity};

#[test]
fn zero_b_layout_save_replaces_the_current_entry_and_preserves_others() {
    let elisp_form = r##"(let ((0blayout-alist
                      '((other . old-other)
                        (work . old-work)))
                     messages)
               (set-frame-parameter nil '0blayout-current "work")
               (cl-letf (((symbol-function
                           'current-window-configuration)
                          (lambda () 'new-work))
                         ((symbol-function 'message)
                          (lambda (format-string &rest args)
                            (let ((text
                                   (apply
                                    #'format
                                    format-string
                                    args)))
                              (push text messages)
                              text))))
                 (let ((result (0blayout-save)))
                   (list
                    result
                    0blayout-alist
                    (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK ("Saved the currently active layout: work" ((work . new-work) (other . old-other)) ("Saved the currently active layout: work"))"#
    ]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_save_records_a_real_window_configuration() {
    let elisp_form = r##"(let ((0blayout-alist nil))
               (set-frame-parameter nil '0blayout-current "real")
               (0blayout-save)
               (list
                (mapcar #'car 0blayout-alist)
                (window-configuration-p
                 (cdr (assq 'real 0blayout-alist)))
                (equal
                 (cdr (assq 'real 0blayout-alist))
                 (current-window-configuration))))"##;
    let expect = expect!["OK ((real) t nil)"];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_new_runs_save_reset_and_rename_in_order() {
    let elisp_form = r##"(let (events)
               (cl-letf (((symbol-function '0blayout-save)
                          (lambda () (push 'save events)))
                         ((symbol-function 'delete-other-windows)
                          (lambda (&rest _)
                            (push 'delete-other-windows events)))
                         ((symbol-function 'switch-to-buffer)
                          (lambda (buffer &rest _)
                            (push
                             (list 'switch-to-buffer buffer)
                             events)))
                         ((symbol-function
                           '0blayout-set-current-name)
                          (lambda (name)
                            (push
                             (list 'set-current-name name)
                             events)
                            name)))
                 (let ((result (0blayout-new "focus")))
                   (list result (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("focus" (save delete-other-windows (switch-to-buffer "*scratch*") (set-current-name "focus")))"#
    ]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_new_saves_the_old_layout_and_resets_real_windows() {
    let elisp_form = r##"(let ((0blayout-alist nil)
                     (left (generate-new-buffer " *0blayout-left*"))
                     (right (generate-new-buffer " *0blayout-right*")))
               (unwind-protect
                   (progn
                     (set-frame-parameter
                      nil '0blayout-current "before")
                     (switch-to-buffer left)
                     (let ((other (split-window-right)))
                       (set-window-buffer other right))
                     (0blayout-new "after")
                     (list
                      (one-window-p)
                      (buffer-name)
                      (0blayout-get-current-name)
                      (mapcar #'car 0blayout-alist)
                      (window-configuration-p
                       (cdr (assq 'before
                                  0blayout-alist)))))
                 (when (buffer-live-p left)
                   (kill-buffer left))
                 (when (buffer-live-p right)
                   (kill-buffer right))))"##;
    let expect = expect![[r#"OK (t "*scratch*" "after" (before) t)"#]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_switch_saves_then_restores_a_known_layout() {
    let elisp_form = r##"(let ((0blayout-alist
                      '((target . target-configuration)))
                     events)
               (set-frame-parameter nil '0blayout-current "source")
               (cl-letf (((symbol-function '0blayout-save)
                          (lambda () (push 'save events)))
                         ((symbol-function
                           'set-window-configuration)
                          (lambda (configuration)
                            (push
                             (list
                              'set-window-configuration
                              configuration)
                             events)))
                         ((symbol-function
                           '0blayout-set-current-name)
                          (lambda (name)
                            (push
                             (list 'set-current-name name)
                             events)
                            name))
                         ((symbol-function 'message)
                          (lambda (format-string &rest args)
                            (let ((text
                                   (apply
                                    #'format
                                    format-string
                                    args)))
                              (push (list 'message text) events)
                              text))))
                 (let ((result (0blayout-switch "target")))
                   (list result (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("Switch to layout: 'target'" (save (set-window-configuration target-configuration) (set-current-name "target") (message "Switch to layout: 'target'")))"#
    ]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_switch_saves_before_reporting_an_unknown_layout() {
    let elisp_form = r##"(let ((0blayout-alist nil)
                     events)
               (cl-letf (((symbol-function '0blayout-save)
                          (lambda () (push 'save events)))
                         ((symbol-function 'message)
                          (lambda (format-string &rest args)
                            (let ((text
                                   (apply
                                    #'format
                                    format-string
                                    args)))
                              (push (list 'message text) events)
                              text))))
                 (let ((result (0blayout-switch "missing")))
                   (list result (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("No layout with name: 'missing' is defined" (save (message "No layout with name: 'missing' is defined")))"#
    ]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_switch_restores_a_real_saved_window_configuration() {
    let elisp_form = r##"(let ((0blayout-alist nil)
                     (source
                      (generate-new-buffer
                       " *0blayout-source*"))
                     (target
                      (generate-new-buffer
                       " *0blayout-target*")))
               (unwind-protect
                   (progn
                     (switch-to-buffer target)
                     (set-frame-parameter
                      nil '0blayout-current "target")
                     (0blayout-save)
                     (switch-to-buffer source)
                     (set-frame-parameter
                      nil '0blayout-current "source")
                     (0blayout-switch "target")
                     (list
                      (buffer-name)
                      (0blayout-get-current-name)
                      (mapcar #'car 0blayout-alist)))
                 (when (buffer-live-p source)
                   (kill-buffer source))
                 (when (buffer-live-p target)
                   (kill-buffer target))))"##;
    let expect = expect![[r#"OK (" *0blayout-target*" "target" (source target))"#]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_kill_removes_current_and_selects_the_first_survivor() {
    let elisp_form = r##"(let ((0blayout-alist
                      '((current . current-configuration)
                        (next . next-configuration)
                        (later . later-configuration)))
                     events)
               (cl-letf (((symbol-function
                           '0blayout-get-current-name)
                          (lambda () "current"))
                         ((symbol-function
                           'set-window-configuration)
                          (lambda (configuration)
                            (push
                             (list
                              'set-window-configuration
                              configuration)
                             events)))
                         ((symbol-function
                           '0blayout-set-current-name)
                          (lambda (name)
                            (push
                             (list 'set-current-name name)
                             events)
                            name))
                         ((symbol-function 'message)
                          (lambda (format-string &rest args)
                            (let ((text
                                   (apply
                                    #'format
                                    format-string
                                    args)))
                              (push (list 'message text) events)
                              text))))
                 (let ((result (0blayout-kill)))
                   (list
                    result
                    0blayout-alist
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("next" ((next . next-configuration) (later . later-configuration)) ((message "Killing layout: 'current'") (set-window-configuration next-configuration) (set-current-name "next")))"#
    ]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_kill_recreates_the_default_when_none_survive() {
    let elisp_form = r##"(let ((0blayout-alist
                      '((only . only-configuration)))
                     (0blayout-default "fallback")
                     events)
               (cl-letf (((symbol-function
                           '0blayout-get-current-name)
                          (lambda () "only"))
                         ((symbol-function
                           '0blayout-set-current-name)
                          (lambda (name)
                            (push
                             (list 'set-current-name name)
                             events)
                            name))
                         ((symbol-function '0blayout-new)
                          (lambda (name)
                            (push
                             (list 'new name)
                             events)
                            name))
                         ((symbol-function 'message)
                          (lambda (format-string &rest args)
                            (let ((text
                                   (apply
                                    #'format
                                    format-string
                                    args)))
                              (push (list 'message text) events)
                              text))))
                 (let ((result (0blayout-kill)))
                   (list
                    result
                    0blayout-alist
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("fallback" nil ((message "Killing layout: 'only'") (set-current-name "fallback") (new "fallback")))"#
    ]];

    assert_zero_b_layout_parity(elisp_form, expect);
}

#[test]
fn zero_b_layout_switch_requires_a_string_layout_name() {
    let elisp_form = r##"(let ((0blayout-alist nil))
               (cl-letf (((symbol-function '0blayout-save)
                          #'ignore))
                 (0blayout-switch 'target)))"##;
    let expect = expect![r#"ERR (wrong-type-argument stringp target)"#];

    assert_zero_b_layout_signal_parity(elisp_form, expect);
}
