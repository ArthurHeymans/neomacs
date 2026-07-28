use expect_test::expect;

use super::assert_ample_theme_parity;

/// Switching between the three variants the way the README says to: run
/// `M-x ample-theme', `M-x ample-light-theme' or `M-x ample-flat-theme'.  Each
/// command enables its own theme, and the same seventeen faces are resolved
/// under each so the three palettes can be compared side by side - the light
/// variant is not the dark one inverted, it is a different set of colours.
///
/// Each variant also sets `ansi-color-names-vector', its only non-face setting,
/// and the eight terminal colours differ per variant too.  The first part shows
/// that arriving late is not a problem: `ansi-color' is not loaded when the
/// theme is enabled, and the vector still carries the theme's colours once the
/// library is required, because `custom-declare-variable' consults the enabled
/// themes as the variable is created.
///
/// The last elements assert the editor is left exactly as it was found, with an
/// `equal' between the baseline and the restored report rather than a
/// spot-check.
#[test]
fn each_variant_command_paints_its_own_palette_and_disabling_restores_it() {
    let elisp_form = r##"(let ((baseline (ample-test-face-report ample-test-probe-faces))
      (ansi-color-loaded (featurep 'ansi-color)))
  (ample-theme)
  (require 'ansi-color)
  (let ((deferred (list (copy-sequence custom-enabled-themes)
                        (ample-test-copy-tree ansi-color-names-vector))))
    (ample-test-disable-all)
    (let ((variants
           (mapcar (lambda (command)
                     (let ((returned (funcall command)))
                       (prog1 (list command
                                    returned
                                    (copy-sequence custom-enabled-themes)
                                    (ample-test-copy-tree ansi-color-names-vector)
                                    (ample-test-face-report ample-test-probe-faces))
                         (ample-test-disable-all))))
                   ample-test-variants)))
      (let ((restored (ample-test-face-report ample-test-probe-faces)))
        (list ansi-color-loaded
              deferred
              variants
              (copy-sequence custom-enabled-themes)
              (equal baseline restored))))))"##;
    let expect = expect![[
        r##"OK (nil ((ample) ["#454545" "#cd5542" "#6aaf50" "#baba36" "#5180b3" "#ab75c3" "#68a5e9" "#bdbdb3"]) ((ample-theme t (ample) ["#454545" "#cd5542" "#6aaf50" "#baba36" "#5180b3" "#ab75c3" "#68a5e9" "#bdbdb3"] ((default (:foreground . "#bdbdb3") (:background . "gray13")) (cursor (:background . "#f57e00")) (region (:background . "#303030")) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . "cornsilk4") (:foreground . "#252525")) (mode-line-inactive (:background . "#454545") (:foreground . "cornsilk4")) (fringe (:background . "#1f1f1f")) (font-lock-keyword-face (:foreground . "#5180b3")) (font-lock-string-face (:foreground . "#bdbc61")) (font-lock-comment-face (:foreground . "#757575")) (font-lock-function-name-face (:foreground . "#6aaf50")) (font-lock-variable-name-face (:foreground . "#baba36")) (font-lock-type-face (:foreground . "#cd5542")) (font-lock-warning-face (:foreground . "red")) (link (:foreground . "#68a5e9")) (isearch (:background . "#5180b3") (:foreground . "gray13")) (minibuffer-prompt (:foreground . "#fffe0a")))) (ample-light-theme t (ample-light) ["#757575" "#CD5542" "#4A8F30" "#7D7C21" "#4170B3" "#9B55C3" "#68A5E9" "gray43"] ((default (:foreground . "gray43") (:background . "#cBc9b1")) (cursor (:background . "#F57E00")) (region (:background . "#BBB9A1")) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . "#BBB9A1") (:foreground . "gray43")) (mode-line-inactive (:background . "#ABA991") (:foreground . "#cBc9b1")) (fringe (:background . "#CBC9B1")) (font-lock-keyword-face (:foreground . "#4170B3")) (font-lock-string-face (:foreground . "#5D5C01")) (font-lock-comment-face (:foreground . "#959595")) (font-lock-function-name-face (:foreground . "#4A8F30")) (font-lock-variable-name-face (:foreground . "#787800")) (font-lock-type-face (:foreground . "#CD5542")) (font-lock-warning-face (:foreground . "red")) (link (:foreground . "#68A5E9")) (isearch (:background . "#4170B3") (:foreground . "#cBc9b1")) (minibuffer-prompt (:foreground . "#9B55C3")))) (ample-flat-theme t (ample-flat) ["#504545" "#ad8572" "#a9df90" "#aaca86" "#91a0b3" "#ab85a3" "#afcfef" "#bdbdb3"] ((default (:foreground . "#bdbdb3") (:background . "gray15")) (cursor (:background . "#afffef")) (region (:background . "#343030")) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . "cornsilk4") (:foreground . "#302525")) (mode-line-inactive (:background . "#504545") (:foreground . "cornsilk4")) (fringe (:background . "#262424")) (font-lock-keyword-face (:foreground . "#91a0b3")) (font-lock-string-face (:foreground . "#ddbc91")) (font-lock-comment-face (:foreground . "#857575")) (font-lock-function-name-face (:foreground . "#a9df90")) (font-lock-variable-name-face (:foreground . "#aaca86")) (font-lock-type-face (:foreground . "#ad8572")) (font-lock-warning-face (:foreground . "red")) (link (:foreground . "#afcfef")) (isearch (:background . "#91a0b3") (:foreground . "gray15")) (minibuffer-prompt (:foreground . "#caca86"))))) nil t)"##
    ]];

    assert_ample_theme_parity(elisp_form, expect);
}

/// Reading code in each variant.  The same Elisp buffer is typed out and really
/// font-locked under all three, and every token is reported with the face font
/// lock gave it, the colour that face resolves to, and its weight and slant.
///
/// The weights and slants are the point of including them.  On this display a
/// stock Emacs renders keywords bold, comments bold and italic, and docstrings
/// italic; under every ample variant all three come out `unspecified', because
/// the theme replaced those faces' definitions with a colour and nothing else.
/// The colours differ per variant and the flatness does not, which is the
/// visible half of the mechanism the next workflow pins directly.
#[test]
fn a_font_locked_buffer_takes_each_variants_colours_and_loses_its_emphasis() {
    let elisp_form = r##"(unwind-protect
    (mapcar
     (lambda (command)
       (funcall command)
       (prog1
           (cons command
                 (with-temp-buffer
                   (emacs-lisp-mode)
                   (insert ";; Ample demo\n"
                           "(defun ample-demo (path)\n"
                           "  \"Read PATH; return its contents.\"\n"
                           "  (let ((limit 10))\n"
                           "    (message \"read %s\" path)\n"
                           "    (car limit)))\n")
                   (font-lock-ensure)
                   (list (ample-test-token-faces
                          '(";; Ample demo" "defun" "ample-demo"
                            "\"Read PATH; return its contents.\"" "let"
                            "\"read %s\"" "car"))
                         (buffer-substring-no-properties (point-min) (point-max)))))
         (ample-test-disable-all)))
     ample-test-variants)
  (ample-test-disable-all))"##;
    let expect = expect![[
        r##"OK ((ample-theme ((";; Ample demo" font-lock-comment-delimiter-face "#656565" unspecified unspecified) ("defun" font-lock-keyword-face "#5180b3" unspecified unspecified) ("ample-demo" font-lock-function-name-face "#6aaf50" unspecified unspecified) ("\"Read PATH; return its contents.\"" font-lock-doc-face "#7d7c61" unspecified unspecified) ("let" font-lock-keyword-face "#5180b3" unspecified unspecified) ("\"read %s\"" font-lock-string-face "#bdbc61" unspecified unspecified) ("car" nil nil nil nil)) ";; Ample demo\n(defun ample-demo (path)\n  \"Read PATH; return its contents.\"\n  (let ((limit 10))\n    (message \"read %s\" path)\n    (car limit)))\n") (ample-light-theme ((";; Ample demo" font-lock-comment-delimiter-face "#959595" unspecified unspecified) ("defun" font-lock-keyword-face "#4170B3" unspecified unspecified) ("ample-demo" font-lock-function-name-face "#4A8F30" unspecified unspecified) ("\"Read PATH; return its contents.\"" font-lock-doc-face "#7D7C21" unspecified unspecified) ("let" font-lock-keyword-face "#4170B3" unspecified unspecified) ("\"read %s\"" font-lock-string-face "#5D5C01" unspecified unspecified) ("car" nil nil nil nil)) ";; Ample demo\n(defun ample-demo (path)\n  \"Read PATH; return its contents.\"\n  (let ((limit 10))\n    (message \"read %s\" path)\n    (car limit)))\n") (ample-flat-theme ((";; Ample demo" font-lock-comment-delimiter-face "#706565" unspecified unspecified) ("defun" font-lock-keyword-face "#91a0b3" unspecified unspecified) ("ample-demo" font-lock-function-name-face "#a9df90" unspecified unspecified) ("\"Read PATH; return its contents.\"" font-lock-doc-face "#7c7565" unspecified unspecified) ("let" font-lock-keyword-face "#91a0b3" unspecified unspecified) ("\"read %s\"" font-lock-string-face "#ddbc91" unspecified unspecified) ("car" nil nil nil nil)) ";; Ample demo\n(defun ample-demo (path)\n  \"Read PATH; return its contents.\"\n  (let ((limit 10))\n    (message \"read %s\" path)\n    (car limit)))\n"))"##
    ]];

    assert_ample_theme_parity(elisp_form, expect);
}

