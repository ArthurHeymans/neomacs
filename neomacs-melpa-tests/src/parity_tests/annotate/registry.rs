use expect_test::expect;

use super::{assert_annotate_autoload_parity, assert_annotate_parity};

#[test]
fn annotate_registers_exact_feature_mode_group_and_commands() {
    let elisp_form = r##"(list
         (featurep 'annotate)
         (get 'annotate-mode 'custom-group)
         (get 'annotate 'custom-group)
         (get 'annotate 'group-documentation)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (commandp symbol)
                  (help-function-arglist symbol t)))
          '(annotate-mode annotate-annotate annotate-delete-annotation
            annotate-reply-to annotate-show-thread-at-point
            annotate-show-annotation-summary annotate-switch-db
            annotate-integrate-annotations annotate-export-annotations)))"##;
    let expect = expect![[
        r#"OK (t nil ((annotate-file custom-variable) (annotate-file-buffer-local custom-variable) (annotate-buffer-local-database-extension custom-variable) (annotate-highlight-faces custom-variable) (annotate-annotation-text-faces custom-variable) (annotate-prefix custom-face) (annotate-annotation-column custom-variable) (annotate-diff-export-options custom-variable) (annotate-use-messages custom-variable) (annotate-popup-warning-indirect-buffer custom-variable) (annotate-integrate-marker custom-variable) (annotate-integrate-highlight custom-variable) (annotate-fallback-comment custom-variable) (annotate-blacklist-major-mode custom-variable) (annotate-summary-ask-query custom-variable) (annotate-database-confirm-deletion custom-variable) (annotate-annotation-confirm-deletion custom-variable) (annotate-database-confirm-import custom-variable) (annotate-annotation-max-size-not-place-new-line custom-variable) (annotate-annotation-position-policy custom-variable) (annotate-use-echo-area custom-variable) (annotate-print-annotation-under-cursor custom-variable) (annotate-print-annotation-under-cursor-prefix custom-variable) (annotate-print-annotation-under-cursor-delay custom-variable) (annotate-warn-if-hash-mismatch custom-variable) (annotate-endline-annotate-whole-line custom-variable) (annotate-search-region-lines-delta custom-variable) (annotate-autosave custom-variable) (annotate-annotation-expansion-map custom-variable) (annotate-thread-header-face custom-variable) (annotate-thread-author-face custom-variable) (annotate-thread-tree-arrow-face custom-variable) (annotate-thread-tree-face custom-variable) (annotate-thread-action-face custom-variable)) "Annotate files without changing them." ((annotate-mode t t (&optional arg)) (annotate-annotate t t (&optional color-index)) (annotate-delete-annotation t t (&rest --cl-rest--)) (annotate-reply-to t t nil) (annotate-show-thread-at-point t t nil) (annotate-show-annotation-summary t t (&optional arg-query cut-above-point &rest --cl-rest--)) (annotate-switch-db t t (&rest --cl-rest--)) (annotate-integrate-annotations t t nil) (annotate-export-annotations t t nil)))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_mode_map_preserves_all_documented_workflow_keys() {
    let elisp_form = r##"(mapcar
         (lambda (key)
           (list key
                 (lookup-key annotate-mode-map (kbd key))
                 (commandp (lookup-key annotate-mode-map (kbd key)))))
         '("C-c C-a" "C-c C-d" "C-c C-r" "C-c C-t"
           "C-c C-p" "C-c C-c" "C-c C-s" "C-c ]" "C-c ["))"##;
    let expect = expect![[
        r#"OK (("C-c C-a" annotate-annotate t) ("C-c C-d" annotate-delete-annotation t) ("C-c C-r" annotate-reply-to t) ("C-c C-t" annotate-show-thread-at-point t) ("C-c C-p" annotate-change-annotation-text-position t) ("C-c C-c" annotate-change-annotation-colors t) ("C-c C-s" annotate-show-annotation-summary t) ("C-c ]" annotate-goto-next-annotation t) ("C-c [" annotate-goto-previous-annotation t))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_custom_defaults_capture_storage_rendering_and_safety_contract() {
    let elisp_form = r##"(list
         (file-name-nondirectory annotate-file)
         annotate-file-buffer-local
         annotate-buffer-local-database-extension
         annotate-highlight-faces
         annotate-annotation-text-faces
         annotate-annotation-column
         annotate-integrate-marker
         annotate-integrate-highlight
         annotate-fallback-comment
         annotate-summary-ask-query
         annotate-database-confirm-deletion
         annotate-annotation-confirm-deletion
         annotate-annotation-position-policy
         annotate-use-echo-area
         annotate-warn-if-hash-mismatch
         annotate-autosave
         annotate-search-region-lines-delta
         annotate-allowed-positioning-policy)"##;
    let expect = expect![[
        r##"OK ("annotations" nil "notes" ((:underline "#EEF192") (:underline "#92EEF1") (:underline "#F192EE")) ((:background "#EEF192" :foreground "black") (:background "#92EEF1" :foreground "black") (:background "#F192EE" :foreground "black")) 85 " ANNOTATION: " 126 "#" t t nil :by-length nil t t 2 (:by-length :margin :new-line))"##
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_descriptor_records_exact_pin_requirements_and_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'annotate package-alist)))
               (directory (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar
            (lambda (file)
              (let ((relative (file-relative-name file directory)))
                (list relative
                      (file-attribute-size (file-attributes file))
                      (secure-hash 'sha256 file))))
            (directory-files-recursively directory "." nil))
           (lambda (a b) (string< (car a) (car b))))))"##;
    let expect = expect![[
        r#"OK (annotate "20260514.1320" nil "Annotate files without changing them." ((emacs (27 1))) (("README-elpa" 693 "8d29a8554bd6ed4e458c95e663c64071e25a1891513b44d065227a46033adf4c") ("annotate-autoloads.el" 1602 "af0be2bb40c47fad418649fb9a1f7edf1fb9fbf4812086b3c3703bbf0d16ef31") ("annotate-pkg.el" 404 "d64dc410f320b885ff984b6b2bfd96fe61bb011f8c76e2752a2151d45ba64cf9") ("annotate.el" 225207 "568222b2a6ff8ae23f7609352bd93c6bb51326d6d10b09502c9274de0e474487") ("annotate.elc" 156017 "6c8a0466bb3e20bcab2c775d07291b34bd3311a1a9e44bd3b6dca10f61a48ddb")))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_autoloads_expose_mode_and_user_entrypoints_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'annotate)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (commandp symbol)
                  (autoloadp (symbol-function symbol))
                  (symbol-function symbol)))
          '(annotate-mode annotate-annotate annotate-switch-db
            annotate-show-annotation-summary)))"##;
    let expect = expect![[
        r#"OK (nil ((annotate-mode t t (autoload "annotate" "Toggle Annotate mode.\n\nSee https://github.com/bastibe/annotate.el/ for documentation.\n\nThis is a minor mode.  If called interactively, toggle the `Annotate\nmode' mode.  If the prefix argument is positive, enable the mode, and if\nit is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `annotate-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil)) (annotate-annotate nil nil nil) (annotate-switch-db nil nil nil) (annotate-show-annotation-summary nil nil nil)))"#
    ]];
    assert_annotate_autoload_parity(elisp_form, expect);
}

#[test]
fn annotate_defines_exact_error_hierarchy_and_button_types() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list symbol
                  (get symbol 'error-conditions)
                  (get symbol 'error-message)))
          '(annotate-error annotate-empty-annotation-text-error
            annotate-no-new-line-at-end-file-error annotate-db-file-not-found
            annotate-annotate-region-overlaps annotate-query-parsing-error))
         (mapcar
          (lambda (type)
            (list type
                  (get type 'button-category-symbol)
                  (get type 'follow-link)
                  (get type 'help-echo)))
          '(annotate-summary-show-annotation-button
            annotate-summary-delete-annotation-button
            annotate-summary-replace-annotation-button
            annotate-summary-show-thread-button
            annotate-thread-delete-node-button
            annotate-thread-reply-node-button)))"##;
    let expect = expect![[
        r#"OK (((annotate-error (annotate-error error) "Annotation error") (annotate-empty-annotation-text-error (annotate-empty-annotation-text-error annotate-error error) "Empty annotation text") (annotate-no-new-line-at-end-file-error (annotate-no-new-line-at-end-file-error annotate-error error) "No newline found at the end of the buffer") (annotate-db-file-not-found (annotate-db-file-not-found annotate-error error) "Annotations database file not found") (annotate-annotate-region-overlaps (annotate-annotate-region-overlaps annotate-error error) "Error: the region overlaps with at least an already existing annotation") (annotate-query-parsing-error (annotate-query-parsing-error annotate-error error) "Parsing failed:")) ((annotate-summary-show-annotation-button annotate-summary-show-annotation-button-button nil nil) (annotate-summary-delete-annotation-button annotate-summary-delete-annotation-button-button nil nil) (annotate-summary-replace-annotation-button annotate-summary-replace-annotation-button-button nil nil) (annotate-summary-show-thread-button annotate-summary-show-thread-button-button nil nil) (annotate-thread-delete-node-button annotate-thread-delete-node-button-button nil nil) (annotate-thread-reply-node-button annotate-thread-reply-node-button-button nil nil)))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_reload_preserves_user_customization_and_single_feature_registration() {
    let elisp_form = r##"(let ((annotate-annotation-column 47)
               (annotate-fallback-comment ";;")
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list annotate-annotation-column
               annotate-fallback-comment
               (length
                (cl-remove-if-not
                 (lambda (feature) (eq feature 'annotate))
                 features))
               (featurep 'annotate)))"##;
    let expect = expect![[r#"OK (47 ";;" 1 t)"#]];
    assert_annotate_parity(elisp_form, expect);
}
