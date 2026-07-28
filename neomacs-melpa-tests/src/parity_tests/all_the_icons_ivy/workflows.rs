use expect_test::expect;

use super::assert_all_the_icons_ivy_parity;

/// The documented one-liner installation, `(all-the-icons-ivy-setup)`.  Pins
/// both command lists and, for every command in them, what ivy's own display
/// transformer registry ends up holding -- which is the table ivy consults, so
/// this is the real contract rather than a proxy for it.
#[test]
fn setup_registers_the_documented_transformers_for_every_command() {
    let elisp_form = r##"(progn
  (all-the-icons-ivy-setup)
  (list :buffer-commands all-the-icons-ivy-buffer-commands
        :file-commands all-the-icons-ivy-file-commands
        :registered-buffer (mapcar (lambda (c) (cons c (ativ-test-transformer-for c)))
                                   all-the-icons-ivy-buffer-commands)
        :registered-file (mapcar (lambda (c) (cons c (ativ-test-transformer-for c)))
                                 all-the-icons-ivy-file-commands)
        :spacer all-the-icons-spacer))"##;

    let expect = expect![[
        r#"OK (:buffer-commands (ivy-switch-buffer ivy-switch-buffer-other-window counsel-projectile-switch-to-buffer) :file-commands (counsel-find-file counsel-file-jump counsel-recentf counsel-projectile counsel-projectile-find-file counsel-projectile-find-dir counsel-git) :registered-buffer ((ivy-switch-buffer . all-the-icons-ivy-buffer-transformer) (ivy-switch-buffer-other-window . all-the-icons-ivy-buffer-transformer) (counsel-projectile-switch-to-buffer . all-the-icons-ivy-buffer-transformer)) :registered-file ((counsel-find-file . all-the-icons-ivy-file-transformer) (counsel-file-jump . all-the-icons-ivy-file-transformer) (counsel-recentf . all-the-icons-ivy-file-transformer) (counsel-projectile . all-the-icons-ivy-file-transformer) (counsel-projectile-find-file . all-the-icons-ivy-file-transformer) (counsel-projectile-find-dir . all-the-icons-ivy-file-transformer) (counsel-git . all-the-icons-ivy-file-transformer)) :spacer "\11")"#
    ]];

    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

/// The buffer transformer over real buffers.  Each candidate comes back as a
/// tab carrying the icon as a `display` property, then the spacer, then the
/// name; a file-visiting buffer with unsaved changes additionally gets
/// `ivy-modified-buffer` on its name, and an unmodified one does not.
///
/// `:prop-names-at-0` is the property plist's key order.  The package builds
/// every candidate with `format` over a propertized string, which is the shape
/// catalogue entry 22 describes, so the order is pinned rather than the values.
#[test]
fn the_buffer_transformer_prefixes_a_candidate_and_marks_modified_buffers() {
    let elisp_form = r##"(let ((el (ativ-test-write "ativ-code.el" ";; Grüße\n")))
  (find-file-noselect el)
  (with-current-buffer (get-buffer-create "ativ-plain") (fundamental-mode))
  (with-current-buffer (get-buffer "ativ-code.el") (insert "geändert"))
  (list :modified-file-buffer (ativ-test-describe
                               (all-the-icons-ivy-buffer-transformer "ativ-code.el"))
        :plain-buffer (ativ-test-describe
                       (all-the-icons-ivy-buffer-transformer "ativ-plain"))))"##;

    let expect = expect![[
        r#"OK (:modified-file-buffer (:text "\11\11ativ-code.el" :length 14 :first-char 9 :prop-names-at-0 (display) :icon-one-char-string t :icon-prop-names (face font-lock-face display rear-nonsticky) :face-on-name ivy-modified-buffer) :plain-buffer (:text "\11\11ativ-plain" :length 12 :first-char 9 :prop-names-at-0 (display) :icon-one-char-string t :icon-prop-names (face font-lock-face display rear-nonsticky) :face-on-name nil))"#
    ]];

    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

