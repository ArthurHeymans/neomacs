use expect_test::expect;

use super::assert_ac_c_headers_parity;

#[test]
fn ac_c_headers_files_update_indexes_headers_directories_dot_entries_and_search_roots_in_order() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (first
                     (expand-file-name
                      "files-first/"
                      root))
                    (second
                     (expand-file-name
                      "files-second/"
                      root))
                    (cc-search-directories
                     (list first second))
                    (ac-c-headers--files-cache
                     nil))
               (make-directory
                (expand-file-name
                 "nested"
                 first)
                t)
               (make-directory second t)
               (dolist
                   (path
                    (list
                     (expand-file-name
                      "alpha.h"
                      first)
                     (expand-file-name
                      "ignored.hpp"
                      first)
                     (expand-file-name
                      "alpha.h"
                      second)
                     (expand-file-name
                      "beta.H"
                      second)
                     (expand-file-name
                      "plain"
                      second)))
                 (with-temp-file path
                   (insert "fixture")))
               (list
                (ac-c-headers--files-update)
                ac-c-headers--files-cache
                (ac-c-headers--files-update)
                ac-c-headers--files-cache))"##;
    let expect = expect![[
        r#"OK (#1=(("" "./" "../" "alpha.h" "nested/" "./" "../" "alpha.h" "beta.H")) #1# nil #1#)"#
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_files_update_scopes_subdirectory_prefixes_and_caches_empty_results() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (include
                     (expand-file-name
                      "prefix-cache/"
                      root))
                    (cc-search-directories
                     (list include))
                    (ac-c-headers--files-cache
                     nil))
               (make-directory
                (expand-file-name
                 "sub/deeper"
                 include)
                t)
               (with-temp-file
                   (expand-file-name
                    "sub/item.h"
                    include)
                 (insert "fixture"))
               (list
                (ac-c-headers--files-update
                 "sub/")
                ac-c-headers--files-cache
                (ac-c-headers--files-update
                 "missing/")
                ac-c-headers--files-cache
                (progn
                  (make-directory
                   (expand-file-name
                    "missing"
                    include)
                   t)
                  (with-temp-file
                      (expand-file-name
                       "missing/later.h"
                       include)
                    (insert "later"))
                  (ac-c-headers--files-update
                   "missing/"))
                ac-c-headers--files-cache))"##;
    let expect = expect![[
        r#"OK (#1=(("sub/" "./" "../" "deeper/" "item.h")) #1# #2=(("missing/") . #1#) #2# nil #2#)"#
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_files_list_uses_directory_prefix_at_optional_point_and_preserves_point() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (include
                     (expand-file-name
                      "files-list/"
                      root))
                    (cc-search-directories
                     (list include))
                    (ac-c-headers--files-cache
                     nil))
               (make-directory
                (expand-file-name
                 "sub"
                 include)
                t)
               (with-temp-file
                   (expand-file-name
                    "sub/alpha.h"
                    include)
                 (insert "fixture"))
               (with-temp-buffer
                 (insert
                  "#include <sub/al>\n"
                  "#include \"root")
                 (let ((sub-point
                        (progn
                          (goto-char
                           (point-min))
                          (search-forward
                           "sub/al")
                          (point))))
                   (goto-char (point-max))
                   (list
                    (ac-c-headers--files-list
                     sub-point)
                    (point)
                    (ac-c-headers--files-list)
                    (point)
                    ac-c-headers--files-cache))))"##;
    let expect = expect![[
        r#"OK (#2=("./" "../" "alpha.h") 33 #1=("./" "../" "sub/") 33 (("" . #1#) ("sub/" . #2#)))"#
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_files_list_rejects_non_include_contexts_without_populating_cache() {
    let elisp_form = r##"(let ((ac-c-headers--files-cache nil)
                    (cc-search-directories nil))
               (with-temp-buffer
                 (insert
                  "sub/header.h\n"
                  "#include <closed.h>\n"
                  "#include malformed")
                 (list
                  (ac-c-headers--files-list)
                  ac-c-headers--files-cache
                  (progn
                    (goto-char (point-min))
                    (search-forward "header")
                    (ac-c-headers--files-list))
                  ac-c-headers--files-cache)))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_files_list_accepts_one_past_point_max_as_an_empty_prefix() {
    let elisp_form = r##"(let ((cc-search-directories nil)
                    (ac-c-headers--files-cache nil))
               (with-temp-buffer
                 (insert "#include <a")
                 (list
                  (ac-c-headers--files-list
                   (+ (point-max) 1))
                  (point)
                  ac-c-headers--files-cache)))"##;
    let expect = expect![[r#"OK (nil 12 (("")))"#]];

    assert_ac_c_headers_parity(elisp_form, expect);
}
