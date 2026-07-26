use super::assert_ace_mc_parity;
use expect_test::expect;

#[test]
fn ace_mc_registers_its_jump_hooks_exactly_once() {
    let elisp_form = r##"(list
         (memq #'ace-mc-maybe-jump-start
               ace-jump-mode-before-jump-hook)
         (length
          (seq-filter
           (lambda (function)
             (eq function
                 #'ace-mc-maybe-jump-start))
           ace-jump-mode-before-jump-hook))
         (memq #'ace-mc-maybe-jump-end
               ace-jump-mode-end-hook)
         (length
          (seq-filter
           (lambda (function)
             (eq function
                 #'ace-mc-maybe-jump-end))
           ace-jump-mode-end-hook)))"##;
    let expect = expect!["OK ((ace-mc-maybe-jump-start) 1 (ace-mc-maybe-jump-end) 1)"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_registers_each_integration_command_to_run_once() {
    let elisp_form = r##"(let ((owned
              '(ace-mc-add-char
                ace-mc-do-keyboard-reset
                ace-mc-add-multiple-cursors
                ace-mc-add-single-cursor
                ace-mc-quick-exchange
                ace-jump-move
                ace-jump-done)))
         (list
          (-filter
           (lambda (command)
             (memq command owned))
           mc/cmds-to-run-once)
          (mapcar
           (lambda (command)
             (cons command
                   (length
                    (seq-filter
                     (lambda (entry)
                       (eq entry command))
                     mc/cmds-to-run-once))))
           owned)))"##;
    let expect = expect![
        "OK ((ace-jump-done ace-jump-move ace-mc-quick-exchange ace-mc-add-single-cursor ace-mc-add-multiple-cursors ace-mc-do-keyboard-reset ace-mc-add-char) ((ace-mc-add-char . 1) (ace-mc-do-keyboard-reset . 1) (ace-mc-add-multiple-cursors . 1) (ace-mc-add-single-cursor . 1) (ace-mc-quick-exchange . 1) (ace-jump-move . 1) (ace-jump-done . 1)))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_source_reload_does_not_duplicate_hooks_or_run_once_commands() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-mc-add-char
               'defun))
             (owned
              '(ace-mc-add-char
                ace-mc-do-keyboard-reset
                ace-mc-add-multiple-cursors
                ace-mc-add-single-cursor
                ace-mc-quick-exchange
                ace-jump-move
                ace-jump-done)))
         (load path nil t)
         (load path nil t)
         (list
          (length
           (seq-filter
            (lambda (function)
              (eq function
                  #'ace-mc-maybe-jump-start))
            ace-jump-mode-before-jump-hook))
          (length
           (seq-filter
            (lambda (function)
              (eq function
                  #'ace-mc-maybe-jump-end))
            ace-jump-mode-end-hook))
          (mapcar
           (lambda (command)
             (cons command
                   (length
                    (seq-filter
                     (lambda (entry)
                       (eq entry command))
                     mc/cmds-to-run-once))))
           owned)))"##;
    let expect = expect![
        "OK (1 1 ((ace-mc-add-char . 1) (ace-mc-do-keyboard-reset . 1) (ace-mc-add-multiple-cursors . 1) (ace-mc-add-single-cursor . 1) (ace-mc-quick-exchange . 1) (ace-jump-move . 1) (ace-jump-done . 1)))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_source_load_preserves_preexisting_hook_and_run_once_entries() {
    let elisp_form = r##"(let ((path
              (symbol-file
               'ace-mc-add-char
               'defun))
             (ace-jump-mode-before-jump-hook
              '(fixture-before))
             (ace-jump-mode-end-hook
              '(fixture-end))
             (mc/cmds-to-run-once
              '(fixture-command)))
         (load path nil t)
         (list
          ace-jump-mode-before-jump-hook
          ace-jump-mode-end-hook
          mc/cmds-to-run-once))"##;
    let expect = expect![
        "OK ((ace-mc-maybe-jump-start fixture-before) (ace-mc-maybe-jump-end fixture-end) (ace-jump-done ace-jump-move ace-mc-quick-exchange ace-mc-add-single-cursor ace-mc-add-multiple-cursors ace-mc-do-keyboard-reset ace-mc-add-char fixture-command))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}
