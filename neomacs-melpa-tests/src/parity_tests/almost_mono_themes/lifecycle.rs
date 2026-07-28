use expect_test::expect;

use super::assert_almost_mono_themes_parity;

#[test]
fn readme_theme_switching_updates_real_code_and_restores_the_previous_visual_state() {
    let elisp_form = r##"(let ((white 'almost-mono-white)
      (black 'almost-mono-black))
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert
     ";;; Release workflow\n(defun ship-release (candidate)\n  \"Publish CANDIDATE safely.\"\n  (message \"ready: %s\" candidate))\n")
    (font-lock-ensure)
    (let* ((describe-token
            (lambda (token)
              (goto-char (point-min))
              (search-forward token)
              (let* ((position (- (point) (length token)))
                     (face (get-text-property position 'face))
                     (primary (if (listp face) (car face) face)))
                (list
                 token
                 face
                 (and primary
                      (face-attribute
                       primary :foreground nil 'default))
                 (and primary
                      (face-attribute
                       primary :weight nil 'default))
                 (and primary
                      (face-attribute
                       primary :slant nil 'default))))))
           (snapshot
            (lambda ()
              (list
               (copy-sequence custom-enabled-themes)
               (face-attribute 'default :background nil 'default)
               (face-attribute 'default :foreground nil 'default)
               (face-attribute 'region :background nil 'default)
               (list
                (face-attribute
                 'mode-line :background nil 'default)
                (face-attribute
                 'mode-line :foreground nil 'default)
                (copy-tree
                 (face-attribute
                  'mode-line :box nil 'default)))
               (mapcar
                describe-token
                '("Release workflow" "defun" "ship-release"
                  "Publish CANDIDATE" "\"ready: %s\""))))))
      (unwind-protect
          (let ((before (funcall snapshot)))
            (load-theme white t)
            (let ((white-state (funcall snapshot)))
              (load-theme black t)
              (let ((black-state (funcall snapshot)))
                (disable-theme black)
                (let ((restored-white (funcall snapshot)))
                  (disable-theme white)
                  (let ((after (funcall snapshot)))
                    (list
                     before
                     white-state
                     black-state
                     restored-white
                     after
                     (equal white-state restored-white)
                     (equal before after)))))))
        (dolist (theme (list black white))
          (when (memq theme custom-enabled-themes)
            (disable-theme theme)))))))"##;
    let expect = expect![[
        r##"OK ((nil "unspecified-bg" "unspecified-fg" "unspecified-bg" ("unspecified-bg" "unspecified-fg" nil) (("Release workflow" font-lock-comment-face "unspecified-fg" bold italic) ("defun" font-lock-keyword-face "unspecified-fg" bold normal) ("ship-release" font-lock-function-name-face "unspecified-fg" bold normal) ("Publish CANDIDATE" font-lock-doc-face "unspecified-fg" normal italic) ("\"ready: %s\"" font-lock-string-face "unspecified-fg" normal italic))) ((almost-mono-white) "#ffffff" "#000000" "#fda50f" ("#efefef" "#000000" (:line-width -1 :color "#dddddd")) (("Release workflow" font-lock-comment-face "#888888" normal italic) ("defun" font-lock-keyword-face "#000000" bold normal) ("ship-release" font-lock-function-name-face "#000000" bold normal) ("Publish CANDIDATE" font-lock-doc-face "#888888" normal italic) ("\"ready: %s\"" font-lock-string-face "#3c5e2b" normal normal))) ((almost-mono-black almost-mono-white) "#000000" "#ffffff" "#fda50f" ("#222222" "#ffffff" (:line-width -1 :color "#666666")) (("Release workflow" font-lock-comment-face "#aaaaaa" normal italic) ("defun" font-lock-keyword-face "#ffffff" bold normal) ("ship-release" font-lock-function-name-face "#ffffff" bold normal) ("Publish CANDIDATE" font-lock-doc-face "#aaaaaa" normal italic) ("\"ready: %s\"" font-lock-string-face "#a7bca4" normal normal))) ((almost-mono-white) "#ffffff" "#000000" "#fda50f" ("#efefef" "#000000" (:line-width -1 :color "#dddddd")) (("Release workflow" font-lock-comment-face "#888888" normal italic) ("defun" font-lock-keyword-face "#000000" bold normal) ("ship-release" font-lock-function-name-face "#000000" bold normal) ("Publish CANDIDATE" font-lock-doc-face "#888888" normal italic) ("\"ready: %s\"" font-lock-string-face "#3c5e2b" normal normal))) (nil "unspecified-bg" "unspecified-fg" "unspecified-bg" ("unspecified-bg" "unspecified-fg" nil) (("Release workflow" font-lock-comment-face "unspecified-fg" bold italic) ("defun" font-lock-keyword-face "unspecified-fg" bold normal) ("ship-release" font-lock-function-name-face "unspecified-fg" bold normal) ("Publish CANDIDATE" font-lock-doc-face "unspecified-fg" normal italic) ("\"ready: %s\"" font-lock-string-face "unspecified-fg" normal italic))) t t)"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn temporary_face_adjustments_do_not_leak_across_preloaded_theme_switches() {
    let elisp_form = r##"(let ((white 'almost-mono-white)
      (black 'almost-mono-black)
      (snapshot
       (lambda ()
         (list
          (copy-sequence custom-enabled-themes)
          (list
           (face-attribute
            'isearch :background nil 'default)
           (face-attribute
            'isearch :weight nil 'default))
          (list
           (face-attribute
            'font-lock-comment-face
            :foreground nil 'default)
           (face-attribute
            'font-lock-comment-face
            :slant nil 'default))
          (copy-tree
           (face-attribute
            'font-lock-warning-face
            :underline nil 'default))))))
  (unwind-protect
      (progn
        (load-theme white t t)
        (load-theme black t t)
        (enable-theme white)
        (set-face-attribute
         'isearch nil
         :background "#123456"
         :foreground "#ffffff"
         :weight 'normal)
        (set-face-attribute
         'font-lock-comment-face nil
         :foreground "#654321"
         :slant 'normal)
        (set-face-attribute
         'font-lock-warning-face nil
         :foreground "#ffffff"
         :underline
         '(:color "#00ffff" :style line))
        (let ((adjusted-white (funcall snapshot)))
          (disable-theme white)
          (enable-theme black)
          (let ((black-state (funcall snapshot)))
            (disable-theme black)
            (enable-theme white)
            (let ((restored-white (funcall snapshot)))
              (list
               adjusted-white
               black-state
               restored-white)))))
    (dolist (theme (list black white))
      (when (memq theme custom-enabled-themes)
        (disable-theme theme)))))"##;
    let expect = expect![[
        r##"OK (((almost-mono-white) ("#123456" normal) ("#654321" normal) (:color "#00ffff" :style line)) ((almost-mono-black) ("#aaaaaa" bold) ("#aaaaaa" italic) (:color "#ff0000" :style wave)) ((almost-mono-white) ("#888888" bold) ("#888888" italic) (:color "#ff0000" :style wave)))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}
