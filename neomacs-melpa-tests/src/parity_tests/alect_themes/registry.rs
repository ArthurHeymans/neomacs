use expect_test::expect;

use super::{assert_alect_themes_autoload_parity, assert_alect_themes_parity};

#[test]
fn exact_release_descriptor_groups_commands_and_customization_surface() {
    let elisp_form = r##"
(let* ((descriptor
        (cadr (assq 'alect-themes package-alist)))
       (extras (package-desc-extras descriptor)))
  (list
   (package-desc-name descriptor)
   (package-version-join (package-desc-version descriptor))
   (package-desc-reqs descriptor)
   (alist-get :commit extras)
   (alist-get :url extras)
   (featurep 'alect-themes)
   (get 'alect 'group-documentation)
   (get 'alect-faces 'group-documentation)
   (mapcar
    (lambda (option)
      (list
       option
       (custom-variable-p option)
       (get option 'custom-type)
       (get option 'custom-group)))
    '(alect-header-height
      alect-single-title-height
      alect-multiple-titles-height
      alect-overriding-faces
      alect-colors
      alect-inverted-color-regexp
      alect-display-class
      alect-ignored-faces
      alect-ignored-variables))))
"##;
    let expect = expect![[
        r##"OK (alect-themes "20251205.1503" ((emacs (24 0))) "b1f97e4bc0dc6ec91c7e9999fbe9fa371016463b" "https://github.com/alezost/alect-themes" t "Options for alect color themes." "Auxiliary faces used by alect color themes." ((alect-header-height ((funcall #'#[nil (1.13) #1=(t)])) number nil) (alect-single-title-height ((funcall #'#[nil (1.13) #1#])) number nil) (alect-multiple-titles-height ((funcall #'#[nil (1.13) #1#])) number nil) (alect-overriding-faces ((funcall #'#[nil (nil) #1#])) sexp nil) (alect-colors ((funcall #'#[nil ((alect-generate-colors '(light dark black) '((cursor "#1074cd" "#d0d060" "#b1c721") (gray-2 "#fafafa" "#e9e9e9" "#dedede") (gray-1 "#adadad" "#c0c0c0" "#bababa") (gray "#909090" "#9f9f9f" "#9b9b9b") (gray+1 "#444444" "#505050" "#555555") (gray+2 "#070707" "#000000" "#000000") (fg-2 "#6c6c6c" "#8c826d" "#8b806c") (fg-1 "#505050" "#d0bf8f" "#ab9861") (fg "#3f3f3f" "#f0dfaf" "#c4ad63") (fg+1 "#262626" "#d5d2be" "#b2af95") (fg+2 "#101010" "#f6f0e1" "#d6cbae") (bg-2 "#f6f0e1" "#222222" "#404040") (bg-1 "#ded6c5" "#3f3f3f" "#000000") (bg-0.5 "#dcd2bd" "#464646" "#101010") (bg "#d9ceb2" "#4f4f4f" "#202020") (bg+1 "#d4caa7" "#5f5f5f" "#303030") (bg+2 "#ccc19b" "#6f6f6f" "#454545") (red-2 "#fa5151" "#fa6a6e" "#e96060") (red-1 "#e43838" "#fa5151" "#ea4141") (red "#f71010" "#ea3838" "#db4334") (red+1 "#d81212" "#db4334" "#c83029") (red+2 "#b22222" "#c83029" "#ae2823") (red-bg-1 "#ff6868" "#c64242" "#a52621") (red-bg "#fb9494" "#a83838" "#86201c") (red-bg+1 "#eec5c5" "#6a3636" "#531311") (yellow-2 "#ab9c3a" "#f8ffa0" "#e9e953") (yellow-1 "#9ca30b" "#e8e815" "#c9d617") (yellow "#da7710" "#fe8b04" "#dc7700") (yellow+1 "#958323" "#e5c900" "#bcaa00") (yellow+2 "#6a621b" "#abab3a" "#959508") (yellow-bg-1 "#cbcb20" "#909032" "#73712a") (yellow-bg "#dddd44" "#5e5c28" "#565624") (yellow-bg+1 "#e0e0a0" "#3c3c20" "#35351c") (green-2 "#3cb368" "#8ce096" "#47cd57") (green-1 "#1c9e28" "#32cd32" "#29b029") (green "#028902" "#7fb07f" "#60a060") (green+1 "#008b45" "#3cb370" "#319448") (green+2 "#077707" "#099709" "#078607") (green-bg-1 "#58c87c" "#31945c" "#297d4d") (green-bg "#9cdb6c" "#247744" "#1f673b") (green-bg+1 "#c9e6b3" "#2c5434" "#203f26") (cyan-2 "#0eaeae" "#8cf1f1" "#26d5d5") (cyan-1 "#259ea2" "#2fdbde" "#1ec1c4") (cyan "#358d8d" "#1fb3b3" "#1ba1a1") (cyan+1 "#0d7b72" "#528d8d" "#4c8383") (cyan+2 "#286060" "#0c8782" "#0a7874") (cyan-bg-1 "#4ecad7" "#1a758a" "#155f70") (cyan-bg "#80d7db" "#195f73" "#0f414d") (cyan-bg+1 "#c3d4d7" "#235050" "#132c2c") (blue-2 "#0092ff" "#b0c0ff" "#8cb7ff") (blue-1 "#2c53ca" "#94bff3" "#58b1f3") (blue "#1111ff" "#62b6ea" "#00a2f5") (blue+1 "#2020cc" "#30a5f5" "#1e7bda") (blue+2 "#00008b" "#3390dc" "#2062d0") (blue-bg-1 "#7cc0f7" "#1a63b3" "#144f8f") (blue-bg "#b0d0f3" "#134b87" "#0c325a") (blue-bg+1 "#bcd9f5" "#2b3f6b" "#0d1a38") (magenta-2 "#dc63dc" "#ebabde" "#dc8cc3") (magenta-1 "#ba55d3" "#dc8cc3" "#e353b9") (magenta "#a020f0" "#e353b9" "#da26ce") (magenta+1 "#9400d3" "#e81eda" "#c251df") (magenta+2 "#8b008b" "#be59d8" "#a92ec9") (magenta-bg-1 "#e98bb7" "#864d7d" "#72416a") (magenta-bg "#e5b3c4" "#6e4266" "#54324e") (magenta-bg+1 "#ecd0d0" "#55334f" "#351f31")))) #1#])) (alist :key-type symbol :value-type (alist :key-type symbol :value-type color)) nil) (alect-inverted-color-regexp ((funcall #'#[nil ("^\\(red\\|yellow\\|green\\|cyan\\|blue\\|magenta\\)\\([-+]\\)\\([012]\\)$") #1#])) regexp nil) (alect-display-class ((funcall #'#[nil ('((type graphic))) #1#])) (choice (const :tag "Graphical terminals" ((type graphic))) (const :tag "Terminals with at least 256 colors" ((class color) (min-colors 256))) (const :tag "All terminals") (sexp :tag "Other")) nil) (alect-ignored-faces ((funcall #'#[nil (nil) #1#])) (choice (const :tag "Theme (change) all intended faces" nil) (repeat :tag "Choose ignored faces" face)) nil) (alect-ignored-variables ((funcall #'#[nil (nil) #1#])) (choice (const :tag "Theme (change) all intended variables" nil) (const :tag "Ignore all (do not change any variable)" t) (repeat :tag "Choose ignored variables" (radio (variable-item ansi-color-names-vector) (variable-item emms-mode-line-icon-color) (variable-item gnus-mode-line-image-cache) (variable-item gnus-logo-colors) (variable-item diary-entry-marker) (variable-item fci-rule-color) (variable-item vc-annotate-color-map) (variable-item vc-annotate-very-old-color) (variable-item vc-annotate-background)))) nil)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn complete_function_signatures_autoload_contracts_and_documentation_are_stable() {
    let elisp_form = r##"
(mapcar
 (lambda (function)
   (list
    function
    (help-function-arglist function t)
    (interactive-form function)
    (secure-hash 'sha256 (documentation function))))
 '(alect-put-colors
   alect-generate-colors
   alect-set-color
   alect-get-color
   alect-get-customization
   alect-substitute-color
   alect-substitute-colors-in-plist
   alect-substitute-colors-in-faces
   alect-override-faces
   alect-delete-objects))
"##;
    let expect = expect![[
        r#"OK ((alect-put-colors (color-name theme-names color-vals var) nil "48894275c8e6435656def1044b72a7d0e42eb0a5649624a645e18589275856aa") (alect-generate-colors (theme-names colors) nil "4f6bc58a8b286e771fbc99432fd5fa4655bf985bb6ae633420d19a1d1058ad37") (alect-set-color (theme-name color-name color-val) nil "48ffdbd7dbaeace633e3c06e7fdfe97a57dfaaaedaab1192b9ab65387ca8577c") (alect-get-color (theme-name color-name &optional invert) nil "6faafcccc863ad8f59b2ba60f20fa04e38868f217893b7698198a09d939e9768") (alect-get-customization (theme &optional invert) nil "24b89d18ca8e35d7b951bcae88891e8652a1a764c823f776057d1714cf4cb151") (alect-substitute-color (theme-name plist prop) nil "2646c0be0edf3477937b90dec7775dfc75dc4f975c7d7de2cf9abe1a619bf54f") (alect-substitute-colors-in-plist (theme-name plist) nil "2e3e17dbe65d53c6d6e02cc895a789c37b04ca6684121cf7b14c7b6a4eb5b022") (alect-substitute-colors-in-faces (theme-name faces) nil "a67ce1f5019182517d3e7b2d97272f4ff3e6d0137a99ff228fcdfa725b1055af") (alect-override-faces (original overriding) nil "05448046130f4892d8ac6f12b8c43543cf126c6baa7d101c47fe69149d65185b") (alect-delete-objects (original ignored) nil "3ab6e8834e7d04a10e382919c8b5c8fa8f41c327da595a2aa427c1b7a2342cbe"))"#
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn all_auxiliary_faces_are_defined_in_complete_numbered_families() {
    let elisp_form = r##"
(let ((base
       '(alect-prompt alect-time alect-file alect-author
         alect-key alect-selected-item alect-line-number
         alect-title alect-field-title alect-block
         alect-block-title alect-button alect-button-pressed
         alect-button-mouse alect-tab-default
         alect-tab-unselected alect-tab-selected
         alect-tab-mouse))
      (color-levels
       (mapcar
        (lambda (number)
          (intern (format "alect-color-level-%d" number)))
        (number-sequence 1 12)))
      (titles
       (mapcar
        (lambda (number)
          (intern (format "alect-title-%d" number)))
        (number-sequence 1 8))))
  (mapcar
   (lambda (family)
     (list
      (length family)
      (mapcar
       (lambda (face)
         (list
          face
          (and (facep face) t)
          (get face 'face-documentation)
          (get face 'custom-group)))
       family)))
   (list base color-levels titles)))
"##;
    let expect = expect![[
        r#"OK ((18 ((alect-prompt t "Auxiliary face for inheriting by some other faces.\nUsed for various prompts like `minibuffer-prompt' or `eshell-prompt'." nil) (alect-time t "Auxiliary face for inheriting by some other faces.\nUsed for date/time faces like `org-date' or `erc-timestamp-face'." nil) (alect-file t "Auxiliary face for inheriting by some other faces.\nUsed for file name faces like `change-log-file' or\n`compilation-info'." nil) (alect-author t "Auxiliary face for inheriting by some other faces.\nUsed for author faces like `magit-log-author' or `change-log-name'." nil) (alect-key t "Auxiliary face for inheriting by some other faces.\nUsed for key faces like `apropos-keybinding' or `magit-popup-key'." nil) (alect-selected-item t "Auxiliary face for inheriting by some other faces.\nUsed for selected items like `org-date-selected' or\n`gnus-summary-selected'." nil) (alect-line-number t "Auxiliary face for inheriting by some other faces.\nUsed for selected items like `compilation-line-number' or\n`helm-grep-lineno'.\n\nUnfortunately, `display-line-numbers-mode' uses `shadow' face for line\nnumbers, so we cannot make `alect-line-number' work for this mode." nil) (alect-title t "Auxiliary face for inheriting by some other faces.\nUsed for titles without levels like `dired-header' or\n`magit-section-title'." nil) (alect-field-title t "Auxiliary face for inheriting by some other faces.\nUsed for field titles like `package-help-section-name' or\n`message-header-name'." nil) (alect-block t "Auxiliary face for inheriting by some other faces.\nUsed for blocks of text like `org-block' or\n`markdown-code-face'." nil) (alect-block-title t "Auxiliary face for inheriting by some other faces.\nUsed for titles of blocks like `org-meta-line' or\n`markdown-language-keyword-face'." nil) (alect-button t "Auxiliary face for inheriting by some other faces.\nUsed for buttons like `custom-button' or `w3m-form-button'." nil) (alect-button-pressed t "Auxiliary face for inheriting by some other faces.\nUsed for buttons like `custom-button-pressed' or\n`w3m-form-button-pressed'." nil) (alect-button-mouse t "Auxiliary face for inheriting by some other faces.\nUsed for buttons like `custom-button-mouse' or\n`w3m-form-button-mouse'." nil) (alect-tab-default t "Auxiliary face for inheriting by some other faces.\nUsed for faces like `tabbar-default' or `w3m-tab-background'." nil) (alect-tab-unselected t "Auxiliary face for inheriting by some other faces.\nUsed for tabs like `tabbar-unselected' or `w3m-tab-unselected'." nil) (alect-tab-selected t "Auxiliary face for inheriting by some other faces.\nUsed for tabs like `tabbar-selected' or `w3m-tab-selected'." nil) (alect-tab-mouse t "Auxiliary face for inheriting by some other faces.\nUsed for tabs like `tabbar-highlight' or `w3m-tab-mouse'." nil))) (12 ((alect-color-level-1 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-2 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-3 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-4 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-5 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-6 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-7 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-8 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-9 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-10 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-11 t "Auxiliary face for inheriting by some other faces." nil) (alect-color-level-12 t "Auxiliary face for inheriting by some other faces." nil))) (8 ((alect-title-1 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-1' or\n`markdown-header-face-1'." nil) (alect-title-2 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-2' or\n`markdown-header-face-2'." nil) (alect-title-3 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-3' or\n`markdown-header-face-3'." nil) (alect-title-4 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-4' or\n`markdown-header-face-4'." nil) (alect-title-5 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-5' or\n`markdown-header-face-5'." nil) (alect-title-6 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-6' or\n`markdown-header-face-6'." nil) (alect-title-7 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-7' or\n`markdown-header-face-7'." nil) (alect-title-8 t "Auxiliary face for inheriting by some other faces.\nUsed for titles with levels like `org-level-8' or\n`markdown-header-face-8'." nil))))"#
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn every_loader_registers_the_exact_six_theme_manifest_and_complete_settings() {
    let elisp_form = r##"
(let ((themes
       '(alect-light alect-light-alt
         alect-dark alect-dark-alt
         alect-black alect-black-alt)))
  (list
   (mapcar
    (lambda (theme)
      (let* ((loaded (load-theme theme t t))
             (settings (get theme 'theme-settings))
             (faces
              (seq-filter
               (lambda (setting)
                 (eq (car setting) 'theme-face))
               settings))
             (variables
              (seq-filter
               (lambda (setting)
                 (eq (car setting) 'theme-value))
               settings)))
        (list
         theme loaded
         (custom-theme-p theme)
         (get theme 'theme-feature)
         (get theme 'theme-documentation)
         (length settings)
         (length faces)
         (length variables)
         (length
          (delete-dups
           (mapcar #'cadr (copy-sequence faces))))
         (mapcar #'cadr variables)
         (file-name-nondirectory
          (locate-library (format "%s-theme" theme))))))
    themes)
   custom-enabled-themes))
"##;
    let expect = expect![[
        r#"OK (((alect-light t #1=(alect-light user changed) alect-light-theme "The light color theme." 944 935 9 935 (vc-annotate-background vc-annotate-very-old-color vc-annotate-color-map fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color ansi-color-names-vector) "alect-light-theme.el") (alect-light-alt t #2=(alect-light-alt . #1#) alect-light-alt-theme "The alternative light color theme." 944 935 9 935 (vc-annotate-background vc-annotate-very-old-color vc-annotate-color-map fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color ansi-color-names-vector) "alect-light-alt-theme.el") (alect-dark t #3=(alect-dark . #2#) alect-dark-theme "The dark color theme." 944 935 9 935 (vc-annotate-background vc-annotate-very-old-color vc-annotate-color-map fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color ansi-color-names-vector) "alect-dark-theme.el") (alect-dark-alt t #4=(alect-dark-alt . #3#) alect-dark-alt-theme "The alternative dark color theme." 944 935 9 935 (vc-annotate-background vc-annotate-very-old-color vc-annotate-color-map fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color ansi-color-names-vector) "alect-dark-alt-theme.el") (alect-black t #5=(alect-black . #4#) alect-black-theme "The black color theme." 944 935 9 935 (vc-annotate-background vc-annotate-very-old-color vc-annotate-color-map fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color ansi-color-names-vector) "alect-black-theme.el") (alect-black-alt t (alect-black-alt . #5#) alect-black-alt-theme "The alternative black color theme." 944 935 9 935 (vc-annotate-background vc-annotate-very-old-color vc-annotate-color-map fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color ansi-color-names-vector) "alect-black-alt-theme.el")) nil)"#
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn complete_generated_theme_registries_have_stable_per_setting_fingerprints() {
    let elisp_form = r##"
(mapcar
 (lambda (theme)
   (load-theme theme t t)
   (let ((settings (get theme 'theme-settings)))
     (list
      theme
      (length settings)
      (car settings)
      (car (last settings))
      (secure-hash
       'sha256
       (prin1-to-string
        (mapcar
         (lambda (setting)
           (secure-hash
            'sha256
            (prin1-to-string (copy-tree setting))))
         settings))))))
 '(alect-light alect-light-alt
   alect-dark alect-dark-alt
   alect-black alect-black-alt))
"##;
    let expect = expect![[
        r##"OK ((alect-light 944 (theme-face default alect-light ((#1=((type graphic)) :foreground "#262626" :background "#ded6c5"))) (theme-value ansi-color-names-vector alect-light ["#ded6c5" "#f71010" "#028902" "#da7710" "#1111ff" "#a020f0" "#358d8d" "#262626"]) "b12296fc4291cab6b8f02ff7db63a192027c908e30d7733b068a6d8777948d8a") (alect-light-alt 944 (theme-face default alect-light-alt ((#1# :foreground "#262626" :background "#ded6c5"))) (theme-value ansi-color-names-vector alect-light-alt ["#ded6c5" "#f71010" "#028902" "#da7710" "#1111ff" "#a020f0" "#358d8d" "#262626"]) "1b18142abbd327b49f84e393e5bbebaaae03b389cb7ab23697ee18265edc9bcb") (alect-dark 944 (theme-face default alect-dark ((#1# :foreground "#d5d2be" :background "#3f3f3f"))) (theme-value ansi-color-names-vector alect-dark ["#3f3f3f" "#ea3838" "#7fb07f" "#fe8b04" "#62b6ea" "#e353b9" "#1fb3b3" "#d5d2be"]) "fd74d6a2d598c5b060b7b6336ca9e039bbdf9c01cf43622d10d90f33a570891d") (alect-dark-alt 944 (theme-face default alect-dark-alt ((#1# :foreground "#d5d2be" :background "#3f3f3f"))) (theme-value ansi-color-names-vector alect-dark-alt ["#3f3f3f" "#ea3838" "#7fb07f" "#fe8b04" "#62b6ea" "#e353b9" "#1fb3b3" "#d5d2be"]) "a69698a922f2b407604686876f8ac0c06a54f5df0c2a462914886ac7755097d8") (alect-black 944 (theme-face default alect-black ((#1# :foreground "#b2af95" :background "#000000"))) (theme-value ansi-color-names-vector alect-black ["#000000" "#db4334" "#60a060" "#dc7700" "#00a2f5" "#da26ce" "#1ba1a1" "#b2af95"]) "305bc8882a2b49d3626d649b075fc4fc279b12b6af855f04ff2cafd81cad643e") (alect-black-alt 944 (theme-face default alect-black-alt ((#1# :foreground "#b2af95" :background "#000000"))) (theme-value ansi-color-names-vector alect-black-alt ["#000000" "#db4334" "#60a060" "#dc7700" "#00a2f5" "#da26ce" "#1ba1a1" "#b2af95"]) "85ee41c2bba5948b7fbf456fd7caeb26c98505a059a97b05b2aa4fcb54ce8758"))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_register_color_generator_and_all_theme_directories() {
    let elisp_form = r##"
(let* ((source-directory
        (file-name-directory
         (getenv "NEOMACS_PACKAGE_SOURCE")))
       (definition
        (symbol-function 'alect-generate-colors))
       (themes
        '(alect-light alect-light-alt
          alect-dark alect-dark-alt
          alect-black alect-black-alt)))
  (list
   (autoloadp definition)
   (nth 1 definition)
   (nth 3 definition)
   (and
    (member source-directory custom-theme-load-path)
    t)
   (mapcar
    (lambda (theme)
      (list
       theme
       (and
        (memq theme (custom-available-themes))
        t)
       (file-name-nondirectory
        (locate-library (format "%s-theme" theme)))))
    themes)))
"##;
    let expect = expect![[
        r#"OK (t "alect-themes" nil t ((alect-light t "alect-light-theme.el") (alect-light-alt t "alect-light-alt-theme.el") (alect-dark t "alect-dark-theme.el") (alect-dark-alt t "alect-dark-alt-theme.el") (alect-black t "alect-black-theme.el") (alect-black-alt t "alect-black-alt-theme.el")))"#
    ]];
    assert_alect_themes_autoload_parity(elisp_form, expect);
}
