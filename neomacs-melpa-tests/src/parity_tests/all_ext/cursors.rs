use expect_test::expect;

use super::{assert_all_ext_parity, assert_all_ext_signal};

#[test]
fn all_ext_multiple_cursors_use_each_real_match_position_and_restore_primary_cursor() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "From helm-swoop\n"
                       "--------\n"
                       "alpha target one\n"
                       "beta target two\n"
                       "gamma target three\n")
                      (goto-char (point-min))
                      (while (search-forward "target" nil t)
                        (put-text-property
                         (match-beginning 0)
                         (match-end 0)
                         'face 'match))
                      (let (fake-cursors mode-events)
                        (cl-letf
                            (((symbol-function
                               'mc/create-fake-cursor-at-point)
                              (lambda ()
                                (push
                                 (list
                                  (point)
                                  (thing-at-point 'word t)
                                  (line-number-at-pos))
                                 fake-cursors)))
                             ((symbol-function
                               'multiple-cursors-mode)
                              (lambda (&optional argument)
                                (push
                                 (list
                                  argument
                                  (point)
                                  (thing-at-point 'word t))
                                 mode-events)
                                t)))
                          (mc/edit-lines-in-all)
                          (list
                           (point)
                           (thing-at-point 'word t)
                           (line-number-at-pos)
                           (nreverse fake-cursors)
                           (nreverse mode-events)))))"##;
    let expect =
        expect![[r#"OK (32 "target" 3 ((48 "target" 4) (65 "target" 5)) ((nil 32 "target")))"#]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_multiple_cursors_without_match_faces_use_each_result_line_start() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "From anything-occur\n"
                       "--------\n"
                       "first result\n"
                       "second result\n"
                       "third result\n")
                      (let (positions enabled)
                        (cl-letf
                            (((symbol-function
                               'mc/create-fake-cursor-at-point)
                              (lambda ()
                                (push
                                 (list
                                  (point)
                                  (line-number-at-pos)
                                  (thing-at-point 'line t))
                                 positions)))
                             ((symbol-function
                               'multiple-cursors-mode)
                              (lambda (&optional _)
                                (setq enabled t))))
                          (mc/edit-lines-in-all)
                          (list
                           (point)
                           (line-number-at-pos)
                           (thing-at-point 'line t)
                           (nreverse positions)
                           enabled))))"##;
    let expect = expect![[
        r#"OK (30 3 "first result\n" ((43 4 "second result\n") (57 5 "third result\n")) t)"#
    ]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_multiple_cursors_empty_results_signal_on_nil_primary_cursor() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "From helm-occur\n--------\n")
                      (let ((created 0)
                            mode-point)
                        (cl-letf
                            (((symbol-function
                               'mc/create-fake-cursor-at-point)
                              (lambda ()
                                (setq created (1+ created))))
                             ((symbol-function
                               'multiple-cursors-mode)
                              (lambda (&optional _)
                                (setq mode-point (point)))))
                          (mc/edit-lines-in-all)
                          (list
                           created mode-point
                           (point) (eobp)))))"##;
    let expect = expect!["ERR (wrong-type-argument integer-or-marker-p nil)"];
    assert_all_ext_signal(elisp_form, expect);
}
