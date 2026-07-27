use expect_test::expect;

use super::{assert_ansilove_autoload_parity, assert_ansilove_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_dependency_contract() {
    let elisp_form = r##"(let* ((description (cadr (assq 'ansilove package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'ansilove)
   (package-installed-p 'ansilove)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (mapcar
    (lambda (requirement)
      (list
       (car requirement)
       (package-version-join (cadr requirement))
       (or (package-installed-p (car requirement))
           (package-built-in-p (car requirement)))))
    (package-desc-reqs description))
   (file-name-nondirectory (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t ansilove "20250105.1853" "Display buffers as PNG images using ansilove." ((emacs (26 1))) ((emacs "26.1" t)) "ansilove-20250105.1853")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn installed_library_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description (cadr (assq 'ansilove package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((file (expand-file-name name directory)))
       (list
        name
        (file-attribute-size (file-attributes file))
        (with-temp-buffer
          (insert-file-contents-literally file)
          (secure-hash 'sha256 (current-buffer))))))
   '("ansilove.el" "ansilove-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("ansilove.el" 10220 "f2fe2ab301f465d37bb78213c4541d559dfedda416dc1f9c0693c16951a76d3c") ("ansilove-pkg.el" 424 "c65b49b518398d5519ee966f121b9810c321736d04f4173e5b35bcf95dc42aa1"))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn source_preserves_revision_requirements_definition_counts_and_feature_contract() {
    let elisp_form = r##"(let ((source (locate-library "ansilove")))
  (with-temp-buffer
    (insert-file-contents-literally source)
    (let ((contents (buffer-string)))
      (list
       (file-name-nondirectory source)
       (count-lines (point-min) (point-max))
       (how-many "^(defun ansilove")
       (how-many "^(defcustom ansilove")
       (how-many "^;;;###autoload")
       (string-match-p "Package-Version: 20250105\\.1853" contents)
       (string-match-p "Package-Revision: a75eb6c89a1d" contents)
       (string-match-p
        (regexp-quote "Package-Requires: ((emacs \"26.1\"))")
        contents)
       (string-match-p "(provide 'ansilove)" contents)))))"##;
    let expect = expect![[r#"OK ("ansilove.el" 277 9 5 6 1053 1087 1145 10173)"#]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_preserves_arglists_interactivity_and_documentation() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (commandp symbol)
    (help-function-arglist symbol t)
    (interactive-form symbol)
    (documentation symbol t)))
 '(ansilove--init-temporary-directory
   ansilove--convert-file-to-png
   ansilove--buffer-to-png
   ansilove--check-executable
   ansilove-turn-to-editable-mode
   ansilove-mode
   ansilove-clean-temporary-directory
   ansilove-convert-and-display-now
   ansilove
   ansilove-quick-test-example))"##;
    let expect = expect![[
        r#"OK ((ansilove--init-temporary-directory t nil nil nil "Ensure ‘ansilove-temporary-directory’ is writable.") (ansilove--convert-file-to-png t nil (input-file output-file) nil "Wrapper for calling ‘ansilove-executable’.\nCalls ‘ansilove-executable’ given INPUT-FILE as input and\nOUTPUT-FILE as output.") (ansilove--buffer-to-png t nil (buffer) nil "Convert BUFFER contents to a PNG file.\nIf BUFFER is associated with a file take the BUFFER's file as input,\nelse save BUFFER to a temporary file and\nfeed that file to `ansilove--convert-file-to-png'.\nReturns a path to a PNG file created by \"ansilove\"\ninside the ‘ansilove-temporary-directory’.") (ansilove--check-executable t nil nil nil "Check if ‘ansilove-executable’ is usable.\nReturn t if true and nil if false.") (ansilove-turn-to-editable-mode t t nil (interactive nil) "Turn current buffer to a editable mode.") (ansilove-mode t t nil (interactive nil) "Major mode for ANSI image files.\n\nThis mode runs the hook `ansilove-mode-hook', as the final or\npenultimate step during initialization.\n\n\\{ansilove-mode-map}") (ansilove-clean-temporary-directory t t nil (interactive nil) "Remove lingering temporary files form ‘ansilove-temporary-directory’.") (ansilove-convert-and-display-now t t nil (interactive nil) "Convert current buffer using `ansilove--buffer-to-png'.\nDisplay the results by visiting the a temporarily created file.") (ansilove t t nil (interactive nil) "Display current buffer as a PNG image.\nIf ‘ansilove-clean-temporary-directory-before-conversion’ is non-nil\ncall `ansilove-clean-temporary-directory' before starting conversion.") (ansilove-quick-test-example t t nil (interactive nil) "Library showcase on one of the examples from \"ansilove\" repository.\nDownload a file specified by ‘ansilove-quick-example-test-url’ and open it."))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn complete_customization_surface_preserves_defaults_types_safety_and_groups() {
    let elisp_form = r##"(list
 ansilove-version
 (list
  (get 'ansilove 'custom-group)
  (get 'ansilove 'group-documentation))
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (symbol-value symbol)
     (get symbol 'custom-type)
     (get symbol 'custom-group)
     (get symbol 'safe-local-variable)
     (get symbol 'standard-value)
     (get symbol 'variable-documentation)))
  '(ansilove-executable
    ansilove-clean-temporary-directory-before-conversion
    ansilove-quick-test-example-url
    ansilove-mode-hook))
 (list
  (file-name-nondirectory
   (directory-file-name ansilove-temporary-directory))
  (file-name-absolute-p ansilove-temporary-directory)
  (get 'ansilove-temporary-directory 'custom-type)
  (get 'ansilove-temporary-directory 'custom-group)
  (get 'ansilove-temporary-directory 'safe-local-variable)
  (get 'ansilove-temporary-directory 'variable-documentation)))"##;
    let expect = expect![[
        r#"OK ("3.0.0" (((ansilove-executable custom-variable) (ansilove-temporary-directory custom-variable) (ansilove-clean-temporary-directory-before-conversion custom-variable) (ansilove-quick-test-example-url custom-variable) (ansilove-mode-hook custom-variable)) "Ansilove integration.") ((ansilove-executable "ansilove" file nil stringp ((funcall #'#[nil ("ansilove") #1=(t)])) "Path or name to the \"ansilove\" executable.") (ansilove-clean-temporary-directory-before-conversion nil boolean nil nil ((funcall #'#[nil (nil) #1#])) "Non-nil to clean ‘ansilove-temporary-directory’ at `ansilove' start.") (ansilove-quick-test-example-url "https://github.com/ansilove/ansilove/raw/master/examples/burps/bs-alove.ans" url-link nil stringp ((funcall #'#[nil ("https://github.com/ansilove/ansilove/raw/master/examples/burps/bs-alove.ans") #1#])) "File URL to download for `ansilove-quick-example-test'.") (ansilove-mode-hook nil hook nil nil ((funcall #'#[nil (nil) #1#])) "Hook for ansilove major mode.")) (".melpa-test_Emacs_ansilove" t file nil stringp "Temporary directory path used for file conversion via \"ansilove\"."))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_menu_and_supported_extensions_preserve_the_complete_user_interface() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (key)
    (cons key (lookup-key ansilove-mode-map (kbd key))))
  '("?" "C-c C-c" "a" "e" "h" "q" "x"))
 (copy-tree ansilove-mode-menu)
 (copy-sequence ansilove-supported-file-extensions)
 (mapcar
  (lambda (extension)
    (cons extension
          (cdr
           (assoc
            (format "\\.%s\\'" extension)
            auto-mode-alist))))
  ansilove-supported-file-extensions))"##;
    let expect = expect![[
        r#"OK ((("?" . describe-mode) ("C-c C-c" . ansilove) ("a" . ansilove) ("e" . ansilove-turn-to-editable-mode) ("h" . describe-mode) ("q" . quit-window) ("x")) (keymap "AnsiLove" (Convert menu-item "Convert" ansilove) (Edit menu-item "Edit" ansilove-turn-to-editable-mode) (Quit menu-item "Quit" quit-window) (Help menu-item "Help" describe-mode)) ("adf" "ans" "bin" "idf" "pcb" "tnd" "xb") (("adf" . ansilove-mode) ("ans" . ansilove-mode) ("bin" . ansilove-mode) ("idf" . ansilove-mode) ("pcb" . ansilove-mode) ("tnd" . ansilove-mode) ("xb" . ansilove-mode)))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_register_commands_mode_variable_and_every_file_extension() {
    let elisp_form = r##"(list
 (featurep 'ansilove)
 (featurep 'ansilove-autoloads)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (and (fboundp symbol) (autoloadp (symbol-function symbol)))
     (and (fboundp symbol) (commandp symbol))))
  '(ansilove-mode
    ansilove-convert-and-display-now
    ansilove
    ansilove-quick-test-example
    ansilove--buffer-to-png))
 (boundp 'ansilove-supported-file-extensions)
 (and (boundp 'ansilove-supported-file-extensions)
      (copy-sequence ansilove-supported-file-extensions))
 (mapcar
  (lambda (extension)
    (cons extension
          (cdr
           (assoc
            (format "\\.%s\\'" extension)
            auto-mode-alist))))
  ansilove-supported-file-extensions))"##;
    let expect = expect![[
        r#"OK (nil t ((ansilove-mode t t t) (ansilove-convert-and-display-now t t t) (ansilove t t t) (ansilove-quick-test-example t t t) (ansilove--buffer-to-png nil nil nil)) t ("adf" "ans" "bin" "idf" "pcb" "tnd" "xb") (("adf" . ansilove-mode) ("ans" . ansilove-mode) ("bin" . ansilove-mode) ("idf" . ansilove-mode) ("pcb" . ansilove-mode) ("tnd" . ansilove-mode) ("xb" . ansilove-mode)))"#
    ]];
    assert_ansilove_autoload_parity(elisp_form, expect);
}
