use expect_test::expect;

use super::assert_archive_region_parity;

#[test]
fn archive_region_current_archive_file_appends_default_and_custom_suffixes_to_exact_path() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (with-temp-buffer
             (setq-local
              buffer-file-name
              (car spec))
             (let ((archive-region-filename-suffix
                    (cadr spec)))
               (list
                spec
                (archive-region-current-archive-file)))))
         '(("/workspace/notes.org" "_archive")
           ("/workspace/report" ".history")
           ("/workspace/space name.txt" " [archived]")
           ("/workspace/日本語.md" "_保管")
           ("/workspace/no-change" "")))"##;
    let expect = expect![[
        r#"OK ((("/workspace/notes.org" "_archive") "/workspace/notes.org_archive") (("/workspace/report" ".history") "/workspace/report.history") (("/workspace/space name.txt" " [archived]") "/workspace/space name.txt [archived]") (("/workspace/日本語.md" "_保管") "/workspace/日本語.md_保管") (("/workspace/no-change" "") "/workspace/no-change"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_current_original_file_removes_only_quoted_suffix_at_end() {
    let elisp_form = r##"(mapcar
         (lambda (spec)
           (with-temp-buffer
             (setq-local
              buffer-file-name
              (car spec))
             (let ((archive-region-filename-suffix
                    (cadr spec)))
               (list
                spec
                (archive-region-current-original-file)))))
         '(("/workspace/notes.org_archive" "_archive")
           ("/workspace/_archive/notes.org" "_archive")
           ("/workspace/name_archive.extra" "_archive")
           ("/workspace/name.a+b" ".a+b")
           ("/workspace/name[old]" "[old]")
           ("/workspace/日本語_保管" "_保管")
           ("/workspace/plain" "_archive")))"##;
    let expect = expect![[
        r#"OK ((("/workspace/notes.org_archive" "_archive") "/workspace/notes.org") (("/workspace/_archive/notes.org" "_archive") "/workspace/_archive/notes.org") (("/workspace/name_archive.extra" "_archive") "/workspace/name_archive.extra") (("/workspace/name.a+b" ".a+b") "/workspace/name") (("/workspace/name[old]" "[old]") "/workspace/name") (("/workspace/日本語_保管" "_保管") "/workspace/日本語") (("/workspace/plain" "_archive") "/workspace/plain"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_path_helpers_signal_before_fabricating_names_for_non_file_buffers() {
    let elisp_form = r##"(with-temp-buffer
         (list
          (mapcar
           (lambda (function)
             (condition-case error-data
                 (list
                  function
                  :ok
                  (funcall function))
               (error
                (list
                 function
                 :error
                 (car error-data)
                 (cdr error-data)))))
           '(archive-region-current-archive-file
             archive-region-current-original-file))
          buffer-file-name
          (buffer-string)
          (point)))"##;
    let expect = expect![[
        r#"OK (((archive-region-current-archive-file :error error ("Need filename")) (archive-region-current-original-file :error error ("Need filename"))) nil "" 1)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_link_to_original_ports_every_upstream_previous_line_case() {
    let elisp_form = r##"(mapcar
         (lambda (text)
           (with-temp-buffer
             (insert text)
             (list
              text
              (line-number-at-pos)
              (archive-region-link-to-original))))
         '("previous-line\ncurrent-line"
           "previous-nonempty-line\n\ncurrent-line"
           "previous-nonempty-line\n\n\ncurrent-line"
           "first-line"))"##;
    let expect = expect![[
        r#"OK (("previous-line\ncurrent-line" 2 (archive-region-pos "previous-line")) ("previous-nonempty-line\n\ncurrent-line" 3 (archive-region-pos "previous-nonempty-line")) ("previous-nonempty-line\n\n\ncurrent-line" 4 (archive-region-pos "previous-nonempty-line")) ("first-line" 1 (archive-region-pos nil)))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_link_to_original_handles_unicode_whitespace_and_point_inside_current_line() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "見出し α\n"
          "   \n"
          "\t\n"
          "current λ line\n"
          "tail\n")
         (goto-char
          (point-min))
         (forward-line 3)
         (search-forward
          "λ")
         (list
          (line-number-at-pos)
          (current-column)
          (archive-region-link-to-original)
          (point)
          (buffer-string)))"##;
    let expect = expect![[
        r#"OK (4 9 (archive-region-pos "\11") 22 "見出し α\n   \n\11\ncurrent λ line\ntail\n")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_link_to_original_widens_to_find_context_outside_narrowing() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "outside context\n"
          "\n"
          "selected first\n"
          "selected second\n")
         (let ((start
                (progn
                  (goto-char
                   (point-min))
                  (forward-line 2)
                  (point)))
               (end
                (point-max)))
           (narrow-to-region
            start
            end)
           (goto-char
            (point-max))
           (list
            (point-min)
            (point-max)
            (archive-region-link-to-original)
            (point-min)
            (point-max)
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK (18 49 (archive-region-pos "selected second") 18 49 "selected first\nselected second\n")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_link_to_original_preserves_point_mark_and_restriction_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha\n"
          "beta\n"
          "gamma\n"
          "delta\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (let ((start
                (point)))
           (forward-line 2)
           (narrow-to-region
            start
            (point))
           (goto-char
            (point-min))
           (forward-char 2)
           (set-mark
            (point-max))
           (let ((before
                  (list
                   (point)
                   (mark)
                   (point-min)
                   (point-max))))
             (list
              before
              (archive-region-link-to-original)
              (list
               (point)
               (mark)
               (point-min)
               (point-max))
              (buffer-string)))))"##;
    let expect =
        expect![[r#"OK ((9 18 7 18) (archive-region-pos "alpha") (9 18 7 18) "beta\ngamma\n")"#]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_pos_visits_real_original_file_and_exposes_first_duplicate_missing_and_nil_lookup()
{
    let elisp_form = r##"(save-window-excursion
         (let* ((original
                 (archive-region-test-path
                  "navigation.org"))
                (archive
                 (concat
                  original
                  archive-region-filename-suffix)))
           (with-temp-file
               original
             (insert
              "first\n"
              "target\n"
              "middle\n"
              "target\n"
              "last\n"))
           (with-temp-file
               archive
             (insert
              "archive"))
           (unwind-protect
               (with-current-buffer
                   (find-file-noselect
                    archive)
                 (mapcar
                  (lambda (line)
                    (let ((result
                           (archive-region-pos
                            line)))
                      (list
                       line
                       result
                       (file-name-nondirectory
                        buffer-file-name)
                       (line-number-at-pos)
                       (buffer-substring-no-properties
                        (line-beginning-position)
                        (line-end-position)))))
                  '("target"
                    "first"
                    "missing"
                    nil)))
             (archive-region-test-kill-file-buffers)
             (delete-file archive)
             (delete-file original))))"##;
    let expect = expect![[
        r#"OK (("target" 0 "navigation.org" 2 "target") ("first" nil "navigation.org" 1 "first") ("missing" nil "navigation.org" 1 "first") (nil nil "navigation.org" 1 "first"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}
