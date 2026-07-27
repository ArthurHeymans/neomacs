use expect_test::expect;

use super::assert_ahungry_theme_parity;

#[test]
fn ahungry_theme_real_emacs_lisp_font_lock_uses_theme_faces_and_colors() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ahungry)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert ";; note\n(defun demo (value)\n  (let ((text \"hello\")) value))\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (let* ((face (get-text-property (1- (point)) 'face))
                       (resolved (if (listp face) (car face) face)))
                  (list
                   needle
                   face
                   (and resolved
                        (face-attribute resolved :foreground nil 'default)))))
              '("note" "defun" "demo" "value" "hello"))))
       (disable-theme 'ahungry))"##;
    let expect = expect![[
        r##"OK (("note" font-lock-comment-face "#888a85") ("defun" font-lock-keyword-face "#3cff00") ("demo" font-lock-function-name-face "#ffee00") ("value" nil nil) ("hello" font-lock-string-face "#ff0077"))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_real_org_document_exposes_heading_link_todo_and_block_faces() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ahungry)
           (require 'org)
           (with-temp-buffer
             (org-mode)
             (insert "* TODO Ship release\nSee [[https://example.test][docs]].\n#+begin_src emacs-lisp\n(message \"ok\")\n#+end_src\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (let ((face (get-text-property (1- (point)) 'face)))
                  (list needle face)))
              '("TODO" "Ship release" "docs" "#+begin_src" "message"))))
       (disable-theme 'ahungry))"##;
    let expect = expect![[
        r##"OK (("TODO" (org-todo org-level-1)) ("Ship release" org-level-1) ("docs" org-link) ("#+begin_src" org-block-begin-line) ("message" (org-block)))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_real_diff_buffer_uses_added_removed_hunk_and_header_faces() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ahungry)
           (require 'diff-mode)
           (with-temp-buffer
             (insert "diff --git a/a.txt b/a.txt\nindex 111..222 100644\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-old\n+new\n context\n")
             (diff-mode)
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (list needle
                      (get-text-property (1- (point)) 'face)))
              '("diff --git" "@@ -1" "-old" "+new" " context"))))
       (disable-theme 'ahungry))"##;
    let expect = expect![[
        r#"OK (("diff --git" diff-header) ("@@ -1" diff-hunk-header) ("-old" diff-removed) ("+new" diff-added) (" context" diff-context))"#
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_load_theme_uses_installed_theme_path_and_real_activation() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'ahungry-theme package-alist)))
                 (directory
                  (file-name-as-directory
                   (package-desc-dir descriptor)))
                 (custom-theme-load-path
                  (cons directory custom-theme-load-path)))
         (unwind-protect
             (progn
               (require 'org)
               (when (memq 'ahungry custom-enabled-themes)
                 (disable-theme 'ahungry))
               (load-theme 'ahungry t)
               (list
                (car custom-enabled-themes)
                (face-attribute 'font-lock-keyword-face
                                :foreground nil 'default)
                (face-attribute 'org-level-1
                                :height nil 'default)
                (member directory custom-theme-load-path)))
           (when (memq 'ahungry custom-enabled-themes)
             (disable-theme 'ahungry))))"##;
    let expect = expect![[
        r##"OK (ahungry "#3cff00" 182 ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ahungry-theme/20180131.328/home/.emacs.d/elpa/ahungry-theme-20180131.328/" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ahungry-theme/20180131.328/home/.emacs.d/elpa/ahungry-theme-20180131.328/" custom-theme-directory t))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_external_package_faces_resolve_when_created_after_theme_load() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ahungry)
           (mapcar
            (lambda (face)
              (unless (facep face)
                (make-face face))
              (list
               face
               (face-attribute face :foreground nil 'default)
               (face-attribute face :background nil 'default)
               (face-attribute face :weight nil 'default)))
            '(magit-diff-added
              helm-selection
              rainbow-delimiters-depth-3-face
              eyebrowse-mode-line-active
              hackernews-link)))
       (disable-theme 'ahungry))"##;
    let expect = expect![[
        r##"OK ((magit-diff-added "#ffffff" unspecified normal) (helm-selection "#ffffff" unspecified normal) (rainbow-delimiters-depth-3-face "#ffffff" unspecified normal) (eyebrowse-mode-line-active "#ffffff" unspecified normal) (hackernews-link "#ffffff" unspecified normal))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_face_inheritance_resolves_org_quote_and_tooltip_usage() {
    let elisp_form = r##"(unwind-protect
         (progn
           (require 'org)
           (require 'cus-edit)
           (unless (facep 'tool-tips)
             (make-face 'tool-tips))
           (enable-theme 'ahungry)
           (mapcar
            (lambda (face)
              (list
               face
               (face-attribute face :inherit nil nil)
               (face-attribute face :foreground nil 'default)
               (face-attribute face :background nil 'default)
               (face-attribute face :slant nil 'default)
               (face-attribute face :weight nil 'default)))
            '(org-block org-quote org-verse custom-link tooltip tool-tips)))
       (disable-theme 'ahungry))"##;
    let expect = expect![[
        r##"OK ((org-block unspecified "#999999" unspecified normal normal) (org-quote org-block "#999999" unspecified italic bold) (org-verse org-block "#999999" unspecified italic bold) (custom-link 'link "#ffffff" unspecified normal normal) (tooltip 'variable-pitch "black" "#ffff33" normal normal) (tool-tips 'variable-pitch "black" "#ffff33" normal normal))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}
