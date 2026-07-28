use expect_test::expect;

use super::assert_almost_mono_themes_parity;

/// The generation, seen from both ends: the palette table each variant is built
/// from, and the same eighteen user-visible faces resolved under all four
/// themes in turn.  Only the nine palette roles differ - the structure, the
/// weights, the slants and the shared accent colours are identical across
/// white, black, gray and cream.
#[test]
fn every_variant_paints_the_same_faces_from_its_own_palette() {
    let elisp_form = r##"(progn
  (require 'hl-line)
  (require 'org)
  (list
   (mapcar #'car almost-mono-themes-colors)
   (mapcar (lambda (variant)
             (cons (car variant)
                   (mapcar (lambda (role) (cons (car role) (am-test-copy (cdr role))))
                           (cdr variant))))
           almost-mono-themes-colors)
   (mapcar (lambda (theme)
             (cons theme (am-test-with-theme theme (am-test-face-report))))
           am-test-variants)))"##;
    let expect = expect![[
        r##"OK ((white black gray cream) ((white (background . "#ffffff") (foreground . "#000000") (weak . "#888888") (weaker . "#dddddd") (weakest . "#efefef") (highlight . "#fda50f") (warning . "#ff0000") (success . "#00ff00") (string . "#3c5e2b")) (black (background . "#000000") (foreground . "#ffffff") (weak . "#aaaaaa") (weaker . "#666666") (weakest . "#222222") (highlight . "#fda50f") (warning . "#ff0000") (success . "#00ff00") (string . "#a7bca4")) (gray (background . "#2b2b2b") (foreground . "#ffffff") (weak . "#aaaaaa") (weaker . "#666666") (weakest . "#222222") (highlight . "#fda50f") (warning . "#ff0000") (success . "#00ff00") (string . "#a7bca4")) (cream (background . "#f0e5da") (foreground . "#000000") (weak . "#7d7165") (weaker . "#c4baaf") (weakest . "#dbd0c5") (highlight . "#fda50f") (warning . "#ff0000") (success . "#00ff00") (string . "#3c5e2b"))) ((almost-mono-white (default (:background . "#ffffff") (:foreground . "#000000")) (region (:background . "#fda50f") (:foreground . "#000000")) (isearch (:background . "#888888") (:weight . bold)) (lazy-highlight (:background . "#dddddd")) (font-lock-comment-face (:foreground . "#888888") (:slant . italic)) (font-lock-string-face (:foreground . "#3c5e2b")) (font-lock-keyword-face (:weight . bold)) (font-lock-type-face (:slant . italic)) (line-number (:foreground . "#dddddd")) (hl-line (:background . "#efefef")) (mode-line (:background . "#efefef") (:foreground . "#000000") (:box :line-width -1 :color "#dddddd")) (org-todo (:foreground . "#fda50f") (:weight . bold)) (org-done (:foreground . "#00ff00") (:weight . bold)) (show-paren-match (:foreground . "#00ff00") (:weight . bold)) (minibuffer-prompt (:foreground . "#000000") (:weight . bold)) (completions-common-part (:weight . bold) (:underline . t)) (vertical-border (:foreground . "#dddddd"))) (almost-mono-black (default (:background . "#000000") (:foreground . "#ffffff")) (region (:background . "#fda50f") (:foreground . "#ffffff")) (isearch (:background . "#aaaaaa") (:weight . bold)) (lazy-highlight (:background . "#666666")) (font-lock-comment-face (:foreground . "#aaaaaa") (:slant . italic)) (font-lock-string-face (:foreground . "#a7bca4")) (font-lock-keyword-face (:weight . bold)) (font-lock-type-face (:slant . italic)) (line-number (:foreground . "#666666")) (hl-line (:background . "#222222")) (mode-line (:background . "#222222") (:foreground . "#ffffff") (:box :line-width -1 :color "#666666")) (org-todo (:foreground . "#fda50f") (:weight . bold)) (org-done (:foreground . "#00ff00") (:weight . bold)) (show-paren-match (:foreground . "#00ff00") (:weight . bold)) (minibuffer-prompt (:foreground . "#ffffff") (:weight . bold)) (completions-common-part (:weight . bold) (:underline . t)) (vertical-border (:foreground . "#666666"))) (almost-mono-gray (default (:background . "#2b2b2b") (:foreground . "#ffffff")) (region (:background . "#fda50f") (:foreground . "#ffffff")) (isearch (:background . "#aaaaaa") (:weight . bold)) (lazy-highlight (:background . "#666666")) (font-lock-comment-face (:foreground . "#aaaaaa") (:slant . italic)) (font-lock-string-face (:foreground . "#a7bca4")) (font-lock-keyword-face (:weight . bold)) (font-lock-type-face (:slant . italic)) (line-number (:foreground . "#666666")) (hl-line (:background . "#222222")) (mode-line (:background . "#222222") (:foreground . "#ffffff") (:box :line-width -1 :color "#666666")) (org-todo (:foreground . "#fda50f") (:weight . bold)) (org-done (:foreground . "#00ff00") (:weight . bold)) (show-paren-match (:foreground . "#00ff00") (:weight . bold)) (minibuffer-prompt (:foreground . "#ffffff") (:weight . bold)) (completions-common-part (:weight . bold) (:underline . t)) (vertical-border (:foreground . "#666666"))) (almost-mono-cream (default (:background . "#f0e5da") (:foreground . "#000000")) (region (:background . "#fda50f") (:foreground . "#000000")) (isearch (:background . "#7d7165") (:weight . bold)) (lazy-highlight (:background . "#c4baaf")) (font-lock-comment-face (:foreground . "#7d7165") (:slant . italic)) (font-lock-string-face (:foreground . "#3c5e2b")) (font-lock-keyword-face (:weight . bold)) (font-lock-type-face (:slant . italic)) (line-number (:foreground . "#c4baaf")) (hl-line (:background . "#dbd0c5")) (mode-line (:background . "#dbd0c5") (:foreground . "#000000") (:box :line-width -1 :color "#c4baaf")) (org-todo (:foreground . "#fda50f") (:weight . bold)) (org-done (:foreground . "#00ff00") (:weight . bold)) (show-paren-match (:foreground . "#00ff00") (:weight . bold)) (minibuffer-prompt (:foreground . "#000000") (:weight . bold)) (completions-common-part (:weight . bold) (:underline . t)) (vertical-border (:foreground . "#c4baaf")))))"##
    ]];

    assert_almost_mono_themes_parity(elisp_form, expect);
}

/// What a user actually looks at: a real Elisp buffer, font-locked, with the
/// face at each token and the colour that face resolves to.  It also pins two
/// details of the shared spec - `font-lock-doc-face' inherits the comment
/// face's colour and slant, and `font-lock-variable-name-face' is listed twice
/// in the specification, the earlier entry winning, so it ends up with no
/// slant.
#[test]
fn the_white_theme_paints_a_font_locked_elisp_buffer() {
    let elisp_form = r##"(am-test-with-theme 'almost-mono-white
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert ";; Configure the reader\n"
            "(defun demo-reader (path)\n"
            "  \"Read PATH and return its contents.\"\n"
            "  (let ((coding-system-for-read 'utf-8))\n"
            "    (message \"reading %s\" path)\n"
            "    t))\n")
    (font-lock-ensure)
    (list (am-test-token-faces
           '(";; Configure the reader" "defun" "demo-reader"
             "\"Read PATH and return its contents.\"" "let"
             "'utf-8" "message" "\"reading %s\"" "path"))
          (am-test-face-report
           '((default :background :foreground)
             (font-lock-comment-face :foreground :slant)
             (font-lock-doc-face :foreground :slant)
             (font-lock-string-face :foreground)
             (font-lock-constant-face :weight :slant)
             (font-lock-function-name-face :weight)
             (font-lock-variable-name-face :foreground :slant)
             (font-lock-warning-face :foreground :underline)))
          (buffer-substring-no-properties (point-min) (point-max)))))"##;
    let expect = expect![[
        r##"OK (((";; Configure the reader" font-lock-comment-delimiter-face "#888888" unspecified italic) ("defun" font-lock-keyword-face unspecified bold unspecified) ("demo-reader" font-lock-function-name-face unspecified bold unspecified) ("\"Read PATH and return its contents.\"" font-lock-doc-face "#888888" unspecified italic) ("let" font-lock-keyword-face unspecified bold unspecified) ("'utf-8" nil nil nil nil) ("message" nil nil nil nil) ("\"reading %s\"" font-lock-string-face "#3c5e2b" unspecified unspecified) ("path" nil nil nil nil)) ((default (:background . "#ffffff") (:foreground . "#000000")) (font-lock-comment-face (:foreground . "#888888") (:slant . italic)) (font-lock-doc-face (:foreground . "#888888") (:slant . italic)) (font-lock-string-face (:foreground . "#3c5e2b")) (font-lock-constant-face (:weight . bold) (:slant . italic)) (font-lock-function-name-face (:weight . bold)) (font-lock-variable-name-face (:foreground . "#000000") (:slant . unspecified)) (font-lock-warning-face (:foreground . "#000000") (:underline :color "#ff0000" :style wave))) ";; Configure the reader\n(defun demo-reader (path)\n  \"Read PATH and return its contents.\"\n  (let ((coding-system-for-read 'utf-8))\n    (message \"reading %s\" path)\n    t))\n")"##
    ]];

    assert_almost_mono_themes_parity(elisp_form, expect);
}

