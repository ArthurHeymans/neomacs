use expect_test::expect;

use super::assert_all_the_icons_parity;

#[test]
fn all_the_icons_directory_workflow_recognizes_named_plain_and_git_directories() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (plain (expand-file-name "workspace" root))
               (git-dir (expand-file-name "project" root))
               (repo-marker (expand-file-name ".git" git-dir)))
         (make-directory plain t)
         (make-directory repo-marker t)
         (mapcar
          (lambda (directory)
            (let ((icon
                   (all-the-icons-icon-for-dir
                    directory :height 1.6 :v-adjust 0.1
                    :face 'all-the-icons-blue)))
              (list (file-name-base
                     (directory-file-name directory))
                    (string-to-list icon)
                    (all-the-icons-icon-family icon)
                    (text-properties-at 0 icon))))
          (list plain git-dir
                (expand-file-name "Downloads" root)
                (expand-file-name "Music" root))))"##;
    let expect = expect![[
        r#"OK (("workspace" (61535) "github-octicons" (face #1=(:family "github-octicons" :height 1.92 :inherit all-the-icons-blue) font-lock-face #1# display (raise 0.12) rear-nonsticky t)) ("project" (61441) "github-octicons" (face #2=(:family "github-octicons" :height 1.92 :inherit all-the-icons-blue) font-lock-face #2# display (raise 0.12) rear-nonsticky t)) ("Downloads" (61677) "FontAwesome" (face #3=(:family "FontAwesome" :height 1.92 :inherit all-the-icons-blue) font-lock-face #3# display (raise 0.12) rear-nonsticky t)) ("Music" (61441) "github-octicons" (face #4=(:family "FontAwesome" :height 1.92 :inherit all-the-icons-blue) font-lock-face #4# display (raise 0.12) rear-nonsticky t)))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_directory_workflow_distinguishes_symlink_and_git_submodule() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (target (expand-file-name "target" root))
               (link (expand-file-name "linked" root))
               (module (expand-file-name "vendor/lib" root))
               (gitmodules (expand-file-name ".gitmodules" root)))
         (make-directory target t)
         (condition-case nil
             (make-symbolic-link target link t)
           (file-error nil))
         (make-directory (expand-file-name ".git" module) t)
         (with-temp-file gitmodules
           (insert "[submodule \"vendor/lib\"]\n"
                   " path = vendor/lib\n"
                   " url = https://example.invalid/lib.git\n"))
         (list
          (and (file-symlink-p link)
               (string-to-list
                (all-the-icons-icon-for-dir link)))
          (all-the-icons-dir-is-submodule module)
          (string-to-list
           (all-the-icons-icon-for-dir module))))"##;
    let expect = expect!["OK ((61617) 24 (61463))"];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_remote_directory_avoids_local_filesystem_and_uses_terminal_icon() {
    let elisp_form = r##"(let (remote-checks)
         (cl-letf
             (((symbol-function 'file-remote-p)
               (lambda (&rest arguments)
                 (push arguments remote-checks)
                 "/ssh:host:")))
           (let ((icon
                  (all-the-icons-icon-for-dir
                   "/ssh:host:/srv/app")))
             (list (nreverse remote-checks)
                   (string-to-list icon)
                   (all-the-icons-icon-family icon)
                   (text-properties-at 0 icon)))))"##;
    let expect = expect![[
        r#"OK ((("[ORACLE-TMPDIR]/" localname) ("[ORACLE-TMPDIR]/") ("[ORACLE-TMPDIR]/" localname) ("[ORACLE-TMPDIR]/" localname) ("[ORACLE-TMPDIR]/") ("[ORACLE-TMPDIR]/" localname) ("[ORACLE-TMPDIR]/" localname) ("[ORACLE-TMPDIR]/") ("[ORACLE-TMPDIR]/" localname) ("/srv/app" localname) ("/ssh:host:" localname) ("/srv/app") ("/ssh:host:/srv/app")) (61640) "github-octicons" (face #1=(:family "github-octicons" :height 1.2) font-lock-face #1# display (raise -0.12) rear-nonsticky t))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_directory_chevron_workflow_preserves_padding_and_icon_properties() {
    let elisp_form = r##"(let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
               (directory (expand-file-name "Code" root)))
         (make-directory directory t)
         (let ((open
                (all-the-icons-icon-for-dir-with-chevron
                 directory "down" " | "))
               (closed
                (all-the-icons-icon-for-dir-with-chevron
                 directory nil ".")))
           (list
            (substring-no-properties open)
            (string-to-list open)
            (mapcar
             (lambda (position)
               (get-text-property position 'face open))
             (number-sequence 0 (1- (length open))))
            (substring-no-properties closed)
            (string-to-list closed))))"##;
    let expect = expect![[
        r#"OK (" |  |  | " (32 124 32 61603 32 124 32 61535 32 124 32) (nil nil nil (:family "github-octicons" :height 0.96) nil nil nil (:family "github-octicons" :height 1.32) nil nil nil) "..." (46 46 61535 46))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}
