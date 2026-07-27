use expect_test::expect;

use super::assert_all_the_icons_dired_parity;

#[test]
fn all_the_icons_dired_directory_icon_uses_directory_face_and_vertical_adjustment() {
    let elisp_form = r##"(let ((all-the-icons-dired-v-adjust 0.375)
               call)
         (cl-letf
             (((symbol-function 'file-directory-p)
               (lambda (file)
                 (setq call (list 'directory-p file))
                 t))
              ((symbol-function 'all-the-icons-icon-for-dir)
               (lambda (&rest arguments)
                 (setq call (append call arguments))
                 "DIR-ICON")))
           (list
            (all-the-icons-dired--icon "/project/src")
            call)))"##;
    let expect = expect![[
        r#"OK ("DIR-ICON" (directory-p "/project/src" "/project/src" :face all-the-icons-dired-dir-face :v-adjust 0.375))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_monochrome_file_icon_inherits_face_at_point() {
    let elisp_form = r##"(let ((all-the-icons-dired-v-adjust -0.125)
               (all-the-icons-dired-monochrome t)
               call)
         (cl-letf
             (((symbol-function 'file-directory-p)
               (lambda (_file) nil))
              ((symbol-function 'face-at-point)
               (lambda () 'dired-marked))
              ((symbol-function 'all-the-icons-icon-for-file)
               (lambda (&rest arguments)
                 (setq call arguments)
                 "FILE-ICON")))
           (list
            (all-the-icons-dired--icon "main.rs")
            call)))"##;
    let expect = expect![[r#"OK ("FILE-ICON" ("main.rs" :v-adjust -0.125 :face dired-marked))"#]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_colored_file_icon_omits_line_face_override() {
    let elisp_form = r##"(let ((all-the-icons-dired-v-adjust 0.01)
               (all-the-icons-dired-monochrome nil)
               face-called
               call)
         (cl-letf
             (((symbol-function 'file-directory-p)
               (lambda (_file) nil))
              ((symbol-function 'face-at-point)
               (lambda ()
                 (setq face-called t)
                 'unexpected))
              ((symbol-function 'all-the-icons-icon-for-file)
               (lambda (&rest arguments)
                 (setq call arguments)
                 "FILE-ICON")))
           (list
            (all-the-icons-dired--icon "main.el")
            call face-called)))"##;
    let expect = expect![[r#"OK ("FILE-ICON" ("main.el" :v-adjust 0.01) nil)"#]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_real_dependency_renders_practical_file_and_directory_icons() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (directory (expand-file-name "src" root))
               (file (expand-file-name "main.rs" root)))
         (make-directory directory t)
         (with-temp-file file (insert "fn main() {}\n"))
         (let ((all-the-icons-dired-monochrome nil)
               (dir-icon
                (all-the-icons-dired--icon directory))
               (file-icon
                (all-the-icons-dired--icon file)))
           (list
            (string-to-list dir-icon)
            (all-the-icons-icon-family dir-icon)
            (text-properties-at 0 dir-icon)
            (string-to-list file-icon)
            (all-the-icons-icon-family file-icon)
            (text-properties-at 0 file-icon))))"##;
    let expect = expect![[
        r#"OK ((61462) "github-octicons" (face #1=(:family "github-octicons" :height 1.2 :inherit all-the-icons-dired-dir-face) font-lock-face #1# display (raise 0.012) rear-nonsticky t) (59692) "all-the-icons" (face #2=(:family "all-the-icons" :height 1.44) font-lock-face #2# display (raise 0.012) rear-nonsticky t))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}