/// The lifecycle a user goes through: capture the untouched faces, enable white,
/// enable black on top - both are enabled at once and the later one wins -
/// disable black and get white's colours back, then disable white and get the
/// original values back exactly, `equal' to the captured baseline.
#[test]
fn switching_variants_repaints_and_disabling_restores_the_baseline() {
    let elisp_form = r##"(progn
  (require 'hl-line)
  (let ((baseline (am-test-face-report
                   '((default :background :foreground)
                     (region :background)
                     (font-lock-comment-face :foreground :slant)
                     (line-number :foreground)
                     (hl-line :background)))))
    (load-theme 'almost-mono-white t)
    (let ((white (list (copy-sequence custom-enabled-themes)
                       (am-test-face-report
                        '((default :background :foreground)
                          (font-lock-comment-face :foreground)
                          (hl-line :background))))))
      (load-theme 'almost-mono-black t)
      (let ((black-on-top (list (copy-sequence custom-enabled-themes)
                                (am-test-face-report
                                 '((default :background :foreground)
                                   (font-lock-comment-face :foreground)
                                   (hl-line :background))))))
        (disable-theme 'almost-mono-black)
        (let ((back-to-white (list (copy-sequence custom-enabled-themes)
                                   (am-test-face-report
                                    '((default :background :foreground)
                                      (font-lock-comment-face :foreground)
                                      (hl-line :background))))))
          (disable-theme 'almost-mono-white)
          (let ((restored (am-test-face-report
                           '((default :background :foreground)
                             (region :background)
                             (font-lock-comment-face :foreground :slant)
                             (line-number :foreground)
                             (hl-line :background)))))
            (list baseline white black-on-top back-to-white
                  (copy-sequence custom-enabled-themes)
                  restored
                  (equal baseline restored))))))))"##;
    let expect = expect![[
        r##"OK (((default (:background . "unspecified-bg") (:foreground . "unspecified-fg")) (region (:background . unspecified)) (font-lock-comment-face (:foreground . unspecified) (:slant . italic)) (line-number (:foreground . "unspecified-fg")) (hl-line (:background . unspecified))) ((almost-mono-white) ((default (:background . "#ffffff") (:foreground . "#000000")) (font-lock-comment-face (:foreground . "#888888")) (hl-line (:background . "#efefef")))) ((almost-mono-black almost-mono-white) ((default (:background . "#000000") (:foreground . "#ffffff")) (font-lock-comment-face (:foreground . "#aaaaaa")) (hl-line (:background . "#222222")))) ((almost-mono-white) ((default (:background . "#ffffff") (:foreground . "#000000")) (font-lock-comment-face (:foreground . "#888888")) (hl-line (:background . "#efefef")))) nil ((default (:background . "unspecified-bg") (:foreground . "unspecified-fg")) (region (:background . unspecified)) (font-lock-comment-face (:foreground . unspecified) (:slant . italic)) (line-number (:foreground . "unspecified-fg")) (hl-line (:background . unspecified))) t)"##
    ]];

    assert_almost_mono_themes_parity(elisp_form, expect);
}

