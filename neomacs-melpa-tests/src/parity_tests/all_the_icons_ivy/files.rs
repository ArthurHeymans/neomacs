use expect_test::expect;

use super::assert_all_the_icons_ivy_parity;

#[test]
fn all_the_icons_ivy_file_icon_selects_directory_and_extension_icons() {
    let elisp_form = r##"(mapcar
         (lambda (candidate)
           (let ((icon
                  (all-the-icons-ivy-icon-for-file
                   candidate)))
             (list candidate
                   (string-to-list icon)
                   (all-the-icons-icon-family icon)
                   (text-properties-at 0 icon))))
         '("src/" "lib.rs" "README.md" "photo.png"
           "archive.tar.gz" "unknown.opaque"))"##;
    let expect = expect![[
        r#"OK (("src/" (61462) "github-octicons" (face #1=(:family "github-octicons" :height 1.2 :inherit all-the-icons-ivy-dir-face) font-lock-face #1# display (raise -0.24) rear-nonsticky t)) ("lib.rs" (59692) "all-the-icons" (face #2=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) font-lock-face #2# display (raise -0.24) rear-nonsticky t)) ("README.md" (61447) "github-octicons" (face #3=(:family "github-octicons" :height 1.2 :inherit all-the-icons-lcyan) font-lock-face #3# display (raise 0.0) rear-nonsticky t)) ("photo.png" (61458) "github-octicons" (face #4=(:family "github-octicons" :height 1.2 :inherit all-the-icons-orange) font-lock-face #4# display (raise 0.0) rear-nonsticky t)) ("archive.tar.gz" (61588) "github-octicons" (face #5=(:family "github-octicons" :height 1.2 :inherit all-the-icons-lmaroon) font-lock-face #5# display (raise 0.0) rear-nonsticky t)) ("unknown.opaque" (61462) "github-octicons" (face #6=(:family "FontAwesome" :height 1.2 :inherit all-the-icons-dsilver) font-lock-face #6# display (raise 0.0) rear-nonsticky t)))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_file_transformer_preserves_candidate_and_custom_spacer() {
    let elisp_form = r##"(let ((all-the-icons-spacer " :: "))
         (mapcar
          (lambda (candidate)
            (let ((transformed
                   (all-the-icons-ivy-file-transformer
                    candidate)))
              (list
               candidate
               (substring-no-properties transformed)
               (get-text-property
                0 'display transformed)
               (text-properties-at 0 transformed))))
          '("src/" "src/main.rs" ".gitignore")))"##;
    let expect = expect![[
        r#"OK (("src/" "\11 :: src/" #("" 0 1 (face #1=(:family #5="github-octicons" :height 1.2 :inherit all-the-icons-ivy-dir-face) font-lock-face #1# display #2=(raise -0.24) rear-nonsticky t)) (display #("" 0 1 (face #1# font-lock-face #1# display #2# rear-nonsticky t)))) ("src/main.rs" "\11 :: src/main.rs" #("" 0 1 (face #3=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) font-lock-face #3# display #4=(raise -0.24) rear-nonsticky t)) (display #("" 0 1 (face #3# font-lock-face #3# display #4# rear-nonsticky t)))) (".gitignore" "\11 :: .gitignore" #("" 0 1 (face #6=(:family #5# :height 1.2) font-lock-face #6# display #7=(raise 0.0) rear-nonsticky t)) (display #("" 0 1 (face #6# font-lock-face #6# display #7# rear-nonsticky t)))))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_real_directory_candidates_render_practical_project_listing() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (project (expand-file-name "project" root))
               candidates)
         (make-directory
          (expand-file-name "src" project) t)
         (with-temp-file
             (expand-file-name "src/main.rs" project)
           (insert "fn main() {}\n"))
         (with-temp-file
             (expand-file-name "README.md" project)
           (insert "# Project\n"))
         (with-temp-file
             (expand-file-name ".gitignore" project)
           (insert "target/\n"))
         (setq candidates
               (sort
                (directory-files project nil nil t)
                #'string<))
         (mapcar
          (lambda (candidate)
            (let* ((file
                    (expand-file-name candidate project))
                   (display-candidate
                    (if (file-directory-p file)
                        (concat candidate "/")
                      candidate))
                   (transformed
                    (all-the-icons-ivy-file-transformer
                     display-candidate)))
              (list
               display-candidate
               (substring-no-properties transformed)
               (string-to-list
                (get-text-property
                 0 'display transformed)))))
          candidates))"##;
    let expect = expect![[
        r#"OK (("./" "\11\11./" (61462)) ("../" "\11\11../" (61462)) (".gitignore" "\11\11.gitignore" (61487)) ("README.md" "\11\11README.md" (61447)) ("src/" "\11\11src/" (61462)))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_ivy_directory_detection_is_trailing_slash_based() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'all-the-icons-octicon)
               (lambda (&rest arguments)
                 (push (cons 'directory arguments) calls)
                 "DIR"))
              ((symbol-function 'all-the-icons-icon-for-file)
               (lambda (&rest arguments)
                 (push (cons 'file arguments) calls)
                 "FILE")))
           (list
            (mapcar
             #'all-the-icons-ivy-icon-for-file
             '("directory/" "directory"
               "/absolute/path/" "name.rs"))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("DIR" "FILE" "DIR" "FILE") ((directory "file-directory" :face all-the-icons-ivy-dir-face) (file "directory") (directory "file-directory" :face all-the-icons-ivy-dir-face) (file "name.rs")))"#
    ]];
    assert_all_the_icons_ivy_parity(elisp_form, expect);
}
