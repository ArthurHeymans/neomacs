use super::assert_assess_robot_parity;
use expect_test::{Expect, expect};

#[test]
fn robot_library_registers_its_complete_macro_function_and_command_surface() {
    let elisp_form = r##"
(list
 (featurep 'assess-robot)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (if (macrop symbol)
         'macro
       (if (commandp symbol)
           'command
         'function))
     (help-function-arglist
      symbol t)
     (file-name-nondirectory
      (or
       (symbol-file
        symbol 'defun)
       ""))))
  '(assess-robot-with-switched-buffer
    assess-robot-with-temp-switched-buffer
    assess-robot-with-switched-buffer-string
    assess-robot-execute-kmacro
    assess-robot-copy-and-finish)))
"##;
    let expect: Expect = expect![[
        r#"OK (t ((assess-robot-with-switched-buffer macro (buffer &rest body) "assess-robot.el") (assess-robot-with-temp-switched-buffer macro (&rest body) "assess-robot.el") (assess-robot-with-switched-buffer-string macro (&rest body) "assess-robot.el") (assess-robot-execute-kmacro function (macro) "assess-robot.el") (assess-robot-copy-and-finish command nil "assess-robot.el")))"#
    ]];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn switched_buffer_macro_runs_body_in_target_and_restores_original_after_value() {
    let elisp_form = r##"
(let ((before
       (current-buffer))
      (target
       (generate-new-buffer
        " *assess-robot-target*"))
      result)
  (unwind-protect
      (progn
        (setq result
              (assess-robot-with-switched-buffer
                  target
                (insert "robot payload")
                (list
                 (eq
                  (current-buffer)
                  target)
                 (buffer-string))))
        (list
         result
         (eq
          before
          (current-buffer))
         (with-current-buffer target
           (buffer-string))
         (buffer-live-p target)))
    (kill-buffer target)))
"##;
    let expect: Expect = expect![[r#"OK ((t "robot payload") t "robot payload" t)"#]];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn switched_buffer_macro_restores_original_after_signal_without_killing_target() {
    let elisp_form = r##"
(let ((before
       (current-buffer))
      (target
       (generate-new-buffer
        " *assess-robot-signal*"))
      condition)
  (unwind-protect
      (progn
        (setq condition
              (condition-case data
                  (assess-robot-with-switched-buffer
                      target
                    (insert "before signal")
                    (signal
                     'error
                     '("robot failure")))
                (error data)))
        (list
         condition
         (eq
          before
          (current-buffer))
         (buffer-live-p target)
         (with-current-buffer target
           (buffer-string))))
    (kill-buffer target)))
"##;
    let expect: Expect = expect![[r#"OK ((error "robot failure") t t "before signal")"#]];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn temporary_switched_buffer_enables_undo_then_kills_buffer_and_restores_selection() {
    let elisp_form = r##"
(let ((before
       (current-buffer))
      escaped
      observed)
  (setq observed
        (assess-robot-with-temp-switched-buffer
          (setq escaped
                (current-buffer))
          (insert "one")
          (insert " two")
          (list
           (buffer-name)
           (buffer-string)
           (listp buffer-undo-list)
           (eq
            (current-buffer)
            escaped))))
  (list
   observed
   (buffer-live-p escaped)
   (eq before
       (current-buffer))))
"##;
    let expect: Expect = expect![[r#"OK ((" *temp*" "one two" t t) nil t)"#]];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn temporary_switched_buffer_is_killed_even_when_body_signals() {
    let elisp_form = r##"
(let ((before
       (current-buffer))
      escaped
      condition)
  (setq condition
        (condition-case data
            (assess-robot-with-temp-switched-buffer
              (setq escaped
                    (current-buffer))
              (insert "transient")
              (signal
               'error
               '("temporary failure")))
          (error data)))
  (list
   condition
   (buffer-live-p escaped)
   (eq before
       (current-buffer))))
"##;
    let expect: Expect = expect![[r#"OK ((error "temporary failure") nil t)"#]];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn switched_buffer_string_and_keyboard_macro_execute_a_practical_edit_sequence() {
    let elisp_form = r##"
(let ((last-kbd-macro
       [ignore]))
  (list
   (assess-robot-with-switched-buffer-string
     (insert "alpha beta")
     (goto-char (point-min))
     (assess-robot-execute-kmacro
      "M-d gamma SPC C-y"))
   (key-description last-kbd-macro)
   (vectorp last-kbd-macro)))
"##;
    let expect: Expect = expect!["OK (\"gamma alpha beta\" \"M-d g a m m a SPC C-y\" t)"];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn copy_and_finish_extracts_edited_macro_text_into_kill_ring_and_finishes_editor() {
    let elisp_form = r##"
(let ((kill-ring nil)
      (finish-calls 0))
  (with-temp-buffer
    (insert
     "Keyboard Macro Editor.  Press C-c C-c to finish; press C-c C-k to finish and call.\n\nMacro:\nC-a hello RET\nC-e !\n")
    (goto-char (point-max))
    (cl-letf
        (((symbol-function
           'edmacro-finish-edit)
          (lambda ()
            (setq finish-calls
                  (1+ finish-calls))
            :finished)))
      (list
       (assess-robot-copy-and-finish)
       finish-calls
       (car kill-ring)
       (current-buffer)
       (point)))))
"##;
    let expect: Expect =
        expect![[r#"OK (:finished 1 "\"C-a hello RET\nC-e !\n\"" (:buffer nil) 112)"#]];
    assert_assess_robot_parity(elisp_form, expect);
}

#[test]
fn loading_edmacro_registers_the_robot_copy_key_binding() {
    let elisp_form = r##"
(progn
  (require 'edmacro)
  (list
   (lookup-key
    edmacro-mode-map
    (kbd "C-c C-k"))
   (commandp
    'assess-robot-copy-and-finish)
   (featurep 'assess-robot)))
"##;
    let expect: Expect = expect!["OK (assess-robot-copy-and-finish t t)"];
    assert_assess_robot_parity(elisp_form, expect);
}
