use expect_test::expect;

use super::assert_all_the_icons_ibuffer_parity;

/// The documented installation: enable the mode in an Ibuffer.  It swaps
/// `ibuffer-formats` for its own layout, so the listing gains the package's
/// header and every line gains an icon column -- a glyph carrying display and
/// face properties, followed by the half-width spacer the package appends.
#[test]
fn enabling_the_mode_swaps_the_formats_and_adds_an_icon_column() {
    let elisp_form = r##"(atib-test-in-ibuffer
 (let ((before-formats-were-default (equal ibuffer-formats
                                           all-the-icons-ibuffer-old-formats)))
   (all-the-icons-ibuffer-mode 1)
   (list :before-formats-were-default before-formats-were-default
         :formats-swapped (equal ibuffer-formats all-the-icons-ibuffer-formats)
         :header (substring-no-properties
                  (buffer-substring (point-min) (min (point-max) (+ (point-min) 69))))
         :icons (atib-test-icon-cells))))"##;

    let expect = expect![[
        r#"OK (:before-formats-were-default t :formats-swapped t :header " MRL   Name                    Size Mode             Filename/Process" :icons (("atib-code.el" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-large" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-org" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-plain" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-script.py" icon-glyph display face 32 ((space :relative-width 0.5)))))"#
    ]];

    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