/// Org is where this family's palette does most of its work: a real dashboard
/// with a title, a TODO and a DONE headline, a properties drawer and a table,
/// font-locked, with the face at each token - several of them composed lists -
/// and the accent colours behind them.
#[test]
fn the_cream_theme_styles_an_org_dashboard() {
    let elisp_form = r##"(progn
  (require 'org)
  (am-test-with-theme 'almost-mono-cream
    (with-temp-buffer
      (org-mode)
      (insert "#+title: Release dashboard\n"
              "* TODO Ship release\n"
              ":PROPERTIES:\n"
              ":Owner: Ada\n"
              ":END:\n"
              "* DONE Archive notes\n"
              "| Item | State |\n"
              "| API  | Ready |\n")
      (font-lock-ensure)
      (list (am-test-token-faces
             '("#+title:" "TODO" "Ship release" ":PROPERTIES:" ":Owner:"
               "DONE" "Archive notes" "| Item | State |"))
            (am-test-face-report
             '((org-todo :foreground :weight)
               (org-done :foreground :weight)
               (org-drawer :foreground)
               (org-special-keyword :foreground :weight)
               (org-property-value :foreground :slant)
               (org-table :foreground)
               (org-document-title :foreground)
               (org-hide :foreground)))))))"##;
    let expect = expect![[
        r##"OK ((("#+title:" org-document-info-keyword unspecified unspecified unspecified) ("TODO" (org-todo org-level-1) "#fda50f" bold unspecified) ("Ship release" org-level-1 unspecified bold unspecified) (":PROPERTIES:" org-drawer "#7d7165" unspecified unspecified) (":Owner:" org-special-keyword "#7d7165" bold unspecified) ("DONE" (org-done org-level-1) "#00ff00" bold unspecified) ("Archive notes" (org-headline-done org-level-1) "#000000" bold unspecified) ("| Item | State |" org-table "#7d7165" unspecified unspecified)) ((org-todo (:foreground . "#fda50f") (:weight . bold)) (org-done (:foreground . "#00ff00") (:weight . bold)) (org-drawer (:foreground . "#7d7165")) (org-special-keyword (:foreground . "#7d7165") (:weight . bold)) (org-property-value (:foreground . "#7d7165") (:slant . italic)) (org-table (:foreground . "#7d7165")) (org-document-title (:foreground . "#000000")) (org-hide (:foreground . "#f0e5da"))))"##
    ]];

    assert_almost_mono_themes_parity(elisp_form, expect);
}

