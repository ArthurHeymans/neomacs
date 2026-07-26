use expect_test::expect;

use super::{assert_ac_math_parity, assert_ac_math_signal_parity};

#[test]
fn ac_math_action_latex_removes_dummy_and_unicode_suffix() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "before \\alpha α after")
               (search-backward
                " after")
               (let ((before
                      (point))
                     (return
                      (ac-math-action-latex)))
                 (list
                  return
                  before
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (nil 16 14 "before \\alpha after")"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_latex_backward_removes_prefix_name_and_dummy_then_moves_over_unicode() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "before \\alpha α after")
               (search-backward
                " after")
               (let ((return
                      (ac-math-action-latex
                       t)))
                 (list
                  return
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (t 9 "before α after")"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_latex_backward_exposes_forward_word_behavior_for_punctuation_unicode() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "before \\rightarrow → after")
               (search-backward
                " after")
               (let ((return
                      (ac-math-action-latex
                       t)))
                 (list
                  return
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (t 15 "before → after")"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_unicode_delegates_the_exact_non_nil_backward_marker() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-math-action-latex)
                     (lambda (&optional argument)
                       (push argument calls)
                       'delegated)))
                 (list
                  (ac-math-action-unicode)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (delegated (backward))"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_latex_without_dummy_preserves_buffer_and_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "\\alpha")
               (let ((before
                      (point))
                     (return
                      (ac-math-action-latex)))
                 (list
                  return
                  before
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (nil 7 7 "\\alpha")"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_latex_changes_only_the_last_completed_symbol_before_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "\\alpha α + \\beta β")
               (let ((return
                      (ac-math-action-latex)))
                 (list
                  return
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (nil 17 "\\alpha α + \\beta")"#]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_latex_backward_without_prefix_signals_search_failed() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "plain α")
               (let ((return
                      (ac-math-action-latex
                       t)))
                 (list
                  return
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"ERR (search-failed "\\\\\\(.*\\)")"#]];

    assert_ac_math_signal_parity(elisp_form, expect);
}

#[test]
fn ac_math_action_latex_uses_the_live_multi_character_dummy_regexp() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "\\alpha:::α")
               (let ((ac-math--dummy
                      "[:]+"))
                 (list
                  (ac-math-action-latex)
                  (point)
                  (buffer-string))))"##;
    let expect = expect![[r#"OK (nil 9 "\\alpha::")"#]];

    assert_ac_math_parity(elisp_form, expect);
}
