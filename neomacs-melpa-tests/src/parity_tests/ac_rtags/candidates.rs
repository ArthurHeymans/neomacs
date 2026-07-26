use expect_test::expect;

use super::assert_ac_rtags_parity;

#[test]
fn ac_rtags_candidates_requires_a_file_and_forwards_modified_buffer_location_and_flags() {
    let elisp_form = r##"(let (calls
                   source-buffer)
               (cl-letf
                   (((symbol-function
                      'rtags-current-location)
                     (lambda ()
                       (push
                        (list
                         'location
                         (eq
                          (current-buffer)
                          source-buffer)
                         buffer-file-name
                         (buffer-string)
                         (buffer-modified-p))
                        calls)
                       "/project/main.cpp:7:11"))
                    ((symbol-function
                      'rtags-call-rc)
                     (lambda (&rest arguments)
                       (let* ((unsaved
                               (plist-get
                                arguments
                                :unsaved))
                              (normalized
                               (copy-sequence
                                arguments)))
                         (plist-put
                          normalized
                          :unsaved
                          (cond
                           ((null
                             unsaved)
                            nil)
                           ((eq
                             unsaved
                             source-buffer)
                            'source-buffer)
                           (t
                            'other-buffer)))
                         (push
                          (list
                           'rc
                           normalized
                           (and
                            (bufferp
                             unsaved)
                            (with-current-buffer
                                unsaved
                              (list
                               (eq
                                (current-buffer)
                                source-buffer)
                               buffer-file-name
                               (buffer-string)
                               (buffer-modified-p))))
                           (list
                            (eq
                             (current-buffer)
                             source-buffer)
                            buffer-file-name
                            (buffer-string)))
                          calls))
                       (insert
                        "(quote (completions (results ((\"name\" \"void name(int)\" \"FunctionDecl\")))))"))))
                 (let ((without-file
                        (with-temp-buffer
                          (insert
                           "name")
                          (setq
                           source-buffer
                           (current-buffer))
                          (ac-rtags-candidates))))
                   (let ((unmodified
                          (with-temp-buffer
                            (setq
                             buffer-file-name
                             "/project/main.cpp"
                             buffer-undo-list
                             nil)
                            (set-buffer-modified-p
                             nil)
                            (setq
                             source-buffer
                             (current-buffer))
                            (ac-rtags-candidates))))
                     (let ((modified
                            (with-temp-buffer
                              (setq
                               buffer-file-name
                               "/project/main.cpp"
                               buffer-undo-list
                               nil)
                              (insert
                               "changed")
                              (setq
                               source-buffer
                               (current-buffer))
                              (ac-rtags-candidates))))
                       (list
                        without-file
                        unmodified
                        modified
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil (#("name" 0 4 (ac-rtags-full "void name(int)" ac-rtags-type "FunctionDecl"))) (#("name" 0 4 (ac-rtags-full "void name(int)" ac-rtags-type "FunctionDecl"))) ((location t nil "name" t) (location t "/project/main.cpp" "" nil) (rc (:path "/project/main.cpp" :unsaved nil "--code-complete-at" "/project/main.cpp:7:11" "--synchronous-completions" "--elisp") nil (nil nil "")) (location t "/project/main.cpp" "changed" t) (rc (:path "/project/main.cpp" :unsaved source-buffer "--code-complete-at" "/project/main.cpp:7:11" "--synchronous-completions" "--elisp") (t "/project/main.cpp" "changed" t) (nil nil ""))))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_candidates_parse_completion_payload_and_preserve_full_and_type_properties() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/project/main.cpp")
               (cl-letf
                   (((symbol-function
                      'rtags-current-location)
                     (lambda ()
                       "location"))
                    ((symbol-function
                      'rtags-call-rc)
                     (lambda (&rest _arguments)
                       (insert
                        "(quote (completions (results ((\"alpha\" \"int alpha\" \"VarDecl\") (\"βeta\" \"void βeta(int, int)\" \"CXXMethod\") (\"alpha\" \"namespace alpha\" \"Namespace\")))))"))))
                 (let ((items
                        (ac-rtags-candidates)))
                   (list
                    items
                    (mapcar
                     (lambda (item)
                       (list
                        (substring-no-properties
                         item)
                        (text-properties-at
                         0
                         item)
                        (ac-rtags-document
                         item)))
                     items)))))"##;
    let expect = expect![[
        r#"OK ((#("alpha" 0 5 (ac-rtags-full "int alpha" ac-rtags-type "VarDecl")) #("βeta" 0 4 (ac-rtags-full "void βeta(int, int)" ac-rtags-type "CXXMethod")) #("alpha" 0 5 (ac-rtags-full "namespace alpha" ac-rtags-type "Namespace"))) (("alpha" (ac-rtags-full "int alpha" ac-rtags-type "VarDecl") "int alpha") ("βeta" (ac-rtags-full "void βeta(int, int)" ac-rtags-type "CXXMethod") "void βeta(int, int)") ("alpha" (ac-rtags-full "namespace alpha" ac-rtags-type "Namespace") "namespace alpha")))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_candidates_cover_non_list_wrong_tag_malformed_shape_read_and_eval_failures() {
    let elisp_form = r##"(let ((responses
                    '("plain text"
                      "(quote 42)"
                      "(quote (other (results ((\"x\" \"full\" \"Type\")))))"
                      "(quote (completions malformed))"
                      "(completions"
                      "(error \"boom\")"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'rtags-current-location)
                     (lambda ()
                       "location"))
                    ((symbol-function
                      'rtags-call-rc)
                     (lambda (&rest _arguments)
                       (insert
                        (pop responses))))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        arguments
                        calls)
                       'messaged)))
                 (mapcar
                  (lambda (_index)
                    (with-temp-buffer
                      (setq
                       buffer-file-name
                       "/project/main.cpp")
                      (let ((result
                             (condition-case error
                                 (list
                                  'value
                                  (ac-rtags-candidates))
                               (error
                                (list
                                 'signal
                                 (car
                                  error)
                                 (cdr
                                  error))))))
                        (prog1
                            (append
                             result
                             (list
                              (nreverse
                               calls)))
                          (setq
                           calls
                           nil)))))
                  '(1 2 3 4 5 6))))"##;
    let expect = expect![[
        r#"OK ((value nil nil) (signal wrong-type-argument (listp 42) nil) (value nil nil) (signal wrong-type-argument (listp malformed) nil) (value nil (("****** Got Completion Error ******"))) (value nil (("****** Got Completion Error ******"))))"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_document_reads_only_the_first_character_full_signature_property() {
    let elisp_form = r##"(list
               (ac-rtags-document
                (propertize
                 "item"
                 'ac-rtags-full
                 "full signature"
                 'ac-rtags-type
                 "Type"))
               (ac-rtags-document
                "plain")
               (ac-rtags-document
                "")
               (let ((item
                      (concat
                       "x"
                       (propertize
                        "y"
                        'ac-rtags-full
                        "late"))))
                 (ac-rtags-document
                  item)))"##;
    let expect = expect![[r#"OK ("full signature" nil nil nil)"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}
