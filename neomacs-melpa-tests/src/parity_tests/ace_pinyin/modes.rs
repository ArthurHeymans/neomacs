use super::assert_ace_pinyin_parity;
use expect_test::expect;

#[test]
fn ace_pinyin_mode_avy_backend_remaps_and_restores_all_character_and_word_commands() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy t)
             (ace-pinyin-treat-word-as-char t)
             (originals
              (mapcar
               (lambda (symbol)
                 (cons symbol
                       (symbol-function symbol)))
               '(avy-goto-char
                 avy-goto-char-2
                 avy-goto-char-in-line
                 avy-goto-word-0
                 avy-goto-word-1
                 avy-goto-subword-0
                 avy-goto-subword-1))))
         (ace-pinyin-mode +1)
         (let ((enabled
                (mapcar
                 (lambda (entry)
                   (list
                    (car entry)
                    (symbol-function
                     (car entry))
                    (eq (symbol-function
                         (car entry))
                        (cdr entry))))
                 originals)))
           (ace-pinyin-mode -1)
           (list
            enabled
            (mapcar
             (lambda (entry)
               (list
                (car entry)
                (eq (symbol-function
                     (car entry))
                    (cdr entry))))
             originals)
            ace-pinyin-mode)))"##;
    let expect = expect![
        "OK (((avy-goto-char ace-pinyin-jump-char nil) (avy-goto-char-2 ace-pinyin-jump-char-2 nil) (avy-goto-char-in-line ace-pinyin-jump-char-in-line nil) (avy-goto-word-0 ace-pinyin-goto-word-0 nil) (avy-goto-word-1 ace-pinyin-goto-word-1 nil) (avy-goto-subword-0 ace-pinyin-goto-subword-0 nil) (avy-goto-subword-1 ace-pinyin-goto-subword-1 nil)) ((avy-goto-char t) (avy-goto-char-2 t) (avy-goto-char-in-line t) (avy-goto-word-0 t) (avy-goto-word-1 t) (avy-goto-subword-0 t) (avy-goto-subword-1 t)) nil)"
    ];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_mode_avy_backend_can_leave_word_commands_unmapped() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy t)
             (ace-pinyin-treat-word-as-char nil)
             (original-char
              (symbol-function 'avy-goto-char))
             (original-word-0
              (symbol-function 'avy-goto-word-0))
             (original-word-1
              (symbol-function 'avy-goto-word-1))
             (original-subword-0
              (symbol-function
               'avy-goto-subword-0))
             (original-subword-1
              (symbol-function
               'avy-goto-subword-1)))
         (ace-pinyin-mode +1)
         (let ((enabled
                (list
                 (eq (indirect-function
                      'avy-goto-char)
                     (symbol-function
                      'ace-pinyin-jump-char))
                 (eq (symbol-function
                      'avy-goto-word-0)
                     original-word-0)
                 (eq (symbol-function
                      'avy-goto-word-1)
                     original-word-1)
                 (eq (symbol-function
                      'avy-goto-subword-0)
                     original-subword-0)
                 (eq (symbol-function
                      'avy-goto-subword-1)
                     original-subword-1))))
           (ace-pinyin-mode -1)
           (list
            enabled
            (eq (symbol-function
                 'avy-goto-char)
                original-char)
            (eq (symbol-function
                 'avy-goto-word-0)
                original-word-0)
            ace-pinyin-mode)))"##;
    let expect = expect!["OK ((t t t t t) t t nil)"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_mode_ace_backend_remaps_and_restores_only_char_command() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy nil)
             (ace-pinyin-treat-word-as-char t)
             (ace-pinyin--original-ace
              'ace-pinyin-fixture-original)
             (ace-pinyin--original-ace-word
              'ace-pinyin-fixture-original-word))
         (fset 'ace-jump-char-mode
               'ace-pinyin-fixture-current)
         (fset 'ace-jump-word-mode
               'ace-pinyin-fixture-current-word)
         (ace-pinyin-mode +1)
         (let ((enabled
                (list
                 (symbol-function
                  'ace-jump-char-mode)
                 (symbol-function
                  'ace-jump-word-mode))))
           (ace-pinyin-mode -1)
           (list
            enabled
            (symbol-function
             'ace-jump-char-mode)
            (symbol-function
             'ace-jump-word-mode)
            ace-pinyin-mode)))"##;
    let expect = expect![
        "OK ((ace-pinyin-jump-char ace-pinyin-fixture-current-word) ace-pinyin-fixture-original ace-pinyin-fixture-current-word nil)"
    ];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_mode_runs_its_hook_after_each_transition() {
    let elisp_form = r##"(let ((ace-pinyin-use-avy t)
             (ace-pinyin-treat-word-as-char nil)
             (ace-pinyin-mode-hook
              (list
               (lambda ()
                 (push ace-pinyin-mode
                       ace-pinyin--test-events)))))
         (setq ace-pinyin--test-events nil)
         (list
          (ace-pinyin-mode +1)
          (ace-pinyin-mode -1)
          (nreverse ace-pinyin--test-events)
          ace-pinyin-mode
          (assq 'ace-pinyin-mode
                minor-mode-alist)))"##;
    let expect = expect![[r#"OK (t nil (t nil) nil (ace-pinyin-mode " AcePY"))"#]];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_turn_on_and_off_commands_forward_explicit_numeric_arguments() {
    let elisp_form = r##"(setq ace-pinyin--test-events nil)
       (cl-letf
           (((symbol-function 'ace-pinyin-mode)
             (lambda (argument)
               (push argument
                     ace-pinyin--test-events)
               (if (> argument 0)
                   'enabled
                 'disabled))))
         (list
          (turn-on-ace-pinyin-mode)
          (turn-off-ace-pinyin-mode)
          (nreverse ace-pinyin--test-events)))"##;
    let expect = expect!["OK (enabled disabled (1 -1))"];
    assert_ace_pinyin_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_global_mode_enable_and_disable_manage_global_state_and_buffers() {
    let elisp_form = r##"(let ((buffer
              (generate-new-buffer
               " *ace-pinyin-global*"))
             (ace-pinyin-use-avy t)
             (ace-pinyin-treat-word-as-char nil))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (fundamental-mode))
               (ace-pinyin-global-mode +1)
               (let ((enabled
                      (list
                       ace-pinyin-global-mode
                       (with-current-buffer buffer
                         ace-pinyin-mode)
                       (memq
                        'ace-pinyin-global-mode-check-buffers
                        after-change-major-mode-hook))))
                 (ace-pinyin-global-mode -1)
                 (list
                  enabled
                  ace-pinyin-global-mode
                  (with-current-buffer buffer
                    ace-pinyin-mode)
                  (memq
                   'ace-pinyin-global-mode-check-buffers
                   after-change-major-mode-hook))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect!["OK ((t t nil) nil nil nil)"];
    assert_ace_pinyin_parity(elisp_form, expect);
}