/// The two gates that suppress the icons.  `all-the-icons-ibuffer-icon` nil
/// turns them off explicitly, and the shipped default for
/// `all-the-icons-ibuffer-display-predicate` is `display-graphic-p`, which is
/// false on a terminal -- so out of the box in a non-graphical Emacs the column
/// renders empty rather than showing boxes.  Both paths leave a plain space
/// with no display and no face.
#[test]
fn the_icon_column_is_empty_when_either_gate_is_closed() {
    let elisp_form = r##"(atib-test-in-ibuffer
 (let ((graphic (let ((all-the-icons-ibuffer-display-predicate #'display-graphic-p))
                  (all-the-icons-ibuffer-mode 1)
                  (list (funcall #'display-graphic-p) (atib-test-icon-cells)))))
   (all-the-icons-ibuffer-mode -1)
   (let ((icon-off (let ((all-the-icons-ibuffer-icon nil))
                     (all-the-icons-ibuffer-mode 1)
                     (atib-test-icon-cells))))
     (list :with-graphic-predicate graphic :with-icon-disabled icon-off))))"##;

    let expect = expect![[
        r#"OK (:with-graphic-predicate (nil (("atib-code.el" 32 no-display no-face 32 nil) ("atib-large" 32 no-display no-face 32 nil) ("atib-org" 32 no-display no-face 32 nil) ("atib-plain" 32 no-display no-face 32 nil) ("atib-script.py" 32 no-display no-face 32 nil))) :with-icon-disabled (("atib-code.el" 32 no-display no-face 32 nil) ("atib-large" 32 no-display no-face 32 nil) ("atib-org" 32 no-display no-face 32 nil) ("atib-plain" 32 no-display no-face 32 nil) ("atib-script.py" 32 no-display no-face 32 nil)))"#
    ]];

    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

/// The package replaces Ibuffer's size and mode columns with its own.
/// `all-the-icons-ibuffer-human-readable-size` decides whether a 2048-byte
/// buffer reads "2k" or "2048".  The Mode column comes out empty for every
/// buffer, and the last value explains why rather than leaving it mysterious:
/// `format-mode-line` returns the empty string in batch, which is what that
/// column is built from.
#[test]
fn the_size_column_honours_the_human_readable_setting() {
    let elisp_form = r##"(atib-test-in-ibuffer
 (all-the-icons-ibuffer-mode 1)
 (let ((human (atib-test-columns)))
   (let ((all-the-icons-ibuffer-human-readable-size nil))
     (ibuffer-update nil t)
     (list :human-readable human
           :raw (atib-test-columns)
           :mode-line-empty (format-mode-line mode-name nil nil (get-buffer "atib-org"))))))"##;

    let expect = expect![[
        r#"OK (:human-readable (("atib-code.el" "atib-code.el" "24" "[ORACLE-SANDBOX]/atib-code.el") ("atib-large" "atib-large" "2k") ("atib-org" "atib-org" "8") ("atib-plain" "atib-plain" "15") ("atib-script.py" "atib-script.py" "15" "[ORACLE-SANDBOX]/atib-script.py")) :raw (("atib-code.el" "atib-code.el" "24" "[ORACLE-SANDBOX]/atib-code.el") ("atib-large" "atib-large" "2048") ("atib-org" "atib-org" "8") ("atib-plain" "atib-plain" "15") ("atib-script.py" "atib-script.py" "15" "[ORACLE-SANDBOX]/atib-script.py")) :mode-line-empty "")"#
    ]];

    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

/// The listing tracks the buffer list.  A buffer created after the mode was
/// enabled appears on the next `ibuffer-update` with an icon of its own, and a
/// killed buffer disappears.  Every name is checked as a sorted set, so this
/// says nothing about the order Emacs happens to return buffers in.
#[test]
fn ibuffer_update_picks_up_a_new_buffer_and_drops_a_killed_one() {
    let elisp_form = r##"(atib-test-in-ibuffer
 (all-the-icons-ibuffer-mode 1)
 (let ((initial (mapcar #'car (atib-test-icon-cells))))
   (with-current-buffer (get-buffer-create "atib-neu")
     (fundamental-mode) (erase-buffer) (insert "neu\n"))
   (ibuffer-update nil t)
   (let ((after-add (atib-test-icon-cells)))
     (let ((kill-buffer-query-functions nil))
       (with-current-buffer "atib-plain" (set-buffer-modified-p nil))
       (kill-buffer "atib-plain"))
     (ibuffer-update nil t)
     (list :initial initial
           :after-add (mapcar #'car after-add)
           :new-line-has-icon (cdr (assoc "atib-neu" after-add))
           :after-kill (mapcar #'car (atib-test-icon-cells))))))"##;

    let expect = expect![[
        r#"OK (:initial ("atib-code.el" "atib-large" "atib-org" "atib-plain" "atib-script.py") :after-add ("atib-code.el" "atib-large" "atib-neu" "atib-org" "atib-plain" "atib-script.py") :new-line-has-icon (icon-glyph display face 32 ((space :relative-width 0.5))) :after-kill ("atib-code.el" "atib-large" "atib-neu" "atib-org" "atib-script.py"))"#
    ]];

    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}

/// Turning the mode off puts back the formats that were in force when the
/// package was loaded and re-renders, so the icon column disappears entirely --
/// the character where the glyph used to be is now the first letter of the
/// buffer name.
#[test]
fn turning_the_mode_off_restores_the_previous_formats() {
    let elisp_form = r##"(atib-test-in-ibuffer
 (all-the-icons-ibuffer-mode 1)
 (let ((on-icons (atib-test-icon-cells))
       (on-formats (equal ibuffer-formats all-the-icons-ibuffer-formats)))
   (all-the-icons-ibuffer-mode -1)
   (list :on-formats on-formats
         :on-icons on-icons
         :off-formats-restored (equal ibuffer-formats all-the-icons-ibuffer-old-formats)
         :off-icons (atib-test-icon-cells)
         :mode-flag (and all-the-icons-ibuffer-mode t))))"##;

    let expect = expect![[
        r#"OK (:on-formats t :on-icons (("atib-code.el" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-large" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-org" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-plain" icon-glyph display face 32 ((space :relative-width 0.5))) ("atib-script.py" icon-glyph display face 32 ((space :relative-width 0.5)))) :off-formats-restored t :off-icons (("atib-code.el" 97 no-display no-face 116 nil) ("atib-large" 97 no-display no-face 116 nil) ("atib-org" 97 no-display no-face 116 nil) ("atib-plain" 97 no-display no-face 116 nil) ("atib-script.py" 97 no-display no-face 116 nil)) :mode-flag nil)"#
    ]];

    assert_all_the_icons_ibuffer_parity(elisp_form, expect);
}
