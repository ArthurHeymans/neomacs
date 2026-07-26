use super::{assert_ace_link_parity, assert_ace_link_signal_parity};
use expect_test::expect;

#[test]
fn ace_link_dispatches_the_first_matching_major_mode_action() {
    let elisp_form = r##"(let ((major-mode
              'fixture-major-mode)
             (ace-link-major-mode-actions
              '((fixture-first other-mode fixture-major-mode)
                (fixture-second fixture-major-mode)))
             (ace-link-minor-mode-actions nil)
             (ace-link-fallback-function nil)
             events)
         (cl-letf (((symbol-function 'fixture-first)
                    (lambda ()
                      (push 'first events)
                      'first-result))
                   ((symbol-function 'fixture-second)
                    (lambda ()
                      (push 'second events)
                      'second-result)))
           (list
            (ace-link)
            (nreverse events))))"##;
    let expect = expect!["OK (first-result (first))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_major_mode_dispatch_precedes_enabled_minor_modes_and_fallback() {
    let elisp_form = r##"(let ((major-mode
              'fixture-major-mode)
             (fixture-minor-mode t)
             (ace-link-major-mode-actions
              '((fixture-major fixture-major-mode)))
             (ace-link-minor-mode-actions
              '((fixture-minor fixture-minor-mode)))
             (ace-link-fallback-function
              (lambda ()
                'fallback))
             events)
         (cl-letf (((symbol-function 'fixture-major)
                    (lambda ()
                      (push 'major events)
                      'major-result))
                   ((symbol-function 'fixture-minor)
                    (lambda ()
                      (push 'minor events)
                      'minor-result)))
           (list
            (ace-link)
            (nreverse events))))"##;
    let expect = expect!["OK (major-result (major))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_minor_dispatch_uses_the_ambient_minor_mode_binding_like_upstream() {
    let elisp_form = r##"(let ((major-mode
              'fixture-major-mode)
             (ace-link-major-mode-actions nil)
             (ace-link-minor-mode-actions
              '((fixture-minor
                 fixture-disabled
                 fixture-enabled)
                (fixture-later fixture-enabled)))
             (ace-link-fallback-function nil)
             events)
         (cl-letf (((symbol-function 'fixture-minor)
                    (lambda ()
                      (push 'minor events)
                      'minor-result))
                   ((symbol-function 'fixture-later)
                    (lambda ()
                      (push 'later events)
                      'later-result)))
           (cl-progv
               '(minor-mode
                 fixture-disabled
                 fixture-enabled)
               '(t nil enabled)
             (list
              (ace-link)
              (nreverse events)))))"##;
    let expect = expect!["OK (minor-result (minor))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_minor_dispatch_treats_a_bound_nil_ambient_minor_mode_as_enabled() {
    let elisp_form = r##"(let (events)
         (let ((major-mode
                'fixture-major-mode)
               (ace-link-major-mode-actions nil)
               (ace-link-minor-mode-actions
                '((fixture-minor
                   fixture-enabled)))
               (ace-link-fallback-function
                (lambda ()
                  (push
                   'fallback
                   events)
                  'fallback-result)))
           (cl-letf (((symbol-function 'fixture-minor)
                      (lambda ()
                        (push
                         'minor
                         events)
                        'minor-result)))
             (cl-progv
                 '(minor-mode
                   fixture-enabled)
                 '(nil t)
               (list
                (ace-link)
                (nreverse events))))))"##;
    let expect = expect!["OK (minor-result (minor))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_minor_dispatch_skips_enabled_candidates_when_ambient_minor_mode_is_unbound() {
    let elisp_form = r##"(let (events)
         (when
             (boundp 'minor-mode)
           (makunbound 'minor-mode))
         (setq fixture-enabled t)
         (let ((major-mode
                'fixture-major-mode)
               (ace-link-major-mode-actions nil)
               (ace-link-minor-mode-actions
                '((fixture-minor
                   fixture-enabled)))
               (ace-link-fallback-function
                (lambda ()
                  (push
                   'fallback
                   events)
                  'fallback-result)))
           (cl-letf (((symbol-function 'fixture-minor)
                      (lambda ()
                        (push
                         'minor
                         events)
                        'minor-result)))
             (list
              (boundp 'minor-mode)
              (ace-link)
              (nreverse events)))))"##;
    let expect = expect!["OK (nil fallback-result (fallback))"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_calls_and_returns_a_truthy_fallback_result() {
    let elisp_form = r##"(let ((calls 0))
         (let ((major-mode
                'fixture-major-mode)
               (ace-link-major-mode-actions nil)
               (ace-link-minor-mode-actions nil)
               (ace-link-fallback-function
                (lambda ()
                  (setq calls
                        (1+ calls))
                  'fallback-result)))
           (list
            (ace-link)
            calls)))"##;
    let expect = expect!["OK (fallback-result 1)"];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_signals_for_an_unsupported_mode_without_a_fallback() {
    let elisp_form = r##"(let ((major-mode
              'fixture-major-mode)
             (ace-link-major-mode-actions nil)
             (ace-link-minor-mode-actions nil)
             (ace-link-fallback-function nil))
         (ace-link))"##;
    let expect = expect![[r#"ERR (error "fixture-major-mode isn’t supported")"#]];
    assert_ace_link_signal_parity(elisp_form, expect);
}

#[test]
fn ace_link_signals_after_a_fallback_returns_nil_exactly_once() {
    let elisp_form = r##"(let ((calls 0))
         (let ((major-mode
                'fixture-major-mode)
               (ace-link-major-mode-actions nil)
               (ace-link-minor-mode-actions nil)
               (ace-link-fallback-function
                (lambda ()
                  (setq calls
                        (1+ calls))
                  nil)))
           (condition-case error-data
               (list
                'unexpected-success
                (ace-link))
             (error
              (list
               error-data
               calls)))))"##;
    let expect = expect![[r#"OK ((error "fixture-major-mode isn’t supported") 1)"#]];
    assert_ace_link_parity(elisp_form, expect);
}
