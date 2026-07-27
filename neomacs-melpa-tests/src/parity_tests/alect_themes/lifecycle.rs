use expect_test::expect;

use super::assert_alect_themes_parity;

#[test]
fn switching_through_all_six_themes_updates_faces_and_themed_terminal_palette() {
    let elisp_form = r##"
(progn
  (require 'ansi-color)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (mapcar
         (lambda (theme)
           (mapc #'disable-theme custom-enabled-themes)
           (let ((result (load-theme theme t)))
             (list
              theme result
              (copy-sequence custom-enabled-themes)
              (face-attribute
               'default :foreground nil 'default)
              (face-attribute
               'default :background nil 'default)
              (face-attribute
               'font-lock-comment-face
               :foreground nil 'default)
              (face-attribute
               'font-lock-string-face
               :foreground nil 'default)
              (face-attribute
               'font-lock-keyword-face
               :foreground nil 'default)
              (append ansi-color-names-vector nil))))
         '(alect-light alect-light-alt
           alect-dark alect-dark-alt
           alect-black alect-black-alt))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK ((alect-light t (alect-light) "#262626" "#ded6c5" "#008b45" "#e43838" "#2020cc" ("#ded6c5" "#f71010" "#028902" "#da7710" "#1111ff" "#a020f0" "#358d8d" "#262626")) (alect-light-alt t (alect-light-alt) "#262626" "#ded6c5" "#1c9e28" "#d81212" "#2c53ca" ("#ded6c5" "#f71010" "#028902" "#da7710" "#1111ff" "#a020f0" "#358d8d" "#262626")) (alect-dark t (alect-dark) "#d5d2be" "#3f3f3f" "#3cb370" "#fa5151" "#30a5f5" ("#3f3f3f" "#ea3838" "#7fb07f" "#fe8b04" "#62b6ea" "#e353b9" "#1fb3b3" "#d5d2be")) (alect-dark-alt t (alect-dark-alt) "#d5d2be" "#3f3f3f" "#32cd32" "#db4334" "#94bff3" ("#3f3f3f" "#ea3838" "#7fb07f" "#fe8b04" "#62b6ea" "#e353b9" "#1fb3b3" "#d5d2be")) (alect-black t (alect-black) "#b2af95" "#000000" "#319448" "#ea4141" "#1e7bda" ("#000000" "#db4334" "#60a060" "#dc7700" "#00a2f5" "#da26ce" "#1ba1a1" "#b2af95")) (alect-black-alt t (alect-black-alt) "#b2af95" "#000000" "#29b029" "#c83029" "#58b1f3" ("#000000" "#db4334" "#60a060" "#dc7700" "#00a2f5" "#da26ce" "#1ba1a1" "#b2af95")))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn stacked_themes_obey_enable_precedence_and_disable_restores_previous_layers() {
    let elisp_form = r##"
(progn
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t)
        (capture
         (lambda ()
           (list
            (copy-sequence custom-enabled-themes)
            (face-attribute
             'default :foreground nil 'default)
            (face-attribute
             'default :background nil 'default)
            (face-attribute
             'font-lock-function-name-face
             :foreground nil 'default)
            (face-attribute
             'mode-line :background nil 'default)))))
    (unwind-protect
        (progn
          (load-theme 'alect-light t)
          (let ((light (funcall capture)))
            (load-theme 'alect-dark t)
            (let ((dark-over-light (funcall capture)))
              (load-theme 'alect-black t)
              (let ((black-over-all (funcall capture)))
                (disable-theme 'alect-black)
                (let ((dark-restored (funcall capture)))
                  (disable-theme 'alect-dark)
                  (list
                   light dark-over-light black-over-all
                   dark-restored (funcall capture)))))))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK (((alect-light) "#262626" "#ded6c5" "#2c53ca" "#f6f0e1") ((alect-dark alect-light) "#d5d2be" "#3f3f3f" "#94bff3" "#222222") ((alect-black alect-dark alect-light) "#b2af95" "#000000" "#58b1f3" "#404040") ((alect-dark alect-light) "#d5d2be" "#3f3f3f" "#94bff3" "#222222") ((alect-light) "#262626" "#ded6c5" "#2c53ca" "#f6f0e1"))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn repeated_noenable_and_enabled_loads_keep_registry_and_enabled_state_idempotent() {
    let elisp_form = r##"
(progn
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t)
        observations)
    (unwind-protect
        (progn
          (dotimes (_ 2)
            (load-theme 'alect-dark-alt t t)
            (let ((settings
                   (get 'alect-dark-alt 'theme-settings)))
              (push
               (list
                'noenable
                (length settings)
                (secure-hash
                 'sha256
                 (prin1-to-string
                  (mapcar
                   (lambda (setting)
                     (secure-hash
                      'sha256
                      (prin1-to-string
                       (copy-tree setting))))
                   settings)))
                (copy-sequence custom-enabled-themes))
               observations)))
          (dotimes (_ 3)
            (load-theme 'alect-dark-alt t)
            (push
             (list
              'enabled
              (length
               (get 'alect-dark-alt 'theme-settings))
              (copy-sequence custom-enabled-themes))
             observations))
          (nreverse observations))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r#"OK ((noenable 944 "1f74bbcfce419d6f05259da443678994e7fbc11b794989b3ecf20376b6a7d47f" nil) (noenable 944 "1f74bbcfce419d6f05259da443678994e7fbc11b794989b3ecf20376b6a7d47f" nil) (enabled 944 (alect-dark-alt)) (enabled 944 (alect-dark-alt)) (enabled 944 (alect-dark-alt)))"#
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn palette_mutation_changes_regenerated_specs_and_restoring_color_restores_fingerprint() {
    let elisp_form = r##"
(let ((alect-colors (copy-tree alect-colors))
      (alect-display-class t))
  (let ((capture
         (lambda ()
           (let ((settings
                  (get 'alect-light 'theme-settings)))
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
                   (copy-tree (nth 3 entry)))))
              '(font-lock-string-face
                diff-removed alect-key term-color-red))))))
    (load-theme 'alect-light t t)
    (let ((original (funcall capture)))
      (alect-set-color 'light 'red-1 "#123456")
      (alect-set-color 'light 'red-2 "#654321")
      (load-theme 'alect-light t t)
      (let ((modified (funcall capture)))
        (alect-set-color 'light 'red-1 "#e43838")
        (alect-set-color 'light 'red-2 "#fa5151")
        (load-theme 'alect-light t t)
        (let ((restored (funcall capture)))
          (list
           original modified restored
           (equal original restored)))))))
"##;
    let expect = expect![""];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn overriding_face_reload_changes_live_attributes_and_reset_restores_original_behavior() {
    let elisp_form = r##"
(progn
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t)
        (alect-overriding-faces nil)
        (capture
         (lambda ()
           (list
            (face-attribute
             'default :foreground nil 'default)
            (face-attribute
             'default :background nil 'default)
            (face-attribute
             'font-lock-string-face
             :foreground nil 'default)
            (face-attribute
             'font-lock-string-face
             :weight nil 'default)))))
    (unwind-protect
        (progn
          (load-theme 'alect-black t)
          (let ((original (funcall capture)))
            (disable-theme 'alect-black)
            (setq
             alect-overriding-faces
             '((default
                ((t :foreground "#abcdef"
                    :background "#010203")))
               (font-lock-string-face
                ((t :foreground green-2
                    :weight bold)))))
            (load-theme 'alect-black t)
            (let ((overridden (funcall capture)))
              (disable-theme 'alect-black)
              (setq alect-overriding-faces nil)
              (load-theme 'alect-black t)
              (list
               original overridden (funcall capture)))))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![""];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn ignored_variable_modes_preserve_selected_values_and_apply_the_remaining_theme_values() {
    let elisp_form = r##"
(progn
  (require 'ansi-color)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t)
        (ansi-color-names-vector
         ["sentinel-0" "sentinel-1" "sentinel-2" "sentinel-3"
          "sentinel-4" "sentinel-5" "sentinel-6" "sentinel-7"])
        (vc-annotate-background "sentinel-background"))
    (unwind-protect
        (let ((alect-ignored-variables t))
          (load-theme 'alect-dark t)
          (let ((all-ignored
                 (list
                  (append ansi-color-names-vector nil)
                  vc-annotate-background)))
            (disable-theme 'alect-dark)
            (let ((alect-ignored-variables
                   '(ansi-color-names-vector)))
              (load-theme 'alect-dark t)
              (list
               all-ignored
               (append ansi-color-names-vector nil)
               vc-annotate-background
               (mapcar
                #'cadr
                (seq-filter
                 (lambda (setting)
                   (eq (car setting) 'theme-value))
                 (get 'alect-dark 'theme-settings)))))))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![""];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn create_theme_macro_builds_and_runs_a_real_theme_from_an_added_palette() {
    let elisp_form = r##"
(progn
  (mapc #'disable-theme custom-enabled-themes)
  (let* ((alect-display-class t)
         (alect-colors (copy-tree alect-colors))
         (paper
          (cons
           'paper
           (copy-tree (cdr (assq 'light alect-colors))))))
    (setcdr (assq 'bg-1 paper) "#fefcf7")
    (setcdr (assq 'fg+1 paper) "#181818")
    (setcdr (assq 'blue-1 paper) "#2050a0")
    (push paper alect-colors)
    (eval '(alect-create-theme paper))
    (unwind-protect
        (progn
          (enable-theme 'alect-paper)
          (list
           (custom-theme-p 'alect-paper)
           (get 'alect-paper 'theme-feature)
           (get 'alect-paper 'theme-documentation)
           (length (get 'alect-paper 'theme-settings))
           custom-enabled-themes
           (face-attribute
            'default :foreground nil 'default)
           (face-attribute
            'default :background nil 'default)
           (face-attribute
            'font-lock-function-name-face
            :foreground nil 'default)))
      (disable-theme 'alect-paper))))
"##;
    let expect = expect![[
        r##"OK ((alect-paper user changed) alect-paper-theme "The paper color theme." 944 (alect-paper) "#181818" "#fefcf7" "#2050a0")"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}
