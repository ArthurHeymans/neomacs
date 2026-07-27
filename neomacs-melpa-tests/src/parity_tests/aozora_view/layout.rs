use expect_test::expect;

use super::assert_aozora_view_parity;

#[test]
fn visual_width_counts_half_height_ascii_and_wide_text_at_half_their_normal_width() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "AB"
                      (propertize
                       "CD漢"
                       'display
                       '((height 0.5)))
                      (propertize
                       "EF"
                       'display
                       '((raise 1)))
                      "字")
                     (list
                      (string-width
                       (buffer-string))
                      (aozora-view-buffer-width
                       (point-min)
                       (point-max))
                      (aozora-view-buffer-width
                       3
                       6)))"##;
    let expect = expect!["OK (10 8 2)"];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn integer_fill_column_wraps_real_prose_and_advances_to_end_of_buffer() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "alpha beta gamma delta epsilon zeta\nshort line\n")
                     (let ((aozora-fill-column
                            13))
                       (goto-char
                        (point-min))
                       (aozora-arrange-fill-lines
                        nil)
                       (list
                        (buffer-string)
                        (point)
                        (point-max)
                        fill-column
                        major-mode)))"##;
    let expect = expect![[
        r#"OK ("alpha beta\ngamma delta\nepsilon zeta\nshort line\n" 48 48 70 text-mode)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn fractional_fill_column_uses_window_width_at_the_moment_of_layout() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "one two three four five six seven eight nine")
                     (let ((aozora-fill-column
                            0.5))
                       (cl-letf
                           (((symbol-function
                              'window-width)
                             (lambda
                               (&optional _window)
                               30)))
                         (goto-char
                          (point-min))
                         (aozora-arrange-fill-lines
                          nil)
                         (list
                          fill-column
                          (buffer-string)))))"##;
    let expect = expect![[r#"OK (70 "one two three\nfour five six\nseven eight\nnine")"#]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn fill_logic_never_breaks_inside_a_ruby_main_text_read_only_span() {
    let elisp_form = r##"(with-temp-buffer
                     (insert
                      "prefix words 青空文庫 suffix words after")
                     (let ((start
                            (progn
                              (goto-char
                               (point-min))
                              (search-forward
                               "青空文庫")
                              (-
                               (point)
                               4))))
                       (put-text-property
                        (1+ start)
                        (+ start 4)
                        'read-only
                        t))
                     (let ((aozora-fill-column
                            10)
                           (inhibit-read-only
                            t))
                       (goto-char
                        (point-min))
                       (aozora-arrange-fill-lines
                        nil)
                       (list
                        (buffer-string)
                        (let ((position
                               (text-property-any
                                (point-min)
                                (point-max)
                                'read-only
                                t)))
                          (list
                           (line-number-at-pos
                            position)
                           (buffer-substring-no-properties
                            position
                            (text-property-not-all
                             position
                             (point-max)
                             'read-only
                             t)))))))"##;
    let expect = expect![[
        r#"OK (#("prefix\nwords 青空文庫\n suffix\n words\n after" 14 17 (read-only t)) (2 "空文庫"))"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}

#[test]
fn complete_layout_inserts_ruby_rows_cleans_internal_properties_and_preserves_source_links() {
    let elisp_form = r##"(let ((source
                         (generate-new-buffer
                          " *aozora-layout-source*")))
                     (unwind-protect
                         (with-temp-buffer
                           (insert
                            "前 青空 後\n次の行")
                           (let ((start
                                  (progn
                                    (goto-char
                                     (point-min))
                                    (search-forward
                                     "青空")
                                    (-
                                     (point)
                                     2))))
                             (put-text-property
                              start
                              (+ start 2)
                              'ruby
                              '(2 . "あおぞら"))
                             (put-text-property
                              (1+ start)
                              (+ start 2)
                              'read-only
                              t))
                           (setq
                            aozora-view-text-file
                            "novel.txt"
                            aozora-view-text-buffer
                            source)
                           (let ((aozora-fill-column
                                  30)
                                 (inhibit-read-only
                                  t))
                             (aozora-view-arrange-fill-lines)
                             (list
                              (buffer-string)
                              (get-text-property
                               (point-min)
                               'display)
                              (next-single-property-change
                               (point-min)
                               'display
                               nil
                               (point-max))
                              (text-property-any
                               (point-min)
                               (point-max)
                               'ruby
                               t)
                              (text-property-any
                               (point-min)
                               (point-max)
                               'read-only
                               t)
                              (text-property-any
                               (point-min)
                               (point-max)
                               'left-margin
                               t)
                              aozora-view-text-file
                              (eq
                               aozora-view-text-buffer
                               source))))
                       (kill-buffer source)))"##;
    let expect = expect![[
        r#"OK (#("　　　あおぞら\n前 青空 後\n\n次の行" 0 7 (display #1=((height 0.5))) 7 8 (display #1#) 11 12 (read-only t) 15 16 (display #1#)) ((height 0.5)) 9 nil 12 nil "novel.txt" t)"#
    ]];
    assert_aozora_view_parity(elisp_form, expect);
}
