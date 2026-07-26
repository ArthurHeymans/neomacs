use expect_test::expect;

use super::{assert_ac_capf_parity, assert_ac_capf_signal_parity};

#[test]
fn ac_capf_candidates_short_circuits_nil_response_and_nil_prefix_before_completion_work() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ac-capf--candidates-response)
                     (lambda ()
                       (push 'response events)
                       (and ac-prefix
                            '(provider 1 1 table))))
                    ((symbol-function
                      'completion-metadata)
                     (lambda (&rest _)
                       (push 'metadata events)
                       nil))
                    ((symbol-function
                      'completion-all-completions)
                     (lambda (&rest _)
                       (push 'completion events)
                       '("candidate"))))
                 (list
                  (let ((ac-prefix nil))
                    (ac-capf--candidates))
                  (let ((ac-prefix "x"))
                    (cl-letf
                        (((symbol-function
                           'ac-capf--candidates-response)
                          (lambda ()
                            (push 'nil-response events)
                            nil)))
                      (ac-capf--candidates)))
                  (nreverse events))))"##;
    let expect = expect!["OK (nil nil (response nil-response))"];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_forwards_buffer_prefix_table_predicate_and_prefix_length_exactly() {
    let elisp_form = r##"(let ((table '(alpha beta))
                    (predicate
                     (lambda (candidate)
                       (eq candidate 'alpha)))
                    (ac-prefix "pre")
                    events)
               (cl-letf
                   (((symbol-function
                      'ac-capf--candidates-response)
                     (lambda ()
                       (list
                        'provider 2 5 table
                        :annotation-function 'ignored
                        :predicate predicate)))
                    ((symbol-function
                      'completion-metadata)
                     (lambda
                       (string actual-table actual-predicate)
                       (push
                        (list
                         'metadata string
                         (eq actual-table table)
                         (eq actual-predicate predicate))
                        events)
                       '(metadata)))
                    ((symbol-function
                      'completion-all-completions)
                     (lambda
                       (string actual-table actual-predicate point)
                       (push
                        (list
                         'all string
                         (eq actual-table table)
                         (eq actual-predicate predicate)
                         point)
                        events)
                       (list
                        (propertize
                         "first"
                         'face 'bold)
                        (propertize
                         "second"
                         'category 'fixture)))))
                 (with-temp-buffer
                   (insert "xword!")
                   (list
                    (ac-capf--candidates)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (("first" "second") ((metadata "wor" t t) (all "pre" t t 3)))"#]];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_strips_every_text_property_from_normal_completion_results() {
    let elisp_form = r##"(let ((ac-prefix "x"))
               (cl-letf
                   (((symbol-function
                      'ac-capf--candidates-response)
                     (lambda ()
                       '(provider 1 1 table)))
                    ((symbol-function
                      'completion-metadata)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'completion-all-completions)
                     (lambda (&rest _)
                       (list
                        (propertize
                         "alpha" 0 5
                         'face 'bold
                         'fixture 1)
                        ""
                        (propertize
                         "βeta" 0 1
                         'category 'unicode)))))
                 (let ((result
                        (with-temp-buffer
                          (ac-capf--candidates))))
                   (list
                    result
                    (mapcar
                     (lambda (candidate)
                       (text-properties-at
                        0 candidate))
                     result)))))"##;
    let expect = expect![[r#"OK (("alpha" "" "βeta") (nil nil nil))"#]];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_removes_a_zero_dotted_base_tail_before_returning_candidates() {
    let elisp_form = r##"(let ((ac-prefix "x"))
               (cl-letf
                   (((symbol-function
                      'ac-capf--candidates-response)
                     (lambda ()
                       '(provider 1 1 table)))
                    ((symbol-function
                      'completion-metadata)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'completion-all-completions)
                     (lambda (&rest _)
                       (cons
                        (propertize
                         "alpha" 'face 'bold)
                        (cons "beta" 0)))))
                 (ac-capf--candidates)))"##;
    let expect = expect![[r#"OK ("alpha" "beta")"#]];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_prepends_the_dynamic_arg_prefix_for_nonzero_base_size() {
    let elisp_form = r##"(let ((ac-prefix "pha"))
               (cl-letf
                   (((symbol-function
                      'ac-capf--candidates-response)
                     (lambda ()
                       '(provider 1 1 table)))
                    ((symbol-function
                      'completion-metadata)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'completion-all-completions)
                     (lambda (&rest _)
                       (cons "pha"
                             (cons "pine" 2)))))
                 (cl-progv '(arg) '("alphabet")
                   (ac-capf--candidates))))"##;
    let expect = expect![[r#"OK ("alpha" "alpine")"#]];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_candidates_nonzero_base_size_exposes_the_upstream_unbound_arg_error() {
    let elisp_form = r##"(progn
               (makunbound 'arg)
               (let ((ac-prefix "pha"))
                 (cl-letf
                     (((symbol-function
                        'ac-capf--candidates-response)
                       (lambda ()
                         '(provider 1 1 table)))
                      ((symbol-function
                        'completion-metadata)
                       (lambda (&rest _)
                         nil))
                      ((symbol-function
                        'completion-all-completions)
                       (lambda (&rest _)
                         (cons "pha"
                               (cons "pine" 2)))))
                   (ac-capf--candidates))))"##;
    let expect = expect!["ERR (void-variable arg)"];

    assert_ac_capf_signal_parity(elisp_form, expect);
}
