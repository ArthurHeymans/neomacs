use super::assert_ace_pinyin_autoload_parity;
use expect_test::expect;

#[test]
fn ace_pinyin_autoload_file_registers_public_commands_without_loading_source() {
    let elisp_form = r##"(let ((symbols
              '(ace-pinyin-jump-word
                ace-pinyin-dwim
                ace-pinyin-mode
                ace-pinyin-global-mode
                turn-on-ace-pinyin-mode
                turn-off-ace-pinyin-mode)))
         (list
          (featurep 'ace-pinyin-autoloads)
          (featurep 'ace-pinyin)
          (mapcar
           (lambda (symbol)
             (let ((function
                    (symbol-function symbol)))
               (list
                symbol
                (autoloadp function)
                (nth 1 function)
                (nth 3 function)
                (nth 4 function))))
           symbols)
          (get 'ace-pinyin-global-mode
               'globalized-minor-mode)
          (bound-and-true-p
           ace-pinyin-global-mode)
          (copy-sequence
           (gethash "ace-pinyin-"
                    definition-prefixes))
          (fboundp 'ace-pinyin--jump-impl)))"##;
    let expect = expect![[
        r#"OK (t nil ((ace-pinyin-jump-word t "ace-pinyin" t nil) (ace-pinyin-dwim t "ace-pinyin" t nil) (ace-pinyin-mode t "ace-pinyin" t nil) (ace-pinyin-global-mode t "ace-pinyin" t nil) (turn-on-ace-pinyin-mode t "ace-pinyin" t nil) (turn-off-ace-pinyin-mode t "ace-pinyin" t nil)) t nil ("ace-pinyin" "ace-pinyin") nil)"#
    ]];
    assert_ace_pinyin_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_pinyin_autoload_dwim_loads_source_and_forwards_prompted_character() {
    let elisp_form = r##"(progn
         (require 'avy)
         (require 'pinyinlib)
         (setq ace-pinyin--test-events nil)
         (cl-letf
             (((symbol-function 'read-char)
               (lambda (prompt)
                 (push (list 'read prompt)
                       ace-pinyin--test-events)
                 ?z))
              ((symbol-function
                'pinyinlib-build-regexp-char)
               (lambda (query no-punctuation traditional prefix)
                 (push
                  (list 'build
                        query
                        no-punctuation
                        traditional
                        prefix)
                  ace-pinyin--test-events)
                 "fixture-regexp"))
              ((symbol-function 'avy-jump)
               (lambda (&rest arguments)
                 (push (cons 'jump arguments)
                       ace-pinyin--test-events)
                 'jump-result)))
           (list
            (ace-pinyin-dwim t)
            (featurep 'ace-pinyin)
            (nreverse ace-pinyin--test-events))))"##;
    let expect = expect![[r#"OK (t t ((read "char: ") (build 122 nil nil t)))"#]];
    assert_ace_pinyin_autoload_parity(elisp_form, expect);
}
