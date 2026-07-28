use expect_test::expect;

use super::assert_auto_complete_exuberant_ctags_parity;

#[test]
fn auto_complete_exuberant_ctags_builds_practical_multilanguage_index() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "multilanguage-index"))
                                (default-directory root))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name "tags" root)
                            (concat
                             "!_TAG_FILE_FORMAT\t2\t/extended format/\n"
                             "render_frame\tui.rs\t/^fn render_frame/;\"\tkind:f\tline:42\tlanguage:Rust\n"
                             "Widget\twidget.hpp\t/^class Widget/;\"\tkind:c\tlanguage:C++\n"
                             "save!\tmodel.rb\t/^  def save!/;\"\tkind:m\tlanguage:Ruby\n"))
                           (ac-exuberant-ctags-build-index))"##;
    let expect = expect![[r#"OK ("save! m Ruby" "Widget c C++" "render_frame f Rust")"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_missing_database_clears_stale_index() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "missing-index"))
                                (default-directory root)
                                (ac-exuberant-ctags-tag-file-search-limit
                                 0)
                                (ac-exuberant-ctags-index
                                 '("stale f C")))
                           (list
                            (ac-exuberant-ctags-build-index)
                            ac-exuberant-ctags-index))"##;
    let expect = expect!["OK (nil nil)"];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_empty_database_clears_stale_index() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "empty-index"))
                                (default-directory root)
                                (ac-exuberant-ctags-index
                                 '("stale f C")))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name "tags" root)
                            "")
                           (list
                            (ac-exuberant-ctags-build-index)
                            ac-exuberant-ctags-index))"##;
    let expect = expect!["OK (nil nil)"];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_parser_filters_blank_header_and_malformed_rows() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "filtered-index"))
                                (default-directory root))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name "tags" root)
                            (concat
                             "\n   \n"
                             "!_TAG_PROGRAM_VERSION\t6.1\n"
                             "missing-fields\tfile.c\t/^x$/\n"
                             "missing-language\tfile.c\t/^x$/;\"\tkind:f\n"
                             "good\tfile.c\t/^x$/;\"\tkind:f\tlanguage:C\n"
                             "language-before-kind\tfile.c\t/^x$/;\"\tlanguage:C\tkind:f\n"))
                           (ac-exuberant-ctags-build-index))"##;
    let expect = expect![[r#"OK ("good f C")"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_parser_preserves_duplicates_and_reverse_file_order() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "duplicate-index"))
                                (default-directory root))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name "tags" root)
                            (concat
                             "same\ta.c\t/^x$/;\"\tkind:f\tlanguage:C\n"
                             "middle\tb.rs\t/^x$/;\"\tkind:m\tlanguage:Rust\n"
                             "same\tc.cpp\t/^x$/;\"\tkind:p\tlanguage:C++\n"))
                           (ac-exuberant-ctags-build-index))"##;
    let expect = expect![[r#"OK ("same p C++" "middle m Rust" "same f C")"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_parser_observes_line_length_limit() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "line-limit-index"))
                                (default-directory root)
                                (short
                                 "ok\tx\tkind:f\tlanguage:C")
                                (long
                                 "too_long\tx\tkind:f\tlanguage:C")
                                (ac-exuberant-ctags-line-length-limit
                                 (length short)))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name "tags" root)
                            (concat short "\n" long "\n"))
                           (list
                            (length short)
                            (length long)
                            (ac-exuberant-ctags-build-index)))"##;
    let expect = expect![[r#"OK (22 28 ("ok f C"))"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_build_index_queries_tag_path_twice() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "query-count"))
                                (tags
                                 (expand-file-name "tags" root))
                                (calls 0))
                           (auto-complete-exuberant-ctags-test-write
                            tags
                            "entry\tx\tkind:v\tlanguage:C\n")
                           (cl-letf
                               (((symbol-function
                                  'ac-exuberant-ctags-get-tag-file)
                                 (lambda ()
                                   (setq calls
                                         (1+ calls))
                                   tags)))
                             (list
                              (ac-exuberant-ctags-build-index)
                              calls)))"##;
    let expect = expect![[r#"OK (("entry v C") 2)"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_custom_database_name_builds_same_index() {
    let elisp_form = r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "custom-index"))
                                (default-directory root)
                                (ac-exuberant-ctags-tag-file-name
                                 ".ctags-index"))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name ".ctags-index" root)
                            "dispatch\tsrc/app.c\t/^dispatch/;\"\tkind:f\tlanguage:C\n")
                           (list
                            (ac-exuberant-ctags-build-index)
                            (file-name-nondirectory
                             (ac-exuberant-ctags-get-tag-file))))"##;
    let expect = expect![[r#"OK (("dispatch f C") ".ctags-index")"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}
