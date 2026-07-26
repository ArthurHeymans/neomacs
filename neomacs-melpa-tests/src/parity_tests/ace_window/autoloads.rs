use super::assert_ace_window_autoload_parity;
use expect_test::expect;

#[test]
fn ace_window_autoload_file_registers_all_public_entry_points_without_loading_sources() {
    let elisp_form = r##"(let ((symbols
              '(ace-select-window
                ace-delete-window
                ace-swap-window
                ace-delete-other-windows
                ace-display-buffer
                ace-window
                ace-window-display-mode
                ace-window-posframe-mode)))
         (list
          (featurep 'ace-window-autoloads)
          (featurep 'ace-window)
          (featurep 'ace-window-posframe)
          (mapcar
           (lambda (symbol)
             (let ((function
                    (symbol-function symbol)))
               (list
                symbol
                (autoloadp function)
                (nth 1 function)
                (nth 3 function)
                (nth 4 function)
                (commandp symbol))))
           symbols)
          (bound-and-true-p
           ace-window-display-mode)
          (bound-and-true-p
           ace-window-posframe-mode)
          (copy-sequence
           (gethash "ace-window"
                    definition-prefixes))
          (copy-sequence
           (gethash "ace-window-posframe"
                    definition-prefixes))))"##;
    let expect = expect![[
        r#"OK (t nil nil ((ace-select-window t "ace-window" t nil t) (ace-delete-window t "ace-window" t nil t) (ace-swap-window t "ace-window" t nil t) (ace-delete-other-windows t "ace-window" t nil t) (ace-display-buffer t "ace-window" nil nil nil) (ace-window t "ace-window" t nil t) (ace-window-display-mode t "ace-window" t nil t) (ace-window-posframe-mode t "ace-window-posframe" t nil t)) nil nil nil nil)"#
    ]];
    assert_ace_window_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_window_command_autoload_loads_main_source_and_dispatches_argument() {
    let elisp_form = r##"(progn
         (require 'avy)
         (setq ace-window--test-events nil
               avy-current-path
               "prebound")
         (autoload-do-load
          (symbol-function 'ace-window)
          'ace-window)
         (cl-letf
             (((symbol-function
                'ace-select-window)
               (lambda ()
                 (push 'select
                       ace-window--test-events)
                 'select-result))
              ((symbol-function
                'ace-swap-window)
               (lambda ()
                 (push 'swap
                       ace-window--test-events)
                 'swap-result)))
           (list
            (ace-window 4)
            (featurep 'ace-window)
            avy-current-path
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect![[r#"OK (swap-result t "" (swap))"#]];
    assert_ace_window_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_window_posframe_mode_autoload_loads_both_sources_and_enables_backend() {
    let elisp_form = r##"(progn
         (setq
          ace-window--test-events
          nil
          ace-window--test-original-require
          (symbol-function 'require))
         (cl-letf
             (((symbol-function 'require)
               (lambda
                   (feature
                    &optional filename noerror)
                 (if (eq feature 'posframe)
                     (progn
                       (push 'require-posframe
                             ace-window--test-events)
                       t)
                   (funcall
                    ace-window--test-original-require
                    feature
                    filename
                    noerror))))
              ((symbol-function
                'posframe-workable-p)
               (lambda ()
                 (push 'workable
                       ace-window--test-events)
                 t)))
           (unwind-protect
               (progn
                 (ace-window-posframe-mode
                  1)
                 (list
                  (featurep 'ace-window)
                  (featurep
                   'ace-window-posframe)
                  ace-window-posframe-mode
                  (eq
                   aw--lead-overlay-fn
                   #'aw--lead-overlay-posframe)
                  (eq
                   aw--remove-leading-chars-fn
                   #'aw--remove-leading-chars-posframe)
                  (nreverse
                   ace-window--test-events)))
             (when
                 (fboundp
                  'ace-window-posframe-mode)
               (ace-window-posframe-mode
                -1)))))"##;
    let expect = expect!["OK (t t t t t (require-posframe workable))"];
    assert_ace_window_autoload_parity(elisp_form, expect);
}
