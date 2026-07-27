use expect_test::expect;

use super::assert_annotation_parity;

#[test]
fn merge_faces_splits_existing_runs_unions_faces_and_rejects_invalid_ranges() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdef")
                      (add-text-properties
                       2 4
                       '(annotation-faces
                         (base-face)
                         face
                         (base-face)))
                      (annotation-merge-faces
                       1 5
                       '(primary-face))
                      (annotation-merge-faces
                       3 7
                       '(secondary-face
                         primary-face))
                      (list
                       (cl-loop
                        for position
                        from (point-min)
                        below (point-max)
                        collect
                        (list
                         position
                         (buffer-substring-no-properties
                          position
                          (1+ position))
                         (get-text-property
                          position
                          'annotation-faces)
                         (get-text-property
                          position
                          'face)))
                       (condition-case error-data
                           (annotation-merge-faces
                            2 2
                            '(invalid))
                         (error
                          (copy-tree
                           error-data)))
                       (condition-case error-data
                           (annotation-merge-faces
                            "two" 3
                            '(invalid))
                         (error
                          (copy-tree
                           error-data)))))"##;
    let expect = expect![[
        r#"OK (((1 "a" #1=(primary-face) #1#) (2 "b" #2=(primary-face base-face) #2#) (3 "c" #3=(secondary-face . #2#) #3#) (4 "d" #4=(secondary-face primary-face) #4#) (5 "e" #4# #4#) (6 "f" #4# #4#)) (cl-assertion-failed (condition-case nil (< start end) (error nil))) (cl-assertion-failed (condition-case nil (< start end) (error nil))))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn annotate_buffer_applies_faces_token_help_and_goto_properties_to_exact_range() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdef")
                      (let ((annotation-bindings
                             '((keyword
                                . font-lock-keyword-face)
                               (string
                                . font-lock-string-face))))
                        (annotation-annotate
                         2 6
                         '(keyword
                           missing
                           string)
                         t
                         "Open declaration"
                         '("Module.agda" . 19))
                        (cl-loop
                         for position
                         from (point-min)
                         below (point-max)
                         collect
                         (list
                          position
                          (get-text-property
                           position
                           'face)
                          (get-text-property
                           position
                           'annotation-faces)
                          (get-text-property
                           position
                           'annotation-token-based)
                          (get-text-property
                           position
                           'help-echo)
                          (get-text-property
                           position
                           'annotation-goto)
                          (get-text-property
                           position
                           'mouse-face)
                          (get-text-property
                           position
                           'annotation-annotated)
                          (get-text-property
                           position
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 nil nil nil nil nil nil nil nil) (2 #1=(font-lock-keyword-face font-lock-string-face) #1# t "Open declaration" #2=("Module.agda" . 19) highlight t #3=(annotation-annotated help-echo mouse-face annotation-goto annotation-token-based annotation-faces face)) (3 #1# #1# t "Open declaration" #2# highlight t #3#) (4 #1# #1# t "Open declaration" #2# highlight t #3#) (5 #1# #1# t "Open declaration" #2# highlight t #3#) (6 nil nil nil nil nil nil nil nil))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn overlapping_annotations_merge_faces_and_accumulate_independent_metadata() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdefgh")
                      (let ((annotation-bindings
                             '((name
                                . font-lock-function-name-face)
                               (type
                                . font-lock-type-face))))
                        (annotation-annotate
                         1 6
                         '(name)
                         nil
                         "Name help")
                        (annotation-annotate
                         4 9
                         '(type)
                         t
                         nil
                         '("Types.agda" . 42))
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
                           'annotation-goto)
                          (get-text-property
                           position
                           'annotation-token-based)
                          (get-text-property
                           position
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 #1=(font-lock-function-name-face) "Name help" nil nil #2=(annotation-annotated help-echo mouse-face annotation-faces face)) (2 #1# "Name help" nil nil #2#) (3 #1# "Name help" nil nil #2#) (4 #3=(font-lock-type-face . #1#) "Name help" #4=("Types.agda" . 42) t #5=(help-echo . #6=(annotation-annotated mouse-face annotation-goto annotation-token-based annotation-faces face))) (5 #3# "Name help" #4# t #5#) (6 #7=(font-lock-type-face) nil #4# t #6#) (7 #7# nil #4# t #6#) (8 #7# nil #4# t #6#))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn unknown_faces_only_create_annotations_when_info_or_goto_adds_real_metadata() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdef")
                      (let ((annotation-bindings
                             '((known
                                . font-lock-keyword-face))))
                        (annotation-annotate
                         1 3
                         '(unknown))
                        (annotation-annotate
                         3 5
                         '(unknown)
                         nil
                         "Documentation only")
                        (annotation-annotate
                         5 7
                         '(unknown)
                         nil
                         nil
                         '("Target.agda" . 2))
                        (list
                         (cl-loop
                          for position
                          from 1
                          below 7
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
                            'annotation-goto)
                           (get-text-property
                            position
                            'annotation-annotated)
                           (get-text-property
                            position
                            'annotation-annotations)))
                         (buffer-modified-p))))"##;
    let expect = expect![[
        r#"OK (((1 nil nil nil nil nil) (2 nil nil nil nil nil) (3 nil "Documentation only" nil t #1=(annotation-annotated help-echo mouse-face)) (4 nil "Documentation only" nil t #1#) (5 nil nil #2=("Target.agda" . 2) t #3=(annotation-annotated mouse-face annotation-goto)) (6 nil nil #2# t #3#)) t)"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn nil_annotation_list_removes_owned_properties_but_preserves_foreign_properties() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdef")
                      (let ((annotation-bindings
                             '((name
                                . font-lock-function-name-face))))
                        (add-text-properties
                         1 7
                         '(foreign-property
                           retained))
                        (annotation-annotate
                         2 6
                         '(name)
                         t
                         "Temporary")
                        (annotation-annotate
                         3 5
                         nil)
                        (cl-loop
                         for position
                         from 1
                         below 7
                         collect
                         (list
                          position
                          (get-text-property
                           position
                           'foreign-property)
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
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 retained nil nil nil nil) (2 retained #1=(font-lock-function-name-face) "Temporary" t #2=(annotation-annotated help-echo mouse-face annotation-token-based annotation-faces face)) (3 retained nil nil nil nil) (4 retained nil nil nil nil) (5 retained #1# "Temporary" t #2#) (6 retained nil nil nil nil))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn string_object_annotation_uses_zero_based_offsets_and_font_lock_face() {
    let elisp_form = r##"(let ((text
                          (copy-sequence
                           "abcdef"))
                         (annotation-bindings
                          '((keyword
                             . font-lock-keyword-face)
                            (type
                             . font-lock-type-face))))
                     (annotation-annotate
                      0 4
                      '(keyword)
                      t
                      "Object help"
                      nil
                      text)
                     (annotation-annotate
                      2 6
                      '(type)
                      nil
                     nil
                      '("Object.agda" . 8)
                      text)
                     (list
                      (substring-no-properties
                       text)
                      (cl-loop
                       for position
                       from 0
                       below (length text)
                       collect
                       (list
                        position
                        (get-text-property
                         position
                         'face
                         text)
                        (get-text-property
                         position
                         'font-lock-face
                         text)
                        (get-text-property
                         position
                         'annotation-faces
                         text)
                        (get-text-property
                         position
                         'annotation-token-based
                         text)
                        (get-text-property
                         position
                         'annotation-goto
                         text)
                        (get-text-property
                         position
                         'annotation-annotations
                         text)))))"##;
    let expect = expect![[
        r#"OK ("abcdef" ((0 nil #1=(font-lock-keyword-face) #1# t nil #2=(annotation-annotated help-echo mouse-face annotation-token-based annotation-faces face)) (1 nil #1# #1# t nil #2#) (2 nil #3=(font-lock-type-face . #1#) #3# t #4=("Object.agda" . 8) #5=(annotation-goto . #2#)) (3 nil #3# #3# t #4# #5#) (4 nil #6=(font-lock-type-face) #6# nil #4# #7=(annotation-annotated mouse-face annotation-goto annotation-faces face)) (5 nil #6# #6# nil #4# #7#)))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn buffer_bounds_and_narrowing_gate_buffer_annotations_but_not_string_objects() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdef")
                      (let ((annotation-bindings
                             '((keyword
                                . font-lock-keyword-face)))
                            (text
                             (copy-sequence
                              "xyz")))
                        (narrow-to-region
                         2 6)
                        (list
                         (annotation-annotate
                          1 4
                          '(keyword))
                         (annotation-annotate
                          2 6
                          '(keyword))
                         (annotation-annotate
                          4 4
                          '(keyword))
                         (annotation-annotate
                          0 3
                          '(keyword)
                          nil nil nil
                          text)
                         (save-restriction
                           (widen)
                           (cl-loop
                            for position
                            from 1
                            below 7
                            collect
                            (get-text-property
                             position
                             'face)))
                         (cl-loop
                          for position
                          from 0
                          below (length text)
                          collect
                          (get-text-property
                           position
                           'font-lock-face
                           text))
                         (condition-case error-data
                             (annotation-annotate
                              -1 4
                              '(keyword)
                              nil nil nil
                              text)
                           (error error-data)))))"##;
    let expect = expect![
        "OK (nil nil nil nil (nil #1=(font-lock-keyword-face) #1# #1# #1# nil) (#2=(font-lock-keyword-face) #2# #2#) (args-out-of-range -1 -1))"
    ];
    assert_annotation_parity(elisp_form, expect);
}
