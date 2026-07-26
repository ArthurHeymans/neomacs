use expect_test::expect;

use super::{assert_magit_parity, assert_magit_signal_parity};

#[test]
fn magit_clone_name_expansion_supports_service_aliases_and_host_formats() {
    let elisp_form = r##"(progn
               (require 'magit-clone)
               (let ((magit-clone-name-alist
                      '(("\\`\\(?:example:\\|ex:\\)\\([^:]+\\)\\'"
                         "git.example.com" "foouser")
                        ("\\`\\(?:github:\\|gh:\\)?\\([^:]+\\)\\'"
                         "github.com" "u")))
                     (magit-clone-url-format
                      '(("git.example.com" . "cow@%h:~%n")
                        (t . "git@%h:%n.git"))))
                 (list
                  (magit-clone--name-to-url "gh:a/b")
                  (magit-clone--name-to-url "github:c/d")
                  (magit-clone--name-to-url "ex:a/b")
                  (magit-clone--name-to-url "example:x/y")
                  (magit-clone--name-to-url "ex:c"))))"##;
    let expect = expect![[
        r#"OK ("git@github.com:a/b.git" "git@github.com:c/d.git" "cow@git.example.com:~a/b" "cow@git.example.com:~x/y" "cow@git.example.com:~foouser/c")"#
    ]];

    assert_magit_parity(elisp_form, expect);
}

#[test]
fn magit_clone_name_expansion_supports_one_global_format() {
    let elisp_form = r##"(progn
               (require 'magit-clone)
               (let ((magit-clone-url-format "bird@%h:%n.git")
                     (magit-clone-name-alist
                      '(("\\`\\(?:github:\\|gh:\\)?\\([^:]+\\)\\'"
                         "github.com" "u")
                        ("\\`\\(?:gitlab:\\|gl:\\)\\([^:]+\\)\\'"
                         "gitlab.com" "u"))))
                 (mapcar
                  #'magit-clone--name-to-url
                  '("gh:a/b" "gl:a/b"
                    "github:c/d" "gitlab:c/d"))))"##;
    let expect = expect![[
        r#"OK ("bird@github.com:a/b.git" "bird@gitlab.com:a/b.git" "bird@github.com:c/d.git" "bird@gitlab.com:c/d.git")"#
    ]];

    assert_magit_parity(elisp_form, expect);
}

#[test]
fn magit_clone_rejects_a_non_string_format() {
    let elisp_form = r##"(progn
               (require 'magit-clone)
               (let ((magit-clone-url-format 3))
                 (magit-clone--name-to-url "gh:a/b")))"##;
    let expect = expect![[
        r#"ERR (user-error "Bogus ‘magit-clone-url-format’ (bad type or missing default)")"#
    ]];

    assert_magit_signal_parity(elisp_form, expect);
}