/// The documented fallback: `ivy-switch-buffer` can offer a recent *file* whose
/// buffer does not exist, and the same transformer is used for it.  A candidate
/// naming no live buffer is handed to the file transformer instead, and still
/// comes back with an icon rather than failing.
#[test]
fn a_candidate_that_names_no_buffer_falls_through_to_the_file_transformer() {
    let elisp_form = r##"(list :missing-buffer (ativ-test-describe
                       (all-the-icons-ivy-buffer-transformer "gibt-es-nicht.py"))
      :same-as-file-transformer
      (equal (substring-no-properties
              (all-the-icons-ivy-buffer-transformer "gibt-es-nicht.py"))
             (substring-no-properties
              (all-the-icons-ivy-file-transformer "gibt-es-nicht.py"))))"##;

    let expect = expect![[
        r#"OK (:missing-buffer (:text "\11\11gibt-es-nicht.py" :length 18 :first-char 9 :prop-names-at-0 (display) :icon-one-char-string t :icon-prop-names (face font-lock-face display rear-nonsticky) :face-on-name nil) :same-as-file-transformer t)"#
    ]];

    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

/// The file transformer's own branch: a candidate ending in a slash is a
/// directory and gets the package's directory icon, carrying this package's
/// `all-the-icons-ivy-dir-face`.  A plain file name does not, which is asserted
/// by looking for that face rather than by naming either glyph.
#[test]
fn the_file_transformer_gives_directories_the_packages_own_face() {
    let elisp_form = r##"(let ((dir (all-the-icons-ivy-file-transformer "src/"))
      (file (all-the-icons-ivy-file-transformer "notes.org")))
  (list :dir-icon-inherits-dir-face
        (eq (plist-get (get-text-property 0 'face (get-text-property 0 'display dir))
                       :inherit)
            'all-the-icons-ivy-dir-face)
        :file-icon-inherits-dir-face
        (eq (plist-get (get-text-property 0 'face (get-text-property 0 'display file))
                       :inherit)
            'all-the-icons-ivy-dir-face)
        :dir-candidate (ativ-test-describe dir)))"##;

    let expect = expect![[
        r#"OK (:dir-icon-inherits-dir-face t :file-icon-inherits-dir-face nil :dir-candidate (:text "\11\11src/" :length 6 :first-char 9 :prop-names-at-0 (display) :icon-one-char-string t :icon-prop-names (face font-lock-face display rear-nonsticky) :face-on-name nil))"#
    ]];

    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

/// The two customizations.  `all-the-icons-spacer` replaces the separator
/// between icon and candidate.  For a mode with no icon of its own and no
/// parent to inherit one from, the family and name fallbacks decide the icon --
/// asserted by comparing the default and customized results, so neither glyph
/// has to be named.
#[test]
fn the_spacer_and_the_buffer_icon_fallback_are_customizable() {
    let elisp_form = r##"(progn
  (define-derived-mode ativ-unknown-mode nil "AtivUnknown")
  (with-current-buffer (get-buffer-create "ativ-unknown") (ativ-unknown-mode))
  (let* ((icon-of (lambda (result)
                    (copy-sequence
                     (substring-no-properties (get-text-property 0 'display result)))))
         (default (all-the-icons-ivy-buffer-transformer "ativ-unknown"))
         (custom (let ((all-the-icons-ivy-family-fallback-for-buffer 'all-the-icons-octicon)
                       (all-the-icons-ivy-name-fallback-for-buffer "database"))
                   (all-the-icons-ivy-buffer-transformer "ativ-unknown"))))
    (list :mode-has-no-icon-of-its-own
          (symbolp (all-the-icons-icon-for-mode 'ativ-unknown-mode))
          :parent (get 'ativ-unknown-mode 'derived-mode-parent)
          :fallback-changes-the-icon
          (not (string= (funcall icon-of default) (funcall icon-of custom)))
          :default-shape (ativ-test-describe default)
          :custom-spacer
          (let ((all-the-icons-spacer " | "))
            (substring-no-properties
             (all-the-icons-ivy-buffer-transformer "ativ-unknown"))))))"##;

    let expect = expect![[
        r#"OK (:mode-has-no-icon-of-its-own t :parent nil :fallback-changes-the-icon t :default-shape (:text "\11\11ativ-unknown" :length 14 :first-char 9 :prop-names-at-0 (display) :icon-one-char-string t :icon-prop-names (face font-lock-face display rear-nonsticky) :face-on-name nil) :custom-spacer "\11 | ativ-unknown")"#
    ]];

    assert_all_the_icons_ivy_parity(elisp_form, expect);
}
