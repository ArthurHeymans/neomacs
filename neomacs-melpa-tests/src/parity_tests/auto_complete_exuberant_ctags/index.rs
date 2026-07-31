use expect_test::expect;

use super::{ParityBatchCase, assert_auto_complete_exuberant_ctags_batch};

fn auto_complete_exuberant_ctags_builds_practical_multilanguage_index() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_builds_practical_multilanguage_index",
        r##"(let* ((root
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
                           (ac-exuberant-ctags-build-index))"##,
        true,
        expect![[r#"OK ("save! m Ruby" "Widget c C++" "render_frame f Rust")"#]],
    )
}

fn auto_complete_exuberant_ctags_missing_database_clears_stale_index() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_missing_database_clears_stale_index",
        r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "missing-index"))
                                (default-directory root)
                                (ac-exuberant-ctags-tag-file-search-limit
                                 0)
                                (ac-exuberant-ctags-index
                                 '("stale f C")))
                           (list
                            (ac-exuberant-ctags-build-index)
                            ac-exuberant-ctags-index))"##,
        true,
        expect!["OK (nil nil)"],
    )
}

fn auto_complete_exuberant_ctags_empty_database_clears_stale_index() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_empty_database_clears_stale_index",
        r##"(let* ((root
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
                            ac-exuberant-ctags-index))"##,
        true,
        expect!["OK (nil nil)"],
    )
}

fn auto_complete_exuberant_ctags_parser_filters_blank_header_and_malformed_rows() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_parser_filters_blank_header_and_malformed_rows",
        r##"(let* ((root
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
                           (ac-exuberant-ctags-build-index))"##,
        true,
        expect![[r#"OK ("good f C")"#]],
    )
}

fn auto_complete_exuberant_ctags_parser_preserves_duplicates_and_reverse_file_order() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_parser_preserves_duplicates_and_reverse_file_order",
        r##"(let* ((root
                                 (auto-complete-exuberant-ctags-test-root
                                  "duplicate-index"))
                                (default-directory root))
                           (auto-complete-exuberant-ctags-test-write
                            (expand-file-name "tags" root)
                            (concat
                             "same\ta.c\t/^x$/;\"\tkind:f\tlanguage:C\n"
                             "middle\tb.rs\t/^x$/;\"\tkind:m\tlanguage:Rust\n"
                             "same\tc.cpp\t/^x$/;\"\tkind:p\tlanguage:C++\n"))
                           (ac-exuberant-ctags-build-index))"##,
        true,
        expect![[r#"OK ("same p C++" "middle m Rust" "same f C")"#]],
    )
}

fn auto_complete_exuberant_ctags_parser_observes_line_length_limit() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_parser_observes_line_length_limit",
        r##"(let* ((root
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
                            (ac-exuberant-ctags-build-index)))"##,
        true,
        expect![[r#"OK (22 28 ("ok f C"))"#]],
    )
}

fn auto_complete_exuberant_ctags_build_index_queries_tag_path_twice() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_build_index_queries_tag_path_twice",
        r##"(let* ((root
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
                              calls)))"##,
        true,
        expect![[r#"OK (("entry v C") 2)"#]],
    )
}

fn auto_complete_exuberant_ctags_custom_database_name_builds_same_index() -> ParityBatchCase {
    ParityBatchCase::new(
        "auto_complete_exuberant_ctags_custom_database_name_builds_same_index",
        r##"(let* ((root
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
                             (ac-exuberant-ctags-get-tag-file))))"##,
        true,
        expect![[r#"OK (("dispatch f C") ".ctags-index")"#]],
    )
}

#[test]
fn index_public_surface_batch() {
    let cases: Vec<ParityBatchCase> = vec![
        auto_complete_exuberant_ctags_builds_practical_multilanguage_index(),
        auto_complete_exuberant_ctags_missing_database_clears_stale_index(),
        auto_complete_exuberant_ctags_empty_database_clears_stale_index(),
        auto_complete_exuberant_ctags_parser_filters_blank_header_and_malformed_rows(),
        auto_complete_exuberant_ctags_parser_preserves_duplicates_and_reverse_file_order(),
        auto_complete_exuberant_ctags_parser_observes_line_length_limit(),
        auto_complete_exuberant_ctags_build_index_queries_tag_path_twice(),
        auto_complete_exuberant_ctags_custom_database_name_builds_same_index(),
    ];
    assert_auto_complete_exuberant_ctags_batch(&cases);
}