/// What a theme this size silently throws away.  Twelve stock faces that carry
/// more than a colour are resolved before and after `ample-theme', alongside the
/// standard definition each one comes from, so the report says where the lost
/// attribute was written down as well as that it went.
///
/// A face spec from a theme replaces the standard definition rather than
/// merging with it, so an attribute survives only if ample restates it.  Three
/// do: `link' and `button' say `:underline t' themselves and stay underlined,
/// and `completions-annotations' says `:slant italic'.  Everything else in the
/// report goes.
///
/// The losses divide into two classes and the standard specs in the report are
/// what tell them apart.  Where the attribute comes from a `default' or an
/// unconditional `(t ...)' clause it was in force on every display, so the loss
/// is real for any user: `font-lock-warning-face' stops inheriting `error',
/// `font-lock-doc-face' stops inheriting `font-lock-string-face', `button'
/// stops inheriting `link', `header-line' stops inheriting `mode-line', and
/// `error' - which says `(default :weight bold)' - stops being bold.  Where it
/// comes from the `(t ...)' fallback of a spec whose earlier clauses are
/// colour-conditional, it was only in force because this batch frame reports
/// zero colours; the weight and slant on the font-lock faces are of that kind,
/// and a user on the 256-colour terminal the theme documents would not have had
/// them either.
///
/// `show-paren-match' is worth reading carefully rather than quickly.  Its
/// `:inherit underline' sits on the last clause of its standard spec, so a
/// colour display never had that underline to begin with and only a display
/// like this one loses it; ample sets a foreground and a background on it and
/// restates nothing, so here the matching paren keeps its highlight and comes
/// out unemphasised.  It reads like the strongest case in the list and is
/// actually one of the weakest.
#[test]
fn enabling_a_variant_drops_the_stock_attributes_it_does_not_mention() {
    let elisp_form = r##"(let* ((probes (mapcar (lambda (face)
                         (cons face ample-test-replaced-attributes))
                       ample-test-replaced-faces))
       (standard (mapcar (lambda (face)
                           (cons face (ample-test-copy-tree (face-default-spec face))))
                         ample-test-replaced-faces))
       (before (ample-test-face-report probes)))
  (unwind-protect
      (progn
        (ample-theme)
        (let ((after (ample-test-face-report probes)))
          (list standard
                before
                after
                (equal before after))))
    (ample-test-disable-all)))"##;
    let expect = expect![[
        r#"OK (((font-lock-warning-face (t :inherit error)) (font-lock-comment-face (((class grayscale) (background light)) :foreground "DimGray" :weight bold :slant italic) (((class grayscale) (background dark)) :foreground "LightGray" :weight bold :slant italic) (((class color) (min-colors 88) (background light)) :foreground "Firebrick") (((class color) (min-colors 88) (background dark)) :foreground "chocolate1") (((class color) (min-colors 16) (background light)) :foreground "red") (((class color) (min-colors 16) (background dark)) :foreground "red1") (((class color) (min-colors 8) (background light)) :foreground "red") (((class color) (min-colors 8) (background dark)) :foreground "yellow") (t :weight bold :slant italic)) (font-lock-string-face (((class grayscale) (background light)) :foreground "DimGray" :slant italic) (((class grayscale) (background dark)) :foreground "LightGray" :slant italic) (((class color) (min-colors 88) (background light)) :foreground "VioletRed4") (((class color) (min-colors 88) (background dark)) :foreground "LightSalmon") (((class color) (min-colors 16) (background light)) :foreground "RosyBrown") (((class color) (min-colors 16) (background dark)) :foreground "LightSalmon") (((class color) (min-colors 8)) :foreground "green") (t :slant italic)) (font-lock-keyword-face (((class grayscale) (background light)) :foreground "LightGray" :weight bold) (((class grayscale) (background dark)) :foreground "DimGray" :weight bold) (((class color) (min-colors 88) (background light)) :foreground "Purple") (((class color) (min-colors 88) (background dark)) :foreground "Cyan1") (((class color) (min-colors 16) (background light)) :foreground "Purple") (((class color) (min-colors 16) (background dark)) :foreground "Cyan") (((class color) (min-colors 8)) :foreground "cyan" :weight bold) (t :weight bold)) (font-lock-doc-face (t :inherit font-lock-string-face)) (font-lock-comment-delimiter-face (default :inherit font-lock-comment-face)) (link (((class color) (min-colors 88) (background light)) :foreground "RoyalBlue3" :underline t) (((class color) (background light)) :foreground "blue" :underline t) (((class color) (min-colors 88) (background dark)) :foreground "cyan1" :underline t) (((class color) (background dark)) :foreground "cyan" :underline t) (t :inherit underline)) (button (t :inherit link)) (show-paren-match (((class color) (background light)) :background "turquoise") (((class color) (background dark)) :background "steelblue3") (((background dark) (min-colors 4)) :background "grey50") (((background light) (min-colors 4)) :background "gray") (t :inherit underline)) (header-line (default :inherit mode-line) (((type tty)) :inverse-video nil :underline t) (((class color grayscale) (background light)) :background "grey90" :foreground "grey20" :box nil) (((class color grayscale) (background dark)) :background "grey20" :foreground "grey90" :box nil) (((class mono) (background light)) :background "white" :foreground "black" :inverse-video nil :box nil :underline t) (((class mono) (background dark)) :background "black" :foreground "white" :inverse-video nil :box nil :underline t)) (completions-annotations (t :inherit (italic shadow))) (error (default :weight bold) (((class color) (min-colors 88) (background light)) :foreground "Red1") (((class color) (min-colors 88) (background dark)) :foreground "Pink") (((class color) (min-colors 16) (background light)) :foreground "Red1") (((class color) (min-colors 16) (background dark)) :foreground "Pink") (((class color) (min-colors 8)) :foreground "red") (t :inverse-video t))) ((font-lock-warning-face (:inherit . error) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-face (:inherit . unspecified) (:weight . bold) (:slant . italic) (:underline . unspecified)) (font-lock-string-face (:inherit . unspecified) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (font-lock-keyword-face (:inherit . unspecified) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-doc-face (:inherit . font-lock-string-face) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (font-lock-comment-delimiter-face (:inherit . font-lock-comment-face) (:weight . bold) (:slant . italic) (:underline . unspecified)) (link (:inherit . underline) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (button (:inherit . link) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (show-paren-match (:inherit . underline) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (header-line (:inherit . mode-line) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (completions-annotations (:inherit italic shadow) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (error (:inherit . unspecified) (:weight . bold) (:slant . unspecified) (:underline . unspecified))) ((font-lock-warning-face (:inherit . unspecified) (:weight . bold) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-string-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-keyword-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-doc-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (font-lock-comment-delimiter-face (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (link (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (button (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . t)) (show-paren-match (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (header-line (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified)) (completions-annotations (:inherit . unspecified) (:weight . unspecified) (:slant . italic) (:underline . unspecified)) (error (:inherit . unspecified) (:weight . unspecified) (:slant . unspecified) (:underline . unspecified))) nil)"#
    ]];

    assert_ample_theme_parity(elisp_form, expect);
}

