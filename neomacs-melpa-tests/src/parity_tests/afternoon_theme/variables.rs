use expect_test::expect;

use super::assert_afternoon_theme_with_prelude_parity;

#[test]
fn afternoon_theme_true_color_variable_settings_are_complete_and_exact() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let ((settings
                (seq-filter
                 (lambda (setting)
                   (eq (car setting) 'theme-value))
                 (reverse
                  (copy-sequence
                   (get 'afternoon 'theme-settings))))))
         (list
          (length settings)
          (mapcar
           (lambda (setting)
             (list
              (nth 1 setting)
              (nth 2 setting)
              (nth 3 setting)))
           settings)
          (secure-hash
           'sha256
           (prin1-to-string
            (mapcar
             (lambda (setting)
               (secure-hash
                'sha256
                (prin1-to-string setting)))
             settings)))))"##;
    let expect = expect![[
        r##"OK (6 ((fci-rule-color afternoon "#14151E") (vc-annotate-color-map afternoon '((20 . "#d54e53") (40 . "goldenrod") (60 . "#e7c547") (80 . "DarkOliveGreen3") (100 . "#70c0b1") (120 . "DeepSkyBlue1") (140 . "#c397d8") (160 . "#d54e53") (180 . "goldenrod") (200 . "#e7c547") (220 . "DarkOliveGreen3") (240 . "#70c0b1") (260 . "DeepSkyBlue1") (280 . "#c397d8") (300 . "#d54e53") (320 . "goldenrod") (340 . "#e7c547") (360 . "DarkOliveGreen3"))) (vc-annotate-very-old-color afternoon nil) (vc-annotate-background afternoon nil) (ansi-color-names-vector afternoon (vector "#eaeaea" "#d54e53" "DarkOliveGreen3" "#e7c547" "DeepSkyBlue1" "#c397d8" "#70c0b1" "#181a26")) (ansi-color-faces-vector afternoon [default bold shadow italic underline bold bold-italic bold])) "bae9f0f9d410ade7b78ac107a14e71d6fc79c8df0ab30d6bd23c18875fd76841")"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_256_color_variable_settings_follow_the_terminal_palette() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 256))"##;
    let elisp_form = r##"(let ((settings
                (seq-filter
                 (lambda (setting)
                   (eq (car setting) 'theme-value))
                 (reverse
                  (copy-sequence
                   (get 'afternoon 'theme-settings))))))
         (mapcar
          (lambda (setting)
            (list
             (nth 1 setting)
             (nth 3 setting)))
          settings))"##;
    let expect = expect![[
        r##"OK ((fci-rule-color "#121212") (vc-annotate-color-map '((20 . "#d54e53") (40 . "goldenrod") (60 . "#e7c547") (80 . "DarkOliveGreen3") (100 . "#70c0b1") (120 . "DeepSkyBlue1") (140 . "#c397d8") (160 . "#d54e53") (180 . "goldenrod") (200 . "#e7c547") (220 . "DarkOliveGreen3") (240 . "#70c0b1") (260 . "DeepSkyBlue1") (280 . "#c397d8") (300 . "#d54e53") (320 . "goldenrod") (340 . "#e7c547") (360 . "DarkOliveGreen3"))) (vc-annotate-very-old-color nil) (vc-annotate-background nil) (ansi-color-names-vector (vector "#eaeaea" "#d54e53" "DarkOliveGreen3" "#e7c547" "DeepSkyBlue1" "#c397d8" "#70c0b1" "#1c1c1c")) (ansi-color-faces-vector [default bold shadow italic underline bold bold-italic bold]))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_vc_age_gradient_and_ansi_palette_have_practical_lookup_contracts() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let* ((settings
                 (get 'afternoon 'theme-settings))
                (value
                 (lambda (symbol)
                   (nth
                    3
                    (seq-find
                     (lambda (setting)
                       (and
                        (eq (car setting) 'theme-value)
                        (eq (nth 1 setting) symbol)))
                     settings))))
                (age-map
                 (eval
                  (funcall value 'vc-annotate-color-map)))
                (ansi-names
                 (eval
                  (funcall value 'ansi-color-names-vector))))
         (list
          (mapcar
           (lambda (age)
             (cons
              age
              (cdr
               (or
                (assq age age-map)
                (seq-find
                 (lambda (entry)
                   (> (car entry) age))
                 age-map)
                (car (last age-map))))))
           '(20 75 140 215 280 359 500))
          (append ansi-names nil)
          (mapcar
           (lambda (index)
             (aref ansi-names index))
           '(0 1 2 3 4 5 6 7))))"##;
    let expect = expect![[
        r##"OK (((20 . "#d54e53") (75 . "DarkOliveGreen3") (140 . "#c397d8") (215 . "DarkOliveGreen3") (280 . "#c397d8") (359 . "DarkOliveGreen3") (500 . "DarkOliveGreen3")) ("#eaeaea" "#d54e53" "DarkOliveGreen3" "#e7c547" "DeepSkyBlue1" "#c397d8" "#70c0b1" "#181a26") ("#eaeaea" "#d54e53" "DarkOliveGreen3" "#e7c547" "DeepSkyBlue1" "#c397d8" "#70c0b1" "#181a26"))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}
