use expect_test::expect;

use super::assert_accent_parity;

#[test]
fn accent_menu_before_replaces_the_character_before_point_with_selected_symbol() {
    let elisp_form = r##"(let ((accent-position
                    'before)
                   calls)
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (options)
                       (push
                        (copy-tree
                         options)
                        calls)
                       'á)))
                 (with-temp-buffer
                   (insert
                    "cat")
                   (goto-char
                    3)
                   (let ((result
                          (accent-menu)))
                     (list
                      result
                      (buffer-string)
                      (point)
                      (nreverse
                       calls))))))"##;
    let expect = expect![[r#"OK (nil "cát" 3 ((à á â ä æ ã å ā)))"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_after_replaces_the_character_after_point_with_selected_symbol() {
    let elisp_form = r##"(let ((accent-position
                    'after)
                   calls)
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (options)
                       (push
                        (copy-tree
                         options)
                        calls)
                       'ā)))
                 (with-temp-buffer
                   (insert
                    "cat")
                   (goto-char
                    2)
                   (let ((result
                          (accent-menu)))
                     (list
                      result
                      (buffer-string)
                      (point)
                      (nreverse
                       calls))))))"##;
    let expect = expect![[r#"OK (nil "cāt" 3 ((à á â ä æ ã å ā)))"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_cancel_preserves_text_point_and_popup_options_in_both_positions() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (options)
                       (push
                        (copy-tree
                         options)
                        calls)
                       nil)))
                 (list
                  (with-temp-buffer
                    (insert
                     "cat")
                    (goto-char
                     3)
                    (let ((accent-position
                           'before))
                      (list
                       (accent-menu)
                       (buffer-string)
                       (point))))
                  (with-temp-buffer
                    (insert
                     "cat")
                    (goto-char
                     2)
                    (let ((accent-position
                           'after))
                      (list
                       (accent-menu)
                       (buffer-string)
                       (point))))
                  (nreverse
                   calls))))"##;
    let expect =
        expect![[r#"OK ((nil "cat" 3) (nil "cat" 2) ((à á â ä æ ã å ā) (à á â ä æ ã å ā)))"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_uses_before_for_every_position_value_other_than_symbol_after() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (_)
                       (push
                        t
                        calls)
                       'á)))
                 (mapcar
                  (lambda (position)
                    (with-temp-buffer
                      (insert
                       "cat")
                      (goto-char
                       3)
                      (let ((accent-position
                             position))
                        (accent-menu)
                        (list
                         position
                         (buffer-string)
                         (point)))))
                  '(before
                    nil
                    "after"
                    t
                    other))))"##;
    let expect = expect![[
        r#"OK ((before "cát" 3) (nil "cát" 3) ("after" "cát" 3) (t "cát" 3) (other "cát" 3))"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_merges_custom_candidates_before_invoking_popup() {
    let elisp_form = r##"(let ((accent-position
                    'before)
                   (accent-custom
                    '((a
                       (ă))))
                   seen)
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (options)
                       (setq
                        seen
                        (copy-tree
                         options))
                       (car
                        (last
                         options)))))
                 (with-temp-buffer
                   (insert
                    "a")
                   (list
                    (accent-menu)
                    (buffer-string)
                    seen))))"##;
    let expect = expect![[r#"OK (nil "ă" (à á â ä æ ã å ā ă))"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_without_diacritics_reports_exact_message_and_never_opens_popup() {
    let elisp_form = r##"(let ((accent-position
                    'before)
                   calls
                   messages)
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'unexpected))
                    ((symbol-function
                      'message)
                     (lambda (format-string
                              &rest arguments)
                       (let ((rendered
                              (apply
                               #'format
                               format-string
                               arguments)))
                         (push
                          rendered
                          messages)
                         rendered))))
                 (with-temp-buffer
                   (insert
                    "x")
                   (list
                    (accent-menu)
                    (buffer-string)
                    (point)
                    calls
                    (nreverse
                     messages)))))"##;
    let expect = expect![[
        r#"OK ("No accented characters available" "x" 2 nil ("No accented characters available"))"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_non_symbol_popup_results_signal_after_deleting_the_original_letter() {
    let elisp_form = r##"(let ((accent-position
                    'before))
               (cl-letf
                   (((symbol-function
                      'popup-menu*)
                     (lambda (_)
                       "not-a-symbol")))
                 (with-temp-buffer
                   (insert
                    "a")
                   (condition-case error
                       (accent-menu)
                     (error
                      (list
                       error
                       (buffer-string)
                       (point)))))))"##;
    let expect = expect![[r#"OK ((wrong-type-argument symbolp "not-a-symbol") "" 1)"#]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_signals_at_the_selected_buffer_boundary() {
    let elisp_form = r##"(list
               (condition-case error
                   (with-temp-buffer
                     (let ((accent-position
                            'before))
                       (accent-menu)))
                 (error
                  error))
               (condition-case error
                   (with-temp-buffer
                     (insert
                      "a")
                     (goto-char
                      (point-max))
                     (let ((accent-position
                            'after))
                       (accent-menu)))
                 (error
                  error)))"##;
    let expect =
        expect!["OK ((wrong-type-argument characterp nil) (wrong-type-argument characterp nil))"];

    assert_accent_parity(elisp_form, expect);
}
