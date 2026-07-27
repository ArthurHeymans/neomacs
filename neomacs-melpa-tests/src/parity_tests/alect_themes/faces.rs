use expect_test::expect;

use super::assert_alect_themes_parity;

#[test]
fn complete_customization_for_all_base_and_inverted_variants_has_stable_fingerprints() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (let* ((customization
           (alect-get-customization
            (car case) (cadr case)))
          (faces (car customization))
          (variables (cdr customization)))
     (list
      case
      (length faces)
      (length variables)
      (length
       (delete-dups
        (mapcar #'car (copy-sequence faces))))
      (secure-hash
       'sha256
       (prin1-to-string
        (mapcar
         (lambda (face)
           (secure-hash
            'sha256
            (prin1-to-string (copy-tree face))))
         faces)))
      (secure-hash
       'sha256
       (prin1-to-string
        (mapcar
         (lambda (variable)
           (secure-hash
            'sha256
            (prin1-to-string (copy-tree variable))))
         variables)))
      (mapcar #'car variables))))
 '((light nil) (light t)
   (dark nil) (dark t)
   (black nil) (black t)))
"##;
    let expect = expect![[
        r#"OK (((light nil) 935 9 935 "58861cbc3bf293af6117b1a6bd8aec747ef453906a6471aa76f7fca9dc6aaf7e" "d4bf13e120d11710366a6f90fcec12e5c540ff5c0c2131bd7ba39d34500d63b5" (ansi-color-names-vector emms-mode-line-icon-color gnus-mode-line-image-cache gnus-logo-colors diary-entry-marker fci-rule-color vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background)) ((light t) 935 9 935 "1ee217118b44fd0872545bf9a02ec020d88587eae3a4201812961bab433cfb33" "6689c265473ddce1d0f9651aaacb0f79435da161408089bcc2246879b9c8fd02" (ansi-color-names-vector emms-mode-line-icon-color gnus-mode-line-image-cache gnus-logo-colors diary-entry-marker fci-rule-color vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background)) ((dark nil) 935 9 935 "507566d9b4890d22e604e0ea4ed0ea789258f51d18bfab7433e61d794ad7a0be" "fb3e9293b3511c3a3e9148d1d5f9b8a5e5e74e31eee06ab4892ba20e353ac40d" (ansi-color-names-vector emms-mode-line-icon-color gnus-mode-line-image-cache gnus-logo-colors diary-entry-marker fci-rule-color vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background)) ((dark t) 935 9 935 "dc85ff4c2078f3e892a705585c664430b5c2f93ff9f77a1cfc71b9cec28d945b" "36574f88fe9c4de4e50464636a4adf79630cf3b0a3423b14f997597e2d912239" (ansi-color-names-vector emms-mode-line-icon-color gnus-mode-line-image-cache gnus-logo-colors diary-entry-marker fci-rule-color vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background)) ((black nil) 935 9 935 "9f9a4c872439c5f95bcae6e87c60d981f7c6334678be2eec1a1f2a34200f69ce" "2ca83c9df2be1949fc39cc579d73114c9bd88c8e3fb9498999879988ff9eb61d" (ansi-color-names-vector emms-mode-line-icon-color gnus-mode-line-image-cache gnus-logo-colors diary-entry-marker fci-rule-color vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background)) ((black t) 935 9 935 "90af5a0e789d70312a236a1dd605a43d9f5733cdb9e5c9dd26cb3352a036e9fb" "fa2f48aa68a006caa9a41c24ca2ce5b040417c1769ee5aa942d3a2367595328d" (ansi-color-names-vector emms-mode-line-icon-color gnus-mode-line-image-cache gnus-logo-colors diary-entry-marker fci-rule-color vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background)))"#
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn representative_core_builtin_and_external_face_specs_cover_every_semantic_family() {
    let elisp_form = r##"
(let ((faces
       '(default cursor fringe header-line region
         success error warning
         alect-prompt alect-time alect-file alect-author
         alect-key alect-selected-item alect-block
         alect-block-title alect-title alect-title-1
         font-lock-comment-face font-lock-doc-face
         font-lock-string-face font-lock-keyword-face
         font-lock-function-name-face
         mode-line mode-line-inactive
         diff-added diff-refine-removed dired-directory
         org-document-title org-level-2 org-block org-todo
         company-tooltip-selection
         magit-branch-current transient-key-stack
         emoji emoji-with-derivations)))
  (mapcar
   (lambda (case)
     (let ((all
            (car
             (alect-get-customization
              (car case) (cadr case)))))
       (cons
        case
        (mapcar
         (lambda (face)
           (list
            face
            (copy-tree (cadr (assq face all)))))
         faces))))
   '((light nil) (light t)
     (dark nil) (dark t)
     (black nil) (black t))))
"##;
    let expect = expect![[
        r##"OK (((light nil) (default ((((type graphic)) :foreground "#262626" :background "#ded6c5"))) (cursor ((((type graphic)) :background "#1074cd"))) (fringe ((((type graphic)) :foreground "#909090" :background "#f6f0e1"))) (header-line ((((type graphic)) :foreground "#101010" :height 1.13 :box (:line-width 1 :color "#101010" :style nil)))) (region ((((type graphic)) :background "#ccc19b"))) (success ((((type graphic)) :foreground "#028902" :weight bold))) (error ((((type graphic)) :foreground "#f71010" :weight bold))) (warning ((((type graphic)) :foreground "#9ca30b"))) (alect-prompt ((((type graphic)) :foreground "#ba55d3" :weight bold))) (alect-time ((((type graphic)) :foreground "#0eaeae"))) (alect-file ((((type graphic)) :foreground "#008b45"))) (alect-author ((((type graphic)) :foreground "#ba55d3"))) (alect-key ((((type graphic)) :foreground "#fa5151" :weight bold))) (alect-selected-item ((((type graphic)) :background "#d9ceb2" :box (:line-width -1 :color "#262626" :style nil)))) (alect-block ((((type graphic)) :background "#dcd2bd" :extend t))) (alect-block-title ((((type graphic)) :inherit alect-block :foreground "#008b45"))) (alect-title ((((type graphic)) :foreground "#077707" :weight bold :height 1.13))) (alect-title-1 ((((type graphic)) :inherit alect-color-level-1 :weight bold :height 1.13))) (font-lock-comment-face ((((type graphic)) :foreground "#008b45"))) (font-lock-doc-face ((((type graphic)) :foreground "#505050" :slant italic))) (font-lock-string-face ((((type graphic)) :foreground "#e43838"))) (font-lock-keyword-face ((((type graphic)) :foreground "#2020cc" :weight bold))) (font-lock-function-name-face ((((type graphic)) :foreground "#2c53ca"))) (mode-line ((((type graphic)) :foreground "#262626" :background "#f6f0e1" :box (:line-width 2 :style released-button)))) (mode-line-inactive ((((type graphic)) :foreground "#6c6c6c" :background "#ded6c5" :box (:line-width 2 :color "#f6f0e1" :style nil)))) (diff-added ((((type graphic)) :foreground "#1c9e28"))) (diff-refine-removed ((((type graphic)) :background "#fb9494" :foreground "#262626"))) (dired-directory ((((type graphic)) :inherit font-lock-function-name-face))) (org-document-title ((((type graphic)) :inherit alect-title))) (org-level-2 ((((type graphic)) :inherit alect-title-2))) (org-block ((((type graphic)) :inherit alect-block))) (org-todo ((((type graphic)) :foreground "#f71010" :weight bold))) (company-tooltip-selection ((((type graphic)) :foreground "#505050" :background "#f6f0e1"))) (magit-branch-current ((((type graphic)) :inherit magit-branch-local :box (:line-width 2 :color "#f71010")))) (transient-key-stack ((((type graphic)) :inherit alect-key :box (:style released-button)))) (emoji ((((type graphic)) :height 2.5))) (emoji-with-derivations ((((type graphic)) :inherit emoji :background "#d9ceb2")))) ((light t) (default ((((type graphic)) :foreground "#262626" :background "#ded6c5"))) (cursor ((((type graphic)) :background "#1074cd"))) (fringe ((((type graphic)) :foreground "#909090" :background "#f6f0e1"))) (header-line ((((type graphic)) :foreground "#101010" :height 1.13 :box (:line-width 1 :color "#101010" :style nil)))) (region ((((type graphic)) :background "#ccc19b"))) (success ((((type graphic)) :foreground "#028902" :weight bold))) (error ((((type graphic)) :foreground "#f71010" :weight bold))) (warning ((((type graphic)) :foreground "#958323"))) (alect-prompt ((((type graphic)) :foreground "#9400d3" :weight bold))) (alect-time ((((type graphic)) :foreground "#286060"))) (alect-file ((((type graphic)) :foreground "#1c9e28"))) (alect-author ((((type graphic)) :foreground "#9400d3"))) (alect-key ((((type graphic)) :foreground "#b22222" :weight bold))) (alect-selected-item ((((type graphic)) :background "#d9ceb2" :box (:line-width -1 :color "#262626" :style nil)))) (alect-block ((((type graphic)) :background "#dcd2bd" :extend t))) (alect-block-title ((((type graphic)) :inherit alect-block :foreground "#1c9e28"))) (alect-title ((((type graphic)) :foreground "#3cb368" :weight bold :height 1.13))) (alect-title-1 ((((type graphic)) :inherit alect-color-level-1 :weight bold :height 1.13))) (font-lock-comment-face ((((type graphic)) :foreground "#1c9e28"))) (font-lock-doc-face ((((type graphic)) :foreground "#505050" :slant italic))) (font-lock-string-face ((((type graphic)) :foreground "#d81212"))) (font-lock-keyword-face ((((type graphic)) :foreground "#2c53ca" :weight bold))) (font-lock-function-name-face ((((type graphic)) :foreground "#2020cc"))) (mode-line ((((type graphic)) :foreground "#262626" :background "#f6f0e1" :box (:line-width 2 :style released-button)))) (mode-line-inactive ((((type graphic)) :foreground "#6c6c6c" :background "#ded6c5" :box (:line-width 2 :color "#f6f0e1" :style nil)))) (diff-added ((((type graphic)) :foreground "#008b45"))) (diff-refine-removed ((((type graphic)) :background "#fb9494" :foreground "#262626"))) (dired-directory ((((type graphic)) :inherit font-lock-function-name-face))) (org-document-title ((((type graphic)) :inherit alect-title))) (org-level-2 ((((type graphic)) :inherit alect-title-2))) (org-block ((((type graphic)) :inherit alect-block))) (org-todo ((((type graphic)) :foreground "#f71010" :weight bold))) (company-tooltip-selection ((((type graphic)) :foreground "#505050" :background "#f6f0e1"))) (magit-branch-current ((((type graphic)) :inherit magit-branch-local :box (:line-width 2 :color "#f71010")))) (transient-key-stack ((((type graphic)) :inherit alect-key :box (:style released-button)))) (emoji ((((type graphic)) :height 2.5))) (emoji-with-derivations ((((type graphic)) :inherit emoji :background "#d9ceb2")))) ((dark nil) (default ((((type graphic)) :foreground "#d5d2be" :background "#3f3f3f"))) (cursor ((((type graphic)) :background "#d0d060"))) (fringe ((((type graphic)) :foreground "#9f9f9f" :background "#222222"))) (header-line ((((type graphic)) :foreground "#f6f0e1" :height 1.13 :box (:line-width 1 :color "#f6f0e1" :style nil)))) (region ((((type graphic)) :background "#6f6f6f"))) (success ((((type graphic)) :foreground "#7fb07f" :weight bold))) (error ((((type graphic)) :foreground "#ea3838" :weight bold))) (warning ((((type graphic)) :foreground "#e8e815"))) (alect-prompt ((((type graphic)) :foreground "#dc8cc3" :weight bold))) (alect-time ((((type graphic)) :foreground "#8cf1f1"))) (alect-file ((((type graphic)) :foreground "#3cb370"))) (alect-author ((((type graphic)) :foreground "#dc8cc3"))) (alect-key ((((type graphic)) :foreground "#fa6a6e" :weight bold))) (alect-selected-item ((((type graphic)) :background "#4f4f4f" :box (:line-width -1 :color "#d5d2be" :style nil)))) (alect-block ((((type graphic)) :background "#464646" :extend t))) (alect-block-title ((((type graphic)) :inherit alect-block :foreground "#3cb370"))) (alect-title ((((type graphic)) :foreground "#099709" :weight bold :height 1.13))) (alect-title-1 ((((type graphic)) :inherit alect-color-level-1 :weight bold :height 1.13))) (font-lock-comment-face ((((type graphic)) :foreground "#3cb370"))) (font-lock-doc-face ((((type graphic)) :foreground "#d0bf8f" :slant italic))) (font-lock-string-face ((((type graphic)) :foreground "#fa5151"))) (font-lock-keyword-face ((((type graphic)) :foreground "#30a5f5" :weight bold))) (font-lock-function-name-face ((((type graphic)) :foreground "#94bff3"))) (mode-line ((((type graphic)) :foreground "#d5d2be" :background "#222222" :box (:line-width 2 :style released-button)))) (mode-line-inactive ((((type graphic)) :foreground "#8c826d" :background "#3f3f3f" :box (:line-width 2 :color "#222222" :style nil)))) (diff-added ((((type graphic)) :foreground "#32cd32"))) (diff-refine-removed ((((type graphic)) :background "#a83838" :foreground "#d5d2be"))) (dired-directory ((((type graphic)) :inherit font-lock-function-name-face))) (org-document-title ((((type graphic)) :inherit alect-title))) (org-level-2 ((((type graphic)) :inherit alect-title-2))) (org-block ((((type graphic)) :inherit alect-block))) (org-todo ((((type graphic)) :foreground "#ea3838" :weight bold))) (company-tooltip-selection ((((type graphic)) :foreground "#d0bf8f" :background "#222222"))) (magit-branch-current ((((type graphic)) :inherit magit-branch-local :box (:line-width 2 :color "#ea3838")))) (transient-key-stack ((((type graphic)) :inherit alect-key :box (:style released-button)))) (emoji ((((type graphic)) :height 2.5))) (emoji-with-derivations ((((type graphic)) :inherit emoji :background "#4f4f4f")))) ((dark t) (default ((((type graphic)) :foreground "#d5d2be" :background "#3f3f3f"))) (cursor ((((type graphic)) :background "#d0d060"))) (fringe ((((type graphic)) :foreground "#9f9f9f" :background "#222222"))) (header-line ((((type graphic)) :foreground "#f6f0e1" :height 1.13 :box (:line-width 1 :color "#f6f0e1" :style nil)))) (region ((((type graphic)) :background "#6f6f6f"))) (success ((((type graphic)) :foreground "#7fb07f" :weight bold))) (error ((((type graphic)) :foreground "#ea3838" :weight bold))) (warning ((((type graphic)) :foreground "#e5c900"))) (alect-prompt ((((type graphic)) :foreground "#e81eda" :weight bold))) (alect-time ((((type graphic)) :foreground "#0c8782"))) (alect-file ((((type graphic)) :foreground "#32cd32"))) (alect-author ((((type graphic)) :foreground "#e81eda"))) (alect-key ((((type graphic)) :foreground "#c83029" :weight bold))) (alect-selected-item ((((type graphic)) :background "#4f4f4f" :box (:line-width -1 :color "#d5d2be" :style nil)))) (alect-block ((((type graphic)) :background "#464646" :extend t))) (alect-block-title ((((type graphic)) :inherit alect-block :foreground "#32cd32"))) (alect-title ((((type graphic)) :foreground "#8ce096" :weight bold :height 1.13))) (alect-title-1 ((((type graphic)) :inherit alect-color-level-1 :weight bold :height 1.13))) (font-lock-comment-face ((((type graphic)) :foreground "#32cd32"))) (font-lock-doc-face ((((type graphic)) :foreground "#d0bf8f" :slant italic))) (font-lock-string-face ((((type graphic)) :foreground "#db4334"))) (font-lock-keyword-face ((((type graphic)) :foreground "#94bff3" :weight bold))) (font-lock-function-name-face ((((type graphic)) :foreground "#30a5f5"))) (mode-line ((((type graphic)) :foreground "#d5d2be" :background "#222222" :box (:line-width 2 :style released-button)))) (mode-line-inactive ((((type graphic)) :foreground "#8c826d" :background "#3f3f3f" :box (:line-width 2 :color "#222222" :style nil)))) (diff-added ((((type graphic)) :foreground "#3cb370"))) (diff-refine-removed ((((type graphic)) :background "#a83838" :foreground "#d5d2be"))) (dired-directory ((((type graphic)) :inherit font-lock-function-name-face))) (org-document-title ((((type graphic)) :inherit alect-title))) (org-level-2 ((((type graphic)) :inherit alect-title-2))) (org-block ((((type graphic)) :inherit alect-block))) (org-todo ((((type graphic)) :foreground "#ea3838" :weight bold))) (company-tooltip-selection ((((type graphic)) :foreground "#d0bf8f" :background "#222222"))) (magit-branch-current ((((type graphic)) :inherit magit-branch-local :box (:line-width 2 :color "#ea3838")))) (transient-key-stack ((((type graphic)) :inherit alect-key :box (:style released-button)))) (emoji ((((type graphic)) :height 2.5))) (emoji-with-derivations ((((type graphic)) :inherit emoji :background "#4f4f4f")))) ((black nil) (default ((((type graphic)) :foreground "#b2af95" :background "#000000"))) (cursor ((((type graphic)) :background "#b1c721"))) (fringe ((((type graphic)) :foreground "#9b9b9b" :background "#404040"))) (header-line ((((type graphic)) :foreground "#d6cbae" :height 1.13 :box (:line-width 1 :color "#d6cbae" :style nil)))) (region ((((type graphic)) :background "#454545"))) (success ((((type graphic)) :foreground "#60a060" :weight bold))) (error ((((type graphic)) :foreground "#db4334" :weight bold))) (warning ((((type graphic)) :foreground "#c9d617"))) (alect-prompt ((((type graphic)) :foreground "#e353b9" :weight bold))) (alect-time ((((type graphic)) :foreground "#26d5d5"))) (alect-file ((((type graphic)) :foreground "#319448"))) (alect-author ((((type graphic)) :foreground "#e353b9"))) (alect-key ((((type graphic)) :foreground "#e96060" :weight bold))) (alect-selected-item ((((type graphic)) :background "#202020" :box (:line-width -1 :color "#b2af95" :style nil)))) (alect-block ((((type graphic)) :background "#101010" :extend t))) (alect-block-title ((((type graphic)) :inherit alect-block :foreground "#319448"))) (alect-title ((((type graphic)) :foreground "#078607" :weight bold :height 1.13))) (alect-title-1 ((((type graphic)) :inherit alect-color-level-1 :weight bold :height 1.13))) (font-lock-comment-face ((((type graphic)) :foreground "#319448"))) (font-lock-doc-face ((((type graphic)) :foreground "#ab9861" :slant italic))) (font-lock-string-face ((((type graphic)) :foreground "#ea4141"))) (font-lock-keyword-face ((((type graphic)) :foreground "#1e7bda" :weight bold))) (font-lock-function-name-face ((((type graphic)) :foreground "#58b1f3"))) (mode-line ((((type graphic)) :foreground "#b2af95" :background "#404040" :box (:line-width 2 :style released-button)))) (mode-line-inactive ((((type graphic)) :foreground "#8b806c" :background "#000000" :box (:line-width 2 :color "#404040" :style nil)))) (diff-added ((((type graphic)) :foreground "#29b029"))) (diff-refine-removed ((((type graphic)) :background "#86201c" :foreground "#b2af95"))) (dired-directory ((((type graphic)) :inherit font-lock-function-name-face))) (org-document-title ((((type graphic)) :inherit alect-title))) (org-level-2 ((((type graphic)) :inherit alect-title-2))) (org-block ((((type graphic)) :inherit alect-block))) (org-todo ((((type graphic)) :foreground "#db4334" :weight bold))) (company-tooltip-selection ((((type graphic)) :foreground "#ab9861" :background "#404040"))) (magit-branch-current ((((type graphic)) :inherit magit-branch-local :box (:line-width 2 :color "#db4334")))) (transient-key-stack ((((type graphic)) :inherit alect-key :box (:style released-button)))) (emoji ((((type graphic)) :height 2.5))) (emoji-with-derivations ((((type graphic)) :inherit emoji :background "#202020")))) ((black t) (default ((((type graphic)) :foreground "#b2af95" :background "#000000"))) (cursor ((((type graphic)) :background "#b1c721"))) (fringe ((((type graphic)) :foreground "#9b9b9b" :background "#404040"))) (header-line ((((type graphic)) :foreground "#d6cbae" :height 1.13 :box (:line-width 1 :color "#d6cbae" :style nil)))) (region ((((type graphic)) :background "#454545"))) (success ((((type graphic)) :foreground "#60a060" :weight bold))) (error ((((type graphic)) :foreground "#db4334" :weight bold))) (warning ((((type graphic)) :foreground "#bcaa00"))) (alect-prompt ((((type graphic)) :foreground "#c251df" :weight bold))) (alect-time ((((type graphic)) :foreground "#0a7874"))) (alect-file ((((type graphic)) :foreground "#29b029"))) (alect-author ((((type graphic)) :foreground "#c251df"))) (alect-key ((((type graphic)) :foreground "#ae2823" :weight bold))) (alect-selected-item ((((type graphic)) :background "#202020" :box (:line-width -1 :color "#b2af95" :style nil)))) (alect-block ((((type graphic)) :background "#101010" :extend t))) (alect-block-title ((((type graphic)) :inherit alect-block :foreground "#29b029"))) (alect-title ((((type graphic)) :foreground "#47cd57" :weight bold :height 1.13))) (alect-title-1 ((((type graphic)) :inherit alect-color-level-1 :weight bold :height 1.13))) (font-lock-comment-face ((((type graphic)) :foreground "#29b029"))) (font-lock-doc-face ((((type graphic)) :foreground "#ab9861" :slant italic))) (font-lock-string-face ((((type graphic)) :foreground "#c83029"))) (font-lock-keyword-face ((((type graphic)) :foreground "#58b1f3" :weight bold))) (font-lock-function-name-face ((((type graphic)) :foreground "#1e7bda"))) (mode-line ((((type graphic)) :foreground "#b2af95" :background "#404040" :box (:line-width 2 :style released-button)))) (mode-line-inactive ((((type graphic)) :foreground "#8b806c" :background "#000000" :box (:line-width 2 :color "#404040" :style nil)))) (diff-added ((((type graphic)) :foreground "#319448"))) (diff-refine-removed ((((type graphic)) :background "#86201c" :foreground "#b2af95"))) (dired-directory ((((type graphic)) :inherit font-lock-function-name-face))) (org-document-title ((((type graphic)) :inherit alect-title))) (org-level-2 ((((type graphic)) :inherit alect-title-2))) (org-block ((((type graphic)) :inherit alect-block))) (org-todo ((((type graphic)) :foreground "#db4334" :weight bold))) (company-tooltip-selection ((((type graphic)) :foreground "#ab9861" :background "#404040"))) (magit-branch-current ((((type graphic)) :inherit magit-branch-local :box (:line-width 2 :color "#db4334")))) (transient-key-stack ((((type graphic)) :inherit alect-key :box (:style released-button)))) (emoji ((((type graphic)) :height 2.5))) (emoji-with-derivations ((((type graphic)) :inherit emoji :background "#202020")))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn inversion_changes_only_matching_signed_palette_lookups_and_keeps_registry_shape() {
    let elisp_form = r##"
(mapcar
 (lambda (theme)
   (let* ((normal
           (car (alect-get-customization theme nil)))
          (inverted
           (car (alect-get-customization theme t)))
          (pairs
           (cl-mapcar #'list normal inverted))
          (changed
           (seq-filter
            (lambda (pair)
              (not (equal (car pair) (cadr pair))))
            pairs))
          (unchanged
           (seq-filter
            (lambda (pair)
              (equal (car pair) (cadr pair)))
            pairs)))
     (list
      theme
      (length normal)
      (length changed)
      (length unchanged)
      (mapcar
       (lambda (face)
         (list
          face
          (copy-tree (cadr (assq face normal)))
          (copy-tree (cadr (assq face inverted)))))
       '(default cursor region
         font-lock-comment-face
         font-lock-string-face
         font-lock-keyword-face
         diff-added org-level-1
         rainbow-delimiters-depth-12-face)))))
 '(light dark black))
"##;
    let expect = expect![[
        r##"OK ((light 935 235 700 ((default ((((type graphic)) :foreground "#262626" :background "#ded6c5")) ((((type graphic)) :foreground "#262626" :background "#ded6c5"))) (cursor ((((type graphic)) :background "#1074cd")) ((((type graphic)) :background "#1074cd"))) (region ((((type graphic)) :background "#ccc19b")) ((((type graphic)) :background "#ccc19b"))) (font-lock-comment-face ((((type graphic)) :foreground "#008b45")) ((((type graphic)) :foreground "#1c9e28"))) (font-lock-string-face ((((type graphic)) :foreground "#e43838")) ((((type graphic)) :foreground "#d81212"))) (font-lock-keyword-face ((((type graphic)) :foreground "#2020cc" :weight bold)) ((((type graphic)) :foreground "#2c53ca" :weight bold))) (diff-added ((((type graphic)) :foreground "#1c9e28")) ((((type graphic)) :foreground "#008b45"))) (org-level-1 ((((type graphic)) :inherit alect-title-1)) ((((type graphic)) :inherit alect-title-1))) (rainbow-delimiters-depth-12-face ((((type graphic)) :foreground "#286060")) ((((type graphic)) :foreground "#0eaeae"))))) (dark 935 235 700 ((default ((((type graphic)) :foreground "#d5d2be" :background "#3f3f3f")) ((((type graphic)) :foreground "#d5d2be" :background "#3f3f3f"))) (cursor ((((type graphic)) :background "#d0d060")) ((((type graphic)) :background "#d0d060"))) (region ((((type graphic)) :background "#6f6f6f")) ((((type graphic)) :background "#6f6f6f"))) (font-lock-comment-face ((((type graphic)) :foreground "#3cb370")) ((((type graphic)) :foreground "#32cd32"))) (font-lock-string-face ((((type graphic)) :foreground "#fa5151")) ((((type graphic)) :foreground "#db4334"))) (font-lock-keyword-face ((((type graphic)) :foreground "#30a5f5" :weight bold)) ((((type graphic)) :foreground "#94bff3" :weight bold))) (diff-added ((((type graphic)) :foreground "#32cd32")) ((((type graphic)) :foreground "#3cb370"))) (org-level-1 ((((type graphic)) :inherit alect-title-1)) ((((type graphic)) :inherit alect-title-1))) (rainbow-delimiters-depth-12-face ((((type graphic)) :foreground "#0c8782")) ((((type graphic)) :foreground "#8cf1f1"))))) (black 935 235 700 ((default ((((type graphic)) :foreground "#b2af95" :background "#000000")) ((((type graphic)) :foreground "#b2af95" :background "#000000"))) (cursor ((((type graphic)) :background "#b1c721")) ((((type graphic)) :background "#b1c721"))) (region ((((type graphic)) :background "#454545")) ((((type graphic)) :background "#454545"))) (font-lock-comment-face ((((type graphic)) :foreground "#319448")) ((((type graphic)) :foreground "#29b029"))) (font-lock-string-face ((((type graphic)) :foreground "#ea4141")) ((((type graphic)) :foreground "#c83029"))) (font-lock-keyword-face ((((type graphic)) :foreground "#1e7bda" :weight bold)) ((((type graphic)) :foreground "#58b1f3" :weight bold))) (diff-added ((((type graphic)) :foreground "#29b029")) ((((type graphic)) :foreground "#319448"))) (org-level-1 ((((type graphic)) :inherit alect-title-1)) ((((type graphic)) :inherit alect-title-1))) (rainbow-delimiters-depth-12-face ((((type graphic)) :foreground "#0a7874")) ((((type graphic)) :foreground "#26d5d5"))))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn display_class_is_embedded_verbatim_and_selects_graphic_terminal_and_universal_specs() {
    let elisp_form = r##"
(let ((frame (selected-frame))
      (cases
       '((graphic ((type graphic)))
         (terminal ((class color) (min-colors 256)))
         (universal t)
         (disabled nil))))
  (mapcar
   (lambda (case)
     (let* ((alect-display-class (cadr case))
            (faces
             (car (alect-get-customization 'light)))
            (default-spec
             (cadr (assq 'default faces))))
       (list
        (car case)
        (copy-tree default-spec)
        (mapcar
         (lambda (environment)
           (cl-letf
               (((symbol-function 'display-color-cells)
                 (lambda (&optional _frame)
                   (cadr environment)))
                ((symbol-function 'window-system)
                 (lambda (&optional _frame)
                   (car environment))))
             (list
              environment
              (face-spec-choose
               default-spec frame 'no-match))))
         '((nil 16)
           (nil 256)
           (x 256)
           (pgtk 16777216))))))
   cases))
"##;
    let expect = expect![[
        r##"OK ((graphic ((((type graphic)) :foreground "#262626" :background "#ded6c5")) ((#2=(nil 16) no-match) (#3=(nil 256) no-match) (#4=(x 256) #1=(:foreground "#262626" :background "#ded6c5")) (#5=(pgtk 16777216) #1#))) (terminal ((((class color) (min-colors 256)) :foreground "#262626" :background "#ded6c5")) ((#2# no-match) (#3# no-match) (#4# no-match) (#5# no-match))) (universal ((t :foreground "#262626" :background "#ded6c5")) ((#2# #6=(:foreground "#262626" :background "#ded6c5")) (#3# #6#) (#4# #6#) (#5# #6#))) (disabled ((nil :foreground "#262626" :background "#ded6c5")) ((#2# #7=(:foreground "#262626" :background "#ded6c5")) (#3# #7#) (#4# #7#) (#5# #7#))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn ignored_faces_and_variables_filter_exact_settings_during_theme_creation() {
    let elisp_form = r##"
(let ((alect-display-class t)
      (alect-ignored-faces
       '(default cursor font-lock-string-face
         org-level-1 missing-face))
      (alect-ignored-variables
       '(ansi-color-names-vector
         vc-annotate-color-map
         missing-variable)))
  (load-theme 'alect-light t t)
  (let* ((settings
          (get 'alect-light 'theme-settings))
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
     (length settings)
     (length faces)
     (length variables)
     (mapcar
      (lambda (face)
        (list
         face
         (and (assq face
                    (mapcar
                     (lambda (setting)
                       (cons (cadr setting) setting))
                     faces))
              t)))
      '(default cursor font-lock-string-face
        org-level-1 mode-line))
     (mapcar #'cadr variables))))
"##;
    let expect = expect![
        "OK (938 931 7 ((default nil) (cursor nil) (font-lock-string-face nil) (org-level-1 nil) (mode-line t)) (vc-annotate-background vc-annotate-very-old-color fci-rule-color diary-entry-marker gnus-logo-colors gnus-mode-line-image-cache emms-mode-line-icon-color))"
    ];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn overriding_faces_replace_defaults_add_new_faces_and_resolve_palette_symbols() {
    let elisp_form = r##"
(let ((alect-display-class t)
      (alect-overriding-faces
       '((default
          ((t :foreground red-2
              :background "#010203")))
         (font-lock-string-face
          ((t :foreground green-1
              :box (:line-width 2
                    :color blue+1
                    :style nil))))
         (alect-test-added-face
          ((t :inherit font-lock-keyword-face
              :foreground magenta+2))))))
  (load-theme 'alect-dark t t)
  (let ((settings
         (get 'alect-dark 'theme-settings)))
    (mapcar
     (lambda (face)
       (let ((entry
              (seq-find
               (lambda (setting)
                 (and
                  (eq (car setting) 'theme-face)
                  (eq (cadr setting) face)))
               settings)))
         (list
          face
          (and entry t)
          (copy-tree (nth 3 entry)))))
     '(default font-lock-string-face
       alect-test-added-face mode-line))))
"##;
    let expect = expect![[
        r##"OK ((default t ((t :foreground "#fa6a6e" :background "#010203"))) (font-lock-string-face t ((t :foreground "#32cd32" :box (:line-width 2 :color "#30a5f5" :style nil)))) (alect-test-added-face t ((t :inherit font-lock-keyword-face :foreground "#be59d8"))) (mode-line t ((t :foreground "#d5d2be" :background "#222222" :box (:line-width 2 :style released-button)))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn height_customization_materializes_in_header_single_and_all_numbered_titles() {
    let elisp_form = r##"
(let ((alect-display-class t)
      (alect-header-height 1.31)
      (alect-single-title-height 1.47)
      (alect-multiple-titles-height 1.19))
  (load-theme 'alect-black t t)
  (let ((settings
         (get 'alect-black 'theme-settings)))
    (mapcar
     (lambda (face)
       (let ((entry
              (seq-find
               (lambda (setting)
                 (and
                  (eq (car setting) 'theme-face)
                  (eq (cadr setting) face)))
               settings)))
         (list face (copy-tree (nth 3 entry)))))
     '(header-line alect-title
       alect-title-1 alect-title-4 alect-title-8
       org-document-title org-level-1 org-level-8
       info-title-1 markdown-header-face-6))))
"##;
    let expect = expect![[
        r##"OK ((header-line ((t :foreground "#d6cbae" :height 1.31 :box (:line-width 1 :color "#d6cbae" :style nil)))) (alect-title ((t :foreground "#078607" :weight bold :height 1.47))) (alect-title-1 ((t :inherit alect-color-level-1 :weight bold :height 1.19))) (alect-title-4 ((t :inherit alect-color-level-4 :weight bold :height 1.19))) (alect-title-8 ((t :inherit alect-color-level-8 :weight bold :height 1.19))) (org-document-title ((t :inherit alect-title))) (org-level-1 ((t :inherit alect-title-1))) (org-level-8 ((t :inherit alect-title-8))) (info-title-1 ((t :inherit alect-color-level-1 :height 1.5 :weight bold))) (markdown-header-face-6 ((t :inherit alect-title-6))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn enabled_universal_theme_resolves_real_inheritance_and_literal_attributes() {
    let elisp_form = r##"
(progn
  (require 'diff-mode)
  (require 'dired)
  (require 'org)
  (require 'compile)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (progn
          (load-theme 'alect-dark t)
          (mapcar
           (lambda (face)
             (list
              face
              (face-attribute
               face :foreground nil 'default)
              (face-attribute
               face :background nil 'default)
              (face-attribute
               face :inherit nil 'default)
              (face-attribute
               face :weight nil 'default)
              (face-attribute
               face :slant nil 'default)
              (face-attribute
               face :box nil 'default)))
           '(default fringe header-line region
             alect-prompt alect-title-1
             font-lock-comment-face
             font-lock-keyword-face
             diff-indicator-added
             dired-directory
             org-level-1 org-block
             compilation-info mode-line)))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK ((default "#d5d2be" "#3f3f3f" nil normal normal nil) (fringe "#9f9f9f" "#222222" nil normal normal nil) (header-line "#f6f0e1" "#3f3f3f" nil normal normal (:line-width 1 :color "#f6f0e1" :style nil)) (region "#d5d2be" "#6f6f6f" nil normal normal nil) (alect-prompt "#dc8cc3" "#3f3f3f" nil bold normal nil) (alect-title-1 "#30a5f5" "#3f3f3f" alect-color-level-1 bold normal nil) (font-lock-comment-face "#3cb370" "#3f3f3f" nil normal normal nil) (font-lock-keyword-face "#30a5f5" "#3f3f3f" nil bold normal nil) (diff-indicator-added "#32cd32" "#3f3f3f" diff-added bold normal nil) (dired-directory "#94bff3" "#3f3f3f" font-lock-function-name-face normal normal nil) (org-level-1 "#30a5f5" "#3f3f3f" alect-title-1 bold normal nil) (org-block "#d5d2be" "#464646" alect-block normal normal nil) (compilation-info "#3cb370" "#3f3f3f" alect-file normal normal nil) (mode-line "#d5d2be" "#222222" nil normal normal (:line-width 2 :style released-button)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn optional_package_faces_defined_after_enable_receive_pending_theme_specs() {
    let elisp_form = r##"
(let ((alect-display-class t))
  (unwind-protect
      (progn
        (load-theme 'alect-light t)
        (dolist
            (definition
             '((company-tooltip-selection
                (:foreground "fallback"
                 :background "fallback"))
               (magit-section-heading
                (:foreground "fallback"))
               (emoji-with-derivations
                (:height 1.0))
               (transient-key-stack
                (:foreground "fallback"))))
          (unless (facep (car definition))
            (eval
             `(defface ,(car definition)
                '((t ,(cadr definition)))
                "Late parity face."))))
        (mapcar
         (lambda (face)
           (list
            face
            (face-attribute
             face :foreground nil 'default)
            (face-attribute
             face :background nil 'default)
            (face-attribute
             face :inherit nil 'default)
            (face-attribute
             face :weight nil 'default)
            (face-attribute
             face :height nil 'default)
            (face-attribute
             face :box nil 'default)))
         '(company-tooltip-selection
           magit-section-heading
           emoji-with-derivations
           transient-key-stack)))
    (mapc #'disable-theme custom-enabled-themes)))
"##;
    let expect = expect![[
        r##"OK ((company-tooltip-selection "#505050" "#f6f0e1" nil normal 1 nil) (magit-section-heading "#077707" "#ded6c5" alect-title bold 1 nil) (emoji-with-derivations "#262626" "#d9ceb2" emoji normal 1 nil) (transient-key-stack "#fa5151" "#ded6c5" alect-key bold 1 (:style released-button)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}
