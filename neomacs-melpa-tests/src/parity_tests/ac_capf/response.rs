use expect_test::expect;

use super::assert_ac_capf_parity;

#[test]
fn ac_capf_candidates_response_filters_default_tags_only_while_running_the_wrapped_hook() {
    let elisp_form = r##"(let ((original-default
                    (default-value
                     'completion-at-point-functions))
                   observed)
               (unwind-protect
                   (progn
                     (set-default
                      'completion-at-point-functions
                      '(default-first
                        tags-completion-at-point-function
                        default-last
                        tags-completion-at-point-function))
                     (with-temp-buffer
                       (setq-local
                        completion-at-point-functions
                        '(local-first t local-last))
                       (cl-letf
                           (((symbol-function
                              'run-hook-wrapped)
                             (lambda
                               (hook wrapper &rest arguments)
                               (setq observed
                                     (list
                                      hook
                                      wrapper
                                      arguments
                                      completion-at-point-functions
                                      (default-value
                                       'completion-at-point-functions)))
                               '(provider 3 7 table
                                 :predicate predicate))))
                         (list
                          (ac-capf--candidates-response)
                          observed
                          completion-at-point-functions
                          (default-value
                           'completion-at-point-functions)))))
                 (set-default
                  'completion-at-point-functions
                  original-default)))"##;
    let expect = expect![
        "OK ((provider 3 7 table :predicate predicate) (completion-at-point-functions completion--capf-wrapper (optimist) #1=(local-first t local-last) (default-first default-last)) #1# (default-first tags-completion-at-point-function default-last tags-completion-at-point-function))"
    ];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_response_applies_its_exact_minimal_shape_gate() {
    let elisp_form = r##"(let ((responses
                    '(nil
                      (provider)
                      (provider . 1)
                      (provider "1" 3 table)
                      (provider 1)
                      (provider 1 nil nil)
                      (provider 0 0 table))))
               (cl-letf
                   (((symbol-function
                      'run-hook-wrapped)
                     (lambda (&rest _)
                       (pop responses))))
                 (list
                  (ac-capf--candidates-response)
                  (ac-capf--candidates-response)
                  (ac-capf--candidates-response)
                  (ac-capf--candidates-response)
                  (ac-capf--candidates-response)
                  (ac-capf--candidates-response)
                  (ac-capf--candidates-response)
                  responses)))"##;
    let expect =
        expect!["OK (nil nil nil nil (provider 1) (provider 1 nil nil) (provider 0 0 table) nil)"];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_response_optimistically_tries_later_hooks_after_a_nonexclusive_response() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'neomacs-ac-capf-empty)
                     (lambda ()
                       (push 'empty events)
                       nil))
                    ((symbol-function
                      'neomacs-ac-capf-valid)
                     (lambda ()
                       (push 'valid events)
                       (list
                        (point-min)
                        (point-max)
                        '("alpha" "beta")
                        :exclusive 'no)))
                    ((symbol-function
                      'neomacs-ac-capf-late)
                     (lambda ()
                       (push 'late events)
                       (list
                        (point-min)
                        (point-max)
                        '("late")))))
                 (with-temp-buffer
                   (insert "ab")
                   (let ((completion-at-point-functions
                          '(neomacs-ac-capf-empty
                            neomacs-ac-capf-valid
                            neomacs-ac-capf-late)))
                     (list
                      (ac-capf--candidates-response)
                      (nreverse events))))))"##;
    let expect = expect![[r#"OK ((neomacs-ac-capf-late 1 3 ("late")) (empty valid late))"#]];

    assert_ac_capf_parity(elisp_form, expect);
}
