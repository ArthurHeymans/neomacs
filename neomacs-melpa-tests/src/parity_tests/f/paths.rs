use expect_test::expect;

use super::{assert_f_parity, assert_f_signal_parity};

#[test]
fn f_path_composition_and_decomposition_match() {
    let elisp_form = r##"(list
              (f-join "alpha" "beta" "gamma.txt")
              (f-join "/alpha" "beta" "/reset" "leaf")
              (f-split "/alpha/beta/gamma.txt")
              (f-split "/")
              (f-filename "/alpha/beta/gamma.txt")
              (f-dirname "/alpha/beta/gamma.txt")
              (f-base "/alpha/beta/archive.tar.gz")
              (f-ext "/alpha/beta/archive.tar.gz")
              (f-no-ext "/alpha/beta/archive.tar.gz")
              (f-swap-ext "/alpha/beta/archive.tar.gz" "xz"))"##;
    let expect = expect![[
        r#"OK ("alpha/beta/gamma.txt" "/reset/leaf" ("/" "alpha" "beta" "gamma.txt") ("/") "gamma.txt" "/alpha/beta" "archive.tar" "gz" "/alpha/beta/archive.tar" "/alpha/beta/archive.tar.xz")"#
    ]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_expand_relative_abbreviate_and_canonicalize_paths() {
    let elisp_form = r##"(let* ((home (getenv "HOME"))
                    (target (f-expand "Code/project/file.el" home)))
               (list
                (f-expand "child/" "/base/")
                (f-relative "/base/one/two.el" "/base/")
                (f-short target)
                (f-abbrev target)
                (equal (f-long "~/Code/project/file.el") target)
                (equal (f-canonical ".") (file-truename "."))
                (f-absolute-p "/alpha")
                (f-absolute? "/alpha")
                (f-relative-p "alpha")
                (f-relative? "alpha")))"##;
    let expect = expect![[
        r#"OK ("/base/child/" "one/two.el" "~/Code/project/file.el" "~/Code/project/file.el" t t t t t t)"#
    ]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_common_parent_covers_empty_root_and_relative_cases() {
    let elisp_form = r##"(list
              (f-common-parent nil)
              (f-common-parent '("alpha/file.el"))
              (f-common-parent '("alpha/one.el" "alpha/two.el"))
              (f-common-parent '("/alpha/one.el" "/beta/two.el"))
              (f-common-parent '("alpha/one.el" "beta/two.el"))
              (f-common-parent '("alpha/beta/one.el"
                                 "alpha/beta/gamma/two.el"
                                 "alpha/beta/delta/three.el")))"##;
    let expect = expect![[r#"OK (nil "alpha/" "alpha/" "/" "" "alpha/beta/")"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_extension_and_base_aliases_cover_boundary_names() {
    let elisp_form = r##"(list
              (f-ext "README")
              (f-ext ".emacs")
              (f-ext "archive.tar.gz")
              (f-no-ext "archive.tar.gz")
              (f-base "archive.tar.gz")
              (f-ext-p "archive.tar.gz")
              (f-ext? "archive.tar.gz" "gz")
              (f-ext-p "README")
              (f-swap-ext "README" "md")
              (f-swap-ext ".emacs" "bak"))"##;
    let expect = expect![[
        r#"OK (nil nil "gz" "archive.tar" "archive.tar" t t nil "README.md" ".emacs.bak")"#
    ]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_swap_ext_rejects_empty_extension() {
    let elisp_form = r##"(f-swap-ext "archive.tar.gz" "")"##;
    let expect = expect![[r#"ERR (error "Extension cannot be empty or nil")"#]];

    assert_f_signal_parity(elisp_form, expect);
}

#[test]
fn f_uniquify_returns_minimal_unique_suffixes_and_alist() {
    let elisp_form = r##"(let ((paths '("/foo/bar"
                            "/foo/baz"
                            "/home/www/bar"
                            "/home/www/baz"
                            "/var/foo"
                            "/opt/foo/www/baz")))
               (list (f-uniquify paths)
                     (f-uniquify-alist paths)))"##;
    let expect = expect![[
        r#"OK (("foo/bar" "www/bar" "foo/baz" "home/www/baz" "foo/www/baz" "foo") (("/foo/bar" . "foo/bar") ("/home/www/bar" . "www/bar") ("/foo/baz" . "foo/baz") ("/home/www/baz" . "home/www/baz") ("/opt/foo/www/baz" . "foo/www/baz") ("/var/foo" . "foo")))"#
    ]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_path_shape_helpers_distinguish_files_directories_and_missing_paths() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-shape-" t))
                    (default-directory (file-name-as-directory root))
                    (dir "dir")
                    (file "file"))
               (unwind-protect
                   (progn
                     (f-mkdir dir)
                     (f-touch file)
                     (list
                      (f-slash dir)
                      (f-slash "dir/")
                      (f-slash file)
                      (f-slash "missing")
                      (string-suffix-p "/dir/" (f-full dir))
                      (string-suffix-p "/file" (f-full file))
                      (f-path-separator)
                      (f-root)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK ("dir/" "dir/" "file" "missing" t t "/" "/")"#]];

    assert_f_parity(elisp_form, expect);
}
