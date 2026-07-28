use expect_test::expect;

use super::assert_all_the_icons_parity;

/// The thing every caller does: ask for the icon of a file it is about to show
/// in a sidebar or a mode line.  Each answer is one glyph from a bundled font,
/// carrying the family that can render it, the scaled height, the colour face
/// and the raise adjustment.  The list covers the three routes into the
/// mapping -- a regexp match on the whole name (Makefile, Dockerfile), an
/// extension match (el, md, json), an extension match that had to be downcased
/// (PNG) -- and one name that matches nothing, which must come back as the
/// default file icon rather than as nil.
#[test]
fn every_file_gets_the_glyph_its_alist_entry_names() {
    let elisp_form = r##"(mapcar (lambda (file)
          (cons file (all-the-icons-test-describe (all-the-icons-icon-for-file file))))
        '("init.el" "README.md" "config.json" "photo.PNG"
          "Makefile" "Dockerfile" "mystery.zzz"))"##;
    let expect = expect![[
        r#"OK (("init.el" :codepoint 59686 :length 1 :face #1=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) :font-lock-face #1# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("README.md" :codepoint 61447 :length 1 :face #2=(:family "github-octicons" :height 1.2 :inherit all-the-icons-lcyan) :font-lock-face #2# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("config.json" :codepoint 61564 :length 1 :face #3=(:family "github-octicons" :height 1.2 :inherit all-the-icons-yellow) :font-lock-face #3# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("photo.PNG" :codepoint 61458 :length 1 :face #4=(:family "github-octicons" :height 1.2 :inherit all-the-icons-orange) :font-lock-face #4# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("Makefile" :codepoint 59001 :length 1 :face #5=(:family "file-icons" :height 1.2 :inherit all-the-icons-dorange) :font-lock-face #5# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("Dockerfile" :codepoint 61702 :length 1 :face #6=(:family "file-icons" :height 1.2 :inherit all-the-icons-blue) :font-lock-face #6# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("mystery.zzz" :codepoint 61462 :length 1 :face #7=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) :font-lock-face #7# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)))"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}

/// The other two entry points a caller reaches for.  A mode is looked up
/// directly and then through its `derived-mode-parent', and a mode nothing
/// knows about comes back as the mode symbol itself rather than as a string --
/// a caller inserting the result gets the mode name, which is the package's
/// documented fallback.  A URL is matched against the URL alist, with a globe
/// for anything unrecognised.
#[test]
fn mode_and_url_lookups_fall_back_when_the_alists_have_no_entry() {
    let elisp_form = r##"(list
 :modes (mapcar (lambda (mode)
                  (cons mode (all-the-icons-test-describe (all-the-icons-icon-for-mode mode))))
                '(emacs-lisp-mode dired-mode text-mode fundamental-mode no-such-mode))
 :urls (mapcar (lambda (url)
                 (cons url (all-the-icons-test-describe (all-the-icons-icon-for-url url))))
               '("https://github.com/domtronn/all-the-icons.el"
                 "https://youtube.com/watch"
                 "https://example.com/"))
 :families (list (all-the-icons-icon-family-for-file "init.el")
                 (all-the-icons-icon-family-for-mode 'dired-mode)
                 (all-the-icons-icon-family-for-mode 'no-such-mode)))"##;
    let expect = expect![[
        r#"OK (:modes ((emacs-lisp-mode :codepoint 59686 :length 1 :face #1=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) :font-lock-face #1# :display (raise -0.12) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) (dired-mode :codepoint 61462 :length 1 :face #2=(:family "github-octicons" :height 1.2) :font-lock-face #2# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) (text-mode :codepoint 61457 :length 1 :face #3=(:family "github-octicons" :height 1.2 :inherit all-the-icons-cyan) :font-lock-face #3# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) (fundamental-mode :codepoint 59686 :length 1 :face #4=(:family "file-icons" :height 1.2 :inherit all-the-icons-dsilver) :font-lock-face #4# :display (raise -0.12) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) (no-such-mode :not-a-string no-such-mode)) :urls (("https://github.com/domtronn/all-the-icons.el" :codepoint 61450 :length 1 :face #5=(:family "github-octicons" :height 1.2) :font-lock-face #5# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("https://youtube.com/watch" :codepoint 61799 :length 1 :face #6=(:family "FontAwesome" :height 1.2) :font-lock-face #6# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("https://example.com/" :codepoint 61612 :length 1 :face #7=(:family "FontAwesome" :height 1.2) :font-lock-face #7# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky))) :families ("file-icons" "github-octicons" nil))"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}

/// Directories are not looked up by name alone: the package inspects the
/// directory on disk first, so a checkout gets the repo icon, a symlink gets
/// the symlink icon, and only a plain directory falls through to the name
/// match -- which is why `Downloads' gets its own glyph at the alist's own
/// height while `plain' gets the generic one.
#[test]
fn directory_icons_describe_what_the_directory_actually_is() {
    let elisp_form = r##"(let* ((root (all-the-icons-test-sandbox "dirs"))
       (plain (expand-file-name "plain" root))
       (repo (expand-file-name "repo" root))
       (link (expand-file-name "link" root))
       (downloads (expand-file-name "Downloads" root)))
  (make-directory plain t)
  (make-directory (expand-file-name ".git" repo) t)
  (make-directory downloads t)
  (make-symbolic-link plain link t)
  (mapcar (lambda (dir)
            (cons (file-name-nondirectory dir)
                  (all-the-icons-test-describe (all-the-icons-icon-for-dir dir))))
          (list plain repo link downloads)))"##;
    let expect = expect![[
        r#"OK (("plain" :codepoint 61462 :length 1 :face #1=(:family "github-octicons" :height 1.2) :font-lock-face #1# :display (raise -0.12) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("repo" :codepoint 61441 :length 1 :face #2=(:family "github-octicons" :height 1.2) :font-lock-face #2# :display (raise -0.12) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("link" :codepoint 61617 :length 1 :face #3=(:family "github-octicons" :height 1.2) :font-lock-face #3# :display (raise -0.12) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) ("Downloads" :codepoint 61677 :length 1 :face #4=(:family "FontAwesome" :height 1.08) :font-lock-face #4# :display (raise -0.12) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)))"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}

/// `:height', `:v-adjust' and `:face' are how a caller fits an icon into its
/// own display: they change the properties and never the glyph.  The height
/// multiplies through `all-the-icons-scale-factor' and the family's own
/// factor, the raise is recomputed the same way, and a caller-supplied face
/// replaces the colour the alist chose -- while the character stays the one
/// the alist named.
#[test]
fn height_and_v_adjust_arguments_change_the_properties_not_the_glyph() {
    let elisp_form = r##"(let ((plain (all-the-icons-icon-for-file "sized.el"))
      (scaled (all-the-icons-icon-for-file "sized.el" :height 2.0 :v-adjust 0.5))
      (faced (all-the-icons-icon-for-file "sized.el" :face 'error))
      (both (all-the-icons-icon-for-file "sized.el" :height 0.5 :face 'shadow)))
  (list :plain (all-the-icons-test-describe plain)
        :scaled (all-the-icons-test-describe scaled)
        :faced (all-the-icons-test-describe faced)
        :both (all-the-icons-test-describe both)
        :same-glyph (list (equal (substring-no-properties plain)
                                 (substring-no-properties scaled))
                          (equal (substring-no-properties plain)
                                 (substring-no-properties faced))
                          (equal (substring-no-properties plain)
                                 (substring-no-properties both)))
        :scale-factor all-the-icons-scale-factor
        :default-adjust all-the-icons-default-adjust))"##;
    let expect = expect![[
        r#"OK (:plain (:codepoint 59686 :length 1 :face #1=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) :font-lock-face #1# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :scaled (:codepoint 59686 :length 1 :face #2=(:family "file-icons" :height 2.4 :inherit all-the-icons-purple) :font-lock-face #2# :display (raise 0.6) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :faced (:codepoint 59686 :length 1 :face #3=(:family "file-icons" :height 1.2 :inherit error) :font-lock-face #3# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :both (:codepoint 59686 :length 1 :face #4=(:family "file-icons" :height 0.6 :inherit shadow) :font-lock-face #4# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :same-glyph (t t t) :scale-factor 1.2 :default-adjust -0.2)"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}

/// The mapping is data, not code: teaching the package a new extension is a
/// matter of putting an entry in `all-the-icons-extension-icon-alist', and an
/// entry that is not there means the default icon.  Both halves are checked on
/// file names nothing has looked up yet, because the lookups are memoised.
/// Asking for an icon name the font does not define is an error naming the
/// icon set.
#[test]
fn the_alists_are_the_lookup_table_and_a_missing_entry_means_the_default() {
    let elisp_form = r##"(list
 :taught (let ((all-the-icons-extension-icon-alist
                (cons '("zzz" all-the-icons-faicon "rocket" :face all-the-icons-red)
                      all-the-icons-extension-icon-alist)))
           (all-the-icons-test-describe (all-the-icons-icon-for-file "taught.zzz")))
 :untaught (all-the-icons-test-describe (all-the-icons-icon-for-file "untaught.zzz"))
 :dropped (let ((all-the-icons-extension-icon-alist
                 (assoc-delete-all "el" (copy-sequence all-the-icons-extension-icon-alist))))
            (all-the-icons-test-describe (all-the-icons-icon-for-file "dropped.el")))
 :default-entry all-the-icons-default-file-icon
 :entries (list (assoc "el" all-the-icons-extension-icon-alist)
                (assoc "md" all-the-icons-extension-icon-alist)
                (assoc 'dired-mode all-the-icons-mode-icon-alist))
 :unknown-icon-name (condition-case error
                        (all-the-icons-fileicon "no-such-icon-in-this-font")
                      (error error)))"##;
    let expect = expect![[
        r#"OK (:taught (:codepoint 61749 :length 1 :face #1=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-red) :font-lock-face #1# :display (raise -0.24) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :untaught (:codepoint 61462 :length 1 :face #2=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) :font-lock-face #2# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :dropped (:codepoint 61462 :length 1 :face #3=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) :font-lock-face #3# :display (raise 0.0) :rear-nonsticky t :property-names (face font-lock-face display rear-nonsticky)) :default-entry (all-the-icons-faicon "file-o" :v-adjust 0.0 :face all-the-icons-dsilver) :entries (("el" all-the-icons-fileicon "elisp" :height 1.0 :v-adjust -0.2 :face all-the-icons-purple) ("md" all-the-icons-octicon "markdown" :v-adjust 0.0 :face all-the-icons-lblue) (dired-mode all-the-icons-octicon "file-directory" :v-adjust 0.0)) :unknown-icon-name (error "Unable to find icon with name ‘no-such-icon-in-this-font’ in icon set ‘fileicon’"))"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}

/// Every lookup is memoised on its exact argument list, so the second call for
/// a file returns the identical string object.  The cache is never invalidated,
/// which a user can see: turning `all-the-icons-color-icons' off leaves an
/// already-looked-up file coloured, while a file seen for the first time
/// afterwards comes back with no colour face at all.
#[test]
fn lookups_are_memoised_per_argument_list_and_the_cache_is_never_invalidated() {
    let elisp_form = r##"(let* ((first (all-the-icons-icon-for-file "cached.el"))
       (second (all-the-icons-icon-for-file "cached.el"))
       (after-turning-colour-off
        (let ((all-the-icons-color-icons nil))
          (all-the-icons-icon-for-file "cached.el")))
       (never-seen-before
        (let ((all-the-icons-color-icons nil))
          (all-the-icons-icon-for-file "uncached.el")))
       (seen-again (all-the-icons-icon-for-file "uncached.el")))
  (list :same-object (eq first second)
        :first-face (all-the-icons-test-face first)
        :colour-off-is-the-cached-object (eq first after-turning-colour-off)
        :colour-off-face (all-the-icons-test-face after-turning-colour-off)
        :never-seen-face (all-the-icons-test-face never-seen-before)
        :seen-again-face (all-the-icons-test-face seen-again)
        :seen-again-is-cached (eq never-seen-before seen-again)
        :cached-functions (mapcar (lambda (function)
                                    (get function 'all-the-icons--cached))
                                  '(all-the-icons-icon-for-file
                                    all-the-icons-icon-for-mode
                                    all-the-icons-icon-for-dir
                                    all-the-icons-icon-for-url))))"##;
    let expect = expect![[
        r#"OK (:same-object t :first-face #1=(:family "file-icons" :height 1.2 :inherit all-the-icons-purple) :colour-off-is-the-cached-object t :colour-off-face #1# :never-seen-face #2=(:family "file-icons" :height 1.2) :seen-again-face #2# :seen-again-is-cached t :cached-functions (t t t t))"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}

/// `all-the-icons-install-fonts' is the one command that leaves the editor: it
/// fetches six font files over HTTPS and refreshes the font cache.  Here the
/// transport is recorded rather than performed -- no request is made -- so
/// what is pinned is the package's own work: one URL per bundled font, the
/// destination chosen from `XDG_DATA_HOME' on this system, the directory it
/// creates, the files that end up in it, and the `fc-cache' invocation and
/// completion message the user sees.
#[test]
fn installing_the_fonts_writes_every_bundled_font_without_touching_the_network() {
    let elisp_form = r##"(let ((requests nil)
      (shell-commands nil)
      (home (all-the-icons-test-sandbox "fonts-home")))
  (make-directory (expand-file-name "share" home) t)
  (cl-letf (((symbol-function 'url-copy-file)
             (lambda (url newname &optional ok-if-exists &rest _)
               (push (list url (file-name-nondirectory newname) ok-if-exists) requests)
               (write-region "" nil newname nil 'silent)
               t))
            ((symbol-function 'shell-command-to-string)
             (lambda (command) (push command shell-commands) "")))
    (let ((process-environment
           (append (list (format "HOME=%s" home)
                         (format "XDG_DATA_HOME=%s" (expand-file-name "share" home)))
                   process-environment)))
      (all-the-icons-install-fonts t)))
  (list :requests (nreverse requests)
        :shell-commands shell-commands
        :installed (sort (directory-files (expand-file-name "share/fonts" home) nil "\\`[^.]")
                         #'string<)
        :font-names all-the-icons-font-names
        :font-families all-the-icons-font-families
        :subdirectory all-the-icons-fonts-subdirectory))"##;
    let expect = expect![[
        r#"OK (:requests (("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/material-design-icons.ttf" "material-design-icons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/weathericons.ttf" "weathericons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/octicons.ttf" "octicons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/fontawesome.ttf" "fontawesome.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/file-icons.ttf" "file-icons.ttf" t) ("https://raw.githubusercontent.com/domtronn/all-the-icons.el/master/fonts/all-the-icons.ttf" "all-the-icons.ttf" t)) :shell-commands ("fc-cache -f -v") :installed ("all-the-icons.ttf" "file-icons.ttf" "fontawesome.ttf" "material-design-icons.ttf" "octicons.ttf" "weathericons.ttf") :font-names ("material-design-icons.ttf" "weathericons.ttf" "octicons.ttf" "fontawesome.ttf" "file-icons.ttf" "all-the-icons.ttf") :font-families (material wicon octicon faicon fileicon alltheicon) :subdirectory nil)"#
    ]];

    assert_all_the_icons_parity(elisp_form, expect);
}
