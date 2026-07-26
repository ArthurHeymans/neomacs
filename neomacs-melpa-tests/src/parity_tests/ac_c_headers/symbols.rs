use expect_test::expect;

use super::assert_ac_c_headers_parity;

#[test]
fn ac_c_headers_search_header_file_selects_first_existing_root_and_handles_slashes() {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (first
                     (expand-file-name
                      "search-first/"
                      root))
                    (second
                     (expand-file-name
                      "search-second"
                      root))
                    (cc-search-directories
                     (list first second)))
               (make-directory first t)
               (make-directory second t)
               (with-temp-file
                   (expand-file-name
                    "shared.h"
                    first)
                 (insert "first"))
               (with-temp-file
                   (expand-file-name
                    "shared.h"
                    second)
                 (insert "second"))
               (with-temp-file
                   (expand-file-name
                    "second.h"
                    second)
                 (insert "second-only"))
               (list
                (equal
                 (ac-c-headers--search-header-file
                  "shared.h")
                 (expand-file-name
                  "shared.h"
                  first))
                (equal
                 (ac-c-headers--search-header-file
                  "second.h")
                 (expand-file-name
                  "second.h"
                  second))
                (ac-c-headers--search-header-file
                 "missing.h")))"##;
    let expect = expect!["OK (t t nil)"];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_symbols_update_strips_comments_deduplicates_and_keeps_reverse_last_occurrence_order()
 {
    let elisp_form = r##"(let* ((root
                     (getenv
                      "NEOMACS_TEST_SANDBOX_ROOT"))
                    (include
                     (expand-file-name
                      "symbols-update/"
                      root))
                    (cc-search-directories
                     (list include))
                    (ac-c-headers--symbols-cache
                     nil))
               (make-directory include t)
               (with-temp-file
                   (expand-file-name
                    "fixture.h"
                    include)
                 (insert
                  "#define ALPHA 1\n"
                  "int alpha;\n"
                  "int shared;\n"
                  "/* HiddenBlock HiddenAgain */\n"
                  "// HiddenLine\n"
                  "char shared;\n"
                  "typedef struct Thing {\n"
                  "  int member;\n"
                  "} Thing;\n"))
               (list
                (ac-c-headers--symbols-update
                 "fixture.h")
                ac-c-headers--symbols-cache
                (ac-c-headers--symbols-update
                 "fixture.h")
                ac-c-headers--symbols-cache))"##;
    let expect = expect![[
        r#"OK (#1=(("fixture.h" "Thing" "member" "int" "struct" "typedef" "shared" "char" "alpha" "ALPHA" "define")) #1# nil #1#)"#
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_symbols_update_does_not_cache_missing_headers_and_retries_lookup() {
    let elisp_form = r##"(let ((ac-c-headers--symbols-cache nil)
                    calls)
               (cl-letf
                   (((symbol-function
                      'ac-c-headers--search-header-file)
                     (lambda (header)
                       (push header calls)
                       nil)))
                 (list
                  (ac-c-headers--symbols-update
                   "missing.h")
                  ac-c-headers--symbols-cache
                  (ac-c-headers--symbols-update
                   "missing.h")
                  ac-c-headers--symbols-cache
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil nil nil nil ("missing.h" "missing.h"))"#]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_symbols_list_scans_strict_include_lines_prepends_later_headers_and_repeats_duplicates()
 {
    let elisp_form = r##"(let ((ac-c-headers--symbols-cache
                    '(("a.h" "A" "shared")
                      ("b.h" "B"))))
               (with-temp-buffer
                 (insert
                  "#include <a.h>\n"
                  " #include <b.h>\n"
                  "#include\"b.h\"\n"
                  "#include <a.h>\n"
                  "#include <missing.h> trailing\n")
                 (cl-letf
                     (((symbol-function
                        'ac-c-headers--symbols-update)
                       (lambda (header)
                         (push
                          (cons header
                                (list
                                 (concat
                                  "loaded-"
                                  header)))
                          ac-c-headers--symbols-cache))))
                   (list
                    (ac-c-headers--symbols-list)
                    ac-c-headers--symbols-cache))))"##;
    let expect = expect![[
        r#"OK (("loaded-missing.h" "A" "shared" "B" "A" "shared") (("missing.h" "loaded-missing.h") ("a.h" "A" "shared") ("b.h" "B")))"#
    ]];

    assert_ac_c_headers_parity(elisp_form, expect);
}

#[test]
fn ac_c_headers_symbols_list_honors_explicit_buffer_without_changing_current_buffer_or_point() {
    let elisp_form = r##"(let ((source
                    (generate-new-buffer
                     " *ac-c-headers-source*"))
                   (ac-c-headers--symbols-cache
                    '(("one.h" "ONE"))))
               (unwind-protect
                   (with-temp-buffer
                     (insert "caller")
                     (goto-char 3)
                     (with-current-buffer source
                       (insert
                        "#include <one.h>\n"))
                     (list
                      (ac-c-headers--symbols-list
                       source)
                      (buffer-string)
                      (point)
                      (eq
                       (current-buffer)
                       source)))
                 (kill-buffer source)))"##;
    let expect = expect![[r#"OK (("ONE") "caller" 3 nil)"#]];

    assert_ac_c_headers_parity(elisp_form, expect);
}
