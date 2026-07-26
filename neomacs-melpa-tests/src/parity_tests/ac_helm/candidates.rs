use expect_test::expect;

use super::{assert_ac_helm_parity, assert_ac_helm_signal_parity};

#[test]
fn ac_helm_candidates_preserve_plain_identity_and_format_action_candidates() {
    let elisp_form = r##"(let* ((plain
                     (propertize
                      "plain"
                      'fixture 'plain))
                    (action
                     (propertize
                      "run"
                      'action 'fixture-action
                      'fixture 'action))
                    (ac-candidates
                     (list plain action))
                    (menu-width 7))
               (cl-letf
                   (((symbol-function
                      'helm-attr)
                     (lambda (attribute)
                       (pcase attribute
                         ('ac-candidates
                          ac-candidates)
                         ('menu-width
                          menu-width)))))
                 (let ((result
                        (helm-auto-complete-candidates)))
                   (list
                    (mapcar
                     (lambda (pair)
                       (list
                        (substring-no-properties
                         (car pair))
                        (substring-no-properties
                         (cdr pair))))
                     result)
                    (eq
                     (caar result)
                     plain)
                    (eq
                     (cdar result)
                     plain)
                    (get-text-property
                     0
                     'fixture
                     (car
                      (cadr result)))
                    (get-text-property
                     0
                     'action
                     (car
                      (cadr result)))
                    (eq
                     (cdr
                      (cadr result))
                     action)))))"##;
    let expect = expect![[
        r#"OK ((("plain" "plain") ("run     <fixture-action>" "run")) t t action fixture-action t)"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_formatted_action_candidate_text_property_order_matches_gnu() {
    let elisp_form = r##"(let ((candidate
                    (propertize
                     "run"
                     'action 'fixture-action
                     'fixture 'fixture-value)))
               (cl-letf
                   (((symbol-function
                      'helm-attr)
                     (lambda (attribute)
                       (pcase attribute
                         ('ac-candidates
                          (list candidate))
                         ('menu-width 3)))))
                 (text-properties-at
                  0
                  (caar
                   (helm-auto-complete-candidates)))))"##;
    let expect = expect!["OK (fixture fixture-value action fixture-action)"];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_candidates_cover_empty_strings_zero_width_and_function_action_names() {
    let elisp_form = r##"(let* ((empty
                     "")
                    (action
                     (propertize
                      ""
                      'action
                      (lambda ()
                        'fixture)))
                    (ac-candidates
                     (list empty action)))
               (cl-letf
                   (((symbol-function
                      'helm-attr)
                     (lambda (attribute)
                       (pcase attribute
                         ('ac-candidates
                          ac-candidates)
                         ('menu-width 0)))))
                 (helm-auto-complete-candidates)))"##;
    let expect = expect![[r#"OK (("" . "") ("" . ""))"#]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_candidates_returns_nil_for_an_empty_attribute() {
    let elisp_form = r##"(cl-letf
               (((symbol-function
                  'helm-attr)
                 (lambda (attribute)
                   (pcase attribute
                     ('ac-candidates nil)
                     ('menu-width 10)))))
               (helm-auto-complete-candidates))"##;
    let expect = expect!["OK nil"];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_candidates_signal_when_action_candidate_exceeds_menu_width() {
    let elisp_form = r##"(let ((candidate
                    (propertize
                     "long"
                     'action 'fixture)))
               (cl-letf
                   (((symbol-function
                      'helm-attr)
                     (lambda (attribute)
                       (pcase attribute
                         ('ac-candidates
                          (list candidate))
                         ('menu-width 2)))))
                 (helm-auto-complete-candidates)))"##;
    let expect = expect!["ERR (wrong-type-argument wholenump -2)"];

    assert_ac_helm_signal_parity(elisp_form, expect);
}
