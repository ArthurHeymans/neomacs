use expect_test::expect;

use super::assert_amaranth_dark_theme_parity;

/// The whole point of a theme, both ways round: the user runs `load-theme' and
/// thirty-three of the faces they look at all day change colour together, then
/// runs `disable-theme' and gets the untouched editor back.
///
/// The report is taken three times from the same face list so the round trip is
/// visible as one value, and the last element asserts that the restored state is
/// `equal' to the baseline rather than merely plausible.  Alongside the faces it
/// pins the theme's one non-face setting - `custom-theme-set-variables' puts
/// `frame-background-mode' to `dark' and disabling puts it back to nil.
///
/// Two spec shapes that resolve to something other than a colour are in the
/// list deliberately.  `fringe', `tab-bar-tab' and `tab-bar-tab-inactive' are
/// written with an explicit `:background nil', which does not paint them the
/// theme's background but drops their stock grey and leaves them
/// `unspecified'.  And `line-number' replaces the stock `(shadow default)'
/// inherit with plain `default', which is what stops the line numbers picking
/// up `shadow' on top of the colour the theme gives them.
#[test]
fn enabling_the_theme_repaints_every_core_face_and_disabling_restores_them() {
    let elisp_form = r##"(let ((baseline (amaranth-test-face-report amaranth-test-core-faces)))
  (load-theme 'amaranth-dark t)
  (let ((enabled (list (copy-sequence custom-enabled-themes)
                       (and (custom-theme-enabled-p 'amaranth-dark) t)
                       frame-background-mode
                       (amaranth-test-face-report amaranth-test-core-faces))))
    (disable-theme 'amaranth-dark)
    (let ((restored (amaranth-test-face-report amaranth-test-core-faces)))
      (list baseline
            enabled
            (copy-sequence custom-enabled-themes)
            frame-background-mode
            restored
            (equal baseline restored)))))"##;
    let expect = expect![[
        r##"OK (((default (:foreground . "unspecified-fg") (:background . "unspecified-bg")) (cursor (:background . "white")) (region (:background . unspecified) (:foreground . unspecified)) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . unspecified) (:foreground . unspecified)) (mode-line-inactive (:background . unspecified) (:foreground . unspecified)) (font-lock-keyword-face (:foreground . unspecified) (:weight . bold)) (font-lock-comment-face (:foreground . unspecified)) (font-lock-string-face (:foreground . unspecified)) (font-lock-function-name-face (:foreground . unspecified)) (font-lock-variable-name-face (:foreground . unspecified)) (font-lock-type-face (:foreground . unspecified)) (font-lock-constant-face (:foreground . unspecified)) (font-lock-doc-face (:foreground . unspecified)) (font-lock-warning-face (:foreground . unspecified)) (link (:foreground . unspecified) (:underline . t)) (link-visited (:foreground . unspecified) (:underline . t)) (line-number (:foreground . "unspecified-fg") (:inherit shadow default)) (line-number-current-line (:foreground . "unspecified-fg") (:inherit . line-number)) (isearch (:foreground . unspecified) (:background . unspecified)) (isearch-fail (:foreground . unspecified) (:background . unspecified)) (fringe (:background . "gray") (:foreground . unspecified)) (shadow (:foreground . unspecified)) (minibuffer-prompt (:foreground . "cyan")) (trailing-whitespace (:foreground . unspecified) (:background . unspecified)) (tooltip (:background . unspecified) (:foreground . unspecified)) (secondary-selection (:background . unspecified) (:foreground . unspecified)) (match (:background . unspecified)) (vertical-border (:foreground . unspecified)) (border (:background . unspecified) (:foreground . unspecified)) (tab-bar (:background . "grey") (:foreground . unspecified)) (tab-bar-tab (:background . "grey") (:foreground . unspecified) (:weight . unspecified)) (tab-bar-tab-inactive (:background . "grey"))) ((amaranth-dark) t dark ((default (:foreground . "#e4e4ef") (:background . "#000000")) (cursor (:background . "#ffd966")) (region (:background . "#4f4949") (:foreground . unspecified)) (highlight (:background . "#101010") (:foreground . unspecified)) (mode-line (:background . "#101010") (:foreground . "#ffffff")) (mode-line-inactive (:background . "#101010") (:foreground . "#959da3")) (font-lock-keyword-face (:foreground . "#ffd966") (:weight . bold)) (font-lock-comment-face (:foreground . "#7b7171")) (font-lock-string-face (:foreground . "#598b43")) (font-lock-function-name-face (:foreground . "#97a1b5")) (font-lock-variable-name-face (:foreground . "#f4f4ff")) (font-lock-type-face (:foreground . "#959da3")) (font-lock-constant-face (:foreground . "#959da3")) (font-lock-doc-face (:foreground . "#598b43")) (font-lock-warning-face (:foreground . "#a02e2e")) (link (:foreground . "#97a1b5") (:underline . t)) (link-visited (:foreground . "#a64d79") (:underline . t)) (line-number (:foreground . "#7b7171") (:inherit . default)) (line-number-current-line (:foreground . "#ffd966") (:inherit . line-number)) (isearch (:foreground . "#000000") (:background . "#f5f5f5")) (isearch-fail (:foreground . "#000000") (:background . "#a02e2e")) (fringe (:background . unspecified) (:foreground . "#302d2d")) (shadow (:foreground . "#7b7171")) (minibuffer-prompt (:foreground . "#97a1b5")) (trailing-whitespace (:foreground . "#000000") (:background . "#a02e2e")) (tooltip (:background . "#7b7171") (:foreground . "#ffffff")) (secondary-selection (:background . "#4f4949") (:foreground . unspecified)) (match (:background . "#7b7171")) (vertical-border (:foreground . "#302d2d")) (border (:background . "#080808") (:foreground . "#302d2d")) (tab-bar (:background . "#101010") (:foreground . "#7b7171")) (tab-bar-tab (:background . unspecified) (:foreground . "#ffd966") (:weight . bold)) (tab-bar-tab-inactive (:background . unspecified)))) nil nil ((default (:foreground . "unspecified-fg") (:background . "unspecified-bg")) (cursor (:background . "white")) (region (:background . unspecified) (:foreground . unspecified)) (highlight (:background . unspecified) (:foreground . unspecified)) (mode-line (:background . unspecified) (:foreground . unspecified)) (mode-line-inactive (:background . unspecified) (:foreground . unspecified)) (font-lock-keyword-face (:foreground . unspecified) (:weight . bold)) (font-lock-comment-face (:foreground . unspecified)) (font-lock-string-face (:foreground . unspecified)) (font-lock-function-name-face (:foreground . unspecified)) (font-lock-variable-name-face (:foreground . unspecified)) (font-lock-type-face (:foreground . unspecified)) (font-lock-constant-face (:foreground . unspecified)) (font-lock-doc-face (:foreground . unspecified)) (font-lock-warning-face (:foreground . unspecified)) (link (:foreground . unspecified) (:underline . t)) (link-visited (:foreground . unspecified) (:underline . t)) (line-number (:foreground . "unspecified-fg") (:inherit shadow default)) (line-number-current-line (:foreground . "unspecified-fg") (:inherit . line-number)) (isearch (:foreground . unspecified) (:background . unspecified)) (isearch-fail (:foreground . unspecified) (:background . unspecified)) (fringe (:background . "gray") (:foreground . unspecified)) (shadow (:foreground . unspecified)) (minibuffer-prompt (:foreground . "cyan")) (trailing-whitespace (:foreground . unspecified) (:background . unspecified)) (tooltip (:background . unspecified) (:foreground . unspecified)) (secondary-selection (:background . unspecified) (:foreground . unspecified)) (match (:background . unspecified)) (vertical-border (:foreground . unspecified)) (border (:background . unspecified) (:foreground . unspecified)) (tab-bar (:background . "grey") (:foreground . unspecified)) (tab-bar-tab (:background . "grey") (:foreground . unspecified) (:weight . unspecified)) (tab-bar-tab-inactive (:background . "grey"))) t)"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

/// What the theme is for: reading code in it.  Two real buffers are typed out
/// and font-locked for real, and every token is reported with the face font lock
/// gave it and the colour and weight that face resolves to under the theme.
///
/// Elisp and C are both here because they reach different halves of the
/// palette - C alone produces `font-lock-preprocessor-face' and
/// `font-lock-type-face', and it shows that an include's `<stdio.h>' is a
/// string to font lock and so comes out in the theme's green.  The buffer text
/// is asserted with the faces so a fixture that failed to say what this comment
/// says would be visible as text rather than hidden behind a face name.
#[test]
fn a_font_locked_elisp_and_c_buffer_are_painted_in_the_amaranth_palette() {
    let elisp_form = r##"(amaranth-test-with-theme
  (list
   (with-temp-buffer
     (emacs-lisp-mode)
     (insert ";; Amaranth demo\n"
             "(defun amaranth-demo (path)\n"
             "  \"Read PATH; return its contents.\"\n"
             "  (let ((limit 10))\n"
             "    (message \"read %s\" path)\n"
             "    (car limit)))\n")
     (font-lock-ensure)
     (list (amaranth-test-token-faces
            '(";; Amaranth demo" "defun" "amaranth-demo"
              "\"Read PATH; return its contents.\"" "let"
              "\"read %s\"" "car"))
           (buffer-substring-no-properties (point-min) (point-max))))
   (with-temp-buffer
     (c-mode)
     (insert "/* amaranth */\n"
             "#include <stdio.h>\n"
             "int main(void) {\n"
             "  const char *s = \"hi\";\n"
             "  return 0;\n"
             "}\n")
     (font-lock-ensure)
     (list (amaranth-test-token-faces
            '("/* amaranth */" "#include" "stdio.h" "int" "main"
              "const" "\"hi\"" "return"))
           (buffer-substring-no-properties (point-min) (point-max))))))"##;
    let expect = expect![[
        r##"OK ((((";; Amaranth demo" font-lock-comment-delimiter-face "#7b7171" unspecified) ("defun" font-lock-keyword-face "#ffd966" bold) ("amaranth-demo" font-lock-function-name-face "#97a1b5" unspecified) ("\"Read PATH; return its contents.\"" font-lock-doc-face "#598b43" unspecified) ("let" font-lock-keyword-face "#ffd966" bold) ("\"read %s\"" font-lock-string-face "#598b43" unspecified) ("car" nil nil nil)) ";; Amaranth demo\n(defun amaranth-demo (path)\n  \"Read PATH; return its contents.\"\n  (let ((limit 10))\n    (message \"read %s\" path)\n    (car limit)))\n") ((("/* amaranth */" font-lock-comment-delimiter-face "#7b7171" unspecified) ("#include" font-lock-preprocessor-face "#959da3" unspecified) ("stdio.h" font-lock-string-face "#598b43" unspecified) ("int" font-lock-type-face "#959da3" unspecified) ("main" font-lock-function-name-face "#97a1b5" unspecified) ("const" font-lock-keyword-face "#ffd966" bold) ("\"hi\"" font-lock-string-face "#598b43" unspecified) ("return" font-lock-keyword-face "#ffd966" bold)) "/* amaranth */\n#include <stdio.h>\nint main(void) {\n  const char *s = \"hi\";\n  return 0;\n}\n"))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

/// The order a user actually does this in: the theme goes in the init file and
/// the packages it styles are loaded later, on demand.  None of these eleven
/// faces exists when the theme is enabled - `facep' is nil for every one of
/// them - and each still comes out in the theme's colours once its library is
/// required, because `custom-declare-face' consults the enabled themes as the
/// face is created.
///
/// The four spellings the theme uses for weight and inheritance are all in this
/// group, and they resolve differently from each other: `ido-only-match' says
/// `:weight bold', `ido-first-match' says the obsolete `:bold nil' and comes
/// out `normal', `dired-directory' says `:weight bold' and comes out bold, and
/// `dired-ignored' explicitly clears the stock inherit with
/// `:inherit unspecified'.
#[test]
fn libraries_loaded_after_the_theme_still_receive_its_colours() {
    let elisp_form = r##"(let ((before (amaranth-test-face-presence
                '(whitespace-space whitespace-trailing whitespace-line
                  whitespace-empty term-color-red term-color-white
                  ido-first-match ido-only-match ido-subdir
                  dired-directory dired-ignored))))
  (amaranth-test-with-theme
    (require 'whitespace)
    (require 'term)
    (require 'ido)
    (require 'dired)
    (list before
          (amaranth-test-face-report
           '((whitespace-space :background :foreground)
             (whitespace-trailing :background :foreground)
             (whitespace-line :background :foreground)
             (whitespace-empty :background :foreground)
             (term-color-red :foreground :background)
             (term-color-white :foreground :background)
             (ido-first-match :foreground :weight)
             (ido-only-match :foreground :weight)
             (ido-subdir :foreground :weight)
             (dired-directory :foreground :weight)
             (dired-ignored :foreground :inherit))))))"##;
    let expect = expect![[
        r##"OK (((whitespace-space nil nil) (whitespace-trailing nil nil) (whitespace-line nil nil) (whitespace-empty nil nil) (term-color-red nil nil) (term-color-white nil nil) (ido-first-match nil nil) (ido-only-match nil nil) (ido-subdir nil nil) (dired-directory nil nil) (dired-ignored nil nil)) ((whitespace-space (:background . "#000000") (:foreground . "#101010")) (whitespace-trailing (:background . "#a02e2e") (:foreground . "#a02e2e")) (whitespace-line (:background . "#302d2d") (:foreground . "#c81a1a")) (whitespace-empty (:background . "#ffd966") (:foreground . "#ffd966")) (term-color-red (:foreground . "#c73c3f") (:background . "#c73c3f")) (term-color-white (:foreground . "#e4e4ef") (:background . "#ffffff")) (ido-first-match (:foreground . "#ffd966") (:weight . normal)) (ido-only-match (:foreground . "#7b7171") (:weight . bold)) (ido-subdir (:foreground . "#97a1b5") (:weight . bold)) (dired-directory (:foreground . "#97a1b5") (:weight . bold)) (dired-ignored (:foreground . "#959da3") (:inherit . unspecified))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

/// The one spec shape in this theme whose branch depends on the display.  The
/// five flymake and flyspell faces are written as two clauses - a wavy coloured
/// underline where `(supports :underline (:style wave))' holds, and a plain
/// bold underline everywhere else - so what a user sees depends on the terminal
/// they are in.
///
/// A batch frame is a zero-colour, non-graphic `static-gray' display that does
/// not support a wave underline, so the second clause is the one that applies.
/// The workflow records those display facts, the registered two-clause spec,
/// what `face-spec-choose' picks out of it, and the attributes flyspell's faces
/// end up with, so the reason for the chosen branch is on the record next to its
/// effect rather than being inferred from the colours alone.
#[test]
fn the_wave_underline_specs_fall_back_to_a_plain_underline_on_this_display() {
    let elisp_form = r##"(amaranth-test-with-theme
  (require 'flyspell)
  (list (list :graphic (display-graphic-p)
              :color-cells (display-color-cells)
              :visual-class (display-visual-class)
              :supports-wave (display-supports-face-attributes-p
                              '(:underline (:style wave)))
              :matches-wave-clause (face-spec-set-match-display
                                    '((supports :underline (:style wave))) nil)
              :matches-default-clause (face-spec-set-match-display t nil))
        (amaranth-test-theme-spec 'flyspell-incorrect)
        (amaranth-test-copy-tree
         (face-spec-choose (amaranth-test-theme-spec 'flyspell-incorrect)))
        (amaranth-test-face-report
         '((flyspell-incorrect :underline :foreground :weight :inherit)
           (flyspell-duplicate :underline :foreground :weight :inherit)))))"##;
    let expect = expect![[
        r##"OK ((:graphic nil :color-cells 0 :visual-class static-gray :supports-wave nil :matches-wave-clause nil :matches-default-clause t) ((((supports :underline (:style wave))) (:underline (:style wave :color "#a02e2e") :inherit unspecified)) (t (:foreground "#a02e2e" :weight bold :underline t))) (:foreground "#a02e2e" :weight bold :underline t) ((flyspell-incorrect (:underline . t) (:foreground . "#a02e2e") (:weight . bold) (:inherit . unspecified)) (flyspell-duplicate (:underline . t) (:foreground . "#ffd966") (:weight . bold) (:inherit . unspecified))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

/// Where the theme silently does nothing.  Several of its specs name faces that
/// Emacs no longer has, so the surfaces they were written for keep their stock
/// appearance on the theme's near-black background: `show-paren-match-face',
/// `isearch-lazy-highlight-face' and `holiday-face' are not faces at all, and
/// `show-paren-match', `lazy-highlight' and `holiday' are left unspecified.
///
/// `flymake-errline' shows that an obsolete alias does not rescue the spec
/// either.  By the time flymake defines the alias the theme has already
/// recorded a setting under the old name, and the setting does not follow the
/// alias, so `flymake-error' stays unthemed.
///
/// Two cases do not fail, and they are here for the contrast.  `hl-line' is
/// themed despite `highlight-current-line-face' not existing, because the stock
/// face inherits from `highlight', which the theme does set.  And
/// `completions-annotations' fails for a different reason - a stray quote makes
/// its spec `:inherit 'shadow', so the attribute resolves to the two-element
/// list `(quote shadow)' rather than to the themed `shadow' face, and the
/// annotation keeps no colour at all.
///
/// Both editors leave every one of those surfaces unthemed, and agree on the
/// colours, the specs and the aliasing.  The single thing they do not agree on
/// is whether `flymake-errline' is a face at all once flymake has aliased it;
/// see DIVERGENCES entry 10, face aliases are not followed.
#[test]
fn specs_naming_removed_faces_leave_those_surfaces_unthemed() {
    let elisp_form = r##"(amaranth-test-with-theme
  (require 'paren)
  (require 'holidays)
  (require 'hl-line)
  (require 'flymake)
  (list (amaranth-test-face-presence
         '(show-paren-match-face show-paren-match
           isearch-lazy-highlight-face lazy-highlight
           holiday-face holiday
           highlight-current-line-face hl-line
           flymake-errline flymake-error
           flymake-infoline flymake-note))
        (amaranth-test-face-report
         '((show-paren-match :background :foreground)
           (lazy-highlight :background :foreground)
           (holiday :background :foreground)
           (hl-line :background :inherit)
           (highlight :background)
           (flymake-error :underline :foreground :weight)
           (flymake-note :underline :foreground :weight)
           (completions-annotations :inherit :foreground)
           (shadow :foreground)))
        (amaranth-test-theme-spec 'show-paren-match-face)
        (amaranth-test-theme-spec 'completions-annotations)))"##;
    let expect = expect![[
        r##"OK (((show-paren-match-face nil nil) (show-paren-match t nil) (isearch-lazy-highlight-face nil nil) (lazy-highlight t nil) (holiday-face nil nil) (holiday t nil) (highlight-current-line-face nil nil) (hl-line t nil) (flymake-errline t flymake-error) (flymake-error t nil) (flymake-infoline nil nil) (flymake-note t nil)) ((show-paren-match (:background . unspecified) (:foreground . unspecified)) (lazy-highlight (:background . unspecified) (:foreground . unspecified)) (holiday (:background . unspecified) (:foreground . unspecified)) (hl-line (:background . "#101010") (:inherit . highlight)) (highlight (:background . "#101010")) (flymake-error (:underline . unspecified) (:foreground . unspecified) (:weight . bold)) (flymake-note (:underline . unspecified) (:foreground . unspecified) (:weight . bold)) (completions-annotations (:inherit quote shadow) (:foreground . unspecified)) (shadow (:foreground . "#7b7171"))) ((t (:background "#7b7171"))) ((t (:inherit 'shadow))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}
