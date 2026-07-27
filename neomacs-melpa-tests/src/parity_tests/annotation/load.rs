use expect_test::expect;

use super::assert_annotation_parity;

#[test]
fn load_applies_ordered_string_object_commands_with_default_and_explicit_help() {
    let elisp_form = r##"(let ((text
                          (copy-sequence
                           "abcdef"))
                         (annotation-bindings
                          '((keyword
                             . font-lock-keyword-face)
                            (string
                             . font-lock-string-face))))
                     (annotation-load
                      "Follow declaration"
                      nil
                      text
                      '(0 3
                        (keyword)
                        t nil
                        ("Module.agda" . 17))
                      nil
                      '(3 6
                        (string)
                        nil
                        "Literal documentation"))
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
                        'annotation-token-based
                        text)
                       (get-text-property
                        position
                        'help-echo
                        text)
                       (get-text-property
                        position
                        'annotation-goto
                        text)
                       (get-text-property
                        position
                        'mouse-face
                        text)
                       (get-text-property
                        position
                        'annotation-annotations
                        text))))"##;
    let expect = expect![[
        r#"OK ((0 #1=(font-lock-keyword-face) t "Follow declaration" #2=("Module.agda" . 17) highlight #3=(annotation-annotated help-echo mouse-face annotation-goto annotation-token-based annotation-faces face)) (1 #1# t "Follow declaration" #2# highlight #3#) (2 #1# t "Follow declaration" #2# highlight #3#) (3 #4=(font-lock-string-face) nil "Literal documentation" nil highlight #5=(annotation-annotated help-echo mouse-face annotation-faces face)) (4 #4# nil "Literal documentation" nil highlight #5#) (5 #4# nil "Literal documentation" nil highlight #5#))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn incremental_buffer_load_replaces_old_tokens_and_preserves_disjoint_semantics() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdefgh")
                      (let ((annotation-bindings
                             '((old
                                . font-lock-comment-face)
                               (new
                                . font-lock-keyword-face)
                               (semantic
                                . font-lock-type-face))))
                        (annotation-annotate
                         1 4
                         '(old)
                         t
                         "Old left")
                        (annotation-annotate
                         6 9
                         '(old)
                         t
                         "Old right")
                        (annotation-annotate
                         4 7
                         '(semantic)
                         nil
                         "Semantic")
                        (annotation-load
                         "Jump"
                         t
                         nil
                         '(2 5
                           (new)
                           t nil
                           ("New.agda" . 9)))
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
                           'annotation-token-based)
                          (get-text-property
                           position
                           'help-echo)
                          (get-text-property
                           position
                           'annotation-goto)
                          (get-text-property
                           position
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 nil nil nil nil nil) (2 #1=(font-lock-keyword-face) t "Jump" #2=("New.agda" . 9) #3=(annotation-annotated help-echo mouse-face annotation-goto annotation-token-based annotation-faces face)) (3 #1# t "Jump" #2# #3#) (4 (font-lock-keyword-face . #4=(font-lock-type-face)) t "Jump" #2# #3#) (5 #4# nil "Semantic" nil (annotation-annotated help-echo mouse-face annotation-faces face)) (6 nil nil nil nil nil) (7 nil nil nil nil nil) (8 nil nil nil nil nil))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn non_removing_load_merges_new_faces_with_existing_token_metadata() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "abcdef")
                      (let ((annotation-bindings
                             '((old
                                . font-lock-comment-face)
                               (new
                                . font-lock-keyword-face))))
                        (annotation-annotate
                         1 5
                         '(old)
                         t
                         "Old")
                        (annotation-load
                         "Default"
                         nil
                         nil
                         '(3 7
                           (new)
                           nil
                           "New"))
                        (cl-loop
                         for position
                         from 1
                         below 7
                         collect
                         (list
                          position
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
                           'annotation-annotations)))))"##;
    let expect = expect![[
        r#"OK ((1 #1=(font-lock-comment-face) t "Old" #2=(annotation-annotated help-echo mouse-face annotation-token-based annotation-faces face)) (2 #1# t "Old" #2#) (3 #3=(font-lock-keyword-face . #1#) t "New" #2#) (4 #3# t "New" #2#) (5 #4=(font-lock-keyword-face) nil "New" #5=(annotation-annotated help-echo mouse-face annotation-faces face)) (6 #4# nil "New" #5#))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn removing_load_with_no_commands_clears_all_tokens_but_keeps_non_tokens() {
    let elisp_form = r##"(let ((text
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
                     (annotation-load
                      "Unused"
                      t
                      text
                      nil)
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
                        'annotation-token-based
                        text)
                       (get-text-property
                        position
                        'help-echo
                        text)
                       (get-text-property
                        position
                        'annotation-annotations
                        text))))"##;
    let expect = expect![[
        r#"OK ((0 #1=(font-lock-keyword-face) nil nil nil) (1 #1# nil nil nil) (2 #1# nil nil nil) (3 #2=(font-lock-type-face) nil "Semantic" #3=(annotation-annotated help-echo mouse-face annotation-faces face)) (4 #2# nil "Semantic" #3#) (5 #2# nil "Semantic" #3#))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}
