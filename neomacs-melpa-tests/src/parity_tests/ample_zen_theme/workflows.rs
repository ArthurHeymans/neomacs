use expect_test::expect;

use super::assert_ample_zen_theme_parity;

/// The round trip: twenty-two faces resolved before `load-theme', while the
/// theme is on, and after `disable-theme', with an `equal' between the baseline
/// and the restored report.
///
/// ample-zen also makes five `custom-theme-set-variables' settings, and they are
/// in the report because they land in three different ways.
/// `ansi-color-names-vector' and the three `vc-annotate' settings reach
/// libraries that are loaded afterwards, and carry the theme's values once they
/// are - `custom-declare-variable' consults the enabled themes as the variable
/// is created.  `fci-rule-color' belongs to fill-column-indicator, which is not
/// installed, so that setting is recorded and the variable is never created at
/// all.
///
/// Loading the file also appends a `rainbow-mode' form to
/// `safe-local-eval-forms', which is a global side effect of loading rather than
/// of enabling: it is already there before `load-theme' runs and it survives
/// `disable-theme'.
#[test]
fn enabling_the_theme_repaints_the_editor_and_sets_its_five_variables() {
    let elisp_form = r##"(let ((baseline (zen-test-face-report zen-test-probe-faces))
      (rainbow-form '(when (require 'rainbow-mode nil t) (rainbow-mode 1))))
  (let ((eval-form-before (and (member rainbow-form safe-local-eval-forms) t)))
    (load-theme 'ample-zen t)
    (require 'ansi-color)
    (require 'vc-annotate)
    (let ((enabled (list (copy-sequence custom-enabled-themes)
                         (and (custom-theme-enabled-p 'ample-zen) t)
                         (zen-test-copy-tree ansi-color-names-vector)
                         (boundp 'fci-rule-color)
                         (zen-test-copy-tree vc-annotate-very-old-color)
                         (zen-test-copy-tree vc-annotate-background)
                         (length vc-annotate-color-map)
                         (zen-test-copy-tree (car vc-annotate-color-map))
                         (zen-test-copy-tree (car (last vc-annotate-color-map)))
                         (zen-test-face-report zen-test-probe-faces))))
      (disable-theme 'ample-zen)
      (let ((restored (zen-test-face-report zen-test-probe-faces)))
        (list eval-form-before
              enabled
              (copy-sequence custom-enabled-themes)
              (and (member rainbow-form safe-local-eval-forms) t)
              (equal baseline restored))))))"##;
    let expect = expect![[
        r##"OK (t ((ample-zen) t ["#212121" "#CC5542" "#6aaf50" "#7d7c61" "#5180b3" "#DC8CC3" "#9b55c3" "#bdbdb3"] nil "#DC8CC3" "#3b3b3b" 18 (20 . "#dd5542") (360 . "#DC8CC3") ((default (:foreground . "#bdbdb3") (:background . "#212121")) (cursor (:foreground . "#bdbdb3") (:background . "#cc8512")) (fringe (:foreground . "#bdbdb3") (:background . "#212121")) (highlight (:foreground . unspecified) (:background . "#2e2e2e")) (minibuffer-prompt (:foreground . "#7d7c61")) (link (:foreground . "#7d7c61") (:underline . t) (:weight . bold)) (link-visited (:foreground . "#baba36") (:underline . t) (:weight . normal)) (button (:underline . t)) (isearch (:foreground . "#baba36") (:background . "#3b3b3b")) (lazy-highlight (:foreground . "#baba36") (:background . "#2e2e2e")) (font-lock-keyword-face (:foreground . "#7d7c61") (:weight . bold)) (font-lock-string-face (:foreground . "#CC5542")) (font-lock-comment-face (:foreground . "#6aaf50")) (font-lock-function-name-face (:foreground . "#9b55c3")) (font-lock-variable-name-face (:foreground . "#fb8512")) (font-lock-type-face (:foreground . "#528fd1")) (font-lock-warning-face (:foreground . "#baba36") (:weight . bold)) (mode-line-buffer-id (:foreground . "#cc8512") (:weight . bold)) (mode-line-inactive (:foreground . "#9b9b9b") (:background . "#3b3b3b") (:weight . light)) (secondary-selection (:background . "#0a0a0a")) (trailing-whitespace (:background . "#CC5542")) (vertical-border (:foreground . "#bdbdb3")))) nil t t)"##
    ]];

    assert_ample_zen_theme_parity(elisp_form, expect);
}

/// The nine faces ample-zen writes against `class' rather than `t', which is
/// where this theme differs from the rest of its family.  `class' is bound by
/// `ample-zen-with-color-variables' to `((class color) (min-colors 89))', and a
/// batch frame is a zero-colour `static-gray' display, so that clause does not
/// match - `face-spec-set-match-display' says so for the clause and for `t',
/// both of which are in the report next to the display facts.
///
/// The theme does not simply fail to apply on such a display.  Every one of the
/// nine carries a second clause, and those are what a user gets here: `mode-line'
/// and `region' become inverse video with no colours at all, `hl-line' becomes
/// bold with no background, and `diff-added', `diff-removed' and the two diff
/// headers take a *different* set of colours from the ones the colour clause
/// would have given - `#6abd50' rather than `#6a7550' for an added line, and an
/// inverted header rather than a dark one.  Both clauses of each registered spec
/// are pinned so the two can be compared.
///
/// Only two of the nine exist when the theme is enabled.  Five more are created
/// when `diff-mode' and `hl-line' are loaded afterwards, and are resolved again
/// then, which shows the fallback being chosen at the moment the face is created
/// rather than at the moment the theme was enabled.  The last two,
/// `hl-line-face' and `hl-sexp-face', are names no current Emacs defines at all
/// - `hl-line-face' is a variable rather than a face now - so those two specs
/// reach nothing on either clause.
#[test]
fn the_faces_written_for_a_colour_display_take_their_fallback_clause_here() {
    let elisp_form = r##"(let ((existence-before (mapcar (lambda (face) (list face (and (facep face) t)))
                                zen-test-class-faces))
      (specs (mapcar (lambda (face) (cons face (zen-test-theme-spec face)))
                     zen-test-class-faces)))
  (unwind-protect
      (progn
        (load-theme 'ample-zen t)
        (let ((display (list :graphic (display-graphic-p)
                             :color-cells (display-color-cells)
                             :visual-class (display-visual-class)
                             :matches-colour-clause
                             (face-spec-set-match-display
                              '((class color) (min-colors 89)) nil)
                             :matches-fallback (face-spec-set-match-display t nil)))
              (already-defined (zen-test-face-report
                                '((mode-line :foreground :background :box :inverse-video)
                                  (region :background :inverse-video)))))
          (require 'diff-mode)
          (require 'hl-line)
          (list display
                existence-before
                specs
                already-defined
                (zen-test-face-report
                 '((diff-added :foreground :background)
                   (diff-removed :foreground :background)
                   (diff-header :background :foreground)
                   (diff-file-header :background :foreground :weight)
                   (hl-line :background :weight))))))
    (disable-theme 'ample-zen)))"##;
    let expect = expect![[
        r##"OK ((:graphic nil :color-cells 0 :visual-class static-gray :matches-colour-clause nil :matches-fallback t) ((mode-line t) (region t) (diff-added nil) (diff-removed nil) (diff-header nil) (diff-file-header nil) (hl-line nil) (hl-line-face nil) (hl-sexp-face nil)) ((mode-line (((class color) (min-colors 89)) (:foreground "#c9c9c9" :background "#000000" :box (:line-width -1 :style released-button))) (t :inverse-video t)) (region (((class color) (min-colors 89)) (:background "#3b3b3b")) (t :inverse-video t)) (diff-added (((class color) (min-colors 89)) (:foreground "#6a7550" :background nil)) (t (:foreground "#6abd50" :background nil))) (diff-removed (((class color) (min-colors 89)) (:foreground "#CC5542" :background nil)) (t (:foreground "#ff5542" :background nil))) (diff-header (((class color) (min-colors 89)) (:background "#0a0a0a")) (t (:background "#bdbdb3" :foreground "#212121"))) (diff-file-header (((class color) (min-colors 89)) (:background "#0a0a0a" :foreground "#bdbdb3" :bold t)) (t (:background "#bdbdb3" :foreground "#212121" :bold t))) (hl-line (((class color) (min-colors 89)) (:background "#2e2e2e")) (t :weight bold)) (hl-line-face (((class color) (min-colors 89)) (:background "#2e2e2e")) (t :weight bold)) (hl-sexp-face (((class color) (min-colors 89)) (:background "#141414")) (t :weight bold))) ((mode-line (:foreground . unspecified) (:background . unspecified) (:box . unspecified) (:inverse-video . t)) (region (:background . unspecified) (:inverse-video . t))) ((diff-added (:foreground . "#6abd50") (:background . unspecified)) (diff-removed (:foreground . "#ff5542") (:background . unspecified)) (diff-header (:background . "#bdbdb3") (:foreground . "#212121")) (diff-file-header (:background . "#bdbdb3") (:foreground . "#212121") (:weight . bold)) (hl-line (:background . unspecified) (:weight . bold))))"##
    ]];

    assert_ample_zen_theme_parity(elisp_form, expect);
}

/// Reading code in it: a real Elisp buffer, really font-locked, with each
/// token's face, colour, weight and slant.
///
/// The palette the colours come from is public - `ample-zen-colors-alist' is a
/// documented `defvar' and `ample-zen-with-color-variables' is the macro the
/// theme is written with, offered to anyone extending it - so the workflow reads
/// four entries out of the alist, two of them again through the macro, and shows
/// the same strings arriving on the tokens.  `class' is bound by that macro too,
/// and its value is in the report, which is where the clause in the previous
/// workflow comes from.
#[test]
fn a_font_locked_buffer_is_painted_from_the_public_palette() {
    let elisp_form = r##"(unwind-protect
    (progn
      (load-theme 'ample-zen t)
      (list (length ample-zen-colors-alist)
            (zen-test-copy-tree
             (mapcar (lambda (name) (assoc name ample-zen-colors-alist))
                     '("ample-zen-fg" "ample-zen-bg" "ample-zen-yellow"
                       "ample-zen-magenta")))
            (ample-zen-with-color-variables
              (list class
                    (copy-sequence ample-zen-fg)
                    (copy-sequence ample-zen-yellow)))
            (with-temp-buffer
              (emacs-lisp-mode)
              (insert ";; Zen demo\n"
                      "(defun zen-demo (path)\n"
                      "  \"Read PATH; return its contents.\"\n"
                      "  (let ((limit 10))\n"
                      "    (message \"read %s\" path)\n"
                      "    (car limit)))\n")
              (font-lock-ensure)
              (list (zen-test-token-faces
                     '(";; Zen demo" "defun" "zen-demo"
                       "\"Read PATH; return its contents.\"" "let"
                       "\"read %s\"" "car"))
                    (buffer-substring-no-properties (point-min) (point-max))))))
  (disable-theme 'ample-zen))"##;
    let expect = expect![[
        r##"OK (36 (("ample-zen-fg" . "#bdbdb3") ("ample-zen-bg" . "#212121") ("ample-zen-yellow" . "#7d7c61") ("ample-zen-magenta" . "#DC8CC3")) (((class color) (min-colors 89)) "#bdbdb3" "#7d7c61") (((";; Zen demo" font-lock-comment-delimiter-face "#6abd50" unspecified unspecified) ("defun" font-lock-keyword-face "#7d7c61" bold unspecified) ("zen-demo" font-lock-function-name-face "#9b55c3" unspecified unspecified) ("\"Read PATH; return its contents.\"" font-lock-doc-face "#6a9550" unspecified unspecified) ("let" font-lock-keyword-face "#7d7c61" bold unspecified) ("\"read %s\"" font-lock-string-face "#CC5542" unspecified unspecified) ("car" nil nil nil nil)) ";; Zen demo\n(defun zen-demo (path)\n  \"Read PATH; return its contents.\"\n  (let ((limit 10))\n    (message \"read %s\" path)\n    (car limit)))\n"))"##
    ]];

    assert_ample_zen_theme_parity(elisp_form, expect);
}

/// The same reading applied here as to ample-theme, on a theme that loses more:
/// twelve stock faces resolved across the whole enable/disable pair, next to the
/// standard definition each attribute came from, with an `equal' showing every
/// one of them comes back when the theme is turned off.
///
/// A theme's face spec replaces the standard definition rather than merging with
/// it, so an attribute survives only if ample-zen restates it.  `link',
/// `link-visited' and `button' do keep their underline, because the theme says
/// `:underline t' on all three itself - and `link' additionally gains a weight
/// the stock face does not have.  The rest go.
///
/// The standard specs in the report are what separate the two classes of loss.
/// `font-lock-warning-face' (`(t :inherit error)'), `font-lock-doc-face',
/// `font-lock-comment-delimiter-face', `font-lock-preprocessor-face',
/// `link-visited', `button', `header-line' and `mode-line-inactive' all carry
/// their `:inherit' on an unconditional or `default' clause, so those losses
/// hold on any display.  The weight and slant on the comment and string faces,
/// and `show-paren-match''s underline, sit on `(t ...)' fallbacks reached only
/// because this frame reports zero colours, and a user on a colour terminal
/// never had them to lose.
#[test]
fn enabling_the_theme_drops_the_stock_attributes_it_does_not_mention() {
    let elisp_form = r##"(let* ((probes (mapcar (lambda (face)
                         (cons face zen-test-replaced-attributes))
                       zen-test-replaced-faces))
       (standard (mapcar (lambda (face)
                           (cons face (zen-test-copy-tree (face-default-spec face))))
                         zen-test-replaced-faces))
       (before (zen-test-face-report probes)))
  (unwind-protect
      (progn
        (load-theme 'ample-zen t)
        (let ((after (zen-test-face-report probes)))
          (disable-theme 'ample-zen)
          (let ((restored (zen-test-face-report probes)))
            (list standard
                  before
                  after
                  (equal before after)
                  restored
                  (equal before restored)))))
    (when (custom-theme-enabled-p 'ample-zen)
      (disable-theme 'ample-zen))))"##;
    let expect = expect![[
        r#"OK (((font-lock-warning-face (t :inherit error)) (font-lock-doc-face (t :inherit font-lock-string-face)) (font-lock-comment-delimiter-face (default :inherit font-lock-comment-face)) (font-lock-preprocessor-face (t :inherit font-lock-builtin-face)) (font-lock-comment-face (((class grayscale) (background light)) :foreground "DimGray" :weight bold :slant italic) (((class grayscale) (background dark)) :foreground "LightGray" :weight bold :slant italic) (((class color) (min-colors 88) (background light)) :foreground "Firebrick") (((class color) (min-colors 88) (background dark)) :foreground "chocolate1") (((class color) (min-colors 16) (background light)) :foreground "red") (((class color) (min-colors 16) (background dark)) :foreground "red1") (((class color) (min-colors 8) (background light)) :foreground "red") (((class color) (min-colors 8) (background dark)) :foreground "yellow") (t :weight bold :slant italic)) (font-lock-string-face (((class grayscale) (background light)) :foreground "DimGray" :slant italic) (((class grayscale) (background dark)) :foreground "LightGray" :slant italic) (((class color) (min-colors 88) (background light)) :foreground "VioletRed4") (((class color) (min-colors 88) (background dark)) :foreground "LightSalmon") (((class color) (min-colors 16) (background light)) :foreground "RosyBrown") (((class color) (min-colors 16) (background dark)) :foreground "LightSalmon") (((class color) (min-colors 8)) :foreground "green") (t :slant italic)) (link (((class color) (min-colors 88) (background light)) :foreground "RoyalBlue3" :underline t) (((class color) (background light)) :foreground "blue" :underline t) (((class color) (min-colors 88) (background dark)) :foreground "cyan1" :underline t) (((class color) (background dark)) :foreground "cyan" :underline t) (t :inherit underline)) (link-visited (default :inherit link) (((class color) (background light)) :foreground "magenta4") (((class color) (background dark)) :foreground "violet")) (button (t :inherit link)) (header-line (default :inherit mode-line) (((type tty)) :inverse-video nil :underline t) (((class color grayscale) (background light)) :background "grey90" :foreground "grey20" :box nil) (((class color grayscale) (background dark)) :background "grey20" :foreground "grey90" :box nil) (((class mono) (background light)) :background "white" :foreground "black" :inverse-video nil :box nil :underline t) (((class mono) (background dark)) :background "black" :foreground "white" :inverse-video nil :box nil :underline t)) (mode-line-inactive (default :inherit mode-line) (((class color grayscale) (min-colors 88) (background light)) :weight light :box (:line-width -1 :color "grey75" :style nil) :foreground "grey20" :background "grey90") (((class color grayscale) (min-colors 88) (background dark)) :weight light :box (:line-width -1 :color "grey40" :style nil) :foreground "grey80" :background "grey30")) (show-paren-match (((class color) (background light)) :background "turquoise") (((class color) (background dark)) :background "steelblue3") (((background dark) (min-colors 4)) :background "grey50") (((background light) (min-colors 4)) :background "gray") (t :inherit underline))) ((font-lock-warning-face (:inherit . error) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-doc-face (:inherit . font-lock-string-face) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (font-lock-comment-delimiter-face (:inherit . font-lock-comment-face) (:weight . bold) (:slant . italic) (:underline . unspecified)) (font-lock-preprocessor-face (:inherit . font-lock-builtin-face) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-face (:inherit . unspecified) (:weight . bold) (:slant . italic) (:underline . unspecified)) (font-lock-string-face (:inherit . unspecified) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (link (:inherit . underline) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (link-visited (:inherit . link) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (button (:inherit . link) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (header-line (:inherit . mode-line) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (mode-line-inactive (:inherit . mode-line) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (show-paren-match (:inherit . underline) (:weight . unspecified) (:slant . unspecified) (:underline . t))) ((font-lock-warning-face (:inherit . unspecified) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-doc-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-delimiter-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-preprocessor-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-string-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (link (:inherit . unspecified) (:weight . bold) (:slant . unspecified) (:underline . t)) (link-visited (:inherit . unspecified) (:weight . normal) (:slant . unspecified) (:underline . t)) (button (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (header-line (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (mode-line-inactive (:inherit . unspecified) (:weight . light) (:slant . unspecified) (:underline . unspecified)) (show-paren-match (:inherit . unspecified) (:weight . bold) (:slant . unspecified) (:underline . unspecified))) nil ((font-lock-warning-face (:inherit . error) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-doc-face (:inherit . font-lock-string-face) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (font-lock-comment-delimiter-face (:inherit . font-lock-comment-face) (:weight . bold) (:slant . italic) (:underline . unspecified)) (font-lock-preprocessor-face (:inherit . font-lock-builtin-face) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-face (:inherit . unspecified) (:weight . bold) (:slant . italic) (:underline . unspecified)) (font-lock-string-face (:inherit . unspecified) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (link (:inherit . underline) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (link-visited (:inherit . link) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (button (:inherit . link) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (header-line (:inherit . mode-line) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (mode-line-inactive (:inherit . mode-line) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (show-paren-match (:inherit . underline) (:weight . unspecified) (:slant . unspecified) (:underline . t))) t)"#
    ]];

    assert_ample_zen_theme_parity(elisp_form, expect);
}
