use expect_test::expect;

use super::assert_ac_helm_parity;

#[test]
fn ac_helm_command_invokes_auto_complete_when_idle_then_starts_exact_helm_source() {
    let elisp_form = r##"(with-temp-buffer
               (insert "prefix")
               (goto-char
                (point-max))
               (let ((ac-completing nil)
                     (ac-point 2)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'call-interactively)
                       (lambda (command
                                &optional record keys)
                         (push
                          (list
                           'call-interactively
                           command record keys)
                          calls)
                         (setq ac-completing t)
                         'completed))
                      ((symbol-function
                        'helm)
                       (lambda (&rest arguments)
                         (push
                          (cons 'helm arguments)
                          calls)
                         'helm-result)))
                   (list
                    (ac-complete-with-helm)
                    (nreverse calls)
                    (point)
                    (buffer-string)
                    (interactive-form
                     #'ac-complete-with-helm)))))"##;
    let expect = expect![[
        r#"OK (helm-result ((call-interactively auto-complete nil nil) (helm :sources helm-source-auto-complete-candidates :buffer "*helm auto-complete*")) 7 "prefix" (interactive nil))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_command_skips_auto_complete_when_already_completing() {
    let elisp_form = r##"(with-temp-buffer
               (insert "prefix")
               (goto-char
                (point-max))
               (let ((ac-completing t)
                     (ac-point 1)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'call-interactively)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'unexpected arguments)
                          calls)))
                      ((symbol-function
                        'helm)
                       (lambda (&rest arguments)
                         (push
                          (cons 'helm arguments)
                          calls)
                         'helm-result)))
                   (list
                    (ac-complete-with-helm)
                    (nreverse calls)
                    (point)))))"##;
    let expect = expect![[
        r#"OK (helm-result ((helm :sources helm-source-auto-complete-candidates :buffer "*helm auto-complete*")) 7)"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_command_uses_the_live_point_after_auto_complete_returns() {
    let elisp_form = r##"(with-temp-buffer
               (insert "abcdef")
               (goto-char 4)
               (let ((ac-completing nil)
                     (ac-point 1)
                     helm-point)
                 (cl-letf
                     (((symbol-function
                        'call-interactively)
                       (lambda (_command
                                &rest _arguments)
                         (setq ac-point 3)
                         (goto-char 6)))
                      ((symbol-function
                        'helm)
                       (lambda (&rest _arguments)
                         (setq helm-point
                               (point))
                         'helm-result)))
                   (list
                    (ac-complete-with-helm)
                    helm-point
                    (point)
                    ac-point))))"##;
    let expect = expect!["OK (helm-result 6 6 3)"];

    assert_ac_helm_parity(elisp_form, expect);
}
