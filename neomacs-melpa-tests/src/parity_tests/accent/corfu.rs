use expect_test::expect;

use super::assert_accent_parity;

#[test]
fn accent_corfu_before_deletes_the_letter_and_installs_exact_buffer_local_capf() {
    let elisp_form = r##"(let ((accent-position
                    'before)
                   captured
                   (default-before
                    (default-value
                     'completion-at-point-functions)))
               (cl-letf
                   (((symbol-function
                      'completion-at-point)
                     (lambda ()
                       (setq
                        captured
                        (funcall
                         (car
                          completion-at-point-functions)))
                       'completed)))
                 (with-temp-buffer
                   (insert
                    "cat")
                   (goto-char
                    3)
                   (list
                    (accent-corfu)
                    (buffer-string)
                    (point)
                    captured
                    (local-variable-p
                     'completion-at-point-functions)
                    (length
                     completion-at-point-functions)
                    (equal
                     default-before
                     (default-value
                      'completion-at-point-functions))))))"##;
    let expect = expect![[
        r#"OK (completed "ct" 2 (2 2 ("à" "á" "â" "ä" "æ" "ã" "å" "ā") :exclusive no) t 1 t)"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_after_deletes_the_letter_after_point_and_installs_exact_capf() {
    let elisp_form = r##"(let ((accent-position
                    'after)
                   captured)
               (cl-letf
                   (((symbol-function
                      'completion-at-point)
                     (lambda ()
                       (setq
                        captured
                        (funcall
                         (car
                          completion-at-point-functions)))
                       'completed)))
                 (with-temp-buffer
                   (insert
                    "cat")
                   (goto-char
                    2)
                   (list
                    (accent-corfu)
                    (buffer-string)
                    (point)
                    captured
                    (local-variable-p
                     'completion-at-point-functions)))))"##;
    let expect = expect![[
        r#"OK (completed "ct" 2 (2 2 ("à" "á" "â" "ä" "æ" "ã" "å" "ā") :exclusive no) t)"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_capf_closure_keeps_candidates_but_reads_point_at_each_invocation() {
    let elisp_form = r##"(let ((accent-position
                    'before))
               (cl-letf
                   (((symbol-function
                      'completion-at-point)
                     (lambda ()
                       'completed)))
                 (with-temp-buffer
                   (insert
                    "a")
                   (accent-corfu)
                   (let ((capf
                          (car
                           completion-at-point-functions)))
                     (list
                      (funcall
                       capf)
                      (progn
                        (insert
                         "xy")
                        (funcall
                         capf))
                      (buffer-string)
                      (point))))))"##;
    let expect = expect![[
        r#"OK ((1 1 ("à" "á" "â" "ä" "æ" "ã" "å" "ā") :exclusive no) (3 3 ("à" "á" "â" "ä" "æ" "ã" "å" "ā") :exclusive no) "xy" 3)"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_custom_candidates_are_appended_in_exact_order() {
    let elisp_form = r##"(let ((accent-position
                    'before)
                   (accent-custom
                    '((a
                       (ă
                        á))))
                   captured)
               (cl-letf
                   (((symbol-function
                      'completion-at-point)
                     (lambda ()
                       (setq
                        captured
                        (funcall
                         (car
                          completion-at-point-functions)))
                       'completed)))
                 (with-temp-buffer
                   (insert
                    "a")
                   (list
                    (accent-corfu)
                    captured
                    (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (completed (1 1 ("à" "á" "â" "ä" "æ" "ã" "å" "ā" "ă" "á") :exclusive no) "")"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_without_diacritics_preserves_existing_capf_and_reports_message() {
    let elisp_form = r##"(let ((accent-position
                    'before)
                   completion-calls
                   messages)
               (cl-letf
                   (((symbol-function
                      'completion-at-point)
                     (lambda ()
                       (push
                        t
                        completion-calls)
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
                   (setq-local
                    completion-at-point-functions
                    '(existing-capf))
                   (list
                    (accent-corfu)
                    (buffer-string)
                    (point)
                    completion-at-point-functions
                    completion-calls
                    (nreverse
                     messages)))))"##;
    let expect = expect![[
        r#"OK ("No accented characters available" "x" 2 (existing-capf) nil ("No accented characters available"))"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_completion_errors_propagate_after_text_and_capf_mutation() {
    let elisp_form = r##"(let ((accent-position
                    'before))
               (cl-letf
                   (((symbol-function
                      'completion-at-point)
                     (lambda ()
                       (error
                        "completion failed"))))
                 (with-temp-buffer
                   (insert
                    "a")
                   (condition-case error
                       (accent-corfu)
                     (error
                      (list
                       error
                       (buffer-string)
                       (point)
                       (local-variable-p
                        'completion-at-point-functions)
                       (funcall
                        (car
                         completion-at-point-functions))))))))"##;
    let expect = expect![[
        r#"OK ((error "completion failed") "" 1 t (1 1 ("à" "á" "â" "ä" "æ" "ã" "å" "ā") :exclusive no))"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_signals_at_the_selected_buffer_boundary() {
    let elisp_form = r##"(list
               (condition-case error
                   (with-temp-buffer
                     (let ((accent-position
                            'before))
                       (accent-corfu)))
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
                       (accent-corfu)))
                 (error
                  error)))"##;
    let expect =
        expect!["OK ((wrong-type-argument characterp nil) (wrong-type-argument characterp nil))"];

    assert_accent_parity(elisp_form, expect);
}
