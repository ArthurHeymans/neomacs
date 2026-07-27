use expect_test::expect;

use super::assert_apt_sources_list_parity;

#[test]
fn mode_activation_establishes_prog_parent_comments_syntax_and_font_lock_environment() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (list
   major-mode mode-name
   (derived-mode-p 'prog-mode)
   comment-start
   comment-start-skip
   (char-syntax ?#)
   (char-syntax ?\n)
   (syntax-table-p (syntax-table))
   (length font-lock-keywords)
   (member apt-sources-list-font-lock-keywords
           font-lock-keywords)
   (get 'apt-sources-list-mode
        'derived-mode-parent)))"##;
    let expect = expect![[
        r##"OK (apt-sources-list-mode "apt/sources.list" prog-mode "#" "#+ *" 60 62 t 3 ((#1=("^[[:blank:]]*\\(\\(?:deb\\(?:-src\\)?\\)\\)[[:blank:]]+\\(?:\\[\\([^]\n#]+\\)][[:blank:]]+\\)?\\([.0-9A-Z_a-z-]+:[^\11\n #]+\\)[[:blank:]]+\\([^\11\n #]*/\\|[^\11\n #]*[^\11\n #/][[:blank:]]+\\([^\11\n #]+\\(?:[[:blank:]]+[^\11\n #]+\\)*\\)\\)[[:blank:]]*\\(?:$\\|#\\)" (1 'apt-sources-list-type) (2 'apt-sources-list-options nil t) (3 'apt-sources-list-uri) (4 'apt-sources-list-suite) (5 'apt-sources-list-components t t))) #1#) prog-mode)"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn practical_font_lock_assigns_every_semantic_and_comment_face_on_complex_sources() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb-src [arch=amd64 signed-by=/keys/acme.gpg] https://apt.example/debian bookworm-updates main contrib non-free-firmware # production\n"
   "deb file:/srv/mirror dists/stable/main/binary-amd64/ # exact\n")
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (let ((start
            (- (point) (length needle))))
       (list
        needle
        (get-text-property start 'face)
        (get-text-property start
                           'font-lock-face))))
   '("deb-src"
     "arch=amd64"
     "https://apt.example/debian"
     "bookworm-updates"
     "main"
     "contrib"
     "non-free-firmware"
     "# production"
     "production"
     "file:/srv/mirror"
     "dists/stable/main/binary-amd64/"
     "# exact"
     "exact")))"##;
    let expect = expect![[
        r##"OK (("deb-src" apt-sources-list-type nil) ("arch=amd64" apt-sources-list-options nil) ("https://apt.example/debian" apt-sources-list-uri nil) ("bookworm-updates" apt-sources-list-suite nil) ("main" apt-sources-list-components nil) ("contrib" apt-sources-list-components nil) ("non-free-firmware" apt-sources-list-components nil) ("# production" font-lock-comment-delimiter-face nil) ("production" font-lock-comment-face nil) ("file:/srv/mirror" apt-sources-list-uri nil) ("dists/stable/main/binary-amd64/" apt-sources-list-suite nil) ("# exact" font-lock-comment-delimiter-face nil) ("exact" font-lock-comment-face nil))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn malformed_source_text_remains_unfontified_while_comments_use_prog_mode_faces() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb malformed line\n"
   "rpm https://packages.example stable main\n"
   "# deb https://commented.example stable main\n")
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (list
      needle
      (get-text-property
       (- (point) (length needle))
       'face)))
   '("deb" "malformed" "rpm" "packages.example"
     "#" "commented.example")))"##;
    let expect = expect![[
        r##"OK (("deb" nil) ("malformed" nil) ("rpm" nil) ("packages.example" nil) ("#" font-lock-comment-delimiter-face) ("commented.example" font-lock-comment-face))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_exposes_every_editor_binding_and_standard_list_motion_remap() {
    let elisp_form = r##"(list
 (mapcar
  (lambda (key)
    (list key
          (lookup-key
           apt-sources-list-mode-map
           (kbd key))))
  '("C-c C-i"
    "C-c C-r"
    "C-c C-t"
    "C-c C-o"
    "C-c C-u"
    "C-c C-s"
    "C-c C-c"))
 (mapcar
  (lambda (command)
    (list command
          (lookup-key
           apt-sources-list-mode-map
           (vector 'remap command))))
  '(forward-list backward-list))
 (eq (keymap-parent
      apt-sources-list-mode-map)
     prog-mode-map))"##;
    let expect = expect![[
        r#"OK ((("C-c C-i" apt-sources-list-insert) ("C-c C-r" apt-sources-list-replicate) ("C-c C-t" apt-sources-list-change-type) ("C-c C-o" apt-sources-list-change-options) ("C-c C-u" apt-sources-list-change-uri) ("C-c C-s" apt-sources-list-change-suite) ("C-c C-c" apt-sources-list-change-components)) ((forward-list apt-sources-list-forward-source) (backward-list apt-sources-list-backward-source)) nil)"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn mode_menu_preserves_command_order_separators_and_context_enable_forms() {
    let elisp_form = r##"(list
 (boundp 'apt-sources-list-mode-menu)
 (keymapp apt-sources-list-mode-menu)
 (copy-tree apt-sources-list-mode-menu))"##;
    let expect = expect![[
        r#"OK (t t (keymap "APT" (Insert\ Source menu-item "Insert Source" apt-sources-list-insert) (Copy\ Source menu-item "Copy Source" apt-sources-list-replicate :enable (apt-sources-list-source-p)) (nil "--") (Backward\ Source menu-item "Backward Source" apt-sources-list-backward-source) (Forward\ Source menu-item "Forward Source" apt-sources-list-forward-source) (nil-5 "--") (Change\ Type menu-item "Change Type" apt-sources-list-change-type :enable (apt-sources-list-source-p)) (Change\ Options menu-item "Change Options" apt-sources-list-change-options :enable (apt-sources-list-source-p)) (Change\ URI menu-item "Change URI" apt-sources-list-change-uri :enable (apt-sources-list-source-p)) (Change\ Suite menu-item "Change Suite" apt-sources-list-change-suite :enable (apt-sources-list-source-p)) (Change\ Components menu-item "Change Components" apt-sources-list-change-components :enable (ignore-errors (and (apt-sources-list-match-source) (match-string 5))))))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn where_is_internal_orders_keyboard_bindings_before_menu_paths_like_gnu_emacs() {
    let elisp_form = r##"(list
 (where-is-internal
  'apt-sources-list-insert
  apt-sources-list-mode-map)
 (where-is-internal
  'apt-sources-list-change-components
  apt-sources-list-mode-map))"##;
    let expect = expect![[
        r#"OK (([3 9] [menu-bar apt Insert\ Source]) ([3 3] [menu-bar apt Change\ Components]))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn auto_mode_detection_selects_the_mode_for_system_relative_and_drop_in_list_files_only() {
    let elisp_form = r##"(mapcar
 (lambda (filename)
   (with-temp-buffer
     (setq buffer-file-name filename)
     (set-auto-mode)
     (list filename major-mode)))
 '("/etc/apt/sources.list"
   "./sources.list"
   "/workspace/sources.list"
   "/etc/apt/sources.list.d/debian.list"
   "/srv/config/sources.list.d/vendor.list"
   "/etc/apt/sources.list.d/vendor.sources"
   "/workspace/debian.list"
   "/workspace/sources.list.backup"))"##;
    let expect = expect![[
        r#"OK (("/etc/apt/sources.list" apt-sources-list-mode) ("./sources.list" apt-sources-list-mode) ("/workspace/sources.list" apt-sources-list-mode) ("/etc/apt/sources.list.d/debian.list" apt-sources-list-mode) ("/srv/config/sources.list.d/vendor.list" apt-sources-list-mode) ("/etc/apt/sources.list.d/vendor.sources" fundamental-mode) ("/workspace/debian.list" fundamental-mode) ("/workspace/sources.list.backup" fundamental-mode))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn standard_comment_commands_round_trip_multiple_repository_lines_with_mode_syntax() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb https://one.example/debian stable main\n"
   "deb-src https://two.example/debian stable main\n")
  (let ((start (point-min))
        (end (point-max)))
    (comment-region start end)
    (let ((commented (buffer-string)))
      (uncomment-region (point-min) (point-max))
      (list
       commented
       (buffer-string)
       (mapcar
        (lambda (line)
          (goto-char (point-min))
          (forward-line line)
          (apt-sources-list-source-p))
        '(0 1))))))"##;
    let expect = expect![[
        r##"OK ("# deb https://one.example/debian stable main\n# deb-src https://two.example/debian stable main\n" "deb https://one.example/debian stable main\ndeb-src https://two.example/debian stable main\n" (0 0))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}
