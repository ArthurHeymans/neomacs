use expect_test::expect;

use super::assert_apropospriate_theme_parity;

#[test]
fn apropospriate_variants_have_identical_face_and_variable_inventory() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-light t)
         (cl-labels
             ((inventory
               (theme kind)
               (mapcar
                #'cadr
                (cl-remove-if-not
                 (lambda (entry)
                   (eq (car entry) kind))
                 (get theme 'theme-settings)))))
           (let ((dark-faces
                  (inventory
                   'apropospriate-dark 'theme-face))
                 (light-faces
                  (inventory
                   'apropospriate-light 'theme-face))
                 (dark-vars
                  (inventory
                   'apropospriate-dark 'theme-value))
                 (light-vars
                  (inventory
                   'apropospriate-light 'theme-value)))
             (list
              (equal dark-faces light-faces)
              (length dark-faces)
              (cl-set-difference dark-faces light-faces)
              (cl-set-difference light-faces dark-faces)
              (equal dark-vars light-vars)
              dark-vars light-vars))))"##;
    let expect = expect![
        "OK (t 636 nil nil t (ansi-color-names-vector indent-bars-color tabbar-background-color highlight-tail-colors beacon-color vc-annotate-very-old-color vc-annotate-color-map highlight-symbol-colors highlight-symbol-foreground-color highlight-indent-guides-auto-enabled mlscroll-out-color mlscroll-in-color pos-tip-background-color pos-tip-foreground-color evil-visual-state-cursor evil-normal-state-cursor evil-insert-state-cursor evil-emacs-state-cursor diff-hl-show-hunk-posframe-internal-border-color) (ansi-color-names-vector indent-bars-color tabbar-background-color highlight-tail-colors beacon-color vc-annotate-very-old-color vc-annotate-color-map highlight-symbol-colors highlight-symbol-foreground-color highlight-indent-guides-auto-enabled mlscroll-out-color mlscroll-in-color pos-tip-background-color pos-tip-foreground-color evil-visual-state-cursor evil-normal-state-cursor evil-insert-state-cursor evil-emacs-state-cursor diff-hl-show-hunk-posframe-internal-border-color))"
    ];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_variants_contrast_core_face_specs_exactly() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-light t)
         (cl-labels
             ((spec
               (theme face)
               (cl-find-if
                (lambda (entry)
                  (and (eq (car entry) 'theme-face)
                       (eq (cadr entry) face)))
                (get theme 'theme-settings))))
           (mapcar
            (lambda (face)
              (list
               face
               (spec 'apropospriate-dark face)
               (spec 'apropospriate-light face)))
            '(default cursor highlight hl-line region
              header-line fringe vertical-border
              mode-line mode-line-inactive
              font-lock-comment-face
              font-lock-keyword-face
              font-lock-string-face
              font-lock-function-name-face))))"##;
    let expect = expect![[
        r##"OK ((default (theme-face default apropospriate-dark ((#1=((class color) (min-colors 89)) (:background "#424242" :foreground "#E0E0E0")))) (theme-face default apropospriate-light ((#1# (:background "#F5F5F5" :foreground "#546E7A"))))) (cursor (theme-face cursor apropospriate-dark ((#1# (:background "#FF80AB" . #2=(:inverse-video t))))) (theme-face cursor apropospriate-light ((#1# (:background "#EC407A" . #2#))))) (highlight (theme-face highlight apropospriate-dark ((#1# (:background "#494949")))) (theme-face highlight apropospriate-light ((#1# (:background "#F0F0F0"))))) (hl-line (theme-face hl-line apropospriate-dark ((#1# (:background "#494949")))) (theme-face hl-line apropospriate-light ((#1# (:background "#FBFBFB"))))) (region (theme-face region apropospriate-dark ((#1# (:background "#515151")))) (theme-face region apropospriate-light ((#1# (:background "#EBEBEB"))))) (header-line (theme-face header-line apropospriate-dark ((#1# (:foreground "#E1BEE7" . #3=(:background unspecified))))) (theme-face header-line apropospriate-light ((#1# (:foreground "#7E57C2" . #3#))))) (fringe (theme-face fringe apropospriate-dark ((#1# (:background "#424242" :foreground "#9E9E9E")))) (theme-face fringe apropospriate-light ((#1# (:background "#F5F5F5" :foreground "#78909C"))))) (vertical-border (theme-face vertical-border apropospriate-dark ((#1# (:foreground "#515151")))) (theme-face vertical-border apropospriate-light ((#1# (:foreground "#EBEBEB"))))) (mode-line (theme-face mode-line apropospriate-dark ((#1# (:box (:line-width 4 :color "#2A2A2A" . #4=(:style nil)) :background "#323232" :foreground "#E0E0E0" :height 0.95)))) (theme-face mode-line apropospriate-light ((#1# (:box (:line-width 4 :color "#E6E6E6" . #4#) :background "#FDFDFD" :foreground "#546E7A" :height 0.95))))) (mode-line-inactive (theme-face mode-line-inactive apropospriate-dark ((#1# (:box (:line-width 4 :color "#494949" . #5=(:style nil)) :background "#494949" :foreground "#9E9E9E" :height 0.95)))) (theme-face mode-line-inactive apropospriate-light ((#1# (:box (:line-width 4 :color "#F0F0F0" . #5#) :background "#F0F0F0" :foreground "#78909C" :height 0.95))))) (font-lock-comment-face (theme-face font-lock-comment-face apropospriate-dark ((#1# (:foreground "#757575")))) (theme-face font-lock-comment-face apropospriate-light ((#1# (:foreground "#90A4AE"))))) (font-lock-keyword-face (theme-face font-lock-keyword-face apropospriate-dark ((#1# (:foreground "#E1BEE7")))) (theme-face font-lock-keyword-face apropospriate-light ((#1# (:foreground "#7E57C2"))))) (font-lock-string-face (theme-face font-lock-string-face apropospriate-dark ((#1# (:foreground "#C5E1A5")))) (theme-face font-lock-string-face apropospriate-light ((#1# (:foreground "#66BB6A"))))) (font-lock-function-name-face (theme-face font-lock-function-name-face apropospriate-dark ((#1# (:foreground "#64B5F6")))) (theme-face font-lock-function-name-face apropospriate-light ((#1# (:foreground "#42A5F5"))))))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_variants_contrast_all_theme_variable_values() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-light t)
         (cl-labels
             ((values
               (theme)
               (cl-remove-if-not
                (lambda (entry)
                  (eq (car entry) 'theme-value))
                (get theme 'theme-settings))))
           (list
            (values 'apropospriate-dark)
            (values 'apropospriate-light))))"##;
    let expect = expect![[
        r##"OK (((theme-value ansi-color-names-vector apropospriate-dark ["#424242" "#EF9A9A" "#C5E1A5" "#FFEE58" "#64B5F6" "#E1BEE7" "#80DEEA" "#E0E0E0"]) (theme-value indent-bars-color apropospriate-dark #1='(highlight :face-bg t :blend 0.123)) (theme-value tabbar-background-color apropospriate-dark "#323232") (theme-value highlight-tail-colors apropospriate-dark '(("#EE758C" . 0) ("#424242" . 100))) (theme-value beacon-color apropospriate-dark "#EE758C") (theme-value vc-annotate-very-old-color apropospriate-dark "#9E9E9E") (theme-value vc-annotate-color-map apropospriate-dark '((20 . "#E57373") (40 . "#FFA726") (60 . "#FFCC80") (80 . "#FFF59D") (100 . "#FFEE58") (120 . "#C5E1A5") (140 . "#C5E1A5") (160 . "#C5E1A5") (180 . "#C5E1A5") (200 . "#F4FF81") (220 . "#F4FF81") (240 . "#80DEEA") (260 . "#80DEEA") (280 . "#26C6DA") (300 . "#26C6DA") (320 . "#64B5F6") (340 . "#64B5F6") (360 . "#42A5F5"))) (theme-value highlight-symbol-colors apropospriate-dark '("#FFEE58" "#C5E1A5" "#80DEEA" "#64B5F6" "#E1BEE7" "#FFCC80")) (theme-value highlight-symbol-foreground-color apropospriate-dark "#E0E0E0") (theme-value highlight-indent-guides-auto-enabled apropospriate-dark nil) (theme-value mlscroll-out-color apropospriate-dark "#424242") (theme-value mlscroll-in-color apropospriate-dark "#515151") (theme-value pos-tip-background-color apropospriate-dark "#3A3A3A") (theme-value pos-tip-foreground-color apropospriate-dark "#E0E0E0") (theme-value evil-visual-state-cursor apropospriate-dark '("#C5E1A5" . #2=(box))) (theme-value evil-normal-state-cursor apropospriate-dark '("#FFEE58" . #3=(box))) (theme-value evil-insert-state-cursor apropospriate-dark '("#E57373" . #4=(bar))) (theme-value evil-emacs-state-cursor apropospriate-dark '("#E57373" . #5=(hbar))) (theme-value diff-hl-show-hunk-posframe-internal-border-color apropospriate-dark "#323232")) ((theme-value ansi-color-names-vector apropospriate-light ["#F5F5F5" "#FF1744" "#66BB6A" "#F57F17" "#42A5F5" "#7E57C2" "#0097A7" "#546E7A"]) (theme-value indent-bars-color apropospriate-light #1#) (theme-value tabbar-background-color apropospriate-light "#FDFDFD") (theme-value highlight-tail-colors apropospriate-light '(("#FCE2EB" . 0) ("#F5F5F5" . 100))) (theme-value beacon-color apropospriate-light "#FCE2EB") (theme-value vc-annotate-very-old-color apropospriate-light "#78909C") (theme-value vc-annotate-color-map apropospriate-light '((20 . "#D50000") (40 . "#FF5722") (60 . "#D84315") (80 . "#F9A725") (100 . "#F57F17") (120 . "#66BB6A") (140 . "#66BB6A") (160 . "#66BB6A") (180 . "#66BB6A") (200 . "#558B2F") (220 . "#558B2F") (240 . "#0097A7") (260 . "#0097A7") (280 . "#00B8D4") (300 . "#00B8D4") (320 . "#42A5F5") (340 . "#42A5F5") (360 . "#1E88E5"))) (theme-value highlight-symbol-colors apropospriate-light '("#F57F17" "#66BB6A" "#0097A7" "#42A5F5" "#7E57C2" "#D84315")) (theme-value highlight-symbol-foreground-color apropospriate-light "#546E7A") (theme-value highlight-indent-guides-auto-enabled apropospriate-light nil) (theme-value mlscroll-out-color apropospriate-light "#F5F5F5") (theme-value mlscroll-in-color apropospriate-light "#EBEBEB") (theme-value pos-tip-background-color apropospriate-light "#FBFBFB") (theme-value pos-tip-foreground-color apropospriate-light "#546E7A") (theme-value evil-visual-state-cursor apropospriate-light '("#66BB6A" . #2#)) (theme-value evil-normal-state-cursor apropospriate-light '("#F57F17" . #3#)) (theme-value evil-insert-state-cursor apropospriate-light '("#D50000" . #4#)) (theme-value evil-emacs-state-cursor apropospriate-light '("#D50000" . #5#)) (theme-value diff-hl-show-hunk-posframe-internal-border-color apropospriate-light "#FDFDFD")))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_switches_between_variants_in_real_theme_lifecycle() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (let ((dark
                (list
                 custom-enabled-themes
                 (face-attribute
                  'default :foreground nil 'default)
                 (face-attribute
                  'default :background nil 'default)
                 (face-attribute
                  'font-lock-string-face
                  :foreground nil 'default))))
           (disable-theme 'apropospriate-dark)
           (load-theme 'apropospriate-light t)
           (let ((light
                  (list
                   custom-enabled-themes
                   (face-attribute
                    'default :foreground nil 'default)
                   (face-attribute
                    'default :background nil 'default)
                   (face-attribute
                    'font-lock-string-face
                    :foreground nil 'default))))
             (list dark light))))"##;
    let expect = expect![[
        r#"OK (((apropospriate-dark) "unspecified-fg" "unspecified-bg" "unspecified-fg") ((apropospriate-light) "unspecified-fg" "unspecified-bg" "unspecified-fg"))"#
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_stacking_variants_obeys_enabled_theme_precedence() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-light t)
         (let ((stacked
                (list
                 custom-enabled-themes
                 (face-attribute
                  'default :background nil 'default)
                 (face-attribute
                  'font-lock-keyword-face
                  :foreground nil 'default))))
           (disable-theme 'apropospriate-light)
           (list
            stacked
            custom-enabled-themes
            (face-attribute
             'default :background nil 'default)
            (face-attribute
             'font-lock-keyword-face
             :foreground nil 'default))))"##;
    let expect = expect![[
        r#"OK (((apropospriate-light . #1=(apropospriate-dark)) "unspecified-bg" "unspecified-fg") #1# "unspecified-bg" "unspecified-fg")"#
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_face_inventory_duplicate_definitions_are_preserved() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-light t)
         (mapcar
          (lambda (theme)
            (let ((faces
                   (mapcar
                    #'cadr
                    (cl-remove-if-not
                     (lambda (entry)
                       (eq (car entry) 'theme-face))
                     (get theme 'theme-settings)))))
              (list
               theme
               (cl-count 'helm-visible-mark faces)
               (cl-count 'org-block-end-line faces)
               (cl-count 'org-ellipsis faces)
               (cl-remove-if
                (lambda (face)
                  (= (cl-count face faces) 1))
                (delete-dups (copy-sequence faces))))))
          '(apropospriate-dark apropospriate-light)))"##;
    let expect = expect![
        "OK ((apropospriate-dark 2 2 2 (org-ellipsis org-block-end-line helm-visible-mark)) (apropospriate-light 2 2 2 (org-ellipsis org-block-end-line helm-visible-mark)))"
    ];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_recreating_same_theme_replaces_settings_deterministically() {
    let elisp_form = r##"(let ((theme 'apropospriate-recreated))
         (custom-declare-theme
          theme 'apropospriate-recreated-theme
          "Parity fixture" nil)
         (let ((apropospriate-mode-line-height 0.8)
               (apropospriate-org-level-resizing t))
           (create-apropospriate-theme 'dark theme))
         (let ((first (copy-tree (get theme 'theme-settings))))
           (let ((apropospriate-mode-line-height nil)
                 (apropospriate-org-level-resizing nil))
             (create-apropospriate-theme 'light theme))
           (let ((second (get theme 'theme-settings)))
             (list
              (length first)
              (length second)
              (= (length first) (length second))
              (cl-count 'default
                        (mapcar #'cadr second))
              (cl-find-if
               (lambda (entry)
                 (and (eq (car entry) 'theme-face)
                      (eq (cadr entry) 'default)))
               second)
              (cl-find-if
               (lambda (entry)
                 (and (eq (car entry) 'theme-face)
                      (eq (cadr entry) 'org-level-1)))
               second)))))"##;
    let expect = expect![[
        r##"OK (655 1310 nil 2 (theme-face default apropospriate-recreated ((#1=((class color) (min-colors 89)) (:background "#F5F5F5" :foreground "#546E7A")))) (theme-face org-level-1 apropospriate-recreated ((#1# (:inherit header-line :height 1.0)))))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_variant_argument_uses_dark_branch_for_non_light_values() {
    let elisp_form = r##"(mapcar
         (lambda (variant)
           (apropospriate-with-color-variables variant
             (list variant base00 base03 red green
                   light-emphasis highlight-line-color)))
         '(light dark nil unknown 42))"##;
    let expect = expect![[
        r##"OK ((light "#F5F5F5" "#546E7A" "#D50000" "#66BB6A" "#E6E6E6" "#EEEEEE") (dark "#424242" "#E0E0E0" "#E57373" "#C5E1A5" "#2A2A2A" "#444444") (nil "#424242" "#E0E0E0" "#E57373" "#C5E1A5" "#2A2A2A" "#444444") (unknown "#424242" "#E0E0E0" "#E57373" "#C5E1A5" "#2A2A2A" "#444444") (42 "#424242" "#E0E0E0" "#E57373" "#C5E1A5" "#2A2A2A" "#444444"))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}
