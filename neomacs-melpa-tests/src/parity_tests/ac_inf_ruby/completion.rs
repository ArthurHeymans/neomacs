use expect_test::expect;

use super::{assert_ac_inf_ruby_parity, assert_ac_inf_ruby_signal_parity};

#[test]
fn ac_inf_ruby_candidates_forwards_the_exact_prefix_once_and_preserves_result_identity() {
    let elisp_form = r##"(let ((ac-prefix
                    "object.méthod")
                   (result
                    '("alpha" "βeta" nil))
                   events)
               (cl-letf
                   (((symbol-function
                      'inf-ruby-completions)
                     (lambda (prefix)
                       (push prefix events)
                       result)))
                 (let ((returned
                        (ac-inf-ruby-candidates)))
                   (list
                    returned
                    (eq returned result)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (("alpha" "βeta" nil) t ("object.méthod"))"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_candidates_forwards_nil_prefix_without_normalization() {
    let elisp_form = r##"(let ((ac-prefix nil)
                   observed)
               (cl-letf
                   (((symbol-function
                      'inf-ruby-completions)
                     (lambda (prefix)
                       (setq observed prefix)
                       '(nil-prefix-result))))
                 (list
                  (ac-inf-ruby-candidates)
                  observed)))"##;
    let expect = expect![[r#"OK ((nil-prefix-result) nil)"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_candidates_propagates_the_exact_completion_signal() {
    let elisp_form = r##"(let ((ac-prefix
                    'fixture-prefix))
               (cl-letf
                   (((symbol-function
                      'inf-ruby-completions)
                     (lambda (_prefix)
                       (signal
                        'wrong-type-argument
                        '(stringp
                          fixture-prefix)))))
                 (ac-inf-ruby-candidates)))"##;
    let expect = expect![[r#"ERR (wrong-type-argument stringp fixture-prefix)"#]];

    assert_ac_inf_ruby_signal_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_prefix_short_circuits_bounds_outside_a_top_level_prompt() {
    let elisp_form = r##"(let ((inf-ruby-at-top-level-prompt-p
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'inf-ruby-completion-bounds-of-expr-at-point)
                     (lambda ()
                       (setq calls
                             (1+ (or calls 0)))
                       (error
                        "bounds must not run"))))
                 (list
                  (ac-inf-ruby-prefix)
                  calls)))"##;
    let expect = expect![[r#"OK (nil nil)"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_prefix_returns_the_first_bound_for_cons_list_and_nil_results() {
    let elisp_form = r##"(let ((inf-ruby-at-top-level-prompt-p
                    'top-level)
                   (bounds
                    '((4 . 11)
                      (9 14)
                      nil))
                   events)
               (cl-letf
                   (((symbol-function
                      'inf-ruby-completion-bounds-of-expr-at-point)
                     (lambda ()
                       (let ((next
                              (car bounds)))
                         (setq bounds
                               (cdr bounds))
                         (push next events)
                         next))))
                 (list
                  (ac-inf-ruby-prefix)
                  (ac-inf-ruby-prefix)
                  (ac-inf-ruby-prefix)
                  bounds
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (4 9 nil nil ((4 . 11) (9 14) nil))"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_available_accepts_only_the_exact_inf_ruby_major_mode_symbol() {
    let elisp_form = r##"(mapcar
               (lambda (mode)
                 (let ((major-mode mode))
                   (ac-inf-ruby-available)))
               '(inf-ruby-mode
                 ruby-mode
                 fundamental-mode
                 nil))"##;
    let expect = expect![[r#"OK (t nil nil nil)"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_available_rejects_a_non_symbol_major_mode_binding_like_gnu() {
    let elisp_form = r##"(let ((major-mode
                    "inf-ruby-mode"))
               (ac-inf-ruby-available))"##;
    let expect = expect![[r#"ERR (wrong-type-argument symbolp "inf-ruby-mode")"#]];

    assert_ac_inf_ruby_signal_parity(elisp_form, expect);
}
