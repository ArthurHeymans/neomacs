use super::{
    assert_ace_jump_zap_autoload_parity, assert_ace_jump_zap_autoload_with_prelude_parity,
};
use expect_test::expect;

#[test]
fn ace_jump_zap_fresh_autoload_provides_only_autoload_feature() {
    let elisp_form = r##"(list
         (featurep 'ace-jump-zap-autoloads)
         (featurep 'ace-jump-zap)
         (featurep 'ace-jump-mode)
         (featurep 'dash))"##;
    let expect = expect!["OK (t nil nil nil)"];
    assert_ace_jump_zap_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_fresh_autoload_commands_have_exact_objects() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((definition
                  (symbol-function symbol)))
             (list
              symbol
              (autoloadp definition)
              (nth 1 definition)
              (nth 2 definition)
              (nth 3 definition)
              (nth 4 definition)
              (commandp symbol)
              (file-name-nondirectory
               (symbol-file symbol 'defun)))))
         '(ace-jump-zap-up-to-char
           ace-jump-zap-to-char
           ace-jump-zap-to-char-dwim
           ace-jump-zap-up-to-char-dwim))"##;
    let expect = expect![[
        r#"OK ((ace-jump-zap-up-to-char t "ace-jump-zap" "Call `ace-jump-char-mode' and zap all characters up to the selected character." t nil t "ace-jump-zap.el") (ace-jump-zap-to-char t "ace-jump-zap" "Call `ace-jump-char-mode' and zap all characters up to and including the selected character." t nil t "ace-jump-zap.el") (ace-jump-zap-to-char-dwim t "ace-jump-zap" "Without PREFIX, call `zap-to-char'.\nWith PREFIX, call `ace-jump-zap-to-char'.\n\n(fn &optional PREFIX)" t nil t "ace-jump-zap.el") (ace-jump-zap-up-to-char-dwim t "ace-jump-zap" "Without PREFIX, call `zap-up-to-char'.\nWith PREFIX, call `ace-jump-zap-up-to-char'.\n\n(fn &optional PREFIX)" t nil t "ace-jump-zap.el"))"#
    ]];
    assert_ace_jump_zap_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_fresh_autoload_leaves_internal_surface_undefined() {
    let elisp_form = r##"(let ((functions
              '(ajz/maybe-zap-start
                ajz/maybe-zap-end
                ajz/reset
                ajz/keyboard-reset
                ajz/forward-query
                ajz/closeness-to-point
                ajz/maybe-limit-candidate-length
                ajz/maybe-sort-candidate-list))
             (variables
              '(ajz/zapping
                ajz/to-char
                ajz/saved-point
                ajz/zap-function
                ajz/forward-only
                ajz/sort-by-closest
                ajz/52-character-limit)))
         (list
          (delq nil
                (mapcar
                 (lambda (symbol)
                   (and
                    (fboundp symbol)
                    symbol))
                 functions))
          (delq nil
                (mapcar
                 (lambda (symbol)
                   (and
                    (boundp symbol)
                    symbol))
                 variables))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_zap_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_fresh_autoload_registers_definition_prefixes() {
    let elisp_form = r##"(list
         (gethash
          "ace-jump-zap-"
          definition-prefixes)
         (gethash
          "ajz/"
          definition-prefixes)
         (gethash
          "ace-jump-zap"
          definition-prefixes))"##;
    let expect = expect![[r#"OK (nil ("ace-jump-zap" "ace-jump-zap") nil)"#]];
    assert_ace_jump_zap_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_zap_fresh_autoload_command_loads_package_then_dispatches_builtin() {
    let elisp_form = r##"(list
         (ace-jump-zap-to-char-dwim nil)
         (featurep 'ace-jump-zap)
         (autoloadp
          (symbol-function
           'ace-jump-zap-to-char-dwim))
         (nreverse
          ace-jump-zap-test-calls))"##;
    let expect = expect!["OK (builtin-result t nil (zap-to-char))"];
    assert_ace_jump_zap_autoload_with_prelude_parity(
        r##"(progn
         (defvar ace-jump-zap-test-calls nil)
         (fset
          'zap-to-char
          (lambda ()
            (interactive)
            (setq ace-jump-zap-test-calls
                  (cons
                   'zap-to-char
                   ace-jump-zap-test-calls))
            'builtin-result)))"##,
        elisp_form,
        expect,
    );
}
