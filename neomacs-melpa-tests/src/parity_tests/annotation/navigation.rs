use expect_test::expect;

use super::assert_annotation_parity;

#[test]
fn goto_visits_readable_file_at_exact_position_and_signals_missing_target() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (directory
                            (expand-file-name
                             "annotation-navigation"
                             sandbox))
                           (target
                            (expand-file-name
                             "Target.agda"
                             directory))
                           (missing
                            (expand-file-name
                             "Missing.agda"
                             directory))
                           target-buffer)
                      (make-directory
                       directory t)
                      (with-temp-file target
                        (insert
                         "0123456789\n"))
                      (unwind-protect
                          (list
                           (annotation-goto
                            nil)
                           (annotation-goto
                            (cons target 6))
                           (progn
                             (setq target-buffer
                                   (current-buffer))
                             (list
                              buffer-file-name
                              (point)
                              (char-after)))
                           (condition-case error-data
                               (annotation-goto
                                (cons missing 2))
                             (error error-data)))
                        (when
                            (buffer-live-p
                             target-buffer)
                          (kill-buffer
                           target-buffer))))"##;
    let expect = expect![[
        r#"OK (nil t ("[ORACLE-SANDBOX]/annotation-navigation/Target.agda" 6 53) (error "File does not exist or is unreadable: [ORACLE-SANDBOX]/annotation-navigation/Missing.agda."))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn goto_dispatches_normal_and_other_window_paths_without_touching_real_windows() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function
                             'file-readable-p)
                            (lambda (_)
                              t))
                           ((symbol-function
                             'find-file)
                            (lambda (file)
                              (push
                               (list 'same file)
                               calls)))
                           ((symbol-function
                             'find-file-other-window)
                            (lambda (file)
                              (push
                               (list 'other file)
                               calls)))
                           ((symbol-function
                             'goto-char)
                            (lambda (position)
                              (push
                               (list
                                'position
                                position)
                               calls))))
                        (list
                         (annotation-goto
                          '("One.agda" . 11))
                         (annotation-goto
                          '("Two.agda" . 27)
                          t)
                         (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (t t ((same "One.agda") (position 11) (other "Two.agda") (position 27)))"#]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn goto_and_push_records_real_source_location_only_after_successful_movement() {
    let elisp_form = r##"(let* ((sandbox
                            (getenv
                             "NEOMACS_TEST_SANDBOX_ROOT"))
                           (directory
                            (expand-file-name
                             "annotation-stack"
                             sandbox))
                           (source-file
                            (expand-file-name
                             "Source.agda"
                             directory))
                           (target-file
                            (expand-file-name
                             "Target.agda"
                             directory))
                           (source
                            (generate-new-buffer
                             " *annotation-source*"))
                           target-buffer
                           (annotation-goto-stack
                            nil))
                      (make-directory
                       directory t)
                      (with-temp-file target-file
                        (insert
                         "target contents"))
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (setq buffer-file-name
                                    source-file)
                              (insert "source")
                              (goto-char 4)
                              (annotation-goto-and-push
                               source
                               (point)
                               (cons
                                target-file
                                8)))
                            (setq target-buffer
                                  (get-file-buffer
                                   target-file))
                            (let ((after-real
                                   (list
                                    annotation-goto-stack
                                    (with-current-buffer
                                        target-buffer
                                      (list
                                       buffer-file-name
                                       (point))))))
                              (setq annotation-goto-stack
                                    nil)
                              (with-current-buffer source
                                (goto-char 2)
                                (cl-letf
                                    (((symbol-function
                                       'annotation-goto)
                                      (lambda
                                          (&rest _)
                                        t)))
                                  (list
                                   after-real
                                   (annotation-goto-and-push
                                    source 2
                                    '("Same.agda"
                                      . 2))
                                   annotation-goto-stack
                                   (cl-letf
                                       (((symbol-function
                                          'annotation-goto)
                                         (lambda
                                             (&rest _)
                                           nil)))
                                     (annotation-goto-and-push
                                      source 3
                                      '("Failed.agda"
                                        . 3)))
                                   annotation-goto-stack)))))
                        (kill-buffer source)
                        (when
                            (buffer-live-p
                             target-buffer)
                          (kill-buffer
                           target-buffer))))"##;
    let expect = expect![[
        r#"OK (((("[ORACLE-SANDBOX]/annotation-stack/Source.agda" . 4)) ("[ORACLE-SANDBOX]/annotation-stack/Target.agda" 8)) t nil nil nil)"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn go_back_pops_history_in_lifo_order_and_is_noop_when_empty() {
    let elisp_form = r##"(let ((annotation-goto-stack
                           '(("Newest.agda"
                              . 9)
                             ("Oldest.agda"
                              . 2)))
                          calls)
                      (cl-letf
                          (((symbol-function
                             'annotation-goto)
                            (lambda (filepos
                                     &optional
                                     other-window)
                              (push
                               (list
                                filepos
                                other-window)
                               calls)
                              t)))
                        (list
                         (annotation-go-back)
                         (annotation-go-back)
                         (annotation-go-back)
                         (nreverse calls)
                         annotation-goto-stack)))"##;
    let expect =
        expect![[r#"OK (t t nil ((("Newest.agda" . 9) nil) (("Oldest.agda" . 2) nil)) nil)"#]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn indirect_integer_link_preserves_event_classification_and_documented_integer_branch() {
    let elisp_form = r##"(let ((source
                          (generate-new-buffer
                           " *annotation-indirect*"))
                         calls)
                      (unwind-protect
                          (with-current-buffer source
                            (insert "abcdef")
                            (put-text-property
                             3 5
                             'annotation-goto
                             '("Definition.agda"
                               . 44))
                            (cl-letf
                                (((symbol-function
                                   'annotation-goto-and-push)
                                  (lambda
                                      (source-buffer
                                       source-pos
                                       target
                                       &optional
                                       other-window)
                                    (push
                                     (list
                                      (buffer-name
                                       source-buffer)
                                      source-pos
                                      target
                                      other-window)
                                     calls)
                                    t)))
                              (list
                               (annotation-goto-indirect
                                1)
                               (annotation-goto-indirect
                                3 t)
                               (cl-letf
                                   (((symbol-function
                                      'eventp)
                                     (lambda (_)
                                       nil)))
                                 (annotation-goto-indirect
                                  3 t))
                               (condition-case error-data
                                   (annotation-goto-indirect
                                    "not-a-link")
                                 (error
                                  error-data))
                               (nreverse calls))))
                        (kill-buffer source)))"##;
    let expect = expect![[
        r#"OK (nil nil t (error "Not an integer or event object: \"not-a-link\"") ((" *annotation-indirect*" 3 ("Definition.agda" . 44) t)))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn indirect_event_uses_event_window_position_selection_and_ignores_non_text_areas() {
    let elisp_form = r##"(let ((source
                          (generate-new-buffer
                           " *annotation-event*"))
                         (event-area nil)
                         selected calls)
                      (unwind-protect
                          (progn
                            (with-current-buffer source
                              (insert "abcdef")
                              (put-text-property
                               2 4
                               'annotation-goto
                               '("Event.agda"
                                 . 12)))
                            (cl-letf
                                (((symbol-function
                                   'eventp)
                                  (lambda (value)
                                    (eq
                                     value
                                     'mock-event)))
                                 ((symbol-function
                                   'event-end)
                                  (lambda (_)
                                    'mock-position))
                                 ((symbol-function
                                   'posn-area)
                                  (lambda (_)
                                    event-area))
                                 ((symbol-function
                                   'posn-point)
                                  (lambda (_)
                                    2))
                                 ((symbol-function
                                   'posn-window)
                                  (lambda (_)
                                    'source-window))
                                 ((symbol-function
                                   'window-buffer)
                                  (lambda (_)
                                    source))
                                 ((symbol-function
                                   'selected-window)
                                  (lambda ()
                                    'other-window))
                                 ((symbol-function
                                   'select-window)
                                  (lambda (window)
                                    (setq selected
                                          window)))
                                 ((symbol-function
                                   'annotation-goto-and-push)
                                  (lambda
                                      (source-buffer
                                       source-pos
                                       target
                                       &optional
                                       other-window)
                                    (push
                                     (list
                                      (buffer-name
                                       source-buffer)
                                      source-pos
                                      target
                                      other-window)
                                     calls)
                                    t)))
                              (let ((text-result
                                     (annotation-goto-indirect
                                      'mock-event t)))
                                (setq event-area
                                      'mode-line)
                                (list
                                 text-result
                                 selected
                                 (annotation-goto-indirect
                                  'mock-event)
                                 (nreverse calls)))))
                        (kill-buffer source)))"##;
    let expect =
        expect![[r#"OK (t source-window nil ((" *annotation-event*" 2 ("Event.agda" . 12) t)))"#]];
    assert_annotation_parity(elisp_form, expect);
}
