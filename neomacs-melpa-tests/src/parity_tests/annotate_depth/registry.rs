use expect_test::expect;

use super::{assert_annotate_depth_autoload_parity, assert_annotate_depth_parity};

#[test]
fn annotate_depth_registers_exact_feature_group_mode_and_surface() {
    let elisp_form = r##"(list
         (featurep 'annotate-depth)
         (get 'annotate-depth 'custom-group)
         (get 'annotate-depth 'group-documentation)
         (get 'annotate-depth-mode 'custom-group)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (commandp symbol)
                  (help-function-arglist symbol t)))
          '(annotate-depth-mode annotate-depth-enter annotate-depth-exit
            annotate-depth--annotate annotate-depth--add-overlay
            annotate-depth--clear-overlays annotate-depth--determine-indent
            annotate-depth--create-idle-timer annotate-depth--stop-timer)))"##;
    let expect = expect![[
        r#"OK (t ((annotate-depth custom-face) (annotate-depth-face custom-variable) (annotate-depth-threshold custom-variable) (annotate-depth-idle-timeout custom-variable) (annotate-depth-lighter custom-variable)) "Annotate buffer if indentation depth is beyond threshold." nil ((annotate-depth-mode t t (&optional arg)) (annotate-depth-enter t nil nil) (annotate-depth-exit t nil nil) (annotate-depth--annotate t nil nil) (annotate-depth--add-overlay t nil nil) (annotate-depth--clear-overlays t nil nil) (annotate-depth--determine-indent t nil nil) (annotate-depth--create-idle-timer t nil nil) (annotate-depth--stop-timer t nil nil)))"#
    ]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_custom_defaults_and_face_are_exact() {
    let elisp_form = r##"(list
         annotate-depth-face
         annotate-depth-threshold
         annotate-depth-idle-timeout
         annotate-depth-lighter
         (get 'annotate-depth 'face-defface-spec)
         (get 'annotate-depth-face 'custom-type)
         (get 'annotate-depth-threshold 'custom-type)
         (get 'annotate-depth-idle-timeout 'custom-type)
         (get 'annotate-depth-lighter 'custom-type))"##;
    let expect = expect![[
        r##"OK (annotate-depth 5 2 " Depth" ((t :background "#770000")) face integer integer string)"##
    ]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_mode_variables_are_buffer_local_and_keymap_is_empty() {
    let elisp_form = r##"(list
         (local-variable-if-set-p 'annotate-depth--overlays)
         (local-variable-if-set-p 'annotate-depth--idle-timer)
         (keymapp annotate-depth-map)
         (where-is-internal 'annotate-depth-mode annotate-depth-map)
         (with-temp-buffer
           (setq annotate-depth--overlays '(local)
                 annotate-depth--idle-timer 'timer)
           (list annotate-depth--overlays annotate-depth--idle-timer))
         (list annotate-depth--overlays annotate-depth--idle-timer))"##;
    let expect = expect!["OK (t t t nil ((local) timer) (nil nil))"];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_descriptor_records_exact_pin_and_installed_payload() {
    let elisp_form = r##"(let* ((description (cadr (assq 'annotate-depth package-alist)))
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
        r#"OK (annotate-depth "20160520.2040" nil "Annotate buffer if indentation depth is beyond threshold." nil (("README-elpa" 753 "302a02fba6df7f8b9ac07c50de898ebde6ee0a0e686112c1e690ebd2b9871c4e") ("annotate-depth-autoloads.el" 1401 "7062df23c416145507ee6ea1c4718a79fc577982afa3b8650c18719506687050") ("annotate-depth-pkg.el" 467 "ddab422ce96ae46971994fe3b12e844d58bc6b60799cd49e9e6e5d279b9423a8") ("annotate-depth.el" 6338 "82d6f89cb89d6dca284ed53c1d23a70d40461f00035917f52222e32a849d4031") ("annotate-depth.elc" 6154 "cb9aa7a98af70942434f97926d65e5ace7be0c1a0880cf07fcbffc2c43577506")))"#
    ]];
    assert_annotate_depth_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_autoload_exposes_only_mode_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'annotate-depth)
         (commandp 'annotate-depth-mode)
         (autoloadp (symbol-function 'annotate-depth-mode))
         (symbol-function 'annotate-depth-mode)
         (fboundp 'annotate-depth-enter)
         (boundp 'annotate-depth-threshold))"##;
    let expect = expect![[
        r#"OK (nil t t (autoload "annotate-depth" "Minor mode for annotating indentation when too deep.\n\nThis is a minor mode.  If called interactively, toggle the\n`Annotate-Depth mode' mode.  If the prefix argument is positive, enable\nthe mode, and if it is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `annotate-depth-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil) nil nil)"#
    ]];
    assert_annotate_depth_autoload_parity(elisp_form, expect);
}

#[test]
fn annotate_depth_reload_preserves_dynamic_customization_and_single_feature() {
    let elisp_form = r##"(let ((annotate-depth-threshold 9)
               (annotate-depth-lighter " Deep")
               (source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (load source nil t t)
         (load source nil t t)
         (list annotate-depth-threshold
               annotate-depth-lighter
               (length
                (cl-remove-if-not
                 (lambda (feature) (eq feature 'annotate-depth))
                 features))
               (featurep 'annotate-depth)))"##;
    let expect = expect![[r#"OK (9 " Deep" 1 t)"#]];
    assert_annotate_depth_parity(elisp_form, expect);
}
