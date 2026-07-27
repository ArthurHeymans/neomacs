use expect_test::expect;

use super::assert_apel_source_parity;

#[test]
fn static_macros_choose_branches_and_capture_compile_time_values() {
    let elisp_form = r##"(progn
                      (setq apel-static-input 7)
                      (list (macroexpand '(static-if (> apel-static-input 5)
                                           'large 'small))
                            (macroexpand '(static-when (= apel-static-input 7)
                                           (list 'matched apel-static-input)))
                            (macroexpand '(static-unless nil
                                           (list 'fallback apel-static-input)))
                            (macroexpand '(static-cond
                                           ((= apel-static-input 1) 'one)
                                           ((= apel-static-input 7) 'seven)
                                           (t 'other)))
                            (macroexpand '(static-condition-case error
                                           (/ 1 0)
                                           (arith-error (list 'caught error))))))"##;
    let expect = expect![
        "OK ('large (progn (list 'matched apel-static-input)) (progn (list 'fallback apel-static-input)) (progn 'seven) (funcall #[(error) ((list 'caught error)) nil] '(arith-error)))"
    ];
    assert_apel_source_parity("static.el", elisp_form, expect);
}

#[test]
fn static_defconst_installs_the_same_value_during_expansion_and_evaluation() {
    let elisp_form = r##"(progn
                      (makunbound 'apel-static-record)
                      (let ((expansion
                             (macroexpand
                              '(static-defconst apel-static-record
                                 (list :alpha (+ 2 3)) "record"))))
                        (list expansion
                              (boundp 'apel-static-record)
                              apel-static-record
                              (eval expansion)
                              apel-static-record
                              (documentation-property
                               'apel-static-record 'variable-documentation))))"##;
    let expect = expect![[
        r#"OK ((defconst apel-static-record '#1=(:alpha 5) "record") t #1# apel-static-record #1# "record")"#
    ]];
    assert_apel_source_parity("static.el", elisp_form, expect);
}

#[test]
fn maybe_macros_preserve_existing_bindings_and_define_only_missing_ones() {
    let elisp_form = r##"(progn
                      (fset 'apel-existing-function (lambda () :original))
                      (set 'apel-existing-variable :original)
                      (fmakunbound 'apel-new-function)
                      (makunbound 'apel-new-variable)
                      (eval '(defun-maybe apel-existing-function () :replacement))
                      (eval '(defun-maybe apel-new-function (x) (+ x 4)))
                      (eval '(defvar-maybe apel-existing-variable :replacement))
                      (eval '(defconst-maybe apel-new-variable '(1 2 3)))
                      (list (apel-existing-function)
                            (apel-new-function 6)
                            apel-existing-variable
                            apel-new-variable
                            (get 'apel-new-function 'defun-maybe)
                            (get 'apel-new-variable 'defconst-maybe)))"##;
    let expect = expect!["OK (:original 10 :original (1 2 3) t t)"];
    assert_apel_source_parity("pym.el", elisp_form, expect);
}

#[test]
fn conditional_definition_macros_build_practical_functions_and_macros() {
    let elisp_form = r##"(progn
                      (fmakunbound 'apel-platform-label)
                      (fmakunbound 'apel-platform-wrap)
                      (fmakunbound 'apel-platform-plus)
                      (eval '(defun-maybe-cond apel-platform-label (value)
                               "Choose implementation statically."
                               ((featurep 'apel) (list :apel value))
                               (t (list :fallback value))))
                      (eval '(defmacro-maybe-cond apel-platform-wrap (form)
                               ((featurep 'apel) (list 'list :wrapped form))
                               (t form)))
                      (eval '(defsubst-maybe-cond apel-platform-plus (left right)
                               ((>= emacs-major-version 24) (+ left right))
                               (t 0)))
                      (list (apel-platform-label "mail")
                            (eval '(apel-platform-wrap (+ 2 3)))
                            (apel-platform-plus 8 9)
                            (macrop 'apel-platform-wrap)
                            (get 'apel-platform-label 'defun-maybe)))"##;
    let expect = expect![[r#"OK ((:fallback "mail") 5 17 t t)"#]];
    assert_apel_source_parity("pym.el", elisp_form, expect);
}

#[test]
fn broken_facility_registry_drives_static_guards_and_checkers() {
    let elisp_form = r##"(progn
                      (eval '(broken-facility apel-known-bug
                               "legacy implementation is broken"))
                      (eval '(broken-facility apel-working-feature nil))
                      (list (get 'apel-known-bug 'broken)
                            (get 'apel-working-feature 'broken)
                            (macroexpand '(if-broken apel-known-bug
                                           'workaround 'native))
                            (macroexpand '(when-broken apel-known-bug
                                           (list :patched)))
                            (macroexpand '(unless-broken apel-working-feature
                                           (list :native)))
                            (condition-case error
                                (eval '(check-broken-facility apel-known-bug))
                              (error (list (car error)
                                           (error-message-string error))))
                            (eval '(check-broken-facility
                                    apel-working-feature))))"##;
    let expect = expect!["OK (t t 'workaround (progn (list :patched)) nil nil nil)"];
    assert_apel_source_parity("broken.el", elisp_form, expect);
}

#[test]
fn aliases_substitutions_edebug_specs_and_builtin_detection_are_observable() {
    let elisp_form = r##"(progn
                      (fmakunbound 'apel-maybe-alias)
                      (fmakunbound 'apel-maybe-subst)
                      (eval '(defalias-maybe 'apel-maybe-alias
                               (lambda (x) (* x x))))
                      (eval '(defsubst-maybe apel-maybe-subst (x) (+ x 1)))
                      (eval '(def-edebug-spec apel-maybe-alias (form)))
                      (list (apel-maybe-alias 7)
                            (apel-maybe-subst 7)
                            (get 'apel-maybe-alias 'defalias-maybe)
                            (get 'apel-maybe-alias 'edebug-form-spec)
                            (subr-fboundp 'car)
                            (subr-fboundp 'apel-maybe-alias)
                            (macrop 'def-edebug-spec)))"##;
    let expect = expect!["OK (49 8 t (form) t nil t)"];
    assert_apel_source_parity("pym.el", elisp_form, expect);
}
