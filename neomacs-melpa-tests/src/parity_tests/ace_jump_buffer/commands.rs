use expect_test::expect;

use super::assert_ace_jump_buffer_parity;

#[test]
fn ace_jump_buffer_primary_command_sets_complete_dynamic_and_buffer_local_state_in_order() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abc")
               (goto-char 2)
               (let ((avy-background 'outer-background)
                     (avy-all-windows 'outer-windows)
                     (bs-attributes-list 'outer-attributes)
                     (avy-handler-function 'outer-handler)
                     (avy-style 'outer-style)
                     (ajb/showing 'outer-showing)
                     (ajb-bs-configuration "chosen")
                     (ajb-max-window-height 17)
                     (ajb-style 'words)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'bs--show-with-configuration)
                       (lambda (configuration)
                         (push
                          (list
                           'show
                           configuration
                           avy-background
                           avy-all-windows
                           bs-attributes-list
                           avy-handler-function
                           avy-style
                           ajb/showing)
                          calls)
                         'show-result))
                      ((symbol-function
                        'face-remap-add-relative)
                       (lambda (&rest arguments)
                         (push
                          (cons 'face arguments)
                          calls)
                         'face-cookie))
                      ((symbol-function 'bs--set-window-height)
                       (lambda ()
                         (push
                          (list
                           'height
                           (point)
                           bs-header-lines-length
                           bs-max-window-height)
                          calls)
                         'height-result))
                      ((symbol-function
                        'ajb/goto-line-and-buffer)
                       (lambda ()
                         (push
                          (list
                           'goto
                           (point)
                           bs-header-lines-length
                           bs-max-window-height)
                          calls)
                         'goto-result)))
                   (let ((result (ace-jump-buffer)))
                     (list
                      result
                      (point)
                      (local-variable-p
                       'bs-header-lines-length)
                      bs-header-lines-length
                      (local-variable-p
                       'bs-max-window-height)
                      bs-max-window-height
                      avy-background
                      avy-all-windows
                      bs-attributes-list
                      avy-handler-function
                      avy-style
                      ajb/showing
                      (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (goto-result 2 t 0 t 17 outer-background outer-windows outer-attributes outer-handler outer-style outer-showing ((show "chosen" nil nil (("" 2 2 left " ") ("" 1 1 left bs--get-marked-string) ("" 1 1 left " ") ("Buffer" bs--get-name-length 10 left bs--get-name)) ajb/exit words t) (face default ajb-face) (height 1 0 17) (goto 1 0 17)))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_primary_command_restores_point_and_dynamic_state_after_show_error() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abc")
               (goto-char 2)
               (let ((avy-background 'outer-background)
                     (avy-all-windows 'outer-windows)
                     (bs-attributes-list 'outer-attributes)
                     (avy-handler-function 'outer-handler)
                     (avy-style 'outer-style)
                     (ajb/showing 'outer-showing)
                     calls)
                 (let ((outcome
                        (condition-case error-data
                            (cl-letf
                                (((symbol-function
                                   'bs--show-with-configuration)
                                  (lambda (_configuration)
                                    (push 'show calls)
                                    (goto-char (point-max))
                                    (error
                                     "synthetic show failure")))
                                 ((symbol-function
                                   'face-remap-add-relative)
                                  (lambda (&rest _arguments)
                                    (push 'face calls))))
                              (ace-jump-buffer))
                          (error error-data))))
                   (list
                    outcome
                    (point)
                    avy-background
                    avy-all-windows
                    bs-attributes-list
                    avy-handler-function
                    avy-style
                    ajb/showing
                    (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((error "synthetic show failure") 2 outer-background outer-windows outer-attributes outer-handler outer-style outer-showing (show))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_other_window_wrapper_sets_only_other_window_for_the_call() {
    let elisp_form = r##"(let ((ajb/other-window 'outer-other)
                   (ajb/in-one-window 'outer-one)
                   calls)
               (cl-letf
                   (((symbol-function 'ace-jump-buffer)
                     (lambda ()
                       (push
                        (list
                         ajb/other-window
                         ajb/in-one-window)
                        calls)
                       'jump-result)))
                 (list
                  (ace-jump-buffer-other-window)
                  ajb/other-window
                  ajb/in-one-window
                  (nreverse calls))))"##;
    let expect = expect!["OK (jump-result outer-other outer-one ((t outer-one)))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_one_window_wrapper_sets_only_one_window_for_the_call() {
    let elisp_form = r##"(let ((ajb/other-window 'outer-other)
                   (ajb/in-one-window 'outer-one)
                   calls)
               (cl-letf
                   (((symbol-function 'ace-jump-buffer)
                     (lambda ()
                       (push
                        (list
                         ajb/other-window
                         ajb/in-one-window)
                        calls)
                       'jump-result)))
                 (list
                  (ace-jump-buffer-in-one-window)
                  ajb/other-window
                  ajb/in-one-window
                  (nreverse calls))))"##;
    let expect = expect!["OK (jump-result outer-other outer-one ((outer-other t)))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_command_reads_names_history_and_default_then_restores_configuration()
 {
    let elisp_form = r##"(let ((bs-configurations
                    '(("one" nil)
                      ("two" nil)
                      ("three" nil)))
                   (ajb/configuration-history
                    '("three" "one"))
                   (ajb-bs-configuration "outer")
                   calls)
               (cl-letf
                   (((symbol-function 'completing-read)
                     (lambda (&rest arguments)
                       (push
                        (cons 'read arguments)
                        calls)
                       "two"))
                    ((symbol-function 'ace-jump-buffer)
                     (lambda ()
                       (push
                        (list
                         'jump
                         ajb-bs-configuration)
                        calls)
                       'jump-result)))
                 (list
                  (ace-jump-buffer-with-configuration)
                  ajb-bs-configuration
                  ajb/configuration-history
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (jump-result "outer" ("three" "one") ((read "Ace jump buffer with configuration: " ("one" "two" "three") nil t nil ajb/configuration-history "three") (jump "two")))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_command_uses_nil_default_for_empty_history() {
    let elisp_form = r##"(let ((bs-configurations
                    '(("all" nil)))
                   (ajb/configuration-history nil)
                   calls)
               (cl-letf
                   (((symbol-function 'completing-read)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       "all"))
                    ((symbol-function 'ace-jump-buffer)
                     (lambda ()
                       (push
                        (list 'jump ajb-bs-configuration)
                        calls)
                       'jump-result)))
                 (list
                  (ace-jump-buffer-with-configuration)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (jump-result (("Ace jump buffer with configuration: " ("all") nil t nil ajb/configuration-history nil) (jump "all")))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