/// The pair the package exists for: a user on the dark variant during the night
/// runs `M-x ample-light-theme' in the morning.  Emacs does not turn the first
/// one off - both end up in `custom-enabled-themes', most recent first, and the
/// later one wins every face they both set.
///
/// Disabling only the light one is the interesting step, because it is what a
/// user does to go back: the dark variant is still enabled underneath and its
/// colours return without being reloaded.  Disabling it too leaves the editor at
/// its baseline, asserted with an `equal'.
#[test]
fn enabling_the_light_variant_over_the_dark_one_layers_rather_than_replaces() {
    let elisp_form = r##"(let ((baseline (ample-test-face-report ample-test-probe-faces)))
  (unwind-protect
      (progn
        (ample-theme)
        (let ((dark (list (copy-sequence custom-enabled-themes)
                          (ample-test-face-report ample-test-probe-faces))))
          (ample-light-theme)
          (let ((light (list (copy-sequence custom-enabled-themes)
                             (and (custom-theme-enabled-p 'ample) t)
                             (ample-test-face-report ample-test-probe-faces))))
            (disable-theme 'ample-light)
            (let ((back (list (copy-sequence custom-enabled-themes)
                              (ample-test-face-report ample-test-probe-faces))))
              (disable-theme 'ample)
              (list dark
                    light
                    back
                    (equal (cadr dark) (cadr back))
                    (copy-sequence custom-enabled-themes)
                    (equal baseline
                           (ample-test-face-report ample-test-probe-faces)))))))
    (ample-test-disable-all)))"##;
    let expect = expect![[
        r##"OK (((ample) ((default (:foreground . "#bdbdb3") (:background . "gray13")) (cursor (:background . "#f57e00")) (region (:background . "#303030")) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . "cornsilk4") (:foreground . "#252525")) (mode-line-inactive (:background . "#454545") (:foreground . "cornsilk4")) (fringe (:background . "#1f1f1f")) (font-lock-keyword-face (:foreground . "#5180b3")) (font-lock-string-face (:foreground . "#bdbc61")) (font-lock-comment-face (:foreground . "#757575")) (font-lock-function-name-face (:foreground . "#6aaf50")) (font-lock-variable-name-face (:foreground . "#baba36")) (font-lock-type-face (:foreground . "#cd5542")) (font-lock-warning-face (:foreground . "red")) (link (:foreground . "#68a5e9")) (isearch (:background . "#5180b3") (:foreground . "gray13")) (minibuffer-prompt (:foreground . "#fffe0a")))) ((ample-light ample) t ((default (:foreground . "gray43") (:background . "#cBc9b1")) (cursor (:background . "#F57E00")) (region (:background . "#BBB9A1")) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . "#BBB9A1") (:foreground . "gray43")) (mode-line-inactive (:background . "#ABA991") (:foreground . "#cBc9b1")) (fringe (:background . "#CBC9B1")) (font-lock-keyword-face (:foreground . "#4170B3")) (font-lock-string-face (:foreground . "#5D5C01")) (font-lock-comment-face (:foreground . "#959595")) (font-lock-function-name-face (:foreground . "#4A8F30")) (font-lock-variable-name-face (:foreground . "#787800")) (font-lock-type-face (:foreground . "#CD5542")) (font-lock-warning-face (:foreground . "red")) (link (:foreground . "#68A5E9")) (isearch (:background . "#4170B3") (:foreground . "#cBc9b1")) (minibuffer-prompt (:foreground . "#9B55C3")))) ((ample) ((default (:foreground . "#bdbdb3") (:background . "gray13")) (cursor (:background . "#f57e00")) (region (:background . "#303030")) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . "cornsilk4") (:foreground . "#252525")) (mode-line-inactive (:background . "#454545") (:foreground . "cornsilk4")) (fringe (:background . "#1f1f1f")) (font-lock-keyword-face (:foreground . "#5180b3")) (font-lock-string-face (:foreground . "#bdbc61")) (font-lock-comment-face (:foreground . "#757575")) (font-lock-function-name-face (:foreground . "#6aaf50")) (font-lock-variable-name-face (:foreground . "#baba36")) (font-lock-type-face (:foreground . "#cd5542")) (font-lock-warning-face (:foreground . "red")) (link (:foreground . "#68a5e9")) (isearch (:background . "#5180b3") (:foreground . "gray13")) (minibuffer-prompt (:foreground . "#fffe0a")))) t nil t)"##
    ]];

    assert_ample_theme_parity(elisp_form, expect);
}
