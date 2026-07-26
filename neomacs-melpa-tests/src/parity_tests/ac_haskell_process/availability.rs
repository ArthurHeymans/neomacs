use expect_test::expect;

use super::assert_ac_haskell_process_parity;

#[test]
fn ac_haskell_process_available_short_circuits_major_mode_for_a_live_session() {
    let elisp_form = r##"(let ((major-mode
                    'unsupported-mode)
                   calls)
               (cl-letf
                   (((symbol-function
                      'haskell-session-maybe)
                     (lambda ()
                       (push 'session calls)
                       'fixture-session)))
                 (list
                  (ac-haskell-process-available-p)
                  (nreverse calls)
                  major-mode)))"##;
    let expect = expect!["OK (fixture-session (session) unsupported-mode)"];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_available_accepts_only_both_documented_modes_without_session() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'haskell-session-maybe)
                 (lambda () nil)))
               (mapcar
                (lambda (mode)
                  (let ((major-mode mode))
                    (list
                     mode
                     (ac-haskell-process-available-p))))
                '(haskell-mode
                  haskell-interactive-mode
                  interactive-haskell-mode
                  fundamental-mode
                  nil)))"##;
    let expect = expect![
        "OK ((haskell-mode (haskell-mode . #1=(haskell-interactive-mode))) (haskell-interactive-mode #1#) (interactive-haskell-mode nil) (fundamental-mode nil) (nil nil))"
    ];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_available_preserves_non_boolean_session_identity() {
    let elisp_form = r##"(let ((session
                    (list 'session 'identity))
                   (major-mode 'fundamental-mode))
               (cl-letf
                   (((symbol-function
                      'haskell-session-maybe)
                     (lambda () session)))
                 (let ((result
                        (ac-haskell-process-available-p)))
                   (list
                    result
                    (eq result session)))))"##;
    let expect = expect!["OK ((session identity) t)"];

    assert_ac_haskell_process_parity(elisp_form, expect);
}
