use expect_test::expect;

use super::{assert_annotation_autoload_parity, assert_annotation_parity};

#[test]
fn installed_descriptor_source_and_feature_match_exact_melpa_transaction() {
    let elisp_form = r##"(let ((descriptor
                         (cadr
                          (assq
                           'annotation
                           package-alist))))
                     (list
                      (featurep 'annotation)
                      (package-desc-name descriptor)
                      (package-version-join
                       (package-desc-version descriptor))
                      (package-desc-reqs descriptor)
                      (package-desc-summary descriptor)
                      (file-name-nondirectory
                       (symbol-file
                        'annotation-annotate
                        'defun))
                      (and
                       (string=
                        (file-name-nondirectory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))
                        "annotation.el")
                       t)))"##;
    let expect = expect![[
        r#"OK (t annotation "20250805.1029" nil "Functions for annotating text with faces and help bubbles." "annotation.el" t)"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_has_no_fabricated_entry_points_before_source_load() {
    let elisp_form = r##"(list
                      (featurep 'annotation)
                      (featurep
                       'annotation-autoloads)
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (fboundp symbol)
                          (and
                           (fboundp symbol)
                           (autoloadp
                            (symbol-function
                             symbol)))))
                       '(annotation-annotate
                         annotation-load
                         annotation-goto
                         annotation-go-back))
                      (boundp
                       'annotation-bindings)
                      (and
                       (member
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![
        "OK (nil t ((annotation-annotate nil nil) (annotation-load nil nil) (annotation-goto nil nil) (annotation-go-back nil nil)) nil nil)"
    ];
    assert_annotation_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_callable_and_macro_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (fboundp function)
                         (macrop function)
                         (copy-tree
                          (help-function-arglist
                           function t))
                         (commandp function)))
                      '(annotation-goto-indirect
                        annotation-go-back
                        annotation-goto-and-push
                        annotation-goto
                        annotation-merge-faces
                        annotation-annotate
                        annotation-preserve-mod-p-and-undo
                        annotation-remove-annotations
                        annotation-load))"##;
    let expect = expect![
        "OK ((annotation-goto-indirect t nil (link &optional other-window) nil) (annotation-go-back t nil nil nil) (annotation-goto-and-push t nil (source-buffer source-pos filepos &optional other-window) nil) (annotation-goto t nil (filepos &optional other-window) nil) (annotation-merge-faces t nil (start end faces &optional object) nil) (annotation-annotate t nil (start end anns &optional token-based info goto object) nil) (annotation-preserve-mod-p-and-undo t t (&rest code) nil) (annotation-remove-annotations t nil (&optional token-based start end object) nil) (annotation-load t nil (goto-help remove object &rest cmds) nil))"
    ];
    assert_annotation_parity(elisp_form, expect);
}

#[test]
fn bindings_are_buffer_local_while_navigation_history_is_process_global() {
    let elisp_form = r##"(let ((one
                          (generate-new-buffer
                           " *annotation-one*"))
                         (two
                          (generate-new-buffer
                           " *annotation-two*"))
                         (annotation-goto-stack
                          nil))
                     (unwind-protect
                         (progn
                           (with-current-buffer one
                             (setq annotation-bindings
                                   '((keyword
                                      . font-lock-keyword-face)))
                             (push
                              '("One.agda" . 3)
                              annotation-goto-stack))
                           (with-current-buffer two
                             (setq annotation-bindings
                                   '((string
                                      . font-lock-string-face)))
                             (push
                              '("Two.agda" . 7)
                              annotation-goto-stack))
                           (list
                            (local-variable-p
                             'annotation-bindings
                             one)
                            (local-variable-p
                             'annotation-bindings
                             two)
                            (with-current-buffer one
                              annotation-bindings)
                            (with-current-buffer two
                              annotation-bindings)
                            (default-value
                             'annotation-bindings)
                            annotation-goto-stack
                            (local-variable-p
                             'annotation-goto-stack
                             one)
                            (get
                             'annotation-preserve-mod-p-and-undo
                             'edebug-form-spec)))
                       (kill-buffer one)
                       (kill-buffer two)))"##;
    let expect = expect![[
        r#"OK (t t ((keyword . font-lock-keyword-face)) ((string . font-lock-string-face)) nil (("Two.agda" . 7) ("One.agda" . 3)) nil (&rest form))"#
    ]];
    assert_annotation_parity(elisp_form, expect);
}
