use expect_test::expect;

use super::assert_alect_themes_parity;

#[test]
fn all_three_palettes_have_exact_complete_order_unique_keys_and_fingerprints() {
    let elisp_form = r##"
(mapcar
 (lambda (entry)
   (let* ((theme (car entry))
          (palette (cdr entry))
          (keys (mapcar #'car palette)))
     (list
      theme
      (length palette)
      (length (delete-dups (copy-sequence keys)))
      (car palette)
      (car (last palette))
      (secure-hash 'sha256 (prin1-to-string palette))
      (mapcar
       (lambda (key)
         (assq key palette))
       '(cursor gray-2 gray gray+2
         fg-2 fg fg+2 bg-2 bg-0.5 bg+2
         red-2 red red+2 red-bg
         yellow-2 yellow yellow+2 yellow-bg
         green-2 green green+2 green-bg
         cyan-2 cyan cyan+2 cyan-bg
         blue-2 blue blue+2 blue-bg
         magenta-2 magenta magenta+2 magenta-bg)))))
 alect-colors)
"##;
    let expect = expect![[
        r##"OK ((light 65 65 (magenta-bg+1 . "#ecd0d0") #1=(cursor . "#1074cd") "4a87072ad55f4dd4a45c0455f3334541614679cd132b8fc8a8aa55cadcfc9b41" (#1# (gray-2 . "#fafafa") (gray . "#909090") (gray+2 . "#070707") (fg-2 . "#6c6c6c") (fg . "#3f3f3f") (fg+2 . "#101010") (bg-2 . "#f6f0e1") (bg-0.5 . "#dcd2bd") (bg+2 . "#ccc19b") (red-2 . "#fa5151") (red . "#f71010") (red+2 . "#b22222") (red-bg . "#fb9494") (yellow-2 . "#ab9c3a") (yellow . "#da7710") (yellow+2 . "#6a621b") (yellow-bg . "#dddd44") (green-2 . "#3cb368") (green . "#028902") (green+2 . "#077707") (green-bg . "#9cdb6c") (cyan-2 . "#0eaeae") (cyan . "#358d8d") (cyan+2 . "#286060") (cyan-bg . "#80d7db") (blue-2 . "#0092ff") (blue . "#1111ff") (blue+2 . "#00008b") (blue-bg . "#b0d0f3") (magenta-2 . "#dc63dc") (magenta . "#a020f0") (magenta+2 . "#8b008b") (magenta-bg . "#e5b3c4"))) (dark 65 65 (magenta-bg+1 . "#55334f") #2=(cursor . "#d0d060") "a92db0ae7513b14f19273b171bd3a05e2e4d2a065e2297cf5fcca4997b4f6de9" (#2# (gray-2 . "#e9e9e9") (gray . "#9f9f9f") (gray+2 . "#000000") (fg-2 . "#8c826d") (fg . "#f0dfaf") (fg+2 . "#f6f0e1") (bg-2 . "#222222") (bg-0.5 . "#464646") (bg+2 . "#6f6f6f") (red-2 . "#fa6a6e") (red . "#ea3838") (red+2 . "#c83029") (red-bg . "#a83838") (yellow-2 . "#f8ffa0") (yellow . "#fe8b04") (yellow+2 . "#abab3a") (yellow-bg . "#5e5c28") (green-2 . "#8ce096") (green . "#7fb07f") (green+2 . "#099709") (green-bg . "#247744") (cyan-2 . "#8cf1f1") (cyan . "#1fb3b3") (cyan+2 . "#0c8782") (cyan-bg . "#195f73") (blue-2 . "#b0c0ff") (blue . "#62b6ea") (blue+2 . "#3390dc") (blue-bg . "#134b87") (magenta-2 . "#ebabde") (magenta . "#e353b9") (magenta+2 . "#be59d8") (magenta-bg . "#6e4266"))) (black 65 65 (magenta-bg+1 . "#351f31") #3=(cursor . "#b1c721") "3fd91c4a635aa51b65fa7fa13233d7769193bcbc8bcfea1d6c9244cdab3925af" (#3# (gray-2 . "#dedede") (gray . "#9b9b9b") (gray+2 . "#000000") (fg-2 . "#8b806c") (fg . "#c4ad63") (fg+2 . "#d6cbae") (bg-2 . "#404040") (bg-0.5 . "#101010") (bg+2 . "#454545") (red-2 . "#e96060") (red . "#db4334") (red+2 . "#ae2823") (red-bg . "#86201c") (yellow-2 . "#e9e953") (yellow . "#dc7700") (yellow+2 . "#959508") (yellow-bg . "#565624") (green-2 . "#47cd57") (green . "#60a060") (green+2 . "#078607") (green-bg . "#1f673b") (cyan-2 . "#26d5d5") (cyan . "#1ba1a1") (cyan+2 . "#0a7874") (cyan-bg . "#0f414d") (blue-2 . "#8cb7ff") (blue . "#00a2f5") (blue+2 . "#2062d0") (blue-bg . "#0c325a") (magenta-2 . "#dc8cc3") (magenta . "#da26ce") (magenta+2 . "#a92ec9") (magenta-bg . "#54324e"))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn color_generation_preserves_theme_order_reverses_color_rows_and_handles_ragged_values() {
    let elisp_form = r##"
(let* ((theme-names '(paper dusk void))
       (rows
        '((base "#eeeeee" "#222222" "#000000")
          (accent "#cc0000" "#00cc00")
          (cursor "#111111" "#dddddd" "#ffffff" "unused")))
       (generated
        (alect-generate-colors theme-names rows)))
  (list
   generated
   theme-names
   rows
   (mapcar #'length generated)
   (mapcar
    (lambda (entry)
      (mapcar #'car (cdr entry)))
    generated)))
"##;
    let expect = expect![[
        r##"OK (((paper (cursor . "#111111") (accent . "#cc0000") (base . "#eeeeee")) (dusk (cursor . "#dddddd") (accent . "#00cc00") (base . "#222222")) (void (cursor . "#ffffff") (accent) (base . "#000000"))) (paper dusk void) ((base "#eeeeee" "#222222" "#000000") (accent "#cc0000" "#00cc00") (cursor "#111111" "#dddddd" "#ffffff" "unused")) (4 4 4) ((cursor accent base) (cursor accent base) (cursor accent base)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn put_colors_mutates_existing_theme_cells_in_place_and_returns_recursive_nil() {
    let elisp_form = r##"
(let* ((themes
        (list
         (list 'paper '(existing . "old"))
         (list 'dusk '(existing . "older"))))
       (paper (car themes))
       (dusk (cadr themes))
       (result
        (alect-put-colors
         'accent
         '(paper dusk)
         '("#f00" "#0f0")
         themes)))
  (list
   result
   themes
   (eq paper (car themes))
   (eq dusk (cadr themes))
   (eq (cdr paper) (cdr (car themes)))
   (alect-put-colors 'unused nil nil themes)
   themes))
"##;
    let expect = expect![[
        r##"OK (nil #1=((paper (accent . "#f00") (existing . "old")) (dusk (accent . "#0f0") (existing . "older"))) t t t nil #1#)"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn direct_and_inverted_lookup_cover_every_color_family_and_custom_regexp() {
    let elisp_form = r##"
(let ((keys
       '(cursor gray-2 gray+2 fg-2 fg fg+2
         bg-2 bg bg+2
         red-2 red-1 red red+1 red+2
         yellow-2 yellow+2
         green-2 green+2
         cyan-2 cyan+2
         blue-2 blue+2
         magenta-2 magenta+2)))
  (list
   (mapcar
    (lambda (theme)
      (list
       theme
       (mapcar
        (lambda (key)
          (list
           key
           (alect-get-color theme key)
           (alect-get-color theme key t)))
        keys)))
    '(light dark black))
   (let ((alect-inverted-color-regexp
          "^\\(bg\\)\\([-+]\\)\\([012]\\)$"))
     (mapcar
      (lambda (key)
        (list
         key
         (alect-get-color 'dark key t)))
      '(bg-2 bg-1 bg bg+1 bg+2 red-2 red+2)))
   (alect-get-color 'missing 'red)
   (alect-get-color 'light 'missing t)))
"##;
    let expect = expect![[
        r##"OK (((light ((cursor "#1074cd" "#1074cd") (gray-2 "#fafafa" "#fafafa") (gray+2 "#070707" "#070707") (fg-2 "#6c6c6c" "#6c6c6c") (fg "#3f3f3f" "#3f3f3f") (fg+2 "#101010" "#101010") (bg-2 "#f6f0e1" "#f6f0e1") (bg "#d9ceb2" "#d9ceb2") (bg+2 "#ccc19b" "#ccc19b") (red-2 "#fa5151" "#b22222") (red-1 "#e43838" "#d81212") (red "#f71010" "#f71010") (red+1 "#d81212" "#e43838") (red+2 "#b22222" "#fa5151") (yellow-2 "#ab9c3a" "#6a621b") (yellow+2 "#6a621b" "#ab9c3a") (green-2 "#3cb368" "#077707") (green+2 "#077707" "#3cb368") (cyan-2 "#0eaeae" "#286060") (cyan+2 "#286060" "#0eaeae") (blue-2 "#0092ff" "#00008b") (blue+2 "#00008b" "#0092ff") (magenta-2 "#dc63dc" "#8b008b") (magenta+2 "#8b008b" "#dc63dc"))) (dark ((cursor "#d0d060" "#d0d060") (gray-2 "#e9e9e9" "#e9e9e9") (gray+2 "#000000" "#000000") (fg-2 "#8c826d" "#8c826d") (fg "#f0dfaf" "#f0dfaf") (fg+2 "#f6f0e1" "#f6f0e1") (bg-2 "#222222" "#222222") (bg "#4f4f4f" "#4f4f4f") (bg+2 "#6f6f6f" "#6f6f6f") (red-2 "#fa6a6e" "#c83029") (red-1 "#fa5151" "#db4334") (red "#ea3838" "#ea3838") (red+1 "#db4334" "#fa5151") (red+2 "#c83029" "#fa6a6e") (yellow-2 "#f8ffa0" "#abab3a") (yellow+2 "#abab3a" "#f8ffa0") (green-2 "#8ce096" "#099709") (green+2 "#099709" "#8ce096") (cyan-2 "#8cf1f1" "#0c8782") (cyan+2 "#0c8782" "#8cf1f1") (blue-2 "#b0c0ff" "#3390dc") (blue+2 "#3390dc" "#b0c0ff") (magenta-2 "#ebabde" "#be59d8") (magenta+2 "#be59d8" "#ebabde"))) (black ((cursor "#b1c721" "#b1c721") (gray-2 "#dedede" "#dedede") (gray+2 "#000000" "#000000") (fg-2 "#8b806c" "#8b806c") (fg "#c4ad63" "#c4ad63") (fg+2 "#d6cbae" "#d6cbae") (bg-2 "#404040" "#404040") (bg "#202020" "#202020") (bg+2 "#454545" "#454545") (red-2 "#e96060" "#ae2823") (red-1 "#ea4141" "#c83029") (red "#db4334" "#db4334") (red+1 "#c83029" "#ea4141") (red+2 "#ae2823" "#e96060") (yellow-2 "#e9e953" "#959508") (yellow+2 "#959508" "#e9e953") (green-2 "#47cd57" "#078607") (green+2 "#078607" "#47cd57") (cyan-2 "#26d5d5" "#0a7874") (cyan+2 "#0a7874" "#26d5d5") (blue-2 "#8cb7ff" "#2062d0") (blue+2 "#2062d0" "#8cb7ff") (magenta-2 "#dc8cc3" "#a92ec9") (magenta+2 "#a92ec9" "#dc8cc3")))) ((bg-2 "#6f6f6f") (bg-1 "#5f5f5f") (bg "#4f4f4f") (bg+1 "#3f3f3f") (bg+2 "#222222") (red-2 "#fa6a6e") (red+2 "#c83029")) nil nil)"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn set_color_updates_only_the_requested_cell_and_signals_precise_invalid_inputs() {
    let elisp_form = r##"
(let ((alect-colors (copy-tree alect-colors))
      before
      successful
      failures)
  (setq before
        (mapcar
         (lambda (theme)
           (alect-get-color theme 'cyan-2))
         '(light dark black)))
  (setq successful
        (list
         (alect-set-color 'light 'cyan-2 "#00a8a8")
         (mapcar
          (lambda (theme)
            (alect-get-color theme 'cyan-2))
          '(light dark black))))
  (dolist
      (arguments
       '((missing cyan-2 "#111111")
         (light missing "#222222")))
    (condition-case error-data
        (apply #'alect-set-color arguments)
      (error
       (push
        (list (car error-data) (cadr error-data))
        failures))))
  (list
   before successful (nreverse failures)
   (length alect-colors)
   (mapcar
    (lambda (entry)
      (length (cdr entry)))
    alect-colors)))
"##;
    let expect = expect![[
        r##"OK (("#0eaeae" "#8cf1f1" "#26d5d5") ("#00a8a8" ("#00a8a8" "#8cf1f1" "#26d5d5")) ((error "Theme ’missing’ does not exist") (error "Color ’missing’ does not exist")) 3 (65 65 65))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn strengthened_upstream_substitution_contract_handles_nested_boxes_and_display_specs() {
    let elisp_form = r##"
(list
 (alect-substitute-color
  'dark
  '(:foreground "pink" :background green-1)
  :foreground)
 (alect-substitute-color
  'dark
  '(:foreground "pink" :background green-1)
  :background)
 (alect-substitute-colors-in-plist
  'light
  '(((:foreground "pink"
      :background green-1
      :underline t
      :box (:line-width 1 :color fg :style nil)))))
 (alect-substitute-colors-in-faces
  'light
  '((fringe ((t (:background "pink"))))
    (font-lock-string-face ((t :foreground green-1)))
    (button
     ((((class color) (min-colors 88) (:background blue))
       (:foreground magenta))
      (((class color) (background dark))
       :foreground "LightSkyBlue")
      (((class color) (min-colors 16))
       (:bold t :background fg+2))
      (t
       (:slant italic
        :box (:line-width 1 :color red-1
              :background cyan)))))))
 (alect-substitute-colors-in-faces
  'dark
  '((hl-line
     ((((class color) (min-colors 256))
       :background bg)
      (t nil))))))
"##;
    let expect = expect![[
        r##"OK ((:foreground "pink" :background green-1) (:foreground "pink" :background "#32cd32") (:foreground "pink" :background "#1c9e28" :underline t :box (:line-width 1 :color "#3f3f3f" :style nil)) ((fringe ((t :background "pink"))) (font-lock-string-face ((t :foreground "#1c9e28"))) (button ((((class color) (min-colors 88) (:background blue)) :foreground "#a020f0") (((class color) (background dark)) :foreground "LightSkyBlue") (((class color) (min-colors 16)) :bold t :background "#101010") (t :slant italic :box (:line-width 1 :color "#e43838" :background cyan))))) ((hl-line ((((class color) (min-colors 256)) :background "#4f4f4f") (t nil)))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn substitution_is_destructive_only_at_documented_plist_and_nested_box_seams() {
    let elisp_form = r##"
(let* ((box (list :line-width 2 :color 'blue+1 :style nil))
       (plist
        (list
         :foreground 'red-1
         :background "literal"
         :box box
         :underline t))
       (wrapper (list plist))
       (result
        (alect-substitute-colors-in-plist 'black wrapper)))
  (list
   result
   plist
   wrapper
   (eq result plist)
   (eq (plist-get result :box) box)
   (alect-substitute-colors-in-plist
    'light
    '(:foreground absent
      :background nil
      :box (:color absent)))))
"##;
    let expect = expect![[
        r##"OK (#1=(:foreground "#ea4141" :background "literal" :box (:line-width 2 :color "#1e7bda" :style nil) :underline t) #1# (#1#) t t (:foreground absent :background nil :box (:color absent)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn face_overrides_replace_existing_specs_prepend_new_faces_and_preserve_other_order() {
    let elisp_form = r##"
(let* ((original
        '((default ((t :foreground "old")))
          (mode-line ((t :background "old-bg")))
          (font-lock-string-face
           ((t :foreground "old-string")))))
       (original-copy (copy-tree original))
       (overriding
        '((mode-line
           ((t :foreground "new-fg"
               :background "new-bg")))
          (new-package-face
           ((t :inherit font-lock-keyword-face)))))
       (result
        (alect-override-faces original overriding)))
  (list
   result
   original
   original-copy
   (eq result original)
   (mapcar #'car result)
   (length
    (seq-filter
     (lambda (face)
       (eq (car face) 'mode-line))
     result))))
"##;
    let expect = expect![[
        r#"OK (((new-package-face ((t :inherit font-lock-keyword-face))) (mode-line ((t :foreground "new-fg" :background "new-bg"))) . #1=((default ((t :foreground "old"))) (font-lock-string-face ((t :foreground "old-string"))))) #1# ((default ((t :foreground "old"))) (mode-line ((t :background "old-bg"))) (font-lock-string-face ((t :foreground "old-string")))) nil (new-package-face mode-line default font-lock-string-face) 1)"#
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn delete_objects_covers_nil_all_selected_missing_duplicate_and_destructive_paths() {
    let elisp_form = r##"
(let ((objects
       '((alpha 1)
         (beta 2)
         (alpha 3)
         (gamma 4))))
  (mapcar
   (lambda (ignored)
     (let* ((input (copy-tree objects))
            (result
             (alect-delete-objects input ignored)))
       (list
        ignored
        result
        input
        (eq result input))))
   '(nil t (alpha) (beta missing) (alpha gamma))))
"##;
    let expect = expect![
        "OK ((nil #1=((alpha 1) (beta 2) (alpha 3) (gamma 4)) #1# t) (t nil ((alpha 1) (beta 2) (alpha 3) (gamma 4)) nil) ((alpha) #2=((beta 2) (gamma 4)) ((alpha 1) . #2#) nil) ((beta missing) #3=((alpha 1) (alpha 3) (gamma 4)) #3# t) ((alpha gamma) #4=((beta 2)) ((alpha 1) . #4#) nil))"
    ];
    assert_alect_themes_parity(elisp_form, expect);
}
