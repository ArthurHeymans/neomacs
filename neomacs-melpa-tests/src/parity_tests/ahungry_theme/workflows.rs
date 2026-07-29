use expect_test::expect;

use super::assert_ahungry_theme_parity;

/// The install route the package's own summary line documents -- "Make sure to
/// (load-theme 'ahungry)" -- rather than the `enable-theme' the other two files
/// use on an already-registered theme.
///
/// For `load-theme' to find the file at all, the `;;;###autoload' form at the
/// bottom of the theme has to have put the package's own directory on
/// `custom-theme-load-path', so that is asserted first.  Loading then enables
/// the theme and registers 216 settings: 215 face specs across 214 distinct
/// faces -- the two counts differ because `link' is specified twice, which the
/// duplicate-spec workflow takes up -- plus one variable setting.
///
/// The theme also ends with a `custom-theme-set-variables' block declaring a
/// global variable named `red'.  Enabling the theme does not bind it: `red' is
/// not a defcustom, so there is nothing for the theme to set, and the block is
/// inert.  `boundp' is reported separately from the value so that "unbound"
/// and "bound to nil" cannot be confused -- without that the assertion would
/// read the same either way.
#[test]
fn the_documented_load_theme_route_registers_the_faces_and_a_global_red_variable() {
    let elisp_form = r##"(let* ((directory (file-name-directory (getenv "NEOMACS_PACKAGE_SOURCE")))
       (observed nil))
  (ahungry-test-with-theme-off
   (lambda ()
     (push (list :before
                 (list :on-load-path
                       (and (member (file-name-as-directory directory)
                                    custom-theme-load-path)
                            t)
                       :enabled (and (memq 'ahungry custom-enabled-themes) t)
                       :red-bound (boundp 'red)))
           observed)
     (load-theme 'ahungry t)
     (push (list :after-load-theme
                 (list :enabled (and (memq 'ahungry custom-enabled-themes) t)
                       :is-a-theme (and (custom-theme-p 'ahungry) t)
                       :faces-set (length (ahungry-test-theme-faces))
                       :spec-count (length (get 'ahungry 'theme-settings))
                       :red-bound (boundp 'red)
                       :red (and (boundp 'red) (symbol-value 'red))
                       :default-foreground
                       (face-attribute 'default :foreground nil 'default)))
           observed)
     (disable-theme 'ahungry)
     (push (list :after-disable
                 (list :enabled (and (memq 'ahungry custom-enabled-themes) t)
                       :still-a-theme (and (custom-theme-p 'ahungry) t)
                       :red-bound (boundp 'red)
                       :red (and (boundp 'red) (symbol-value 'red))))
           observed)))
  (nreverse observed))"##;

    let expect = expect![[
        r##"OK ((:before (:on-load-path t :enabled nil :red-bound nil)) (:after-load-theme (:enabled t :is-a-theme t :faces-set 214 :spec-count 216 :red-bound nil :red nil :default-foreground "#ffffff")) (:after-disable (:enabled nil :still-a-theme t :red-bound nil :red nil)))"##
    ]];

    assert_ahungry_theme_parity(elisp_form, expect);
}

/// The Commentary's documented split: "If you load it from a terminal, you will
/// be able to make use of the transparent background.  If you load it from a
/// GUI, it will default to a dark background."
///
/// That is not a display clause -- every one of the theme's specs is `((t ...))'
/// -- it is `(let ((mainbg (when (display-graphic-p) "#101010"))))' evaluated
/// once, at load time, and spliced into exactly one face.
///
/// So this workflow pins the gate's *answer* and the whole of the behaviour on
/// the side this editor really is, and does not manufacture the other side.
/// Loading the theme again with `display-graphic-p' stubbed to t was tried and
/// is not merely discouraged but refused: the editor signals
/// `(error "Window system frame should be used")', because building that spec
/// wants a frame that does not exist here.  Asserting anything through such a
/// stub would be measuring an invented display, and an assertion built on
/// lying to the editor would be free to diverge between the two editors for
/// reasons that say nothing about this theme.
///
/// The graphical value is `"#101010"' from the `let' at ahungry-theme.el:121;
/// it is stated here rather than asserted.  What the workflow does establish is
/// that the terminal branch is real: `default' is the only face whose
/// background comes from the gate, and it is genuinely unset rather than set to
/// something dark.
///
/// Five faces end up with no background, and the difference between them is
/// the point of listing all five: `default' gets its nil from `mainbg', while
/// `bold', `italic', `erc-prompt-face' and `erc-timestamp-face' have a literal
/// `:background nil' written into the theme and would be backgroundless on a
/// GUI too.  A count alone would run the two causes together.
#[test]
fn the_transparent_background_is_a_load_time_display_graphic_p_gate_on_one_face() {
    let elisp_form = r##"(let ((observed nil))
  (push (list :gate (list :display-graphic-p (display-graphic-p)
                          :mainbg-would-be (when (display-graphic-p) "#101010")))
        observed)
  (push (list :terminal-branch
              (list :stored-default-spec (ahungry-test-stored-spec 'default)
                    :faces-with-no-background
                    (seq-filter
                     (lambda (face)
                       (let ((spec (ahungry-test-stored-spec face)))
                         (and (plist-member (car (cdar spec)) :background)
                              (null (plist-get (car (cdar spec)) :background)))))
                     (ahungry-test-theme-faces))))
        observed)
  (enable-theme 'ahungry)
  (push (list :terminal-branch-resolved
              (list :default-background
                    (face-attribute 'default :background nil 'default)
                    :default-foreground
                    (face-attribute 'default :foreground nil 'default)))
        observed)
  (disable-theme 'ahungry)
  (nreverse observed))"##;

    let expect = expect![[
        r##"OK ((:gate (:display-graphic-p nil :mainbg-would-be nil)) (:terminal-branch (:stored-default-spec ((t (:foreground "#ffffff" :background nil :family "Terminus" :foundry "xos4" :slant normal :weight normal :height 130 :width normal))) :faces-with-no-background (erc-timestamp-face erc-prompt-face italic bold default))) (:terminal-branch-resolved (:default-background unspecified :default-foreground "#ffffff")))"##
    ]];

    assert_ahungry_theme_parity(elisp_form, expect);
}

/// `ahungry-theme-font-settings' is the theme's one documented customization
/// point: "If set to nil, will avoid overriding the user font settings."
///
/// Two things a user needs to know come out of exercising it.  It is spliced
/// into the `default' face with `,@' inside the same load-time `let', so
/// setting it has no effect at all unless the theme file is loaded again
/// afterwards -- changing it on a running Emacs and re-enabling the theme keeps
/// the old font.  And its actual value disagrees with its own docstring, which
/// documents the default as `:height 100' while the code ships `:height 130'.
///
/// Both branches are asserted through the resolved `default' face, so this is
/// the font the user really gets in each case.
///
/// THIS WORKFLOW IS RED ON NEOMACS ON PURPOSE.  It is the one that surfaced
/// DIVERGENCES.md entry 35: because the theme sets `:family' beside
/// `:foreground' on `default', GNU discards the family and reports `"default"'
/// while Neomacs reports `"Terminus"'.  GNU's answer is the expectation, per
/// the standards.  The suite's colour-focused workflows read a narrowed
/// attribute list (`ahungry-test-colour') so that this one test carries the
/// divergence and the failure count keeps meaning something.
#[test]
fn setting_the_font_settings_variable_only_takes_effect_when_the_theme_is_reloaded() {
    let elisp_form = r##"(let ((source (getenv "NEOMACS_PACKAGE_SOURCE"))
      (original ahungry-theme-font-settings)
      (observed nil))
  (unwind-protect
      (progn
        (push (list :shipped-default
                    (list :value (copy-tree ahungry-theme-font-settings)
                          :docstring-claims-height
                          (and (string-match ":height \\([0-9]+\\)"
                                             (documentation-property
                                              'ahungry-theme-font-settings
                                              'variable-documentation))
                               (match-string 1
                                             (documentation-property
                                              'ahungry-theme-font-settings
                                              'variable-documentation)))))
              observed)
        (enable-theme 'ahungry)
        (push (list :with-the-shipped-font
                    (list :family (face-attribute 'default :family nil 'default)
                          :foundry (face-attribute 'default :foundry nil 'default)
                          :height (face-attribute 'default :height nil 'default)))
              observed)
        ;; Setting the variable and re-enabling is what a user would try first.
        (setq ahungry-theme-font-settings nil)
        (disable-theme 'ahungry)
        (enable-theme 'ahungry)
        (push (list :set-to-nil-and-re-enabled
                    (list :family (face-attribute 'default :family nil 'default)
                          :height (face-attribute 'default :height nil 'default)))
              observed)
        ;; Only reloading the file re-evaluates the splice.
        (load source nil t t)
        (enable-theme 'ahungry)
        (push (list :set-to-nil-and-reloaded
                    (list :stored-default-spec
                          (ahungry-test-stored-spec 'default)
                          :family (face-attribute 'default :family nil 'default)
                          :height (face-attribute 'default :height nil 'default)))
              observed))
    (setq ahungry-theme-font-settings original)
    (load source nil t t)
    (when (memq 'ahungry custom-enabled-themes) (disable-theme 'ahungry)))
  (nreverse observed))"##;

    let expect = expect![[
        r##"OK ((:shipped-default (:value (:family "Terminus" :foundry "xos4" :slant normal :weight normal :height 130 :width normal) :docstring-claims-height "100")) (:with-the-shipped-font (:family "default" :foundry "default" :height 130)) (:set-to-nil-and-re-enabled (:family "default" :height 130)) (:set-to-nil-and-reloaded (:stored-default-spec ((t (:foreground "#ffffff" :background nil))) :family "default" :height 1)))"##
    ]];

    assert_ahungry_theme_parity(elisp_form, expect);
}

/// The theme specifies `link' twice -- `:underline t :foreground "#33ff99"' at
/// line 143 and `:foreground "#af0"' at line 342, the second added in a later
/// release alongside `hackernews-link'.  Only one of them can win, and the
/// answer decides both the colour of every link the user sees and whether links
/// are underlined at all.
///
/// The workflow pins which spec is in force, that both were registered, and the
/// consequence for a real `link' next to `hackernews-link' -- which got the new
/// colour and so ends up *not* matching the `link' face it was added to match.
#[test]
fn the_duplicate_link_spec_leaves_the_later_colour_dead_and_hackernews_link_mismatched() {
    let elisp_form = r##"(let ((observed nil))
  ;; The theme styles `hackernews-link' for users who have hackernews.el; it is
  ;; not installed here, so stand in for it the way `rendering.rs' does for the
  ;; helm faces.  Without this the face does not exist and `face-attribute'
  ;; signals rather than reporting the theme's colour.
  (unless (facep 'hackernews-link) (make-face 'hackernews-link))
  (push (list :registered
              (list :link-spec-count (ahungry-test-face-spec-count 'link)
                    :every-link-spec (ahungry-test-all-stored-specs 'link)
                    :hackernews-link-spec
                    (ahungry-test-stored-spec 'hackernews-link)))
        observed)
  (enable-theme 'ahungry)
  (push (list :in-force
              (list :link (ahungry-test-resolved 'link ahungry-test-colour)
                    :hackernews-link
                    (ahungry-test-resolved 'hackernews-link ahungry-test-colour)
                    :they-match
                    (equal (ahungry-test-resolved 'link ahungry-test-colour)
                           (ahungry-test-resolved 'hackernews-link
                                                  ahungry-test-colour))))
        observed)
  (with-temp-buffer
    (set-window-buffer (selected-window) (current-buffer))
    (insert (propertize "documentation" 'face 'link) "\n"
            (propertize "front page" 'face 'hackernews-link) "\n")
    (goto-char (point-min))
    (search-forward "documentation")
    (let ((link-position (- (point) (length "documentation"))))
      (search-forward "front page")
      (push (list :rendered
                  (list :link-foreground
                        (face-attribute
                         (get-text-property link-position 'face)
                         :foreground nil 'default)
                        :hackernews-foreground
                        (face-attribute
                         (get-text-property (- (point) (length "front page"))
                                            'face)
                         :foreground nil 'default)))
            observed)))
  (disable-theme 'ahungry)
  (nreverse observed))"##;

    let expect = expect![[
        r##"OK ((:registered (:link-spec-count 2 :every-link-spec (((t (:foreground "#af0"))) ((t (:underline t :foreground "#33ff99")))) :hackernews-link-spec ((t (:foreground "#af0"))))) (:in-force (:link ((:foreground . "#33ff99") (:weight . normal) (:slant . normal) (:underline . t)) :hackernews-link ((:foreground . "#af0") (:weight . normal) (:slant . normal)) :they-match nil)) (:rendered (:link-foreground "#33ff99" :hackernews-foreground "#af0")))"##
    ]];

    assert_ahungry_theme_parity(elisp_form, expect);
}

/// What enabling the theme costs the faces it does not fully restate.
///
/// `face-spec-recalc' applies a defface spec only when no enabled theme has
/// one, so a theme's spec REPLACES the standard definition rather than merging
/// with it: every attribute the theme omits is dropped for as long as the theme
/// is enabled.  With 215 specs, most of which set only a foreground, that is a
/// real effect and not a hypothetical one.
///
/// The workflow measures it rather than sampling: how many of the theme's faces
/// already exist at bare startup, and of those, exactly which lose which
/// attributes.  `face-default-spec' is reported beside each loss, because an
/// attribute on an unconditional clause was in force for every user, while one
/// on a colour-conditional clause's fallback was only in force on a display
/// like this one -- without that, a green test reads as far more alarming than
/// the truth.  Restoration on disable is asserted too, so a loss is known to be
/// temporary.
#[test]
fn enabling_the_theme_drops_stock_attributes_from_faces_it_does_not_restate() {
    let elisp_form = r##"(let ((observed nil))
  (ahungry-test-with-theme-off
   (lambda ()
     (let* ((themed (ahungry-test-theme-faces))
            (existing (seq-filter #'facep themed))
            (before (ahungry-test-capture existing))
            (after nil)
            (restored nil)
            (losses nil))
       (enable-theme 'ahungry)
       (setq after (ahungry-test-capture existing))
       (setq losses (ahungry-test-losses before after))
       (disable-theme 'ahungry)
       (setq restored (ahungry-test-capture existing))
       (push (list :sizes
                   (list :faces-the-theme-sets (length themed)
                         :already-existing (length existing)
                         :losing-at-least-one-attribute (length losses)))
             observed)
       (push (list :losses losses) observed)
       (push (list :restored-on-disable (equal before restored)) observed))))
  (nreverse observed))"##;

    let expect = expect![[
        r#"OK ((:sizes (:faces-the-theme-sets 214 :already-existing 28 :losing-at-least-one-attribute 14)) (:losses ((link (:inherit) ((((class color) (min-colors 88) (background light)) :foreground "RoyalBlue3" :underline t) (((class color) (background light)) :foreground "blue" :underline t) (((class color) (min-colors 88) (background dark)) :foreground "cyan1" :underline t) (((class color) (background dark)) :foreground "cyan" :underline t) (t :inherit underline))) (button (:inherit) ((t :inherit link))) (isearch (:inverse-video) ((((class color) (min-colors 88) (background light)) (:background "magenta3" :foreground "lightskyblue1")) (((class color) (min-colors 88) (background dark)) (:background "palevioletred2" :foreground "brown4")) (((class color) (min-colors 16)) (:background "magenta4" :foreground "cyan1")) (((class color) (min-colors 8)) (:background "magenta4" :foreground "cyan1")) (t (:inverse-video t)))) (font-lock-function-name-face (:inverse-video) ((((class color) (min-colors 88) (background light)) :foreground "Blue1") (((class color) (min-colors 88) (background dark)) :foreground "LightSkyBlue") (((class color) (min-colors 16) (background light)) :foreground "Blue") (((class color) (min-colors 16) (background dark)) :foreground "LightSkyBlue") (((class color) (min-colors 8)) :foreground "blue" :weight bold) (t :inverse-video t :weight bold))) (font-lock-warning-face (:inverse-video :inherit) ((t :inherit error))) (font-lock-type-face (:underline) ((((class grayscale) (background light)) :foreground "Gray90" :weight bold) (((class grayscale) (background dark)) :foreground "DimGray" :weight bold) (((class color) (min-colors 88) (background light)) :foreground "ForestGreen") (((class color) (min-colors 88) (background dark)) :foreground "PaleGreen") (((class color) (min-colors 16) (background light)) :foreground "ForestGreen") (((class color) (min-colors 16) (background dark)) :foreground "PaleGreen") (((class color) (min-colors 8)) :foreground "green") (t :weight bold :underline t))) (font-lock-doc-face (:inherit) ((t :inherit font-lock-string-face))) (font-lock-constant-face (:underline) ((((class grayscale) (background light)) :foreground "LightGray" :weight bold :underline t) (((class grayscale) (background dark)) :foreground "Gray50" :weight bold :underline t) (((class color) (min-colors 88) (background light)) :foreground "dark cyan") (((class color) (min-colors 88) (background dark)) :foreground "Aquamarine") (((class color) (min-colors 16) (background light)) :foreground "CadetBlue") (((class color) (min-colors 16) (background dark)) :foreground "Aquamarine") (((class color) (min-colors 8)) :foreground "magenta") (t :weight bold :underline t))) (match (:inverse-video) ((((class color) (min-colors 88) (background light)) :background "khaki1") (((class color) (min-colors 88) (background dark)) :background "RoyalBlue3") (((class color) (min-colors 8) (background light)) :background "yellow" :foreground "black") (((class color) (min-colors 8) (background dark)) :background "blue" :foreground "white") (((type tty) (class mono)) :inverse-video t) (t :background "gray"))) (region (:inverse-video) ((((class color) (min-colors 88) (background dark)) :background "blue3" :extend t) (((class color) (min-colors 88) (background light)) :background "lightgoldenrod2" :extend t) (((class color) (min-colors 16) (background dark)) :background "blue3" :extend t) (((class color) (min-colors 16) (background light)) :background "lightgoldenrod2" :extend t) (((class color) (min-colors 8)) :background "blue" :foreground "white" :extend t) (((type tty) (class mono)) :inverse-video t) (t :background "gray" :extend t))) (mode-line-inactive (:inverse-video :inherit) ((default :inherit mode-line) (((class color grayscale) (min-colors 88) (background light)) :weight light :box (:line-width -1 :color "grey75" :style nil) :foreground "grey20" :background "grey90") (((class color grayscale) (min-colors 88) (background dark)) :weight light :box (:line-width -1 :color "grey40" :style nil) :foreground "grey80" :background "grey30"))) (mode-line (:inverse-video) ((((class color grayscale) (min-colors 88) (background light)) :box (:line-width -1 :style released-button) :background "grey75" :foreground "black") (((class color grayscale) (min-colors 88) (background dark)) :box (:line-width -1 :style released-button) :background "grey20" :foreground "white") (t :inverse-video t))) (error (:inverse-video) ((default :weight bold) (((class color) (min-colors 88) (background light)) :foreground "Red1") (((class color) (min-colors 88) (background dark)) :foreground "Pink") (((class color) (min-colors 16) (background light)) :foreground "Red1") (((class color) (min-colors 16) (background dark)) :foreground "Pink") (((class color) (min-colors 8)) :foreground "red") (t :inverse-video t))) (highlight (:inverse-video) ((((class color) (min-colors 88) (background light)) :background "darkseagreen2") (((class color) (min-colors 88) (background dark)) :background "darkolivegreen") (((class color) (min-colors 16) (background light)) :background "darkseagreen2") (((class color) (min-colors 16) (background dark)) :background "darkolivegreen") (((class color) (min-colors 8)) :background "green" :foreground "black") (t :inverse-video t))))) (:restored-on-disable t))"#
    ]];

    assert_ahungry_theme_parity(elisp_form, expect);
}
