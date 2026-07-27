use expect_test::expect;

use super::assert_apheleia_parity;

#[test]
fn apheleia_edit_distance_table_ports_the_upstream_header_example_exactly() {
    let elisp_form = r##"(list
         (apheleia-test-table
          "hello"
          "heo")
         (apheleia-test-table
          ""
          "abc")
         (apheleia-test-table
          "abc"
          "")
         (apheleia-test-table
          "same"
          "same"))"##;
    let expect = expect![
        "OK (((0 1 2 3 4 5) (1 0 1 2 3 4) (2 1 0 1 2 3) (3 2 1 1 2 2)) ((0) (1) (2) (3)) ((0 1 2 3)) ((0 1 2 3 4) (1 0 1 2 3) (2 1 0 1 2) (3 2 1 0 1) (4 3 2 1 0)))"
    ];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_align_point_ports_and_strengthens_all_upstream_alignment_cases() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (let* ((before-spec
                   (car case))
                  (after-spec
                   (cadr case))
                  (before-point
                   (string-match-p
                    "|"
                    before-spec))
                  (before
                   (replace-regexp-in-string
                    "|"
                    ""
                    before-spec
                    nil
                    'literal))
                  (after
                   (replace-regexp-in-string
                    "|"
                    ""
                    after-spec
                    nil
                    'literal))
                  (aligned
                   (apheleia--align-point
                    before
                    after
                    before-point)))
             (list
              before-spec
              after-spec
              aligned
              (concat
               (substring after 0 aligned)
               "|"
               (substring after aligned)))))
         '(("hel|lo" "he|o")
           ("hello| world" "helo| word")
           ("hello | world" "hello|world")
           ("|prefix" "new |prefix")
           ("suffix|" "suffix plus|")
           ("a😀b|c" "a😀|c")
           ("abc|def" "abcXYZ|def")
           ("      | <div class=\"left-[40rem] fixed\">\n  <svg\n"
            "|<div class=\"left-[40rem] fixed\">\n <svg")))"##;
    let expect = expect![[
        r#"OK (("hel|lo" "he|o" 2 "he|o") ("hello| world" "helo| word" 4 "helo| word") ("hello | world" "hello|world" 5 "hello|world") ("|prefix" "new |prefix" 0 "|new prefix") ("suffix|" "suffix plus|" 6 "suffix| plus") ("a😀b|c" "a😀|c" 2 "a😀|c") ("abc|def" "abcXYZ|def" 3 "abc|XYZdef") ("      | <div class=\"left-[40rem] fixed\">\n  <svg\n" "|<div class=\"left-[40rem] fixed\">\n <svg" 0 "|<div class=\"left-[40rem] fixed\">\n <svg"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_rcs_parser_maps_additions_deletions_and_zero_line_insertions() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "a0 2\n"
          "first\n"
          "second\n"
          "d3 4\n"
          "a7 1\n"
          "replacement\n")
         (let (commands)
           (apheleia--map-rcs-patch
            (lambda (command)
              (setq commands
                    (append
                     commands
                     (list
                      (copy-tree
                       command))))))
           commands))"##;
    let expect = expect![[
        r#"OK (((command . addition) (start . 0) (lines . 2) (text . "first\nsecond\n")) ((command . deletion) (start . 3) (lines . 4)) ((command . addition) (start . 7) (lines . 1) (text . "replacement\n")))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_rcs_parser_reports_malformed_commands_with_exact_error_data() {
    let elisp_form = r##"(mapcar
         (lambda (patch)
           (with-temp-buffer
             (insert patch)
             (condition-case error
                 (progn
                   (apheleia--map-rcs-patch
                    #'ignore)
                   :ok)
               (error
                (list
                 (car error)
                 (cadr error))))))
         '("x1 2\n"
           "a1 nope\n"
           "a1 2\nonly-one-line\n"
           "\ninvalid\n"))"##;
    let expect = expect![[
        r#"OK ((error "Malformed RCS patch: 1") (error "Malformed RCS patch: 1") :ok (error "Malformed RCS patch: 2"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_apply_rcs_patch_replaces_text_and_aligns_point_mark_and_mark_ring() {
    let elisp_form = r##"(let ((content
                (generate-new-buffer
                 " *apheleia-rcs-content*"))
               (patch
                (generate-new-buffer
                 " *apheleia-rcs-patch*")))
         (unwind-protect
             (progn
               (with-current-buffer content
                 (insert
                  "alpha\n"
                  "hello world\n"
                  "omega\n")
                 (goto-char
                  (point-min))
                 (search-forward
                  "hello")
                 (push-mark
                  (point)
                  t)
                 (search-forward
                  "world")
                 (push
                  (copy-marker
                   (- (point) 3))
                  mark-ring))
               (with-current-buffer patch
                 (insert
                  "d2 1\n"
                  "a2 1\n"
                  "hello brave world\n"))
               (with-current-buffer content
                 (apheleia--apply-rcs-patch
                  content
                  patch)
                 (list
                  (buffer-string)
                  (point)
                  (current-column)
                  (marker-position
                   (mark-marker))
                  (mapcar
                   #'marker-position
                   mark-ring)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position)))))
           (kill-buffer content)
           (kill-buffer patch)))"##;
    let expect =
        expect![[r#"OK ("alpha\nhello brave world\nomega\n" 24 17 7 (13) "hello brave world")"#]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_apply_rcs_patch_handles_top_bottom_insertions_and_multiline_deletion() {
    let elisp_form = r##"(let ((content
                (generate-new-buffer
                 " *apheleia-rcs-boundaries*"))
               (patch
                (generate-new-buffer
                 " *apheleia-rcs-boundary-patch*")))
         (unwind-protect
             (progn
               (with-current-buffer content
                 (insert
                  "one\n"
                  "two\n"
                  "three\n"
                  "four\n")
                 (goto-char
                  (point-max)))
               (with-current-buffer patch
                 (insert
                  "a0 1\n"
                  "zero\n"
                  "d2 2\n"
                  "a4 1\n"
                  "five\n"))
               (with-current-buffer content
                 (apheleia--apply-rcs-patch
                  content
                  patch)
                 (list
                  (buffer-string)
                  (point)
                  (line-number-at-pos)
                  (current-column))))
           (kill-buffer content)
           (kill-buffer patch)))"##;
    let expect = expect![[r#"OK ("zero\none\nfour\nfive\n" 15 4 0)"#]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_apply_rcs_patch_ports_upstream_window_scroll_regression_case() {
    let elisp_form = r##"(save-window-excursion
         (let ((content
                (generate-new-buffer
                 " *apheleia-scroll-content*"))
               (patch
                (generate-new-buffer
                 " *apheleia-scroll-patch*")))
           (unwind-protect
               (progn
                 (with-current-buffer content
                   (dotimes (index 50)
                     (if (= index 20)
                         (insert
                          (format
                           "    line %02d\n"
                           index))
                       (insert
                        (format
                         "line %02d\n"
                         index))))
                   (switch-to-buffer content)
                   (goto-char
                    (point-min))
                   (forward-line 10)
                   (set-window-start
                    (selected-window)
                    (point))
                   (goto-char
                    (point-min))
                   (forward-line 20)
                   (forward-char 4))
                 (with-current-buffer patch
                   (insert
                    "d21 1\n"
                    "a21 1\n"
                    "line 20\n"))
                 (with-current-buffer content
                   (apheleia--apply-rcs-patch
                    content
                    patch)
                   (list
                    (line-number-at-pos
                     (window-start))
                    (line-number-at-pos)
                    (current-column)
                    (window-start)
                    (point))))
             (kill-buffer content)
             (kill-buffer patch))))"##;
    let expect = expect!["OK (11 21 0 81 161)"];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_alignment_size_limit_changes_cursor_strategy_without_changing_patch_text() {
    let elisp_form = r##"(mapcar
         (lambda (limit)
           (let ((content
                  (generate-new-buffer
                   " *apheleia-limit-content*"))
                 (patch
                  (generate-new-buffer
                   " *apheleia-limit-patch*"))
                 (apheleia-max-alignment-size
                  limit))
             (unwind-protect
                 (progn
                   (with-current-buffer content
                     (insert
                      "header\n"
                      "abcdefghij\n"
                      "footer\n")
                     (goto-char
                      (point-min))
                     (search-forward
                      "fgh"))
                   (with-current-buffer patch
                     (insert
                      "d2 1\n"
                      "a2 1\n"
                      "abXYZcdefghij\n"))
                   (with-current-buffer content
                     (apheleia--apply-rcs-patch
                      content
                      patch)
                     (list
                      limit
                      (buffer-string)
                      (point)
                      (current-column))))
               (kill-buffer content)
               (kill-buffer patch))))
         '(0 400))"##;
    let expect = expect![[
        r#"OK ((0 "header\nabXYZcdefghij\nfooter\n" 16 8) (400 "header\nabXYZcdefghij\nfooter\n" 19 11))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}
