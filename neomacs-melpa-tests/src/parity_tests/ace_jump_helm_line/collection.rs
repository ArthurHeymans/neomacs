use super::assert_ace_jump_helm_line_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_collect_lines_includes_each_plain_line_and_preserves_restriction() {
    let elisp_form = r##"(with-temp-buffer
               (insert "zero\none\ntwo\nthree\nend\n")
               (narrow-to-region 6 20)
               (goto-char 11)
               (let ((original-point (point))
                     (original-min (point-min))
                     (original-max (point-max)))
                 (cl-letf
                     (((symbol-function
                        'helm-pos-header-line-p)
                       (lambda () nil))
                      ((symbol-function
                        'helm-pos-candidate-separator-p)
                       (lambda () nil))
                      ((symbol-function
                        'selected-window)
                       (lambda () 'selected-window)))
                   (list
                    (ace-jump-helm-line--collect-lines
                     (point-min)
                     (point-max))
                    ace-jump-helm-line--last-win-start
                    (point)
                    original-point
                    (point-min)
                    original-min
                    (point-max)
                    original-max))))"##;
    let expect = expect![
        "OK (((6 . selected-window) (10 . selected-window) (14 . selected-window)) 6 11 11 6 6 20 20)"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_collect_lines_skips_leading_headers_and_internal_separators() {
    let elisp_form = r##"(with-temp-buffer
               (insert "H\nA\nS\nB\n")
               (cl-letf
                   (((symbol-function
                      'helm-pos-header-line-p)
                     (lambda ()
                       (= (point) 1)))
                    ((symbol-function
                      'helm-pos-candidate-separator-p)
                     (lambda ()
                       (= (point) 5)))
                    ((symbol-function
                      'selected-window)
                     (lambda () 'helm-window)))
                 (list
                  (ace-jump-helm-line--collect-lines 1 (point-max))
                  ace-jump-helm-line--last-win-start
                  (point)
                  (point-min)
                  (point-max))))"##;
    let expect = expect!["OK (((3 . helm-window) (7 . helm-window)) 1 9 1 9)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_collect_lines_honors_an_explicit_partial_window_end() {
    let elisp_form = r##"(with-temp-buffer
               (insert "one\ntwo\nthree\nfour\n")
               (cl-letf
                   (((symbol-function
                      'helm-pos-header-line-p)
                     (lambda () nil))
                    ((symbol-function
                      'helm-pos-candidate-separator-p)
                     (lambda () nil))
                    ((symbol-function
                      'selected-window)
                     (lambda () 'helm-window)))
                 (ace-jump-helm-line--collect-lines 5 15)))"##;
    let expect = expect!["OK ((5 . helm-window) (9 . helm-window))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_collect_lines_computes_default_end_from_screen_lines() {
    let elisp_form = r##"(with-temp-buffer
               (insert "one\ntwo\nthree\nfour\n")
               (let (events)
                 (cl-letf
                     (((symbol-function
                        'window-screen-lines)
                       (lambda ()
                         (push
                          (list 'screen-lines (point))
                          events)
                         2))
                      ((symbol-function
                        'helm-pos-header-line-p)
                       (lambda () nil))
                      ((symbol-function
                        'helm-pos-candidate-separator-p)
                       (lambda () nil))
                      ((symbol-function
                        'selected-window)
                       (lambda () 'helm-window)))
                   (list
                    (ace-jump-helm-line--collect-lines 5)
                    (nreverse events)
                    ace-jump-helm-line--last-win-start))))"##;
    let expect = expect!["OK (((5 . helm-window) (9 . helm-window)) ((screen-lines 5)) 5)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_collect_lines_returns_nil_for_an_empty_region() {
    let elisp_form = r##"(with-temp-buffer
               (insert "one\n")
               (cl-letf
                   (((symbol-function
                      'helm-pos-header-line-p)
                     (lambda () nil))
                    ((symbol-function
                      'helm-pos-candidate-separator-p)
                     (lambda () nil))
                    ((symbol-function
                      'selected-window)
                     (lambda () 'unused)))
                 (list
                  (ace-jump-helm-line--collect-lines 2 2)
                  ace-jump-helm-line--last-win-start)))"##;
    let expect = expect!["OK (nil 2)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
