use expect_test::expect;

use super::assert_all_the_icons_dired_parity;

/// Turning the mode on in a real Dired buffer.  Every file gets a three
/// character `" X "` display property on the character before its name, with
/// the middle character carrying the icon's own properties; `.` and `..` get a
/// four space placeholder instead.  The buffer text itself is untouched --
/// icons are display properties, not inserted characters.
#[test]
fn turning_the_mode_on_puts_a_display_icon_before_every_filename() {
    let elisp_form = r##"(atid-test-in-dired
 (let ((before-lines (atid-test-lines))
       (before-count (atid-test-display-count))
       (before-text (atid-test-text)))
   (all-the-icons-dired-mode 1)
   (font-lock-ensure)
   (list :mode-on (and all-the-icons-dired-mode t)
         :lighter all-the-icons-dired-lighter
         :before-count before-count
         :before-lines before-lines
         :after-count (atid-test-display-count)
         :after-lines (atid-test-lines)
         :text-unchanged (string= before-text (atid-test-text)))))"##;

    let expect = expect![[
        r#"OK (:mode-on t :lighter " all-the-icons-dired-mode" :before-count 1 :before-lines (("." none) (".." none) (".hidden-config" none) ("README.md" none) ("notes.org" none) ("script.py" none) ("subdir" none)) :after-count 7 :after-lines (("." (string 4 "    " plain)) (".." (string 4 "    " plain)) (".hidden-config" (string 3 "  " icon-props)) ("README.md" (string 3 "  " icon-props)) ("notes.org" (string 3 "  " icon-props)) ("script.py" (string 3 "  " icon-props)) ("subdir" (string 3 "  " icon-props))) :text-unchanged t)"#
    ]];

    assert_all_the_icons_dired_parity(elisp_form, expect);
}

/// Reverting the listing throws the buffer's text away and rebuilds it, so the
/// icons have to be reapplied by the fontifier the mode installed.  Pins that
/// every line comes back with exactly the same display property it had before.
#[test]
fn reverting_the_listing_reapplies_every_icon() {
    let elisp_form = r##"(atid-test-in-dired
 (all-the-icons-dired-mode 1)
 (font-lock-ensure)
 (let ((before (atid-test-lines)) (before-text (atid-test-text)))
   (revert-buffer)
   (font-lock-ensure)
   (list :after-revert (atid-test-lines)
         :count (atid-test-display-count)
         :same-as-before (equal before (atid-test-lines))
         :text-unchanged (string= before-text (atid-test-text)))))"##;

    let expect = expect![[
        r#"OK (:after-revert (("." (string 4 "    " plain)) (".." (string 4 "    " plain)) (".hidden-config" (string 3 "  " icon-props)) ("README.md" (string 3 "  " icon-props)) ("notes.org" (string 3 "  " icon-props)) ("script.py" (string 3 "  " icon-props)) ("subdir" (string 3 "  " icon-props))) :count 7 :same-as-before t :text-unchanged t)"#
    ]];

    assert_all_the_icons_dired_parity(elisp_form, expect);
}

/// Inserting a subdirectory adds lines the mode never saw, and they get icons
/// too.  The `.` and `..` of the inserted subdirectory do *not* get the
/// placeholder, because the package compares the name Dired reports -- here
/// "subdir/." -- against the literal strings "." and "..", so the guard misses
/// and they are given real icons.
#[test]
fn inserting_a_subdirectory_gets_icons_on_its_lines_and_its_dot_entries() {
    let elisp_form = r##"(atid-test-in-dired
 (all-the-icons-dired-mode 1)
 (font-lock-ensure)
 (let ((before-count (length (atid-test-lines))))
   (goto-char (point-min))
   (search-forward "subdir")
   (dired-maybe-insert-subdir (expand-file-name "subdir" atid-test-tree))
   (font-lock-ensure)
   (list :before-count before-count
         :after (atid-test-lines)
         :display-count (atid-test-display-count))))"##;

    let expect = expect![[
        r#"OK (:before-count 7 :after (("." (string 4 "    " plain)) (".." (string 4 "    " plain)) (".hidden-config" (string 3 "  " icon-props)) ("README.md" (string 3 "  " icon-props)) ("notes.org" (string 3 "  " icon-props)) ("script.py" (string 3 "  " icon-props)) ("subdir" (string 3 "  " icon-props)) ("subdir/." (string 3 "  " icon-props)) ("subdir/.." (string 3 "  " icon-props)) ("subdir/nested.el" (string 3 "  " icon-props))) :display-count 10)"#
    ]];

    assert_all_the_icons_dired_parity(elisp_form, expect);
}

/// Turning the mode off restores the text exactly and leaves no display
/// property behind.  Note the asymmetry the counts record: the pristine buffer
/// already carried one display property of Dired's own, and teardown removes
/// that one too, because it unfontifies the whole buffer rather than removing
/// only what it added.
#[test]
fn turning_the_mode_off_removes_every_display_property_including_dired_s_own() {
    let elisp_form = r##"(atid-test-in-dired
 (let ((pristine-text (atid-test-text))
       (pristine-count (atid-test-display-count)))
   (all-the-icons-dired-mode 1)
   (font-lock-ensure)
   (let ((on-count (atid-test-display-count)))
     (all-the-icons-dired-mode -1)
     (font-lock-ensure)
     (list :pristine-count pristine-count
           :on-count on-count
           :off-count (atid-test-display-count)
           :off-lines (atid-test-lines)
           :mode-off (and all-the-icons-dired-mode t)
           :text-identical (string= pristine-text (atid-test-text))))))"##;

    let expect = expect![[
        r#"OK (:pristine-count 1 :on-count 7 :off-count 0 :off-lines (("." none) (".." none) (".hidden-config" none) ("README.md" none) ("notes.org" none) ("script.py" none) ("subdir" none)) :mode-off nil :text-identical t)"#
    ]];

    assert_all_the_icons_dired_parity(elisp_form, expect);
}

/// The mode is defined globally but only does anything in Dired.  Enabled in an
/// ordinary buffer its flag goes on and its lighter is registered, but it
/// installs no fontifier override, adds nothing to
/// `font-lock-extra-managed-props`, and leaves the buffer alone.
#[test]
fn enabling_the_mode_outside_dired_changes_nothing_but_the_flag() {
    let elisp_form = r##"(let ((buffer (generate-new-buffer "*nicht-dired*")))
  (unwind-protect
      (with-current-buffer buffer
        (fundamental-mode)
        (insert "nur Text\n")
        (let ((before (atid-test-text))
              (fontifier font-lock-fontify-region-function))
          (all-the-icons-dired-mode 1)
          (list :mode-flag (and all-the-icons-dired-mode t)
                :fontifier-unchanged (eq fontifier font-lock-fontify-region-function)
                :extra-props font-lock-extra-managed-props
                :text-unchanged (string= before (atid-test-text))
                :display-count (atid-test-display-count)
                :lighter (cdr (assq 'all-the-icons-dired-mode minor-mode-alist)))))
    (kill-buffer buffer)))"##;

    let expect = expect![[
        r#"OK (:mode-flag t :fontifier-unchanged t :extra-props nil :text-unchanged t :display-count 0 :lighter (all-the-icons-dired-lighter))"#
    ]];

    assert_all_the_icons_dired_parity(elisp_form, expect);
}
