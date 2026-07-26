use super::assert_ace_mc_autoload_parity;
use expect_test::expect;

#[test]
fn ace_mc_autoload_file_registers_only_the_two_public_commands() {
    let elisp_form = r##"(let ((definitions
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
               '(ace-mc-add-multiple-cursors
                 ace-mc-add-single-cursor))))
         (list
          (featurep 'ace-mc-autoloads)
          (featurep 'ace-mc)
          definitions
          (autoloadp
           (symbol-function
            'ace-mc-add-multiple-cursors))
          (autoloadp
           (symbol-function
            'ace-mc-add-single-cursor))
          (fboundp 'ace-mc-add-char)
          (fboundp 'ace-mc-reset)))"##;
    let expect = expect![[
        r#"OK (t nil ((ace-mc-add-multiple-cursors t "ace-mc" t nil) (ace-mc-add-single-cursor t "ace-mc" t nil)) t t nil nil)"#
    ]];
    assert_ace_mc_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_mc_autoload_commands_load_the_package_only_when_invoked() {
    let elisp_form = r##"(progn
         (require 'ace-jump-mode)
         (require 'multiple-cursors-core)
         (require 'dash)
         (let ((events nil))
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
              ((symbol-function 'ace-jump-word-mode)
               (lambda (query)
                 (push (list 'word query) events))))
           (ace-mc-add-multiple-cursors 1 t)
           (list
            (featurep 'ace-mc)
            (nreverse events)))))"##;
    let expect = expect![[r#"OK (t (reset-prompts (read "Query Char:") (word 113)))"#]];
    assert_ace_mc_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_mc_autoload_prefix_registration_matches_the_generated_file() {
    let elisp_form = r##"(list
         (copy-sequence
          (gethash "ace-mc-"
                   definition-prefixes))
         (get 'ace-mc-add-multiple-cursors
              'function-documentation)
         (get 'ace-mc-add-single-cursor
              'function-documentation)
         (file-name-nondirectory
          (symbol-file
           'ace-mc-add-multiple-cursors
           'defun))
         (file-name-nondirectory
          (symbol-file
           'ace-mc-add-single-cursor
           'defun)))"##;
    let expect = expect![[r#"OK (("ace-mc" "ace-mc") nil nil "ace-mc.el" "ace-mc.el")"#]];
    assert_ace_mc_autoload_parity(elisp_form, expect);
}
