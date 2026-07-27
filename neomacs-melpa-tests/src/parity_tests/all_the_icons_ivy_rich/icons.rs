use expect_test::expect;

use super::assert_all_the_icons_ivy_rich_parity;

#[test]
fn icon_formatter_preserves_the_real_dependency_glyph_and_rebuilds_color_and_geometry_properties() {
    // Current Neomacs divergence: `propertize` mutates the original icon's
    // face, while GNU Emacs returns independently propertized strings.
    let elisp_form = r##"(let* ((icon
                     (all-the-icons-faicon
                      "cog"
                      :face 'all-the-icons-blue
                      :height 0.95
                      :v-adjust -0.05))
                    (all-the-icons-ivy-rich-icon-size 1.25)
                    (all-the-icons-ivy-rich-color-icon t)
                    (colored
                     (all-the-icons-ivy-rich--format-icon icon))
                    (all-the-icons-ivy-rich-color-icon nil)
                    (plain
                     (all-the-icons-ivy-rich--format-icon icon)))
               (list
                (list
                 (substring-no-properties icon)
                 (string-to-list
                  (substring-no-properties icon))
                 (get-text-property 0 'face icon)
                 (get-text-property 0 'display icon))
                (list
                 (substring-no-properties colored)
                 (get-text-property 0 'display colored)
                 (get-text-property 1 'face colored)
                 (get-text-property 1 'display colored))
                (list
                 (substring-no-properties plain)
                 (get-text-property 0 'display plain)
                 (get-text-property 1 'face plain)
                 (get-text-property 1 'display plain))))"##;
    let expect = expect![[
        r#"OK (("" (61459) (:family "FontAwesome" :height 1.14 :inherit all-the-icons-blue) #1=(raise -0.06)) (" " #2=((space :relative-width 0.1)) (:inherit all-the-icons-blue :family "FontAwesome" :height 1.25) #1#) (" " #2# (:inherit all-the-icons-ivy-rich-icon-face :family "FontAwesome" :height 1.25) #1#))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn real_all_the_icons_file_lookup_renders_directory_source_document_and_fallback_candidates() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (cl-letf
                   (((symbol-function 'display-graphic-p)
                     (lambda (&optional _frame) t)))
                 (mapcar
                  (lambda (candidate)
                    (let ((icon
                           (all-the-icons-ivy-rich-file-icon
                            candidate)))
                      (list
                       candidate
                       (substring-no-properties icon)
                       (string-to-list
                        (substring-no-properties icon))
                       (get-text-property 1 'face icon)
                       (get-text-property 1 'display icon))))
                  '("src/"
                    "main.rs"
                    "init.el"
                    "README.md"
                    "archive.unknown-extension"
                    ""))))"##;
    let expect = expect![[
        r#"OK (("src/" " " (32 61462) (:inherit all-the-icons-ivy-rich-dir-face :family "github-octicons" :height 1.0) #1=(raise 0.0)) ("main.rs" " " (32 59692) (:inherit all-the-icons-maroon :family "all-the-icons" :height 1.0) #1#) ("init.el" " " (32 59686) (:inherit all-the-icons-purple :family "file-icons" :height 1.0) #1#) ("README.md" " " (32 61447) (:inherit all-the-icons-lcyan :family "github-octicons" :height 1.0) #1#) ("archive.unknown-extension" " " (32 61462) (:inherit all-the-icons-dsilver :family "FontAwesome" :height 1.0) #1#) ("" " " (32 61462) (:inherit all-the-icons-dsilver :family "FontAwesome" :height 1.0) (raise 0.0)))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn completion_category_icons_use_real_all_the_icons_glyphs_and_exact_face_properties() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (cl-letf
                   (((symbol-function 'display-graphic-p)
                     (lambda (&optional _frame) t)))
                 (mapcar
                  (lambda (entry)
                    (let ((icon
                           (funcall (cdr entry) "fixture")))
                      (list
                       (car entry)
                       (substring-no-properties icon)
                       (string-to-list
                        (substring-no-properties icon))
                       (get-text-property 1 'face icon))))
                  '((directory
                     . all-the-icons-ivy-rich-dir-icon)
                    (project
                     . all-the-icons-ivy-rich-project-icon)
                    (mode
                     . all-the-icons-ivy-rich-mode-icon)
                    (command
                     . all-the-icons-ivy-rich-command-icon)
                    (history
                     . all-the-icons-ivy-rich-history-icon)
                    (face
                     . all-the-icons-ivy-rich-face-icon)
                    (theme
                     . all-the-icons-ivy-rich-theme-icon)
                    (keybinding
                     . all-the-icons-ivy-rich-keybinding-icon)
                    (library
                     . all-the-icons-ivy-rich-library-icon)
                    (package
                     . all-the-icons-ivy-rich-package-icon)
                    (font
                     . all-the-icons-ivy-rich-font-icon)
                    (world-clock
                     . all-the-icons-ivy-rich-world-clock-icon)
                    (tramp
                     . all-the-icons-ivy-rich-tramp-icon)
                    (git-branch
                     . all-the-icons-ivy-rich-git-branch-icon)
                    (git-commit
                     . all-the-icons-ivy-rich-git-commit-icon)
                    (process
                     . all-the-icons-ivy-rich-process-icon)
                    (custom-group
                     . all-the-icons-ivy-rich-group-settings-icon)
                    (custom-variable
                     . all-the-icons-ivy-rich-variable-settings-icon)
                    (charset
                     . all-the-icons-ivy-rich-charset-icon)
                    (coding-system
                     . all-the-icons-ivy-rich-coding-system-icon)
                    (language
                     . all-the-icons-ivy-rich-lang-icon)
                    (input-method
                     . all-the-icons-ivy-rich-input-method-icon)
                    (environment-key
                     . all-the-icons-ivy-rich-key-icon)
                    (lsp
                     . all-the-icons-ivy-rich-lsp-icon)))))"##;
    let expect = expect![[
        r#"OK ((directory " " (32 61462) (:inherit all-the-icons-silver :family "github-octicons" :height 1.0)) (project " " (32 61441) (:inherit all-the-icons-silver :family "github-octicons" :height 1.0)) (mode " " (32 61874) (:inherit all-the-icons-blue :family "FontAwesome" :height 1.0)) (command " " (32 61459) (:inherit all-the-icons-blue :family "FontAwesome" :height 1.0)) (history " " (32 59529) (:inherit all-the-icons-lblue :family "Material Icons" :height 1.0)) (face " " (32 58378) (:inherit all-the-icons-blue :family "Material Icons" :height 1.0)) (theme " " (32 58378) (:inherit all-the-icons-lcyan :family "Material Icons" :height 1.0)) (keybinding " " (32 61724) (:inherit all-the-icons-lsilver :family "FontAwesome" :height 1.0)) (library " " (32 59632) (:inherit all-the-icons-lblue :family "Material Icons" :height 1.0)) (package " " (32 61831) (:inherit all-the-icons-silver :family "FontAwesome" :height 1.0)) (font " " (32 61489) (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (world-clock " " (32 61612) (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (tramp " " (32 61488) (:inherit (:family "github-octicons" :height 0.96) :family "github-octicons" :height 1.0)) (git-branch " " (32 61472) (:inherit all-the-icons-green :family "github-octicons" :height 1.0)) (git-commit " " (32 61471) (:inherit all-the-icons-green :family "github-octicons" :height 1.0)) (process " " (32 61671) (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (custom-group " " (32 61564) (:inherit all-the-icons-lblue :family "github-octicons" :height 1.0)) (custom-variable " " (32 61564) (:inherit all-the-icons-lgreen :family "github-octicons" :height 1.0)) (charset " " (32 61646) (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (coding-system " " (32 61646) (:inherit all-the-icons-purple :family "FontAwesome" :height 1.0)) (language " " (32 61867) (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (input-method " " (32 61724) (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (environment-key " " (32 61513) (:inherit (:family "github-octicons" :height 0.96) :family "github-octicons" :height 1.0)) (lsp " " (32 61749) (:inherit all-the-icons-lgreen :family "FontAwesome" :height 1.0)))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn dynamic_function_variable_symbol_and_imenu_icons_follow_real_candidate_semantics() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (defun all-the-icons-ivy-rich-icon-command ()
                 (interactive))
               (defun all-the-icons-ivy-rich-icon-function ())
               (defcustom all-the-icons-ivy-rich-icon-custom t
                 "Fixture."
                 :type 'boolean)
               (defvar all-the-icons-ivy-rich-icon-variable t)
               (defface all-the-icons-ivy-rich-icon-face-fixture
                 '((t :inherit default))
                 "Fixture.")
               (cl-letf
                   (((symbol-function 'display-graphic-p)
                     (lambda (&optional _frame) t)))
                 (mapcar
                  (lambda (entry)
                    (let ((icon
                           (funcall
                            (car entry)
                            (cdr entry))))
                      (list
                       (car entry)
                       (cdr entry)
                       (substring-no-properties icon)
                       (get-text-property 1 'face icon))))
                  '((all-the-icons-ivy-rich-function-icon
                     . "all-the-icons-ivy-rich-icon-command")
                    (all-the-icons-ivy-rich-function-icon
                     . "all-the-icons-ivy-rich-icon-function")
                    (all-the-icons-ivy-rich-variable-icon
                     . "all-the-icons-ivy-rich-icon-custom")
                    (all-the-icons-ivy-rich-variable-icon
                     . "all-the-icons-ivy-rich-icon-variable")
                    (all-the-icons-ivy-rich-symbol-icon
                     . "all-the-icons-ivy-rich-icon-face-fixture")
                    (all-the-icons-ivy-rich-symbol-icon
                     . "Packages: fixture")
                    (all-the-icons-ivy-rich-imenu-icon
                     . "Functions: all-the-icons-ivy-rich-icon-function")
                    (all-the-icons-ivy-rich-imenu-icon
                     . "Variables: all-the-icons-ivy-rich-icon-variable")))))"##;
    let expect = expect![[
        r#"OK ((all-the-icons-ivy-rich-function-icon "all-the-icons-ivy-rich-icon-command" " " (:inherit all-the-icons-blue :family "FontAwesome" :height 1.0)) (all-the-icons-ivy-rich-function-icon "all-the-icons-ivy-rich-icon-function" " " (:inherit all-the-icons-purple :family "FontAwesome" :height 1.0)) (all-the-icons-ivy-rich-variable-icon "all-the-icons-ivy-rich-icon-custom" " " (:inherit all-the-icons-lblue :family "FontAwesome" :height 1.0)) (all-the-icons-ivy-rich-variable-icon "all-the-icons-ivy-rich-icon-variable" " " (:inherit all-the-icons-lblue :family "github-octicons" :height 1.0)) (all-the-icons-ivy-rich-symbol-icon "all-the-icons-ivy-rich-icon-face-fixture" " " (:inherit all-the-icons-blue :family "Material Icons" :height 1.0)) (all-the-icons-ivy-rich-symbol-icon "Packages: fixture" " " (:inherit all-the-icons-silver :family "FontAwesome" :height 1.0)) (all-the-icons-ivy-rich-imenu-icon "Functions: all-the-icons-ivy-rich-icon-function" " " (:inherit all-the-icons-purple :family "FontAwesome" :height 1.0)) (all-the-icons-ivy-rich-imenu-icon "Variables: all-the-icons-ivy-rich-icon-variable" " " (:inherit all-the-icons-lblue :family "github-octicons" :height 1.0)))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn bookmark_icons_distinguish_real_file_directory_and_missing_targets() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (let* ((root
                       (file-name-as-directory
                        (expand-file-name
                         "all-the-icons-ivy-rich-icon-bookmarks"
                         (getenv "TMPDIR"))))
                      (file
                       (expand-file-name "notes.md" root))
                      (bookmark-alist
                       `(("file" (filename . ,file))
                         ("directory" (filename . ,root))
                         ("missing"
                          (filename
                           . ,(expand-file-name
                              "missing.el" root))))))
                 (unwind-protect
                     (progn
                       (when (file-exists-p root)
                         (delete-directory root t))
                       (make-directory root t)
                       (with-temp-file file
                         (insert "# Notes\n"))
                       (cl-letf
                           (((symbol-function 'display-graphic-p)
                             (lambda (&optional _frame) t)))
                         (list
                         (mapcar
                           (lambda (candidate)
                             (let ((icon
                                    (all-the-icons-ivy-rich-bookmark-icon
                                     candidate)))
                               (list
                                candidate
                                (substring-no-properties icon))))
                           '("file"
                             "directory"
                             "missing"))))
                   (when (file-exists-p root)
                     (delete-directory root t))))))"##;
    let expect = expect!["OK nil"];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn grep_icons_parse_real_line_error_and_non_result_candidates() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (cl-letf
                   (((symbol-function 'display-graphic-p)
                     (lambda (&optional _frame) t)))
                 (mapcar
                  (lambda (candidate)
                    (let ((icon
                           (all-the-icons-ivy-rich-grep-file-icon
                            candidate)))
                      (list
                       candidate
                       (and icon
                            (substring-no-properties icon)))))
                  '("notes.md:12:heading"
                    "notes.md:error(failed)"
                    "not-a-result"))))"##;
    let expect = expect![[
        r#"OK (("notes.md:12:heading" " ") ("notes.md:error(failed)" " ") ("not-a-result" nil))"#
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn markdown_link_icons_distinguish_anchor_and_external_link_candidates() {
    let elisp_form = r##"(progn
               (require 'cl-lib)
               (cl-letf
                   (((symbol-function 'display-graphic-p)
                     (lambda (&optional _frame) t)))
                 (mapcar
                  (lambda (candidate)
                    (let ((icon
                           (all-the-icons-ivy-rich-link-icon
                            candidate)))
                      (list
                       candidate
                       (substring-no-properties icon)
                       (get-text-property 1 'face icon))))
                  '("#section"
                    "https://example.invalid/page"))))"##;
    let expect = expect![[
        r##"OK (("#section" " " (:inherit all-the-icons-green :family "FontAwesome" :height 1.0)) ("https://example.invalid/page" " " (:inherit all-the-icons-blue :family "Material Icons" :height 1.0)))"##
    ]];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}

#[test]
fn every_icon_entry_point_is_suppressed_in_the_real_batch_nongraphical_runtime() {
    let elisp_form = r##"(list
               (display-graphic-p)
               (mapcar
                (lambda (function)
                  (list
                   function
                   (funcall function "fixture")))
                '(all-the-icons-ivy-rich-file-icon
                  all-the-icons-ivy-rich-dir-icon
                  all-the-icons-ivy-rich-project-icon
                  all-the-icons-ivy-rich-function-icon
                  all-the-icons-ivy-rich-variable-icon
                  all-the-icons-ivy-rich-face-icon
                  all-the-icons-ivy-rich-theme-icon
                  all-the-icons-ivy-rich-library-icon
                  all-the-icons-ivy-rich-package-icon
                  all-the-icons-ivy-rich-font-icon
                  all-the-icons-ivy-rich-world-clock-icon
                  all-the-icons-ivy-rich-process-icon
                  all-the-icons-ivy-rich-key-icon
                  all-the-icons-ivy-rich-lsp-icon)))"##;
    let expect = expect![
        "OK (nil ((all-the-icons-ivy-rich-file-icon nil) (all-the-icons-ivy-rich-dir-icon nil) (all-the-icons-ivy-rich-project-icon nil) (all-the-icons-ivy-rich-function-icon nil) (all-the-icons-ivy-rich-variable-icon nil) (all-the-icons-ivy-rich-face-icon nil) (all-the-icons-ivy-rich-theme-icon nil) (all-the-icons-ivy-rich-library-icon nil) (all-the-icons-ivy-rich-package-icon nil) (all-the-icons-ivy-rich-font-icon nil) (all-the-icons-ivy-rich-world-clock-icon nil) (all-the-icons-ivy-rich-process-icon nil) (all-the-icons-ivy-rich-key-icon nil) (all-the-icons-ivy-rich-lsp-icon nil)))"
    ];

    assert_all_the_icons_ivy_rich_parity(elisp_form, expect);
}