/// A real unified diff under the dark variant, showing which faces diff-mode
/// gives each line and the surrounding chrome the theme sets: the background,
/// line numbers, the region and search highlights, and the window divider.
#[test]
fn the_gray_theme_styles_a_unified_diff() {
    let elisp_form = r##"(progn
  (require 'diff-mode)
  (am-test-with-theme 'almost-mono-gray
    (with-temp-buffer
      (diff-mode)
      (insert "--- a/config.el\n"
              "+++ b/config.el\n"
              "@@ -1,4 +1,4 @@\n"
              " (setq inhibit-startup-screen t)\n"
              "-(setq make-backup-files t)\n"
              "+(setq make-backup-files nil)\n"
              " (global-display-line-numbers-mode)\n")
      (font-lock-ensure)
      (list (am-test-token-faces
             '("--- a/config.el" "+++ b/config.el" "@@ -1,4 +1,4 @@"
               "-(setq make-backup-files t)" "+(setq make-backup-files nil)"))
            (am-test-face-report
             '((default :background :foreground)
               (line-number :foreground)
               (region :background)
               (isearch :background :weight)
               (lazy-highlight :background)
               (vertical-border :foreground)))))))"##;
    let expect = expect![[
        r##"OK ((("--- a/config.el" diff-header unspecified bold unspecified) ("+++ b/config.el" diff-header unspecified bold unspecified) ("@@ -1,4 +1,4 @@" diff-hunk-header unspecified bold unspecified) ("-(setq make-backup-files t)" diff-indicator-removed unspecified unspecified unspecified) ("+(setq make-backup-files nil)" diff-indicator-added unspecified unspecified unspecified)) ((default (:background . "#2b2b2b") (:foreground . "#ffffff")) (line-number (:foreground . "#666666")) (region (:background . "#fda50f")) (isearch (:background . "#aaaaaa") (:weight . bold)) (lazy-highlight (:background . "#666666")) (vertical-border (:foreground . "#666666"))))"##
    ]];

    assert_almost_mono_themes_parity(elisp_form, expect);
}

