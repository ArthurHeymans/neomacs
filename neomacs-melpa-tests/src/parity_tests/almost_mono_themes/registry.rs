use expect_test::expect;

use super::{assert_almost_mono_themes_parity, assert_almost_mono_themes_signal_parity};

#[test]
fn almost_mono_themes_loads_exact_package_and_registers_theme_directory() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'almost-mono-themes package-alist)))
       (directory (file-name-as-directory
                   (package-desc-dir description))))
  (list
   (featurep 'almost-mono-themes)
   (package-installed-p 'almost-mono-themes)
   (package-version-join (package-desc-version description))
   (member directory custom-theme-load-path)
   (file-readable-p
    (expand-file-name "almost-mono-white-theme.el" directory))
   (file-readable-p
    (expand-file-name "almost-mono-black-theme.el" directory))))"##;
    let expect = expect![[
        r#"OK (t t "20250722.1957" ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/almost-mono-themes/20250722.1957/home/.emacs.d/elpa/almost-mono-themes-20250722.1957/" custom-theme-directory t) t t)"#
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn almost_mono_themes_palette_contains_complete_ordered_rendering_roles() {
    let elisp_form = r##"(mapcar
 (lambda (variant)
   (let ((colors (cdr (assq variant almost-mono-themes-colors))))
     (list variant
           (length colors)
           (mapcar #'car colors)
           (mapcar #'cdr colors))))
 '(white black gray cream))"##;
    let expect = expect![[
        r##"OK ((white 9 (background foreground weak weaker weakest highlight warning success string) ("#ffffff" "#000000" "#888888" "#dddddd" "#efefef" "#fda50f" "#ff0000" "#00ff00" "#3c5e2b")) (black 9 (background foreground weak weaker weakest highlight warning success string) ("#000000" "#ffffff" "#aaaaaa" "#666666" "#222222" "#fda50f" "#ff0000" "#00ff00" "#a7bca4")) (gray 9 (background foreground weak weaker weakest highlight warning success string) ("#2b2b2b" "#ffffff" "#aaaaaa" "#666666" "#222222" "#fda50f" "#ff0000" "#00ff00" "#a7bca4")) (cream 9 (background foreground weak weaker weakest highlight warning success string) ("#f0e5da" "#000000" "#7d7165" "#c4baaf" "#dbd0c5" "#fda50f" "#ff0000" "#00ff00" "#3c5e2b")))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn variant_name_builds_theme_symbols_for_real_and_extension_variants() {
    let elisp_form = r##"(mapcar
 (lambda (variant)
   (let ((name (almost-mono-themes--variant-name variant)))
     (list variant name (symbolp name) (symbol-name name))))
 '(white black gray cream sepia high-contrast))"##;
    let expect = expect![[
        r#"OK ((white almost-mono-white t "almost-mono-white") (black almost-mono-black t "almost-mono-black") (gray almost-mono-gray t "almost-mono-gray") (cream almost-mono-cream t "almost-mono-cream") (sepia almost-mono-sepia t "almost-mono-sepia") (high-contrast almost-mono-high-contrast t "almost-mono-high-contrast"))"#
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn variant_with_colors_binds_every_role_for_each_real_theme() {
    let elisp_form = r##"(mapcar
 (lambda (variant)
   (eval
    `(almost-mono-themes--variant-with-colors
      ',variant
      (list background foreground weak weaker weakest
            highlight warning success string))))
 '(white black gray cream))"##;
    let expect = expect![[
        r##"OK (("#ffffff" "#000000" "#888888" "#dddddd" "#efefef" "#fda50f" "#ff0000" "#00ff00" "#3c5e2b") ("#000000" "#ffffff" "#aaaaaa" "#666666" "#222222" "#fda50f" "#ff0000" "#00ff00" "#a7bca4") ("#2b2b2b" "#ffffff" "#aaaaaa" "#666666" "#222222" "#fda50f" "#ff0000" "#00ff00" "#a7bca4") ("#f0e5da" "#000000" "#7d7165" "#c4baaf" "#dbd0c5" "#fda50f" "#ff0000" "#00ff00" "#3c5e2b"))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn variant_with_colors_rejects_an_unknown_theme_before_running_body() {
    let elisp_form = r##"(almost-mono-themes--variant-with-colors
 'missing-variant
 (list background foreground))"##;
    let expect = expect![[r#"ERR (error "No such theme variant")"#]];
    assert_almost_mono_themes_signal_parity(elisp_form, expect);
}

#[test]
fn faces_spec_generates_complete_face_entries_and_preserves_duplicate_override_order() {
    let elisp_form = r##"(almost-mono-themes--variant-with-colors
 'white
 (let* ((spec (almost-mono-themes--faces-spec))
        (names (mapcar #'car spec))
        (variable-entries
         (delq nil
               (mapcar
                (lambda (entry)
                  (and (eq (car entry)
                           'font-lock-variable-name-face)
                       entry))
                spec))))
   (list
    (length spec)
    (length (delete-dups (copy-sequence names)))
    (car names)
    (car (last names))
    variable-entries
    (assq 'default spec)
    (assq 'mode-line spec)
    (assq 'font-lock-warning-face spec))))"##;
    let expect = expect![[
        r##"OK (73 72 default orderless-match-face-3 ((font-lock-variable-name-face ((t (:foreground "#000000")))) (font-lock-variable-name-face ((t (:foreground "#000000" :italic t))))) (default ((t (:background "#ffffff" :foreground "#000000")))) (mode-line ((t (:box (:line-width -1 :color "#dddddd") :background "#efefef" :foreground "#000000")))) (font-lock-warning-face ((t (:foreground "#000000" :underline (:color "#ff0000" :style wave))))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}
