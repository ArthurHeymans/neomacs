use expect_test::expect;

use super::{assert_ac_mozc_parity, assert_ac_mozc_signal_parity};

#[test]
fn ac_mozc_action_removes_only_the_single_qualifying_space_and_clears_saved_point() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (nth 0 fixture))
                   (goto-char
                    (point-max))
                   (let ((ac-mozc-ac-point
                          (nth 1 fixture))
                         (ac-mozc-remove-space
                          (nth 2 fixture)))
                     (list
                      fixture
                      (ac-mozc-action)
                      (point)
                      (buffer-string)
                      ac-mozc-ac-point))))
               '(("Emacs 日本"
                  7
                  t)
                 ("Emacs 日本"
                  7
                  nil)
                 ("! 日本"
                  3
                  t)
                 (" 日本"
                  2
                  t)
                 ("A  日本"
                  4
                  t)
                 ("日本 日本"
                  4
                  t)))"##;
    let expect = expect![[
        r#"OK ((("Emacs 日本" 7 t) nil 8 "Emacs日本" nil) (("Emacs 日本" 7 nil) nil 9 "Emacs 日本" nil) (("! 日本" 3 t) nil 5 "! 日本" nil) ((" 日本" 2 t) nil 4 " 日本" nil) (("A  日本" 4 t) nil 6 "A  日本" nil) (("日本 日本" 4 t) nil 5 "日本日本" nil))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_action_uses_the_saved_marker_and_preserves_caller_point_via_save_excursion() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "alpha 日本 tail")
               (goto-char
                (point-max))
               (let ((saved
                      (copy-marker 7))
                     (ac-mozc-remove-space
                      t))
                 (setq
                  ac-mozc-ac-point
                  saved)
                 (list
                  (ac-mozc-action)
                  (point)
                  (marker-position
                   saved)
                  (buffer-string)
                  ac-mozc-ac-point)))"##;
    let expect = expect![[r#"OK (nil 13 6 "alpha日本 tail" nil)"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_action_with_space_removal_enabled_requires_a_saved_completion_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "text")
               (let ((ac-mozc-remove-space
                      t)
                     (ac-mozc-ac-point
                      nil))
                 (ac-mozc-action)))"##;
    let expect = expect![[r#"ERR (wrong-type-argument integer-or-marker-p nil)"#]];

    assert_ac_mozc_signal_parity(elisp_form, expect);
}

#[test]
fn ac_mozc_action_out_of_range_point_clamps_and_clears_saved_state() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "text")
               (let ((ac-mozc-remove-space
                      t)
                     (ac-mozc-ac-point
                      99))
                 (condition-case error-data
                     (list
                      'returned
                      (ac-mozc-action)
                      (point)
                      (buffer-string)
                      ac-mozc-ac-point)
                   (error
                    (list
                     'signaled
                     error-data
                     (point)
                     (buffer-string)
                     ac-mozc-ac-point)))))"##;
    let expect = expect![[r#"OK (returned nil 5 "text" nil)"#]];

    assert_ac_mozc_parity(elisp_form, expect);
}