/// What installation gives you: the package directory is on
/// `custom-theme-load-path', the four one-line variant files are there, all
/// four themes are offered by `custom-available-themes', each loads by name
/// with the same 73 face settings and its own background, each is marked
/// `theme-immediate', and a variant that does not exist reports the usual
/// missing-theme error.
#[test]
fn installing_the_package_offers_all_four_variants_by_name() {
    let elisp_form = r##"(list
 (let ((directory (file-name-directory (locate-library "almost-mono-themes"))))
   (list (and (member (file-name-as-directory directory) custom-theme-load-path) t)
         (sort (mapcar #'am-test-copy
                       (mapcar #'file-name-nondirectory
                               (directory-files directory t "almost-mono.*\\.el\\'")))
               #'string<)))
 (sort (cl-remove-if-not (lambda (theme)
                           (string-prefix-p "almost-mono" (symbol-name theme)))
                         (custom-available-themes))
       (lambda (a b) (string< (symbol-name a) (symbol-name b))))
 (mapcar (lambda (theme)
           (list theme
                 (and (custom-theme-p theme) t)
                 (get theme 'theme-immediate)
                 (am-test-with-theme theme
                   (list (and (custom-theme-p theme) t)
                         (get theme 'theme-immediate)
                         (length (get theme 'theme-settings))
                         (am-test-copy (face-attribute 'default :background nil t))))))
         am-test-variants)
 (condition-case error (progn (load-theme 'almost-mono-purple t) :loaded)
   (error (list (car error) (am-test-copy (cadr error))))))"##;
    let expect = expect![[
        r##"OK ((t ("almost-mono-black-theme.el" "almost-mono-cream-theme.el" "almost-mono-gray-theme.el" "almost-mono-themes-autoloads.el" "almost-mono-themes-pkg.el" "almost-mono-themes.el" "almost-mono-white-theme.el")) (almost-mono-black almost-mono-cream almost-mono-gray almost-mono-white) ((almost-mono-white nil nil (t t 73 "#ffffff")) (almost-mono-black nil nil (t t 73 "#000000")) (almost-mono-gray nil nil (t t 73 "#2b2b2b")) (almost-mono-cream nil nil (t t 73 "#f0e5da"))) (error "Unable to find theme file for ‘almost-mono-purple’"))"##
    ]];

    assert_almost_mono_themes_parity(elisp_form, expect);
}
