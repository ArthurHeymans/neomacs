use expect_test::expect;

use super::assert_all_the_icons_dired_parity;

#[test]
fn all_the_icons_dired_put_icon_adds_spaced_textual_icon_before_filename() {
    let elisp_form = r##"(with-temp-buffer
         (insert "  main.rs")
         (let ((position 3))
           (cl-letf
               (((symbol-function 'dired-get-filename)
                 (lambda (&rest _) "main.rs"))
                ((symbol-function 'all-the-icons-dired--icon)
                 (lambda (_file)
                   (propertize
                    "R" 'face 'bold 'display '(raise 0.2)))))
             (all-the-icons-dired--put-icon position)
             (list
              (buffer-string)
              (get-text-property 2 'display)
              (text-properties-at 2)))))"##;
    let expect = expect![[
        r#"OK (#("  main.rs" 1 2 (display #(" R " 1 2 (display #1=(raise 0.2) face bold)))) #(" R " 1 2 (display #1# face bold)) (display #(" R " 1 2 (display #1# face bold))))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_put_icon_reserves_spaces_for_dot_entries() {
    let elisp_form = r##"(mapcar
         (lambda (file)
           (with-temp-buffer
             (insert "  entry")
             (cl-letf
                 (((symbol-function 'dired-get-filename)
                   (lambda (&rest _) file))
                  ((symbol-function 'all-the-icons-dired--icon)
                   (lambda (_file)
                     (propertize "X" 'display '(raise 0)))))
               (all-the-icons-dired--put-icon 3)
               (list file
                     (get-text-property 2 'display)
                     (buffer-string)))))
         '("." ".."))"##;
    let expect = expect![[
        r#"OK (("." "    " #("  entry" 1 2 (display #1="    "))) (".." "    " #("  entry" 1 2 (display #1#))))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_put_icon_transfers_image_properties_and_scaled_margin() {
    let elisp_form = r##"(with-temp-buffer
         (insert "  photo.png")
         (let ((image '(image :type png :data "bytes"))
               set-property-call)
           (cl-letf
               (((symbol-function 'dired-get-filename)
                 (lambda (&rest _) "photo.png"))
                ((symbol-function 'all-the-icons-dired--icon)
                 (lambda (_file)
                   (propertize
                    "I"
                    'face 'all-the-icons-blue
                    'display image
                    'rear-nonsticky t)))
                ((symbol-function 'window-text-width)
                 (lambda (&optional _window pixels)
                   (if pixels 900 90)))
                ((symbol-function 'image-property)
                 (lambda (spec property)
                   (plist-get (cdr spec) property)))
                ((symbol-function 'set-image-property)
                 (lambda (spec property value)
                   (setq set-property-call
                         (list spec property value))
                   value)))
             (all-the-icons-dired--put-icon 3)
             (list
              set-property-call
              (get-text-property 2 'display)
              (get-text-property 2 'face)
              (get-text-property 2 'rear-nonsticky)
              (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (nil (image :type png :data "bytes" :margin (10 . 0)) all-the-icons-blue t #("  photo.png" 1 2 (display (image :type png :data "bytes" :margin (10 . 0)) rear-nonsticky t face all-the-icons-blue)))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_fontify_region_uses_extended_jit_bounds_and_each_filename() {
    let elisp_form = r##"(with-temp-buffer
         (insert "line one\nline two\nline three\n")
         (let (put-positions)
           (cl-letf
               (((symbol-function
                 'font-lock-default-fontify-region)
                 (lambda (start end loudly)
                   (ignore loudly)
                   (cons 'jit-lock-bounds
                         (cons (1+ start) end))))
                ((symbol-function 'dired-move-to-filename)
                 (lambda (&rest _)
                   (+ (line-beginning-position) 2)))
                ((symbol-function 'all-the-icons-dired--put-icon)
                 (lambda (position)
                   (push position put-positions))))
             (list
              (all-the-icons-dired--fontify-region
               1 (point-max) 'loud)
              (nreverse put-positions)
              (point)
              (buffer-modified-p)))))"##;
    let expect = expect!["OK ((jit-lock-bounds 2 . 30) (3 12 21) 30 t)"];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_real_dired_buffer_fontifies_files_with_display_icons() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (directory (expand-file-name "listing" root))
               buffer results)
         (make-directory (expand-file-name "src" directory) t)
         (with-temp-file
             (expand-file-name "main.rs" directory)
           (insert "fn main() {}\n"))
         (with-temp-file
             (expand-file-name "README.md" directory)
           (insert "# Project\n"))
         (setq buffer (dired-noselect directory))
         (unwind-protect
             (with-current-buffer buffer
               (all-the-icons-dired-mode 1)
               (font-lock-ensure)
               (dolist (file '("main.rs" "README.md" "src"))
                 (goto-char (point-min))
                 (when (re-search-forward
                        (concat "[[:space:]]"
                                (regexp-quote file)
                                "$")
                        nil t)
                   (let ((position
                          (save-excursion
                            (beginning-of-line)
                            (dired-move-to-filename))))
                     (push
                      (list file
                            (get-text-property
                             (1- position) 'display)
                            (text-properties-at
                             (1- position)))
                      results))))
               (list all-the-icons-dired-mode
                     (nreverse results)
                     (memq 'display
                           font-lock-extra-managed-props)))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (t (("main.rs" #("  " 1 2 (rear-nonsticky t display #2=(raise 0.012) font-lock-face #1=(:family "all-the-icons" :height 1.44) face #1#)) (display #("  " 1 2 (rear-nonsticky t display #2# font-lock-face #1# face #1#)))) ("README.md" #("  " 1 2 (rear-nonsticky t display #4=(raise 0.012) font-lock-face #3=(:family #5="github-octicons" :height 1.2) face #3#)) (display #("  " 1 2 (rear-nonsticky t display #4# font-lock-face #3# face #3#)))) ("src" #("  " 1 2 (rear-nonsticky t display #7=(raise 0.012) font-lock-face #6=(:family #5# :height 1.2 :inherit all-the-icons-dired-dir-face) face #6#)) (display #("  " 1 2 (rear-nonsticky t display #7# font-lock-face #6# face #6#))))) (display))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}
