use expect_test::expect;

use super::assert_amber_glow_theme_parity;

#[test]
fn amber_glow_theme_declares_complete_exact_face_setting_corpus() {
    let elisp_form = r##"(let ((settings
                        (get 'amber-glow
                             'theme-settings)))
         (list
          (length settings)
          (mapcar
           (lambda (setting)
             (list
              (car setting)
              (cadr setting)))
           settings)
          (secure-hash
           'sha256
           (prin1-to-string settings))))"##;
    let expect = expect![[
        r#"OK (18 ((theme-face minibuffer-prompt) (theme-face mode-line-inactive) (theme-face mode-line) (theme-face font-lock-warning-face) (theme-face font-lock-variable-name-face) (theme-face font-lock-type-face) (theme-face font-lock-string-face) (theme-face font-lock-keyword-face) (theme-face font-lock-function-name-face) (theme-face font-lock-constant-face) (theme-face font-lock-comment-face) (theme-face font-lock-builtin-face) (theme-face vertical-border) (theme-face highlight) (theme-face region) (theme-face fringe) (theme-face cursor) (theme-face default)) "c215dbb74fd2eeb301069e9076cf4126425281e2823843d08b2eb6fd6681208c")"#
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_basic_palette_specs_are_exact() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (list face
                 (cadr
                  (assq
                   face
                   (get 'amber-glow
                        'theme-settings)))))
         '(default cursor fringe region
           highlight vertical-border))"##;
    let expect = expect![
        "OK ((default nil) (cursor nil) (fringe nil) (region nil) (highlight nil) (vertical-border nil))"
    ];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_font_lock_palette_specs_are_exact() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (list face
                 (cadr
                  (assq
                   face
                   (get 'amber-glow
                        'theme-settings)))))
         '(font-lock-builtin-face
           font-lock-comment-face
           font-lock-constant-face
           font-lock-function-name-face
           font-lock-keyword-face
           font-lock-string-face
           font-lock-type-face
           font-lock-variable-name-face
           font-lock-warning-face))"##;
    let expect = expect![
        "OK ((font-lock-builtin-face nil) (font-lock-comment-face nil) (font-lock-constant-face nil) (font-lock-function-name-face nil) (font-lock-keyword-face nil) (font-lock-string-face nil) (font-lock-type-face nil) (font-lock-variable-name-face nil) (font-lock-warning-face nil))"
    ];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_ui_palette_specs_are_exact() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (list face
                 (cadr
                  (assq
                   face
                   (get 'amber-glow
                        'theme-settings)))))
         '(mode-line mode-line-inactive
           minibuffer-prompt))"##;
    let expect = expect!["OK ((mode-line nil) (mode-line-inactive nil) (minibuffer-prompt nil))"];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_defines_only_its_eighteen_intended_faces() {
    let elisp_form = r##"(let ((faces
                        (mapcar
                         #'cadr
                         (get 'amber-glow
                              'theme-settings))))
         (list
          faces
          (length faces)
          (= (length faces)
             (length
              (delete-dups
               (copy-sequence faces))))
          (seq-difference
           faces
           '(default cursor fringe region
             highlight vertical-border
             font-lock-builtin-face
             font-lock-comment-face
             font-lock-constant-face
             font-lock-function-name-face
             font-lock-keyword-face
             font-lock-string-face
             font-lock-type-face
             font-lock-variable-name-face
             font-lock-warning-face
             mode-line mode-line-inactive
             minibuffer-prompt))))"##;
    let expect = expect![
        "OK ((minibuffer-prompt mode-line-inactive mode-line font-lock-warning-face font-lock-variable-name-face font-lock-type-face font-lock-string-face font-lock-keyword-face font-lock-function-name-face font-lock-constant-face font-lock-comment-face font-lock-builtin-face vertical-border highlight region fringe cursor default) 18 t nil)"
    ];
    assert_amber_glow_theme_parity(elisp_form, expect);
}
