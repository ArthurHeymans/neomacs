use expect_test::expect;

use super::assert_annotation_parity;

#[test]
fn remove_all_annotations_preserves_foreign_properties_modification_state_undo_and_hooks() {
    let elisp_form = r##"(with-temp-buffer
                      (buffer-enable-undo)
                      (insert "abcdef")
                      (let ((annotation-bindings
                             '((name
                                . font-lock-function-name-face)))
                            (before-count 0)
                            (after-count 0))
                        (add-text-properties
                         1 7
                         '(foreign-property
                           retained))
                        (annotation-annotate
                         2 6
                         '(name)
                         t
                         "Temporary"
                         '("Target.agda" . 5))
                        (set-buffer-modified-p
                         nil)
                        (setq buffer-undo-list
                              nil)
                        (add-hook
                         'before-change-functions
                         (lambda (&rest _)
                           (cl-incf
                            before-count))
                         nil t)
                        (add-hook
                         'after-change-functions
                         (lambda (&rest _)
                           (cl-incf
                            after-count))
                         nil t)
                        (annotation-remove-annotations)
                        (list
                         (buffer-modified-p)
                         buffer-undo-list
                         before-count
                         after-count
                         (cl-loop
                          for position
                          from 1
                          below 7
                          collect
                          (list
                           (get-text-property
                            position
                            'foreign-property)
                           (get-text-property
                            position
                            'face)
                           (get-text-property
                            position
                            'annotation-annotated)
                           (get-text-property
                            position
                            'annotation-annotations))))))"##;
    let expect = expect![
        "OK (nil nil 0 0 ((retained nil nil nil) (retained nil nil nil) (retained nil nil nil) (retained nil nil nil) (retained nil nil nil) (retained nil nil nil)))"
    ];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn token_only_removal_keeps_disjoint_non_token_semantic_annotations() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdefgh")
                      (let ((annotation-bindings
                             '((token
                                . font-lock-keyword-face)
                               (semantic
                                . font-lock-type-face))))
                        (annotation-annotate
                         1 4
                         '(token)
                         t
                         "Token")
                        (annotation-annotate
                         5 9
                         '(semantic)
                         nil
                         "Semantic")
                        (annotation-remove-annotations
                         t)
                        (cl-loop
                         for position
                         from 1
                         below 9
                         collect
                         (list
                          position
                          (get-text-property
                           position
                           'face)
                          (get-text-property
                           position
                           'help-echo)
                          (get-text-property
                           position
                           'annotation-token-based)
                          (get-text-property
                           position
                           'annotation-annotated)
                          (get-text-property
                           position
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 nil nil nil nil nil) (2 nil nil nil nil nil) (3 nil nil nil nil nil) (4 nil nil nil nil nil) (5 #1=(font-lock-type-face) "Semantic" nil t #2=(annotation-annotated help-echo mouse-face annotation-faces face)) (6 #1# "Semantic" nil t #2#) (7 #1# "Semantic" nil t #2#) (8 #1# "Semantic" nil t #2#))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn overlapping_token_removal_exposes_exact_owned_property_union_behavior() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdefgh")
                      (let ((annotation-bindings
                             '((token
                                . font-lock-keyword-face)
                               (semantic
                                . font-lock-type-face))))
                        (annotation-annotate
                         1 6
                         '(semantic)
                         nil
                         "Semantic")
                        (annotation-annotate
                         3 8
                         '(token)
                         t
                         "Token")
                        (annotation-remove-annotations
                         t 2 7)
                        (cl-loop
                         for position
                         from 1
                         below 9
                         collect
                         (list
                          position
                          (get-text-property
                           position
                           'annotation-faces)
                          (get-text-property
                           position
                           'help-echo)
                          (get-text-property
                           position
                           'annotation-token-based)
                          (get-text-property
                           position
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 #1=(font-lock-type-face) "Semantic" nil #2=(annotation-annotated help-echo mouse-face annotation-faces face)) (2 #1# "Semantic" nil #2#) (3 nil nil nil nil) (4 nil nil nil nil) (5 nil nil nil nil) (6 nil nil nil nil) (7 (font-lock-keyword-face) "Token" t (annotation-annotated help-echo mouse-face annotation-token-based annotation-faces face)) (8 nil nil nil nil))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn string_object_removal_uses_zero_based_ranges_without_mutating_current_buffer() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "current")
                      (set-buffer-modified-p
                       nil)
                      (let ((text
                             (copy-sequence
                              "abcdef"))
                            (annotation-bindings
                             '((token
                                . font-lock-keyword-face)
                               (semantic
                                . font-lock-type-face))))
                        (annotation-annotate
                         0 3
                         '(token)
                         t
                         "Token"
                         nil
                         text)
                        (annotation-annotate
                         3 6
                         '(semantic)
                         nil
                         "Semantic"
                         nil
                         text)
                        (annotation-remove-annotations
                         t 1 5 text)
                        (list
                         (buffer-string)
                         (buffer-modified-p)
                         (cl-loop
                          for position
                          from 0
                          below (length text)
                          collect
                          (list
                           position
                           (get-text-property
                            position
                            'font-lock-face
                            text)
                           (get-text-property
                            position
                            'help-echo
                            text)
                           (get-text-property
                            position
                            'annotation-token-based
                            text)
                           (get-text-property
                            position
                            'annotation-annotations
                            text))))))"##;
    let expect = expect![[
        r#"OK ("current" nil ((0 #1=(font-lock-keyword-face) "Token" t (annotation-annotated help-echo mouse-face annotation-token-based annotation-faces face)) (1 #1# nil nil nil) (2 #1# nil nil nil) (3 #2=(font-lock-type-face) "Semantic" nil #3=(annotation-annotated help-echo mouse-face annotation-faces face)) (4 #2# "Semantic" nil #3#) (5 #2# "Semantic" nil #3#)))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn preserve_macro_keeps_state_and_undo_on_success_and_error_without_rolling_back_edits() {
    let elisp_form = r##"(list
                      (with-temp-buffer
                        (buffer-enable-undo)
                        (insert "abc")
                        (set-buffer-modified-p
                         nil)
                        (setq buffer-undo-list
                              nil)
                        (let ((hook-count 0))
                          (add-hook
                           'before-change-functions
                           (lambda (&rest _)
                             (cl-incf
                              hook-count))
                           nil t)
                          (list
                           (annotation-preserve-mod-p-and-undo
                            (put-text-property
                             1 2 'success t)
                            'returned)
                           (buffer-modified-p)
                           buffer-undo-list
                           hook-count
                           (get-text-property
                            1 'success))))
                      (with-temp-buffer
                        (buffer-enable-undo)
                        (insert "xyz")
                        (set-buffer-modified-p
                         nil)
                        (setq buffer-undo-list
                              nil)
                        (let ((hook-count 0))
                          (add-hook
                           'before-change-functions
                           (lambda (&rest _)
                             (cl-incf
                              hook-count))
                           nil t)
                          (list
                           (condition-case error-data
                               (annotation-preserve-mod-p-and-undo
                                (put-text-property
                                 1 3 'before-error t)
                                (error
                                 "annotation failed"))
                             (error error-data))
                           (buffer-modified-p)
                           buffer-undo-list
                           hook-count
                           (get-text-property
                            1 'before-error)))))"##;
    let expect =
        expect![[r#"OK ((returned nil nil 0 t) ((error "annotation failed") nil nil 0 t))"#]];
    assert_annotation_parity(elisp_form, expect);
}
