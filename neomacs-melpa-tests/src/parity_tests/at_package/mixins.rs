use expect_test::expect;

use super::{assert_at_parity, assert_at_signal_parity};

#[test]
fn at_soft_get_returns_configured_fallback_while_explicit_default_still_wins() {
    let elisp_form = r##"(let ((first
                    (@extend @soft-get))
                   (second
                    (@extend
                     @soft-get
                     :default-get 'soft)))
               (list
                (@ first :missing)
                (@ second :missing)
                (@ second :missing
                   :default 'explicit)
                (@ second :default-get)))"##;
    let expect = expect!["OK (nil soft explicit soft)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_immutable_rejects_assignment_with_exact_property_error() {
    let elisp_form = r##"(let ((object
                    (@extend @immutable)))
               (setf
                (@ object :blocked)
                10))"##;
    let expect = expect![[r#"ERR (error "Object is immutable, cannot set :blocked")"#]];

    assert_at_signal_parity(elisp_form, expect);
}

#[test]
fn at_immutable_disabled_setter_returns_nil_without_assigning() {
    let elisp_form = r##"(let ((object
                    (@extend
                     @immutable
                     :immutable-error nil)))
               (list
                (setf
                 (@ object :blocked)
                 10)
                (@ object :blocked
                   :default 'absent)
                (@ object
                   :immutable-error)))"##;
    let expect = expect!["OK (nil absent nil)"];

    assert_at_parity(elisp_form, expect);
}

#[test]
fn at_watchable_notifies_in_order_assigns_after_callbacks_and_unwatches() {
    let elisp_form = r##"(let (events)
               (let* ((first
                       (lambda (object
                                property new)
                         (push
                          (list
                           'first property
                           (if (eq property
                                   :watchers)
                               (length new)
                             new)
                           (let ((current
                                  (@ object property
                                     :default
                                     'absent)))
                             (if (eq property
                                     :watchers)
                                 (length current)
                               current)))
                          events)))
                      (second
                       (lambda (object
                                property new)
                         (push
                          (list
                           'second property
                           (if (eq property
                                   :watchers)
                               (length new)
                             new)
                           (let ((current
                                  (@ object property
                                     :default
                                     'absent)))
                             (if (eq property
                                     :watchers)
                                 (length current)
                               current)))
                          events)))
                      (object
                       (@extend
                        @watchable
                        :watchers
                        (list first second))))
                 (setf (@ object :foo) 1)
                 (@! object :unwatch second)
                 (setf (@ object :bar) 2)
                 (list
                  (@ object :foo)
                  (@ object :bar)
                  (length
                   (@ object :watchers))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (1 2 1 ((first :foo 1 absent) (second :foo 1 absent) (first :watchers 1 2) (second :watchers 1 2) (first :bar 2 absent)))"#
    ]];

    assert_at_parity(elisp_form, expect);
}
