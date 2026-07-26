use super::assert_ack_menu_parity;
use expect_test::expect;

#[test]
fn ack_menu_mode_initializes_exact_keymap_locals_hook_and_navigation_state() {
    let elisp_form = r##"(progn
         (defvar ack-menu-test-hook-runs)
         (let ((ack-menu-test-hook-runs
                0)
               (ack-mode-hook
                (list
                 (lambda ()
                   (setq ack-menu-test-hook-runs
                         (1+
                          ack-menu-test-hook-runs))))))
           (with-temp-buffer
             (setq overlay-arrow-position
                   (copy-marker
                    1))
             (ack-mode)
             (list
              ack-menu-test-hook-runs
              major-mode
              mode-name
              font-lock-extra-managed-props
              (local-variable-p
               'font-lock-extra-managed-props)
              (local-variable-p
               'overlay-arrow-position)
              overlay-arrow-string
              (local-variable-p
               'overlay-arrow-string)
              next-error-function
              ack-error-pos
              (local-variable-p
               'ack-error-pos)
              (copy-tree
               (current-local-map))
              (keymap-parent
               (current-local-map))))))"##;
    let expect = expect![[
        r#"OK (1 ack-mode "ack" (mouse-face follow-link ack-line ack-file ack-marker ack-match) t t "" t ack-next-error-function nil t (keymap (114 . ack-again) (103 . ack-again) (27 keymap (112 . ack-previous-file) (110 . ack-next-file)) (112 . ack-previous-match) (110 . ack-next-match) (13 . ack-find-match) (mouse-2 . ack-find-match)) nil)"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_again_uses_local_buffer_name_global_name_or_interactive_fallback() {
    let elisp_form = r##"(let ((ack-buffer-name
                "*ack-global*")
               calls)
         (cl-letf
             (((symbol-function
                'ack-run-impl)
               (lambda (&rest arguments)
                 (push
                  (list
                   'run
                   ack-buffer-name
                   arguments)
                  calls)
                 'ran))
              ((symbol-function
                'call-interactively)
               (lambda (function)
                 (push
                  (list
                   'interactive
                   function)
                  calls)
                 'called)))
           (list
            (with-temp-buffer
              (rename-buffer
               "*ack-local-fixture*")
              (setq-local
               ack-buffer--rerun-args
               '("/local/"
                 "--match=one"))
              (list
               (ack--again-buffer-name)
               (ack-again)))
            (with-temp-buffer
              (let ((ack-buffer--rerun-args
                     '("/global/"
                       "--match=two")))
                (list
                 (ack--again-buffer-name)
                 (ack-again))))
            (with-temp-buffer
              (let ((ack-buffer--rerun-args
                     nil))
                (ack-again)))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("*ack-local-fixture*" ran) ("*ack-global*" ran) called ((run "*ack-local-fixture*" ("/local/" "--match=one")) (run "*ack-global*" ("/global/" "--match=two")) (interactive ack)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}
