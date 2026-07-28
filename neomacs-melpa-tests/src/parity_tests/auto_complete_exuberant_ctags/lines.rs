use expect_test::expect;

use super::assert_auto_complete_exuberant_ctags_parity;

#[test]
fn auto_complete_exuberant_ctags_get_line_preserves_practical_tag_records() {
    let elisp_form = r##"(with-temp-buffer
                           (insert
                            "render_frame\tui.rs\t/^fn render_frame/;\"\tkind:f\tlanguage:Rust")
                           (list
                            (ac-exuberant-ctags-get-line
                             (point-min)
                             (point-max))
                            (buffer-string)
                            (point-min)
                            (point-max)))"##;
    let expect = expect![[
        r#"OK ("render_frame\11ui.rs\11/^fn render_frame/;\"\11kind:f\11language:Rust" "render_frame\11ui.rs\11/^fn render_frame/;\"\11kind:f\11language:Rust" 1 61)"#
    ]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_get_line_filters_headers_but_not_embedded_markers() {
    let elisp_form = r##"(mapcar
                           (lambda (line)
                             (with-temp-buffer
                               (insert line)
                               (ac-exuberant-ctags-get-line
                                (point-min)
                                (point-max))))
                           '("!_TAG_FILE_FORMAT\t2"
                             "!_TAG_PROGRAM_NAME\tUniversal Ctags"
                             "alpha!_beta\tfile\tkind:v\tlanguage:C"
                             " !_\tfile\tkind:v\tlanguage:C"))"##;
    let expect = expect![[
        r#"OK ("" "" "alpha!_beta\11file\11kind:v\11language:C" " !_\11file\11kind:v\11language:C")"#
    ]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_get_line_observes_exact_length_boundary() {
    let elisp_form = r##"(let ((ac-exuberant-ctags-line-length-limit 5))
                           (mapcar
                            (lambda (line)
                              (with-temp-buffer
                                (insert line)
                                (list
                                 (length line)
                                 (ac-exuberant-ctags-get-line
                                  (point-min)
                                  (point-max)))))
                            '("" "abcd" "abcde" "abcdef")))"##;
    let expect = expect![[r#"OK ((0 "") (4 "abcd") (5 "abcde") (6 ""))"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_get_line_counts_multibyte_characters() {
    let elisp_form = r##"(let ((ac-exuberant-ctags-line-length-limit 3))
                           (mapcar
                            (lambda (line)
                              (with-temp-buffer
                                (insert line)
                                (list
                                 (length line)
                                 (string-bytes line)
                                 (ac-exuberant-ctags-get-line
                                  (point-min)
                                  (point-max)))))
                            '("λ界x" "λ界xy" "ééé" "éééé")))"##;
    let expect = expect![[r#"OK ((3 6 "λ界x") (4 7 "") (3 6 "ééé") (4 8 ""))"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_get_line_respects_arbitrary_buffer_spans() {
    let elisp_form = r##"(with-temp-buffer
                           (insert "prefix|actual-tag-record|suffix")
                           (let ((ac-exuberant-ctags-line-length-limit
                                  100))
                             (list
                              (ac-exuberant-ctags-get-line 8 25)
                              (ac-exuberant-ctags-get-line 1 7)
                              (ac-exuberant-ctags-get-line 25 32))))"##;
    let expect = expect![[r#"OK ("actual-tag-record" "prefix" "|suffix")"#]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}

#[test]
fn auto_complete_exuberant_ctags_get_line_invalid_ranges_signal_exactly() {
    let elisp_form = r##"(with-temp-buffer
                           (insert "abc")
                           (mapcar
                            (lambda (bounds)
                              (auto-complete-exuberant-ctags-test-error
                               (lambda ()
                                 (ac-exuberant-ctags-get-line
                                  (car bounds)
                                  (cadr bounds)))))
                            '((0 2) (1 9) (3 2))))"##;
    let expect = expect![[
        r#"OK ((:signal args-out-of-range ((:buffer nil) 0 2)) (:signal args-out-of-range ((:buffer nil) 1 9)) (:value "b"))"#
    ]];

    assert_auto_complete_exuberant_ctags_parity(elisp_form, expect);
}
