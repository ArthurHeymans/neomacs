use super::assert_ace_mc_parity;
use expect_test::expect;

#[test]
fn ace_mc_single_cursor_workflow_adds_at_jump_target_and_restores_origin() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha beta gamma")
         (goto-char 2)
         (let ((map (make-sparse-keymap))
               (overriding-local-map nil)
               (ace-jump-mode-submode-list
                '(ace-mc-fixture-jump)))
           (setq ace-mc--test-events nil)
           (cl-letf
               (((symbol-function 'use-region-p)
                 (lambda () nil))
                ((symbol-function 'mc--reset-read-prompts)
                 (lambda ()
                   (push 'reset-prompts
                         ace-mc--test-events)))
                ((symbol-function 'read-char)
                 (lambda (prompt)
                   (push (list 'read prompt)
                         ace-mc--test-events)
                   ?b))
                ((symbol-function 'ace-mc-fixture-jump)
                 (lambda (query)
                   (push
                    (list 'jump
                          query
                          ace-jump-mode-scope
                          ace-mc-marking)
                    ace-mc--test-events)
                   (run-hooks
                    'ace-jump-mode-before-jump-hook)
                   (goto-char 8)
                   (setq overriding-local-map map)
                   (run-hooks 'ace-jump-mode-end-hook)
                   'jump-result))
                ((symbol-function 'overlays-at)
                 (lambda (point)
                   (push (list 'overlays point)
                         ace-mc--test-events)
                   nil))
                ((symbol-function 'mc/create-fake-cursor-at-point)
                 (lambda ()
                   (push (list 'create (point))
                         ace-mc--test-events)
                   'created))
                ((symbol-function 'mc/maybe-multiple-cursors-mode)
                 (lambda ()
                   (push 'maybe-mode
                         ace-mc--test-events))))
             (list
              (ace-mc-add-single-cursor 1)
              (point)
              ace-mc-marking
              ace-mc-saved-point
              ace-mc-query-char
              (lookup-key map (kbd "C-c C-c"))
              (lookup-key map [t])
              (nreverse ace-mc--test-events)))))"##;
    let expect = expect![[
        r#"OK (ace-mc-do-keyboard-reset 2 nil 2 98 ace-mc-quick-exchange ace-mc-do-keyboard-reset (reset-prompts (read "Query Char:") (jump 98 window t) (overlays 8) (create 8) maybe-mode))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_single_cursor_workflow_removes_existing_cursor_and_restores_origin() {
    let elisp_form = r##"(with-temp-buffer
         (insert "alpha beta gamma")
         (goto-char 3)
         (let ((ace-jump-mode-submode-list
                '(ace-mc-fixture-jump)))
           (setq ace-mc--test-events nil)
           (cl-letf
               (((symbol-function 'use-region-p)
                 (lambda () nil))
                ((symbol-function 'mc--reset-read-prompts)
                 (lambda ()
                   (push 'reset-prompts
                         ace-mc--test-events)))
                ((symbol-function 'read-char)
                 (lambda (_prompt) ?b))
                ((symbol-function 'ace-mc-fixture-jump)
                 (lambda (_query)
                   (run-hooks
                    'ace-jump-mode-before-jump-hook)
                   (goto-char 8)
                   (run-hooks 'ace-jump-mode-end-hook)
                   'jump-result))
                ((symbol-function 'overlays-at)
                 (lambda (point)
                   (push (list 'overlays point)
                         ace-mc--test-events)
                   '(ordinary existing-cursor)))
                ((symbol-function 'mc/fake-cursor-p)
                 (lambda (overlay)
                   (eq overlay 'existing-cursor)))
                ((symbol-function 'mc/remove-fake-cursor)
                 (lambda (overlay)
                   (push (list 'remove overlay)
                         ace-mc--test-events)))
                ((symbol-function 'mc/create-fake-cursor-at-point)
                 (lambda ()
                   (push 'unexpected-create
                         ace-mc--test-events)))
                ((symbol-function 'mc/maybe-multiple-cursors-mode)
                 (lambda ()
                   (push 'maybe-mode
                         ace-mc--test-events))))
             (list
              (ace-mc-add-single-cursor 1)
              (point)
              ace-mc-marking
              ace-mc-saved-point
              ace-mc-query-char
              (nreverse ace-mc--test-events)))))"##;
    let expect = expect![
        "OK (nil 3 nil 3 98 (reset-prompts (overlays 8) (remove existing-cursor) maybe-mode))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}
