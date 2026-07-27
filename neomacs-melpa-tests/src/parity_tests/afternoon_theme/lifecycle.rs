use expect_test::expect;

use super::assert_afternoon_theme_with_prelude_parity;

#[test]
fn afternoon_theme_real_load_enable_disable_cycle_applies_and_restores_all_variables() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(progn
         (defvar fci-rule-color nil)
         (defvar vc-annotate-color-map nil)
         (defvar vc-annotate-very-old-color nil)
         (defvar vc-annotate-background nil)
         (defvar ansi-color-names-vector nil)
         (defvar ansi-color-faces-vector nil)
         (let* ((symbols
                 '(fci-rule-color
                   vc-annotate-color-map
                   vc-annotate-very-old-color
                   vc-annotate-background
                   ansi-color-names-vector
                   ansi-color-faces-vector))
                (originals
                 (mapcar
                  (lambda (symbol)
                    (cons symbol (default-value symbol)))
                  symbols)))
           (unwind-protect
               (progn
                 (when (custom-theme-enabled-p 'afternoon)
                   (disable-theme 'afternoon))
                 (dolist (symbol symbols)
                   (set-default symbol
                                (intern
                                 (format "before-%s" symbol))))
                 (let ((loaded (load-theme 'afternoon t)))
                   (let ((enabled
                          (list
                           loaded
                           (custom-theme-enabled-p 'afternoon)
                           custom-enabled-themes
                           (mapcar
                            (lambda (symbol)
                              (list
                               symbol
                               (default-value symbol)))
                            symbols))))
                     (disable-theme 'afternoon)
                     (list
                      enabled
                      (custom-theme-enabled-p 'afternoon)
                      custom-enabled-themes
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (default-value symbol)))
                       symbols)))))
             (when (custom-theme-enabled-p 'afternoon)
               (disable-theme 'afternoon))
             (dolist (entry originals)
               (set-default (car entry) (cdr entry))))))"##;
    let expect = expect![[
        r##"OK ((t #1=(afternoon) #1# ((fci-rule-color "#14151E") (vc-annotate-color-map ((20 . "#d54e53") (40 . "goldenrod") (60 . "#e7c547") (80 . "DarkOliveGreen3") (100 . "#70c0b1") (120 . "DeepSkyBlue1") (140 . "#c397d8") (160 . "#d54e53") (180 . "goldenrod") (200 . "#e7c547") (220 . "DarkOliveGreen3") (240 . "#70c0b1") (260 . "DeepSkyBlue1") (280 . "#c397d8") (300 . "#d54e53") (320 . "goldenrod") (340 . "#e7c547") (360 . "DarkOliveGreen3"))) (vc-annotate-very-old-color nil) (vc-annotate-background nil) (ansi-color-names-vector ["#eaeaea" "#d54e53" "DarkOliveGreen3" "#e7c547" "DeepSkyBlue1" "#c397d8" "#70c0b1" "#181a26"]) (ansi-color-faces-vector [default bold shadow italic underline bold bold-italic bold]))) nil nil ((fci-rule-color before-fci-rule-color) (vc-annotate-color-map before-vc-annotate-color-map) (vc-annotate-very-old-color before-vc-annotate-very-old-color) (vc-annotate-background before-vc-annotate-background) (ansi-color-names-vector before-ansi-color-names-vector) (ansi-color-faces-vector before-ansi-color-faces-vector)))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_enabled_ansi_palette_renders_real_colored_text_properties() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(progn
         (require 'ansi-color)
         (when (custom-theme-enabled-p 'afternoon)
           (disable-theme 'afternoon))
         (unwind-protect
             (progn
               (load-theme 'afternoon t)
               (let ((rendered
                      (ansi-color-apply
                       "\e[31merror\e[0m plain \e[32;1mok\e[0m")))
                 (list
                  (substring-no-properties rendered)
                  (mapcar
                   (lambda (index)
                     (list
                      index
                      (aref rendered index)
                      (copy-tree
                       (text-properties-at index rendered))))
                   '(0 4 5 11 12 13))
                  (append ansi-color-names-vector nil)
                  (append ansi-color-faces-vector nil))))
           (when (custom-theme-enabled-p 'afternoon)
             (disable-theme 'afternoon))))"##;
    let expect = expect![[
        r##"OK ("error plain ok" ((0 101 (font-lock-face (:foreground "red3"))) (4 114 (font-lock-face (:foreground "red3"))) (5 32 nil) (11 32 nil) (12 111 (font-lock-face (ansi-color-bold (:foreground "green3")))) (13 107 (font-lock-face (ansi-color-bold (:foreground "green3"))))) ("#eaeaea" "#d54e53" "DarkOliveGreen3" "#e7c547" "DeepSkyBlue1" "#c397d8" "#70c0b1" "#181a26") (default bold shadow italic underline bold bold-italic bold))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}
