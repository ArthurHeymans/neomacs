use super::{assert_ace_mc_parity, assert_ace_mc_signal_parity};
use expect_test::expect;

#[test]
fn ace_mc_reset_clears_only_the_marking_flag() {
    let elisp_form = r##"(let ((ace-mc-marking 'active)
             (ace-mc-keyboard-reset 'keyboard)
             (ace-mc-query-char ?q)
             (ace-mc-loop-marking 'loop)
             (ace-mc-saved-point 7)
             (ace-mc-ace-mode-function 'fixture-mode))
         (list
          (ace-mc-reset)
          ace-mc-marking
          ace-mc-keyboard-reset
          ace-mc-query-char
          ace-mc-loop-marking
          ace-mc-saved-point
          ace-mc-ace-mode-function))"##;
    let expect = expect!["OK (nil nil keyboard 113 loop 7 fixture-mode)"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_keyboard_reset_clears_marking_before_finishing_ace_jump() {
    let elisp_form = r##"(let ((events nil)
             (ace-mc-marking t))
         (cl-letf
             (((symbol-function 'ace-jump-done)
               (lambda ()
                 (push
                  (list 'done ace-mc-marking)
                  events)
                 'done-result)))
           (list
            (ace-mc-do-keyboard-reset)
            ace-mc-marking
            (nreverse events))))"##;
    let expect = expect!["OK (done-result nil ((done nil)))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_quick_exchange_switches_word_and_char_modes_but_preserves_other_modes() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (let ((events nil)
                 (ace-jump-current-mode
                  (car fixture))
                 (ace-mc-ace-mode-function
                  (cdr fixture))
                 (ace-mc-query-char ?q))
             (cl-letf
                 (((symbol-function 'ace-jump-done)
                   (lambda ()
                     (push
                      (list 'done
                            ace-mc-ace-mode-function)
                      events)))
                  ((symbol-function 'ace-mc-add-char)
                   (lambda (query)
                     (push
                      (list 'add
                            query
                            ace-mc-ace-mode-function)
                      events)
                     'add-result)))
               (list
                fixture
                (ace-mc-quick-exchange)
                ace-mc-ace-mode-function
                (nreverse events)))))
         '((ace-jump-word-mode . original-word)
           (ace-jump-char-mode . original-char)
           (ace-jump-line-mode . original-line)))"##;
    let expect = expect![
        "OK (((ace-jump-word-mode . original-word) add-result ace-jump-char-mode ((done ace-jump-char-mode) (add 113 ace-jump-char-mode))) ((ace-jump-char-mode . original-char) add-result ace-jump-word-mode ((done ace-jump-word-mode) (add 113 ace-jump-word-mode))) ((ace-jump-line-mode . original-line) add-result original-line ((done original-line) (add 113 original-line))))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_regexp_mode_quotes_literal_text_before_ace_jump() {
    let elisp_form = r##"(let ((events nil))
         (cl-letf
             (((symbol-function 'ace-jump-do)
               (lambda (regexp)
                 (push regexp events)
                 'jump-result)))
           (list
            (ace-mc-regexp-mode "a+b[c].*")
            (nreverse events))))"##;
    let expect = expect![[r#"OK (jump-result ("a\\+b\\[c]\\.\\*"))"#]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_add_char_passes_query_with_window_scope_and_installs_reset_bindings() {
    let elisp_form = r##"(let ((map (make-sparse-keymap))
             (ace-jump-mode-scope 'global)
             (ace-mc-marking nil)
             (ace-mc-query-char nil)
             (ace-mc-ace-mode-function 'ace-mc-fixture-mode))
         (setq ace-mc--test-events nil)
         (cl-letf
             (((symbol-function 'ace-mc-fixture-mode)
               (lambda (query)
                 (push
                  (list 'mode
                        query
                        ace-jump-mode-scope
                        ace-mc-marking
                        ace-mc-query-char)
                  ace-mc--test-events)
                 'mode-result)))
           (let ((overriding-local-map map))
             (list
              (ace-mc-add-char ?z)
              ace-jump-mode-scope
              ace-mc-marking
              ace-mc-query-char
              (lookup-key map (kbd "C-c C-c"))
              (lookup-key map [t])
              (nreverse ace-mc--test-events)))))"##;
    let expect = expect![
        "OK (ace-mc-do-keyboard-reset global t 122 ace-mc-quick-exchange ace-mc-do-keyboard-reset ((mode 122 window t 122)))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_add_char_calls_zero_argument_mode_and_leaves_absent_map_untouched() {
    let elisp_form = r##"(let ((overriding-local-map nil)
             (ace-jump-mode-scope 'global)
             (ace-mc-marking nil)
             (ace-mc-query-char 'old)
             (ace-mc-ace-mode-function 'ace-mc-fixture-mode))
         (setq ace-mc--test-events nil)
         (cl-letf
             (((symbol-function 'ace-mc-fixture-mode)
               (lambda ()
                 (push
                  (list 'mode
                        ace-jump-mode-scope
                        ace-mc-marking
                        ace-mc-query-char)
                  ace-mc--test-events)
                 'mode-result)))
           (list
            (ace-mc-add-char nil)
            ace-jump-mode-scope
            ace-mc-marking
            ace-mc-query-char
            overriding-local-map
            (nreverse ace-mc--test-events))))"##;
    let expect = expect!["OK (nil global t nil nil ((mode window t nil)))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_multiple_cursor_command_maps_prefixes_and_clamps_to_available_modes() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (let ((events nil)
                 (ace-jump-mode-submode-list
                  '(fixture-word
                    fixture-char
                    fixture-line)))
             (cl-letf
                 (((symbol-function 'use-region-p)
                   (lambda () nil))
                  ((symbol-function 'mc--reset-read-prompts)
                   (lambda ()
                     (push 'reset-prompts events)))
                  ((symbol-function 'read-char)
                   (lambda (prompt)
                     (push (list 'read prompt) events)
                     ?q))
                  ((symbol-function 'ace-mc-add-char)
                   (lambda (query)
                     (push
                      (list 'add
                            query
                            ace-mc-ace-mode-function
                            ace-mc-loop-marking)
                      events)
                     'add-result)))
               (list
                prefix
                (ace-mc-add-multiple-cursors
                 prefix
                 nil)
                ace-mc-ace-mode-function
                ace-mc-loop-marking
                (nreverse events)))))
         '(0 1 2 4 16 64))"##;
    let expect = expect![[
        r#"OK ((0 add-result fixture-word t (reset-prompts (read "Query Char:") (add 113 fixture-word t))) (1 add-result fixture-word t (reset-prompts (read "Query Char:") (add 113 fixture-word t))) (2 add-result fixture-word t (reset-prompts (read "Query Char:") (add 113 fixture-word t))) (4 add-result fixture-char t (reset-prompts (read "Query Char:") (add 113 fixture-char t))) (16 add-result fixture-line t (reset-prompts (read "Query Char:") (add 113 fixture-line t))) (64 add-result fixture-line t (reset-prompts (read "Query Char:") (add 113 fixture-line t))))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_single_mode_suppresses_looping_and_line_mode_suppresses_character_prompt() {
    let elisp_form = r##"(let ((events nil)
             (ace-jump-mode-submode-list
              '(fixture-word
                fixture-char
                ace-jump-line-mode)))
         (cl-letf
             (((symbol-function 'use-region-p)
               (lambda () nil))
              ((symbol-function 'mc--reset-read-prompts)
               (lambda ()
                 (push 'reset-prompts events)))
              ((symbol-function 'read-char)
               (lambda (prompt)
                 (push (list 'unexpected-read prompt)
                       events)
                 ?q))
              ((symbol-function 'ace-mc-add-char)
               (lambda (query)
                 (push
                  (list 'add
                        query
                        ace-mc-ace-mode-function
                        ace-mc-loop-marking)
                  events)
                 'add-result)))
           (list
            (ace-mc-add-multiple-cursors 16 t)
            ace-mc-ace-mode-function
            ace-mc-loop-marking
            (nreverse events))))"##;
    let expect = expect![
        "OK (add-result ace-jump-line-mode nil (reset-prompts (add nil ace-jump-line-mode nil)))"
    ];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_multiple_cursor_command_uses_active_region_text_as_literal_regexp_query() {
    let elisp_form = r##"(with-temp-buffer
         (insert "before a+b[c] after")
         (goto-char 8)
         (set-mark 14)
         (activate-mark)
         (let ((events nil)
               (ace-jump-mode-submode-list
                '(fixture-word
                  fixture-char
                  fixture-line)))
           (cl-letf
               (((symbol-function 'mc--reset-read-prompts)
                 (lambda ()
                   (push 'reset-prompts events)))
                ((symbol-function 'mc/execute-command-for-all-fake-cursors)
                 (lambda (command)
                   (push
                    (list 'fake-cursors command)
                    events)))
                ((symbol-function 'ace-mc-add-char)
                 (lambda (query)
                   (push
                    (list 'add
                          query
                          ace-mc-ace-mode-function
                          ace-mc-loop-marking)
                    events)
                   'add-result)))
             (list
              (ace-mc-add-multiple-cursors 1 nil)
              ace-mc-ace-mode-function
              ace-mc-loop-marking
              mark-active
              (point)
              (mark)
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (add-result ace-mc-regexp-mode t nil 8 14 (reset-prompts (add "a+b[c]" ace-mc-regexp-mode t)))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_multiple_cursor_command_normalizes_a_reversed_active_region_for_all_cursors() {
    let elisp_form = r##"(with-temp-buffer
         (insert "before target after")
         (goto-char 14)
         (set-mark 8)
         (activate-mark)
         (let ((events nil)
               (ace-jump-mode-submode-list
                '(fixture-word
                  fixture-char
                  fixture-line)))
           (cl-letf
               (((symbol-function 'mc--reset-read-prompts)
                 (lambda ()
                   (push 'reset-prompts events)))
                ((symbol-function 'mc/execute-command-for-all-fake-cursors)
                 (lambda (command)
                   (push
                    (list 'fake-cursors
                          command
                          (point)
                          (mark))
                    events)))
                ((symbol-function 'ace-mc-add-char)
                 (lambda (query)
                   (push
                    (list 'add
                          query
                          (point)
                          (mark)
                          mark-active)
                    events)
                   'add-result)))
             (list
              (ace-mc-add-multiple-cursors 4 t)
              ace-mc-ace-mode-function
              ace-mc-loop-marking
              mark-active
              (point)
              (mark)
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (add-result ace-mc-regexp-mode nil nil 8 14 (reset-prompts (fake-cursors exchange-point-and-mark 8 14) (add "target" 8 14 nil)))"#
    ]];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_single_cursor_command_forwards_prefix_and_true_single_mode() {
    let elisp_form = r##"(let ((events nil))
         (cl-letf
             (((symbol-function
                'ace-mc-add-multiple-cursors)
               (lambda (prefix single-mode)
                 (push
                  (list prefix single-mode)
                  events)
                 'multiple-result)))
           (list
            (ace-mc-add-single-cursor 16)
            (nreverse events))))"##;
    let expect = expect!["OK (multiple-result ((16 t)))"];
    assert_ace_mc_parity(elisp_form, expect);
}

#[test]
fn ace_mc_multiple_cursor_command_rejects_a_nil_prefix_like_gnu() {
    let elisp_form = r##"(let ((ace-jump-mode-submode-list
              '(fixture-word)))
         (ace-mc-add-multiple-cursors nil nil))"##;
    let expect = expect!["ERR (wrong-type-argument numberp nil)"];
    assert_ace_mc_signal_parity(elisp_form, expect);
}

#[test]
fn ace_mc_multiple_cursor_command_rejects_an_empty_submode_list_like_gnu() {
    let elisp_form = r##"(let ((ace-jump-mode-submode-list nil))
         (cl-letf
             (((symbol-function 'use-region-p)
               (lambda () nil))
              ((symbol-function 'mc--reset-read-prompts)
               (lambda () nil))
              ((symbol-function 'read-char)
               (lambda (_prompt) ?q)))
           (ace-mc-add-multiple-cursors 1 nil)))"##;
    let expect = expect!["ERR (void-function nil)"];
    assert_ace_mc_signal_parity(elisp_form, expect);
}
